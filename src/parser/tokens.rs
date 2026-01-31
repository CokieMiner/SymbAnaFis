//! Token types and operator definitions for the parser
//!
//! Defines [`Token`] for lexer output and [`Operator`] for arithmetic and built-in functions.

use std::borrow::Cow;

/// Token types produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'src> {
    Number(f64),
    Identifier(Cow<'src, str>),
    Operator(Operator),
    LeftParen,
    RightParen,
    Comma,
    Derivative {
        order: u32,
        func: Cow<'src, str>,
        args: Vec<Self>,
        var: Cow<'src, str>,
    },
}

impl Token<'_> {
    /// Convert token to a user-friendly string for error messages
    pub fn to_user_string(&self) -> String {
        match self {
            Self::Number(n) => format!("number '{n}'"),
            Self::Identifier(s) => format!("variable '{s}'"),
            Self::Operator(op) => format!("operator '{}'", op.to_name()),
            Self::LeftParen => "'('".to_owned(),
            Self::RightParen => "')'".to_owned(),
            Self::Comma => "','".to_owned(),
            Self::Derivative {
                func, var, order, ..
            } => {
                format!("derivative \u{2202}^{order}{func}({var}) / \u{2202}{var}^{order}")
            }
        }
    }
}

/// Operator types (arithmetic and built-in functions)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Pow, // Both ^ and **

    // Trigonometric
    Sin,
    Cos,
    Tan,
    Cot,
    Sec,
    Csc,

    // Inverse Trigonometric
    Asin,
    Acos,
    Atan,
    Atan2,
    Acot,
    Asec,
    Acsc,

    // Logarithmic/Exponential
    Ln,
    Exp,

    // Hyperbolic
    Sinh,
    Cosh,
    Tanh,
    Coth,
    Sech,
    Csch,

    // Inverse Hyperbolic (Tier 2)
    Asinh,
    Acosh,
    Atanh,
    Acoth,
    Asech,
    Acsch,

    // Roots
    Sqrt,
    Cbrt,

    // Logarithmic variants (Tier 2)
    Log, // log(x, base) - needs multi-arg support
    Log10,
    Log2,

    // Special (Tier 2)
    Sinc,
    ExpPolar,

    // Utility Functions
    Abs,
    Signum,
    Floor,
    Ceil,
    Round,

    // Error & Probability (Tier 3)
    Erf,
    Erfc,

    // Gamma functions (Tier 3)
    Gamma,
    Digamma,
    Trigamma,
    Tetragamma,
    Polygamma,
    Beta,
    Zeta,
    ZetaDeriv,

    // Bessel functions (Tier 3)
    BesselJ,
    BesselY,
    BesselI,
    BesselK,

    // Advanced (Tier 3)
    LambertW,
    Ynm,
    AssocLegendre,
    Hermite,
    EllipticE,
    EllipticK,
}

impl Operator {
    /// Check if this operator represents a function (vs arithmetic)
    pub const fn is_function(&self) -> bool {
        self.precedence() == 40
    }

    /// Get the canonical string name for this operator
    ///
    /// This is the inverse of `parse_str` and provides a single source of truth
    /// for Operator → String conversion, used by the Pratt parser and other components.
    pub const fn to_name(&self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Pow => "^",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Cot => "cot",
            Self::Sec => "sec",
            Self::Csc => "csc",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Atan2 => "atan2",
            Self::Acot => "acot",
            Self::Asec => "asec",
            Self::Acsc => "acsc",
            Self::Ln => "ln",
            Self::Exp => "exp",
            Self::Sinh => "sinh",
            Self::Cosh => "cosh",
            Self::Tanh => "tanh",
            Self::Coth => "coth",
            Self::Sech => "sech",
            Self::Csch => "csch",
            Self::Asinh => "asinh",
            Self::Acosh => "acosh",
            Self::Atanh => "atanh",
            Self::Acoth => "acoth",
            Self::Asech => "asech",
            Self::Acsch => "acsch",
            Self::Sqrt => "sqrt",
            Self::Cbrt => "cbrt",
            Self::Log => "log",
            Self::Log10 => "log10",
            Self::Log2 => "log2",
            Self::Sinc => "sinc",
            Self::ExpPolar => "exp_polar",
            Self::Abs => "abs",
            Self::Signum => "signum",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Round => "round",
            Self::Erf => "erf",
            Self::Erfc => "erfc",
            Self::Gamma => "gamma",
            Self::Digamma => "digamma",
            Self::Trigamma => "trigamma",
            Self::Tetragamma => "tetragamma",
            Self::Polygamma => "polygamma",
            Self::Beta => "beta",
            Self::Zeta => "zeta",
            Self::ZetaDeriv => "zeta_deriv",
            Self::BesselJ => "besselj",
            Self::BesselY => "bessely",
            Self::BesselI => "besseli",
            Self::BesselK => "besselk",
            Self::LambertW => "lambertw",
            Self::Ynm => "ynm",
            Self::AssocLegendre => "assoc_legendre",
            Self::Hermite => "hermite",
            Self::EllipticE => "elliptic_e",
            Self::EllipticK => "elliptic_k",
        }
    }

    /// Convert a string to an operator
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "+" => Some(Self::Add),
            "-" => Some(Self::Sub),
            "*" => Some(Self::Mul),
            "/" => Some(Self::Div),
            "^" | "**" => Some(Self::Pow),
            "sin" | "sen" => Some(Self::Sin), // sen is Portuguese/Spanish alias
            "cos" => Some(Self::Cos),
            "tan" => Some(Self::Tan),
            "cot" => Some(Self::Cot),
            "sec" => Some(Self::Sec),
            "csc" => Some(Self::Csc),
            "asin" => Some(Self::Asin),
            "acos" => Some(Self::Acos),
            "atan" => Some(Self::Atan),
            "atan2" => Some(Self::Atan2),
            "acot" => Some(Self::Acot),
            "asec" => Some(Self::Asec),
            "acsc" => Some(Self::Acsc),
            "ln" => Some(Self::Ln),
            "exp" => Some(Self::Exp),
            "sinh" => Some(Self::Sinh),
            "cosh" => Some(Self::Cosh),
            "tanh" => Some(Self::Tanh),
            "coth" => Some(Self::Coth),
            "sech" => Some(Self::Sech),
            "csch" => Some(Self::Csch),
            "asinh" => Some(Self::Asinh),
            "acosh" => Some(Self::Acosh),
            "atanh" => Some(Self::Atanh),
            "acoth" => Some(Self::Acoth),
            "asech" => Some(Self::Asech),
            "acsch" => Some(Self::Acsch),
            "sqrt" => Some(Self::Sqrt),
            "cbrt" => Some(Self::Cbrt),
            "log" => Some(Self::Log),
            "log10" => Some(Self::Log10),
            "log2" => Some(Self::Log2),
            "sinc" => Some(Self::Sinc),
            "exp_polar" => Some(Self::ExpPolar),
            "abs" => Some(Self::Abs),
            "sign" | "sgn" | "signum" => Some(Self::Signum),
            "floor" => Some(Self::Floor),
            "ceil" => Some(Self::Ceil),
            "round" => Some(Self::Round),
            "erf" => Some(Self::Erf),
            "erfc" => Some(Self::Erfc),
            "gamma" => Some(Self::Gamma),
            "digamma" => Some(Self::Digamma),
            "trigamma" => Some(Self::Trigamma),
            "tetragamma" => Some(Self::Tetragamma),
            "polygamma" => Some(Self::Polygamma),
            "beta" => Some(Self::Beta),
            "zeta" => Some(Self::Zeta),
            "zeta_deriv" => Some(Self::ZetaDeriv),
            "besselj" => Some(Self::BesselJ),
            "bessely" => Some(Self::BesselY),
            "besseli" => Some(Self::BesselI),
            "besselk" => Some(Self::BesselK),
            "lambertw" => Some(Self::LambertW),
            "ynm" | "spherical_harmonic" => Some(Self::Ynm),
            "assoc_legendre" => Some(Self::AssocLegendre),
            "hermite" => Some(Self::Hermite),
            "elliptic_e" => Some(Self::EllipticE),
            "elliptic_k" => Some(Self::EllipticK),
            _ => None,
        }
    }

    /// Get the precedence level (higher = binds tighter)
    ///
    /// # Precedence Hierarchy
    ///
    /// | Precedence | Operators | Associativity | Notes |
    /// |------------|-----------|---------------|-------|
    /// | 40 | Functions (sin, cos, ...) | N/A | Function application |
    /// | 30 | `^`, `**` | Right | Exponentiation |
    /// | 25 | Unary `-`, `+` | N/A | Prefix operators |
    /// | 20 | `*`, `/` | Left | Multiplicative |
    /// | 10 | `+`, `-` | Left | Additive |
    ///
    /// # Examples
    /// - `2 + 3 * 4` parses as `2 + (3 * 4)` (mul > add)
    /// - `2^3^4` parses as `2^(3^4)` (right associative)
    /// - `-x^2` parses as `-(x^2)` (pow > unary minus)
    pub const fn precedence(&self) -> u8 {
        match self {
            // Functions (highest precedence) - All Tiers
            Self::Sin
            | Self::Cos
            | Self::Tan
            | Self::Cot
            | Self::Sec
            | Self::Csc
            | Self::Asin
            | Self::Acos
            | Self::Atan
            | Self::Atan2
            | Self::Acot
            | Self::Asec
            | Self::Acsc
            | Self::Ln
            | Self::Exp
            | Self::Log
            | Self::Log10
            | Self::Log2
            | Self::ExpPolar
            | Self::Sinh
            | Self::Cosh
            | Self::Tanh
            | Self::Coth
            | Self::Sech
            | Self::Csch
            | Self::Asinh
            | Self::Acosh
            | Self::Atanh
            | Self::Acoth
            | Self::Asech
            | Self::Acsch
            | Self::Sqrt
            | Self::Cbrt
            | Self::Sinc
            | Self::Abs
            | Self::Signum
            | Self::Floor
            | Self::Ceil
            | Self::Round
            | Self::Erf
            | Self::Erfc
            | Self::Gamma
            | Self::Digamma
            | Self::Trigamma
            | Self::Tetragamma
            | Self::Polygamma
            | Self::Beta
            | Self::Zeta
            | Self::ZetaDeriv
            | Self::BesselJ
            | Self::BesselY
            | Self::BesselI
            | Self::BesselK
            | Self::LambertW
            | Self::Ynm
            | Self::AssocLegendre
            | Self::Hermite
            | Self::EllipticE
            | Self::EllipticK => 40,
            Self::Pow => 30,
            Self::Mul | Self::Div => 20,
            Self::Add | Self::Sub => 10,
        }
    }

    /// Get the minimum number of arguments required for this function operator
    ///
    /// Returns 0 for arithmetic operators (not applicable).
    /// Returns the minimum arity for functions - some functions accept additional args.
    pub const fn min_arity(&self) -> usize {
        match self {
            // Binary functions (require exactly 2 args)
            Self::Atan2
            | Self::Polygamma
            | Self::Beta
            | Self::ZetaDeriv
            | Self::BesselJ
            | Self::BesselY
            | Self::BesselI
            | Self::BesselK
            | Self::Hermite => 2,

            // Ternary functions (require exactly 3 args)
            Self::AssocLegendre => 3,

            // Quaternary functions (require exactly 4 args)
            Self::Ynm => 4,

            // All other functions require 1 argument
            _ if self.is_function() => 1,

            // Arithmetic operators - not applicable
            _ => 0,
        }
    }
}

impl std::str::FromStr for Operator {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s).ok_or(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod tests {
    use super::*;

    #[test]
    fn test_is_function() {
        assert!(Operator::Sin.is_function());
        assert!(Operator::Cos.is_function());
        assert!(!Operator::Add.is_function());
        assert!(!Operator::Mul.is_function());
    }

    #[test]
    fn test_from_str() {
        assert_eq!(Operator::parse_str("+"), Some(Operator::Add));
        assert_eq!(Operator::parse_str("sin"), Some(Operator::Sin));
        assert_eq!(Operator::parse_str("**"), Some(Operator::Pow));
        assert_eq!(Operator::parse_str("invalid"), None);
    }

    #[test]
    fn test_precedence() {
        assert!(Operator::Sin.precedence() > Operator::Pow.precedence());
        assert!(Operator::Pow.precedence() > Operator::Mul.precedence());
        assert!(Operator::Mul.precedence() > Operator::Add.precedence());
    }
}
