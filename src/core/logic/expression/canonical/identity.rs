//! Canonical pointer identity, literal conversions, and the optional
//! `num-traits` identities.

#![expect(
    unsafe_code,
    reason = "Expr is repr(transparent) over a pointer-sized Arc handle with no padding, so slices hash as contiguous initialized bytes"
)]

use core::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    hash::{Hash, Hasher},
    mem::{size_of, size_of_val},
    slice::from_raw_parts,
};

use alloc::sync::Arc;

use num_anafis::Number;
#[cfg(feature = "num-traits")]
use num_traits::{One, Zero};

use super::{Expr, Node};

impl PartialEq for Expr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Expr {}

impl Hash for Expr {
    #[inline]
    fn hash<HasherType: Hasher>(&self, state: &mut HasherType) {
        self.pointer().hash(state);
    }

    #[inline]
    fn hash_slice<HasherType: Hasher>(data: &[Self], state: &mut HasherType) {
        const {
            assert!(
                size_of::<Self>() == size_of::<*const ()>(),
                "Expr has pointer size and no padding"
            );
        }
        // SAFETY: `Expr` is `#[repr(transparent)]` over `ManuallyDrop<Arc<Stored>>`,
        // which has the size and layout of a single non-null pointer with no
        // padding, so every byte of `data` is initialized.
        let bytes = unsafe { from_raw_parts(data.as_ptr().cast::<u8>(), size_of_val(data)) };
        state.write(bytes);
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
    #[inline]
    fn from(value: i32) -> Self {
        Node::small_int(i64::from(value)).unwrap_or_else(|| Self::number(value))
    }
}

impl From<i64> for Expr {
    #[inline]
    fn from(value: i64) -> Self {
        Node::small_int(value).unwrap_or_else(|| Self::number(value))
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
        Self::from(0_i64)
    }

    fn is_zero(&self) -> bool {
        self.is_zero_num()
    }
}

#[cfg(feature = "num-traits")]
impl One for Expr {
    fn one() -> Self {
        Self::from(1_i64)
    }

    fn is_one(&self) -> bool {
        self.is_one_num()
    }
}
