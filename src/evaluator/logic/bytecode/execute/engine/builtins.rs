//! Centralized dispatch for builtin mathematical functions.
//!
//! This module decouples the VM execution loop from the implementation details
//! of each mathematical operation, improving modularity and maintainability.

use super::helpers::{eval_sinc, round_to_i32};
use crate::evaluator::logic::bytecode::FnOp;
use crate::math::{
    bessel_i, bessel_j, bessel_k, bessel_y, eval_assoc_legendre, eval_beta, eval_digamma,
    eval_elliptic_e, eval_elliptic_k, eval_erf, eval_erfc, eval_exp_polar, eval_gamma,
    eval_hermite, eval_lambert_w, eval_lgamma, eval_polygamma, eval_spherical_harmonic,
    eval_tetragamma, eval_trigamma, eval_zeta, eval_zeta_deriv,
};
#[cfg(feature = "parallel")]
use std::array::from_fn;
use std::f64::consts::FRAC_PI_2;

#[cfg(feature = "parallel")]
use wide::f64x4;

#[cold]
#[inline(never)]
fn unreachable_builtin(arity: usize, op: FnOp) -> f64 {
    debug_assert!(false, "Reached unreachable Builtin{arity} op: {op:?}");
    f64::NAN
}

#[cfg(feature = "parallel")]
#[cold]
#[inline(never)]
fn unreachable_simd_builtin(arity: usize, op: FnOp) -> f64x4 {
    debug_assert!(false, "Reached unreachable SIMD Builtin{arity} op: {op:?}");
    f64x4::splat(f64::NAN)
}

/// Dispatches a 1-argument builtin function for scalar evaluation.
#[inline]
pub fn eval_builtin1(op: FnOp, x: f64) -> f64 {
    match op {
        FnOp::Tan => x.tan(),
        FnOp::Cot => 1.0 / x.tan(),
        FnOp::Sec => 1.0 / x.cos(),
        FnOp::Csc => 1.0 / x.sin(),
        FnOp::Asin => x.asin(),
        FnOp::Acos => x.acos(),
        FnOp::Atan => x.atan(),
        FnOp::Acot => FRAC_PI_2 - x.atan(),
        FnOp::Asec => (1.0 / x).acos(),
        FnOp::Acsc => (1.0 / x).asin(),
        FnOp::Sinh => x.sinh(),
        FnOp::Cosh => x.cosh(),
        FnOp::Tanh => x.tanh(),
        FnOp::Coth => 1.0 / x.tanh(),
        FnOp::Sech => 1.0 / x.cosh(),
        FnOp::Csch => 1.0 / x.sinh(),
        FnOp::Asinh => x.asinh(),
        FnOp::Acosh => x.acosh(),
        FnOp::Atanh => x.atanh(),
        FnOp::Acoth => (1.0 / x).atanh(),
        FnOp::Acsch => (1.0 / x).asinh(),
        FnOp::Asech => (1.0 / x).acosh(),
        FnOp::Expm1 => x.exp_m1(),
        FnOp::ExpNeg => (-x).exp(),
        FnOp::Log1p => x.ln_1p(),
        FnOp::Cbrt => x.cbrt(),
        FnOp::Abs => x.abs(),
        FnOp::Signum => x.signum(),
        FnOp::Floor => x.floor(),
        FnOp::Ceil => x.ceil(),
        FnOp::Round => x.round(),
        FnOp::Erf => eval_erf(x),
        FnOp::Erfc => eval_erfc(x),
        FnOp::Gamma => eval_gamma(x),
        FnOp::Lgamma => eval_lgamma(x),
        FnOp::Digamma => eval_digamma(x),
        FnOp::Trigamma => eval_trigamma(x),
        FnOp::Tetragamma => eval_tetragamma(x),
        FnOp::Sinc => eval_sinc(x),
        FnOp::LambertW => eval_lambert_w(x),
        FnOp::EllipticK => eval_elliptic_k(x),
        FnOp::EllipticE => eval_elliptic_e(x),
        FnOp::Zeta => eval_zeta(x),
        FnOp::ExpPolar => eval_exp_polar(x),
        _ => unreachable_builtin(1, op),
    }
}

/// Dispatches a 2-argument builtin function for scalar evaluation.
#[inline]
pub fn eval_builtin2(op: FnOp, x1: f64, x2: f64) -> f64 {
    match op {
        FnOp::Atan2 => x1.atan2(x2),
        FnOp::Log =>
        {
            #[allow(
                clippy::float_cmp,
                reason = "Exact comparison with 0.0 and 1.0 is intentional for mathematical domain boundaries"
            )]
            if x1 <= 0.0 || x1 == 1.0 || x2 < 0.0 {
                f64::NAN
            } else {
                x2.log(x1)
            }
        }
        FnOp::BesselJ => round_to_i32(x1).map_or(f64::NAN, |n| bessel_j(n, x2)),
        FnOp::BesselY => round_to_i32(x1).map_or(f64::NAN, |n| bessel_y(n, x2)),
        FnOp::BesselI => round_to_i32(x1).map_or(f64::NAN, |n| bessel_i(n, x2)),
        FnOp::BesselK => round_to_i32(x1).map_or(f64::NAN, |n| bessel_k(n, x2)),
        FnOp::Polygamma => round_to_i32(x1).map_or(f64::NAN, |n| eval_polygamma(n, x2)),
        FnOp::Beta => eval_beta(x1, x2),
        FnOp::ZetaDeriv => round_to_i32(x1).map_or(f64::NAN, |n| eval_zeta_deriv(n, x2)),
        FnOp::Hermite => round_to_i32(x1).map_or(f64::NAN, |n| eval_hermite(n, x2)),
        _ => unreachable_builtin(2, op),
    }
}

/// Dispatches a 3-argument builtin function for scalar evaluation.
#[inline]
pub fn eval_builtin3(op: FnOp, x1: f64, x2: f64, x3: f64) -> f64 {
    match op {
        FnOp::AssocLegendre => match (round_to_i32(x1), round_to_i32(x2)) {
            (Some(l), Some(m)) => eval_assoc_legendre(l, m, x3),
            _ => f64::NAN,
        },
        _ => unreachable_builtin(3, op),
    }
}

/// Dispatches a 4-argument builtin function for scalar evaluation.
#[inline]
pub fn eval_builtin4(op: FnOp, x1: f64, x2: f64, x3: f64, x4: f64) -> f64 {
    match op {
        FnOp::SphericalHarmonic => match (round_to_i32(x1), round_to_i32(x2)) {
            (Some(l), Some(m)) => eval_spherical_harmonic(l, m, x3, x4),
            _ => f64::NAN,
        },
        _ => unreachable_builtin(4, op),
    }
}

/// Dispatches a 1-argument builtin function for SIMD evaluation.
///
/// SIMD speedup is achieved for arithmetic operations (Add, Mul, etc.) via vectorized dispatch macros.
/// Transcendental functions (sin, exp, gamma, etc.) are evaluated lane-by-lane in scalar code,
/// as there are no portable SIMD intrinsics for these or their vectorized equivalents don't do proper NaN propagation.
#[cfg(feature = "parallel")]
#[inline]
pub fn eval_builtin1_simd(op: FnOp, x: f64x4) -> f64x4 {
    if op == FnOp::Abs {
        return x.abs();
    }

    let arr = x.to_array();
    match op {
        FnOp::Tan => f64x4::new(arr.map(f64::tan)),
        FnOp::Cot => f64x4::splat(1.0) / f64x4::new(arr.map(f64::tan)),
        FnOp::Sec => f64x4::splat(1.0) / f64x4::new(arr.map(f64::cos)),
        FnOp::Csc => f64x4::splat(1.0) / f64x4::new(arr.map(f64::sin)),
        FnOp::Asin => f64x4::new(arr.map(f64::asin)),
        FnOp::Acos => f64x4::new(arr.map(f64::acos)),
        FnOp::Atan => f64x4::new(arr.map(f64::atan)),
        FnOp::Acot => f64x4::splat(FRAC_PI_2) - f64x4::from(arr.map(f64::atan)),
        FnOp::Asec => f64x4::new((f64x4::splat(1.0) / x).to_array().map(f64::acos)),
        FnOp::Acsc => f64x4::new((f64x4::splat(1.0) / x).to_array().map(f64::asin)),
        FnOp::Sinh => f64x4::new(arr.map(f64::sinh)),
        FnOp::Cosh => f64x4::new(arr.map(f64::cosh)),
        FnOp::Tanh => f64x4::new(arr.map(f64::tanh)),
        FnOp::Coth => f64x4::splat(1.0) / f64x4::new(arr.map(f64::tanh)),
        FnOp::Sech => f64x4::splat(1.0) / f64x4::new(arr.map(f64::cosh)),
        FnOp::Csch => f64x4::splat(1.0) / f64x4::new(arr.map(f64::sinh)),
        FnOp::Asinh => f64x4::new(arr.map(f64::asinh)),
        FnOp::Acosh => f64x4::new(arr.map(f64::acosh)),
        FnOp::Atanh => f64x4::new(arr.map(f64::atanh)),
        FnOp::Acoth => f64x4::new((f64x4::splat(1.0) / x).to_array().map(f64::atanh)),
        FnOp::Acsch => f64x4::new((f64x4::splat(1.0) / x).to_array().map(f64::asinh)),
        FnOp::Asech => f64x4::new((f64x4::splat(1.0) / x).to_array().map(f64::acosh)),
        FnOp::Expm1 => f64x4::new(arr.map(f64::exp_m1)),
        FnOp::ExpNeg => f64x4::new((-x).to_array().map(f64::exp)),
        FnOp::Log1p => f64x4::new(arr.map(f64::ln_1p)),
        FnOp::Cbrt => f64x4::new(arr.map(f64::cbrt)),
        FnOp::Signum => f64x4::new(arr.map(f64::signum)),
        FnOp::Floor => f64x4::new(arr.map(f64::floor)),
        FnOp::Ceil => f64x4::new(arr.map(f64::ceil)),
        FnOp::Round => f64x4::new(arr.map(f64::round)),
        FnOp::Erf => f64x4::new(arr.map(eval_erf)),
        FnOp::Erfc => f64x4::new(arr.map(eval_erfc)),
        FnOp::Gamma => f64x4::new(arr.map(eval_gamma)),
        FnOp::Lgamma => f64x4::new(arr.map(eval_lgamma)),
        FnOp::Digamma => f64x4::new(arr.map(eval_digamma)),
        FnOp::Trigamma => f64x4::new(arr.map(eval_trigamma)),
        FnOp::Tetragamma => f64x4::new(arr.map(eval_tetragamma)),
        FnOp::Sinc => f64x4::new(arr.map(eval_sinc)),
        FnOp::LambertW => f64x4::new(arr.map(eval_lambert_w)),
        FnOp::EllipticK => f64x4::new(arr.map(eval_elliptic_k)),
        FnOp::EllipticE => f64x4::new(arr.map(eval_elliptic_e)),
        FnOp::Zeta => f64x4::new(arr.map(eval_zeta)),
        FnOp::ExpPolar => f64x4::new(arr.map(eval_exp_polar)),
        _ => unreachable_simd_builtin(1, op),
    }
}

/// Dispatches a 2-argument builtin function for SIMD evaluation.
#[cfg(feature = "parallel")]
#[inline]
pub fn eval_builtin2_simd(op: FnOp, x1: f64x4, x2: f64x4) -> f64x4 {
    let arr1 = x1.to_array();
    let arr2 = x2.to_array();
    match op {
        FnOp::Atan2 => f64x4::new(from_fn(|i| arr1[i].atan2(arr2[i]))),
        FnOp::Log => {
            let l = |base: f64, val: f64| {
                #[allow(
                    clippy::float_cmp,
                    reason = "Exact comparison with 0.0 and 1.0 is intentional for mathematical domain boundaries"
                )]
                if base <= 0.0 || base == 1.0 || val < 0.0 {
                    f64::NAN
                } else {
                    val.log(base)
                }
            };
            f64x4::new(from_fn(|i| l(arr1[i], arr2[i])))
        }
        FnOp::BesselJ => {
            let f = |n_f: f64, val: f64| round_to_i32(n_f).map_or(f64::NAN, |n| bessel_j(n, val));
            f64x4::new(from_fn(|i| f(arr1[i], arr2[i])))
        }
        FnOp::BesselY => {
            let f = |n_f: f64, val: f64| round_to_i32(n_f).map_or(f64::NAN, |n| bessel_y(n, val));
            f64x4::new(from_fn(|i| f(arr1[i], arr2[i])))
        }
        FnOp::BesselI => {
            let f = |n_f: f64, val: f64| round_to_i32(n_f).map_or(f64::NAN, |n| bessel_i(n, val));
            f64x4::new(from_fn(|i| f(arr1[i], arr2[i])))
        }
        FnOp::BesselK => {
            let f = |n_f: f64, val: f64| round_to_i32(n_f).map_or(f64::NAN, |n| bessel_k(n, val));
            f64x4::new(from_fn(|i| f(arr1[i], arr2[i])))
        }
        FnOp::Polygamma => {
            let f =
                |n_f: f64, val: f64| round_to_i32(n_f).map_or(f64::NAN, |n| eval_polygamma(n, val));
            f64x4::new(from_fn(|i| f(arr1[i], arr2[i])))
        }
        FnOp::Beta => f64x4::new(from_fn(|i| eval_beta(arr1[i], arr2[i]))),
        FnOp::ZetaDeriv => {
            let f = |n_f: f64, val: f64| {
                round_to_i32(n_f).map_or(f64::NAN, |n| eval_zeta_deriv(n, val))
            };
            f64x4::new(from_fn(|i| f(arr1[i], arr2[i])))
        }
        FnOp::Hermite => {
            let f =
                |n_f: f64, val: f64| round_to_i32(n_f).map_or(f64::NAN, |n| eval_hermite(n, val));
            f64x4::new(from_fn(|i| f(arr1[i], arr2[i])))
        }
        _ => unreachable_simd_builtin(2, op),
    }
}

/// Dispatches a 3-argument builtin function for SIMD evaluation.
#[cfg(feature = "parallel")]
#[inline]
pub fn eval_builtin3_simd(op: FnOp, x1: f64x4, x2: f64x4, x3: f64x4) -> f64x4 {
    let arr1 = x1.to_array();
    let arr2 = x2.to_array();
    let arr3 = x3.to_array();
    match op {
        FnOp::AssocLegendre => {
            let f = |l_f: f64, m_f: f64, val: f64| match (round_to_i32(l_f), round_to_i32(m_f)) {
                (Some(l), Some(m)) => eval_assoc_legendre(l, m, val),
                _ => f64::NAN,
            };
            f64x4::new(from_fn(|i| f(arr1[i], arr2[i], arr3[i])))
        }
        _ => unreachable_simd_builtin(3, op),
    }
}

/// Dispatches a 4-argument builtin function for SIMD evaluation.
#[cfg(feature = "parallel")]
#[inline]
pub fn eval_builtin4_simd(op: FnOp, x1: f64x4, x2: f64x4, x3: f64x4, x4: f64x4) -> f64x4 {
    let arr1 = x1.to_array();
    let arr2 = x2.to_array();
    let arr3 = x3.to_array();
    let arr4 = x4.to_array();
    match op {
        FnOp::SphericalHarmonic => {
            let f =
                |l_f: f64, m_f: f64, t: f64, p: f64| match (round_to_i32(l_f), round_to_i32(m_f)) {
                    (Some(l), Some(m)) => eval_spherical_harmonic(l, m, t, p),
                    _ => f64::NAN,
                };
            f64x4::new(from_fn(|i| f(arr1[i], arr2[i], arr3[i], arr4[i])))
        }
        _ => unreachable_simd_builtin(4, op),
    }
}
