//! Nonblocking probes for the blocking backend's guard contracts.

use std::sync::TryLockError;

use super::super::{Mutex, MutexGuard};

impl<Value> Mutex<Value> {
    pub fn try_lock(&self) -> Option<MutexGuard<'_, Value>> {
        let inner = match self.inner.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(poison)) => poison.into_inner(),
            Err(TryLockError::WouldBlock) => return None,
        };
        Some(MutexGuard { inner })
    }
}
