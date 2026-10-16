//! Symbolic helpers using exact cells and finite approximate constructor folds.

use alloc::{vec, vec::Vec};

use super::{BuiltinFunction, Expr, FunctionDefinition, FunctionName, Node, Symbol};

impl FunctionDefinition {
    /// Construct a function after public validation or a fixed-arity builtin helper.
    pub fn construct(function: FunctionName, args: Vec<Expr>) -> Expr {
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
        Expr::intern(Node::Function {
            name: function,
            args,
        })
    }
}

macro_rules! unary_functions {
    ($($name:ident => $builtin:ident),* $(,)?) => {
        impl Symbol<'_> {
            $(
                #[doc = concat!("Construct the symbolic `", stringify!($name), "` function using this borrowed identity.")]
                #[must_use]
                pub fn $name(&self) -> Expr { self.to_expr().$name() }
            )*
        }
        impl Expr {
            $(
                #[doc = concat!("Construct the builtin `", stringify!($name), "` function, folding proved integer cells and finite approximate inputs.\n\n# Panics\nNumeric folding can propagate the backend's rational capacity panics.")]
                #[must_use]
                pub fn $name(&self) -> Self {
                    FunctionDefinition::construct(BuiltinFunction::$builtin.function(), vec![self.clone()])
                }
            )*
        }
    };
}

unary_functions! {
    sin => Sin, cos => Cos, tan => Tan, cot => Cot, sec => Sec, csc => Csc,
    asin => Asin, acos => Acos, atan => Atan, acot => Acot, asec => Asec, acsc => Acsc,
    sinh => Sinh, cosh => Cosh, tanh => Tanh, coth => Coth, sech => Sech, csch => Csch,
    asinh => Asinh, acosh => Acosh, atanh => Atanh, acoth => Acoth,
    asech => Asech, acsch => Acsch,
    exp => Exp, exp_m1 => ExpM1, exp_neg => ExpNeg, ln => Ln, ln_1p => Ln1p,
    log10 => Log10, log2 => Log2, sqrt => Sqrt, cbrt => Cbrt,
    floor => Floor, ceil => Ceil, round => Round, trunc => Trunc, fract => Fract,
    abs => Abs, signum => Signum, sinc => Sinc, erf => Erf, erfc => Erfc,
    gamma => Gamma, lgamma => Lgamma, digamma => Digamma, trigamma => Trigamma,
    tetragamma => Tetragamma, zeta => Zeta,
    elliptic_k => EllipticK, elliptic_e => EllipticE,
}

macro_rules! prefix_functions {
    ($($name:ident => $builtin:ident),* $(,)?) => {
        impl Symbol<'_> {
            $(
                #[doc = concat!("Construct `", stringify!($name), "(parameter, self)` using this borrowed identity.")]
                #[must_use]
                pub fn $name(&self, parameter: impl Into<Expr>) -> Expr { self.to_expr().$name(parameter) }
            )*
        }
        impl Expr {
            $(
                #[doc = concat!("Construct `", stringify!($name), "(parameter, self)`, folding proved integer cells and finite approximate inputs.\n\n# Panics\nNumeric folding can propagate the backend's rational capacity panics.")]
                #[must_use]
                pub fn $name(&self, parameter: impl Into<Self>) -> Self {
                    FunctionDefinition::construct(
                        BuiltinFunction::$builtin.function(),
                        vec![parameter.into(), self.clone()],
                    )
                }
            )*
        }
    };
}

prefix_functions! {
    polygamma => Polygamma, besselj => BesselJ,
    bessely => BesselY, besseli => BesselI, besselk => BesselK,
    zeta_deriv => ZetaDeriv, log => Log, hermite => Hermite,
    lambertw => LambertW,
}

macro_rules! forward {
    ($($name:ident($($argument:ident),+)),* $(,)?) => {
        impl Symbol<'_> {
            $(
                #[doc = concat!("Construct `", stringify!($name), "` with this borrowed symbol identity.\n\n# Panics\nNumeric folding can propagate the backend's rational capacity panics.")]
                #[must_use]
                pub fn $name(&self, $($argument: impl Into<Expr>),+) -> Expr { self.to_expr().$name($($argument),+) }
            )*
        }
    };
}

forward! { pow(exponent), beta(other), atan2(x), log_base(base), assoc_legendre(l, m), spherical_harmonic(l, m, phi) }

impl Expr {
    /// Construct the beta function `beta(self, other)`.
    ///
    /// # Panics
    /// Numeric folding can propagate the backend's rational capacity panics.
    #[must_use]
    pub fn beta(&self, other: impl Into<Self>) -> Self {
        FunctionDefinition::construct(
            BuiltinFunction::Beta.function(),
            vec![self.clone(), other.into()],
        )
    }

    /// Construct the angle function `atan2(self, x)`.
    ///
    /// # Panics
    /// Numeric folding can propagate the backend's rational capacity panics.
    #[must_use]
    pub fn atan2(&self, x: impl Into<Self>) -> Self {
        FunctionDefinition::construct(
            BuiltinFunction::Atan2.function(),
            vec![self.clone(), x.into()],
        )
    }

    /// Construct `log_base(self, base)` in the numeric backend's argument order.
    ///
    /// # Panics
    /// Numeric folding can propagate the backend's rational capacity panics.
    #[must_use]
    pub fn log_base(&self, base: impl Into<Self>) -> Self {
        FunctionDefinition::construct(
            BuiltinFunction::LogBase.function(),
            vec![self.clone(), base.into()],
        )
    }

    /// Construct `assoc_legendre(l, m, self)`.
    ///
    /// # Panics
    /// Numeric folding can propagate the backend's rational capacity panics.
    #[must_use]
    pub fn assoc_legendre(&self, l: impl Into<Self>, m: impl Into<Self>) -> Self {
        FunctionDefinition::construct(
            BuiltinFunction::AssocLegendre.function(),
            vec![l.into(), m.into(), self.clone()],
        )
    }

    /// Construct `spherical_harmonic(l, m, self, phi)`.
    ///
    /// # Panics
    /// Numeric folding can propagate the backend's rational capacity panics.
    #[must_use]
    pub fn spherical_harmonic(
        &self,
        l: impl Into<Self>,
        m: impl Into<Self>,
        phi: impl Into<Self>,
    ) -> Self {
        FunctionDefinition::construct(
            BuiltinFunction::SphericalHarmonic.function(),
            vec![l.into(), m.into(), self.clone(), phi.into()],
        )
    }
}
