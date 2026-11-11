//! Exclusive access with a blocking std backend, an atomic `no_std` backend,
//! and cache-line padding for independently contended locks.
//!
//! Guards release during unwinding, and poisoned std guards remain accessible.
//! The `no_std` backend requires the current owner to make progress. Reentry,
//! including from an interrupt that suspends an owner, is unsupported.

#![cfg_attr(
    not(feature = "std"),
    expect(
        unsafe_code,
        reason = "the acquisition flag serializes access to the private UnsafeCell across threads"
    )
)]

#[cfg(not(target_has_atomic = "ptr"))]
compile_error!("SymbAnaFis requires pointer-width atomics for Arc and global DAG interning");

#[cfg(not(feature = "std"))]
use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};
use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::{Deref, DerefMut},
};
#[cfg(all(test, feature = "std"))]
use std::sync::TryLockError;
#[cfg(feature = "std")]
use std::sync::{Mutex as BlockingMutex, MutexGuard as BlockingGuard, PoisonError};

/// A cache-line padded wrapper that isolates independently contended values
/// from false sharing.
///
/// Alignment tracks the worst-case cache-line or spatial-prefetcher granularity
/// of each target architecture (following `crossbeam-utils`): 256 bytes on
/// `s390x`, 128 bytes on `x86_64`/`aarch64`/`arm64ec`/`powerpc64`, 32 bytes on
/// `arm`/`mips`/`mips32r6`/`mips64`/`mips64r6`/`sparc`/`hexagon`, 16 bytes on
/// `m68k`, and 64 bytes on all other architectures.
#[repr(C)]
#[cfg_attr(target_arch = "s390x", repr(align(256)))]
#[cfg_attr(
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "arm64ec",
        target_arch = "powerpc64",
    ),
    repr(align(128))
)]
#[cfg_attr(
    any(
        target_arch = "arm",
        target_arch = "mips",
        target_arch = "mips32r6",
        target_arch = "mips64",
        target_arch = "mips64r6",
        target_arch = "sparc",
        target_arch = "hexagon",
    ),
    repr(align(32))
)]
#[cfg_attr(target_arch = "m68k", repr(align(16)))]
#[cfg_attr(
    not(any(
        target_arch = "s390x",
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "arm64ec",
        target_arch = "powerpc64",
        target_arch = "arm",
        target_arch = "mips",
        target_arch = "mips32r6",
        target_arch = "mips64",
        target_arch = "mips64r6",
        target_arch = "sparc",
        target_arch = "hexagon",
        target_arch = "m68k",
    )),
    repr(align(64))
)]
#[derive(Debug, Default)]
pub struct CachePadded<Value>(Value);

/// A lock over one privately owned value, recovering access after unwinding.
#[derive(Debug)]
pub struct Mutex<Value> {
    #[cfg(feature = "std")]
    inner: BlockingMutex<Value>,
    #[cfg(not(feature = "std"))]
    locked: AtomicUsize,
    #[cfg(not(feature = "std"))]
    value: UnsafeCell<Value>,
}

/// Exclusive access that releases the selected backend when dropped.
pub struct MutexGuard<'lock, Value> {
    #[cfg(feature = "std")]
    inner: BlockingGuard<'lock, Value>,
    #[cfg(not(feature = "std"))]
    lock: &'lock Mutex<Value>,
    #[cfg(not(feature = "std"))]
    borrow: PhantomData<&'lock mut Value>,
    #[cfg(not(feature = "std"))]
    not_send: PhantomData<*const ()>,
}

#[cfg(not(feature = "std"))]
// SAFETY: Only a successful Acquire creates a guard, and Release ends its
// exclusive access. Transferring that access requires Value: Send.
unsafe impl<Value: Send> Sync for Mutex<Value> {}

#[cfg(not(feature = "std"))]
// SAFETY: A guard owns exclusive access to `Value` for `'lock` while
// `not_send` keeps ownership and `Drop` on the acquiring thread. Sharing
// `&MutexGuard<'_, Value>` across threads only exposes `&Value` via `Deref`,
// which is sound whenever `Value: Sync`.
unsafe impl<Value: Sync> Sync for MutexGuard<'_, Value> {}

impl<Value> CachePadded<Value> {
    /// Wrap `value` aligned to the target cache-line size.
    #[must_use]
    pub const fn new(value: Value) -> Self {
        Self(value)
    }
}

impl<Value> Deref for CachePadded<Value> {
    type Target = Value;

    #[inline]
    fn deref(&self) -> &Value {
        &self.0
    }
}

impl<Value> DerefMut for CachePadded<Value> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Value {
        &mut self.0
    }
}

impl<Value> Mutex<Value> {
    /// Create an unlocked value without allocating or runtime initialization.
    pub const fn new(value: Value) -> Self {
        Self {
            #[cfg(feature = "std")]
            inner: BlockingMutex::new(value),
            #[cfg(not(feature = "std"))]
            locked: AtomicUsize::new(0),
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
                if self.locked.swap(1, Ordering::Acquire) == 0 {
                    return MutexGuard {
                        lock: self,
                        borrow: PhantomData,
                        not_send: PhantomData,
                    };
                }
                while self.locked.load(Ordering::Relaxed) != 0 {
                    spin_loop();
                }
            }
        }
    }

    /// Attempt acquisition once without waiting for another owner.
    #[cfg(test)]
    pub fn try_lock(&self) -> Option<MutexGuard<'_, Value>> {
        #[cfg(feature = "std")]
        {
            match self.inner.try_lock() {
                Ok(inner) => Some(MutexGuard { inner }),
                Err(TryLockError::Poisoned(error)) => Some(MutexGuard {
                    inner: error.into_inner(),
                }),
                Err(TryLockError::WouldBlock) => None,
            }
        }
        #[cfg(not(feature = "std"))]
        {
            self.locked
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
                .then(|| MutexGuard {
                    lock: self,
                    borrow: PhantomData,
                    not_send: PhantomData,
                })
        }
    }
}

impl<Value: Debug> Debug for MutexGuard<'_, Value> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&**self, formatter)
    }
}

impl<Value> Deref for MutexGuard<'_, Value> {
    type Target = Value;

    #[inline]
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
    #[inline]
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
    #[inline]
    fn drop(&mut self) {
        self.lock.locked.store(0, Ordering::Release);
    }
}
