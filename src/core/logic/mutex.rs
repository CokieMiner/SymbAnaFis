//! Exclusive access with a blocking std backend and an atomic `no_std` backend.
//!
//! Guards release during unwinding, and poisoned std guards remain accessible.
//! The `no_std` backend requires the current owner to make progress. Reentry,
//! including from an interrupt that suspends an owner, is unsupported.

#![cfg_attr(
    test,
    expect(
        clippy::self_named_module_files,
        reason = "descendant tests verify the private synchronization state beside its owner"
    )
)]

#[cfg(not(target_has_atomic = "ptr"))]
compile_error!("SymbAnaFis requires pointer-width atomics for Arc and global DAG interning");

use core::ops::{Deref, DerefMut};
#[cfg(not(feature = "std"))]
use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    marker::PhantomData,
    sync::atomic::{AtomicBool, Ordering},
};
#[cfg(feature = "std")]
use std::sync::{Mutex as BlockingMutex, MutexGuard as BlockingGuard, PoisonError};

/// A lock over one privately owned value, recovering access after unwinding.
#[derive(Debug)]
pub struct Mutex<Value> {
    #[cfg(feature = "std")]
    inner: BlockingMutex<Value>,
    #[cfg(not(feature = "std"))]
    locked: AtomicBool,
    #[cfg(not(feature = "std"))]
    value: UnsafeCell<Value>,
}

/// Exclusive access that releases the selected backend when dropped.
#[derive(Debug)]
pub struct MutexGuard<'lock, Value> {
    #[cfg(feature = "std")]
    inner: BlockingGuard<'lock, Value>,
    #[cfg(not(feature = "std"))]
    lock: &'lock Mutex<Value>,
    #[cfg(not(feature = "std"))]
    borrow: PhantomData<&'lock mut Value>,
}

#[cfg(not(feature = "std"))]
#[expect(
    unsafe_code,
    reason = "the acquisition flag serializes access to the private UnsafeCell across threads"
)]
// SAFETY: Only a successful Acquire creates a guard, and Release ends its
// exclusive access. Transferring that access requires Value: Send. Guards carry
// PhantomData<&mut Value>, so sharing a guard additionally requires Value: Sync.
unsafe impl<Value: Send> Sync for Mutex<Value> {}

impl<Value> Mutex<Value> {
    /// Create an unlocked value without allocating or runtime initialization.
    pub const fn new(value: Value) -> Self {
        Self {
            #[cfg(feature = "std")]
            inner: BlockingMutex::new(value),
            #[cfg(not(feature = "std"))]
            locked: AtomicBool::new(false),
            #[cfg(not(feature = "std"))]
            value: UnsafeCell::new(value),
        }
    }

    /// Acquire exclusive access, blocking with std and polling with `no_std`.
    pub fn lock(&self) -> MutexGuard<'_, Value> {
        #[cfg(feature = "std")]
        {
            MutexGuard {
                inner: self.inner.lock().unwrap_or_else(PoisonError::into_inner),
            }
        }
        #[cfg(not(feature = "std"))]
        {
            loop {
                if let Some(guard) = self.try_lock() {
                    return guard;
                }
                while self.locked.load(Ordering::Relaxed) {
                    spin_loop();
                }
            }
        }
    }

    /// Attempt acquisition once without waiting for another owner.
    #[cfg(not(feature = "std"))]
    pub fn try_lock(&self) -> Option<MutexGuard<'_, Value>> {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
            .then(|| MutexGuard {
                lock: self,
                borrow: PhantomData,
            })
    }
}

impl<Value> Deref for MutexGuard<'_, Value> {
    type Target = Value;

    #[cfg_attr(
        not(feature = "std"),
        expect(
            unsafe_code,
            reason = "an acquired guard borrows the initialized cell until its Release unlock"
        )
    )]
    fn deref(&self) -> &Value {
        #[cfg(feature = "std")]
        {
            &self.inner
        }
        #[cfg(not(feature = "std"))]
        {
            // SAFETY: This guard owns the sole acquisition. The initialized value
            // remains owned by the lock, which is borrowed for the guard's lifetime.
            unsafe { &*self.lock.value.get() }
        }
    }
}

impl<Value> DerefMut for MutexGuard<'_, Value> {
    #[cfg_attr(
        not(feature = "std"),
        expect(
            unsafe_code,
            reason = "an acquired guard exclusively borrows the cell until its Release unlock"
        )
    )]
    fn deref_mut(&mut self) -> &mut Value {
        #[cfg(feature = "std")]
        {
            &mut self.inner
        }
        #[cfg(not(feature = "std"))]
        {
            // SAFETY: This guard owns the sole acquisition. Its mutable borrow
            // excludes other references through the guard until this borrow ends.
            unsafe { &mut *self.lock.value.get() }
        }
    }
}

#[cfg(not(feature = "std"))]
impl<Value> Drop for MutexGuard<'_, Value> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests;
