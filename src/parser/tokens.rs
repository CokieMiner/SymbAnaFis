//! Token types and operator definitions for the parser
//!
//! Defines [`Token`] for lexer output and [`Operator`] for arithmetic and built-in functions.

/// Token types produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    Number(f64),
    Identifier(String),
    Operator(Operator),
    LeftParen,
    RightParen,
    Comma,
    Derivative {
        order: u32,
        func: String,
        args: Vec<Token>,
        var: String,
    },
}

impl Token {
    /// Convert token to a user-friendly string for error messages
    pub fn to_user_string(&self) -> String {
        match self {
            Token::Number(n) => format!("number '{}'", n),
            Token::Identifier(s) => format!("variable '{}'", s),
            Token::Operator(op) => format!("operator '{}'", op.to_name()),
            Token::LeftParen => "'('".to_string(),
            Token::RightParen => "')'".to_string(),
            Token::Comma => "','".to_string(),
            Token::Derivative {
                func, var, order, ..
            } => {
                format!(
                    "derivative ∂^{}{}({}) / ∂{}^{}",
                    order, func, var, var, order
                )
            }
        }
    }
}

/// Operator types (arithmetic and built-in functions)
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Operator {
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
    pub fn is_function(&self) -> bool {
        self.precedence() == 40
    }

    /// Get the canonical string name for this operator
    ///
    /// This is the inverse of `parse_str` and provides a single source of truth
    /// for Operator → String conversion, used by the Pratt parser and other components.
    pub fn to_name(&self) -> &'static str {
        match self {
            Operator::Add => "+",
            Operator::Sub => "-",
            Operator::Mul => "*",
            Operator::Div => "/",
            Operator::Pow => "^",
            Operator::Sin => "sin",
            Operator::Cos => "cos",
            Operator::Tan => "tan",
            Operator::Cot => "cot",
            Operator::Sec => "sec",
            Operator::Csc => "csc",
            Operator::Asin => "asin",
            Operator::Acos => "acos",
            Operator::Atan => "atan",
            Operator::Atan2 => "atan2",
            Operator::Acot => "acot",
            Operator::Asec => "asec",
            Operator::Acsc => "acsc",
            Operator::Ln => "ln",
            Operator::Exp => "exp",
            Operator::Sinh => "sinh",
            Operator::Cosh => "cosh",
            Operator::Tanh => "tanh",
            Operator::Coth => "coth",
            Operator::Sech => "sech",
            Operator::Csch => "csch",
            Operator::Asinh => "asinh",
            Operator::Acosh => "acosh",
            Operator::Atanh => "atanh",
            Operator::Acoth => "acoth",
            Operator::Asech => "asech",
            Operator::Acsch => "acsch",
            Operator::Sqrt => "sqrt",
            Operator::Cbrt => "cbrt",
            Operator::Log => "log",
            Operator::Log10 => "log10",
            Operator::Log2 => "log2",
            Operator::Sinc => "sinc",
            Operator::ExpPolar => "exp_polar",
            Operator::Abs => "abs",
            Operator::Signum => "signum",
            Operator::Floor => "floor",
            Operator::Ceil => "ceil",
            Operator::Round => "round",
            Operator::Erf => "erf",
            Operator::Erfc => "erfc",
            Operator::Gamma => "gamma",
            Operator::Digamma => "digamma",
            Operator::Trigamma => "trigamma",
            Operator::Tetragamma => "tetragamma",
            Operator::Polygamma => "polygamma",
            Operator::Beta => "beta",
            Operator::Zeta => "zeta",
            Operator::ZetaDeriv => "zeta_deriv",
            Operator::BesselJ => "besselj",
            Operator::BesselY => "bessely",
            Operator::BesselI => "besseli",
            Operator::BesselK => "besselk",
            Operator::LambertW => "lambertw",
            Operator::Ynm => "ynm",
            Operator::AssocLegendre => "assoc_legendre",
            Operator::Hermite => "hermite",
            Operator::EllipticE => "elliptic_e",
            Operator::EllipticK => "elliptic_k",
        }
    }

    /// Convert a string to an operator
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "+" => Some(Operator::Add),
            "-" => Some(Operator::Sub),
            "*" => Some(Operator::Mul),
            "/" => Some(Operator::Div),
            "^" | "**" => Some(Operator::Pow),
            "sin" | "sen" => Some(Operator::Sin), // sen is Portuguese/Spanish alias
            "cos" => Some(Operator::Cos),
            "tan" => Some(Operator::Tan),
            "cot" => Some(Operator::Cot),
            "sec" => Some(Operator::Sec),
            "csc" => Some(Operator::Csc),
            "asin" => Some(Operator::Asin),
            "acos" => Some(Operator::Acos),
            "atan" => Some(Operator::Atan),
            "atan2" => Some(Operator::Atan2),
            "acot" => Some(Operator::Acot),
            "asec" => Some(Operator::Asec),
            "acsc" => Some(Operator::Acsc),
            "ln" => Some(Operator::Ln),
            "exp" => Some(Operator::Exp),
            "sinh" => Some(Operator::Sinh),
            "cosh" => Some(Operator::Cosh),
            "tanh" => Some(Operator::Tanh),
            "coth" => Some(Operator::Coth),
            "sech" => Some(Operator::Sech),
            "csch" => Some(Operator::Csch),
            "asinh" => Some(Operator::Asinh),
            "acosh" => Some(Operator::Acosh),
            "atanh" => Some(Operator::Atanh),
            "acoth" => Some(Operator::Acoth),
            "asech" => Some(Operator::Asech),
            "acsch" => Some(Operator::Acsch),
            "sqrt" => Some(Operator::Sqrt),
            "cbrt" => Some(Operator::Cbrt),
            "log" => Some(Operator::Log),
            "log10" => Some(Operator::Log10),
            "log2" => Some(Operator::Log2),
            "sinc" => Some(Operator::Sinc),
            "exp_polar" => Some(Operator::ExpPolar),
            "abs" => Some(Operator::Abs),
            "sign" | "sgn" | "signum" => Some(Operator::Signum),
            "floor" => Some(Operator::Floor),
            "ceil" => Some(Operator::Ceil),
            "round" => Some(Operator::Round),
            "erf" => Some(Operator::Erf),
            "erfc" => Some(Operator::Erfc),
            "gamma" => Some(Operator::Gamma),
            "digamma" => Some(Operator::Digamma),
            "trigamma" => Some(Operator::Trigamma),
            "tetragamma" => Some(Operator::Tetragamma),
            "polygamma" => Some(Operator::Polygamma),
            "beta" => Some(Operator::Beta),
            "zeta" => Some(Operator::Zeta),
            "zeta_deriv" => Some(Operator::ZetaDeriv),
            "besselj" => Some(Operator::BesselJ),
            "bessely" => Some(Operator::BesselY),
            "besseli" => Some(Operator::BesselI),
            "besselk" => Some(Operator::BesselK),
            "lambertw" => Some(Operator::LambertW),
            "ynm" | "spherical_harmonic" => Some(Operator::Ynm),
            "assoc_legendre" => Some(Operator::AssocLegendre),
            "hermite" => Some(Operator::Hermite),
            "elliptic_e" => Some(Operator::EllipticE),
            "elliptic_k" => Some(Operator::EllipticK),
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
    pub fn precedence(&self) -> u8 {
        match self {
            // Functions (highest precedence) - All Tiers
            Operator::Sin
            | Operator::Cos
            | Operator::Tan
            | Operator::Cot
            | Operator::Sec
            | Operator::Csc
            | Operator::Asin
            | Operator::Acos
            | Operator::Atan
            | Operator::Atan2
            | Operator::Acot
            | Operator::Asec
            | Operator::Acsc
            | Operator::Ln
            | Operator::Exp
            | Operator::Log
            | Operator::Log10
            | Operator::Log2
            | Operator::ExpPolar
            | Operator::Sinh
            | Operator::Cosh
            | Operator::Tanh
            | Operator::Coth
            | Operator::Sech
            | Operator::Csch
            | Operator::Asinh
            | Operator::Acosh
            | Operator::Atanh
            | Operator::Acoth
            | Operator::Asech
            | Operator::Acsch
            | Operator::Sqrt
            | Operator::Cbrt
            | Operator::Sinc
            | Operator::Abs
            | Operator::Signum
            | Operator::Floor
            | Operator::Ceil
            | Operator::Round
            | Operator::Erf
            | Operator::Erfc
            | Operator::Gamma
            | Operator::Digamma
            | Operator::Trigamma
            | Operator::Tetragamma
            | Operator::Polygamma
            | Operator::Beta
            | Operator::Zeta
            | Operator::ZetaDeriv
            | Operator::BesselJ
            | Operator::BesselY
            | Operator::BesselI
            | Operator::BesselK
            | Operator::LambertW
            | Operator::Ynm
            | Operator::AssocLegendre
            | Operator::Hermite
            | Operator::EllipticE
            | Operator::EllipticK => 40,
            Operator::Pow => 30,
            Operator::Mul | Operator::Div => 20,
            Operator::Add | Operator::Sub => 10,
        }
    }

    /// Get the minimum number of arguments required for this function operator
    ///
    /// Returns 0 for arithmetic operators (not applicable).
    /// Returns the minimum arity for functions - some functions accept additional args.
    pub fn min_arity(&self) -> usize {
        match self {
            // Binary functions (require exactly 2 args)
            Operator::Atan2
            | Operator::Polygamma
            | Operator::Beta
            | Operator::ZetaDeriv
            | Operator::BesselJ
            | Operator::BesselY
            | Operator::BesselI
            | Operator::BesselK
            | Operator::Hermite => 2,

            // Ternary functions (require exactly 3 args)
            Operator::AssocLegendre => 3,

            // Quaternary functions (require exactly 4 args)
            Operator::Ynm => 4,

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
        Operator::parse_str(s).ok_or(())
    }
}

#[cfg(test)]
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
