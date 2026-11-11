//! Registry sharing and identity preservation under name rebinding.

use core::ops::{Add, Div, Mul, Sub};

use alloc::{format, sync::Arc};

use proptest::prelude::{
    ProptestConfig, any, prop_assert, prop_assert_eq, prop_assert_ne, proptest,
};

use crate::core::{Context, Expr, Symbol};

use super::super::super::{SymbolData, SymbolError, symbols::SymbolRegistry};

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
            prop_assert!(SymbolData::lookup_permanent(original.id()).is_ok(), "global metadata remains valid after unbinding");
            prop_assert_eq!(original.as_str(), Some(name.as_str()));
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
            prop_assert_eq!(first, repeated);
            prop_assert!(Arc::ptr_eq(&SymbolData::of(first), &SymbolData::of(repeated)), "interning reuses the owning payload");
        }
        prop_assert_eq!(registry.len(), 1);
        prop_assert!(matches!(registry.register(&name), Err(SymbolError::DuplicateName(_))), "explicit registration rejects a bound name");
    }

    #[test]
    fn symbol_display_and_as_str_match_expression_display(name in "[a-z]{1,12}") {
        let context = Context::new().expect("context");
        let local = context.symb(&name).expect("local symbol");
        prop_assert_eq!(local.as_str(), Some(name.as_str()));
        prop_assert_eq!(format!("{local}"), format!("{}", local.to_expr()));

        let anon = Symbol::anon().expect("anonymous symbol");
        prop_assert_eq!(anon.as_str(), None);
        prop_assert_eq!(format!("{anon}"), format!("{}", anon.to_expr()));
        prop_assert_eq!(format!("{anon}"), format!("${}", anon.id()));
    }

    #[test]
    fn symbol_operators_with_i64_rhs_agree_with_expr_operators(
        value in any::<i64>(),
        name in "[a-z]{1,12}",
    ) {
        let context = Context::new().expect("context");
        let x = context.symb(&name).expect("local symbol");
        let x_expr = x.to_expr();
        let num = Expr::number(value);
        prop_assert_eq!(Add::add(x, value), Add::add(&x_expr, &num));
        prop_assert_eq!(Add::add(&x, value), Add::add(&x_expr, &num));
        prop_assert_eq!(Sub::sub(x, value), Sub::sub(&x_expr, &num));
        prop_assert_eq!(Sub::sub(&x, value), Sub::sub(&x_expr, &num));
        prop_assert_eq!(Mul::mul(x, value), Mul::mul(&x_expr, &num));
        prop_assert_eq!(Mul::mul(&x, value), Mul::mul(&x_expr, &num));
        prop_assert_eq!(Div::div(x, value), Div::div(&x_expr, &num));
        prop_assert_eq!(Div::div(&x, value), Div::div(&x_expr, &num));
    }
}
