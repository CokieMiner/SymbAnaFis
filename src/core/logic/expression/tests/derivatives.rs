//! Derivative identities, checked orders, variable identities, and reconstruction.

use core::{num::NonZeroU32, ops::Div};

use alloc::vec;

use proptest::{collection, prop_assert, prop_assert_eq, proptest};

use crate::core::{Context, Expr, ExprView, Pattern, RewriteRule, Substitution};

use super::super::{Node, Symbol, SymbolData};

proptest! {
    #[test]
    fn zero_order_retains_the_original_expression_and_its_domains(kind in 0_usize..6) {
        let context = Context::new();
        let x = context.symb("x")?;
        let unused = context.symb("unused")?;
        let input = match kind {
            0 => x.to_expr(),
            1 => x.sin(),
            2 => Div::div(Expr::number(1), x),
            3 => Expr::sum(vec![x.pow(2), x.pow(3)]),
            4 => Expr::number(0).pow(0),
            _ => Expr::derivative(x.sin(), x, 3),
        };
        prop_assert_eq!(&Expr::derivative(input.clone(), unused, 0), &input);
        prop_assert!(!input.contains_symbol(unused));
    }

    #[test]
    fn same_variable_orders_collect_into_one_positive_node(
        orders in collection::vec(0_u32..=32, 0..=16),
        kind in 0_usize..4,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let base = match kind {
            0 => y.to_expr(),
            1 => y.sin(),
            2 => Div::div(Expr::number(1), y),
            _ => Expr::sum(vec![y.pow(2), y.pow(3)]),
        };
        let mut nested = base.clone();
        let mut total = 0_u32;
        for order in orders {
            total = total.saturating_add(order);
            nested = stored_derivative(nested, x, order);
        }
        prop_assert_eq!(&nested, &stored_derivative(base.clone(), x, total));
        if total == 0 {
            prop_assert_eq!(nested, base);
        } else {
            let valid = matches!(nested.view(), ExprView::Derivative { inner, var, order }
                if inner == &base && var == x && order == total);
            prop_assert!(valid);
            prop_assert_eq!(nested.max_depth(), base.max_depth().saturating_add(1));
        }
    }

    #[test]
    fn equal_names_do_not_merge_distinct_differentiation_variables(
        first in 1_u32..=128,
        second in 1_u32..=128,
    ) {
        let first_context = Context::new();
        let second_context = Context::new();
        let x = first_context.symb("x")?;
        let other_x = second_context.symb("x")?;
        let inner = stored_derivative(x.sin(), x, first);
        let outer = stored_derivative(inner.clone(), other_x, second);
        let distinct = matches!(outer.view(), ExprView::Derivative { inner: child, var, order }
            if child == &inner && var == other_x && order == second);
        prop_assert!(distinct);
        prop_assert!(outer.contains_symbol(x));
        prop_assert!(outer.contains_symbol(other_x));
    }

    #[test]
    fn rebuilt_derivatives_use_the_canonical_order_kernel(
        inner_order in 1_u32..=64,
        outer_order in 1_u32..=64,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let target = context.symb("target")?;
        let base = x.abs();
        let replacement = Expr::derivative(base.clone(), x, inner_order);
        let input = stored_derivative(target.into(), x, outer_order);
        let expected = Expr::derivative(base, x, inner_order.saturating_add(outer_order));
        prop_assert_eq!(&input.substitute_symbol(target, &replacement), &expected);
        let substitution = Substitution::new(target.into(), replacement.clone());
        prop_assert_eq!(&substitution.apply_all(&input), &expected);
        let mapped = input.map(|node| Ok::<_, ()>((node.as_symbol() == Some(target)).then(|| replacement.clone())));
        prop_assert_eq!(mapped.as_ref(), Ok(&expected));
        let rule = RewriteRule::new(Pattern::exact(target), Pattern::exact(replacement))?;
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(expected));
    }
}

#[test]
fn maximum_order_is_exact_and_overflow_retains_nested_nodes() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let base = x.abs();
    let near_limit = Expr::derivative(base.clone(), x, u32::MAX.saturating_sub(1));
    let maximum = Expr::derivative(base.clone(), x, u32::MAX);
    assert_eq!(Expr::derivative(near_limit, x, 1), maximum);
    let overflow = Expr::derivative(maximum.clone(), x, 1);
    assert!(
        matches!(overflow.view(), ExprView::Derivative { inner, var, order: 1 }
        if inner == &maximum && var == x)
    );
    let reverse = Expr::derivative(Expr::derivative(base, x, 1), x, u32::MAX);
    assert!(
        matches!(reverse.view(), ExprView::Derivative { inner, order: u32::MAX, .. }
        if matches!(inner.view(), ExprView::Derivative { order: 1, .. }))
    );
}

#[test]
fn repeated_derivative_construction_has_bounded_stack_and_depth() {
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| {
            let context = Context::new();
            let x = context.symb("x").expect("symbol allocation");
            let mut expression = x.abs();
            for _ in 0..10_000 {
                expression = Expr::derivative(expression, x, 1);
            }
            assert!(matches!(
                expression.view(),
                ExprView::Derivative { order: 10_000, .. }
            ));
            assert_eq!(expression.max_depth(), 3);
            assert_eq!(expression.node_count(), 3);
        })
        .expect("small-stack derivative worker")
        .join()
        .expect("derivative normalization and destruction remain iterative");
}

fn stored_derivative(inner: Expr, variable: Symbol<'_>, order: u32) -> Expr {
    match NonZeroU32::new(order) {
        Some(positive) => {
            Node::normalize_derivative(inner, SymbolData::for_symbol(variable), positive)
        }
        None => inner,
    }
}
