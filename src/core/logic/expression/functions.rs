//! One fluent builtin catalogue for expressions, borrowed symbols, and patterns.

use core::ops::Neg;

use alloc::{vec, vec::Vec};

use num_anafis::{Number, RationalType};

use super::{
    BuiltinFunction, Expr, FunctionDefinition, FunctionName, Node, NumericValue, Pattern, Symbol,
};

macro_rules! builtin_methods {
    ($($name:ident => $builtin:ident([$($before:ident),*], [$($after:ident),*])),* $(,)?) => {
        impl Expr {
            $(
                #[doc = concat!("Construct `", stringify!($name), "(", $(stringify!($before), ", ",)* "self", $(", ", stringify!($after),)* ")`, folding proved exact cells and finite approximate inputs.\n\n# Panics\nNumeric folding can propagate the backend's rational capacity panics.")]
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
                #[doc = concat!("Construct `", stringify!($name), "(", $(stringify!($before), ", ",)* "self", $(", ", stringify!($after),)* ")` using this borrowed identity.\n\n# Panics\nNumeric folding can propagate the backend's rational capacity panics.")]
                #[must_use]
                pub fn $name(&self, $($before: impl Into<Expr>,)* $($after: impl Into<Expr>,)*) -> Expr {
                    FunctionDefinition::construct(
                        BuiltinFunction::$builtin.function(),
                        vec![$($before.into(),)* self.to_expr(), $($after.into(),)*],
                    )
                }
            )*
        }

        impl Pattern {
            $(
                #[doc = concat!("Construct `", stringify!($name), "(", $(stringify!($before), ", ",)* "self", $(", ", stringify!($after),)* ")` with captured or exact arguments. Literal calls share expression folding.\n\n# Panics\nNumeric folding can propagate the backend's rational capacity panics.")]
                #[must_use]
                pub fn $name(&self, $($before: impl Into<Self>,)* $($after: impl Into<Self>,)*) -> Self {
                    FunctionDefinition::template(
                        BuiltinFunction::$builtin.function(),
                        vec![$($before.into(),)* self.clone(), $($after.into(),)*],
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
    spherical_harmonic => SphericalHarmonic([degree, order], [phi]),
}

impl Symbol<'_> {
    /// Raise this borrowed symbol to an expression power.
    ///
    /// # Panics
    /// Numeric folding can propagate the backend's rational capacity panics.
    #[must_use]
    pub fn pow(&self, exponent: impl Into<Expr>) -> Expr {
        self.to_expr().pow(exponent)
    }
}

impl FunctionDefinition {
    /// Construct a function after public validation or a fixed-arity builtin helper.
    pub fn construct(function: FunctionName, args: Vec<Expr>) -> Expr {
        if function == BuiltinFunction::Log.function()
            && let [base, argument] = args.as_slice()
            && let Some(alias) = Self::logarithm_alias(base)
        {
            return Self::construct(alias.function(), vec![argument.clone()]);
        }
        if function == BuiltinFunction::ExpNeg.function()
            && let [argument] = args.as_slice()
        {
            return Self::construct(BuiltinFunction::Exp.function(), vec![Neg::neg(argument)]);
        }
        if let [argument] = args.as_slice()
            && matches!(argument.node(), Node::Euler)
        {
            if function == BuiltinFunction::Sqrt.function() {
                return Self::construct(
                    BuiltinFunction::Exp.function(),
                    vec![Expr::number(Number::from_rational(RationalType::new(1, 2)))],
                );
            }
            if function == BuiltinFunction::Ln.function() {
                return Expr::number(1);
            }
        }
        let folded = Self::get(&function).and_then(|definition| match args.as_slice() {
            [argument] => definition.fold(&[argument.literal()?]),
            [first, second] => definition.fold(&[first.literal()?, second.literal()?]),
            [first, second, third] => {
                definition.fold(&[first.literal()?, second.literal()?, third.literal()?])
            }
            [first, second, third, fourth] => definition.fold(&[
                first.literal()?,
                second.literal()?,
                third.literal()?,
                fourth.literal()?,
            ]),
            _ => None,
        });
        if let Some(value) = folded {
            return Expr::number(value);
        }
        if function == BuiltinFunction::Exp.function()
            && let [argument] = args.as_slice()
            && argument.literal().and_then(NumericValue::exact_ratio) == Some((1, 1))
        {
            return Expr::e();
        }
        Expr::intern(Node::Function {
            name: function,
            args,
        })
    }
}
