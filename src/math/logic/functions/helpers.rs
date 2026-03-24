use crate::core::traits::MathScalar;

#[inline]
pub(super) fn sign_from_delta<T: MathScalar>(delta: T) -> T {
    if delta.is_nan() {
        return T::nan();
    }
    if delta.is_sign_negative() {
        -T::one()
    } else {
        T::one()
    }
}

#[inline]
pub(super) fn signed_infinity<T: MathScalar>(sign: T) -> T {
    if sign.is_nan() {
        return T::nan();
    }
    if sign.is_sign_negative() {
        -T::infinity()
    } else {
        T::infinity()
    }
}

#[inline]
pub(super) fn signed_infinity_from_delta<T: MathScalar>(delta: T) -> T {
    signed_infinity(sign_from_delta(delta))
}

#[inline]
pub(super) fn gamma_pole_sign<T: MathScalar>(x: T) -> T {
    let n = (-x).to_i64().unwrap_or(0);
    if (n & 1) == 0 { T::one() } else { -T::one() }
}
