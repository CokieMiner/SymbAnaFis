/// Token types produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Identifier(String),
    Operator(Operator),
    LeftParen,
    RightParen,
    Comma,
    Derivative {
        order: u32,
        func: String,
        args: String,
        var: String,
    },
}

/// Operator types (arithmetic and built-in functions)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    // Arithmetic
    Add,
    Sub, // NEW: Subtraction
    Mul,
    Div, // NEW: Division
    Pow, // Both ^ and **

    // Trigonometric
    Sin,
    Cos,
    Tan, // NEW
    Cot, // NEW
    Sec, // NEW
    Csc, // NEW

    // Inverse Trigonometric
    Asin, // NEW
    Acos, // NEW
    Atan, // NEW
    Acot, // NEW
    Asec, // NEW
    Acsc, // NEW

    // Logarithmic/Exponential
    Ln,
    Exp,

    // Hyperbolic
    Sinh,
    Cosh,
    Tanh,
    Coth, // NEW (Tier 2 preparation)
    Sech, // NEW (Tier 2 preparation)
    Csch, // NEW (Tier 2 preparation)

    // Inverse Hyperbolic (Tier 2)
    Asinh,
    Acosh,
    Atanh,
    Acoth,
    Asech,
    Acsch,

    // Roots
    Sqrt, // NEW
    Cbrt, // NEW

    // Logarithmic variants (Tier 2)
    Log, // log(x, base) - needs multi-arg support
    Log10,
    Log2,

    // Special (Tier 2)
    Sinc,
    ExpPolar,

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
        matches!(
            self,
            Operator::Sin
                | Operator::Cos
                | Operator::Tan
                | Operator::Cot
                | Operator::Sec
                | Operator::Csc
                | Operator::Asin
                | Operator::Acos
                | Operator::Atan
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
                | Operator::Erf
                | Operator::Erfc
                | Operator::Gamma
                | Operator::Digamma
                | Operator::Trigamma
                | Operator::Tetragamma
                | Operator::Polygamma
                | Operator::Beta
                | Operator::BesselJ
                | Operator::BesselY
                | Operator::BesselI
                | Operator::BesselK
                | Operator::LambertW
                | Operator::Ynm
                | Operator::AssocLegendre
                | Operator::Hermite
                | Operator::EllipticE
                | Operator::EllipticK
        )
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
            "erf" => Some(Operator::Erf),
            "erfc" => Some(Operator::Erfc),
            "gamma" => Some(Operator::Gamma),
            "digamma" => Some(Operator::Digamma),
            "trigamma" => Some(Operator::Trigamma),
            "tetragamma" => Some(Operator::Tetragamma),
            "polygamma" => Some(Operator::Polygamma),
            "beta" => Some(Operator::Beta),
            "besselj" => Some(Operator::BesselJ),
            "bessely" => Some(Operator::BesselY),
            "besseli" => Some(Operator::BesselI),
            "besselk" => Some(Operator::BesselK),
            "LambertW" => Some(Operator::LambertW),
            "Ynm" => Some(Operator::Ynm),
            "assoc_legendre" => Some(Operator::AssocLegendre),
            "hermite" => Some(Operator::Hermite),
            "elliptic_e" => Some(Operator::EllipticE),
            "elliptic_k" => Some(Operator::EllipticK),
            _ => None,
        }
    }

    /// Get the precedence level (higher = binds tighter)
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
            | Operator::Erf
            | Operator::Erfc
            | Operator::Gamma
            | Operator::Digamma
            | Operator::Trigamma
            | Operator::Tetragamma
            | Operator::Polygamma
            | Operator::Beta
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
