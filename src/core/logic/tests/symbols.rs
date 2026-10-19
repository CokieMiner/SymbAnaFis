//! Registry sharing and identity preservation under name rebinding.

use alloc::sync::Arc;

use proptest::prelude::{ProptestConfig, prop_assert, prop_assert_eq, prop_assert_ne, proptest};

use super::super::{SymbolData, SymbolError, symbols::SymbolRegistry};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn registry_rebindings_keep_distinct_permanent_identities(
        name in "[a-z]{1,12}", rebindings in 1_usize..16,
    ) {
        let registry = SymbolRegistry::new();
        let mut original = registry.intern(&name).expect("a fresh registry accepts a name");
        for _ in 0..rebindings {
            prop_assert!(registry.remove(&name), "the bound name is removable");
            let replacement = registry.intern(&name).expect("the unbound name can be rebound");
            prop_assert_ne!(replacement.id(), original.id());
            prop_assert!(SymbolData::lookup(original.id()).is_ok(), "global metadata remains valid after unbinding");
            prop_assert_eq!(original.name(), Some(name.as_str()));
            original = replacement;
        }
    }

    #[test]
    fn repeated_name_interning_shares_one_payload_and_rejects_registration(
        name in "[a-z]{1,12}", repetitions in 1_usize..16,
    ) {
        let registry = SymbolRegistry::new();
        let first = registry.intern(&name).expect("a fresh registry accepts a name");
        for _ in 0..repetitions {
            let repeated = registry.intern(&name).expect("interning a bound name succeeds");
            prop_assert!(Arc::ptr_eq(&first, &repeated), "interning reuses the owning payload");
        }
        prop_assert_eq!(registry.len(), 1);
        prop_assert!(matches!(registry.register(&name), Err(SymbolError::DuplicateName(_))), "explicit registration rejects a bound name");
    }
}
