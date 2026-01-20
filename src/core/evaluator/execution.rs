//! Scalar evaluation implementation for the bytecode evaluator.
//!
//! This module provides the `evaluate` method for single-point evaluation.
//! It uses a stack-based virtual machine to execute bytecode instructions.
//!
//! # Performance Optimizations
//!
//! 1. **Inline stack**: For small expressions (≤32 stack depth), uses a
//!    fixed-size array on the CPU stack instead of heap allocation.
//!
//! 2. **Unsafe stack operations**: Stack bounds are validated at compile time,
//!    allowing bounds-check-free access in the hot path.
//!
//! 3. **Instruction dispatch**: Uses a match statement which compiles to an
//!    efficient jump table on most architectures.

use super::CompiledEvaluator;
use super::instruction::Instruction;
use super::stack;

/// Size of the inline stack buffer (on CPU stack, not heap).
///
/// 32 elements * 8 bytes = 256 bytes, fits comfortably in L1 cache.
/// Expressions with deeper stacks fall back to heap allocation.
const INLINE_STACK_SIZE: usize = 32;

/// Size of the inline CSE cache buffer.
///
/// 16 slots * 8 bytes = 128 bytes.
const INLINE_CACHE_SIZE: usize = 16;

impl CompiledEvaluator {
    /// Fast evaluation - no allocations in hot path, no tree traversal.
    ///
    /// # Parameters
    ///
    /// * `params` - Parameter values in the same order as `param_names()`
    ///
    /// # Returns
    ///
    /// The evaluation result. Returns `NaN` for expressions that evaluate
    /// to undefined values (e.g., `ln(-1)`, `1/0`).
    ///
    /// # Panics
    ///
    /// Panics only in debug builds if stack operations are invalid (indicates
    /// a compiler bug). Release builds propagate NaN instead.
    ///
    /// # Example
    ///
    /// ```
    /// use symb_anafis::{symb, CompiledEvaluator};
    ///
    /// let x = symb("x");
    /// let expr = x.pow(2.0) + 1.0;
    /// let eval = expr.compile().expect("compile");
    ///
    /// assert!((eval.evaluate(&[3.0]) - 10.0).abs() < 1e-10);
    /// ```
    #[inline]
    #[must_use]
    pub fn evaluate(&self, params: &[f64]) -> f64 {
        // Fast path: use stack-allocated buffers for common expressions
        if self.stack_size <= INLINE_STACK_SIZE && self.cache_size <= INLINE_CACHE_SIZE {
            self.evaluate_inline(params)
        } else {
            // Large expression: fall back to heap-allocated Vec
            let mut stack: Vec<f64> = Vec::with_capacity(self.stack_size);
            let mut cache: Vec<f64> = vec![0.0; self.cache_size];
            self.evaluate_with_cache(params, &mut stack, &mut cache)
        }
    }

    /// Evaluate using inline (stack-allocated) buffers.
    ///
    /// This avoids all heap allocation for expressions that fit within
    /// the inline buffer sizes.
    //
    // Allow too_many_lines: This function handles the complete inline evaluation
    // fast path with all instruction types inlined for performance.
    #[allow(clippy::too_many_lines)]
    #[inline]
    fn evaluate_inline(&self, params: &[f64]) -> f64 {
        let mut inline_stack = [0.0_f64; INLINE_STACK_SIZE];
        let mut inline_cache = [0.0_f64; INLINE_CACHE_SIZE];
        let mut len = 0_usize;

        for instr in self.instructions.iter() {
            // SAFETY: All stack operations are bounds-checked at compile time by the Compiler.
            // The compiler validates max_stack <= INLINE_STACK_SIZE before we reach this path.
            // Debug builds will catch any bugs via assertions. Using get_unchecked for hot path.
            unsafe {
                match *instr {
                    Instruction::LoadConst(c) => {
                        *inline_stack.get_unchecked_mut(len) =
                            *self.constants.get_unchecked(c as usize);
                        len += 1;
                    }
                    Instruction::LoadParam(p) => {
                        *inline_stack.get_unchecked_mut(len) = *params.get_unchecked(p as usize);
                        len += 1;
                    }
                    Instruction::Add => {
                        len -= 1;
                        *inline_stack.get_unchecked_mut(len - 1) +=
                            *inline_stack.get_unchecked(len);
                    }
                    Instruction::Mul => {
                        len -= 1;
                        *inline_stack.get_unchecked_mut(len - 1) *=
                            *inline_stack.get_unchecked(len);
                    }
                    Instruction::Div => {
                        len -= 1;
                        *inline_stack.get_unchecked_mut(len - 1) /=
                            *inline_stack.get_unchecked(len);
                    }
                    Instruction::Pow => {
                        len -= 1;
                        let base = *inline_stack.get_unchecked(len - 1);
                        let exp = *inline_stack.get_unchecked(len);
                        *inline_stack.get_unchecked_mut(len - 1) = base.powf(exp);
                    }
                    Instruction::Neg => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = -*top;
                    }
                    Instruction::Sin => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = top.sin();
                    }
                    Instruction::Cos => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = top.cos();
                    }
                    Instruction::Tan => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = top.tan();
                    }
                    Instruction::Sqrt => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = top.sqrt();
                    }
                    Instruction::Exp => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = top.exp();
                    }
                    Instruction::Ln => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = top.ln();
                    }
                    Instruction::Abs => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = top.abs();
                    }
                    Instruction::Square => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = *top * *top;
                    }
                    Instruction::Cube => {
                        let x = *inline_stack.get_unchecked(len - 1);
                        *inline_stack.get_unchecked_mut(len - 1) = x * x * x;
                    }
                    Instruction::Pow4 => {
                        let x = *inline_stack.get_unchecked(len - 1);
                        let x2 = x * x;
                        *inline_stack.get_unchecked_mut(len - 1) = x2 * x2;
                    }
                    Instruction::Recip => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = 1.0 / *top;
                    }
                    Instruction::Powi(n) => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = top.powi(i32::from(n));
                    }
                    Instruction::SinCos => {
                        let x = *inline_stack.get_unchecked(len - 1);
                        let (s, c) = x.sin_cos();
                        *inline_stack.get_unchecked_mut(len - 1) = c;
                        *inline_stack.get_unchecked_mut(len) = s;
                        len += 1;
                    }
                    Instruction::MulAdd => {
                        let c = *inline_stack.get_unchecked(len - 1);
                        let b = *inline_stack.get_unchecked(len - 2);
                        let a = *inline_stack.get_unchecked(len - 3);
                        *inline_stack.get_unchecked_mut(len - 3) = a.mul_add(b, c);
                        len -= 2;
                    }
                    Instruction::Sinh => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = top.sinh();
                    }
                    Instruction::Cosh => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = top.cosh();
                    }
                    Instruction::Tanh => {
                        let top = inline_stack.get_unchecked_mut(len - 1);
                        *top = top.tanh();
                    }
                    Instruction::Dup => {
                        *inline_stack.get_unchecked_mut(len) = *inline_stack.get_unchecked(len - 1);
                        len += 1;
                    }
                    Instruction::StoreCached(slot) => {
                        *inline_cache.get_unchecked_mut(slot as usize) =
                            *inline_stack.get_unchecked(len - 1);
                    }
                    Instruction::LoadCached(slot) => {
                        *inline_stack.get_unchecked_mut(len) =
                            *inline_cache.get_unchecked(slot as usize);
                        len += 1;
                    }
                    _ => {
                        // Slow path: expression uses uncommon instructions
                        // Fall back to heap-allocated Vec evaluation
                        let mut vec_stack: Vec<f64> = Vec::with_capacity(self.stack_size);
                        let mut vec_cache: Vec<f64> = vec![0.0; self.cache_size];
                        return self.evaluate_with_cache(params, &mut vec_stack, &mut vec_cache);
                    }
                }
            }
        }

        if len > 0 {
            // SAFETY: len > 0 guaranteed by check
            unsafe { *inline_stack.get_unchecked(len - 1) }
        } else {
            f64::NAN
        }
    }

    /// Evaluate using provided stack and cache buffers (avoids allocation).
    ///
    /// This method is useful when you want to reuse buffers across multiple
    /// evaluations to avoid repeated memory allocation.
    ///
    /// # Parameters
    ///
    /// * `params` - Parameter values in the same order as `param_names()`
    /// * `stack` - Pre-allocated stack buffer (will be cleared)
    /// * `cache` - Pre-allocated CSE cache buffer (must have `cache_size` elements)
    #[inline]
    pub fn evaluate_with_cache(
        &self,
        params: &[f64],
        stack: &mut Vec<f64>,
        cache: &mut [f64],
    ) -> f64 {
        stack.clear();

        let constants = &self.constants;
        for instr in self.instructions.iter() {
            match *instr {
                // CSE instructions need cache access
                Instruction::Dup => {
                    // SAFETY: Stack is non-empty after prior instructions pushed values.
                    // Stack depth validated at compile time by Compiler.
                    let top_val = unsafe { *stack::scalar_stack_top_mut(stack) };
                    stack.push(top_val);
                }
                Instruction::StoreCached(slot) => {
                    // SAFETY: Stack is non-empty - validated at compile time.
                    cache[slot as usize] = unsafe { *stack::scalar_stack_top_mut(stack) };
                }
                Instruction::LoadCached(slot) => {
                    stack.push(cache[slot as usize]);
                }
                // Delegate all other instructions to the execution helper
                _ => {
                    exec_instruction(*instr, stack, constants, params);
                }
            }
        }
        stack.pop().unwrap_or(f64::NAN)
    }
}

/// Execute a single instruction on the stack.
///
/// This is the slow path for instructions not handled in the inline fast path.
/// Shared between scalar remainder path and batch processing.
///
/// # Parameters
///
/// * `instr` - The instruction to execute
/// * `stack` - The value stack
/// * `constants` - The constant pool
/// * `params` - Parameter values (may be empty for batch path where `LoadParam` is handled separately)
///
/// # Safety Invariant
///
/// All unsafe stack operations in this function have identical safety requirements:
/// - Stack depth is validated at compile time by the `Compiler`
/// - The compiler tracks stack effects for every instruction
/// - Debug builds include runtime assertions to catch any compiler bugs
/// - Release builds use unchecked access for performance
//
// Allow undocumented_unsafe_blocks: All ~60 unsafe blocks in this match have the same
// safety invariant documented above. Individual comments would be redundant noise.
// Allow too_many_lines: This is a dispatch function for 60+ instruction types.
// Splitting would hurt code locality and readability.
#[allow(clippy::undocumented_unsafe_blocks)]
#[allow(clippy::too_many_lines)]
#[inline]
pub(super) fn exec_instruction(
    instr: Instruction,
    stack: &mut Vec<f64>,
    constants: &[f64],
    params: &[f64],
) {
    match instr {
        Instruction::LoadConst(idx) => stack.push(constants[idx as usize]),
        Instruction::LoadParam(i) => stack.push(params[i as usize]),

        // Binary operations
        Instruction::Add => unsafe { stack::scalar_stack_binop_assign_add(stack) },
        Instruction::Mul => unsafe { stack::scalar_stack_binop_assign_mul(stack) },
        Instruction::Div => unsafe { stack::scalar_stack_binop_assign_div(stack) },
        Instruction::Pow => unsafe {
            stack::scalar_stack_binop(stack, f64::powf);
        },

        // Fused operations
        Instruction::Square => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = *top * *top;
        }
        Instruction::Cube => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            let x = *top;
            *top = x * x * x;
        }
        Instruction::Pow4 => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            let x = *top;
            let x2 = x * x;
            *top = x2 * x2;
        }
        Instruction::Recip => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = 1.0 / *top;
        }
        Instruction::Powi(n) => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.powi(i32::from(n));
        }
        Instruction::SinCos => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            let x = *top;
            let (s, c) = x.sin_cos();
            *top = c;
            stack.push(s);
        }
        Instruction::MulAdd => unsafe { stack::scalar_stack_muladd(stack) },

        // Unary operations
        Instruction::Neg => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = -*top;
        }

        // Trigonometric
        Instruction::Sin => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.sin();
        }
        Instruction::Cos => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.cos();
        }
        Instruction::Tan => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.tan();
        }
        Instruction::Asin => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.asin();
        }
        Instruction::Acos => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.acos();
        }
        Instruction::Atan => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.atan();
        }
        Instruction::Cot => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = 1.0 / top.tan();
        }
        Instruction::Sec => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = 1.0 / top.cos();
        }
        Instruction::Csc => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = 1.0 / top.sin();
        }
        Instruction::Acot => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = stack::eval_acot(*top);
        }
        Instruction::Asec => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = (1.0 / *top).acos();
        }
        Instruction::Acsc => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = (1.0 / *top).asin();
        }

        // Hyperbolic
        Instruction::Sinh => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.sinh();
        }
        Instruction::Cosh => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.cosh();
        }
        Instruction::Tanh => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.tanh();
        }
        Instruction::Asinh => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.asinh();
        }
        Instruction::Acosh => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.acosh();
        }
        Instruction::Atanh => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.atanh();
        }
        Instruction::Coth => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = 1.0 / top.tanh();
        }
        Instruction::Sech => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = 1.0 / top.cosh();
        }
        Instruction::Csch => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = 1.0 / top.sinh();
        }
        Instruction::Acoth => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = stack::eval_acoth(*top);
        }
        Instruction::Asech => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = (1.0 / *top).acosh();
        }
        Instruction::Acsch => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = (1.0 / *top).asinh();
        }

        // Exponential/Logarithmic
        Instruction::Exp => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.exp();
        }
        Instruction::Ln => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.ln();
        }
        Instruction::Log10 => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.log10();
        }
        Instruction::Log2 => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.log2();
        }
        Instruction::Sqrt => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.sqrt();
        }
        Instruction::Cbrt => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.cbrt();
        }

        // Special functions (unary)
        Instruction::Abs => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.abs();
        }
        Instruction::Signum => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.signum();
        }
        Instruction::Floor => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.floor();
        }
        Instruction::Ceil => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.ceil();
        }
        Instruction::Round => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = top.round();
        }
        Instruction::Erf => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = crate::math::eval_erf(*top);
        }
        Instruction::Erfc => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = 1.0 - crate::math::eval_erf(*top);
        }
        Instruction::Gamma => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = crate::math::eval_gamma(*top).unwrap_or(f64::NAN);
        }
        Instruction::Digamma => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = crate::math::eval_digamma(*top).unwrap_or(f64::NAN);
        }
        Instruction::Trigamma => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = crate::math::eval_trigamma(*top).unwrap_or(f64::NAN);
        }
        Instruction::Tetragamma => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = crate::math::eval_polygamma(3, *top).unwrap_or(f64::NAN);
        }
        Instruction::Sinc => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = stack::eval_sinc(*top);
        }
        Instruction::LambertW => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = crate::math::eval_lambert_w(*top).unwrap_or(f64::NAN);
        }
        Instruction::EllipticK => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = crate::math::eval_elliptic_k(*top).unwrap_or(f64::NAN);
        }
        Instruction::EllipticE => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = crate::math::eval_elliptic_e(*top).unwrap_or(f64::NAN);
        }
        Instruction::Zeta => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = crate::math::eval_zeta(*top).unwrap_or(f64::NAN);
        }
        Instruction::ExpPolar => {
            let top = unsafe { stack::scalar_stack_top_mut(stack) };
            *top = crate::math::eval_exp_polar(*top);
        }

        // Two-argument functions
        Instruction::Log => {
            let x = unsafe { stack::scalar_stack_pop(stack) };
            let base = unsafe { stack::scalar_stack_top_mut(stack) };
            // Exact comparison for base == 1.0 is mathematically intentional
            #[allow(clippy::float_cmp)]
            let invalid = *base <= 0.0 || *base == 1.0 || x <= 0.0;
            *base = if invalid { f64::NAN } else { x.log(*base) };
        }
        Instruction::Atan2 => {
            let x = unsafe { stack::scalar_stack_pop(stack) };
            let y = unsafe { stack::scalar_stack_top_mut(stack) };
            *y = y.atan2(x);
        }
        Instruction::BesselJ => {
            let x = unsafe { stack::scalar_stack_pop(stack) };
            let n = unsafe { stack::scalar_stack_top_mut(stack) };
            // Bessel order is always a small integer
            #[allow(clippy::cast_possible_truncation)]
            let order = (*n).round() as i32;
            *n = crate::math::bessel_j(order, x).unwrap_or(f64::NAN);
        }
        Instruction::BesselY => {
            let x = unsafe { stack::scalar_stack_pop(stack) };
            let n = unsafe { stack::scalar_stack_top_mut(stack) };
            #[allow(clippy::cast_possible_truncation)]
            let order = (*n).round() as i32;
            *n = crate::math::bessel_y(order, x).unwrap_or(f64::NAN);
        }
        Instruction::BesselI => {
            let x = unsafe { stack::scalar_stack_pop(stack) };
            let n = unsafe { stack::scalar_stack_top_mut(stack) };
            #[allow(clippy::cast_possible_truncation)]
            let order = (*n).round() as i32;
            *n = crate::math::bessel_i(order, x);
        }
        Instruction::BesselK => {
            let x = unsafe { stack::scalar_stack_pop(stack) };
            let n = unsafe { stack::scalar_stack_top_mut(stack) };
            #[allow(clippy::cast_possible_truncation)]
            let order = (*n).round() as i32;
            *n = crate::math::bessel_k(order, x).unwrap_or(f64::NAN);
        }
        Instruction::Polygamma => {
            let x = unsafe { stack::scalar_stack_pop(stack) };
            let n = unsafe { stack::scalar_stack_top_mut(stack) };
            #[allow(clippy::cast_possible_truncation)]
            let order = (*n).round() as i32;
            *n = crate::math::eval_polygamma(order, x).unwrap_or(f64::NAN);
        }
        Instruction::Beta => {
            let b = unsafe { stack::scalar_stack_pop(stack) };
            let a = unsafe { stack::scalar_stack_top_mut(stack) };
            let ga = crate::math::eval_gamma(*a);
            let gb = crate::math::eval_gamma(b);
            let gab = crate::math::eval_gamma(*a + b);
            *a = match (ga, gb, gab) {
                (Some(ga), Some(gb), Some(gab)) => ga * gb / gab,
                _ => f64::NAN,
            };
        }
        Instruction::ZetaDeriv => {
            let s = unsafe { stack::scalar_stack_pop(stack) };
            let n = unsafe { stack::scalar_stack_top_mut(stack) };
            #[allow(clippy::cast_possible_truncation)]
            let order = (*n).round() as i32;
            *n = crate::math::eval_zeta_deriv(order, s).unwrap_or(f64::NAN);
        }
        Instruction::Hermite => {
            let x = unsafe { stack::scalar_stack_pop(stack) };
            let n = unsafe { stack::scalar_stack_top_mut(stack) };
            #[allow(clippy::cast_possible_truncation)]
            let degree = (*n).round() as i32;
            *n = crate::math::eval_hermite(degree, x).unwrap_or(f64::NAN);
        }

        // Three-argument functions
        Instruction::AssocLegendre => {
            let x = unsafe { stack::scalar_stack_pop(stack) };
            let m = unsafe { stack::scalar_stack_pop(stack) };
            let l = unsafe { stack::scalar_stack_top_mut(stack) };
            #[allow(clippy::cast_possible_truncation)]
            let l_int = (*l).round() as i32;
            #[allow(clippy::cast_possible_truncation)]
            let m_int = m.round() as i32;
            *l = crate::math::eval_assoc_legendre(l_int, m_int, x).unwrap_or(f64::NAN);
        }

        // Four-argument functions
        Instruction::SphericalHarmonic => {
            let phi = unsafe { stack::scalar_stack_pop(stack) };
            let theta = unsafe { stack::scalar_stack_pop(stack) };
            let m = unsafe { stack::scalar_stack_pop(stack) };
            let l = unsafe { stack::scalar_stack_top_mut(stack) };
            #[allow(clippy::cast_possible_truncation)]
            let l_int = (*l).round() as i32;
            #[allow(clippy::cast_possible_truncation)]
            let m_int = m.round() as i32;
            *l = crate::math::eval_spherical_harmonic(l_int, m_int, theta, phi).unwrap_or(f64::NAN);
        }

        // CSE instructions should be handled by caller
        Instruction::Dup | Instruction::StoreCached(_) | Instruction::LoadCached(_) => {
            // These are handled in the main evaluation loop with cache access
            debug_assert!(false, "CSE instructions should be handled by caller");
        }
    }
}
