use std::{sync::Barrier, thread};

use alloc::vec::Vec;

use crate::{
    core::{Context, Symbol},
    errors::SymbolError,
};

use super::super::{SymbolData, SymbolRegistry};

#[test]
fn concurrent_name_registration_publishes_one_id_and_removal_preserves_it() {
    let registry = SymbolRegistry::new();
    let barrier = Barrier::new(9);
    let symbols = thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    registry
                        .intern("registry_concurrent_name")
                        .expect("registration succeeded")
                })
            })
            .collect();
        barrier.wait();
        workers
            .into_iter()
            .map(|worker| worker.join().expect("worker succeeded"))
            .collect::<Vec<_>>()
    });
    let first = symbols.first().expect("workers produced metadata");
    assert!(symbols.iter().all(|symbol| symbol.id() == first.id()));
    assert_eq!(registry.len(), 1);
    assert!(registry.remove("registry_concurrent_name"));
    assert_eq!(registry.len(), 0);
    assert!(SymbolData::lookup(first.id()).is_ok());
}

#[test]
fn implicit_conversion_survives_concurrent_unbinding_and_replacement() {
    let registry = SymbolRegistry::new();
    let data = registry
        .intern("concurrent_unbound_symbol")
        .expect("identity");
    let symbol = Symbol::from_id(data.id()).expect("published identity");
    let expression = symbol.to_expr();
    let sum = symbol + 1;
    let barrier = Barrier::new(9);
    thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let expression = &expression;
                let sum = &sum;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    for _ in 0..64 {
                        assert!(symbol.to_expr().same_node(expression));
                        assert!((symbol + 1).same_node(sum));
                        assert_eq!(Symbol::from_id(symbol.id()), Ok(symbol));
                    }
                })
            })
            .collect();
        barrier.wait();
        for _ in 0..32 {
            assert!(registry.remove("concurrent_unbound_symbol"));
            let replacement = registry
                .intern("concurrent_unbound_symbol")
                .expect("replacement identity");
            assert_ne!(replacement.id(), symbol.id());
        }
        for worker in workers {
            worker.join().expect("conversion worker succeeded");
        }
    });
    drop(registry);
    drop(data);
    assert!((symbol + 1).same_node(&sum));
}

#[test]
fn scoped_conversion_survives_concurrent_name_clearing_and_clone_reset() {
    let context = Context::new();
    let mut manager = context.clone();
    let symbol = context.symb("concurrent_scoped_symbol");
    let id = symbol.id();
    let expression = symbol.to_expr();
    let sum = symbol + 1;
    let metadata = alloc::sync::Arc::downgrade(&SymbolData::lookup(id).expect("local identity"));
    let barrier = Barrier::new(9);
    thread::scope(|scope| {
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let expression = &expression;
                let sum = &sum;
                let barrier = &barrier;
                let context = &context;
                scope.spawn(move || {
                    barrier.wait();
                    for _ in 0..64 {
                        assert!(symbol.to_expr().same_node(expression));
                        assert!((symbol + 1).same_node(sum));
                        assert_eq!(context.symbol_from_id(id), Ok(symbol));
                        assert_eq!(Symbol::from_id(id), Err(SymbolError::InvalidId(id)));
                    }
                })
            })
            .collect();
        barrier.wait();
        for _ in 0..32 {
            context.clear_all();
            let replacement = context.symb("concurrent_scoped_symbol");
            assert_ne!(replacement.id(), id);
            manager.reset();
        }
        for worker in workers {
            worker.join().expect("scoped conversion worker succeeded");
        }
    });
    drop(context);
    assert!(metadata.upgrade().is_some());
    drop(expression);
    drop(sum);
    assert!(metadata.upgrade().is_none());
    assert!(SymbolData::lookup(id).is_err());
}
