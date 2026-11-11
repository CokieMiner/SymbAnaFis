//! Canonical pointer identity, literal conversions, and the optional
//! `num-traits` identities.

use core::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
    ptr::eq,
};

use num_anafis::Number;
#[cfg(feature = "num-traits")]
use num_traits::{One, Zero};

use super::Expr;

impl PartialEq for Expr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        eq(self.pointer(), other.pointer())
    }
}

impl Eq for Expr {}

impl Hash for Expr {
    #[inline]
    fn hash<HasherType: Hasher>(&self, state: &mut HasherType) {
        self.pointer().hash(state);
    }
}

impl Debug for Expr {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(self, formatter)
    }
}

impl From<&Self> for Expr {
    fn from(expression: &Self) -> Self {
        expression.clone()
    }
}

impl From<f64> for Expr {
    fn from(value: f64) -> Self {
        Self::number(value)
    }
}

impl From<i32> for Expr {
    fn from(value: i32) -> Self {
        Self::number(value)
    }
}

impl From<i64> for Expr {
    fn from(value: i64) -> Self {
        Self::number(value)
    }
}

impl From<Number> for Expr {
    fn from(value: Number) -> Self {
        Self::number(value)
    }
}

impl From<&Number> for Expr {
    fn from(value: &Number) -> Self {
        Self::number(value.clone())
    }
}

#[cfg(feature = "num-traits")]
impl Zero for Expr {
    fn zero() -> Self {
        Self::number(0)
    }

    fn is_zero(&self) -> bool {
        self.is_zero_num()
    }
}

#[cfg(feature = "num-traits")]
impl One for Expr {
    fn one() -> Self {
        Self::number(1)
    }

    fn is_one(&self) -> bool {
        self.is_one_num()
    }
}
