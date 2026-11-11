//! The explicit selection bit set: membership, count, and growth.

use alloc::collections::BTreeSet;

use proptest::prelude::*;

use super::super::Taken;

proptest! {
    #[test]
    fn membership_and_count_agree_with_a_sorted_set(
        positions in prop::collection::vec(0_usize..4096, 0..256),
    ) {
        let mut taken = Taken::default();
        let mut expected = BTreeSet::new();
        for position in positions {
            taken.insert(position);
            expected.insert(position);
            prop_assert_eq!(taken.len(), expected.len());
        }
        for position in 0..4096 {
            prop_assert_eq!(taken.contains(position), expected.contains(&position));
        }
    }

    #[test]
    fn first_free_agrees_with_a_linear_scan(
        positions in prop::collection::vec(0_usize..4096, 0..256),
        from in 0_usize..4096,
    ) {
        let mut taken = Taken::default();
        for position in &positions {
            taken.insert(*position);
        }
        let expected = (from..4096)
            .find(|position| !taken.contains(*position))
            .unwrap_or(4096);
        prop_assert_eq!(taken.first_free(from).min(4096), expected);
    }
}

#[test]
fn positions_across_word_boundaries_are_distinct() {
    let mut taken = Taken::default();
    for position in [0, 63, 64, 127, 128] {
        taken.insert(position);
    }
    assert_eq!(taken.len(), 5);
    for position in [0, 63, 64, 127, 128] {
        assert!(taken.contains(position));
    }
    for position in [1, 62, 65, 126, 129] {
        assert!(!taken.contains(position));
    }
}

#[test]
fn a_repeated_position_counts_once() {
    let mut taken = Taken::default();
    taken.insert(7);
    taken.insert(7);
    assert_eq!(taken.len(), 1);
}

#[test]
fn extend_takes_every_distinct_position() {
    let mut taken = Taken::default();
    taken.extend([3, 70, 3]);
    assert_eq!(taken.len(), 2);
    assert!(taken.contains(3));
    assert!(taken.contains(70));
}
