//! Bytecode instruction set for the expression evaluator.
//!
//! This module defines the [`Instruction`] enum representing all operations
//! that the compiled evaluator can perform. Instructions are organized into
//! logical categories for maintainability.
//!
//! # Instruction Categories
//!
//! | Category | Description | Examples |
//! |----------|-------------|----------|
//! | Memory | Load/store operations | `LoadConst`, `LoadParam`, `LoadCached` |
//! | Arithmetic | Basic math operations | `Add`, `Mul`, `Div`, `Neg`, `Pow` |
//! | Trigonometric | Trig functions | `Sin`, `Cos`, `Tan`, `Asin`, etc. |
//! | Hyperbolic | Hyperbolic functions | `Sinh`, `Cosh`, `Tanh`, etc. |
//! | Exponential | Exp/log functions | `Exp`, `Ln`, `Log10`, `Sqrt` |
//! | Special | Special math functions | `Erf`, `Gamma`, `BesselJ`, etc. |
//! | Fused | Performance optimizations | `Square`, `Cube`, `MulAdd` |
//!
//! # Stack Machine Model
//!
//! The evaluator uses a stack-based virtual machine:
//! - `Load*` instructions push values onto the stack
//! - Unary operations pop one value, compute, and push the result
//! - Binary operations pop two values, compute, and push the result
//! - The final result is the single value remaining on the stack

/// Bytecode instruction for the stack-based expression evaluator.
///
/// Each instruction operates on a stack of `f64` values. The compiler
/// ensures that the stack never underflows and that the final result
/// is always a single value.
///
/// # Memory Layout
///
/// Most variants are zero-sized or contain small payloads (u32, i8),
/// keeping the enum at 8 bytes for efficient instruction dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    // =========================================================================
    // Memory Operations
    // =========================================================================
    /// Push a constant from the constant pool onto the stack.
    ///
    /// The index refers to the evaluator's `constants` array.
    LoadConst(u32),

    /// Push a parameter value onto the stack.
    ///
    /// The index refers to the parameter order passed to `compile()`.
    LoadParam(u32),

    /// Duplicate the top of stack (for CSE - Common Subexpression Elimination).
    Dup,

    /// Store the top of stack to a cache slot (does NOT pop).
    ///
    /// Used by CSE to save expensive subexpression results.
    StoreCached(u32),

    /// Load a value from a cache slot onto the stack.
    ///
    /// Used by CSE to reuse previously computed subexpressions.
    LoadCached(u32),

    /// Pop the top value from the stack and discard it.
    Pop,

    /// Swap the top two values on the stack: `[a, b] -> [b, a]`.
    Swap,

    // =========================================================================
    // Core Arithmetic Operations
    // =========================================================================
    /// Pop two values, push their sum: `[a, b] → [a + b]`
    Add,

    /// Pop two values, push their product: `[a, b] → [a * b]`
    Mul,

    /// Pop two values, push their quotient: `[a, b] → [a / b]`
    Div,

    /// Multiply the top of stack by a constant: `[a] → [a * C]`
    MulConst(u32),

    /// Pop two values, push their difference: `[a, b] → [a - b]`
    Sub,

    /// Add a constant to the top of stack: `[a] → [a + C]`
    AddConst(u32),

    /// Subtract a constant from the top of stack: `[a] → [a - C]`
    SubConst(u32),

    /// Subtract the top of stack from a constant: `[a] → [C - a]`
    ConstSub(u32),

    /// Negate the top of stack: `[a] → [-a]`
    Neg,

    /// Pop two values, push base raised to exponent: `[base, exp] → [base^exp]`
    Pow,

    // =========================================================================
    // Trigonometric Functions (Unary)
    // =========================================================================
    /// Sine: `[x] → [sin(x)]`
    Sin,

    /// Cosine: `[x] → [cos(x)]`
    Cos,

    /// Tangent: `[x] → [tan(x)]`
    Tan,

    /// Arcsine: `[x] → [asin(x)]`, domain: `[-1, 1]`
    Asin,

    /// Arccosine: `[x] → [acos(x)]`, domain: `[-1, 1]`
    Acos,

    /// Arctangent: `[x] → [atan(x)]`
    Atan,

    // =========================================================================
    // Hyperbolic Functions (Unary)
    // =========================================================================
    /// Hyperbolic sine: `[x] → [sinh(x)]`
    Sinh,

    /// Hyperbolic cosine: `[x] → [cosh(x)]`
    Cosh,

    /// Hyperbolic tangent: `[x] → [tanh(x)]`
    Tanh,

    /// Inverse hyperbolic sine: `[x] → [asinh(x)]`
    Asinh,

    /// Inverse hyperbolic cosine: `[x] → [acosh(x)]`, domain: `x ≥ 1`
    Acosh,

    /// Inverse hyperbolic tangent: `[x] → [atanh(x)]`, domain: `|x| < 1`
    Atanh,

    // =========================================================================
    // Exponential and Logarithmic Functions (Unary)
    // =========================================================================
    /// Exponential: `[x] → [e^x]`
    Exp,

    /// Compute `exp(x) - 1` (more accurate for x near 0).
    Expm1,

    /// Compute `ln(1 + x)` (more accurate for x near 0).
    Log1p,

    /// Compute `exp(-x)`.
    ExpNeg,

    /// Natural logarithm: `[x] → [ln(x)]`, domain: `x > 0`
    Ln,

    /// Square root: `[x] → [√x]`, domain: `x ≥ 0`
    Sqrt,

    /// Cube root: `[x] → [∛x]`
    Cbrt,

    // =========================================================================
    // Basic Special Functions (Unary)
    // =========================================================================
    /// Absolute value: `[x] → [|x|]`
    Abs,

    /// Sign function: `[x] → [signum(x)]` returns -1, 0, or 1
    Signum,

    /// Floor: `[x] → [⌊x⌋]`
    Floor,

    /// Ceiling: `[x] → [⌈x⌉]`
    Ceil,

    /// Round to nearest integer: `[x] → [round(x)]`
    Round,

    // =========================================================================
    // Advanced Special Functions (Unary)
    // =========================================================================
    /// Error function: `[x] → [erf(x)]`
    ///
    /// `erf(x) = (2/√π) ∫₀ˣ e^(-t²) dt`
    Erf,

    /// Complementary error function: `[x] → [erfc(x)] = [1 - erf(x)]`
    Erfc,

    /// Gamma function: `[x] → [Γ(x)]`
    ///
    /// Poles at non-positive integers. Returns NaN at poles.
    Gamma,

    /// Digamma function: `[x] → [ψ(x)] = [Γ'(x)/Γ(x)]`
    Digamma,

    /// Trigamma function: `[x] → [ψ₁(x)] = [ψ'(x)]`
    Trigamma,

    /// Tetragamma function: `[x] → [ψ₂(x)] = [ψ''(x)]`
    Tetragamma,

    /// Sinc function: `[x] → [sinc(x)] = [sin(x)/x]`
    ///
    /// Returns 1 at x=0 (removable singularity handled).
    Sinc,

    /// Lambert W function (principal branch): `[x] → [W(x)]`
    ///
    /// Solves `W(x) * e^W(x) = x`. Domain: `x ≥ -1/e`.
    LambertW,

    /// Complete elliptic integral of the first kind: `[k] → [K(k)]`
    ///
    /// Domain: `|k| < 1`.
    EllipticK,

    /// Complete elliptic integral of the second kind: `[k] → [E(k)]`
    ///
    /// Domain: `|k| ≤ 1`.
    EllipticE,

    /// Riemann zeta function: `[s] → [ζ(s)]`
    ///
    /// Pole at s=1. Returns NaN at pole.
    Zeta,

    /// Exponential in polar form: `[x] → [exp_polar(x)]`
    ///
    /// Currently equivalent to `exp(x)`, placeholder for complex extension.
    ExpPolar,

    // =========================================================================
    // Two-Argument Functions
    // =========================================================================
    /// Two-argument arctangent: `[y, x] → [atan2(y, x)]`
    ///
    /// Returns the angle in radians between the positive x-axis and (x, y).
    Atan2,

    /// Logarithm with arbitrary base: `[base, x] → [log_base(x)]`
    ///
    /// Domain: `base > 0, base ≠ 1, x > 0`.
    Log,

    /// Bessel function of the first kind: `[n, x] → [Jₙ(x)]`
    BesselJ,

    /// Bessel function of the second kind: `[n, x] → [Yₙ(x)]`
    ///
    /// Domain: `x > 0`.
    BesselY,

    /// Modified Bessel function of the first kind: `[n, x] → [Iₙ(x)]`
    BesselI,

    /// Modified Bessel function of the second kind: `[n, x] → [Kₙ(x)]`
    ///
    /// Domain: `x > 0`.
    BesselK,

    /// Polygamma function: `[n, x] → [ψⁿ(x)]`
    ///
    /// The n-th derivative of the digamma function.
    Polygamma,

    /// Beta function: `[a, b] → [B(a, b)] = [Γ(a)Γ(b)/Γ(a+b)]`
    Beta,

    /// Derivative of Riemann zeta: `[n, s] → [ζ⁽ⁿ⁾(s)]`
    ZetaDeriv,

    /// Hermite polynomial: `[n, x] → [Hₙ(x)]`
    Hermite,

    // =========================================================================
    // Three-Argument Functions
    // =========================================================================
    /// Associated Legendre polynomial: `[l, m, x] → [Pₗᵐ(x)]`
    ///
    /// Domain: `|x| ≤ 1`, `l ≥ 0`, `|m| ≤ l`.
    AssocLegendre,

    // =========================================================================
    // Four-Argument Functions
    // =========================================================================
    /// Spherical harmonic: `[l, m, θ, φ] → [Yₗᵐ(θ, φ)]`
    ///
    /// Returns the real part of the spherical harmonic.
    SphericalHarmonic,

    // =========================================================================
    // Fused Operations (Performance Optimizations)
    // =========================================================================
    /// Square: `[x] → [x²]`
    ///
    /// Faster than `Pow` with exponent 2.
    Square,

    /// Cube: `[x] → [x³]`
    ///
    /// Faster than `Pow` with exponent 3.
    Cube,

    /// Fourth power: `[x] → [x⁴]`
    ///
    /// Computed as `(x²)²` for efficiency.
    Pow4,

    /// 1.5 power: `[x] → [x^(1.5)]`
    ///
    /// Computed as `x * sqrt(x)` for efficiency.
    Pow3_2,

    /// Inverse 1.5 power: `[x] → [x^(-1.5)]`
    ///
    /// Computed as `1 / (x * sqrt(x))` for efficiency.
    InvPow3_2,

    /// Inverse square root: `[x] → [1/√x]`
    InvSqrt,

    /// Inverse square: `[x] → [1/x²]`
    InvSquare,

    /// Inverse cube: `[x] → [1/x³]`
    InvCube,

    /// Reciprocal: `[x] → [1/x]`
    ///
    /// Faster than `LoadConst(1) + Div`.
    Recip,

    /// Integer power: `[x] → [xⁿ]` for small integer n
    ///
    /// Uses `powi` for n in range `[-2^31, 2^31-1]`, avoiding `powf` overhead.
    Powi(i32),

    /// Fused multiply-add: `[a, b, c] → [a * b + c]`
    ///
    /// Uses hardware FMA instruction when available for better precision
    /// and performance.
    MulAdd,

    /// Fused multiply-subtract: `[a, b, c] → [a * b - c]`
    MulSub,

    /// Fused negative multiply-add: `[a, b, c] → [-a * b + c]`
    NegMulAdd,

    /// Evaluate polynomial: `[x] → [P(x)]`
    ///
    /// Payload is index into constant pool where:
    ///   - `constants[idx]` = degree (n)
    ///   - `constants[idx+1]` = coeff of x^n
    ///   - `constants[idx+2]` = coeff of x^(n-1)
    ///     ...
    ///   - `constants[idx+n+1]` = coeff of x^0
    PolyEval(u32),

    /// Reciprocal of Expm1: `[x] → [1 / (e^x - 1)]`
    ///
    /// Useful for Planck's law and Bose-Einstein distributions.
    RecipExpm1,

    /// Exponential of square: `[x] → [e^(x²)]`
    ExpSqr,

    /// Exponential of negative square: `[x] → [e^(-x²)]`
    ///
    /// Useful for Gaussian distributions.
    ExpSqrNeg,
}

impl Instruction {
    /// Returns the stack effect of this instruction.
    ///
    /// Positive values indicate pushes, negative indicate pops.
    /// The net effect is the change in stack depth after execution.
    ///
    /// # Examples
    ///
    /// ```text
    /// assert_eq!(Instruction::LoadConst(0).stack_effect(), 1);  // Pushes 1
    /// assert_eq!(Instruction::Add.stack_effect(), -1);          // Pops 2, pushes 1
    /// assert_eq!(Instruction::Sin.stack_effect(), 0);           // Pops 1, pushes 1
    /// ```
    //
    // Allow match_same_arms: Arms are grouped by semantic category (push, binary, unary, etc.)
    // for documentation clarity, not just by return value.
    // Allow trivially_copy_pass_by_ref: Method convention - &self is idiomatic even for Copy types.
    #[allow(
        clippy::match_same_arms,
        reason = "Arms are grouped by semantic category for documentation clarity, not just by return value"
    )]
    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "Method convention - &self is idiomatic even for Copy types"
    )]
    #[must_use]
    pub const fn stack_effect(&self) -> i32 {
        match self {
            // Push 1 value
            Self::LoadConst(_) | Self::LoadParam(_) | Self::Dup | Self::LoadCached(_) => 1,

            // Pop 2, push 1 (net: -1)
            Self::Add | Self::Mul | Self::Div | Self::Sub | Self::Pow => -1,

            // Pop 1, push 1 (net: 0) - all unary operations
            Self::Neg
            | Self::Sin
            | Self::Cos
            | Self::Tan
            | Self::Asin
            | Self::Acos
            | Self::Atan
            | Self::Sinh
            | Self::Cosh
            | Self::Tanh
            | Self::Asinh
            | Self::Acosh
            | Self::Atanh
            | Self::Exp
            | Self::Expm1
            | Self::ExpNeg
            | Self::Ln
            | Self::Log1p
            | Self::Sqrt
            | Self::Cbrt
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
            | Self::Sinc
            | Self::LambertW
            | Self::EllipticK
            | Self::EllipticE
            | Self::Zeta
            | Self::ExpPolar
            | Self::Square
            | Self::Cube
            | Self::Pow4
            | Self::Pow3_2
            | Self::InvPow3_2
            | Self::InvSqrt
            | Self::InvSquare
            | Self::InvCube
            | Self::Recip
            | Self::Powi(_)
            | Self::MulConst(_)
            | Self::AddConst(_)
            | Self::SubConst(_)
            | Self::ConstSub(_) => 0,

            // Store does not change stack
            Self::StoreCached(_) => 0,

            // Two-arg functions: pop 2, push 1 (net: -1)
            Self::Atan2
            | Self::Log
            | Self::BesselJ
            | Self::BesselY
            | Self::BesselI
            | Self::BesselK
            | Self::Polygamma
            | Self::Beta
            | Self::ZetaDeriv
            | Self::Hermite => -1,

            // Three-arg: pop 3, push 1 (net: -2)
            Self::AssocLegendre => -2,

            // Four-arg: pop 4, push 1 (net: -3)
            Self::SphericalHarmonic => -3,

            // MulAdd: pop 3, push 1 (net: -2)
            Self::MulAdd | Self::MulSub | Self::NegMulAdd => -2,

            // Hypot: pop n, push 1 (net: 1 - n)
            // PolyEval: pop 1, push 1 (net: 0)
            Self::PolyEval(_) => 0,

            // RecipExpm1, ExpSqr, ExpSqrNeg: pop 1, push 1 (net: 0)
            Self::RecipExpm1 | Self::ExpSqr | Self::ExpSqrNeg => 0,

            // Pop: pop 1, push 0 (net: -1)
            Self::Pop => -1,

            // Swap: pop 2, push 2 (net: 0)
            Self::Swap => 0,
        }
    }

    /// Returns true if this instruction is a "fast path" operation
    /// that can be efficiently vectorized with SIMD.
    //
    // Allow trivially_copy_pass_by_ref: Method convention - &self is idiomatic even for Copy types.
    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "Method convention - &self is idiomatic even for Copy types"
    )]
    #[must_use]
    pub const fn is_simd_fast_path(&self) -> bool {
        matches!(
            self,
            Self::LoadConst(_)
                | Self::LoadParam(_)
                | Self::Add
                | Self::Mul
                | Self::MulConst(_)
                | Self::AddConst(_)
                | Self::SubConst(_)
                | Self::ConstSub(_)
                | Self::Div
                | Self::Sub
                | Self::Pow
                | Self::Neg
                | Self::Sin
                | Self::Cos
                | Self::Tan
                | Self::Sqrt
                | Self::Exp
                | Self::Expm1
                | Self::ExpNeg
                | Self::Ln
                | Self::Log1p
                | Self::Abs
                | Self::Square
                | Self::Cube
                | Self::Pow4
                | Self::Pow3_2
                | Self::InvPow3_2
                | Self::InvSqrt
                | Self::InvSquare
                | Self::InvCube
                | Self::Cbrt
                | Self::Recip
                | Self::Powi(_)
                | Self::Sinh
                | Self::Cosh
                | Self::Tanh
                | Self::MulAdd
                | Self::MulSub
                | Self::NegMulAdd
                | Self::PolyEval(_)
                | Self::Sinc
                | Self::Dup
                | Self::StoreCached(_)
                | Self::LoadCached(_)
                | Self::RecipExpm1
                | Self::ExpSqr
                | Self::ExpSqrNeg
                | Self::Pop
                | Self::Swap
        )
    }

    /// Returns a human-readable name for this instruction (for debugging).
    //
    // Allow trivially_copy_pass_by_ref: Method convention - &self is idiomatic even for Copy types.
    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "Method convention - &self is idiomatic even for Copy types"
    )]
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::LoadConst(_) => "LoadConst",
            Self::LoadParam(_) => "LoadParam",
            Self::Dup => "Dup",
            Self::Sub => "Sub",
            Self::StoreCached(_) => "StoreCached",
            Self::LoadCached(_) => "LoadCached",
            Self::Pop => "Pop",
            Self::Swap => "Swap",
            Self::Add => "Add",
            Self::Mul => "Mul",
            Self::MulConst(_) => "MulConst",
            Self::Div => "Div",
            Self::Neg => "Neg",
            Self::Pow => "Pow",
            Self::Sin => "Sin",
            Self::Cos => "Cos",
            Self::Tan => "Tan",
            Self::Asin => "Asin",
            Self::Acos => "Acos",
            Self::Atan => "Atan",
            Self::Sinh => "Sinh",
            Self::Cosh => "Cosh",
            Self::Tanh => "Tanh",
            Self::Asinh => "Asinh",
            Self::Acosh => "Acosh",
            Self::Atanh => "Atanh",
            Self::Expm1 => "Expm1",
            Self::ExpNeg => "ExpNeg",
            Self::Ln => "Ln",
            Self::Log1p => "Log1p",
            Self::Exp => "Exp",
            Self::Sqrt => "Sqrt",
            Self::Cbrt => "Cbrt",
            Self::Abs => "Abs",
            Self::Signum => "Signum",
            Self::Floor => "Floor",
            Self::Ceil => "Ceil",
            Self::Round => "Round",
            Self::Erf => "Erf",
            Self::Erfc => "Erfc",
            Self::Gamma => "Gamma",
            Self::Digamma => "Digamma",
            Self::Trigamma => "Trigamma",
            Self::Tetragamma => "Tetragamma",
            Self::Sinc => "Sinc",
            Self::LambertW => "LambertW",
            Self::EllipticK => "EllipticK",
            Self::EllipticE => "EllipticE",
            Self::Zeta => "Zeta",
            Self::ExpPolar => "ExpPolar",
            Self::Atan2 => "Atan2",
            Self::Log => "Log",
            Self::BesselJ => "BesselJ",
            Self::BesselY => "BesselY",
            Self::BesselI => "BesselI",
            Self::BesselK => "BesselK",
            Self::Polygamma => "Polygamma",
            Self::Beta => "Beta",
            Self::ZetaDeriv => "ZetaDeriv",
            Self::Hermite => "Hermite",
            Self::AssocLegendre => "AssocLegendre",
            Self::SphericalHarmonic => "SphericalHarmonic",
            Self::Square => "Square",
            Self::Cube => "Cube",
            Self::Pow4 => "Pow4",
            Self::Pow3_2 => "Pow3_2",
            Self::InvPow3_2 => "InvPow3_2",
            Self::InvSqrt => "InvSqrt",
            Self::InvSquare => "InvSquare",
            Self::InvCube => "InvCube",
            Self::AddConst(_) => "AddConst",
            Self::SubConst(_) => "SubConst",
            Self::ConstSub(_) => "ConstSub",
            Self::NegMulAdd => "NegMulAdd",
            Self::MulSub => "MulSub",
            Self::Recip => "Recip",
            Self::Powi(_) => "Powi",
            Self::MulAdd => "MulAdd",
            Self::PolyEval(_) => "PolyEval",
            Self::RecipExpm1 => "RecipExpm1",
            Self::ExpSqr => "ExpSqr",
            Self::ExpSqrNeg => "ExpSqrNeg",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_size() {
        // Ensure instruction enum stays small for cache efficiency
        assert!(
            std::mem::size_of::<Instruction>() <= 8,
            "Instruction size should be <= 8 bytes for cache efficiency"
        );
    }

    #[test]
    fn test_stack_effects() {
        // Verify some key stack effects
        assert_eq!(Instruction::LoadConst(0).stack_effect(), 1);
        assert_eq!(Instruction::Add.stack_effect(), -1);
        assert_eq!(Instruction::Sin.stack_effect(), 0);
        assert_eq!(Instruction::MulAdd.stack_effect(), -2);
    }
}
