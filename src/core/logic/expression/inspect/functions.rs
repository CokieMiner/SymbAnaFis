//! Builtin function methods for expressions and symbols.

use core::ops::Neg;

use alloc::{vec, vec::Vec};

use num_anafis::{Number, RationalType};

use super::{BuiltinFunction, ExactIdentity, Expr, Function, FunctionDefinition, Node, Symbol};

/// The builtin methods: each method name, its builtin, and the names of the
/// arguments before and after the receiver. The pattern methods repeat this
/// table and are checked against [`BuiltinFunction::is_method`] at compile
/// time.
macro_rules! builtin_methods {
    ($($name:ident => $builtin:ident([$($before:ident),*], [$($after:ident),*])),* $(,)?) => {
        impl BuiltinFunction {
            /// Whether `name`, with `before` arguments ahead of the receiver
            /// and `after` behind it, is this builtin's method form. Every
            /// builtin has exactly one.
            pub const fn is_method(self, name: &str, before: usize, after: usize) -> bool {
                let (expected, expected_before, expected_after) = match self {
                    $(Self::$builtin => (
                        stringify!($name),
                        <[&str]>::len(&[$(stringify!($before)),*]),
                        <[&str]>::len(&[$(stringify!($after)),*]),
                    ),)*
                };
                if before != expected_before || after != expected_after {
                    return false;
                }
                let (mut left, mut right) = (name.as_bytes(), expected.as_bytes());
                loop {
                    match (left, right) {
                        ([], []) => return true,
                        ([first, rest @ ..], [second, others @ ..]) if *first == *second => {
                            left = rest;
                            right = others;
                        }
                        _ => return false,
                    }
                }
            }
        }

        // The receiver and the named arguments fill the builtin's arity.
        const _: () = {
            $(assert!(
                BuiltinFunction::$builtin.definition().arity()
                    == 1 + <[&str]>::len(&[$(stringify!($before)),*])
                        + <[&str]>::len(&[$(stringify!($after)),*]),
                concat!("the method `", stringify!($name), "` does not match its builtin's arity"),
            );)*
        };

        impl Expr {
            $(
                #[doc = concat!("Construct `", stringify!($name), "(", $(stringify!($before), ", ",)* "self", $(", ", stringify!($after),)* ")`. Literal arguments fold through an exact cell, or numerically when an argument is approximate and the result is finite.")]
                #[must_use]
                pub fn $name(&self, $($before: impl Into<Self>,)* $($after: impl Into<Self>,)*) -> Self {
                    FunctionDefinition::construct(
                        BuiltinFunction::$builtin.function(),
                        vec![$($before.into(),)* self.clone(), $($after.into(),)*],
                    )
                }
            )*
        }

        impl Symbol<'_> {
            $(
                #[doc = concat!("Construct `", stringify!($name), "(", $(stringify!($before), ", ",)* "self", $(", ", stringify!($after),)* ")` for this symbol.")]
                #[must_use]
                pub fn $name(&self, $($before: impl Into<Expr>,)* $($after: impl Into<Expr>,)*) -> Expr {
                    FunctionDefinition::construct(
                        BuiltinFunction::$builtin.function(),
                        vec![$($before.into(),)* self.to_expr(), $($after.into(),)*],
                    )
                }
            )*
        }
    };
}

builtin_methods! {
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

impl Symbol<'_> {
    /// This symbol raised to `exponent`.
    #[must_use]
    pub fn pow(&self, exponent: impl Into<Expr>) -> Expr {
        self.to_expr().pow(exponent)
    }
}

impl FunctionDefinition {
    /// Construct a validated call, applying the builtin's canonical aliases and
    /// exact literal cells. Custom functions are stored unchanged.
    pub fn construct(function: Function, args: Vec<Expr>) -> Expr {
        let Some(builtin) = BuiltinFunction::of(&function) else {
            return Expr::intern(Node::Function {
                name: function,
                args,
            });
        };
        match (builtin, args.as_slice()) {
            (BuiltinFunction::Log, [base, argument]) => {
                if let Some(alias) = Self::logarithm_alias(base) {
                    return Self::construct(alias.function(), vec![argument.clone()]);
                }
            }
            (BuiltinFunction::ExpNeg, [argument]) => {
                return Self::construct(BuiltinFunction::Exp.function(), vec![Neg::neg(argument)]);
            }
            (BuiltinFunction::Sqrt, [argument]) if matches!(argument.node(), Node::Euler) => {
                return Self::construct(
                    BuiltinFunction::Exp.function(),
                    vec![Expr::number(Number::from_rational(RationalType::new(1, 2)))],
                );
            }
            (BuiltinFunction::Ln, [argument]) if matches!(argument.node(), Node::Euler) => {
                return Expr::number(1);
            }
            (BuiltinFunction::Exp, [argument]) if argument.exact_ratio() == Some((1, 1)) => {
                return Expr::e();
            }
            _ => {}
        }
        if let Some(value) = builtin.definition().fold_literals(&args) {
            return Expr::number(value);
        }
        Expr::intern(Node::Function {
            name: function,
            args,
        })
    }

    /// Fold a call whose arguments are all literals.
    fn fold_literals(&self, args: &[Expr]) -> Option<Number> {
        match args {
            [argument] => self.fold(&[argument.literal()?]),
            [first, second] => self.fold(&[first.literal()?, second.literal()?]),
            [first, second, third] => {
                self.fold(&[first.literal()?, second.literal()?, third.literal()?])
            }
            _ => None,
        }
    }
}
