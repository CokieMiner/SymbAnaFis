//! Public API for numeric scalar types.

pub use super::logic::clifford_number::{CliffordNumber, Signature};
pub use super::logic::interval::Interval;
pub use super::logic::scalar::Number;
pub use super::logic::vector::Vector;

/// Convert any value that implements `Into<Number>` into [`Number`].
///
/// This is a convenience helper for concise numeric expressions.
///
/// # Examples
///
/// ```
/// use num_anafis::{Number, i, s};
///
/// let z = s(3) + s(5) * i();
/// assert_eq!(z, s(8));
/// ```
#[must_use]
pub fn s<T>(value: T) -> Number
where
    T: Into<Number>,
{
    value.into()
}

/// Integer argument accepted by [`r`].
pub trait RationalInt {
    /// Convert this integer value into [`Number`] while preserving exactness when possible.
    fn into_number(self) -> Number;
}

macro_rules! impl_rational_int {
    ($($t:ty),+ $(,)?) => {
        $(
            impl RationalInt for $t {
                fn into_number(self) -> Number {
                    Number::from(self)
                }
            }
        )+
    };
}

impl_rational_int!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, usize);

impl RationalInt for isize {
    fn into_number(self) -> Number {
        i64::try_from(self).map_or_else(
            |_| {
                if self.is_negative() {
                    Number::float(f64::NEG_INFINITY)
                } else {
                    Number::float(f64::INFINITY)
                }
            },
            Number::from,
        )
    }
}

/// Construct a rational [`Number`] from integer numerator and denominator.
///
/// This is a convenience helper for concise algebraic expressions.
///
/// # Examples
///
/// ```
/// use num_anafis::{r, s};
///
/// let x = r(1, 2) + s(3);
/// assert_eq!(x.to_f64_lossy(), 3.5);
/// ```
#[must_use]
pub fn r<N, D>(num: N, den: D) -> Number
where
    N: RationalInt,
    D: RationalInt,
{
    num.into_number() / den.into_number()
}

/// Convenience constructor for the complex unit `i`.
#[must_use]
pub fn i() -> CliffordNumber {
    CliffordNumber::i()
}

/// Convenience constructor for the split-complex unit `j`.
#[must_use]
pub fn j() -> CliffordNumber {
    CliffordNumber::j()
}

/// Convenience constructor for the dual unit `eps`.
#[must_use]
pub fn eps() -> CliffordNumber {
    CliffordNumber::eps()
}

/// Convenience constructor for the Euclidean basis vector `e1` in R3.
#[must_use]
pub fn e1() -> CliffordNumber {
    CliffordNumber::e1()
}

/// Convenience constructor for the Euclidean basis vector `e2` in R3.
#[must_use]
pub fn e2() -> CliffordNumber {
    CliffordNumber::e2()
}

/// Convenience constructor for the Euclidean basis vector `e3` in R3.
#[must_use]
pub fn e3() -> CliffordNumber {
    CliffordNumber::e3()
}
