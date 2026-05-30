#![allow(
    clippy::trivially_copy_pass_by_ref,
    clippy::unnecessary_wraps,
    reason = "&T API for non-Copy backend uniformity; Option<IntType> for uniform API"
)]

#[path = "shared_ops.rs"]
#[macro_use]
mod shared_ops;

use alloc::string::String;
use core::cmp::Ordering;
pub(super) type BackingInt = i64;

#[inline]
pub(super) const fn from_i64(value: i64) -> Option<BackingInt> {
    Some(value)
}

impl_shared_int_ops!();
