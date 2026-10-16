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
