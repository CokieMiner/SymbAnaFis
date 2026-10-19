//! Ordered operand groups, overlap priority, and surviving original descendants.

use alloc::{format, vec, vec::Vec};

use proptest::{arbitrary::any, prop_assert, prop_assert_eq, proptest};

use crate::core::{Context, Expr, ExprView, Substitution, Substitutions, UserFunction};

fn combine(sum: bool, operands: Vec<Expr>) -> Expr {
    if sum {
        Expr::sum(operands)
    } else {
        Expr::product(operands)
    }
}

proptest! {
    #[test]
    fn concrete_groups_preserve_unmatched_operands_and_shared_edges(
        sum in any::<bool>(),
        remainder_count in 1_usize..=12,
        edge_count in 1_usize..=8,
        coefficient in 2_i64..=20,
        reverse in any::<bool>(),
    ) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let u = context.symb("u")?.to_expr();
        let source = combine(sum, vec![x.clone(), y.clone()]);
        let mut rest: Vec<_> = (0..remainder_count)
            .map(|index| context.symb(&format!("r{index}")).expect("symbol allocation").to_expr())
            .collect();
        rest.push(Expr::number(coefficient));
        let mut original = rest.clone();
        original.extend([x, y]);
        if reverse {
            original.reverse();
        }
        let input = combine(sum, original);
        rest.push(u.clone());
        let expected = combine(sum, rest);
        let substitution = Substitution::new(source, u);
        let shared = context.register_function("shared", UserFunction::default()).expect("the custom name is unbound");
        prop_assert_eq!(
            substitution.apply_all(&shared.call(vec![input; edge_count]).expect("shared is variadic")),
            shared.call(vec![expected; edge_count]).expect("shared is variadic"),
        );
    }

    #[test]
    fn group_matches_prune_consumed_subtrees_and_transform_only_the_remainder(
        sum in any::<bool>(),
        value in -20_i64..=20,
        reverse_entries in any::<bool>(),
    ) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let z = context.symb("z")?.to_expr();
        let u = context.symb("u")?.to_expr();
        let a = context.symb("a")?.to_expr();
        let source = combine(sum, vec![x.clone(), y.clone()]);
        let mut entries = vec![
            (source, u.clone()),
            (x.clone(), a.clone()),
            (y.clone(), Expr::number(99)),
            (z.clone(), Expr::number(value)),
            (u.clone(), Expr::number(100)),
        ];
        if reverse_entries {
            entries.reverse();
        }
        let substitutions = Substitutions::new(entries).expect("the sources are distinct");
        let input = combine(sum, vec![x.clone(), y, z, x.sin()]);
        let expected = combine(sum, vec![u, Expr::number(value), a.sin()]);
        prop_assert_eq!(substitutions.apply_all(&input), expected);
    }

    #[test]
    fn overlapping_groups_follow_entry_order(sum in any::<bool>(), reverse in any::<bool>()) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let z = context.symb("z")?.to_expr();
        let u = context.symb("u")?.to_expr();
        let v = context.symb("v")?.to_expr();
        let first = combine(sum, vec![x.clone(), y.clone()]);
        let second = combine(sum, vec![y.clone(), z.clone()]);
        let mut entries = vec![(first.clone(), u.clone()), (second.clone(), v.clone())];
        if reverse {
            entries.reverse();
        }
        let substitutions = Substitutions::new(entries).expect("the sources are distinct");
        let expected = if reverse {
            combine(sum, vec![v, x.clone()])
        } else {
            combine(sum, vec![u, z.clone()])
        };
        let iterated: Vec<_> = substitutions.iter().map(|(from, _)| from.clone()).collect();
        prop_assert_eq!(iterated, if reverse {vec![second, first]} else {vec![first, second]});
        prop_assert_eq!(substitutions.apply_all(&combine(sum, vec![x, y, z])), expected);
    }

    #[test]
    fn disjoint_groups_are_simultaneous_and_inserted_sources_are_not_revisited(sum in any::<bool>()) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let z = context.symb("z")?.to_expr();
        let w = context.symb("w")?.to_expr();
        let first = combine(sum, vec![x.clone(), y.clone()]);
        let second = combine(sum, vec![z.clone(), w.clone()]);
        let substitutions = Substitutions::new([
            (first.clone(), second.clone()), (second, first),
        ]).expect("the sources are distinct");
        let input = combine(sum, vec![x, y, z, w]);
        prop_assert_eq!(substitutions.apply_all(&input), input);
        prop_assert_eq!(substitutions.len(), 2);
    }

    #[test]
    fn polynomial_operand_subsets_preserve_other_monomials_and_outer_constants(
        low in 2_i64..=4,
        high in 6_i64..=8,
        other in 10_i64..=12,
        constant in -20_i64..=20,
    ) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let u = context.symb("u")?.to_expr();
        let first = x.pow(low);
        let second = x.pow(high);
        let source = Expr::sum(vec![first.clone(), second.clone()]);
        prop_assert!(matches!(source.view(), ExprView::Sum(_)));
        let input = Expr::sum(vec![first, second, x.pow(other), y.pow(low), y.pow(high), Expr::number(constant)]);
        let expected = Expr::sum(vec![u.clone(), x.pow(other), y.pow(low), y.pow(high), Expr::number(constant)]);
        prop_assert_eq!(Substitution::new(source, u).apply_all(&input), expected);
    }

    #[test]
    fn exact_whole_matches_override_groups_and_identity_groups_preserve_children(sum in any::<bool>()) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let z = context.symb("z")?.to_expr();
        let source = combine(sum, vec![x.clone(), y.clone()]);
        let input = combine(sum, vec![x.clone(), y, z]);
        let destination = context.symb("destination")?.to_expr();
        let whole = Substitutions::new([
            (source.clone(), Expr::number(99)), (input.clone(), destination.clone()),
        ]).expect("the sources are distinct");
        prop_assert_eq!(&whole.apply_all(&input), &destination);
        let identity = Substitutions::new([
            (source.clone(), source), (x, Expr::number(99)),
        ]).expect("the sources are distinct");
        prop_assert_eq!(identity.apply_all(&input), input);
    }
}
