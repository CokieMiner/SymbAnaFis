//! Public template constructors, builtin function methods, arithmetic
//! operators with exact conversion of concrete operands, and bounded debug
//! summaries.

use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::{Add, Div, Mul, Neg, Sub},
};

use alloc::{sync::Arc, vec, vec::Vec};

use num_anafis::Number;

use super::{
    Arithmetic, BuiltinFunction, CapturePredicate, Expr, Function, FunctionDefinition,
    FunctionError, Pattern, PatternNode, Replacement, ReplacementProgram, RewriteRule, Symbol,
};

/// The root shape of a template, so formatting a deep template uses constant
/// stack depth.
struct RootSummary<'pattern>(&'pattern PatternNode);

/// The builtin methods of patterns: a copy of the expression method table.
/// The build fails when an entry is not its builtin's method form
/// ([`BuiltinFunction::is_method`]) or when a builtin is missing or repeated.
macro_rules! pattern_methods {
    ($($name:ident => $builtin:ident([$($before:ident),*], [$($after:ident),*])),* $(,)?) => {
        impl Pattern {
            $(
                #[doc = concat!("Construct `", stringify!($name), "(", $(stringify!($before), ", ",)* "self", $(", ", stringify!($after),)* ")` as a pattern. Literal arguments fold as in expressions.")]
                #[must_use]
                pub fn $name(&self, $($before: impl Into<Self>,)* $($after: impl Into<Self>,)*) -> Self {
                    FunctionDefinition::template(
                        BuiltinFunction::$builtin.function(),
                        vec![$($before.into(),)* self.clone(), $($after.into(),)*],
                    )
                }
            )*
        }

        const _: () = {
            $(assert!(
                BuiltinFunction::$builtin.is_method(
                    stringify!($name),
                    <[&str]>::len(&[$(stringify!($before)),*]),
                    <[&str]>::len(&[$(stringify!($after)),*]),
                ),
                concat!("`Pattern::", stringify!($name), "` differs from the expression method table"),
            );)*
            match BuiltinFunction::Sin {
                $(BuiltinFunction::$builtin)|* => {}
            }
        };
    };
}

pattern_methods! {
    sin => Sin([], []), cos => Cos([], []), tan => Tan([], []),
    cot => Cot([], []), sec => Sec([], []), csc => Csc([], []),
    asin => Asin([], []), acos => Acos([], []), atan => Atan([], []),
    acot => Acot([], []), asec => Asec([], []), acsc => Acsc([], []),
    sinh => Sinh([], []), cosh => Cosh([], []), tanh => Tanh([], []),
    coth => Coth([], []), sech => Sech([], []), csch => Csch([], []),
    asinh => Asinh([], []), acosh => Acosh([], []), atanh => Atanh([], []),
    acoth => Acoth([], []), asech => Asech([], []), acsch => Acsch([], []),
    exp => Exp([], []), exp_m1 => ExpM1([], []), exp_neg => ExpNeg([], []),
    ln => Ln([], []), ln_1p => Ln1p([], []), log10 => Log10([], []), log2 => Log2([], []),
    sqrt => Sqrt([], []), cbrt => Cbrt([], []),
    floor => Floor([], []), ceil => Ceil([], []), round => Round([], []),
    trunc => Trunc([], []), fract => Fract([], []),
    abs => Abs([], []), signum => Signum([], []), sinc => Sinc([], []),
    erf => Erf([], []), erfc => Erfc([], []), gamma => Gamma([], []),
    lgamma => Lgamma([], []), digamma => Digamma([], []), trigamma => Trigamma([], []),
    tetragamma => Tetragamma([], []), zeta => Zeta([], []),
    elliptic_k => EllipticK([], []), elliptic_e => EllipticE([], []),
    polygamma => Polygamma([order], []), besselj => BesselJ([order], []),
    bessely => BesselY([order], []), besseli => BesselI([order], []),
    besselk => BesselK([order], []), zeta_deriv => ZetaDeriv([order], []),
    log => Log([base], []), hermite => Hermite([degree], []),
    lambertw => LambertW([branch], []),
    beta => Beta([], [other]), atan2 => Atan2([], [x]),
    assoc_legendre => AssocLegendre([degree, order], []),
}

impl Pattern {
    /// A capture named `name`, accepting a subexpression when the optional
    /// predicate holds.
    ///
    /// Repeated names bind the same expression and enforce all their
    /// predicates, also after arithmetic normalization collects occurrences. A
    /// replacement reads the bound value. The predicate must be pure and give
    /// the same answer for the same expression, since the search may evaluate it
    /// more than once; its panics propagate.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use symb_anafis::{Expr, Pattern, Property, RewriteRule, Truth};
    /// let a = Pattern::capture("a", Some(Arc::new(|node| {
    ///     node.properties().get(Property::Real) == Truth::True
    /// })));
    /// let rule = RewriteRule::new(a.pow(2).sqrt(), a.abs())?;
    /// let x = Expr::symbol("predicate_example_x")?;
    /// assert_eq!(rule.apply_all(&x.pow(2).sqrt(), usize::MAX, None)?, x.pow(2).sqrt());
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    #[must_use]
    pub fn capture(name: impl AsRef<str>, predicate: Option<Arc<CapturePredicate>>) -> Self {
        Self::from_node(PatternNode::Capture {
            name: Arc::from(name.as_ref()),
            predicates: predicate.into_iter().collect(),
        })
    }

    /// A pattern that matches exactly `expression`.
    #[must_use]
    pub fn exact(expression: impl Into<Expr>) -> Self {
        Self::from_node(PatternNode::Exact(expression.into()))
    }

    /// The call template `function(args)`.
    ///
    /// # Errors
    /// Returns [`FunctionError::InvalidArity`] when `args.len()` is outside the
    /// arity.
    pub fn apply(function: Function, args: Vec<Self>) -> Result<Self, FunctionError> {
        FunctionDefinition::validate(&function, args.len())?;
        Ok(FunctionDefinition::template(function, args))
    }

    /// A normalized sum template. At a rule's root unmatched terms are kept; a
    /// nested sum must match its complete operand list.
    #[must_use]
    pub fn sum(terms: Vec<Self>) -> Self {
        PatternNode::normalize(Arithmetic::Sum(terms))
    }

    /// A normalized product template. At a rule's root unmatched factors are
    /// kept; a nested product must match its complete operand list.
    #[must_use]
    pub fn product(factors: Vec<Self>) -> Self {
        PatternNode::normalize(Arithmetic::Product(factors))
    }

    /// This template raised to `exponent`, normalized as an expression.
    #[must_use]
    pub fn pow(&self, exponent: impl Into<Self>) -> Self {
        PatternNode::normalize(Arithmetic::Pow(self.clone(), exponent.into()))
    }
}

impl FunctionDefinition {
    /// A validated call template. Builtin aliases normalize, and a call whose
    /// arguments are all exact becomes an exact expression.
    pub fn template(function: Function, args: Vec<Pattern>) -> Pattern {
        if function == BuiltinFunction::Log.function()
            && let [base, argument] = args.as_slice()
            && let PatternNode::Exact(expression) = base.node()
            && let Some(alias) = Self::logarithm_alias(expression)
        {
            return Self::template(alias.function(), vec![argument.clone()]);
        }
        if function == BuiltinFunction::ExpNeg.function()
            && let [argument] = args.as_slice()
        {
            return Self::template(BuiltinFunction::Exp.function(), vec![Neg::neg(argument)]);
        }
        let exact: Option<Vec<_>> = args
            .iter()
            .map(|argument| match argument.node() {
                PatternNode::Exact(expression) => Some(expression.clone()),
                PatternNode::Capture { .. }
                | PatternNode::Function { .. }
                | PatternNode::Sum(_)
                | PatternNode::Product(_)
                | PatternNode::Div(..)
                | PatternNode::Pow(..) => None,
            })
            .collect();
        if let Some(values) = exact {
            return Pattern::exact(Self::construct(function, values));
        }
        Pattern::from_node(PatternNode::Function {
            name: function,
            args,
        })
    }
}

macro_rules! pattern_operators {
    ($left:ty) => {
        impl<Right: Into<Pattern>> Add<Right> for $left {
            type Output = Pattern;
            fn add(self, right: Right) -> Pattern {
                PatternNode::normalize(Arithmetic::Sum(vec![self.into(), right.into()]))
            }
        }
        impl<Right: Into<Pattern>> Sub<Right> for $left {
            type Output = Pattern;
            fn sub(self, right: Right) -> Pattern {
                PatternNode::normalize(Arithmetic::Sub(self.into(), right.into()))
            }
        }
        impl<Right: Into<Pattern>> Mul<Right> for $left {
            type Output = Pattern;
            fn mul(self, right: Right) -> Pattern {
                PatternNode::normalize(Arithmetic::Product(vec![self.into(), right.into()]))
            }
        }
        impl<Right: Into<Pattern>> Div<Right> for $left {
            type Output = Pattern;
            fn div(self, right: Right) -> Pattern {
                PatternNode::normalize(Arithmetic::Div(self.into(), right.into()))
            }
        }
    };
}

pattern_operators!(Pattern);
pattern_operators!(&Pattern);

macro_rules! concrete_operators {
    ($left:ty, $right:ty) => {
        impl Add<$right> for $left {
            type Output = Pattern;
            fn add(self, right: $right) -> Pattern {
                PatternNode::normalize(Arithmetic::Sum(vec![self.into(), right.into()]))
            }
        }
        impl Sub<$right> for $left {
            type Output = Pattern;
            fn sub(self, right: $right) -> Pattern {
                PatternNode::normalize(Arithmetic::Sub(self.into(), right.into()))
            }
        }
        impl Mul<$right> for $left {
            type Output = Pattern;
            fn mul(self, right: $right) -> Pattern {
                PatternNode::normalize(Arithmetic::Product(vec![self.into(), right.into()]))
            }
        }
        impl Div<$right> for $left {
            type Output = Pattern;
            fn div(self, right: $right) -> Pattern {
                PatternNode::normalize(Arithmetic::Div(self.into(), right.into()))
            }
        }
    };
}

macro_rules! concrete_operands {
    ($($operand:ty),+ $(,)?) => {
        $(
            impl From<$operand> for Pattern {
                fn from(value: $operand) -> Self {
                    Self::exact(value)
                }
            }
            concrete_operators!($operand, Pattern);
            concrete_operators!($operand, &Pattern);
        )+
    };
}

concrete_operands!(
    Expr,
    &Expr,
    Symbol<'_>,
    &Symbol<'_>,
    i32,
    i64,
    f64,
    Number,
    &Number
);

impl From<&Self> for Pattern {
    fn from(pattern: &Self) -> Self {
        pattern.clone()
    }
}

impl Neg for Pattern {
    type Output = Self;
    fn neg(self) -> Self {
        PatternNode::normalize(Arithmetic::Product(vec![Self::exact(-1), self]))
    }
}

impl Neg for &Pattern {
    type Output = Pattern;
    fn neg(self) -> Pattern {
        PatternNode::normalize(Arithmetic::Product(vec![Pattern::exact(-1), self.clone()]))
    }
}

impl Debug for Pattern {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("Pattern")
            .field("root", &RootSummary(self.node()))
            .finish()
    }
}

impl Debug for RewriteRule {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        let (operations, callback, concrete) = match &self.replacement {
            ReplacementProgram::Template(template) => {
                (template.operands.len().saturating_add(1), false, false)
            }
            ReplacementProgram::Callback(_) => (0, true, false),
            ReplacementProgram::Concrete(_) => (0, false, true),
            ReplacementProgram::ConcreteCallback { .. } => (0, true, true),
        };
        formatter
            .debug_struct("RewriteRule")
            .field("concrete", &concrete)
            .field("pattern", &RootSummary(self.pattern.node()))
            .field("compiled_operations", &operations)
            .field("callback", &callback)
            .field("associative", &self.associative)
            .field("heads", &self.heads)
            .field("counted", &self.counted)
            .field("grounds", &self.grounds.len())
            .finish()
    }
}

impl Debug for Replacement {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Pattern(pattern) => formatter
                .debug_tuple("Pattern")
                .field(&RootSummary(pattern.node()))
                .finish(),
            Self::Callback(_) => formatter.write_str("Callback(..)"),
        }
    }
}

impl Debug for RootSummary<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self.0 {
            PatternNode::Capture { name, predicates } => formatter
                .debug_struct("Capture")
                .field("name", name)
                .field("predicates", &predicates.len())
                .finish(),
            PatternNode::Exact(_) => formatter.write_str("Exact(..)"),
            PatternNode::Function { name, args } => formatter
                .debug_struct("Function")
                .field("name", &name.as_str())
                .field("arity", &args.len())
                .finish(),
            PatternNode::Sum(args) => formatter
                .debug_struct("Sum")
                .field("arity", &args.len())
                .finish(),
            PatternNode::Product(args) => formatter
                .debug_struct("Product")
                .field("arity", &args.len())
                .finish(),
            PatternNode::Div(..) => formatter.write_str("Div"),
            PatternNode::Pow(..) => formatter.write_str("Pow"),
        }
    }
}
