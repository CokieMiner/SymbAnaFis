//! Compiled expression evaluator for fast numerical evaluation
//!
//! # Safety
//! This module uses unsafe code in performance-critical stack operations.
//! Safety is guaranteed by the Compiler which validates stack depth at compile time.
#![allow(unsafe_code)]
//!
//! Converts an expression tree to flat bytecode that can be evaluated
//! efficiently without tree traversal. Thread-safe for parallel evaluation.
//!
//! # Example
//! ```
//! use symb_anafis::parse;
//! use std::collections::HashSet;
//!
//! let expr = parse("sin(x) * cos(x) + x^2", &HashSet::new(), &HashSet::new(), None).unwrap();
//! let evaluator = expr.compile().unwrap();
//!
//! // Evaluate at x = 0.5
//! let result = evaluator.evaluate(&[0.5]);
//! assert!((result - (0.5_f64.sin() * 0.5_f64.cos() + 0.25)).abs() < 1e-10);
//! ```

use crate::core::error::DiffError;
use crate::core::traits::EPSILON;
use crate::core::unified_context::Context;
use crate::{Expr, ExprKind};
use std::collections::HashMap;
use std::sync::Arc;
use wide::f64x4;

/// Bytecode instruction for stack-based evaluation
#[derive(Clone, Copy, Debug)]
pub(crate) enum Instruction {
    /// Push a constant value onto the stack
    LoadConst(f64),
    /// Push a parameter value onto the stack (by index)
    LoadParam(usize),

    // Arithmetic operations (pop operands, push result)
    Add,
    Mul,
    Div,
    Neg,
    Pow,

    // Trigonometric functions (unary)
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Cot,
    Sec,
    Csc,
    Acot,
    Asec,
    Acsc,

    // Hyperbolic functions (unary)
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    Coth,
    Sech,
    Csch,
    Acoth,
    Asech,
    Acsch,

    // Exponential/Logarithmic (unary)
    Exp,
    Ln,
    Log10,
    Log2,
    Sqrt,
    Cbrt,

    // Special functions (unary)
    Abs,
    Signum,
    Floor,
    Ceil,
    Round,
    Erf,
    Erfc,
    Gamma,
    Digamma,
    Trigamma,
    Tetragamma,
    Sinc,
    LambertW,
    EllipticK,
    EllipticE,
    Zeta,
    ExpPolar,

    // Two-argument functions
    Atan2,
    Log, // log(base, x)
    BesselJ,
    BesselY,
    BesselI,
    BesselK,
    Polygamma,
    Beta,
    ZetaDeriv,
    Hermite,

    // Three-argument functions
    AssocLegendre,

    // Four-argument functions
    SphericalHarmonic,

    // Fused operations (performance optimizations)
    /// x^2 - faster than Pow with exponent 2
    Square,
    /// x^3 - faster than Pow with exponent 3
    Cube,
    /// 1/x - faster than LoadConst(1) + Div
    Recip,
}

// =============================================================================
// Unsafe Stack Helpers (zero-cost in release builds)
// =============================================================================
// These macros provide fast stack operations by using unsafe get_unchecked.
// Safety is guaranteed by the Compiler which tracks max_stack at compile time.
// Stack underflow is impossible in correctly compiled bytecode.

/// Get mutable reference to the top of stack (unsafe, but validated by debug_assert)
macro_rules! stack_top_mut {
    ($stack:expr) => {{
        debug_assert!(!$stack.is_empty(), "Stack empty - compiler bug");
        let len = $stack.len();
        unsafe { $stack.get_unchecked_mut(len - 1) }
    }};
}

/// Binary operation: pop B, apply op to top (A op= B)
/// This handles the borrow correctly by doing everything in one unsafe block
macro_rules! stack_binop {
    ($stack:expr, $op:tt) => {{
        debug_assert!($stack.len() >= 2, "Stack underflow - compiler bug");
        unsafe {
            let len = $stack.len();
            let b = *$stack.get_unchecked(len - 1);
            *$stack.get_unchecked_mut(len - 2) $op b;
            $stack.set_len(len - 1);
        }
    }};
}

/// Binary operation with custom expression: pop B, apply f(A, B) to A
macro_rules! stack_binop_fn {
    ($stack:expr, $f:expr) => {{
        debug_assert!($stack.len() >= 2, "Stack underflow - compiler bug");
        unsafe {
            let len = $stack.len();
            let b = *$stack.get_unchecked(len - 1);
            let a = $stack.get_unchecked_mut(len - 2);
            *a = $f(*a, b);
            $stack.set_len(len - 1);
        }
    }};
}
/// Macro to process a single instruction
/// $instr: The instruction to process
/// $stack: The stack to operate on (`Vec<f64>`)
/// $load_param: Closure/Expression to load a parameter by index: |idx| -> f64
macro_rules! process_instruction {
    ($instr:expr, $stack:ident, $load_param:expr) => {
        match *$instr {
            Instruction::LoadConst(c) => $stack.push(c),
            Instruction::LoadParam(i) => $stack.push($load_param(i)),

            // Binary operations (using unsafe for performance)
            Instruction::Add => stack_binop!($stack, +=),
            Instruction::Mul => stack_binop!($stack, *=),
            Instruction::Div => stack_binop!($stack, /=),
            Instruction::Pow => stack_binop_fn!($stack, |a: f64, b: f64| a.powf(b)),

            // Fused operations (performance optimizations)
            Instruction::Square => {
                let top = stack_top_mut!($stack);
                *top = *top * *top;
            }
            Instruction::Cube => {
                let top = stack_top_mut!($stack);
                let x = *top;
                *top = x * x * x;
            }
            Instruction::Recip => {
                let top = stack_top_mut!($stack);
                *top = 1.0 / *top;
            }

            // Unary operations (using unsafe for performance)
            Instruction::Neg => {
                let top = stack_top_mut!($stack);
                *top = -*top;
            }

            // Trigonometric
            Instruction::Sin => {
                let top = stack_top_mut!($stack);
                *top = top.sin();
            }
            Instruction::Cos => {
                let top = stack_top_mut!($stack);
                *top = top.cos();
            }
            Instruction::Tan => {
                let top = stack_top_mut!($stack);
                *top = top.tan();
            }
            Instruction::Asin => {
                let top = stack_top_mut!($stack);
                *top = top.asin();
            }
            Instruction::Acos => {
                let top = stack_top_mut!($stack);
                *top = top.acos();
            }
            Instruction::Atan => {
                let top = stack_top_mut!($stack);
                *top = top.atan();
            }
            Instruction::Cot => {
                let top = stack_top_mut!($stack);
                *top = 1.0 / top.tan();
            }
            Instruction::Sec => {
                let top = stack_top_mut!($stack);
                *top = 1.0 / top.cos();
            }
            Instruction::Csc => {
                let top = stack_top_mut!($stack);
                *top = 1.0 / top.sin();
            }
            Instruction::Acot => {
                let top = stack_top_mut!($stack);
                let x = *top;
                *top = if x.abs() < EPSILON {
                    std::f64::consts::PI / 2.0
                } else if x > 0.0 {
                    (1.0 / x).atan()
                } else {
                    (1.0 / x).atan() + std::f64::consts::PI
                };
            }
            Instruction::Asec => {
                let top = stack_top_mut!($stack);
                *top = (1.0 / *top).acos();
            }
            Instruction::Acsc => {
                let top = stack_top_mut!($stack);
                *top = (1.0 / *top).asin();
            }

            // Hyperbolic
            Instruction::Sinh => {
                let top = stack_top_mut!($stack);
                *top = top.sinh();
            }
            Instruction::Cosh => {
                let top = stack_top_mut!($stack);
                *top = top.cosh();
            }
            Instruction::Tanh => {
                let top = stack_top_mut!($stack);
                *top = top.tanh();
            }
            Instruction::Asinh => {
                let top = stack_top_mut!($stack);
                *top = top.asinh();
            }
            Instruction::Acosh => {
                let top = stack_top_mut!($stack);
                *top = top.acosh();
            }
            Instruction::Atanh => {
                let top = stack_top_mut!($stack);
                *top = top.atanh();
            }
            Instruction::Coth => {
                let top = stack_top_mut!($stack);
                *top = 1.0 / top.tanh();
            }
            Instruction::Sech => {
                let top = stack_top_mut!($stack);
                *top = 1.0 / top.cosh();
            }
            Instruction::Csch => {
                let top = stack_top_mut!($stack);
                *top = 1.0 / top.sinh();
            }
            Instruction::Acoth => {
                let top = stack_top_mut!($stack);
                let x = *top;
                *top = 0.5 * ((x + 1.0) / (x - 1.0)).ln();
            }
            Instruction::Asech => {
                let top = stack_top_mut!($stack);
                *top = (1.0 / *top).acosh();
            }
            Instruction::Acsch => {
                let top = stack_top_mut!($stack);
                *top = (1.0 / *top).asinh();
            }

            // Exponential/Logarithmic
            Instruction::Exp => {
                let top = stack_top_mut!($stack);
                *top = top.exp();
            }
            Instruction::Ln => {
                let top = stack_top_mut!($stack);
                *top = top.ln();
            }
            Instruction::Log10 => {
                let top = stack_top_mut!($stack);
                *top = top.log10();
            }
            Instruction::Log2 => {
                let top = stack_top_mut!($stack);
                *top = top.log2();
            }
            Instruction::Sqrt => {
                let top = stack_top_mut!($stack);
                *top = top.sqrt();
            }
            Instruction::Cbrt => {
                let top = stack_top_mut!($stack);
                *top = top.cbrt();
            }

            // Special functions (unary)
            Instruction::Abs => {
                let top = stack_top_mut!($stack);
                *top = top.abs();
            }
            Instruction::Signum => {
                let top = stack_top_mut!($stack);
                *top = top.signum();
            }
            Instruction::Floor => {
                let top = stack_top_mut!($stack);
                *top = top.floor();
            }
            Instruction::Ceil => {
                let top = stack_top_mut!($stack);
                *top = top.ceil();
            }
            Instruction::Round => {
                let top = stack_top_mut!($stack);
                *top = top.round();
            }

            // Special functions - use safe access pattern since these have
            // domain handling, error conditions, and the .unwrap() overhead
            // is negligible compared to the function computation itself
            Instruction::Erf => {
                *$stack.last_mut().unwrap() = crate::math::eval_erf(*$stack.last().unwrap())
            }
            Instruction::Erfc => {
                *$stack.last_mut().unwrap() = 1.0 - crate::math::eval_erf(*$stack.last().unwrap())
            }
            Instruction::Gamma => {
                *$stack.last_mut().unwrap() =
                    crate::math::eval_gamma(*$stack.last().unwrap()).unwrap_or(f64::NAN)
            }
            Instruction::Digamma => {
                *$stack.last_mut().unwrap() =
                    crate::math::eval_digamma(*$stack.last().unwrap()).unwrap_or(f64::NAN)
            }
            Instruction::Trigamma => {
                *$stack.last_mut().unwrap() =
                    crate::math::eval_trigamma(*$stack.last().unwrap()).unwrap_or(f64::NAN)
            }
            Instruction::Tetragamma => {
                // Tetragamma is polygamma(3, x) - the 4th derivative of ln(Gamma(x))
                *$stack.last_mut().unwrap() =
                    crate::math::eval_polygamma(3, *$stack.last().unwrap()).unwrap_or(f64::NAN)
            }
            Instruction::Sinc => {
                let x = *$stack.last().unwrap();
                *$stack.last_mut().unwrap() = if x.abs() < EPSILON { 1.0 } else { x.sin() / x };
            }
            Instruction::LambertW => {
                *$stack.last_mut().unwrap() =
                    crate::math::eval_lambert_w(*$stack.last().unwrap()).unwrap_or(f64::NAN)
            }
            Instruction::EllipticK => {
                *$stack.last_mut().unwrap() =
                    crate::math::eval_elliptic_k(*$stack.last().unwrap()).unwrap_or(f64::NAN)
            }
            Instruction::EllipticE => {
                *$stack.last_mut().unwrap() =
                    crate::math::eval_elliptic_e(*$stack.last().unwrap()).unwrap_or(f64::NAN)
            }
            Instruction::Zeta => {
                *$stack.last_mut().unwrap() =
                    crate::math::eval_zeta(*$stack.last().unwrap()).unwrap_or(f64::NAN)
            }
            Instruction::ExpPolar => {
                *$stack.last_mut().unwrap() = crate::math::eval_exp_polar(*$stack.last().unwrap())
            }

            // Two-argument functions
            Instruction::Log => {
                // log(base, x) = ln(x) / ln(base)
                let x = $stack.pop().unwrap();
                let base = $stack.last_mut().unwrap();
                *base = if *base <= 0.0 || *base == 1.0 || x <= 0.0 {
                    f64::NAN
                } else {
                    x.ln() / base.ln()
                };
            }
            Instruction::Atan2 => {
                let x = $stack.pop().unwrap();
                let y = $stack.last_mut().unwrap();
                *y = y.atan2(x);
            }
            Instruction::BesselJ => {
                let x = $stack.pop().unwrap();
                let n = $stack.last_mut().unwrap();
                *n = crate::math::bessel_j((*n).round() as i32, x).unwrap_or(f64::NAN);
            }
            Instruction::BesselY => {
                let x = $stack.pop().unwrap();
                let n = $stack.last_mut().unwrap();
                *n = crate::math::bessel_y((*n).round() as i32, x).unwrap_or(f64::NAN);
            }
            Instruction::BesselI => {
                let x = $stack.pop().unwrap();
                let n = $stack.last_mut().unwrap();
                *n = crate::math::bessel_i((*n).round() as i32, x).unwrap_or(f64::NAN);
            }
            Instruction::BesselK => {
                let x = $stack.pop().unwrap();
                let n = $stack.last_mut().unwrap();
                *n = crate::math::bessel_k((*n).round() as i32, x).unwrap_or(f64::NAN);
            }
            Instruction::Polygamma => {
                let x = $stack.pop().unwrap();
                let n = $stack.last_mut().unwrap();
                *n = crate::math::eval_polygamma((*n).round() as i32, x).unwrap_or(f64::NAN);
            }
            Instruction::Beta => {
                let b = $stack.pop().unwrap();
                let a = $stack.last_mut().unwrap();
                let ga = crate::math::eval_gamma(*a);
                let gb = crate::math::eval_gamma(b);
                let gab = crate::math::eval_gamma(*a + b);
                *a = match (ga, gb, gab) {
                    (Some(ga), Some(gb), Some(gab)) => ga * gb / gab,
                    _ => f64::NAN,
                };
            }
            Instruction::ZetaDeriv => {
                let s = $stack.pop().unwrap();
                let n = $stack.last_mut().unwrap();
                *n = crate::math::eval_zeta_deriv((*n).round() as i32, s).unwrap_or(f64::NAN);
            }
            Instruction::Hermite => {
                let x = $stack.pop().unwrap();
                let n = $stack.last_mut().unwrap();
                *n = crate::math::eval_hermite((*n).round() as i32, x).unwrap_or(f64::NAN);
            }

            // Three-argument functions
            Instruction::AssocLegendre => {
                let x = $stack.pop().unwrap();
                let m = $stack.pop().unwrap();
                let l = $stack.last_mut().unwrap();
                *l = crate::math::eval_assoc_legendre((*l).round() as i32, m.round() as i32, x)
                    .unwrap_or(f64::NAN);
            }

            // Four-argument functions
            Instruction::SphericalHarmonic => {
                let phi = $stack.pop().unwrap();
                let theta = $stack.pop().unwrap();
                let m = $stack.pop().unwrap();
                let l = $stack.last_mut().unwrap();
                *l = crate::math::eval_spherical_harmonic(
                    (*l).round() as i32,
                    m.round() as i32,
                    theta,
                    phi,
                )
                .unwrap_or(f64::NAN);
            }
        }
    };
}

/// Macro for fast-path instruction dispatch (single evaluation)
/// $instr: The instruction to process
/// $stack: The stack `Vec<f64>`
/// $params: The params slice `&[f64]`
/// $self: Reference to CompiledEvaluator (for slow path fallback)
macro_rules! single_fast_path {
    ($instr:expr, $stack:ident, $params:expr, $self_ref:expr) => {
        match *$instr {
            Instruction::LoadConst(c) => $stack.push(c),
            Instruction::LoadParam(p) => $stack.push($params[p]),
            Instruction::Add => {
                let b = $stack.pop().unwrap();
                *$stack.last_mut().unwrap() += b;
            }
            Instruction::Mul => {
                let b = $stack.pop().unwrap();
                *$stack.last_mut().unwrap() *= b;
            }
            Instruction::Div => {
                let b = $stack.pop().unwrap();
                *$stack.last_mut().unwrap() /= b;
            }
            Instruction::Pow => {
                let exp = $stack.pop().unwrap();
                let base = $stack.last_mut().unwrap();
                *base = base.powf(exp);
            }
            Instruction::Neg => {
                let top = $stack.last_mut().unwrap();
                *top = -*top;
            }
            Instruction::Sin => {
                let top = $stack.last_mut().unwrap();
                *top = top.sin();
            }
            Instruction::Cos => {
                let top = $stack.last_mut().unwrap();
                *top = top.cos();
            }
            Instruction::Sqrt => {
                let top = $stack.last_mut().unwrap();
                *top = top.sqrt();
            }
            Instruction::Exp => {
                let top = $stack.last_mut().unwrap();
                *top = top.exp();
            }
            Instruction::Ln => {
                let top = $stack.last_mut().unwrap();
                *top = top.ln();
            }
            Instruction::Tan => {
                let top = $stack.last_mut().unwrap();
                *top = top.tan();
            }
            Instruction::Abs => {
                let top = $stack.last_mut().unwrap();
                *top = top.abs();
            }
            // Fused operations
            Instruction::Square => {
                let top = $stack.last_mut().unwrap();
                *top = *top * *top;
            }
            Instruction::Cube => {
                let top = $stack.last_mut().unwrap();
                let x = *top;
                *top = x * x * x;
            }
            Instruction::Recip => {
                let top = $stack.last_mut().unwrap();
                *top = 1.0 / *top;
            }
            _ => $self_ref.exec_slow_instruction_single($instr, &mut *$stack, $params),
        }
    };
}

/// Macro for fast-path batch instruction dispatch (scalar, used for remainder)
/// $instr: The instruction to process
/// $stack: The stack `Vec<f64>`
/// $columns: The columnar data `&[&[f64]]`
/// $point_idx: The current point index
/// $self: Reference to CompiledEvaluator (for slow path fallback)
macro_rules! batch_fast_path {
    ($instr:expr, $stack:ident, $columns:expr, $point_idx:expr, $self_ref:expr) => {
        match *$instr {
            Instruction::LoadConst(c) => $stack.push(c),
            Instruction::LoadParam(p) => $stack.push($columns[p][$point_idx]),
            Instruction::Add => {
                let b = $stack.pop().unwrap();
                *$stack.last_mut().unwrap() += b;
            }
            Instruction::Mul => {
                let b = $stack.pop().unwrap();
                *$stack.last_mut().unwrap() *= b;
            }
            Instruction::Div => {
                let b = $stack.pop().unwrap();
                *$stack.last_mut().unwrap() /= b;
            }
            Instruction::Pow => {
                let exp = $stack.pop().unwrap();
                let base = $stack.last_mut().unwrap();
                *base = base.powf(exp);
            }
            Instruction::Neg => {
                let top = $stack.last_mut().unwrap();
                *top = -*top;
            }
            Instruction::Sin => {
                let top = $stack.last_mut().unwrap();
                *top = top.sin();
            }
            Instruction::Cos => {
                let top = $stack.last_mut().unwrap();
                *top = top.cos();
            }
            Instruction::Sqrt => {
                let top = $stack.last_mut().unwrap();
                *top = top.sqrt();
            }
            Instruction::Exp => {
                let top = $stack.last_mut().unwrap();
                *top = top.exp();
            }
            Instruction::Ln => {
                let top = $stack.last_mut().unwrap();
                *top = top.ln();
            }
            Instruction::Tan => {
                let top = $stack.last_mut().unwrap();
                *top = top.tan();
            }
            Instruction::Abs => {
                let top = $stack.last_mut().unwrap();
                *top = top.abs();
            }
            // Fused operations
            Instruction::Square => {
                let top = $stack.last_mut().unwrap();
                *top = *top * *top;
            }
            Instruction::Cube => {
                let top = $stack.last_mut().unwrap();
                let x = *top;
                *top = x * x * x;
            }
            Instruction::Recip => {
                let top = $stack.last_mut().unwrap();
                *top = 1.0 / *top;
            }
            _ => $self_ref.exec_slow_instruction($instr, &mut $stack),
        }
    };
}

/// SIMD macro for fast-path batch instruction dispatch using f64x4
/// Processes 4 f64 values simultaneously for ~2x speedup
/// $instr: The instruction to process
/// $stack: The stack `Vec<f64x4>`
/// $columns: The columnar data `&[&[f64]]`
/// $base: Base index for the 4-point chunk
/// $self: Reference to CompiledEvaluator (for slow path fallback)
macro_rules! simd_batch_fast_path {
    ($instr:expr, $stack:ident, $columns:expr, $base:expr, $self_ref:expr) => {
        match *$instr {
            Instruction::LoadConst(c) => $stack.push(f64x4::splat(c)),
            Instruction::LoadParam(p) => {
                let col = $columns[p];
                $stack.push(f64x4::new([
                    col[$base],
                    col[$base + 1],
                    col[$base + 2],
                    col[$base + 3],
                ]));
            }
            Instruction::Add => {
                let b = $stack.pop().unwrap();
                *$stack.last_mut().unwrap() += b;
            }
            Instruction::Mul => {
                let b = $stack.pop().unwrap();
                *$stack.last_mut().unwrap() *= b;
            }
            Instruction::Div => {
                let b = $stack.pop().unwrap();
                *$stack.last_mut().unwrap() /= b;
            }
            Instruction::Pow => {
                let exp = $stack.pop().unwrap();
                let base = $stack.last_mut().unwrap();
                *base = base.pow_f64x4(exp);
            }
            Instruction::Neg => {
                let top = $stack.last_mut().unwrap();
                *top = -*top;
            }
            Instruction::Sin => {
                let top = $stack.last_mut().unwrap();
                *top = top.sin();
            }
            Instruction::Cos => {
                let top = $stack.last_mut().unwrap();
                *top = top.cos();
            }
            Instruction::Sqrt => {
                let top = $stack.last_mut().unwrap();
                *top = top.sqrt();
            }
            Instruction::Exp => {
                let top = $stack.last_mut().unwrap();
                *top = top.exp();
            }
            Instruction::Ln => {
                let top = $stack.last_mut().unwrap();
                *top = top.ln();
            }
            Instruction::Tan => {
                let top = $stack.last_mut().unwrap();
                *top = top.tan();
            }
            Instruction::Abs => {
                let top = $stack.last_mut().unwrap();
                *top = top.abs();
            }
            // Fused operations (SIMD)
            Instruction::Square => {
                let top = $stack.last_mut().unwrap();
                *top = *top * *top;
            }
            Instruction::Cube => {
                let top = $stack.last_mut().unwrap();
                let x = *top;
                *top = x * x * x;
            }
            Instruction::Recip => {
                let top = $stack.last_mut().unwrap();
                *top = f64x4::splat(1.0) / *top;
            }
            _ => $self_ref.exec_simd_slow_instruction($instr, &mut $stack),
        }
    };
}

/// Compiled expression evaluator - thread-safe, reusable
///
/// The evaluator holds immutable bytecode that can be shared across threads.
/// Each call to `evaluate` uses a thread-local or per-call stack.
#[derive(Clone)]
pub struct CompiledEvaluator {
    /// Bytecode instructions (immutable after compilation)
    instructions: Arc<[Instruction]>,
    /// Required stack depth for evaluation
    stack_size: usize,
    /// Parameter names in order (for mapping HashMap -> array)
    param_names: Arc<[String]>,
}

impl CompiledEvaluator {
    /// Compile an expression to bytecode
    ///
    /// The `param_order` specifies the order of parameters in the evaluation array.
    /// All variables in the expression must be in this list.
    pub fn compile(expr: &Expr, param_order: &[&str]) -> Result<Self, DiffError> {
        Self::compile_with_context(expr, param_order, None)
    }

    /// Compile an expression with a custom function context
    pub fn compile_with_context(
        expr: &Expr,
        param_order: &[&str],
        context: Option<&Context>,
    ) -> Result<Self, DiffError> {
        // First, expand any user function calls with their body expressions
        // This allows the compiled evaluator to evaluate them numerically
        let expanded_expr = if let Some(ctx) = context {
            let mut expanding = std::collections::HashSet::new();
            Self::expand_user_functions(expr, ctx, &mut expanding, 0)
        } else {
            expr.clone()
        };

        let mut compiler = Compiler::new(param_order, context);
        compiler.compile_expr(&expanded_expr)?;

        Ok(Self {
            instructions: compiler.instructions.into(),
            stack_size: compiler.max_stack,
            param_names: param_order.iter().map(|s| s.to_string()).collect(),
        })
    }

    /// Recursively expand user function calls with their body expressions.
    ///
    /// This substitutes `f(arg1, arg2, ...)` with the body expression where
    /// formal parameters are replaced by the actual argument expressions.
    ///
    /// The `expanding` set tracks functions currently being expanded to prevent
    /// infinite recursion from self-referential or mutually recursive functions.
    /// The `depth` parameter limits recursion depth to prevent stack overflow.
    fn expand_user_functions(
        expr: &Expr,
        ctx: &Context,
        expanding: &mut std::collections::HashSet<String>,
        depth: usize,
    ) -> Expr {
        const MAX_EXPANSION_DEPTH: usize = 100;

        if depth > MAX_EXPANSION_DEPTH {
            // Return unexpanded to prevent stack overflow
            return expr.clone();
        }

        match &expr.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => expr.clone(),

            ExprKind::Sum(terms) => {
                let expanded: Vec<Expr> = terms
                    .iter()
                    .map(|t| Self::expand_user_functions(t, ctx, expanding, depth + 1))
                    .collect();
                Expr::sum(expanded)
            }

            ExprKind::Product(factors) => {
                let expanded: Vec<Expr> = factors
                    .iter()
                    .map(|f| Self::expand_user_functions(f, ctx, expanding, depth + 1))
                    .collect();
                Expr::product(expanded)
            }

            ExprKind::Div(num, den) => {
                let num_exp = Self::expand_user_functions(num, ctx, expanding, depth + 1);
                let den_exp = Self::expand_user_functions(den, ctx, expanding, depth + 1);
                Expr::div_expr(num_exp, den_exp)
            }

            ExprKind::Pow(base, exp) => {
                let base_exp = Self::expand_user_functions(base, ctx, expanding, depth + 1);
                let exp_exp = Self::expand_user_functions(exp, ctx, expanding, depth + 1);
                Expr::pow_static(base_exp, exp_exp)
            }

            ExprKind::FunctionCall { name, args } => {
                // First expand arguments
                let expanded_args: Vec<Expr> = args
                    .iter()
                    .map(|a| Self::expand_user_functions(a, ctx, expanding, depth + 1))
                    .collect();

                let fn_name = name.as_str().to_string();

                // Check for recursion and if this is a user function with a body
                // Check for recursion and if this is a user function with a body
                if !expanding.contains(&fn_name)
                    && let Some(user_fn) = ctx.get_user_fn(&fn_name)
                    && user_fn.accepts_arity(expanded_args.len())
                    && let Some(body_fn) = &user_fn.body
                {
                    // Mark as expanding to prevent infinite recursion
                    expanding.insert(fn_name.clone());

                    let arc_args: Vec<Arc<Expr>> =
                        expanded_args.iter().map(|a| Arc::new(a.clone())).collect();
                    let body_expr = body_fn(&arc_args);
                    let result = Self::expand_user_functions(&body_expr, ctx, expanding, depth + 1);

                    expanding.remove(&fn_name);
                    return result;
                }

                // Not expandable - return as-is with expanded args
                Expr::func_multi(name.as_str(), expanded_args)
            }

            ExprKind::Poly(poly) => {
                let poly_expr = poly.to_expr();
                Self::expand_user_functions(&poly_expr, ctx, expanding, depth + 1)
            }

            ExprKind::Derivative { inner, var, order } => {
                let expanded_inner = Self::expand_user_functions(inner, ctx, expanding, depth + 1);
                Expr::derivative_interned(expanded_inner, var.clone(), *order)
            }
        }
    }

    /// Compile an expression, automatically determining parameter order from variables
    pub fn compile_auto(expr: &Expr) -> Result<Self, DiffError> {
        Self::compile_auto_with_context(expr, None)
    }

    /// Compile auto with context
    pub fn compile_auto_with_context(
        expr: &Expr,
        context: Option<&Context>,
    ) -> Result<Self, DiffError> {
        let vars = expr.variables();
        let mut param_order: Vec<String> = vars
            .into_iter()
            .filter(|v| !crate::core::known_symbols::is_known_constant(v.as_str()))
            .collect();
        param_order.sort(); // Consistent ordering

        let param_refs: Vec<&str> = param_order.iter().map(|s| s.as_str()).collect();
        Self::compile_with_context(expr, &param_refs, context)
    }

    /// Get the required stack size for this expression
    pub fn stack_size(&self) -> usize {
        self.stack_size
    }

    /// Fast evaluation - no allocations in hot path, no tree traversal
    ///
    /// # Parameters
    /// `params` - Parameter values in the same order as `param_names()`
    ///
    /// # Panics
    /// Panics if stack underflow (indicates compiler bug)
    #[inline]
    pub fn evaluate(&self, params: &[f64]) -> f64 {
        // Fast path: use stack-allocated buffer for common expressions (stack_size <= 32)
        // This avoids heap allocation entirely for most use cases
        const INLINE_STACK_SIZE: usize = 32;

        if self.stack_size <= INLINE_STACK_SIZE {
            // Use a fixed-size array on the stack - zero allocation
            let mut inline_stack = [0.0_f64; INLINE_STACK_SIZE];
            let mut len = 0usize;

            for instr in self.instructions.iter() {
                // Inline the fast path operations directly to avoid Vec overhead
                match *instr {
                    Instruction::LoadConst(c) => {
                        inline_stack[len] = c;
                        len += 1;
                    }
                    Instruction::LoadParam(p) => {
                        inline_stack[len] = params[p];
                        len += 1;
                    }
                    Instruction::Add => {
                        len -= 1;
                        inline_stack[len - 1] += inline_stack[len];
                    }
                    Instruction::Mul => {
                        len -= 1;
                        inline_stack[len - 1] *= inline_stack[len];
                    }
                    Instruction::Div => {
                        len -= 1;
                        inline_stack[len - 1] /= inline_stack[len];
                    }
                    Instruction::Pow => {
                        len -= 1;
                        inline_stack[len - 1] = inline_stack[len - 1].powf(inline_stack[len]);
                    }
                    Instruction::Neg => {
                        inline_stack[len - 1] = -inline_stack[len - 1];
                    }
                    Instruction::Sin => {
                        inline_stack[len - 1] = inline_stack[len - 1].sin();
                    }
                    Instruction::Cos => {
                        inline_stack[len - 1] = inline_stack[len - 1].cos();
                    }
                    Instruction::Sqrt => {
                        inline_stack[len - 1] = inline_stack[len - 1].sqrt();
                    }
                    Instruction::Exp => {
                        inline_stack[len - 1] = inline_stack[len - 1].exp();
                    }
                    Instruction::Ln => {
                        inline_stack[len - 1] = inline_stack[len - 1].ln();
                    }
                    Instruction::Tan => {
                        inline_stack[len - 1] = inline_stack[len - 1].tan();
                    }
                    Instruction::Abs => {
                        inline_stack[len - 1] = inline_stack[len - 1].abs();
                    }
                    Instruction::Square => {
                        let x = inline_stack[len - 1];
                        inline_stack[len - 1] = x * x;
                    }
                    Instruction::Cube => {
                        let x = inline_stack[len - 1];
                        inline_stack[len - 1] = x * x * x;
                    }
                    Instruction::Recip => {
                        inline_stack[len - 1] = 1.0 / inline_stack[len - 1];
                    }
                    _ => {
                        // Slow path: expression uses uncommon instructions
                        // Fall back to heap-allocated Vec evaluation
                        let mut vec_stack: Vec<f64> = Vec::with_capacity(self.stack_size);
                        return self.evaluate_with_stack(params, &mut vec_stack);
                    }
                }
            }

            if len > 0 {
                inline_stack[len - 1]
            } else {
                f64::NAN
            }
        } else {
            // Large expression: fall back to heap-allocated Vec
            let mut stack: Vec<f64> = Vec::with_capacity(self.stack_size);
            self.evaluate_with_stack(params, &mut stack)
        }
    }

    /// Evaluate using an existing stack buffer (avoids allocation)
    #[inline]
    pub fn evaluate_with_stack(&self, params: &[f64], stack: &mut Vec<f64>) -> f64 {
        stack.clear();

        for instr in self.instructions.iter() {
            single_fast_path!(instr, stack, params, self);
        }
        stack.pop().unwrap_or(f64::NAN)
    }

    /// Get parameter names in order
    #[inline]
    pub fn param_names(&self) -> &[String] {
        &self.param_names
    }

    /// Get number of parameters
    #[inline]
    pub fn param_count(&self) -> usize {
        self.param_names.len()
    }

    /// Get number of bytecode instructions (for debugging/profiling)
    #[inline]
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Batch evaluation - evaluate expression at multiple data points
    ///
    /// This method processes all data points in a single call, moving the evaluation
    /// loop inside the VM for better cache locality. Data is expected in columnar format:
    /// each slice in `columns` corresponds to one parameter (in `param_names()` order),
    /// and each element within a column is a data point.
    ///
    /// # Parameters
    /// - `columns`: Columnar data, where `columns[param_idx][point_idx]` gives the value
    ///   of parameter `param_idx` at data point `point_idx`
    /// - `output`: Mutable slice to write results, must have length >= number of data points
    ///
    /// # Errors
    /// - `EvalColumnMismatch` if `columns.len()` != `param_count()`
    /// - `EvalColumnLengthMismatch` if column lengths don't all match
    /// - `EvalOutputTooSmall` if `output.len()` < number of data points
    #[inline]
    pub fn eval_batch(&self, columns: &[&[f64]], output: &mut [f64]) -> Result<(), DiffError> {
        if columns.len() != self.param_names.len() {
            return Err(DiffError::EvalColumnMismatch {
                expected: self.param_names.len(),
                got: columns.len(),
            });
        }

        let n_points = if columns.is_empty() {
            1
        } else {
            columns[0].len()
        };

        if !columns.iter().all(|c| c.len() == n_points) {
            return Err(DiffError::EvalColumnLengthMismatch);
        }
        if output.len() < n_points {
            return Err(DiffError::EvalOutputTooSmall {
                needed: n_points,
                got: output.len(),
            });
        }

        // Process in chunks of 4 using SIMD
        let full_chunks = n_points / 4;
        let mut simd_stack: Vec<f64x4> = Vec::with_capacity(self.stack_size);

        for chunk in 0..full_chunks {
            let base = chunk * 4;
            simd_stack.clear();

            for instr in self.instructions.iter() {
                simd_batch_fast_path!(instr, simd_stack, columns, base, self);
            }

            let result = simd_stack.pop().unwrap_or(f64x4::splat(f64::NAN));
            let arr = result.to_array();
            output[base] = arr[0];
            output[base + 1] = arr[1];
            output[base + 2] = arr[2];
            output[base + 3] = arr[3];
        }

        // Handle remainder with scalar path
        let remainder_start = full_chunks * 4;
        if remainder_start < n_points {
            let mut scalar_stack: Vec<f64> = Vec::with_capacity(self.stack_size);
            for (i, out) in output[remainder_start..n_points].iter_mut().enumerate() {
                let point_idx = remainder_start + i;
                scalar_stack.clear();
                for instr in self.instructions.iter() {
                    batch_fast_path!(instr, scalar_stack, columns, point_idx, self);
                }
                *out = scalar_stack.pop().unwrap_or(f64::NAN);
            }
        }
        Ok(())
    }

    #[inline(never)]
    #[cold]
    fn exec_slow_instruction(&self, instr: &Instruction, stack: &mut Vec<f64>) {
        process_instruction!(instr, stack, |_| unreachable!(
            "LoadParam should be handled in fast path"
        ));
    }

    #[inline(never)]
    #[cold]
    fn exec_slow_instruction_single(
        &self,
        instr: &Instruction,
        stack: &mut Vec<f64>,
        params: &[f64],
    ) {
        process_instruction!(instr, stack, |i| params[i]);
    }

    /// SIMD slow path for handling less common instructions
    /// Falls back to scalar computation for each of the 4 lanes
    #[inline(never)]
    #[cold]
    #[allow(clippy::ptr_arg)] // Vec is needed for potential push/pop in slow path
    fn exec_simd_slow_instruction(&self, instr: &Instruction, stack: &mut Vec<f64x4>) {
        // Safety: stack operations are validated at compile time by the Compiler
        debug_assert!(
            !stack.is_empty(),
            "Stack empty in SIMD slow path - compiler bug"
        );
        match *instr {
            // Inverse trig (compute per-lane)
            Instruction::Asin => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([arr[0].asin(), arr[1].asin(), arr[2].asin(), arr[3].asin()]);
            }
            Instruction::Acos => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([arr[0].acos(), arr[1].acos(), arr[2].acos(), arr[3].acos()]);
            }
            Instruction::Atan => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([arr[0].atan(), arr[1].atan(), arr[2].atan(), arr[3].atan()]);
            }
            Instruction::Acot => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                let acot = |x: f64| -> f64 {
                    if x.abs() < EPSILON {
                        std::f64::consts::PI / 2.0
                    } else if x > 0.0 {
                        (1.0 / x).atan()
                    } else {
                        (1.0 / x).atan() + std::f64::consts::PI
                    }
                };
                *top = f64x4::new([acot(arr[0]), acot(arr[1]), acot(arr[2]), acot(arr[3])]);
            }
            Instruction::Asec => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    (1.0 / arr[0]).acos(),
                    (1.0 / arr[1]).acos(),
                    (1.0 / arr[2]).acos(),
                    (1.0 / arr[3]).acos(),
                ]);
            }
            Instruction::Acsc => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    (1.0 / arr[0]).asin(),
                    (1.0 / arr[1]).asin(),
                    (1.0 / arr[2]).asin(),
                    (1.0 / arr[3]).asin(),
                ]);
            }
            Instruction::Cot => {
                let top = stack.last_mut().unwrap();
                *top = f64x4::ONE / top.tan();
            }
            Instruction::Sec => {
                let top = stack.last_mut().unwrap();
                *top = f64x4::ONE / top.cos();
            }
            Instruction::Csc => {
                let top = stack.last_mut().unwrap();
                *top = f64x4::ONE / top.sin();
            }
            // Hyperbolic (compute per-lane)
            Instruction::Sinh => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([arr[0].sinh(), arr[1].sinh(), arr[2].sinh(), arr[3].sinh()]);
            }
            Instruction::Cosh => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([arr[0].cosh(), arr[1].cosh(), arr[2].cosh(), arr[3].cosh()]);
            }
            Instruction::Tanh => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([arr[0].tanh(), arr[1].tanh(), arr[2].tanh(), arr[3].tanh()]);
            }
            Instruction::Asinh => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    arr[0].asinh(),
                    arr[1].asinh(),
                    arr[2].asinh(),
                    arr[3].asinh(),
                ]);
            }
            Instruction::Acosh => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    arr[0].acosh(),
                    arr[1].acosh(),
                    arr[2].acosh(),
                    arr[3].acosh(),
                ]);
            }
            Instruction::Atanh => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    arr[0].atanh(),
                    arr[1].atanh(),
                    arr[2].atanh(),
                    arr[3].atanh(),
                ]);
            }
            Instruction::Coth => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    1.0 / arr[0].tanh(),
                    1.0 / arr[1].tanh(),
                    1.0 / arr[2].tanh(),
                    1.0 / arr[3].tanh(),
                ]);
            }
            Instruction::Sech => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    1.0 / arr[0].cosh(),
                    1.0 / arr[1].cosh(),
                    1.0 / arr[2].cosh(),
                    1.0 / arr[3].cosh(),
                ]);
            }
            Instruction::Csch => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    1.0 / arr[0].sinh(),
                    1.0 / arr[1].sinh(),
                    1.0 / arr[2].sinh(),
                    1.0 / arr[3].sinh(),
                ]);
            }
            Instruction::Acoth => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                let acoth = |x: f64| 0.5 * ((x + 1.0) / (x - 1.0)).ln();
                *top = f64x4::new([acoth(arr[0]), acoth(arr[1]), acoth(arr[2]), acoth(arr[3])]);
            }
            Instruction::Asech => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    (1.0 / arr[0]).acosh(),
                    (1.0 / arr[1]).acosh(),
                    (1.0 / arr[2]).acosh(),
                    (1.0 / arr[3]).acosh(),
                ]);
            }
            Instruction::Acsch => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    (1.0 / arr[0]).asinh(),
                    (1.0 / arr[1]).asinh(),
                    (1.0 / arr[2]).asinh(),
                    (1.0 / arr[3]).asinh(),
                ]);
            }
            // Log functions
            Instruction::Log10 => {
                let top = stack.last_mut().unwrap();
                *top = top.log10();
            }
            Instruction::Log2 => {
                let top = stack.last_mut().unwrap();
                *top = top.log2();
            }
            Instruction::Cbrt => {
                let top = stack.last_mut().unwrap();
                *top = top.pow_f64x4(f64x4::splat(1.0 / 3.0));
            }
            // Rounding
            Instruction::Floor => {
                let top = stack.last_mut().unwrap();
                *top = top.floor();
            }
            Instruction::Ceil => {
                let top = stack.last_mut().unwrap();
                *top = top.ceil();
            }
            Instruction::Round => {
                let top = stack.last_mut().unwrap();
                *top = top.round();
            }
            Instruction::Signum => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    arr[0].signum(),
                    arr[1].signum(),
                    arr[2].signum(),
                    arr[3].signum(),
                ]);
            }
            // Special functions (unary) - compute per-lane using scalar implementations
            Instruction::Erf => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_erf(arr[0]),
                    crate::math::eval_erf(arr[1]),
                    crate::math::eval_erf(arr[2]),
                    crate::math::eval_erf(arr[3]),
                ]);
            }
            Instruction::Erfc => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    1.0 - crate::math::eval_erf(arr[0]),
                    1.0 - crate::math::eval_erf(arr[1]),
                    1.0 - crate::math::eval_erf(arr[2]),
                    1.0 - crate::math::eval_erf(arr[3]),
                ]);
            }
            Instruction::Gamma => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_gamma(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_gamma(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_gamma(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_gamma(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Digamma => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_digamma(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_digamma(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_digamma(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_digamma(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Trigamma => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_trigamma(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_trigamma(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_trigamma(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_trigamma(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Tetragamma => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_polygamma(3, arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_polygamma(3, arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_polygamma(3, arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_polygamma(3, arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Sinc => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                let sinc = |x: f64| if x.abs() < EPSILON { 1.0 } else { x.sin() / x };
                *top = f64x4::new([sinc(arr[0]), sinc(arr[1]), sinc(arr[2]), sinc(arr[3])]);
            }
            Instruction::LambertW => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_lambert_w(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_lambert_w(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_lambert_w(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_lambert_w(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::EllipticK => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_elliptic_k(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_k(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_k(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_k(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::EllipticE => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_elliptic_e(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_e(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_e(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_e(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Zeta => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_zeta(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_zeta(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_zeta(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_zeta(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::ExpPolar => {
                let top = stack.last_mut().unwrap();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_exp_polar(arr[0]),
                    crate::math::eval_exp_polar(arr[1]),
                    crate::math::eval_exp_polar(arr[2]),
                    crate::math::eval_exp_polar(arr[3]),
                ]);
            }
            // Two-argument functions (pop second operand, compute per-lane)
            Instruction::Log => {
                // log(base, x) = ln(x) / ln(base)
                let x = stack.pop().unwrap();
                let base = stack.last_mut().unwrap();
                let x_arr = x.to_array();
                let base_arr = base.to_array();
                let log_fn = |b: f64, v: f64| -> f64 {
                    if b <= 0.0 || b == 1.0 || v <= 0.0 {
                        f64::NAN
                    } else {
                        v.ln() / b.ln()
                    }
                };
                *base = f64x4::new([
                    log_fn(base_arr[0], x_arr[0]),
                    log_fn(base_arr[1], x_arr[1]),
                    log_fn(base_arr[2], x_arr[2]),
                    log_fn(base_arr[3], x_arr[3]),
                ]);
            }
            Instruction::Atan2 => {
                let x = stack.pop().unwrap();
                let y = stack.last_mut().unwrap();
                let x_arr = x.to_array();
                let y_arr = y.to_array();
                *y = f64x4::new([
                    y_arr[0].atan2(x_arr[0]),
                    y_arr[1].atan2(x_arr[1]),
                    y_arr[2].atan2(x_arr[2]),
                    y_arr[3].atan2(x_arr[3]),
                ]);
            }
            Instruction::BesselJ => {
                let x = stack.pop().unwrap();
                let n = stack.last_mut().unwrap();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                *n = f64x4::new([
                    crate::math::bessel_j(n_arr[0].round() as i32, x_arr[0]).unwrap_or(f64::NAN),
                    crate::math::bessel_j(n_arr[1].round() as i32, x_arr[1]).unwrap_or(f64::NAN),
                    crate::math::bessel_j(n_arr[2].round() as i32, x_arr[2]).unwrap_or(f64::NAN),
                    crate::math::bessel_j(n_arr[3].round() as i32, x_arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::BesselY => {
                let x = stack.pop().unwrap();
                let n = stack.last_mut().unwrap();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                *n = f64x4::new([
                    crate::math::bessel_y(n_arr[0].round() as i32, x_arr[0]).unwrap_or(f64::NAN),
                    crate::math::bessel_y(n_arr[1].round() as i32, x_arr[1]).unwrap_or(f64::NAN),
                    crate::math::bessel_y(n_arr[2].round() as i32, x_arr[2]).unwrap_or(f64::NAN),
                    crate::math::bessel_y(n_arr[3].round() as i32, x_arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::BesselI => {
                let x = stack.pop().unwrap();
                let n = stack.last_mut().unwrap();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                *n = f64x4::new([
                    crate::math::bessel_i(n_arr[0].round() as i32, x_arr[0]).unwrap_or(f64::NAN),
                    crate::math::bessel_i(n_arr[1].round() as i32, x_arr[1]).unwrap_or(f64::NAN),
                    crate::math::bessel_i(n_arr[2].round() as i32, x_arr[2]).unwrap_or(f64::NAN),
                    crate::math::bessel_i(n_arr[3].round() as i32, x_arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::BesselK => {
                let x = stack.pop().unwrap();
                let n = stack.last_mut().unwrap();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                *n = f64x4::new([
                    crate::math::bessel_k(n_arr[0].round() as i32, x_arr[0]).unwrap_or(f64::NAN),
                    crate::math::bessel_k(n_arr[1].round() as i32, x_arr[1]).unwrap_or(f64::NAN),
                    crate::math::bessel_k(n_arr[2].round() as i32, x_arr[2]).unwrap_or(f64::NAN),
                    crate::math::bessel_k(n_arr[3].round() as i32, x_arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Polygamma => {
                let x = stack.pop().unwrap();
                let n = stack.last_mut().unwrap();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                *n = f64x4::new([
                    crate::math::eval_polygamma(n_arr[0].round() as i32, x_arr[0])
                        .unwrap_or(f64::NAN),
                    crate::math::eval_polygamma(n_arr[1].round() as i32, x_arr[1])
                        .unwrap_or(f64::NAN),
                    crate::math::eval_polygamma(n_arr[2].round() as i32, x_arr[2])
                        .unwrap_or(f64::NAN),
                    crate::math::eval_polygamma(n_arr[3].round() as i32, x_arr[3])
                        .unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Beta => {
                let b = stack.pop().unwrap();
                let a = stack.last_mut().unwrap();
                let a_arr = a.to_array();
                let b_arr = b.to_array();
                let beta = |a: f64, b: f64| -> f64 {
                    match (
                        crate::math::eval_gamma(a),
                        crate::math::eval_gamma(b),
                        crate::math::eval_gamma(a + b),
                    ) {
                        (Some(ga), Some(gb), Some(gab)) => ga * gb / gab,
                        _ => f64::NAN,
                    }
                };
                *a = f64x4::new([
                    beta(a_arr[0], b_arr[0]),
                    beta(a_arr[1], b_arr[1]),
                    beta(a_arr[2], b_arr[2]),
                    beta(a_arr[3], b_arr[3]),
                ]);
            }
            Instruction::ZetaDeriv => {
                let s = stack.pop().unwrap();
                let n = stack.last_mut().unwrap();
                let s_arr = s.to_array();
                let n_arr = n.to_array();
                *n = f64x4::new([
                    crate::math::eval_zeta_deriv(n_arr[0].round() as i32, s_arr[0])
                        .unwrap_or(f64::NAN),
                    crate::math::eval_zeta_deriv(n_arr[1].round() as i32, s_arr[1])
                        .unwrap_or(f64::NAN),
                    crate::math::eval_zeta_deriv(n_arr[2].round() as i32, s_arr[2])
                        .unwrap_or(f64::NAN),
                    crate::math::eval_zeta_deriv(n_arr[3].round() as i32, s_arr[3])
                        .unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Hermite => {
                let x = stack.pop().unwrap();
                let n = stack.last_mut().unwrap();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                *n = f64x4::new([
                    crate::math::eval_hermite(n_arr[0].round() as i32, x_arr[0])
                        .unwrap_or(f64::NAN),
                    crate::math::eval_hermite(n_arr[1].round() as i32, x_arr[1])
                        .unwrap_or(f64::NAN),
                    crate::math::eval_hermite(n_arr[2].round() as i32, x_arr[2])
                        .unwrap_or(f64::NAN),
                    crate::math::eval_hermite(n_arr[3].round() as i32, x_arr[3])
                        .unwrap_or(f64::NAN),
                ]);
            }
            // Three-argument functions
            Instruction::AssocLegendre => {
                let x = stack.pop().unwrap();
                let m = stack.pop().unwrap();
                let l = stack.last_mut().unwrap();
                let x_arr = x.to_array();
                let m_arr = m.to_array();
                let l_arr = l.to_array();
                *l = f64x4::new([
                    crate::math::eval_assoc_legendre(
                        l_arr[0].round() as i32,
                        m_arr[0].round() as i32,
                        x_arr[0],
                    )
                    .unwrap_or(f64::NAN),
                    crate::math::eval_assoc_legendre(
                        l_arr[1].round() as i32,
                        m_arr[1].round() as i32,
                        x_arr[1],
                    )
                    .unwrap_or(f64::NAN),
                    crate::math::eval_assoc_legendre(
                        l_arr[2].round() as i32,
                        m_arr[2].round() as i32,
                        x_arr[2],
                    )
                    .unwrap_or(f64::NAN),
                    crate::math::eval_assoc_legendre(
                        l_arr[3].round() as i32,
                        m_arr[3].round() as i32,
                        x_arr[3],
                    )
                    .unwrap_or(f64::NAN),
                ]);
            }
            // Four-argument functions
            Instruction::SphericalHarmonic => {
                let phi = stack.pop().unwrap();
                let theta = stack.pop().unwrap();
                let m = stack.pop().unwrap();
                let l = stack.last_mut().unwrap();
                let phi_arr = phi.to_array();
                let theta_arr = theta.to_array();
                let m_arr = m.to_array();
                let l_arr = l.to_array();
                *l = f64x4::new([
                    crate::math::eval_spherical_harmonic(
                        l_arr[0].round() as i32,
                        m_arr[0].round() as i32,
                        theta_arr[0],
                        phi_arr[0],
                    )
                    .unwrap_or(f64::NAN),
                    crate::math::eval_spherical_harmonic(
                        l_arr[1].round() as i32,
                        m_arr[1].round() as i32,
                        theta_arr[1],
                        phi_arr[1],
                    )
                    .unwrap_or(f64::NAN),
                    crate::math::eval_spherical_harmonic(
                        l_arr[2].round() as i32,
                        m_arr[2].round() as i32,
                        theta_arr[2],
                        phi_arr[2],
                    )
                    .unwrap_or(f64::NAN),
                    crate::math::eval_spherical_harmonic(
                        l_arr[3].round() as i32,
                        m_arr[3].round() as i32,
                        theta_arr[3],
                        phi_arr[3],
                    )
                    .unwrap_or(f64::NAN),
                ]);
            }
            // Remaining cases that should be in fast path but might slip through
            _ => {
                // This should only be reached for LoadConst, LoadParam, Add, Mul, Div, Pow,
                // Neg, Sin, Cos, Tan, Sqrt, Exp, Ln, Abs, Square, Cube, Recip
                // which are all handled in simd_batch_fast_path macro.
                // If we get here, it's a bug - log a warning in debug builds
                #[cfg(debug_assertions)]
                eprintln!(
                    "Warning: Unhandled SIMD instruction {:?}, returning NaN",
                    instr
                );
                let top = stack.last_mut().unwrap();
                *top = f64x4::new([f64::NAN, f64::NAN, f64::NAN, f64::NAN]);
            }
        }
    }

    /// Parallel batch evaluation - evaluate expression at multiple data points in parallel
    ///
    /// Similar to `eval_batch`, but processes data points in parallel using Rayon.
    /// Best for large datasets (>256 points) where parallel overhead is justified.
    ///
    /// # Parameters
    /// - `columns`: Columnar data, where `columns[param_idx][point_idx]` gives the value
    ///   of parameter `param_idx` at data point `point_idx`
    ///
    /// # Returns
    /// Vec of evaluation results for each data point
    ///
    /// # Errors
    /// - `EvalColumnMismatch` if `columns.len()` != `param_count()`
    /// - `EvalColumnLengthMismatch` if column lengths don't all match
    #[cfg(feature = "parallel")]
    pub fn eval_batch_parallel(&self, columns: &[&[f64]]) -> Result<Vec<f64>, DiffError> {
        use rayon::prelude::*;

        if columns.len() != self.param_names.len() {
            return Err(DiffError::EvalColumnMismatch {
                expected: self.param_names.len(),
                got: columns.len(),
            });
        }

        let n_points = if columns.is_empty() {
            1
        } else {
            columns[0].len()
        };

        if !columns.iter().all(|c| c.len() == n_points) {
            return Err(DiffError::EvalColumnLengthMismatch);
        }

        // For small point counts, fall back to sequential to avoid overhead
        const MIN_PARALLEL_SIZE: usize = 256;
        if n_points < MIN_PARALLEL_SIZE {
            let mut output = vec![0.0; n_points];
            self.eval_batch(columns, &mut output)?;
            return Ok(output);
        }

        // Parallel chunked evaluation using SIMD-enabled eval_batch
        // Each chunk slices the columns and calls eval_batch (uses f64x4 SIMD)
        let mut output = vec![0.0; n_points];
        output
            .par_chunks_mut(MIN_PARALLEL_SIZE)
            .enumerate()
            .for_each(|(chunk_idx, chunk)| {
                let start = chunk_idx * MIN_PARALLEL_SIZE;
                let end = start + chunk.len();
                // Slice columns for this chunk
                let col_slices: Vec<&[f64]> = columns.iter().map(|col| &col[start..end]).collect();
                // Note: eval_batch cannot fail here because we've already validated inputs
                self.eval_batch(&col_slices, chunk)
                    .expect("eval_batch failed in parallel chunk");
            });
        Ok(output)
    }
}

/// Maximum allowed stack depth to prevent deeply nested expressions from causing issues
const MAX_STACK_DEPTH: usize = 1024;

/// Internal compiler state
struct Compiler<'a> {
    instructions: Vec<Instruction>,
    param_map: HashMap<&'a str, usize>,
    current_stack: usize,
    max_stack: usize,
    function_context: Option<&'a Context>,
}

impl<'a> Compiler<'a> {
    fn new(param_order: &[&'a str], context: Option<&'a Context>) -> Self {
        let param_map: HashMap<&str, usize> = param_order
            .iter()
            .enumerate()
            .map(|(i, name)| (*name, i))
            .collect();

        Self {
            instructions: Vec::with_capacity(64),
            param_map,
            current_stack: 0,
            max_stack: 0,
            function_context: context,
        }
    }

    fn push(&mut self) -> Result<(), DiffError> {
        self.current_stack += 1;
        if self.current_stack > MAX_STACK_DEPTH {
            return Err(DiffError::StackOverflow {
                depth: self.current_stack,
                limit: MAX_STACK_DEPTH,
            });
        }
        self.max_stack = self.max_stack.max(self.current_stack);
        Ok(())
    }

    fn pop(&mut self) {
        self.current_stack = self.current_stack.saturating_sub(1);
    }

    fn emit(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }

    /// Try to evaluate a constant expression at compile time
    /// Returns None if the expression contains variables or unsupported functions
    fn try_eval_const(expr: &Expr) -> Option<f64> {
        match &expr.kind {
            ExprKind::Number(n) => Some(*n),
            ExprKind::Symbol(s) => crate::core::known_symbols::get_constant_value(s.as_str()),
            ExprKind::Sum(terms) => {
                let mut sum = 0.0;
                for term in terms {
                    sum += Self::try_eval_const(term)?;
                }
                Some(sum)
            }
            ExprKind::Product(factors) => {
                let mut product = 1.0;
                for factor in factors {
                    product *= Self::try_eval_const(factor)?;
                }
                Some(product)
            }
            ExprKind::Div(num, den) => {
                let n = Self::try_eval_const(num)?;
                let d = Self::try_eval_const(den)?;
                Some(n / d)
            }
            ExprKind::Pow(base, exp) => {
                let b = Self::try_eval_const(base)?;
                let e = Self::try_eval_const(exp)?;
                Some(b.powf(e))
            }
            ExprKind::FunctionCall { name, args } => {
                let arg_vals: Vec<f64> = args
                    .iter()
                    .filter_map(|a| Self::try_eval_const(a))
                    .collect();
                if arg_vals.len() != args.len() {
                    return None;
                }
                match (name.as_str(), arg_vals.as_slice()) {
                    ("sin", [x]) => Some(x.sin()),
                    ("cos", [x]) => Some(x.cos()),
                    ("tan", [x]) => Some(x.tan()),
                    ("exp", [x]) => Some(x.exp()),
                    ("ln", [x]) => Some(x.ln()),
                    ("log", [x]) => Some(x.ln()),
                    ("sqrt", [x]) => Some(x.sqrt()),
                    ("abs", [x]) => Some(x.abs()),
                    ("floor", [x]) => Some(x.floor()),
                    ("ceil", [x]) => Some(x.ceil()),
                    ("round", [x]) => Some(x.round()),
                    _ => None, // Unsupported function for constant folding
                }
            }
            _ => None,
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), DiffError> {
        // Try constant folding first - evaluate constant expressions at compile time
        if let Some(value) = Self::try_eval_const(expr) {
            self.emit(Instruction::LoadConst(value));
            self.push()?;
            return Ok(());
        }

        match &expr.kind {
            ExprKind::Number(n) => {
                self.emit(Instruction::LoadConst(*n));
                self.push()?;
            }

            ExprKind::Symbol(s) => {
                let name = s.as_str();
                // Handle known constants
                if let Some(value) = crate::core::known_symbols::get_constant_value(name) {
                    self.emit(Instruction::LoadConst(value));
                    self.push()?;
                } else if let Some(&idx) = self.param_map.get(name) {
                    // Look up in parameter map
                    self.emit(Instruction::LoadParam(idx));
                    self.push()?;
                } else {
                    return Err(DiffError::UnboundVariable(name.to_string()));
                }
            }

            ExprKind::Sum(terms) => {
                if terms.is_empty() {
                    self.emit(Instruction::LoadConst(0.0));
                    self.push()?;
                } else {
                    // Compile first term
                    self.compile_expr(&terms[0])?;
                    // Add remaining terms
                    for term in &terms[1..] {
                        self.compile_expr(term)?;
                        self.emit(Instruction::Add);
                        self.pop(); // Two operands -> one result
                    }
                }
            }

            ExprKind::Product(factors) => {
                if factors.is_empty() {
                    self.emit(Instruction::LoadConst(1.0));
                    self.push()?;
                } else {
                    // Check for negation pattern: Product([-1, x]) = -x
                    if factors.len() == 2
                        && let ExprKind::Number(n) = &factors[0].kind
                        && *n == -1.0
                    {
                        self.compile_expr(&factors[1])?;
                        self.emit(Instruction::Neg);
                        return Ok(());
                    }

                    // Compile first factor
                    self.compile_expr(&factors[0])?;
                    // Multiply remaining factors
                    for factor in &factors[1..] {
                        self.compile_expr(factor)?;
                        self.emit(Instruction::Mul);
                        self.pop();
                    }
                }
            }

            ExprKind::Div(num, den) => {
                self.compile_expr(num)?;
                self.compile_expr(den)?;
                self.emit(Instruction::Div);
                self.pop();
            }

            ExprKind::Pow(base, exp) => {
                // Check for fused instruction patterns
                if let ExprKind::Number(n) = &exp.kind {
                    if (*n - 2.0).abs() < 1e-10 {
                        // x^2 -> Square (faster than powf(2))
                        self.compile_expr(base)?;
                        self.emit(Instruction::Square);
                        return Ok(());
                    } else if (*n - 3.0).abs() < 1e-10 {
                        // x^3 -> Cube (faster than powf(3))
                        self.compile_expr(base)?;
                        self.emit(Instruction::Cube);
                        return Ok(());
                    } else if (*n + 1.0).abs() < 1e-10 {
                        // x^-1 -> Recip (faster than powf(-1))
                        self.compile_expr(base)?;
                        self.emit(Instruction::Recip);
                        return Ok(());
                    }
                }
                // General case
                self.compile_expr(base)?;
                self.compile_expr(exp)?;
                self.emit(Instruction::Pow);
                self.pop();
            }

            ExprKind::FunctionCall { name, args } => {
                let func_name = name.as_str();

                // Compile arguments first (in order for proper stack layout)
                for arg in args {
                    self.compile_expr(arg)?;
                }

                // Emit function instruction
                let instr = match (func_name, args.len()) {
                    // Trigonometric (unary)
                    ("sin", 1) => Instruction::Sin,
                    ("cos", 1) => Instruction::Cos,
                    ("tan", 1) => Instruction::Tan,
                    ("asin", 1) => Instruction::Asin,
                    ("acos", 1) => Instruction::Acos,
                    ("atan", 1) => Instruction::Atan,
                    ("cot", 1) => Instruction::Cot,
                    ("sec", 1) => Instruction::Sec,
                    ("csc", 1) => Instruction::Csc,
                    ("acot", 1) => Instruction::Acot,
                    ("asec", 1) => Instruction::Asec,
                    ("acsc", 1) => Instruction::Acsc,

                    ("sinh", 1) => Instruction::Sinh,
                    ("cosh", 1) => Instruction::Cosh,
                    ("tanh", 1) => Instruction::Tanh,
                    ("asinh", 1) => Instruction::Asinh,
                    ("acosh", 1) => Instruction::Acosh,
                    ("atanh", 1) => Instruction::Atanh,
                    ("coth", 1) => Instruction::Coth,
                    ("sech", 1) => Instruction::Sech,
                    ("csch", 1) => Instruction::Csch,
                    ("acoth", 1) => Instruction::Acoth,
                    ("asech", 1) => Instruction::Asech,
                    ("acsch", 1) => Instruction::Acsch,

                    // Exponential/Logarithmic (unary)
                    ("exp", 1) => Instruction::Exp,
                    ("ln", 1) => Instruction::Ln,
                    ("log10", 1) => Instruction::Log10,
                    ("log2", 1) => Instruction::Log2,
                    ("sqrt", 1) => Instruction::Sqrt,
                    ("cbrt", 1) => Instruction::Cbrt,

                    // Special functions (unary)
                    ("abs", 1) => Instruction::Abs,
                    ("signum", 1) => Instruction::Signum,
                    ("floor", 1) => Instruction::Floor,
                    ("ceil", 1) => Instruction::Ceil,
                    ("round", 1) => Instruction::Round,
                    ("erf", 1) => Instruction::Erf,
                    ("erfc", 1) => Instruction::Erfc,
                    ("gamma", 1) => Instruction::Gamma,
                    ("digamma", 1) => Instruction::Digamma,
                    ("trigamma", 1) => Instruction::Trigamma,
                    ("tetragamma", 1) => Instruction::Tetragamma,
                    ("sinc", 1) => Instruction::Sinc,
                    ("lambertw", 1) => Instruction::LambertW,
                    ("elliptic_k", 1) => Instruction::EllipticK,
                    ("elliptic_e", 1) => Instruction::EllipticE,
                    ("zeta", 1) => Instruction::Zeta,
                    ("exp_polar", 1) => Instruction::ExpPolar,

                    // Two-argument functions
                    ("log", 2) => {
                        self.pop(); // Two args -> one result
                        Instruction::Log
                    }
                    ("atan2", 2) => {
                        self.pop(); // Two args -> one result
                        Instruction::Atan2
                    }
                    ("besselj", 2) => {
                        self.pop();
                        Instruction::BesselJ
                    }
                    ("bessely", 2) => {
                        self.pop();
                        Instruction::BesselY
                    }
                    ("besseli", 2) => {
                        self.pop();
                        Instruction::BesselI
                    }
                    ("besselk", 2) => {
                        self.pop();
                        Instruction::BesselK
                    }
                    ("polygamma", 2) => {
                        self.pop();
                        Instruction::Polygamma
                    }
                    ("beta", 2) => {
                        self.pop();
                        Instruction::Beta
                    }
                    ("zeta_deriv", 2) => {
                        self.pop();
                        Instruction::ZetaDeriv
                    }
                    ("hermite", 2) => {
                        self.pop();
                        Instruction::Hermite
                    }

                    // Three-argument functions
                    ("assoc_legendre", 3) => {
                        self.pop();
                        self.pop();
                        Instruction::AssocLegendre
                    }

                    // Four-argument functions
                    ("spherical_harmonic", 4) | ("ynm", 4) => {
                        self.pop();
                        self.pop();
                        self.pop();
                        Instruction::SphericalHarmonic
                    }

                    _ => {
                        // Check if function exists in the context
                        if let Some(ctx) = self.function_context {
                            let sym = crate::core::symbol::symb(func_name);
                            let id = sym.id();

                            if let Some(user_fn) = ctx.get_user_fn_by_id(id) {
                                if user_fn.arity.contains(&args.len()) {
                                    // User function exists but has no symbolic body,
                                    // so we can't evaluate it numerically.
                                    // Return an error at compile time instead of
                                    // silently returning NaN at runtime.
                                    return Err(DiffError::UnsupportedFunction(format!(
                                        "{}: user function has no body for numeric evaluation. \
                                         Define a body with `with_function(.., body: Some(expr))`",
                                        func_name
                                    )));
                                } else {
                                    return Err(DiffError::UnsupportedFunction(format!(
                                        "{}: invalid arity (expected {:?}, got {})",
                                        func_name,
                                        user_fn.arity,
                                        args.len()
                                    )));
                                }
                            } else {
                                return Err(DiffError::UnsupportedFunction(func_name.to_string()));
                            }
                        } else {
                            return Err(DiffError::UnsupportedFunction(func_name.to_string()));
                        }
                    }
                };

                self.emit(instr);
            }

            ExprKind::Poly(poly) => {
                // Polynomial evaluation using Horner's method for efficiency
                // P(x) = a_n*x^n + a_{n-1}*x^{n-1} + ... + a_0
                // Horner: ((a_n*x + a_{n-1})*x + ...)*x + a_0

                let terms = poly.terms();
                if terms.is_empty() {
                    self.emit(Instruction::LoadConst(0.0));
                    self.push()?;
                    return Ok(());
                }

                // Sort terms by power descending for Horner's method
                let mut sorted_terms: Vec<_> = terms.to_vec();
                sorted_terms.sort_by(|a, b| b.0.cmp(&a.0));

                // Get max power
                let max_pow = sorted_terms[0].0;

                // For simple polynomial with Symbol base, we can use the param directly
                // For complex bases, we'd need to compile them and store/reload
                // (Currently only handle simple Symbol case)
                let base_param_idx = if let ExprKind::Symbol(s) = &poly.base().kind {
                    let name = s.as_str();
                    match name {
                        _ if crate::core::known_symbols::is_known_constant(name) => None, // Constants, not params
                        _ => self.param_map.get(name).copied(),
                    }
                } else {
                    None
                };

                match base_param_idx {
                    Some(idx) => {
                        // Fast path: base is a simple parameter, use Horner's method
                        // Start with highest coefficient
                        self.emit(Instruction::LoadConst(sorted_terms[0].1));
                        self.push()?;

                        let mut term_iter = sorted_terms.iter().skip(1).peekable();

                        for pow in (0..max_pow).rev() {
                            // Multiply by x
                            self.emit(Instruction::LoadParam(idx));
                            self.push()?;
                            self.emit(Instruction::Mul);
                            self.pop();

                            // Add coefficient if this power exists
                            if term_iter.peek().is_some_and(|(p, _)| *p == pow) {
                                let (_, coeff) = term_iter.next().unwrap();
                                self.emit(Instruction::LoadConst(*coeff));
                                self.push()?;
                                self.emit(Instruction::Add);
                                self.pop();
                            }
                        }
                    }
                    None => {
                        // Slow path: expand the polynomial explicitly
                        // Evaluate as sum of coeff * base^power
                        // OPTIMIZATION: Cache base instructions instead of recompiling for each term
                        let base = poly.base();

                        // 1. Compile base once to learn instructions and stack usage
                        let base_start_stack = self.current_stack;
                        let base_start_instruction = self.instructions.len();

                        self.compile_expr(base)?;

                        let base_end_instruction = self.instructions.len();
                        // Calculate how much stack the base expression needs relative to its start
                        // max_stack tracks the global high-water mark, so (max_stack - start) covers
                        // the deepest excursion during base compilation.
                        let base_headroom = self.max_stack.saturating_sub(base_start_stack);

                        let base_instrs: Vec<Instruction> = self.instructions
                            [base_start_instruction..base_end_instruction]
                            .to_vec();

                        // 2. Reset state to before base compilation (truncate instructions)
                        self.instructions.truncate(base_start_instruction);
                        self.current_stack = base_start_stack;

                        // 3. Emit polynomial expansion using the cached instructions
                        // First term
                        let (first_pow, first_coeff) = sorted_terms[0];
                        self.emit(Instruction::LoadConst(first_coeff));
                        self.push()?;

                        // Replaying base: ensure we have enough stack space!
                        // We are at `current_stack`, and base needs `base_headroom` above that.
                        self.max_stack = self.max_stack.max(self.current_stack + base_headroom);

                        for instr in &base_instrs {
                            self.emit(*instr);
                        }
                        // Manually track the stack effect of the base expression (it pushes 1 value)
                        self.push()?;

                        self.emit(Instruction::LoadConst(first_pow as f64));
                        self.push()?;
                        self.emit(Instruction::Pow);
                        self.pop();
                        self.emit(Instruction::Mul);
                        self.pop();

                        // Remaining terms
                        for &(pow, coeff) in &sorted_terms[1..] {
                            self.emit(Instruction::LoadConst(coeff));
                            self.push()?;

                            // Replay base again
                            self.max_stack = self.max_stack.max(self.current_stack + base_headroom);
                            for instr in &base_instrs {
                                self.emit(*instr);
                            }
                            self.push()?;

                            self.emit(Instruction::LoadConst(pow as f64));
                            self.push()?;
                            self.emit(Instruction::Pow);
                            self.pop();
                            self.emit(Instruction::Mul);
                            self.pop();
                            self.emit(Instruction::Add);
                            self.pop();
                        }
                    }
                }
            }

            ExprKind::Derivative { .. } => {
                return Err(DiffError::UnsupportedExpression(
                    "Derivatives cannot be numerically evaluated - simplify first".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use std::collections::HashSet;

    fn parse_expr(s: &str) -> Expr {
        parser::parse(s, &HashSet::new(), &HashSet::new(), None).unwrap()
    }

    #[test]
    fn test_simple_arithmetic() {
        let expr = parse_expr("x + 2");
        let eval = CompiledEvaluator::compile(&expr, &["x"]).unwrap();
        assert!((eval.evaluate(&[3.0]) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_polynomial() {
        let expr = parse_expr("x^2 + 2*x + 1");
        let eval = CompiledEvaluator::compile(&expr, &["x"]).unwrap();
        assert!((eval.evaluate(&[3.0]) - 16.0).abs() < 1e-10);
    }

    #[test]
    fn test_trig() {
        let expr = parse_expr("sin(x)^2 + cos(x)^2");
        let eval = CompiledEvaluator::compile(&expr, &["x"]).unwrap();
        // Should always equal 1
        assert!((eval.evaluate(&[0.5]) - 1.0).abs() < 1e-10);
        assert!((eval.evaluate(&[1.23]) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_constants() {
        let expr = parse_expr("pi * e");
        let eval = CompiledEvaluator::compile(&expr, &[]).unwrap();
        let expected = std::f64::consts::PI * std::f64::consts::E;
        assert!((eval.evaluate(&[]) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_multi_var() {
        let expr = parse_expr("x * y + z");
        let eval = CompiledEvaluator::compile(&expr, &["x", "y", "z"]).unwrap();
        assert!((eval.evaluate(&[2.0, 3.0, 4.0]) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_unbound_variable_error() {
        let expr = parse_expr("x + y");
        let result = CompiledEvaluator::compile(&expr, &["x"]);
        assert!(matches!(result, Err(DiffError::UnboundVariable(_))));
    }

    #[test]
    fn test_compile_auto() {
        let expr = parse_expr("x^2 + y");
        let eval = CompiledEvaluator::compile_auto(&expr).unwrap();
        // Auto compilation sorts parameters alphabetically
        assert_eq!(eval.param_names(), &["x", "y"]);
    }

    // ===== Batch/SIMD Evaluation Tests =====

    #[test]
    fn test_eval_batch_simd_path() {
        // Tests the SIMD path (4+ points processed with f64x4)
        let expr = parse_expr("x^2 + 2*x + 1");
        let eval = CompiledEvaluator::compile(&expr, &["x"]).unwrap();

        // 8 points to ensure full SIMD chunks
        let x_vals: Vec<f64> = (0..8).map(|i| i as f64).collect();
        let columns: Vec<&[f64]> = vec![&x_vals];
        let mut output = vec![0.0; 8];

        eval.eval_batch(&columns, &mut output).unwrap();

        // Verify each result: (x+1)^2
        for (i, &result) in output.iter().enumerate() {
            let x = i as f64;
            let expected = (x + 1.0).powi(2);
            assert!(
                (result - expected).abs() < 1e-10,
                "Mismatch at i={}: got {}, expected {}",
                i,
                result,
                expected
            );
        }
    }

    #[test]
    fn test_eval_batch_remainder_path() {
        // Tests the scalar remainder path (points not divisible by 4)
        let expr = parse_expr("sin(x) + cos(x)");
        let eval = CompiledEvaluator::compile(&expr, &["x"]).unwrap();

        // 6 points: 4 SIMD + 2 remainder
        let x_vals: Vec<f64> = vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.5];
        let columns: Vec<&[f64]> = vec![&x_vals];
        let mut output = vec![0.0; 6];

        eval.eval_batch(&columns, &mut output).unwrap();

        for (i, &result) in output.iter().enumerate() {
            let x = x_vals[i];
            let expected = x.sin() + x.cos();
            assert!(
                (result - expected).abs() < 1e-10,
                "Mismatch at i={}: got {}, expected {}",
                i,
                result,
                expected
            );
        }
    }

    #[test]
    fn test_eval_batch_multi_var() {
        // Tests batch evaluation with multiple variables
        let expr = parse_expr("x * y + z");
        let eval = CompiledEvaluator::compile(&expr, &["x", "y", "z"]).unwrap();

        let x_vals = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y_vals = vec![2.0, 3.0, 4.0, 5.0, 6.0];
        let z_vals = vec![0.5, 1.0, 1.5, 2.0, 2.5];
        let columns: Vec<&[f64]> = vec![&x_vals, &y_vals, &z_vals];
        let mut output = vec![0.0; 5];

        eval.eval_batch(&columns, &mut output).unwrap();

        for i in 0..5 {
            let expected = x_vals[i] * y_vals[i] + z_vals[i];
            assert!(
                (output[i] - expected).abs() < 1e-10,
                "Mismatch at i={}: got {}, expected {}",
                i,
                output[i],
                expected
            );
        }
    }

    #[test]
    fn test_eval_batch_special_functions() {
        // Tests SIMD slow path for special functions
        let expr = parse_expr("exp(x) + sqrt(x)");
        let eval = CompiledEvaluator::compile(&expr, &["x"]).unwrap();

        let x_vals: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        let columns: Vec<&[f64]> = vec![&x_vals];
        let mut output = vec![0.0; 4];

        eval.eval_batch(&columns, &mut output).unwrap();

        for (i, &result) in output.iter().enumerate() {
            let x = x_vals[i];
            let expected = x.exp() + x.sqrt();
            assert!(
                (result - expected).abs() < 1e-10,
                "Mismatch at i={}: got {}, expected {}",
                i,
                result,
                expected
            );
        }
    }

    #[test]
    fn test_eval_batch_single_point() {
        // Edge case: single point (no SIMD, just remainder)
        let expr = parse_expr("x^2");
        let eval = CompiledEvaluator::compile(&expr, &["x"]).unwrap();

        let x_vals = vec![3.0];
        let columns: Vec<&[f64]> = vec![&x_vals];
        let mut output = vec![0.0; 1];

        eval.eval_batch(&columns, &mut output).unwrap();

        assert!((output[0] - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_eval_batch_constant_expr() {
        // Edge case: expression with no variables
        let expr = parse_expr("pi * 2");
        let eval = CompiledEvaluator::compile(&expr, &[]).unwrap();

        let columns: Vec<&[f64]> = vec![];
        let mut output = vec![0.0; 1];

        eval.eval_batch(&columns, &mut output).unwrap();

        let expected = std::f64::consts::PI * 2.0;
        assert!((output[0] - expected).abs() < 1e-10);
    }

    #[test]
    fn test_eval_batch_vs_single() {
        // Verify batch and single evaluation produce identical results
        let expr = parse_expr("sin(x) * cos(y) + exp(x/y)");
        let eval = CompiledEvaluator::compile(&expr, &["x", "y"]).unwrap();

        let x_vals: Vec<f64> = (1..=8).map(|i| i as f64 * 0.5).collect();
        let y_vals: Vec<f64> = (1..=8).map(|i| i as f64 * 0.3 + 0.1).collect();
        let columns: Vec<&[f64]> = vec![&x_vals, &y_vals];
        let mut batch_output = vec![0.0; 8];

        eval.eval_batch(&columns, &mut batch_output).unwrap();

        // Compare with single evaluations
        for i in 0..8 {
            let single_result = eval.evaluate(&[x_vals[i], y_vals[i]]);
            assert!(
                (batch_output[i] - single_result).abs() < 1e-10,
                "Batch/single mismatch at i={}: batch={}, single={}",
                i,
                batch_output[i],
                single_result
            );
        }
    }

    #[test]
    fn test_user_function_expansion() {
        use crate::core::unified_context::{Context, UserFunction};

        // Define f(x) = x^2 + 1
        let ctx = Context::new().with_function(
            "f",
            UserFunction::new(1..=1).body(|args| {
                let x = (*args[0]).clone();
                x.pow(2.0) + 1.0
            }),
        );

        // Create expression: f(x) + 2
        let x = crate::symb("x");
        let expr = Expr::func("f", x.to_expr()) + 2.0;

        // Compile with context - user function should be expanded
        let eval = CompiledEvaluator::compile_with_context(&expr, &["x"], Some(&ctx)).unwrap();

        // f(3) + 2 = (3^2 + 1) + 2 = 10 + 2 = 12
        let result = eval.evaluate(&[3.0]);
        assert!(
            (result - 12.0).abs() < 1e-10,
            "Expected 12.0, got {}",
            result
        );

        // f(0) + 2 = (0^2 + 1) + 2 = 1 + 2 = 3
        let result2 = eval.evaluate(&[0.0]);
        assert!(
            (result2 - 3.0).abs() < 1e-10,
            "Expected 3.0, got {}",
            result2
        );
    }

    #[test]
    fn test_nested_user_function_expansion() {
        use crate::core::unified_context::{Context, UserFunction};

        // Define g(x) = 2*x
        // Define f(x) = g(x) + 1  (nested call)
        let ctx = Context::new()
            .with_function(
                "g",
                UserFunction::new(1..=1).body(|args| 2.0 * (*args[0]).clone()),
            )
            .with_function(
                "f",
                UserFunction::new(1..=1).body(|args| {
                    // f(x) = g(x) + 1
                    Expr::func("g", (*args[0]).clone()) + 1.0
                }),
            );

        // Create expression: f(x)
        let x = crate::symb("x");
        let expr = Expr::func("f", x.to_expr());

        // Compile with context - nested function calls should be expanded
        let eval = CompiledEvaluator::compile_with_context(&expr, &["x"], Some(&ctx)).unwrap();

        // f(5) = g(5) + 1 = 2*5 + 1 = 11
        let result = eval.evaluate(&[5.0]);
        assert!(
            (result - 11.0).abs() < 1e-10,
            "Expected 11.0, got {}",
            result
        );
    }
}
