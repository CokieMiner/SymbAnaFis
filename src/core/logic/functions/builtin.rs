//! Static builtin identities: spellings, arities, exact literal cells, and sets
//! of builtins.

use num_anafis::{Number, Real};

use super::{Expr, ExprView, NumericValue};

macro_rules! builtin_functions {
    ($($variant:ident => ($name:literal $(| $alias:literal)*, $arity:literal, $unary:expr, $binary:expr)),* $(,)?) => {
        /// A builtin identity independent of text interning and custom namespaces.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(u8)]
        pub enum BuiltinFunction {
            $(#[doc = concat!("The builtin `", $name, "` function.")] $variant,)*
        }

        impl BuiltinFunction {
            /// The number of builtins; every discriminant is below it.
            pub const COUNT: usize = [$(stringify!($variant)),*].len();

            /// Resolve a builtin spelling without allocating or taking a lock.
            #[must_use]
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(Self::$variant), $($alias => Some(Self::$variant),)*)*
                    _ => None,
                }
            }

            /// Borrow the builtin's static display name.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)*
                }
            }

            /// Dispatch directly to an immutable constructor definition.
            #[must_use]
            pub const fn definition(self) -> &'static FunctionDefinition {
                match self {
                    $(Self::$variant => {
                        const DEFINITION: FunctionDefinition =
                            FunctionDefinition::new(BuiltinFunction::$variant, $arity, $unary, $binary);
                        &DEFINITION
                    },)*
                }
            }
        }
    };
}

builtin_functions! {
    Sin => ("sin", 1, Some(UnaryRule::AtZero(0)), None),
    Cos => ("cos", 1, Some(UnaryRule::AtZero(1)), None),
    Tan => ("tan", 1, Some(UnaryRule::AtZero(0)), None),
    Cot => ("cot", 1, None, None),
    Sec => ("sec", 1, Some(UnaryRule::AtZero(1)), None),
    Csc => ("csc", 1, None, None),
    Asin => ("asin", 1, Some(UnaryRule::AtZero(0)), None),
    Acos => ("acos", 1, Some(UnaryRule::AtOneZero), None),
    Atan => ("atan", 1, Some(UnaryRule::AtZero(0)), None),
    Acot => ("acot", 1, None, None),
    Asec => ("asec", 1, Some(UnaryRule::AtOneZero), None),
    Acsc => ("acsc", 1, None, None),
    Sinh => ("sinh", 1, Some(UnaryRule::AtZero(0)), None),
    Cosh => ("cosh", 1, Some(UnaryRule::AtZero(1)), None),
    Tanh => ("tanh", 1, Some(UnaryRule::AtZero(0)), None),
    Coth => ("coth", 1, None, None),
    Sech => ("sech", 1, Some(UnaryRule::AtZero(1)), None),
    Csch => ("csch", 1, None, None),
    Asinh => ("asinh", 1, Some(UnaryRule::AtZero(0)), None),
    Acosh => ("acosh", 1, Some(UnaryRule::AtOneZero), None),
    Atanh => ("atanh", 1, Some(UnaryRule::AtZero(0)), None),
    Acoth => ("acoth", 1, None, None),
    Asech => ("asech", 1, Some(UnaryRule::AtOneZero), None),
    Acsch => ("acsch", 1, None, None),
    Exp => ("exp", 1, Some(UnaryRule::AtZero(1)), None),
    ExpM1 => ("exp_m1", 1, Some(UnaryRule::AtZero(0)), None),
    ExpNeg => ("exp_neg", 1, Some(UnaryRule::AtZero(1)), None),
    Ln => ("ln", 1, Some(UnaryRule::AtOneZero), None),
    Ln1p => ("ln_1p", 1, Some(UnaryRule::AtZero(0)), None),
    Log10 => ("log10", 1, Some(UnaryRule::Log(10)), None),
    Log2 => ("log2", 1, Some(UnaryRule::Log(2)), None),
    Sqrt => ("sqrt", 1, Some(UnaryRule::Sqrt), None),
    Cbrt => ("cbrt", 1, Some(UnaryRule::Cbrt), None),
    Floor => ("floor", 1, Some(UnaryRule::Floor), None),
    Ceil => ("ceil", 1, Some(UnaryRule::Ceil), None),
    Round => ("round", 1, Some(UnaryRule::Round), None),
    Trunc => ("trunc", 1, Some(UnaryRule::Trunc), None),
    Fract => ("fract", 1, Some(UnaryRule::Fract), None),
    Abs => ("abs", 1, Some(UnaryRule::Abs), None),
    Signum => ("signum", 1, Some(UnaryRule::Sign), None),
    Sinc => ("sinc", 1, Some(UnaryRule::AtZero(1)), None),
    Erf => ("erf", 1, Some(UnaryRule::AtZero(0)), None),
    Erfc => ("erfc", 1, Some(UnaryRule::AtZero(1)), None),
    Gamma => ("gamma", 1, Some(UnaryRule::Gamma), None),
    Lgamma => ("lgamma", 1, Some(UnaryRule::Lgamma), None),
    Digamma => ("digamma", 1, None, None),
    Trigamma => ("trigamma", 1, None, None),
    Tetragamma => ("tetragamma", 1, None, None),
    Zeta => ("zeta", 1, None, None),
    EllipticK => ("elliptic_k", 1, None, None),
    EllipticE => ("elliptic_e", 1, None, None),
    Polygamma => ("polygamma", 2, None, None),
    BesselJ => ("besselj" | "bessel_j", 2, None, Some(BinaryRule::Bessel)),
    BesselY => ("bessely" | "bessel_y", 2, None, None),
    BesselI => ("besseli" | "bessel_i", 2, None, Some(BinaryRule::Bessel)),
    BesselK => ("besselk" | "bessel_k", 2, None, None),
    ZetaDeriv => ("zeta_deriv", 2, None, None),
    Log => ("log", 2, None, Some(BinaryRule::Log)),
    Hermite => ("hermite", 2, None, Some(BinaryRule::Hermite)),
    LambertW => ("lambertw" | "lambert_w", 2, None, Some(BinaryRule::LambertW)),
    Beta => ("beta", 2, None, Some(BinaryRule::Beta)),
    Atan2 => ("atan2", 2, None, Some(BinaryRule::Atan2)),
    AssocLegendre => ("assoc_legendre", 3, None, None),
}

const _: () = assert!(
    BuiltinFunction::COUNT <= 128,
    "every builtin discriminant indexes one bit of a u128"
);

/// A builtin's identity, arity, and exact literal cells.
#[derive(Clone, Copy, Debug)]
pub struct FunctionDefinition {
    builtin: BuiltinFunction,
    arity: usize,
    unary: Option<UnaryRule>,
    binary: Option<BinaryRule>,
}

/// Exact unary cells: algebraic identities, not numerical approximations.
#[derive(Clone, Copy, Debug)]
pub enum UnaryRule {
    Floor,
    Ceil,
    Round,
    Trunc,
    Fract,
    Abs,
    Sign,
    Sqrt,
    Cbrt,
    AtZero(i64),
    AtOneZero,
    Lgamma,
    Gamma,
    Log(i64),
}

/// Exact binary cells in the registered argument order.
#[derive(Clone, Copy, Debug)]
pub enum BinaryRule {
    Log,
    Atan2,
    Beta,
    Bessel,
    Hermite,
    LambertW,
}

/// Builtin function identities, one bit per [`BuiltinFunction`] discriminant.
///
/// Rule dispatch tests whether every builtin of a pattern occurs below a node
/// with one mask operation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BuiltinSet(u128);

impl FunctionDefinition {
    /// Define a builtin identity, arity, and optional exact arithmetic cells.
    pub const fn new(
        builtin: BuiltinFunction,
        arity: usize,
        unary: Option<UnaryRule>,
        binary: Option<BinaryRule>,
    ) -> Self {
        Self {
            builtin,
            arity,
            unary,
            binary,
        }
    }

    /// Fold literal arguments of a call with the validated arity.
    ///
    /// Exact arguments fold only through an exact cell whose result is exact; a
    /// float result stays symbolic even when it is integral. Arguments with an
    /// approximate component fold numerically when the result is finite.
    pub fn fold(&self, arguments: &[&NumericValue]) -> Option<Number> {
        let exact = match arguments {
            [argument] => self.unary.and_then(|rule| argument.evaluate(rule)),
            [left, right] => self
                .binary
                .and_then(|rule| left.evaluate_binary(rule, right)),
            _ => None,
        };
        exact
            .filter(Number::is_exact)
            .or_else(|| NumericValue::evaluate_approximate(self.builtin, arguments))
    }

    /// Select the canonical logarithm for a manifest exact base.
    pub fn logarithm_alias(base: &Expr) -> Option<BuiltinFunction> {
        if matches!(base.view(), ExprView::Euler) {
            return Some(BuiltinFunction::Ln);
        }
        match base.as_number()?.components() {
            (Real::Int(2), Real::Int(0)) => Some(BuiltinFunction::Log2),
            (Real::Int(10), Real::Int(0)) => Some(BuiltinFunction::Log10),
            _ => None,
        }
    }

    /// The builtin's fixed argument count.
    pub const fn arity(&self) -> usize {
        self.arity
    }
}

impl BuiltinSet {
    /// Every builtin, the bound for a destination whose calls are unknown.
    pub const ALL: Self = Self(u128::MAX);

    /// The set with `function` added.
    #[must_use]
    pub const fn with(self, function: BuiltinFunction) -> Self {
        #[expect(
            clippy::as_conversions,
            reason = "a `repr(u8)` discriminant converts losslessly"
        )]
        let index = function as u32;
        // `index < COUNT <= 128` by the assertion above, so the shift is in range.
        Self(self.0 | (1 << index))
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether every builtin of `other` is in this set.
    #[must_use]
    pub const fn contains_all(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// The builtin called at the root of `expression`, if it is a builtin call.
    #[must_use]
    pub fn head(expression: &Expr) -> Self {
        if let ExprView::Function { name, .. } = expression.view()
            && let Some(builtin) = BuiltinFunction::of(name)
        {
            Self::default().with(builtin)
        } else {
            Self::default()
        }
    }
}
