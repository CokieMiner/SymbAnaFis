use alloc::{sync::Arc, vec::Vec};

use proptest::prelude::{any, prop_assert, prop_assert_eq, proptest};

use super::{super::Interner, support::Collision};

proptest! {
    #[test]
    fn equal_and_colliding_values_keep_distinct_canonical_arcs(
        first in any::<u8>(),
        offset in 1_u8..=u8::MAX,
    ) {
        let index = Interner::new();
        let second = first.wrapping_add(offset);
        let first_arc = index.intern(Collision(first));
        let same_arc = index.intern(Collision(first));
        let second_arc = index.intern(Collision(second));

        prop_assert!(Arc::ptr_eq(&first_arc, &same_arc));
        prop_assert!(!Arc::ptr_eq(&first_arc, &second_arc));
        prop_assert_eq!(index.entry_count(), 2);
    }

    #[test]
    fn collision_bucket_reclaims_expired_entries(count in 9_u8..=96) {
        let index = Interner::new();
        let mut values: Vec<_> = (0..count)
            .map(|value| index.intern(Collision(value)))
            .collect();
        let live = values.pop().expect("the bounded burst has one live value");
        let live_value = Collision(count.checked_sub(1).expect("count is nonzero"));

        drop(values);
        index.prune(Interner::<Collision>::fingerprint(&Collision(0)));

        prop_assert_eq!(index.entry_count(), 1);
        prop_assert!(index.bucket_capacity(&live_value) <= 8);
        let shared = index.intern(live_value);
        prop_assert!(Arc::ptr_eq(&live, &shared));
        drop(shared);
        drop(live);
        index.prune(Interner::<Collision>::fingerprint(&live_value));
        prop_assert_eq!(index.entry_count(), 0);
        prop_assert_eq!(index.bucket_capacity(&live_value), 0);
        prop_assert_eq!(index.index_capacity(), 0);
    }
}
