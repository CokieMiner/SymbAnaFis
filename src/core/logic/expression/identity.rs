//! Canonical pointer identity and cheap shared ownership.

use core::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
    ptr::eq,
};

use num_anafis::Number;

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
