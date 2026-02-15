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
/// 48 elements * 8 bytes = 384 bytes, fits comfortably in L1 cache.
/// Expressions with deeper stacks fall back to heap allocation.
const INLINE_STACK_SIZE: usize = 48;

/// Size of the inline CSE cache buffer.
///
/// 32 slots * 8 bytes = 256 bytes.
const INLINE_CACHE_SIZE: usize = 32;

impl CompiledEvaluator {
    /// Fast evaluation - no allocations in hot path, no tree traversal.
    ///
    /// # Parameters
    ///
    /// * `params` - Parameter values in the same order as `param_names()`
    ///   Missing parameters default to `0.0`. Extra values are ignored.
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
        // Super-fast path for constant expressions (e.g. "1 + 2" folded to "3")
        // Compiler always emits at least one instruction. Use get_unchecked for speed.
        if self.instructions.len() == 1
            // SAFETY: Length checked to be 1
            && let Instruction::LoadConst(idx) = unsafe { *self.instructions.get_unchecked(0) }
        {
            // SAFETY: Compiler guarantees valid constant index
            return unsafe { *self.constants.get_unchecked(idx as usize) };
        }

        // Fast path: use stack-allocated buffers for common expressions
        if self.stack_size <= INLINE_STACK_SIZE && self.cache_size <= INLINE_CACHE_SIZE {
            self.evaluate_inline(params)
        } else {
            // Large expression: fall back to heap-allocated Vec
            let mut stack: Vec<f64> = Vec::with_capacity(self.stack_size);
            let mut cache: Vec<f64> = vec![0.0; self.cache_size];
            // The general evaluate_with_cache now only handles the heap-allocated case
            self.evaluate_heap(params, &mut stack, &mut cache)
        }
    }

    /// Evaluate using inline (stack-allocated) buffers.
    ///
    /// This avoids all heap allocation for expressions that fit within
    /// the inline buffer sizes.
    //
    // Allow too_many_lines: This function handles the complete inline evaluation
    // fast path with all instruction types inlined for performance.
    #[allow(
        clippy::too_many_lines,
        reason = "Complete inline evaluation fast path with all instruction types inlined for performance"
    )]
    #[inline]
    fn evaluate_inline(&self, params: &[f64]) -> f64 {
        use std::mem::MaybeUninit;

        let mut inline_stack: [MaybeUninit<f64>; INLINE_STACK_SIZE] =
            [MaybeUninit::uninit(); INLINE_STACK_SIZE];
        let mut inline_cache: [MaybeUninit<f64>; INLINE_CACHE_SIZE] =
            [MaybeUninit::uninit(); INLINE_CACHE_SIZE];
        let mut len = 0_usize;

        // Use raw pointers for zero-overhead stack access
        let stack_ptr = inline_stack.as_mut_ptr().cast::<f64>();
        let cache_ptr = inline_cache.as_mut_ptr().cast::<f64>();

        // Pre-load slices to avoid repeated indirection
        let instrs = &*self.instructions;
        let consts = &*self.constants;

        for instr in instrs {
            // SAFETY: Compiler ensures max_stack <= INLINE_STACK_SIZE and cache_size <= INLINE_CACHE_SIZE.
            // Bytecode guarantees all reads are preceded by writes.
            unsafe {
                match *instr {
                    // Hot instructions first for better branch prediction
                    Instruction::LoadConst(c) => {
                        stack_ptr.add(len).write(*consts.get_unchecked(c as usize));
                        len += 1;
                    }
                    Instruction::LoadParam(p) => {
                        let idx = p as usize;
                        let value = if idx < params.len() {
                            *params.get_unchecked(idx)
                        } else {
                            0.0
                        };
                        stack_ptr.add(len).write(value);
                        len += 1;
                    }
                    Instruction::Add => {
                        len -= 1;
                        let b = stack_ptr.add(len).read();
                        let a_ptr = stack_ptr.add(len - 1);
                        *a_ptr += b;
                    }
                    Instruction::Mul => {
                        len -= 1;
                        let b = stack_ptr.add(len).read();
                        let a_ptr = stack_ptr.add(len - 1);
                        *a_ptr *= b;
                    }
                    Instruction::MulConst(c) => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr *= *consts.get_unchecked(c as usize);
                    }
                    Instruction::Sub => {
                        len -= 1;
                        let b = stack_ptr.add(len).read();
                        let a_ptr = stack_ptr.add(len - 1);
                        *a_ptr -= b;
                    }
                    Instruction::Div => {
                        len -= 1;
                        let b = stack_ptr.add(len).read();
                        let a_ptr = stack_ptr.add(len - 1);
                        *a_ptr /= b;
                    }
                    Instruction::Neg => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = -*top_ptr;
                    }
                    Instruction::Sin => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().sin();
                    }
                    Instruction::Cos => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().cos();
                    }
                    Instruction::Asin => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().asin();
                    }
                    Instruction::Acos => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().acos();
                    }
                    Instruction::Atan => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().atan();
                    }
                    Instruction::Exp => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().exp();
                    }
                    Instruction::Sqrt => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().sqrt();
                    }
                    Instruction::Cbrt => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().cbrt();
                    }
                    Instruction::AddConst(idx) => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr += *consts.get_unchecked(idx as usize);
                    }
                    Instruction::SubConst(idx) => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr -= *consts.get_unchecked(idx as usize);
                    }
                    Instruction::ConstSub(idx) => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = *consts.get_unchecked(idx as usize) - *top_ptr;
                    }
                    Instruction::Pow => {
                        len -= 1;
                        let exp = stack_ptr.add(len).read();
                        let base_ptr = stack_ptr.add(len - 1);
                        *base_ptr = base_ptr.read().powf(exp);
                    }
                    Instruction::Dup => {
                        let val = stack_ptr.add(len - 1).read();
                        stack_ptr.add(len).write(val);
                        len += 1;
                    }
                    Instruction::StoreCached(slot) => {
                        cache_ptr
                            .add(slot as usize)
                            .write(stack_ptr.add(len - 1).read());
                    }
                    Instruction::LoadCached(slot) => {
                        let val = cache_ptr.add(slot as usize).read();
                        stack_ptr.add(len).write(val);
                        len += 1;
                    }

                    // Common mathematical functions
                    Instruction::Tan => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().tan();
                    }
                    Instruction::Expm1 => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().exp_m1();
                    }
                    Instruction::ExpNeg => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = (-top_ptr.read()).exp();
                    }
                    Instruction::Ln => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().ln();
                    }
                    Instruction::Log1p => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().ln_1p();
                    }
                    Instruction::RecipExpm1 => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = 1.0 / top_ptr.read().exp_m1();
                    }
                    Instruction::ExpSqr => {
                        let top_ptr = stack_ptr.add(len - 1);
                        let x = *top_ptr;
                        *top_ptr = (x * x).exp();
                    }
                    Instruction::ExpSqrNeg => {
                        let top_ptr = stack_ptr.add(len - 1);
                        let x = *top_ptr;
                        *top_ptr = (-(x * x)).exp();
                    }
                    Instruction::Abs => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().abs();
                    }
                    Instruction::Signum => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().signum();
                    }
                    Instruction::Floor => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().floor();
                    }
                    Instruction::Ceil => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().ceil();
                    }
                    Instruction::Round => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().round();
                    }
                    Instruction::Sinc => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = stack::eval_sinc(top_ptr.read());
                    }
                    Instruction::Pop => {
                        len -= 1;
                    }
                    Instruction::Swap => {
                        let a_ptr = stack_ptr.add(len - 1);
                        let b_ptr = stack_ptr.add(len - 2);
                        let a = a_ptr.read();
                        let b = b_ptr.read();
                        a_ptr.write(b);
                        b_ptr.write(a);
                    }

                    // Fused operations
                    Instruction::Square => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr *= *top_ptr;
                    }
                    Instruction::Cube => {
                        let top_ptr = stack_ptr.add(len - 1);
                        let x = *top_ptr;
                        *top_ptr = x * x * x;
                    }
                    Instruction::Pow4 => {
                        let top_ptr = stack_ptr.add(len - 1);
                        let x = *top_ptr;
                        let x2 = x * x;
                        *top_ptr = x2 * x2;
                    }
                    Instruction::Pow3_2 => {
                        let top_ptr = stack_ptr.add(len - 1);
                        let x = *top_ptr;
                        *top_ptr = x * x.sqrt();
                    }
                    Instruction::InvPow3_2 => {
                        let top_ptr = stack_ptr.add(len - 1);
                        let x = *top_ptr;
                        *top_ptr = 1.0 / (x * x.sqrt());
                    }
                    Instruction::InvSqrt => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = 1.0 / (*top_ptr).sqrt();
                    }
                    Instruction::InvSquare => {
                        let top_ptr = stack_ptr.add(len - 1);
                        let x = *top_ptr;
                        *top_ptr = 1.0 / (x * x);
                    }
                    Instruction::InvCube => {
                        let top_ptr = stack_ptr.add(len - 1);
                        let x = *top_ptr;
                        *top_ptr = 1.0 / (x * x * x);
                    }
                    Instruction::Recip => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = 1.0 / *top_ptr;
                    }
                    Instruction::Powi(n) => {
                        let top_ptr = stack_ptr.add(len - 1);
                        let x = top_ptr.read();
                        *top_ptr = x.powi(n);
                    }
                    Instruction::MulAdd => {
                        len -= 2;
                        let c = stack_ptr.add(len + 1).read();
                        let b = stack_ptr.add(len).read();
                        let a_ptr = stack_ptr.add(len - 1);
                        *a_ptr = a_ptr.read().mul_add(b, c);
                    }
                    Instruction::MulSub => {
                        len -= 2;
                        let c = stack_ptr.add(len + 1).read();
                        let b = stack_ptr.add(len).read();
                        let a_ptr = stack_ptr.add(len - 1);
                        *a_ptr = a_ptr.read().mul_add(b, -c);
                    }
                    Instruction::NegMulAdd => {
                        len -= 2;
                        let c = stack_ptr.add(len + 1).read();
                        let b = stack_ptr.add(len).read();
                        let a_ptr = stack_ptr.add(len - 1);
                        *a_ptr = (-a_ptr.read()).mul_add(b, c);
                    }
                    Instruction::PolyEval(idx) => {
                        let top_ptr = stack_ptr.add(len - 1);
                        let x = *top_ptr;
                        let start = idx as usize;
                        // Degree is stored as f64
                        #[allow(
                            clippy::cast_possible_truncation,
                            clippy::cast_sign_loss,
                            reason = "Degree is stored as f64 but always an integer, so cast is safe"
                        )]
                        let degree = *consts.get_unchecked(start) as usize;

                        let mut res = *consts.get_unchecked(start + 1);
                        for i in 1..=degree {
                            res = res.mul_add(x, *consts.get_unchecked(start + 1 + i));
                        }
                        *top_ptr = res;
                    }

                    // Hyperbolic
                    Instruction::Sinh => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().sinh();
                    }
                    Instruction::Cosh => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().cosh();
                    }
                    Instruction::Tanh => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().tanh();
                    }
                    Instruction::Asinh => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().asinh();
                    }
                    Instruction::Acosh => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().acosh();
                    }
                    Instruction::Atanh => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = top_ptr.read().atanh();
                    }

                    // Special functions (rare but handled inline to avoid heap fallback)
                    Instruction::Erf => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = crate::math::eval_erf(top_ptr.read());
                    }
                    Instruction::Erfc => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = 1.0 - crate::math::eval_erf(top_ptr.read());
                    }
                    Instruction::Gamma => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = crate::math::eval_gamma(top_ptr.read()).unwrap_or(f64::NAN);
                    }
                    Instruction::Digamma => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = crate::math::eval_digamma(top_ptr.read()).unwrap_or(f64::NAN);
                    }
                    Instruction::Trigamma => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = crate::math::eval_trigamma(top_ptr.read()).unwrap_or(f64::NAN);
                    }
                    Instruction::Tetragamma => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = crate::math::eval_tetragamma(top_ptr.read()).unwrap_or(f64::NAN);
                    }
                    Instruction::LambertW => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = crate::math::eval_lambert_w(top_ptr.read()).unwrap_or(f64::NAN);
                    }
                    Instruction::EllipticK => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = crate::math::eval_elliptic_k(top_ptr.read()).unwrap_or(f64::NAN);
                    }
                    Instruction::EllipticE => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = crate::math::eval_elliptic_e(top_ptr.read()).unwrap_or(f64::NAN);
                    }
                    Instruction::Zeta => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = crate::math::eval_zeta(top_ptr.read()).unwrap_or(f64::NAN);
                    }
                    Instruction::ExpPolar => {
                        let top_ptr = stack_ptr.add(len - 1);
                        *top_ptr = crate::math::eval_exp_polar(top_ptr.read());
                    }

                    // Two-argument special functions
                    Instruction::Log => {
                        len -= 1;
                        let x = stack_ptr.add(len).read();
                        let base_ptr = stack_ptr.add(len - 1);
                        #[allow(
                            clippy::float_cmp,
                            reason = "Exact comparison for base == 1.0 is mathematically intentional"
                        )]
                        let invalid = *base_ptr <= 0.0 || *base_ptr == 1.0 || x <= 0.0;
                        *base_ptr = if invalid { f64::NAN } else { x.log(*base_ptr) };
                    }
                    Instruction::Atan2 => {
                        len -= 1;
                        let x = stack_ptr.add(len).read();
                        let y_ptr = stack_ptr.add(len - 1);
                        *y_ptr = y_ptr.read().atan2(x);
                    }
                    Instruction::BesselJ => {
                        len -= 1;
                        let x = stack_ptr.add(len).read();
                        let n_ptr = stack_ptr.add(len - 1);
                        let n_val = n_ptr.read();
                        *n_ptr = if n_val.is_nan() {
                            f64::NAN
                        } else {
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Bessel order is always a small integer"
                            )]
                            let order = n_val.round() as i32;
                            crate::math::bessel_j(order, x).unwrap_or(f64::NAN)
                        };
                    }
                    Instruction::BesselY => {
                        len -= 1;
                        let x = stack_ptr.add(len).read();
                        let n_ptr = stack_ptr.add(len - 1);
                        let n_val = n_ptr.read();
                        *n_ptr = if n_val.is_nan() {
                            f64::NAN
                        } else {
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Bessel order is always a small integer"
                            )]
                            let order = n_val.round() as i32;
                            crate::math::bessel_y(order, x).unwrap_or(f64::NAN)
                        };
                    }
                    Instruction::BesselI => {
                        len -= 1;
                        let x = stack_ptr.add(len).read();
                        let n_ptr = stack_ptr.add(len - 1);
                        let n_val = n_ptr.read();
                        *n_ptr = if n_val.is_nan() {
                            f64::NAN
                        } else {
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Bessel order is always a small integer"
                            )]
                            let order = n_val.round() as i32;
                            crate::math::bessel_i(order, x)
                        };
                    }
                    Instruction::BesselK => {
                        len -= 1;
                        let x = stack_ptr.add(len).read();
                        let n_ptr = stack_ptr.add(len - 1);
                        let n_val = n_ptr.read();
                        *n_ptr = if n_val.is_nan() {
                            f64::NAN
                        } else {
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Bessel order is always a small integer"
                            )]
                            let order = n_val.round() as i32;
                            crate::math::bessel_k(order, x).unwrap_or(f64::NAN)
                        };
                    }
                    Instruction::Polygamma => {
                        len -= 1;
                        let x = stack_ptr.add(len).read();
                        let n_ptr = stack_ptr.add(len - 1);
                        let n_val = n_ptr.read();
                        *n_ptr = if n_val.is_nan() {
                            f64::NAN
                        } else {
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Polygamma order is always a small integer"
                            )]
                            let order = n_val.round() as i32;
                            crate::math::eval_polygamma(order, x).unwrap_or(f64::NAN)
                        };
                    }
                    Instruction::Beta => {
                        len -= 1;
                        let b = stack_ptr.add(len).read();
                        let a_ptr = stack_ptr.add(len - 1);
                        let ga = crate::math::eval_gamma(*a_ptr);
                        let gb = crate::math::eval_gamma(b);
                        let gab = crate::math::eval_gamma(*a_ptr + b);
                        *a_ptr = match (ga, gb, gab) {
                            (Some(ga), Some(gb), Some(gab)) => ga * gb / gab,
                            _ => f64::NAN,
                        };
                    }
                    Instruction::ZetaDeriv => {
                        len -= 1;
                        let s = stack_ptr.add(len).read();
                        let n_ptr = stack_ptr.add(len - 1);
                        let n_val = n_ptr.read();
                        *n_ptr = if n_val.is_nan() {
                            f64::NAN
                        } else {
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Zeta derivative order is always a small integer"
                            )]
                            let order = n_val.round() as i32;
                            crate::math::eval_zeta_deriv(order, s).unwrap_or(f64::NAN)
                        };
                    }
                    Instruction::Hermite => {
                        len -= 1;
                        let x = stack_ptr.add(len).read();
                        let n_ptr = stack_ptr.add(len - 1);
                        let n_val = n_ptr.read();
                        *n_ptr = if n_val.is_nan() {
                            f64::NAN
                        } else {
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Hermite degree is always a small integer"
                            )]
                            let degree = n_val.round() as i32;
                            crate::math::eval_hermite(degree, x).unwrap_or(f64::NAN)
                        };
                    }

                    // Three-argument functions
                    Instruction::AssocLegendre => {
                        len -= 2;
                        let x = stack_ptr.add(len + 1).read();
                        let m = stack_ptr.add(len).read();
                        let l_ptr = stack_ptr.add(len - 1);
                        let l_val = l_ptr.read();
                        *l_ptr = if l_val.is_nan() || m.is_nan() {
                            f64::NAN
                        } else {
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Associated Legendre degree is always a small integer"
                            )]
                            let l_int = l_val.round() as i32;
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Associated Legendre order is always a small integer"
                            )]
                            let m_int = m.round() as i32;
                            crate::math::eval_assoc_legendre(l_int, m_int, x).unwrap_or(f64::NAN)
                        };
                    }

                    // Four-argument functions
                    Instruction::SphericalHarmonic => {
                        len -= 3;
                        let phi = stack_ptr.add(len + 2).read();
                        let theta = stack_ptr.add(len + 1).read();
                        let m = stack_ptr.add(len).read();
                        let l_ptr = stack_ptr.add(len - 1);
                        let l_val = l_ptr.read();
                        *l_ptr = if l_val.is_nan() || m.is_nan() {
                            f64::NAN
                        } else {
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Spherical harmonic degree/order is always a small integer"
                            )]
                            let l_int = l_val.round() as i32;
                            #[allow(
                                clippy::cast_possible_truncation,
                                reason = "Spherical harmonic degree/order is always a small integer"
                            )]
                            let m_int = m.round() as i32;
                            crate::math::eval_spherical_harmonic(l_int, m_int, theta, phi)
                                .unwrap_or(f64::NAN)
                        };
                    }
                }
            }
        }

        if len > 0 {
            // SAFETY: len > 0 guaranteed by completion of instructions
            unsafe { stack_ptr.add(len - 1).read() }
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
    ///   Evaluate using heap-allocated buffers. For internal and advanced use.
    #[inline]
    #[allow(
        clippy::too_many_lines,
        reason = "Large match statement for instruction dispatch"
    )]
    #[allow(
        clippy::undocumented_unsafe_blocks,
        reason = "Stack operations are validated at compile time."
    )]
    pub(crate) fn evaluate_heap(
        &self,
        params: &[f64],
        stack: &mut Vec<f64>,
        cache: &mut [f64],
    ) -> f64 {
        stack.clear();

        let constants = &self.constants;
        for instr in &*self.instructions {
            match *instr {
                // Binary operations (hot path first)
                // SAFETY: Stack operations are validated at compile time.
                Instruction::Add => unsafe { stack::scalar_stack_binop_assign_add(stack) },
                // SAFETY: Stack operations are validated at compile time.
                Instruction::Mul => unsafe { stack::scalar_stack_binop_assign_mul(stack) },
                Instruction::MulConst(idx) => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top *= constants[idx as usize];
                }
                Instruction::AddConst(idx) => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top += constants[idx as usize];
                }
                Instruction::SubConst(idx) => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top -= constants[idx as usize];
                }
                Instruction::ConstSub(idx) => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = constants[idx as usize] - *top;
                }
                // SAFETY: Stack operations are validated at compile time.
                Instruction::Div => unsafe { stack::scalar_stack_binop_assign_div(stack) },
                // SAFETY: Stack operations are validated at compile time.
                Instruction::Sub => unsafe { stack::scalar_stack_binop_assign_sub(stack) },
                // SAFETY: Stack operations are validated at compile time.
                Instruction::Pow => unsafe {
                    stack::scalar_stack_binop(stack, f64::powf);
                },

                Instruction::LoadConst(idx) => stack.push(constants[idx as usize]),
                Instruction::LoadParam(i) => {
                    let idx = i as usize;
                    stack.push(if idx < params.len() { params[idx] } else { 0.0 });
                }

                // SAFETY: Stack operations are validated at compile time.
                Instruction::Pop => unsafe {
                    stack::scalar_stack_pop(stack);
                },
                // SAFETY: Stack operations are validated at compile time.
                Instruction::Swap => unsafe {
                    stack::scalar_stack_swap(stack);
                },

                // Fused operations
                Instruction::Square => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = *top * *top;
                }
                Instruction::Cube => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    let x = *top;
                    *top = x * x * x;
                }
                Instruction::Pow4 => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    let x = *top;
                    let x2 = x * x;
                    *top = x2 * x2;
                }
                Instruction::Pow3_2 => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    let x = *top;
                    *top = x * x.sqrt();
                }
                Instruction::InvPow3_2 => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    let x = *top;
                    *top = 1.0 / (x * x.sqrt());
                }
                Instruction::InvSqrt => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = 1.0 / top.sqrt();
                }
                Instruction::InvSquare => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    let x = *top;
                    *top = 1.0 / (x * x);
                }
                Instruction::InvCube => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    let x = *top;
                    *top = 1.0 / (x * x * x);
                }
                Instruction::Recip => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = 1.0 / *top;
                }
                Instruction::Powi(n) => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    let x = *top;
                    *top = x.powi(n);
                }
                // SAFETY: Stack operations are validated at compile time.
                Instruction::MulAdd => unsafe { stack::scalar_stack_muladd(stack) },
                // SAFETY: Stack operations are validated at compile time.
                Instruction::MulSub => unsafe { stack::scalar_stack_mulsub(stack) },
                // SAFETY: Stack operations are validated at compile time.
                Instruction::NegMulAdd => unsafe { stack::scalar_stack_neg_muladd(stack) },

                Instruction::PolyEval(idx) => {
                    // SAFETY: Stack operations are validated at compile time.
                    let x_ptr = unsafe { stack::scalar_stack_top_mut(stack) };
                    let x = *x_ptr;
                    let start = idx as usize;
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "Degree is stored as f64 but always an integer, so cast is safe"
                    )]
                    let degree = constants[start] as usize;

                    let mut res = constants[start + 1];
                    for i in 0..degree {
                        res = res.mul_add(x, constants[start + 2 + i]);
                    }
                    *x_ptr = res;
                }

                // Unary operations
                Instruction::Neg => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = -*top;
                }

                // Trigonometric
                Instruction::Sin => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.sin();
                }
                Instruction::Cos => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.cos();
                }
                Instruction::Tan => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.tan();
                }
                Instruction::Asin => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.asin();
                }
                Instruction::Acos => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.acos();
                }
                Instruction::Atan => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.atan();
                }

                // Hyperbolic
                Instruction::Sinh => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.sinh();
                }
                Instruction::Cosh => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.cosh();
                }
                Instruction::Tanh => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.tanh();
                }
                Instruction::Asinh => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.asinh();
                }
                Instruction::Acosh => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.acosh();
                }
                Instruction::Atanh => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.atanh();
                }

                // Exponential/Logarithmic
                Instruction::Exp => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.exp();
                }
                Instruction::Ln => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.ln();
                }
                Instruction::Expm1 => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.exp_m1();
                }
                Instruction::ExpNeg => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = (-*top).exp();
                }
                Instruction::Log1p => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.ln_1p();
                }
                Instruction::RecipExpm1 => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = 1.0 / top.exp_m1();
                }
                Instruction::ExpSqr => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    let x = *top;
                    *top = (x * x).exp();
                }
                Instruction::ExpSqrNeg => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    let x = *top;
                    *top = (-(x * x)).exp();
                }

                Instruction::Sqrt => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.sqrt();
                }
                Instruction::Cbrt => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.cbrt();
                }

                // Special functions (unary)
                Instruction::Abs => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.abs();
                }
                Instruction::Signum => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.signum();
                }
                Instruction::Floor => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.floor();
                }
                Instruction::Ceil => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.ceil();
                }
                Instruction::Round => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = top.round();
                }
                Instruction::Erf => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = crate::math::eval_erf(*top);
                }
                Instruction::Erfc => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = 1.0 - crate::math::eval_erf(*top);
                }
                Instruction::Gamma => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = crate::math::eval_gamma(*top).unwrap_or(f64::NAN);
                }
                Instruction::Digamma => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = crate::math::eval_digamma(*top).unwrap_or(f64::NAN);
                }
                Instruction::Trigamma => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = crate::math::eval_trigamma(*top).unwrap_or(f64::NAN);
                }
                Instruction::Tetragamma => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = crate::math::eval_tetragamma(*top).unwrap_or(f64::NAN);
                }
                Instruction::Sinc => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = stack::eval_sinc(*top);
                }
                Instruction::LambertW => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = crate::math::eval_lambert_w(*top).unwrap_or(f64::NAN);
                }
                Instruction::EllipticK => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = crate::math::eval_elliptic_k(*top).unwrap_or(f64::NAN);
                }
                Instruction::EllipticE => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = crate::math::eval_elliptic_e(*top).unwrap_or(f64::NAN);
                }
                Instruction::Zeta => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = crate::math::eval_zeta(*top).unwrap_or(f64::NAN);
                }
                Instruction::ExpPolar => {
                    // SAFETY: Stack operations are validated at compile time.
                    let top = unsafe { stack::scalar_stack_top_mut(stack) };
                    *top = crate::math::eval_exp_polar(*top);
                }

                // Two-argument functions
                Instruction::Log => {
                    // SAFETY: Stack operations are validated at compile time.
                    let x = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let base = unsafe { stack::scalar_stack_top_mut(stack) };
                    // Exact comparison for base == 1.0 is mathematically intentional
                    #[allow(
                        clippy::float_cmp,
                        reason = "Exact comparison for base == 1.0 is mathematically intentional"
                    )]
                    let invalid = *base <= 0.0 || *base == 1.0 || x <= 0.0;
                    *base = if invalid { f64::NAN } else { x.log(*base) };
                }
                Instruction::Atan2 => {
                    // SAFETY: Stack operations are validated at compile time.
                    let x = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let y = unsafe { stack::scalar_stack_top_mut(stack) };
                    *y = y.atan2(x);
                }
                Instruction::BesselJ => {
                    // SAFETY: Stack operations are validated at compile time.
                    let x = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let n = unsafe { stack::scalar_stack_top_mut(stack) };
                    *n = if (*n).is_nan() {
                        f64::NAN
                    } else {
                        // Bessel order is always a small integer
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Bessel order is always a small integer"
                        )]
                        let order = (*n).round() as i32;
                        crate::math::bessel_j(order, x).unwrap_or(f64::NAN)
                    };
                }
                Instruction::BesselY => {
                    // SAFETY: Stack operations are validated at compile time.
                    let x = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let n = unsafe { stack::scalar_stack_top_mut(stack) };
                    *n = if (*n).is_nan() {
                        f64::NAN
                    } else {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Bessel order is always a small integer"
                        )]
                        let order = (*n).round() as i32;
                        crate::math::bessel_y(order, x).unwrap_or(f64::NAN)
                    };
                }
                Instruction::BesselI => {
                    // SAFETY: Stack operations are validated at compile time.
                    let x = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let n = unsafe { stack::scalar_stack_top_mut(stack) };
                    *n = if (*n).is_nan() {
                        f64::NAN
                    } else {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Bessel order is always a small integer"
                        )]
                        let order = (*n).round() as i32;
                        crate::math::bessel_i(order, x)
                    };
                }
                Instruction::BesselK => {
                    // SAFETY: Stack operations are validated at compile time.
                    let x = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let n = unsafe { stack::scalar_stack_top_mut(stack) };
                    *n = if (*n).is_nan() {
                        f64::NAN
                    } else {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Bessel order is always a small integer"
                        )]
                        let order = (*n).round() as i32;
                        crate::math::bessel_k(order, x).unwrap_or(f64::NAN)
                    };
                }
                Instruction::Polygamma => {
                    // SAFETY: Stack operations are validated at compile time.
                    let x = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let n = unsafe { stack::scalar_stack_top_mut(stack) };
                    *n = if (*n).is_nan() {
                        f64::NAN
                    } else {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Polygamma order is always a small integer"
                        )]
                        let order = (*n).round() as i32;
                        crate::math::eval_polygamma(order, x).unwrap_or(f64::NAN)
                    };
                }
                Instruction::Beta => {
                    // SAFETY: Stack operations are validated at compile time.
                    let b = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
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
                    // SAFETY: Stack operations are validated at compile time.
                    let s = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let n = unsafe { stack::scalar_stack_top_mut(stack) };
                    *n = if (*n).is_nan() {
                        f64::NAN
                    } else {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Zeta derivative order is always a small integer"
                        )]
                        let order = (*n).round() as i32;
                        crate::math::eval_zeta_deriv(order, s).unwrap_or(f64::NAN)
                    };
                }
                Instruction::Hermite => {
                    // SAFETY: Stack operations are validated at compile time.
                    let x = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let n = unsafe { stack::scalar_stack_top_mut(stack) };
                    *n = if (*n).is_nan() {
                        f64::NAN
                    } else {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Hermite polynomial degree is always a small integer"
                        )]
                        let degree = (*n).round() as i32;
                        crate::math::eval_hermite(degree, x).unwrap_or(f64::NAN)
                    };
                }

                // Three-argument functions
                Instruction::AssocLegendre => {
                    // SAFETY: Stack operations are validated at compile time.
                    let x = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let m = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let l = unsafe { stack::scalar_stack_top_mut(stack) };
                    *l = if (*l).is_nan() || m.is_nan() {
                        f64::NAN
                    } else {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Associated Legendre degree is always a small integer"
                        )]
                        let l_int = (*l).round() as i32;
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Associated Legendre order is always a small integer"
                        )]
                        let m_int = m.round() as i32;
                        crate::math::eval_assoc_legendre(l_int, m_int, x).unwrap_or(f64::NAN)
                    };
                }

                // Four-argument functions
                Instruction::SphericalHarmonic => {
                    // SAFETY: Stack operations are validated at compile time.
                    let phi = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let theta = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let m = unsafe { stack::scalar_stack_pop(stack) };
                    // SAFETY: Stack operations are validated at compile time.
                    let l = unsafe { stack::scalar_stack_top_mut(stack) };
                    *l = if (*l).is_nan() || m.is_nan() {
                        f64::NAN
                    } else {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Spherical harmonic degree/order is always a small integer"
                        )]
                        let l_int = (*l).round() as i32;
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Spherical harmonic degree/order is always a small integer"
                        )]
                        let m_int = m.round() as i32;
                        crate::math::eval_spherical_harmonic(l_int, m_int, theta, phi)
                            .unwrap_or(f64::NAN)
                    };
                }
                // CSE instructions
                Instruction::Dup => {
                    // SAFETY: Stack is non-empty - validated at compile time.
                    let top_val = unsafe { stack::scalar_stack_top(stack) };
                    stack.push(top_val);
                }
                Instruction::StoreCached(slot) => {
                    // SAFETY: Stack is non-empty - validated at compile time.
                    cache[slot as usize] = unsafe { *stack::scalar_stack_top_mut(stack) };
                }
                Instruction::LoadCached(slot) => {
                    stack.push(cache[slot as usize]);
                }
            }
        }
        stack.pop().unwrap_or(f64::NAN)
    }
}
