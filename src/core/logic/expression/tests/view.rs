//! Logical inspection, lazy projection, and storage-independent transformation.

use core::ops::Div;

use alloc::{sync::Arc, vec, vec::Vec};

use proptest::{prop_assert, prop_assert_eq, prop_assert_ne, proptest};

use crate::{
    collections::HashMap,
    core::{Context, Pattern, RewriteRule, Symbol, UserFunction},
};

use super::super::{Expr, ExprView, Node};

#[test]
fn anonymous_symbol_view_keeps_identity_without_a_display_name() {
    let anonymous = Symbol::anon();
    let expression = anonymous.to_expr();
    let viewed = expression.view();
    assert!(
        matches!(&viewed, ExprView::Symbol(symbol) if *symbol == anonymous),
        "the view retains the typed identity"
    );
    assert_eq!(expression.symbol_name(), None);
    assert!(
        matches!(&viewed, ExprView::Symbol(symbol) if symbol.name().is_none()),
        "anonymous metadata has no name"
    );
    assert!(anonymous.unregister(), "the anonymous registration exists");
    assert!(
        matches!(&viewed, ExprView::Symbol(symbol) if symbol.to_expr().same_node(&expression)),
        "name unbinding retains metadata"
    );
}

proptest! {
    #[test]
    fn symbol_views_retain_identity_without_sharing_another_owner(
        name in "[a-z][a-z0-9_]{0,12}",
    ) {
        let first_context = Context::new();
        let second_context = Context::new();
        let first = first_context.symb(&name);
        let second = second_context.symb(&name);
        let expression = first.to_expr();
        let node = expression.0.as_ref().expect("a live expression owns its node");
        let owners = Arc::strong_count(node);
        let ExprView::Symbol(viewed) = expression.view() else {
            return Err(proptest::test_runner::TestCaseError::fail("a symbol exposes its identity"));
        };
        prop_assert_eq!(Arc::strong_count(node), owners);
        prop_assert_eq!(viewed, first);
        prop_assert_ne!(viewed, second);
        prop_assert_eq!(expression.as_symbol(), Some(viewed));
        first_context.clear_symbols();
        prop_assert!(viewed.to_expr().same_node(&expression));
        let rebound = first_context.symb(&name);
        prop_assert_ne!(viewed, rebound);
        prop_assert_eq!(expression.symbol_name(), Some(name.as_str()));
        drop(first_context);
        prop_assert!(viewed.to_expr().same_node(&expression));
    }

    #[test]
    fn logical_sum_terms_reconstruct_the_canonical_root(
        low in 2_i64..=5,
        high in 6_i64..=9,
        coefficient in 1_i32..=6,
        constant in -6_i32..=6,
        tail_count in 0_u8..=8,
    ) {
        let context = Context::new();
        let x = context.symb("x");
        let y = context.symb("y");
        let weighted = Expr::product(vec![Expr::number(coefficient), x.pow(low)]);
        let mut inputs = vec![weighted.clone(), x.pow(high), y.to_expr(), Expr::number(constant)];
        inputs.extend((-8_i64..0).take(usize::from(tail_count)).map(|exponent| x.pow(exponent)));
        let root = Expr::sum(inputs);
        prop_assert!(matches!(root.node(), Node::Sum(stored)
            if stored.iter().any(|term| matches!(term.node(), Node::Poly(_)))));
        let ExprView::Sum(terms) = root.view() else {
            return Err(proptest::test_runner::TestCaseError::fail("additive storage exposes a sum"));
        };
        let logical: Vec<_> = terms.collect();
        let expected_count = usize::from(tail_count).saturating_add(if constant == 0 { 3 } else { 4 });
        prop_assert_eq!(logical.len(), expected_count);
        prop_assert!(logical.contains(&weighted));
        prop_assert!(logical.contains(&x.pow(high)));
        prop_assert!(logical.contains(&y.to_expr()));
        prop_assert!(!logical.iter().any(|term| matches!(term.view(), ExprView::Sum(_))));
        let mut cursor = root.children();
        let mut remaining = expected_count;
        while cursor.next().is_some() {
            remaining = remaining.saturating_sub(1);
            prop_assert_eq!(cursor.len(), remaining);
        }
        prop_assert_eq!(remaining, 0);
        prop_assert!(Expr::sum(logical).same_node(&root));

        let exact = Pattern::sum(root.children().map(Pattern::exact).collect());
        let rule = RewriteRule::new(exact, Pattern::exact(y))
            .expect("literal patterns require no captures");
        prop_assert_eq!(rule.apply_all(&root, Some(0), None), Ok(y.to_expr()));
    }

    #[test]
    fn view_counting_and_partial_iteration_do_not_project_unused_terms(
        low in 2_i64..=5,
        high in 6_i64..=9,
    ) {
        let context = Context::new();
        let base = context.symb("x").to_expr();
        let root = Expr::sum(vec![base.pow(low), base.pow(high)]);
        let node = base.0.as_ref().expect("a live expression owns its node");
        let owners = Arc::strong_count(node);
        let ExprView::Sum(mut terms) = root.view() else {
            return Err(proptest::test_runner::TestCaseError::fail("compact storage exposes a sum"));
        };
        prop_assert_eq!(Arc::strong_count(node), owners);
        prop_assert_eq!(terms.clone().count(), 2);
        prop_assert_eq!(Arc::strong_count(node), owners);

        let mut copy = terms.clone();
        let first = terms.next().expect("two logical terms");
        prop_assert_eq!(terms.len(), 1);
        prop_assert_eq!(copy.len(), 2);
        prop_assert_eq!(Arc::strong_count(node), owners.saturating_add(1));
        let copied = copy.next().expect("the cloned cursor starts at the first term");
        prop_assert!(first.same_node(&copied));
        prop_assert_eq!(Arc::strong_count(node), owners.saturating_add(1));
        drop(first);
        drop(copied);
        prop_assert_eq!(Arc::strong_count(node), owners);
    }

    #[test]
    fn commutative_capture_selection_uses_structural_order_across_compact_groups(
        exponent in 2_i64..=12,
    ) {
        let context = Context::new();
        let x = context.symb("x");
        let y = context.symb("y");
        let packed = context.register_function("packed", UserFunction::new(2..=2))
            .expect("the custom name is unbound");
        let power = x.pow(exponent);
        let input = Expr::sum(vec![x.to_expr(), power.clone(), y.to_expr()]);
        let a = Pattern::capture("a");
        let b = Pattern::capture("b");
        let output = Pattern::apply(packed.clone(), vec![a.clone(), b.clone()])
            .expect("packed is binary");
        let rule = RewriteRule::new(Pattern::sum(vec![a, b]), output)
            .expect("both result captures are supplied by the input");
        let grouped = packed.call(vec![x.to_expr(), y.to_expr()]).expect("packed is binary");
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(Expr::sum(vec![grouped, power])));
    }

    #[test]
    fn child_counts_follow_iteration_and_exhaustion_is_fused(
        kind in 0_u8..=11,
        low in 2_i64..=5,
        high in 6_i64..=9,
    ) {
        let context = Context::new();
        let x = context.symb("x");
        let y = context.symb("y");
        let function = context.register_function("pair", UserFunction::new(2..=2))
            .expect("the custom name is unbound");
        let root = match kind {
            0 => Expr::number(low),
            1 => Expr::i(),
            2 => Expr::pi(),
            3 => Expr::e(),
            4 => x.to_expr(),
            5 => x.sin(),
            6 => function.call(vec![x.to_expr(), y.to_expr()]).expect("pair is binary"),
            7 => Expr::product(vec![x.to_expr(), y.to_expr()]),
            8 => Div::div(x.to_expr(), y.to_expr()),
            9 => x.pow(low),
            10 => Expr::derivative_symbol(x.sin(), x, 2),
            _ => Expr::sum(vec![x.pow(low), x.pow(high), Expr::number(1)]),
        };
        let mut children = root.children();
        let mut remaining = children.len();
        let mut yielded = Vec::new();
        prop_assert_eq!(children.size_hint(), (remaining, Some(remaining)));
        while let Some(child) = children.next() {
            yielded.push(child);
            remaining = remaining.saturating_sub(1);
            prop_assert_eq!(children.len(), remaining);
            prop_assert_eq!(children.size_hint(), (remaining, Some(remaining)));
        }
        prop_assert_eq!(remaining, 0);
        prop_assert_eq!(children.next(), None);
        prop_assert_eq!(children.next(), None);
        prop_assert_eq!(root.children().collect::<Vec<_>>(), yielded);
    }

    #[test]
    fn logical_children_own_metadata_after_context_and_root_drop(
        low in 2_i64..=5,
        high in 6_i64..=9,
    ) {
        let children = {
            let context = Context::new();
            let x = context.symb("x");
            let root = Expr::sum(vec![x.pow(low), x.pow(high)]);
            root.children().collect::<Vec<_>>()
        };
        prop_assert_eq!(children.len(), 2);
        prop_assert!(children.iter().all(|child| child.contains_var("x")));
        let reconstructed = Expr::sum(children.clone());
        prop_assert_eq!(reconstructed.children().collect::<Vec<_>>(), children);
    }

    #[test]
    fn fold_and_map_visit_shared_monomials_without_additive_storage_groups(
        low in 2_i64..=5,
        high in 6_i64..=9,
        constant in 1_i32..=6,
    ) {
        let context = Context::new();
        let x = context.symb("x");
        let y = context.symb("y");
        let u = context.symb("u");
        let target = x.pow(low);
        let other = x.pow(high);
        let compact = Expr::sum(vec![target.clone(), other.clone()]);
        let additive = Expr::sum(vec![compact.clone(), y.to_expr(), Expr::number(constant)]);
        let function = context.register_function("pair", UserFunction::new(2..=2))
            .expect("the custom name is unbound");
        let root = function.call(vec![additive, target.clone()]).expect("pair is binary");
        let folded = root.fold(HashMap::<Expr, usize>::new(), |mut counts, node| {
            let count = counts.entry(node.clone()).or_default();
            *count = count.saturating_add(1);
            counts
        });
        prop_assert!(!folded.contains_key(&compact));
        prop_assert_eq!(folded.get(&target), Some(&1));
        prop_assert_eq!(folded.get(&other), Some(&1));
        prop_assert!(folded.values().all(|count| *count == 1));
        let mut mapped = HashMap::<Expr, usize>::new();
        let unchanged = root.map(|node| {
            let count = mapped.entry(node.clone()).or_default();
            *count = count.saturating_add(1);
            Ok::<_, ()>(None)
        }).expect("the counting transformation cannot fail");
        prop_assert!(unchanged.same_node(&root));
        prop_assert_eq!(folded, mapped);

        let transformed = root.map(|node| {
            Ok::<_, ()>(node.same_node(&target).then(|| u.to_expr()))
        }).expect("concrete replacement cannot fail");
        let expected_sum = Expr::sum(vec![u.to_expr(), other, y.to_expr(), Expr::number(constant)]);
        let expected = function.call(vec![expected_sum, u.to_expr()]).expect("pair is binary");
        prop_assert_eq!(transformed, expected);
    }
}
