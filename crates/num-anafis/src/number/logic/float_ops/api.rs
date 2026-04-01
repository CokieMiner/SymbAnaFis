#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "Maintain abstraction for non-Copy high-precision types"
)]
#![allow(
    clippy::missing_const_for_fn,
    reason = "Maintain consistency with backends where methods are not const"
)]

use crate::number::logic::int_math::IntRepr;
use core::cmp::Ordering;

#[cfg(any(feature = "backend_big_rug", feature = "backend_big_astro"))]
use crate::number::logic::int_math::int_clone;

// Priority order when multiple features are enabled:
// 1. backend_big_rug (highest precision, GMP-based)
// 2. backend_big_astro (arbitrary precision, pure Rust)
// 3. backend32 (f32, memory-optimized)
// 4. backend64 / default (f64)
//
// Note: When --all-features is used, priority order is applied automatically.
//       For production use, enable only ONE float backend feature.

#[cfg(feature = "backend_big_rug")]
use super::rug_ops as backend;

#[cfg(all(feature = "backend_big_astro", not(feature = "backend_big_rug")))]
use super::astro_ops as backend;

#[cfg(all(
    feature = "backend32",
    not(feature = "backend_big_astro"),
    not(feature = "backend_big_rug")
))]
use super::f32_ops as backend;

#[cfg(all(
    not(feature = "backend32"),
    not(feature = "backend_big_astro"),
    not(feature = "backend_big_rug")
))]
use super::f64_ops as backend;

pub type FloatRepr = backend::BackingFloat;

pub(in crate::number::logic) fn float_from_int(value: &IntRepr) -> FloatRepr {
    #[cfg(any(feature = "backend_big_rug", feature = "backend_big_astro"))]
    {
        backend::from_int(int_clone(value))
    }

    #[cfg(not(any(feature = "backend_big_rug", feature = "backend_big_astro")))]
    {
        backend::from_int(value)
    }
}

pub(in crate::number::logic) fn float_clone(value: &FloatRepr) -> FloatRepr {
    backend::clone(value)
}

pub(in crate::number::logic) fn float_add(lhs: &FloatRepr, rhs: &FloatRepr) -> FloatRepr {
    backend::add(lhs, rhs)
}

pub(in crate::number::logic) fn float_sub(lhs: &FloatRepr, rhs: &FloatRepr) -> FloatRepr {
    backend::sub(lhs, rhs)
}

pub(in crate::number::logic) fn float_mul(lhs: &FloatRepr, rhs: &FloatRepr) -> FloatRepr {
    backend::mul(lhs, rhs)
}

pub(in crate::number::logic) fn float_div(lhs: &FloatRepr, rhs: &FloatRepr) -> FloatRepr {
    backend::div(lhs, rhs)
}

pub(in crate::number::logic) fn float_neg(value: &FloatRepr) -> FloatRepr {
    backend::neg(value)
}

pub(in crate::number::logic) fn float_abs(value: &FloatRepr) -> FloatRepr {
    backend::abs(value)
}

pub(in crate::number::logic) fn float_pow(lhs: &FloatRepr, rhs: &FloatRepr) -> FloatRepr {
    backend::pow(lhs, rhs)
}

pub(in crate::number::logic) fn float_sqrt(value: &FloatRepr) -> FloatRepr {
    backend::sqrt(value)
}

pub(in crate::number::logic) fn float_cbrt(value: &FloatRepr) -> FloatRepr {
    backend::cbrt(value)
}

pub(in crate::number::logic) fn float_fract(value: &FloatRepr) -> FloatRepr {
    backend::fract(value)
}

pub(in crate::number::logic) fn float_round(value: &FloatRepr) -> FloatRepr {
    backend::round(value)
}

pub(in crate::number::logic) fn float_sin(value: &FloatRepr) -> FloatRepr {
    backend::sin(value)
}

pub(in crate::number::logic) fn float_cos(value: &FloatRepr) -> FloatRepr {
    backend::cos(value)
}

pub(in crate::number::logic) fn float_exp(value: &FloatRepr) -> FloatRepr {
    backend::exp(value)
}

pub(in crate::number::logic) fn float_ln(value: &FloatRepr) -> FloatRepr {
    backend::ln(value)
}

pub(in crate::number::logic) fn float_atan(value: &FloatRepr) -> FloatRepr {
    backend::atan(value)
}

pub(in crate::number::logic) fn float_is_zero(value: &FloatRepr) -> bool {
    backend::is_zero(value)
}

pub(in crate::number::logic) fn float_is_one(value: &FloatRepr) -> bool {
    backend::is_one(value)
}

pub(in crate::number::logic) fn float_is_neg_one(value: &FloatRepr) -> bool {
    backend::is_neg_one(value)
}

pub(in crate::number::logic) fn float_is_integer(value: &FloatRepr) -> bool {
    backend::is_integer(value)
}

pub(in crate::number::logic) fn float_is_finite(value: &FloatRepr) -> bool {
    backend::is_finite(value)
}

pub(in crate::number::logic) fn float_is_negative(value: &FloatRepr) -> bool {
    backend::is_negative(value)
}

pub(in crate::number::logic) fn float_is_positive(value: &FloatRepr) -> bool {
    backend::is_positive(value)
}

pub(in crate::number::logic) fn float_to_f64(value: &FloatRepr) -> f64 {
    backend::to_f64(value)
}

pub(in crate::number::logic) fn float_from_f64(value: f64) -> FloatRepr {
    backend::from_f64(value)
}

pub(in crate::number::logic) fn float_from_i64(value: i64) -> FloatRepr {
    backend::from_i64(value)
}

pub(in crate::number::logic) fn float_to_i64_exact(value: &FloatRepr) -> Option<i64> {
    backend::to_i64_exact(value)
}

pub(in crate::number::logic) fn float_cmp(lhs: &FloatRepr, rhs: &FloatRepr) -> Option<Ordering> {
    backend::cmp(lhs, rhs)
}
