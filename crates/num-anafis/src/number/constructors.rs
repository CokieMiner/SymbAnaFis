//! Convenience constructors for `Scalar`.
//!
//! ```
//! use num_anafis::{s, r};
//!
//! let x = s(42);        // Integer
//! let y = s(3.14);      // Float (normalized to Int if exact)
//! let z = r(1, 3);      // Rational 1/3
//! let w = s(-7_i32);    // Works with any integer type
//! ```

use super::logic::float_ops::{from_f32, from_f64, from_i64 as from_i64_to_float, nan};
use super::logic::int_math::{IntType, clone, from_i64 as from_i64_to_int, is_zero};
use super::logic::rational_math::new;
use super::logic::scalar::{Scalar, ScalarRepr};

// ============================================================================
// IntoScalar — trait for anything that can become a Scalar
// ============================================================================

/// Trait for types that can be converted into a [`Scalar`] with maximum
/// precision. Integers stay as integers, floats normalize to integer when
/// exact, and everything else becomes a float.
pub trait IntoScalar {
    /// Convert into a `Scalar`, choosing the most exact representation.
    fn into_scalar(self) -> Scalar;
}

// --- Small integers → Int (exact) ---

macro_rules! impl_intoscalar_exact {
    ($($ty:ty),*) => {
        $(
            impl IntoScalar for $ty {
                #[inline]
                fn into_scalar(self) -> Scalar {
                    Scalar::from_int(
                        from_i64_to_int(i64::from(self))
                            .expect(concat!(stringify!($ty), " always fits in i64 and IntType")),
                    )
                }
            }
        )*
    };
}
impl_intoscalar_exact!(i8, i16, i32, u8, u16, u32);

// --- Large integers → Int with fallback to Float ---

impl IntoScalar for i64 {
    #[inline]
    fn into_scalar(self) -> Scalar {
        from_i64_to_int(self).map_or_else(
            || Scalar::from_float(from_i64_to_float(self)),
            Scalar::from_int,
        )
    }
}

impl IntoScalar for u64 {
    #[inline]
    fn into_scalar(self) -> Scalar {
        if let Ok(v) = i64::try_from(self)
            && let Some(ir) = from_i64_to_int(v)
        {
            return Scalar::from_int(ir);
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "Fallback: lossy u64→f64 when out of int range"
        )]
        Scalar::from_float(from_f64(self as f64))
    }
}

impl IntoScalar for usize {
    #[inline]
    fn into_scalar(self) -> Scalar {
        if let Ok(v) = i64::try_from(self)
            && let Some(ir) = from_i64_to_int(v)
        {
            return Scalar::from_int(ir);
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "Fallback: lossy usize→f64 when out of int range"
        )]
        Scalar::from_float(from_f64(self as f64))
    }
}

// --- Floats → Float (normalized to Int if exact) ---

impl IntoScalar for f64 {
    #[inline]
    fn into_scalar(self) -> Scalar {
        Scalar::from_float(from_f64(self))
    }
}

impl IntoScalar for f32 {
    #[inline]
    fn into_scalar(self) -> Scalar {
        Scalar::from_float(from_f32(self))
    }
}

// --- Scalar passthrough ---

impl IntoScalar for Scalar {
    #[inline]
    fn into_scalar(self) -> Scalar {
        self
    }
}

impl IntoScalar for &Scalar {
    #[inline]
    fn into_scalar(self) -> Scalar {
        self.clone()
    }
}

// ============================================================================
// Also implement From<T> for Scalar so `.into()` works
// ============================================================================

macro_rules! impl_from_for_scalar {
    ($($ty:ty),*) => {
        $(
            impl From<$ty> for Scalar {
                #[inline]
                fn from(v: $ty) -> Self {
                    IntoScalar::into_scalar(v)
                }
            }
        )*
    };
}

impl_from_for_scalar!(i8, i16, i32, i64, u8, u16, u32, u64, usize, f32, f64);

// ============================================================================
// Convenience free functions
// ============================================================================

/// Create a [`Scalar`] from any numeric value.
///
/// Chooses the most exact internal representation:
/// - Integers stay as `Int`
/// - Floats that are exact integers normalize to `Int`
/// - Everything else becomes `Float`
///
/// # Examples
/// ```
/// use num_anafis::{s, Number};
///
/// let a = s(42);       // Int(42)
/// let b = s(3.14);     // Float(3.14)
/// let c = s(-1_i8);    // Int(-1)
/// let d = s(2.0_f64);  // Int(2) — normalized
/// ```
#[inline]
pub fn s(v: impl IntoScalar) -> Scalar {
    v.into_scalar()
}

/// Create a [`Scalar`] rational from numerator and denominator.
///
/// If the division is exact (e.g. `r(6, 3)`), the result normalizes
/// to an integer. Division by zero produces `NaN`.
///
/// # Examples
/// ```
/// use num_anafis::{r, Number};
///
/// let half = r(1, 2);      // Rational(1/2)
/// let two  = r(6, 3);      // Int(2) — normalized
/// let neg  = r(-3, 4);     // Rational(-3/4)
/// ```
#[inline]
pub fn r(num: impl IntoScalar, den: impl IntoScalar) -> Scalar {
    let n = num.into_scalar();
    let d = den.into_scalar();

    // Try to extract integer representations for exact rational
    if let (Some(ni), Some(di)) = (to_int_repr(&n), to_int_repr(&d)) {
        if is_zero(&di) {
            return Scalar::from_float(nan());
        }
        return Scalar::from_rational(new(ni, di));
    }

    // Fall back to float division
    &n / &d
}

/// Try to extract an `IntType` from a Scalar (only if it's already Int).
fn to_int_repr(s: &Scalar) -> Option<IntType> {
    match s.0 {
        ScalarRepr::Int(ref i) => Some(clone(i)),
        ScalarRepr::Rational(_) | ScalarRepr::Float(_) => None,
    }
}
