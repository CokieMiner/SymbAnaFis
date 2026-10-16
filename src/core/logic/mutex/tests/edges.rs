use core::panic::AssertUnwindSafe;
use std::{panic::catch_unwind, sync::Barrier, thread};

use super::super::Mutex;

#[test]
#[expect(
    clippy::panic,
    reason = "the test unwinds while owning the guard to verify unconditional unlock"
)]
fn unwinding_releases_mutated_state_and_preserves_access() {
    let lock = Mutex::new(0_usize);
    let failure = catch_unwind(AssertUnwindSafe(|| {
        *lock.lock() = 7;
        let _guard = lock.lock();
        panic!("unwind under the acquired guard");
    }));
    assert!(failure.is_err(), "the guarded scope unwound");
    assert_eq!(*lock.lock(), 7);
    assert!(
        lock.try_lock().is_some(),
        "unwinding preserves nonblocking access"
    );
}

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
