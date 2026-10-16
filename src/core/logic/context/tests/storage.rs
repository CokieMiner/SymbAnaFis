//! Context storage capacity and callback release.

use core::sync::atomic::{AtomicBool, Ordering};

use alloc::{format, sync::Arc, vec::Vec};

use crate::core::Expr;

use super::super::{Context, UserFunction};

#[test]
fn churned_binding_maps_reclaim_capacity_while_retaining_live_entries() {
    let context = Context::new();
    let retained = context.symb("context_storage_retained").to_expr();
    let symbol_names: Vec<_> = (0..128)
        .map(|index| format!("context_storage_symbol_{index}"))
        .collect();
    for name in &symbol_names {
        assert_eq!(context.symb(name).name().as_deref(), Some(name.as_str()));
    }
    let symbol_capacity_before = context.bindings.lock().symbols.capacity();
    assert!(symbol_capacity_before > 64, "the symbol churn grew the map");
    for name in symbol_names.iter().take(127) {
        assert!(context.remove_symbol(name));
    }
    let symbol_capacity_after = context.bindings.lock().symbols.capacity();
    assert!(symbol_capacity_after < symbol_capacity_before);
    assert!(symbol_capacity_after <= 64);
    assert!(retained.contains_var("context_storage_retained"));
    assert_eq!(context.symbol_count(), 2);

    let function_names: Vec<_> = (0..128)
        .map(|index| format!("context_storage_function_{index}"))
        .collect();
    for name in &function_names {
        context
            .register_function(name, UserFunction::new(0..=0))
            .expect("generated custom names are distinct");
    }
    let function_capacities_before = {
        let bindings = context.bindings.lock();
        (
            bindings.function_names.capacity(),
            bindings.functions.capacity(),
        )
    };
    assert!(function_capacities_before.0 > 64);
    assert!(function_capacities_before.1 > 64);
    for name in function_names.iter().take(127) {
        assert!(context.remove_function(name));
    }
    let function_capacities_after = {
        let bindings = context.bindings.lock();
        (
            bindings.function_names.capacity(),
            bindings.functions.capacity(),
        )
    };
    assert!(function_capacities_after.0 < function_capacities_before.0);
    assert!(function_capacities_after.1 < function_capacities_before.1);
    assert!(function_capacities_after.0 <= 64);
    assert!(function_capacities_after.1 <= 64);

    context.clear_functions();
    let bindings = context.bindings.lock();
    assert_eq!(bindings.function_names.capacity(), 0);
    assert_eq!(bindings.functions.capacity(), 0);
}

#[test]
fn removing_and_clearing_functions_release_callbacks_after_unlocking() {
    struct Capture(Context, Arc<AtomicBool>);

    impl Drop for Capture {
        fn drop(&mut self) {
            self.1
                .store(self.0.bindings.try_lock().is_some(), Ordering::Relaxed);
        }
    }

    let context = Context::new();
    let removed_observed = Arc::new(AtomicBool::new(false));
    let capture = Capture(context.clone(), Arc::clone(&removed_observed));
    context
        .register_function(
            "context_storage_remove_callback",
            UserFunction::new(0..=0).body(Arc::new(move |_| {
                core::hint::black_box(&capture);
                Expr::number(0)
            })),
        )
        .expect("the custom name is unbound");
    assert!(context.remove_function("context_storage_remove_callback"));
    assert!(removed_observed.load(Ordering::Relaxed));

    let cleared_observed = Arc::new(AtomicBool::new(false));
    let clear_capture = Capture(context.clone(), Arc::clone(&cleared_observed));
    context
        .register_function(
            "context_storage_clear_callback",
            UserFunction::new(0..=0).body(Arc::new(move |_| {
                core::hint::black_box(&clear_capture);
                Expr::number(0)
            })),
        )
        .expect("the second custom name is unbound");
    context.clear_functions();
    assert!(cleared_observed.load(Ordering::Relaxed));
}
