#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "&T API required for non-Copy backend uniformity"
)]

#[path = "shared_ops.rs"]
#[macro_use]
mod shared_ops;

use alloc::string::String;
use core::cmp::Ordering;

pub(super) type BackingInt = i32;

#[inline]
pub(super) fn from_i64(value: i64) -> Option<BackingInt> {
    i32::try_from(value).ok()
}

impl_shared_int_ops!();
