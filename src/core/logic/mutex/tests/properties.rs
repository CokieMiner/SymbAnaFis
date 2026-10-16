use alloc::{string::String, vec::Vec};

use proptest::prelude::{prop, prop_assert, prop_assert_eq, proptest};

use super::super::Mutex;

proptest! {
    #[test]
    fn failed_acquisitions_preserve_exclusion_and_release_keeps_mutations(
        parts in prop::collection::vec("[a-z]{0,12}", 0..32),
        attempts in 0_usize..64,
    ) {
        let lock = Mutex::new(Vec::<String>::new());
        let mut guard = lock.lock();
        for _ in 0..attempts {
            prop_assert!(lock.try_lock().is_none());
        }
        guard.extend(parts.iter().cloned());
        drop(guard);
        let acquired = lock.try_lock().expect("released lock is available");
        prop_assert_eq!(&*acquired, &parts);
        prop_assert!(lock.try_lock().is_none());
        drop(acquired);
        prop_assert!(lock.try_lock().is_some());
    }
}
