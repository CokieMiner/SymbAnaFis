use core::sync::atomic::AtomicUsize;
use std::{sync::Barrier, thread};

use alloc::vec::Vec;

use super::super::IdSequence;

#[test]
fn exhaustion_stays_exhausted_without_reusing_any_identity() {
    let penultimate = usize::MAX
        .checked_sub(1)
        .expect("native word has a predecessor");
    let sequence = IdSequence(AtomicUsize::new(penultimate));
    assert_eq!(sequence.next(), Some(penultimate));
    for _ in 0..32 {
        assert_eq!(sequence.next(), None);
    }
}

#[test]
fn concurrent_allocation_has_no_duplicates_or_gaps() {
    let sequence = IdSequence::new();
    let barrier = Barrier::new(8);
    let mut identities = thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    (0..512)
                        .map(|_| sequence.next().expect("bounded identity count"))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("worker completed"))
            .collect::<Vec<_>>()
    });
    identities.sort_unstable();
    assert_eq!(identities, (1..=4096).collect::<Vec<_>>());
}
