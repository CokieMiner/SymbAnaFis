pub mod arithmetic;
pub mod basic_math;
pub mod special;

use alloc::string::String;
pub use arithmetic::*;
pub use basic_math::*;
use core::cmp::Ordering;
use core::sync::atomic::{AtomicU32, Ordering as MemOrder};
pub use special::*;

use rug::Float;

use crate::number::logic::int_math::IntType;
use crate::number::logic::rational_math::RationalType;

pub type BackingFloat = Float;

static PRECISION: AtomicU32 = AtomicU32::new(256);

/// Set the working precision (bits) for the rug backend.
/// Returns `true` on success, `false` if `bits` is less than 2.
pub fn set_precision(bits: u32) -> bool {
    if bits < 2 {
        return false;
    }
    PRECISION.store(bits, MemOrder::Relaxed);
    true
}

/// Return the current working precision in bits.
pub fn get_precision() -> u32 {
    PRECISION.load(MemOrder::Relaxed)
}

#[inline]
pub fn with_val<T>(v: T) -> BackingFloat
where
    Float: rug::ops::AssignRound<T, Round = rug::float::Round, Ordering = Ordering>,
{
    Float::with_val(get_precision(), v)
}

pub fn nan() -> BackingFloat {
    Float::with_val(get_precision(), rug::float::Special::Nan)
}

// ============================================================================
// Construction & conversion
// ============================================================================

#[inline]
pub fn from_int(value: &IntType) -> BackingFloat {
    with_val(value)
}
#[inline]
pub fn clone(value: &BackingFloat) -> BackingFloat {
    value.clone()
}
#[inline]
pub fn from_f32(v: f32) -> BackingFloat {
    with_val(v)
}
#[inline]
pub fn from_f64(v: f64) -> BackingFloat {
    with_val(v)
}
#[inline]
pub fn from_i64(v: i64) -> BackingFloat {
    with_val(v)
}
#[inline]
pub fn to_int(value: &BackingFloat) -> Option<IntType> {
    if value.is_integer() {
        value.to_integer()
    } else {
        None
    }
}

pub fn to_rational(value: &BackingFloat) -> Option<RationalType> {
    if !value.is_finite() {
        return None;
    }
    // rug::Float to rug::Rational is exact
    value.to_rational()
}

pub fn to_string(value: &BackingFloat) -> String {
    let prec = value.prec();
    // Convert bits to decimal digits:  prec * log₁₀(2) + 2 guard digits.
    // 30_103/100_000 ≈ log₁₀(2) = 0.30102999…, the +2 accounts for the
    // leading digit and a safety margin (Goldberg 1991, "What Every Computer
    // Scientist Should Know About Floating-Point Arithmetic", §Binary - Decimal
    // Conversion).
    let digits = usize::try_from((prec * 30_103).div_ceil(100_000) + 2).unwrap_or(17);
    alloc::format!("{value:.digits$e}")
}

#[cfg(feature = "serde")]
pub fn from_str(value: &str) -> Option<BackingFloat> {
    let parsed = Float::parse(value).ok()?;
    Some(Float::with_val(get_precision(), parsed))
}
