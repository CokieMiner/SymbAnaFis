//! Identity allocation order, exhaustion, and concurrent access.

use std::{sync::Barrier, thread};

use alloc::vec::Vec;

use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

use super::super::IdSequence;

proptest! {
    #[test]
    fn emitted_identities_are_positive_and_strictly_increasing(count in 1_usize..256) {
        let sequence = IdSequence::new();
        let identities: Vec<_> = (0..count)
            .map(|_| sequence.next().expect("bounded identity count"))
            .collect();
        prop_assert_eq!(identities.len(), count);
        prop_assert!(identities.iter().all(|id| *id != 0));
        prop_assert!(identities.windows(2).all(|pair| pair.first() < pair.last()));
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
