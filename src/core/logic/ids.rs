//! Never-reused identities for registry-owned symbols and contexts.

#![cfg_attr(
    test,
    expect(
        clippy::self_named_module_files,
        reason = "descendant tests exercise exhaustion at the private counter boundary"
    )
)]

use core::sync::atomic::{AtomicUsize, Ordering};

/// A checked sequence; exhaustion leaves its last value unchanged.
#[derive(Debug)]
pub struct IdSequence(AtomicUsize);

impl IdSequence {
    /// Start at one, reserving zero as an unallocated identity.
    pub const fn new() -> Self {
        Self(AtomicUsize::new(1))
    }

    /// Allocate an identity without wrapping or reusing previous values.
    pub fn next(&self) -> Option<usize> {
        let mut current = self.0.load(Ordering::Relaxed);
        loop {
            let successor = current.checked_add(1)?;
            match self.0.compare_exchange_weak(
                current,
                successor,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(allocated) => return Some(allocated),
                Err(actual) => current = actual,
            }
        }
    }
}

#[cfg(test)]
mod tests;
