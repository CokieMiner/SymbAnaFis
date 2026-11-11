//! Mutual exclusion, contention, unwinding recovery, and cache-line padding tests.

use core::{
    mem::{align_of, size_of},
    panic::AssertUnwindSafe,
};
use std::{panic::catch_unwind, sync::Barrier, thread};

use super::{CachePadded, Mutex};

const EXPECTED_CACHE_LINE: usize = if cfg!(target_arch = "s390x") {
    256
} else if cfg!(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm64ec",
    target_arch = "powerpc64",
)) {
    128
} else if cfg!(any(
    target_arch = "arm",
    target_arch = "mips",
    target_arch = "mips32r6",
    target_arch = "mips64",
    target_arch = "mips64r6",
    target_arch = "sparc",
    target_arch = "hexagon",
)) {
    32
} else if cfg!(target_arch = "m68k") {
    16
} else {
    64
};

#[test]
fn concurrent_guards_publish_every_update() {
    let lock = Mutex::new(0_usize);
    let barrier = Barrier::new(8);
    thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                barrier.wait();
                for _ in 0..512 {
                    let mut value = lock.lock();
                    *value = value.checked_add(1).expect("bounded test updates");
                }
            });
        }
    });
    assert_eq!(*lock.lock(), 4096);
}

#[test]
fn try_lock_reflects_ownership_and_releases_on_drop() {
    let lock = Mutex::new(10_usize);
    let acquired = Barrier::new(2);
    let release = Barrier::new(2);
    thread::scope(|scope| {
        scope.spawn(|| {
            let mut guard = lock.lock();
            *guard = 25;
            acquired.wait();
            release.wait();
            assert_eq!(*guard, 25);
        });
        acquired.wait();
        assert!(lock.try_lock().is_none());
        release.wait();
    });
    let mut guard = lock
        .try_lock()
        .expect("lock is released after thread exits");
    assert_eq!(*guard, 25);
    *guard = 42;
    drop(guard);
    assert_eq!(*lock.lock(), 42);
}

#[test]
fn guard_releases_and_remains_accessible_after_unwind() {
    let lock = Mutex::new(1_usize);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let mut guard = lock.lock();
        *guard = 99;
        assert_eq!(*guard, 0, "intentional unwind while holding guard");
    }));
    assert!(panicked.is_err());
    {
        let mut guard = lock
            .try_lock()
            .expect("guard released during unwind and poisoned std locks recover");
        assert_eq!(*guard, 99);
        *guard = 100;
    }
    assert_eq!(*lock.lock(), 100);
}

#[test]
fn cache_padded_aligns_to_cache_line_and_forwards_access() {
    assert_eq!(align_of::<CachePadded<u8>>(), EXPECTED_CACHE_LINE);
    assert_eq!(size_of::<CachePadded<u8>>(), EXPECTED_CACHE_LINE);
    assert_eq!(
        size_of::<CachePadded<Mutex<usize>>>() & EXPECTED_CACHE_LINE.saturating_sub(1),
        0
    );
    assert!(size_of::<[CachePadded<Mutex<usize>>; 2]>() >= EXPECTED_CACHE_LINE.saturating_mul(2));

    let mut padded = CachePadded::new(7_u64);
    assert_eq!(*padded, 7);
    *padded = 19;
    assert_eq!(*padded, 19);

    let defaulted = CachePadded::<u64>::default();
    assert_eq!(*defaulted, 0);
}
