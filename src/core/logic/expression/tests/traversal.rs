//! Memoization and matched-subtree pruning in exact substitution traversal.

use alloc::vec;

use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, UserFunction},
};

use super::super::{Expr, Selection};

proptest! {
    #[test]
    fn shared_logical_and_stored_nodes_are_selected_once(
        edge_count in 2_usize..=16,
        first_power in 2_i64..=5,
        second_power in 6_i64..=9,
    ) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let target = x.pow(first_power);
        let polynomial = Expr::sum(vec![target.clone(), x.pow(second_power)]);
        let mut edges = vec![polynomial; edge_count];
        edges.push(target.clone());
        let tuple = context.register_function("tuple", UserFunction::default()).expect("the custom name is unbound");
        let input = tuple.call(edges).expect("tuple accepts any arity");
        let mut counts = HashMap::<Expr, usize>::new();
        let result = input.substitute_logical_with(|node| {
            let count = counts.entry(node.clone()).or_default();
            *count = count.checked_add(1).expect("the bounded graph has a bounded visit count");
            None
        });
        prop_assert_eq!(result, input);
        prop_assert_eq!(counts.get(&target), Some(&1));
        prop_assert!(counts.values().all(|count| *count == 1));
    }

    #[test]
    fn exact_parent_matches_prune_all_descendant_visits(depth in 1_usize..=32) {
        let context = Context::new();
        let leaf = context.symb("leaf")?.to_expr();
        let mut root = leaf;
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("the custom name is unbound");
        for _ in 0..depth {
            root = f.call(vec![root]).expect("f is unary");
        }
        let replacement = context.symb("replacement")?.to_expr();
        let mut visits = 0_usize;
        let result = root.substitute_logical_with(|node| {
            visits = visits.checked_add(1).expect("the bounded graph has a bounded visit count");
            (node == &root).then(|| Selection::Whole(replacement.clone()))
        });
        prop_assert_eq!(result, replacement);
        prop_assert_eq!(visits, 1);
    }

    #[test]
    fn logical_traversal_stops_on_the_first_error_and_preserves_the_input(
        depth in 1_usize..=24,
        stop_after in 0_usize..=24,
        compact in proptest::arbitrary::any::<bool>(),
    ) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let mut input = if compact { Expr::sum(vec![x.pow(2), x.pow(3)]) } else { x };
        let shared = context.register_function("shared", UserFunction::new(2..=2)).expect("the custom name is unbound");
        for _ in 0..depth {
            input = shared.call(vec![input.clone(), input]).expect("shared accepts two arguments");
        }
        let preserved = input.clone();
        let mut total = 0_usize;
        input.map(|_| {
            total = total.checked_add(1).expect("the graph is bounded");
            Ok::<_, ()>(None)
        }).expect("the counting traversal cannot fail");
        let mut visits = 0_usize;
        let result = input.map(|_| {
            visits = visits.checked_add(1).expect("the graph is bounded");
            if visits > stop_after { Err("stop") } else { Ok(None) }
        });
        if stop_after < total {
            prop_assert_eq!(result, Err("stop"));
            prop_assert_eq!(visits, stop_after.checked_add(1).expect("the stop count is bounded"));
        } else {
            prop_assert_eq!(&result.expect("the stop point lies outside the traversal"), &input);
            prop_assert_eq!(visits, total);
        }
        prop_assert_eq!(input, preserved);
    }
}
