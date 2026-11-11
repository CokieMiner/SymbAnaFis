//! Mutual exclusion, unwind safety, and concurrent update publication.

use std::{sync::Barrier, thread};

use super::super::Mutex;

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
