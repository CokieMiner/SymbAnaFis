//! Stack operations for the bytecode evaluator.
//!
//! This module provides safe and unsafe stack primitives for the evaluator.
//! The unsafe variants are used in performance-critical hot paths where the
//! compiler has already validated stack depth at compile time.
//!
//! # Safety Model
//!
//! The evaluator uses a two-phase approach to ensure stack safety:
//!
//! 1. **Compile-time validation**: The [`Compiler`](super::compiler::Compiler) tracks
//!    stack depth for every instruction and validates that:
//!    - Stack never underflows (depth never goes negative)
//!    - Stack never exceeds `MAX_STACK_DEPTH`
//!    - Final stack depth is exactly 1 (single result)
//!
//! 2. **Runtime assertions**: In debug builds, all unsafe operations include
//!    `debug_assert!` checks that will catch any compiler bugs.
//!
//! # Performance Notes
//!
//! The unsafe stack operations avoid bounds checking in release builds,
//! providing ~15-20% speedup on tight evaluation loops. This is safe because
//! the compiler has already proven the stack operations are valid.

use crate::core::traits::EPSILON;

// =============================================================================
// Scalar Stack Operations (f64)
// =============================================================================

/// Get mutable reference to the top of a scalar stack.
///
/// # Safety
///
/// The caller must ensure the stack is non-empty. In the evaluator context,
/// this is guaranteed by the compiler's stack depth tracking.
///
/// # Panics (Debug Only)
///
/// Panics in debug builds if the stack is empty.
#[allow(clippy::inline_always)] // Hot path: called millions of times in evaluation loops
#[allow(clippy::ptr_arg)] // Takes &mut Vec for API consistency with other stack ops that need Vec methods
#[inline(always)]
pub(super) unsafe fn scalar_stack_top_mut(stack: &mut Vec<f64>) -> &mut f64 {
    debug_assert!(!stack.is_empty(), "Stack empty - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees stack is non-empty, validated by debug_assert
    unsafe { stack.get_unchecked_mut(len - 1) }
}

/// Pop and return the top value from a scalar stack.
///
/// # Safety
///
/// The caller must ensure the stack is non-empty.
///
/// # Panics (Debug Only)
///
/// Panics in debug builds if the stack is empty.
#[allow(clippy::inline_always)] // Hot path: called millions of times in evaluation loops
#[inline(always)]
pub(super) unsafe fn scalar_stack_pop(stack: &mut Vec<f64>) -> f64 {
    debug_assert!(!stack.is_empty(), "Stack empty - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees stack is non-empty
    unsafe {
        let val = *stack.get_unchecked(len - 1);
        stack.set_len(len - 1);
        val
    }
}

/// Perform a binary operation on a scalar stack: `[a, b] → [a op b]`
///
/// # Safety
///
/// The caller must ensure the stack has at least 2 elements.
///
/// # Panics (Debug Only)
///
/// Panics in debug builds if stack has fewer than 2 elements.
#[allow(clippy::inline_always)] // Hot path: called millions of times in evaluation loops
#[inline(always)]
pub(super) unsafe fn scalar_stack_binop<F>(stack: &mut Vec<f64>, op: F)
where
    F: FnOnce(f64, f64) -> f64,
{
    debug_assert!(stack.len() >= 2, "Stack underflow - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees len >= 2
    unsafe {
        let b = *stack.get_unchecked(len - 1);
        let a = *stack.get_unchecked(len - 2);
        *stack.get_unchecked_mut(len - 2) = op(a, b);
        stack.set_len(len - 1);
    }
}

/// Perform an in-place binary operation: `[a, b] → [a op= b]`
///
/// This is an optimization for commutative operations like `+=` and `*=`.
///
/// # Safety
///
/// The caller must ensure the stack has at least 2 elements.
#[allow(clippy::inline_always)] // Hot path: called millions of times in evaluation loops
#[inline(always)]
pub(super) unsafe fn scalar_stack_binop_assign_add(stack: &mut Vec<f64>) {
    debug_assert!(stack.len() >= 2, "Stack underflow - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees len >= 2, validated by debug_assert
    unsafe {
        let b = *stack.get_unchecked(len - 1);
        *stack.get_unchecked_mut(len - 2) += b;
        stack.set_len(len - 1);
    }
}

/// Perform an in-place multiplication: `[a, b] → [a * b]`
///
/// # Safety
///
/// The caller must ensure the stack has at least 2 elements.
#[allow(clippy::inline_always)] // Hot path: called millions of times in evaluation loops
#[inline(always)]
pub(super) unsafe fn scalar_stack_binop_assign_mul(stack: &mut Vec<f64>) {
    debug_assert!(stack.len() >= 2, "Stack underflow - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees len >= 2, validated by debug_assert
    unsafe {
        let b = *stack.get_unchecked(len - 1);
        *stack.get_unchecked_mut(len - 2) *= b;
        stack.set_len(len - 1);
    }
}

/// Perform an in-place division: `[a, b] → [a / b]`
///
/// # Safety
///
/// The caller must ensure the stack has at least 2 elements.
#[allow(clippy::inline_always)] // Hot path: called millions of times in evaluation loops
#[inline(always)]
pub(super) unsafe fn scalar_stack_binop_assign_div(stack: &mut Vec<f64>) {
    debug_assert!(stack.len() >= 2, "Stack underflow - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees len >= 2, validated by debug_assert
    unsafe {
        let b = *stack.get_unchecked(len - 1);
        *stack.get_unchecked_mut(len - 2) /= b;
        stack.set_len(len - 1);
    }
}

// =============================================================================
// SIMD Stack Operations (f64x4)
// =============================================================================

use wide::f64x4;

/// Get mutable reference to the top of a SIMD stack.
///
/// # Safety
///
/// The caller must ensure the stack is non-empty.
#[allow(clippy::inline_always)] // Hot path: SIMD evaluation loop
#[allow(clippy::ptr_arg)] // Takes &mut Vec for API consistency with other stack ops that need Vec methods
#[inline(always)]
pub(super) unsafe fn simd_stack_top_mut(stack: &mut Vec<f64x4>) -> &mut f64x4 {
    debug_assert!(!stack.is_empty(), "SIMD stack empty - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees stack is non-empty, validated by debug_assert
    unsafe { stack.get_unchecked_mut(len - 1) }
}

/// Pop and return the top value from a SIMD stack.
///
/// # Safety
///
/// The caller must ensure the stack is non-empty.
///
/// # Panics (Debug Only)
///
/// Panics in debug builds if the stack is empty.
#[allow(clippy::inline_always)] // Used in slow path but still hot relative to other code
#[inline(always)]
pub(super) unsafe fn simd_stack_pop(stack: &mut Vec<f64x4>) -> f64x4 {
    debug_assert!(!stack.is_empty(), "SIMD stack empty - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees stack is non-empty
    unsafe {
        let val = *stack.get_unchecked(len - 1);
        stack.set_len(len - 1);
        val
    }
}

/// Perform an in-place addition on SIMD stack: `[a, b] → [a + b]`
///
/// # Safety
///
/// The caller must ensure the stack has at least 2 elements.
#[allow(clippy::inline_always)] // Hot path: SIMD evaluation loop
#[inline(always)]
pub(super) unsafe fn simd_stack_binop_add(stack: &mut Vec<f64x4>) {
    debug_assert!(stack.len() >= 2, "SIMD stack underflow - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees len >= 2, validated by debug_assert
    unsafe {
        let b = *stack.get_unchecked(len - 1);
        *stack.get_unchecked_mut(len - 2) += b;
        stack.set_len(len - 1);
    }
}

/// Perform an in-place multiplication on SIMD stack: `[a, b] → [a * b]`
///
/// # Safety
///
/// The caller must ensure the stack has at least 2 elements.
#[allow(clippy::inline_always)] // Hot path: SIMD evaluation loop
#[inline(always)]
pub(super) unsafe fn simd_stack_binop_mul(stack: &mut Vec<f64x4>) {
    debug_assert!(stack.len() >= 2, "SIMD stack underflow - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees len >= 2, validated by debug_assert
    unsafe {
        let b = *stack.get_unchecked(len - 1);
        *stack.get_unchecked_mut(len - 2) *= b;
        stack.set_len(len - 1);
    }
}

/// Perform an in-place division on SIMD stack: `[a, b] → [a / b]`
///
/// # Safety
///
/// The caller must ensure the stack has at least 2 elements.
#[allow(clippy::inline_always)] // Hot path: SIMD evaluation loop
#[inline(always)]
pub(super) unsafe fn simd_stack_binop_div(stack: &mut Vec<f64x4>) {
    debug_assert!(stack.len() >= 2, "SIMD stack underflow - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees len >= 2, validated by debug_assert
    unsafe {
        let b = *stack.get_unchecked(len - 1);
        *stack.get_unchecked_mut(len - 2) /= b;
        stack.set_len(len - 1);
    }
}

/// Perform power operation on SIMD stack: `[base, exp] → [base^exp]`
///
/// # Safety
///
/// The caller must ensure the stack has at least 2 elements.
#[allow(clippy::inline_always)] // Hot path: SIMD evaluation loop
#[inline(always)]
pub(super) unsafe fn simd_stack_binop_pow(stack: &mut Vec<f64x4>) {
    debug_assert!(stack.len() >= 2, "SIMD stack underflow - compiler bug");
    let len = stack.len();
    // SAFETY: Caller guarantees len >= 2, validated by debug_assert
    unsafe {
        let exp = *stack.get_unchecked(len - 1);
        let base = stack.get_unchecked_mut(len - 2);
        *base = base.pow_f64x4(exp);
        stack.set_len(len - 1);
    }
}

/// Perform fused multiply-add on SIMD stack: `[a, b, c] → [a * b + c]`
///
/// Uses hardware FMA when available.
///
/// # Safety
///
/// The caller must ensure the stack has at least 3 elements.
#[allow(clippy::inline_always)] // Hot path: SIMD evaluation loop, FMA optimization
#[inline(always)]
pub(super) unsafe fn simd_stack_muladd(stack: &mut Vec<f64x4>) {
    debug_assert!(
        stack.len() >= 3,
        "SIMD stack underflow for MulAdd - compiler bug"
    );
    let len = stack.len();
    // SAFETY: Caller guarantees len >= 3, validated by debug_assert
    unsafe {
        let c = *stack.get_unchecked(len - 1);
        let b = *stack.get_unchecked(len - 2);
        let a = *stack.get_unchecked(len - 3);
        *stack.get_unchecked_mut(len - 3) = a.mul_add(b, c);
        stack.set_len(len - 2);
    }
}

// =============================================================================
// Scalar MulAdd
// =============================================================================

/// Perform fused multiply-add on scalar stack: `[a, b, c] → [a * b + c]`
///
/// # Safety
///
/// The caller must ensure the stack has at least 3 elements.
#[allow(clippy::inline_always)] // Hot path: scalar evaluation loop, FMA optimization
#[inline(always)]
pub(super) unsafe fn scalar_stack_muladd(stack: &mut Vec<f64>) {
    debug_assert!(
        stack.len() >= 3,
        "Stack underflow for MulAdd - compiler bug"
    );
    let len = stack.len();
    // SAFETY: Caller guarantees len >= 3, validated by debug_assert
    unsafe {
        let c = *stack.get_unchecked(len - 1);
        let b = *stack.get_unchecked(len - 2);
        let a = stack.get_unchecked_mut(len - 3);
        *a = a.mul_add(b, c);
        stack.set_len(len - 2);
    }
}

// =============================================================================
// Helper Functions for Special Cases
// =============================================================================

/// Compute arccotangent with proper branch handling.
///
/// `acot(x)` is defined as:
/// - `π/2` for `x = 0`
/// - `atan(1/x)` for `x > 0`
/// - `atan(1/x) + π` for `x < 0`
#[inline]
pub(super) fn eval_acot(x: f64) -> f64 {
    if x.abs() < EPSILON {
        std::f64::consts::FRAC_PI_2
    } else if x > 0.0 {
        (1.0 / x).atan()
    } else {
        (1.0 / x).atan() + std::f64::consts::PI
    }
}

/// Compute inverse hyperbolic cotangent.
///
/// `acoth(x) = 0.5 * ln((x+1)/(x-1))` for `|x| > 1`
#[inline]
pub(super) fn eval_acoth(x: f64) -> f64 {
    0.5 * ((x + 1.0) / (x - 1.0)).ln()
}

/// Compute sinc function with removable singularity handling.
///
/// `sinc(x) = sin(x)/x` with `sinc(0) = 1`
#[inline]
pub(super) fn eval_sinc(x: f64) -> f64 {
    if x.abs() < EPSILON { 1.0 } else { x.sin() / x }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_acot() {
        // acot(0) = π/2
        assert!((eval_acot(0.0) - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
        // acot(1) = π/4
        assert!((eval_acot(1.0) - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
    }

    #[test]
    fn test_eval_sinc() {
        // sinc(0) = 1
        assert!((eval_sinc(0.0) - 1.0).abs() < 1e-10);
        // sinc(π) ≈ 0
        assert!(eval_sinc(std::f64::consts::PI).abs() < 1e-10);
    }
}
