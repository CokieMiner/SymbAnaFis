//! Iterative precedence parsing with batched additive and multiplicative chains.

use core::ops::{Div, Neg};

use alloc::{borrow::Cow, vec, vec::Vec};

use super::{BinaryOp, Context, Expr, Function, Lexer, NumberKind, ParseError, Span, Token};

// Reuse common bindings with fixed storage and no heap allocation.
const CACHED_SYMBOLS: usize = 4;

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

enum Value {
    Expression(Expr),
    Sum(Vec<Expr>),
    Product(Vec<Expr>),
    MinimumMagnitude(Span),
}

impl Context {
    /// Parse a scalar expression using this context's symbol and function bindings.
    ///
    /// Integer literals use exact `i64` values. Decimal and scientific literals
    /// use finite `f64` values. Operators are `+`, `-`, `*`, `/`, and right-associative
    /// `^` or `**`; powers bind more tightly than unary signs: \(-x^2=-(x^2)\).
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
    /// # Panics
    /// Canonical construction can propagate numeric backend capacity panics.
    ///
    /// ```
    /// use symb_anafis::{Context, Expr, UserFunction};
    /// let context = Context::new();
    /// let f = context.register_function("f", UserFunction::new(1..=1))?;
    /// let parsed = context.parse("2x + sin(x)^2 + f(x)")?;
    /// let x = context.symb("x")?;
    /// assert_eq!(parsed, 2 * x + x.sin().pow(2) + f.call(vec![x.into()])?);
    /// assert_eq!(context.parse("1/4")?, Expr::number(1) / 4);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn parse(&self, source: &str) -> Result<Expr, ParseError> {
        let mut tokens = Lexer::new(source).peekable();
        if tokens.peek().is_none() {
            return Err(ParseError::EmptyInput);
        }
        let mut frames = Vec::new();
        let mut symbols = [const { None }; CACHED_SYMBOLS];
        let mut current = None;
        let mut previous = None;
        while let Some(item) = tokens.next() {
            let token = item?;
            let preceding = previous;
            previous = Some(token);
            if matches!(token, Token::Number(..) | Token::Name(..) | Token::Open(_))
                && let Some(left) = current.take()
            {
                if matches!(preceding, Some(Token::Number(..)))
                    && matches!(token, Token::Number(..))
                {
                    return Err(ParseError::UnexpectedToken { span: token.span() });
                }
                let reduced = reduce(&mut frames, left, Some(BinaryOp::Multiply))?;
                frames.push(Frame::Operator(Operator::Binary {
                    left: reduced,
                    operation: BinaryOp::Multiply,
                }));
            }
            match token {
                Token::Number(text, span, kind) => current = Some(Value::number(text, span, kind)?),
                Token::Name(name, span) => {
                    let following = tokens.peek();
                    let reuse = following.is_some();
                    let resolved = if matches!(following, Some(Ok(Token::Open(_)))) {
                        Function::builtin(name).or_else(|| self.get_function(name))
                    } else {
                        None
                    };
                    if let Some(function) = resolved
                        && let Some(Ok(Token::Open(opening))) =
                            tokens.next_if(|next| matches!(next, Ok(Token::Open(_))))
                    {
                        previous = Some(Token::Open(opening));
                        frames.push(Frame::Call {
                            function,
                            arguments: Vec::new(),
                            header: Span::new(span.start(), opening.end()),
                        });
                    } else {
                        current = Some(Value::symbol(self, &mut symbols, name, span, reuse)?);
                    }
                }
                Token::Open(opening) => frames.push(Frame::Parenthesis(opening)),
                Token::Operator(operation, span) => {
                    if let Some(left) = current.take() {
                        let reduced = reduce(&mut frames, left, Some(operation))?;
                        frames.push(Frame::Operator(Operator::Binary {
                            left: reduced,
                            operation,
                        }));
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
                    if current.is_none() && !matches!(preceding, Some(Token::Open(_))) {
                        return Err(ParseError::ExpectedExpression { span: token.span() });
                    }
                    current = complete_group(&mut frames, current.take(), token)?;
                }
            }
        }
        let value = current.ok_or_else(|| ParseError::ExpectedExpression {
            span: Span::new(source.len(), source.len()),
        })?;
        let result = reduce(&mut frames, value, None)?;
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
) -> Result<Option<Value>, ParseError> {
    let span = token.span();
    let value = current
        .map(|right| reduce(frames, right, None))
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
                Value::Sum(terms) => Value::Expression(Expr::sum(terms)),
                Value::Product(factors) => Value::Expression(Expr::product(factors)),
                single @ (Value::Expression(_) | Value::MinimumMagnitude(_)) => single,
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

fn reduce(
    frames: &mut Vec<Frame>,
    mut right: Value,
    next: Option<BinaryOp>,
) -> Result<Value, ParseError> {
    while let Some(Frame::Operator(operator)) = frames.pop_if(|frame| match frame {
        Frame::Parenthesis(_) | Frame::Call { .. } => false,
        Frame::Operator(operator) => {
            let precedence = match operator {
                Operator::Negative => 3,
                Operator::Binary { operation, .. } => operation.precedence(),
            };
            next.is_none_or(|incoming| {
                precedence > incoming.precedence()
                    || (precedence == incoming.precedence() && incoming != BinaryOp::Power)
            })
        }
    }) {
        right = match operator {
            Operator::Negative => Value::Expression(match right {
                Value::MinimumMagnitude(_) => Expr::number(i64::MIN),
                other @ (Value::Expression(_) | Value::Sum(_) | Value::Product(_)) => {
                    Neg::neg(other.expression()?)
                }
            }),
            Operator::Binary { left, operation } => left.binary(operation, right)?,
        };
    }
    Ok(right)
}

impl Value {
    fn number(text: &str, span: Span, kind: NumberKind) -> Result<Self, ParseError> {
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
        if kind == NumberKind::Float {
            return literal
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(|value| Self::Expression(Expr::number(value)))
                .ok_or(ParseError::InvalidNumber { span });
        }
        if let Ok(integer) = literal.parse::<i64>() {
            return Ok(Self::Expression(Expr::number(integer)));
        }
        if literal.parse::<u64>() == Ok(i64::MIN.unsigned_abs()) {
            return Ok(Self::MinimumMagnitude(span));
        }
        Err(ParseError::InvalidNumber { span })
    }

    fn symbol<'source>(
        context: &Context,
        symbols: &mut [Option<(&'source str, Expr)>; CACHED_SYMBOLS],
        name: &'source str,
        span: Span,
        reuse: bool,
    ) -> Result<Self, ParseError> {
        let mut vacant = None;
        // Occupied entries form a prefix; the first empty slot ends the search.
        for binding in symbols {
            match binding {
                Some((bound, expression)) => {
                    if *bound == name {
                        return Ok(Self::Expression(expression.clone()));
                    }
                }
                empty @ None => {
                    vacant = Some(empty);
                    break;
                }
            }
        }
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
        if reuse && let Some(slot) = vacant {
            *slot = Some((name, expression.clone()));
        }
        Ok(Self::Expression(expression))
    }

    fn binary(self, operation: BinaryOp, right: Self) -> Result<Self, ParseError> {
        let operand = right.expression()?;
        match operation {
            BinaryOp::Add | BinaryOp::Subtract => {
                let addend = if operation == BinaryOp::Subtract {
                    Neg::neg(operand)
                } else {
                    operand
                };
                let mut terms = match self {
                    Self::Sum(terms) => terms,
                    other
                    @ (Self::Expression(_) | Self::Product(_) | Self::MinimumMagnitude(_)) => {
                        vec![other.expression()?]
                    }
                };
                terms.push(addend);
                Ok(Self::Sum(terms))
            }
            BinaryOp::Multiply => {
                let mut factors = match self {
                    Self::Product(factors) => factors,
                    other @ (Self::Expression(_) | Self::Sum(_) | Self::MinimumMagnitude(_)) => {
                        vec![other.expression()?]
                    }
                };
                factors.push(operand);
                Ok(Self::Product(factors))
            }
            BinaryOp::Divide => Ok(Self::Expression(Div::div(self.expression()?, operand))),
            BinaryOp::Power => Ok(Self::Expression(self.expression()?.pow(operand))),
        }
    }

    fn expression(self) -> Result<Expr, ParseError> {
        match self {
            Self::Expression(expression) => Ok(expression),
            Self::Sum(terms) => Ok(Expr::sum(terms)),
            Self::Product(factors) => Ok(Expr::product(factors)),
            Self::MinimumMagnitude(span) => Err(ParseError::InvalidNumber { span }),
        }
    }
}
