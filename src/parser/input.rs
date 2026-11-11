//! Iterative precedence parsing with batched additive and multiplicative chains.

use core::{
    mem::{replace, take},
    ops::{Div, Neg},
};

use alloc::{borrow::Cow, vec, vec::Vec};

use num_anafis::Number;

use crate::{
    core::{CompactStorage, Context, Expr, Function},
    errors::{ParseError, Span},
};

use super::{BinaryOp, Cache, Lexer, PowerCache, Token, slots_for};

// Additive chains in long monomial sums commonly reach four terms; reserving
// that many up front keeps the accumulator in one buffer until canonical
// construction consumes it.
const SUM_CAPACITY: usize = 4;

// Multiplicative chains in long monomial sums commonly reach ten factors;
// reserving a dozen avoids repeated moves while canonical construction
// consumes the buffer.
const PRODUCT_CAPACITY: usize = 12;

// Unary minus binds below powers and above products: $-x^2=-(x^2)$.
const NEGATION: u8 = 3;

enum Frame {
    Operator(Operator),
    Parenthesis(Span),
    Call {
        function: Function,
        arguments: Vec<Expr>,
        /// Source range from the function name through its opening parenthesis.
        header: Span,
    },
}

enum Operator {
    Binary { left: Value, operation: BinaryOp },
    Negative,
}

pub enum Value {
    Expression(Expr),
    /// A numeric literal and its representable negation, both cached.
    Literal {
        positive: Expr,
        negative: Option<Expr>,
    },
    /// The factors of each term of an additive chain, stored flat.
    Sum {
        factors: Vec<Expr>,
        ends: Vec<usize>,
    },
    /// A product chain and the cached negation of its leading literal factor.
    Product {
        leading_negative: Option<Expr>,
        factors: Vec<Expr>,
    },
    /// A literal whose value lies in the numeric backend's range only when
    /// negated, held as that negated value; only unary minus accepts it.
    NegatedOnly(Expr, Span),
}

/// The class of the previous token, which implicit products and empty groups
/// depend on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Previous {
    Start,
    Number,
    Open,
    Other,
}

impl Context {
    /// Parse a scalar expression using this context's symbol and function bindings.
    ///
    /// Literals take their values from [`Number::from_decimal`]: digits give
    /// exact integers, and decimal or scientific spellings give finite
    /// approximate values; `_` may separate two digits. Operators are `+`, `-`,
    /// `*`, `/`, and right-associative `^` or `**`; powers bind more tightly
    /// than unary signs: $-x^2=-(x^2)$.
    /// Adjacent expressions imply multiplication, except adjacent numeric literals.
    /// Parentheses must balance; commas separate arguments of known functions.
    ///
    /// Names remain whole: `xy` is one variable. Unknown variable names create
    /// scoped symbols. Existing symbol bindings take precedence over the constants
    /// `pi`, `π`, `e`, and `i`. Calls require a builtin or registered custom function;
    /// an unknown `f(x)` means the product `f * x`. Names created before a parse
    /// error remain bound. Custom bodies are not executed during construction.
    /// Parsing uses heap-backed frames rather than recursive grammar calls.
    ///
    /// # Errors
    /// Returns [`ParseError`] for invalid syntax, out-of-range literals, function
    /// arity errors, or exhausted symbol IDs. Spans are UTF-8 byte ranges.
    ///
    /// ```
    /// use symb_anafis::{Context, Expr, UserFunction};
    /// let context = Context::new()?;
    /// let f = context.register_function("f", UserFunction::new(1..=1))?;
    /// let parsed = context.parse("2x + sin(x)^2 + f(x)")?;
    /// let x = context.symb("x")?;
    /// assert_eq!(parsed, 2 * x + x.sin().pow(2) + f.call(vec![x.into()])?);
    /// assert_eq!(context.parse("1/4")?, Expr::number(1) / 4);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn parse(&self, source: &str) -> Result<Expr, ParseError> {
        let mut lexer = Lexer::new(source);
        let mut frames = Vec::new();
        let slots = slots_for(source.len());
        let mut names = Cache::new(slots);
        let mut literals = Cache::new(slots);
        let mut powers = PowerCache::new(source.len());
        let mut current = None;
        let mut previous = Previous::Start;
        while let Some(item) = lexer.next() {
            let token = item?;
            let preceding = replace(
                &mut previous,
                match token {
                    Token::Number(..) => Previous::Number,
                    Token::Open(_) => Previous::Open,
                    Token::Name(..) | Token::Operator(..) | Token::Close(_) | Token::Comma(_) => {
                        Previous::Other
                    }
                },
            );
            if matches!(token, Token::Number(..) | Token::Name(..) | Token::Open(_))
                && let Some(left) = current.take()
            {
                if preceding == Previous::Number && matches!(token, Token::Number(..)) {
                    return Err(ParseError::UnexpectedToken { span: token.span() });
                }
                push_operator(&mut frames, left, BinaryOp::Multiply, &mut powers)?;
            }
            match token {
                Token::Number(text, span) => {
                    let want_negative = matches!(
                        frames.last(),
                        Some(Frame::Operator(
                            Operator::Negative
                                | Operator::Binary {
                                    operation: BinaryOp::Subtract,
                                    ..
                                }
                        ))
                    );
                    current =
                        Some(literals.resolve(text, want_negative, || Value::number(text, span))?);
                }
                Token::Name(name, span) => {
                    if lexer.at_open()
                        && let Some(function) =
                            Function::builtin(name).or_else(|| self.get_function(name))
                    {
                        let opening = lexer.open();
                        previous = Previous::Open;
                        frames.push(Frame::Call {
                            function,
                            arguments: Vec::new(),
                            header: Span::new(span.start(), opening.end()),
                        });
                    } else {
                        current =
                            Some(names.resolve(name, false, || Value::symbol(self, name, span))?);
                    }
                }
                Token::Open(opening) => frames.push(Frame::Parenthesis(opening)),
                Token::Operator(operation, span) => {
                    if let Some(left) = current.take() {
                        push_operator(&mut frames, left, operation, &mut powers)?;
                    } else {
                        match operation {
                            BinaryOp::Add => (),
                            BinaryOp::Subtract => frames.push(Frame::Operator(Operator::Negative)),
                            BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Power => {
                                return Err(ParseError::ExpectedExpression { span });
                            }
                        }
                    }
                }
                Token::Close(_) | Token::Comma(_) => {
                    if current.is_none() && preceding != Previous::Open {
                        return Err(ParseError::ExpectedExpression { span: token.span() });
                    }
                    current = complete_group(&mut frames, current.take(), token, &mut powers)?;
                }
            }
        }
        if previous == Previous::Start {
            return Err(ParseError::EmptyInput);
        }
        let value = current.ok_or_else(|| ParseError::ExpectedExpression {
            span: Span::new(source.len(), source.len()),
        })?;
        let result = reduce(&mut frames, value, 0, &mut powers)?;
        match frames.last() {
            Some(Frame::Parenthesis(opening)) => {
                Err(ParseError::UnclosedParenthesis { span: *opening })
            }
            Some(Frame::Call { header, .. }) => Err(ParseError::UnclosedParenthesis {
                // The opening parenthesis occupies the header's final ASCII byte.
                span: Span::new(header.end().saturating_sub(1), header.end()),
            }),
            Some(Frame::Operator(_)) | None => result.expression(),
        }
    }
}

fn complete_group(
    frames: &mut Vec<Frame>,
    current: Option<Value>,
    token: Token<'_>,
    powers: &mut PowerCache,
) -> Result<Option<Value>, ParseError> {
    let span = token.span();
    let value = current
        .map(|right| reduce(frames, right, 0, powers))
        .transpose()?;
    if matches!(token, Token::Comma(_)) {
        let argument = value
            .ok_or(ParseError::ExpectedExpression { span })?
            .expression()?;
        if let Some(Frame::Call { arguments, .. }) = frames.last_mut() {
            arguments.push(argument);
            return Ok(None);
        }
        return Err(ParseError::UnexpectedToken { span });
    }
    match frames.pop() {
        Some(Frame::Parenthesis(_)) => {
            let grouped = match value.ok_or(ParseError::ExpectedExpression { span })? {
                Value::Sum { factors, ends } => {
                    Value::Expression(Expr::sum_of_products(factors, ends))
                }
                Value::Product { factors, .. } => Value::Expression(Expr::product(factors)),
                single
                @ (Value::Expression(_) | Value::Literal { .. } | Value::NegatedOnly(..)) => single,
            };
            Ok(Some(grouped))
        }
        Some(Frame::Call {
            function,
            mut arguments,
            header,
        }) => {
            if let Some(argument) = value {
                arguments.push(argument.expression()?);
            } else if !arguments.is_empty() {
                return Err(ParseError::ExpectedExpression { span });
            }
            let expression = function
                .call(arguments)
                .map_err(|error| ParseError::Function {
                    span: Span::new(header.start(), span.end()),
                    error,
                })?;
            Ok(Some(Value::Expression(expression)))
        }
        Some(Frame::Operator(_)) => Err(ParseError::ExpectedExpression { span }),
        None => Err(ParseError::UnexpectedToken { span }),
    }
}

/// Apply to `operand` the pending operators that bind more tightly than
/// `operation`, then wait for the right operand of `operation`.
///
/// A pending left-associative operation has only less tightly binding frames
/// below it within its group, so one of equal precedence is the last to apply:
/// it takes the operand in place and continues as `operation`.
fn push_operator(
    frames: &mut Vec<Frame>,
    operand: Value,
    operation: BinaryOp,
    powers: &mut PowerCache,
) -> Result<(), ParseError> {
    let precedence = operation.precedence();
    let left = reduce(frames, operand, precedence, powers)?;
    if operation != BinaryOp::Power
        && let Some(Frame::Operator(Operator::Binary {
            left: pending,
            operation: waiting,
        })) = frames.last_mut()
        && waiting.precedence() == precedence
    {
        pending.absorb(*waiting, left, powers)?;
        *waiting = operation;
    } else {
        frames.push(Frame::Operator(Operator::Binary { left, operation }));
    }
    Ok(())
}

/// Apply the pending operators that bind more tightly than `precedence`,
/// innermost first; zero applies every operator of the innermost group.
fn reduce(
    frames: &mut Vec<Frame>,
    mut right: Value,
    precedence: u8,
    powers: &mut PowerCache,
) -> Result<Value, ParseError> {
    while let Some(Frame::Operator(operator)) = frames.pop_if(|frame| match frame {
        Frame::Operator(Operator::Negative) => NEGATION > precedence,
        Frame::Operator(Operator::Binary { operation, .. }) => operation.precedence() > precedence,
        Frame::Parenthesis(_) | Frame::Call { .. } => false,
    }) {
        right = match operator {
            Operator::Negative => match right {
                Value::NegatedOnly(negated, _) => Value::Expression(negated),
                other @ (Value::Expression(_)
                | Value::Literal { .. }
                | Value::Sum { .. }
                | Value::Product { .. }) => other.negated()?,
            },
            Operator::Binary {
                mut left,
                operation,
            } => {
                left.absorb(operation, right, powers)?;
                left
            }
        };
    }
    Ok(right)
}

impl Default for Value {
    /// The empty product, $1$.
    fn default() -> Self {
        Self::Product {
            leading_negative: None,
            factors: Vec::new(),
        }
    }
}

impl Value {
    /// The value of a literal token. A `_` separator stands between two
    /// digits; the separator-free spelling takes its value from
    /// [`Number::from_decimal`].
    fn number(text: &str, span: Span) -> Result<Self, ParseError> {
        if !text.is_empty() && text.len() <= 18 && text.bytes().all(|byte| byte.is_ascii_digit()) {
            let mut value = 0_i64;
            for byte in text.bytes() {
                value = value
                    .wrapping_mul(10)
                    .wrapping_add(i64::from(byte.wrapping_sub(b'0')));
            }
            return Ok(Self::Expression(Expr::from(value)));
        }
        let literal = if text.contains('_') {
            let mut previous_digit = false;
            let mut bytes = text.bytes().peekable();
            while let Some(byte) = bytes.next() {
                if byte == b'_'
                    && (!previous_digit || bytes.peek().is_none_or(|next| !next.is_ascii_digit()))
                {
                    return Err(ParseError::InvalidNumber { span });
                }
                previous_digit = byte.is_ascii_digit();
            }
            Cow::Owned(text.replace('_', ""))
        } else {
            Cow::Borrowed(text)
        };
        if let Some(value) = Number::from_decimal(&literal, false) {
            return Ok(Self::Expression(Expr::number(value)));
        }
        // Malformed and nonfinite literals fail with either sign; an integer
        // that the backend holds only when negated waits for a unary minus.
        Number::from_decimal(&literal, true)
            .map(|negated| Self::NegatedOnly(Expr::number(negated), span))
            .ok_or(ParseError::InvalidNumber { span })
    }

    /// The bound symbol of `name`, a constant, or a new scoped symbol.
    fn symbol(context: &Context, name: &str, span: Span) -> Result<Self, ParseError> {
        let expression = if let Some(symbol) = context.get_symbol(name) {
            symbol.to_expr()
        } else {
            match name {
                "pi" | "\u{03c0}" => Expr::pi(),
                "e" => Expr::e(),
                "i" => Expr::i(),
                _ => context
                    .symb(name)
                    .map_err(|error| ParseError::Symbol { span, error })?
                    .to_expr(),
            }
        };
        Ok(Self::Expression(expression))
    }

    /// Combine `self` with the right operand of `operation` in place. Sums and
    /// products collect operands for one canonical construction; quotients and
    /// powers are constructed at once. The right operand converts first.
    fn absorb(
        &mut self,
        operation: BinaryOp,
        right: Self,
        powers: &mut PowerCache,
    ) -> Result<(), ParseError> {
        match operation {
            BinaryOp::Add | BinaryOp::Subtract => {
                let addend = if operation == BinaryOp::Subtract {
                    right.negated()?
                } else {
                    right
                };
                if let Self::Sum { factors, ends } = self {
                    addend.push_into_sum(factors, ends)?;
                } else {
                    let mut factors = Vec::with_capacity(SUM_CAPACITY.saturating_mul(2));
                    let mut ends = Vec::with_capacity(SUM_CAPACITY);
                    take(self).push_into_sum(&mut factors, &mut ends)?;
                    addend.push_into_sum(&mut factors, &mut ends)?;
                    *self = Self::Sum { factors, ends };
                }
            }
            BinaryOp::Multiply => {
                let factor = right.expression()?;
                if let Self::Product { factors, .. } = self {
                    factors.push(factor);
                } else {
                    let mut factors = Vec::with_capacity(PRODUCT_CAPACITY);
                    let leading_negative = match take(self) {
                        Self::Literal { positive, negative } => {
                            factors.push(positive);
                            negative
                        }
                        other @ (Self::Expression(_)
                        | Self::Sum { .. }
                        | Self::Product { .. }
                        | Self::NegatedOnly(..)) => {
                            factors.push(other.expression()?);
                            None
                        }
                    };
                    factors.push(factor);
                    *self = Self::Product {
                        leading_negative,
                        factors,
                    };
                }
            }
            BinaryOp::Divide => {
                let divisor = right.expression()?;
                *self = Self::Expression(Div::div(take(self).expression()?, divisor));
            }
            BinaryOp::Power => {
                let exponent = right.expression()?;
                let base = take(self).expression()?;
                *self = Self::Expression(powers.resolve(base, exponent));
            }
        }
        Ok(())
    }

    /// The negation of a literal, or `None` when it has no representable one:
    /// `Neg` wraps the fixed-width minimum, so the sign stays symbolic there.
    pub fn negated_literal(expression: &Expr) -> Option<Expr> {
        expression.as_number()?.checked_neg().map(Expr::number)
    }

    fn negated(self) -> Result<Self, ParseError> {
        match self {
            Self::Literal { positive, negative } => Ok(negative.map_or_else(
                || Self::Product {
                    leading_negative: None,
                    factors: vec![Expr::number(-1), positive],
                },
                Self::Expression,
            )),
            Self::Expression(expression) => Ok(Self::negated_literal(&expression).map_or_else(
                || Self::Product {
                    leading_negative: None,
                    factors: vec![Expr::number(-1), expression],
                },
                Self::Expression,
            )),
            Self::Product {
                leading_negative,
                mut factors,
            } => {
                if let Some(negated) = leading_negative {
                    if let Some(first) = factors.first_mut() {
                        *first = negated;
                    }
                } else {
                    match factors.first_mut() {
                        Some(first) => match Self::negated_literal(first) {
                            Some(negated) => *first = negated,
                            None => factors.insert(0, Expr::number(-1)),
                        },
                        None => factors.push(Expr::number(-1)),
                    }
                }
                Ok(Self::Product {
                    leading_negative: None,
                    factors,
                })
            }
            Self::Sum { .. } | Self::NegatedOnly(..) => {
                Ok(Self::Expression(Neg::neg(self.expression()?)))
            }
        }
    }

    /// Append the factors of `self` as one term of a flat additive chain.
    fn push_into_sum(
        self,
        factors: &mut Vec<Expr>,
        ends: &mut Vec<usize>,
    ) -> Result<(), ParseError> {
        match self {
            Self::Expression(expression)
            | Self::Literal {
                positive: expression,
                ..
            } => factors.push(expression),
            Self::Sum {
                factors: sub_factors,
                ends: sub_ends,
            } => factors.push(Expr::sum_of_products(sub_factors, sub_ends)),
            Self::Product {
                factors: product_factors,
                ..
            } => factors.extend(product_factors),
            Self::NegatedOnly(_, span) => return Err(ParseError::InvalidNumber { span }),
        }
        ends.push(factors.len());
        Ok(())
    }

    fn expression(self) -> Result<Expr, ParseError> {
        match self {
            Self::Expression(expression)
            | Self::Literal {
                positive: expression,
                ..
            } => Ok(expression),
            Self::Sum { factors, ends } => Ok(Expr::sum_of_products(factors, ends)),
            Self::Product { factors, .. } => Ok(Expr::product(factors)),
            Self::NegatedOnly(_, span) => Err(ParseError::InvalidNumber { span }),
        }
    }
}
