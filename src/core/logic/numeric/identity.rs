//! Exact-tier and bitwise float identity, separate from numeric partial equality.

use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use num_anafis::Real;

use super::NumericValue;

impl PartialEq for NumericValue {
    fn eq(&self, other: &Self) -> bool {
        self.compare(other).is_eq()
    }
}

impl Eq for NumericValue {}

impl Hash for NumericValue {
    fn hash<HasherType: Hasher>(&self, state: &mut HasherType) {
        if let Some(parts) = self.parts() {
            Self::hash_real(&parts.re, state);
            Self::hash_real(&parts.im, state);
        } else {
            self.0.hash(state);
        }
    }
}

impl NumericValue {
    /// Canonical representation order; floating signed zeros remain distinct.
    pub fn compare(&self, other: &Self) -> Ordering {
        match (self.parts(), other.parts()) {
            (Some(left), Some(right)) => Self::compare_real(&left.re, &right.re)
                .then_with(|| Self::compare_real(&left.im, &right.im)),
            _ => self.0.total_cmp(&other.0),
        }
    }

    fn compare_real(left: &Real, right: &Real) -> Ordering {
        Self::rank(left).cmp(&Self::rank(right)).then_with(|| {
            if let (Real::Float(a), Real::Float(b)) = (left, right) {
                a.total_cmp(b)
            } else {
                left.total_cmp(right)
            }
        })
    }

    fn hash_real<HasherType: Hasher>(value: &Real, state: &mut HasherType) {
        Self::rank(value).hash(state);
        if let Real::Float(float) = value {
            float.to_bits().hash(state);
        } else {
            value.hash(state);
        }
    }

    const fn rank(value: &Real) -> u8 {
        match value {
            Real::Int(_) => 0,
            Real::Rational(_) => 1,
            Real::Float(_) => 2,
            _ => 3,
        }
    }
}
