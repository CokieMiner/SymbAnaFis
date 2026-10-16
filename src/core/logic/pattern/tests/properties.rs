//! Generated bounded DAGs exercise matching, memoization, and reconstruction.

use alloc::{format, vec, vec::Vec};

use proptest::{
    prelude::{Strategy, any, prop},
    prop_assert, prop_assert_eq, proptest,
};

use crate::{
    collections::HashMap,
    core::{Context, Function, UserFunction},
};

use super::super::{Expr, ExprView, Pattern, RewriteRule};

fn bounded_expression(steps: Vec<u8>, pair: &Function) -> Expr {
    let x = Expr::symbol("pattern_property_x");
    let y = Expr::symbol("pattern_property_y");
    steps
        .into_iter()
        .fold(x.clone(), |current, step| match step {
            0 => current.sin(),
            1 | 3 => pair
                .call(vec![current, y.clone()])
                .expect("pair accepts two arguments"),
            2 => pair
                .call(vec![current.clone(), current])
                .expect("pair accepts two arguments"),
            _ => pair
                .call(vec![current, x.clone()])
                .expect("pair accepts two arguments"),
        })
}

fn bounded_steps_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(0_u8..5, 0..=7)
}

fn bounded_expression_strategy() -> impl Strategy<Value = Expr> {
    bounded_steps_strategy().prop_map(|steps| {
        let context = Context::new();
        let pair = context
            .register_function("pair", UserFunction::new(2..=2))
            .expect("the custom name is unbound");
        bounded_expression(steps, &pair)
    })
}

fn bounded_rewrite_oracle(steps: &[u8], pair: &Function) -> Expr {
    let x = Expr::symbol("pattern_property_x");
    let y = Expr::symbol("pattern_property_y");
    let mut rewritten = x.sin();
    for &step in steps {
        rewritten = match step {
            0 => rewritten.sin().sin(),
            1 | 3 => pair
                .call(vec![rewritten, y.sin()])
                .expect("pair accepts two arguments")
                .sin(),
            2 => pair
                .call(vec![rewritten.clone(), rewritten])
                .expect("pair accepts two arguments")
                .sin(),
            _ => pair
                .call(vec![rewritten, x.sin()])
                .expect("pair accepts two arguments")
                .sin(),
        };
    }
    rewritten
}

proptest! {
    #[test]
    fn generated_function_patterns_require_exact_arity(
        pattern_arity in 0_usize..=5,
        input_arity in 0_usize..=6,
    ) {
        let context = Context::new();
        let function = context.register_function("arity", UserFunction::new(0..=6)).expect("the custom name is unbound");
        let pattern = Pattern::apply(
            function.clone(),
            (0..pattern_arity).map(|index| Pattern::capture(format!("arg{index}"))).collect(),
        ).expect("the generated pattern arity is accepted");
        let rule = RewriteRule::new(pattern, Pattern::exact(7))
            .expect("an exact replacement needs no captures");
        let arguments: Vec<_> = (0..input_arity)
            .map(|index| {
                Expr::number(
                    i64::try_from(index).expect("the generated arity fits in an integer literal"),
                )
            })
            .collect();
        let input = function.call(arguments).expect("the generated call arity is accepted");
        if pattern_arity == input_arity {
            prop_assert_eq!(
                rule.apply_all(&input, None, None),
                Ok(Expr::number(7)),
                "matching generated arities must instantiate the replacement"
            );
        } else {
            prop_assert_eq!(
                rule.apply_all(&input, None, None),
                Ok(input),
                "different generated arities must preserve the call"
            );
        }
    }
}

fn distinct_operand(index: usize) -> Expr {
    match index {
        0 => Expr::symbol("property_match_a"),
        1 => Expr::symbol("property_match_b"),
        2 => Expr::symbol("property_match_c"),
        3 => Expr::symbol("property_match_d"),
        4 => Expr::symbol("property_match_e"),
        _ => Expr::symbol("property_match_f"),
    }
}

proptest! {
    #[test]
    fn generated_sum_and_product_captures_cover_all_nested_operands(
        pattern_arity in 1_usize..=5,
        input_arity in 2_usize..=6,
    ) {
        let sum = Expr::sum((0..input_arity).map(distinct_operand).collect());
        let context = Context::new();
        let host = context.register_function("host", UserFunction::new(1..=1)).expect("the custom name is unbound");
        let sum_rule = RewriteRule::new(
            Pattern::apply(
                host.clone(),
                vec![Pattern::sum((0..pattern_arity).map(|index| Pattern::capture(format!("arg{index}"))).collect())],
            ).expect("the host is unary"),
            Pattern::exact(9),
        )
        .expect("the generated sum replacement needs no captures");
        let sum_input = host.call(vec![sum]).expect("the host is unary");
        let expected_sum = if pattern_arity <= input_arity {
            Expr::number(9)
        } else {
            sum_input.clone()
        };
        prop_assert_eq!(
            sum_rule.apply_all(&sum_input, None, None),
            Ok(expected_sum),
            "nonempty captures partition the complete nested operand list"
        );

        let product = Expr::product((0..input_arity).map(distinct_operand).collect());
        let product_rule = RewriteRule::new(
            Pattern::apply(
                host.clone(),
                vec![Pattern::product((0..pattern_arity).map(|index| Pattern::capture(format!("arg{index}"))).collect())],
            ).expect("the host is unary"),
            Pattern::exact(9),
        )
        .expect("the generated product replacement needs no captures");
        let product_input = host.call(vec![product]).expect("the host is unary");
        let expected_product = if pattern_arity <= input_arity {
            Expr::number(9)
        } else {
            product_input.clone()
        };
        prop_assert_eq!(
            product_rule.apply_all(&product_input, None, None),
            Ok(expected_product),
            "nonempty captures partition the complete nested operand list"
        );
    }
}

fn leaf(choice: u8) -> Expr {
    match choice {
        0 => Expr::symbol("pattern_leaf_x"),
        1 => Expr::symbol("pattern_leaf_y"),
        2 => Expr::symbol("pattern_leaf_z"),
        _ => Expr::number(7),
    }
}

proptest! {
    #[test]
    fn bounded_shared_template_depth_is_memoized(depth in 0_usize..=9) {
        let capture = Pattern::capture("property_deep_argument");
        let mut pattern = capture.clone();
        let leaf = Expr::symbol("property_deep_x");
        let mut expression = leaf.clone();
        let context = Context::new();
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the custom name is unbound");
        for _ in 0..depth {
            pattern = Pattern::apply(
                pair.clone(),
                vec![pattern.clone(), pattern.clone()],
            ).expect("pair accepts two patterns");
            expression = pair.call(vec![expression.clone(), expression]).expect("pair accepts two arguments");
        }
        let rule = RewriteRule::new(pattern, capture)
            .expect("the generated capture is bound in the replacement");
        prop_assert_eq!(
            rule.apply_all(&expression, None, None),
            Ok(leaf),
            "shared generated pairs must match without expanding exponentially"
        );
    }
}

proptest! {
    #[test]
    fn bounded_dag_rewrite_matches_the_direct_bottom_up_oracle(
        steps in bounded_steps_strategy()
    ) {
        let context = Context::new();
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the custom name is unbound");
        let expression = bounded_expression(steps.clone(), &pair);
        let capture = Pattern::capture("property_argument");
        let rule = RewriteRule::new(capture.clone(), capture.sin())
            .expect("the generated replacement capture is bound");
        prop_assert_eq!(
            rule.apply_all(&expression, None, None),
            Ok(bounded_rewrite_oracle(&steps, &pair)),
            "pattern reconstruction must agree with the independent recipe oracle"
        );
    }
}

proptest! {
    #[test]
    fn repeated_captures_accept_shared_nodes_and_reject_distinct_nodes(
        first in 0_u8..4,
        second in 0_u8..4,
    ) {
        let capture = Pattern::capture("property_repeated_argument");
        let context = Context::new();
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the custom name is unbound");
        let pattern = Pattern::apply(
            pair.clone(),
            vec![capture.clone(), capture.clone()],
        ).expect("pair accepts two patterns");
        let rule = RewriteRule::new(pattern, capture)
            .expect("the repeated capture is bound in the replacement");
        let first_leaf = leaf(first);
        let second_leaf = leaf(second);
        let matching = pair.call(vec![first_leaf.clone(), first_leaf.clone()]).expect("pair accepts two arguments");
        let matching_result = rule.apply_all(&matching, None, None)
            .expect("unrestricted matching cannot exhaust a limit");
        prop_assert_eq!(
            &matching_result,
            &first_leaf,
            "one canonical input satisfies repeated captures"
        );
        let different = first_leaf != second_leaf;
        let mismatch = pair.call(vec![first_leaf, second_leaf]).expect("pair accepts two arguments");
        if different {
            prop_assert_eq!(
                rule.apply_all(&mismatch, None, None),
                Ok(mismatch),
                "distinct canonical inputs cannot satisfy repeated captures"
            );
        }
    }
}

proptest! {
    #[test]
    fn guards_run_once_for_each_distinct_matching_dag_node(
        base_expression in bounded_expression_strategy()
    ) {
        let expression = base_expression.sin();
        let capture = Pattern::capture("property_guard_argument");
        let rule = RewriteRule::new(capture.sin(), capture.cos())
            .expect("the generated guard capture is bound");
        let expected_checks = expression.fold(0_usize, |count, node| {
            match node.view() {
                ExprView::Function { name, args }
                    if name.as_str() == "sin" && args.len() == 1 => count
                        .checked_add(1)
                        .expect("bounded generated DAG cannot exceed usize::MAX nodes"),
                ExprView::Number(_)
                | ExprView::ImaginaryUnit
                | ExprView::Pi
                | ExprView::Euler
                | ExprView::Symbol(_)
                | ExprView::Function { .. }
                | ExprView::Sum(_)
                | ExprView::Product(_)
                | ExprView::Div(..)
                | ExprView::Pow(..)
                | ExprView::Derivative { .. } => count,
            }
        });
        let mut checked: HashMap<Expr, usize> = HashMap::new();
        let result = rule.apply_all(&expression, None, Some(&mut |captures: &HashMap<&str, Expr>| {
            let captured = captures
                .get("property_guard_argument")
                .expect("the generated guard capture is bound");
            checked
                .entry(captured.clone())
                .and_modify(|count| {
                    *count = count
                        .checked_add(1)
                        .expect("bounded generated DAG cannot exceed usize::MAX guard calls");
                })
                .or_insert(1);
            true
        })).expect("unrestricted matching cannot exhaust a limit");
        prop_assert_eq!(
            checked.len(),
            expected_checks,
            "the guard must run once per distinct matching input node"
        );
        prop_assert!(
            checked.values().all(|count| *count == 1),
            "each distinct matching DAG node must invoke the guard exactly once"
        );
        let total_checks = checked.values().try_fold(0_usize, |total, count| {
            total.checked_add(*count)
        });
        prop_assert_eq!(
            total_checks,
            Some(expected_checks),
            "the total guard calls must equal the number of matching DAG nodes"
        );
        prop_assert_eq!(
            result,
            expression.map(|node| {
                Ok::<_, ()>(if let ExprView::Function { name, args } = node.view()
                    && name.as_str() == "sin"
                    && args.len() == 1
                {
                    args.first().map(Expr::cos)
                } else {
                    None
                })
            }).expect("the reference transformation cannot fail"),
            "guarded reconstruction must preserve every accepted edge"
        );
    }
}

proptest! {
    #[test]
    fn shared_polynomial_edges_use_the_same_logical_replacement(
        include_second_base in any::<bool>(),
        include_constant in any::<bool>(),
    ) {
        let x = Expr::symbol("property_poly_x");
        let y = Expr::symbol("property_poly_y");
        let mut terms = vec![x.pow(2), x.pow(3)];
        if include_second_base {
            terms.extend([y.pow(2), y.pow(3)]);
        }
        let polynomial = Expr::sum(terms);
        prop_assert!(
            matches!(polynomial.view(), ExprView::Sum(_)),
            "two positive powers form a logical sum"
        );
        let additive = if include_constant {
            Expr::sum(vec![polynomial, Expr::number(11)])
        } else {
            polynomial
        };
        let context = Context::new();
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the custom name is unbound");
        let input = pair.call(vec![additive.clone(), additive]).expect("pair accepts two arguments");
        let capture = Pattern::capture("property_poly_argument");
        let rule = RewriteRule::new(capture.clone(), capture.cos())
            .expect("the generated polynomial replacement is bound");
        let expected = input.map(|node| Ok::<_, ()>(Some(node.cos())))
            .expect("the reference transformation cannot fail");
        prop_assert_eq!(
            rule.apply_all(&input, None, None),
            Ok(expected),
            "shared polynomial edges must use one logical replacement"
        );
    }
}

proptest! {
    #[test]
    fn generated_polynomial_projection_preserves_order_and_arity(
        include_second_base in any::<bool>(),
        include_constant in any::<bool>(),
    ) {
        let x = Expr::symbol("property_projection_x");
        let y = Expr::symbol("property_projection_y");
        let x_squared = x.pow(2);
        let x_cubed = x.pow(3);
        let y_squared = y.pow(2);
        let y_cubed = y.pow(3);
        let mut terms = vec![x_squared.clone(), x_cubed.clone()];
        if include_second_base {
            terms.extend([y_squared.clone(), y_cubed.clone()]);
        }
        let polynomial = Expr::sum(terms);
        prop_assert!(
            matches!(polynomial.view(), ExprView::Sum(_)),
            "repeated positive powers form a logical sum"
        );

        let additive = if include_constant {
            Expr::sum(vec![polynomial, Expr::number(11)])
        } else {
            polynomial
        };
        let logical: Vec<_> = additive.children().collect();
        let mut expected = Vec::new();
        if include_constant {
            expected.push(Expr::number(11));
        }
        expected.extend([x_squared, x_cubed]);
        if include_second_base {
            expected.extend([y_squared, y_cubed]);
        }
        prop_assert_eq!(
            &logical,
            &expected,
            "logical polynomial projection must retain canonical order"
        );

        let first = Pattern::capture("property_projection_0");
        let pattern_terms = logical
            .iter()
            .enumerate()
            .map(|(index, _)| Pattern::capture(format!("property_projection_{index}")))
            .collect();
        let rule = RewriteRule::new(Pattern::sum(pattern_terms), first)
            .expect("the generated projected captures are bound");
        prop_assert_eq!(
            rule.apply_all(&additive, None, None),
            Ok(logical.first().expect("the projection has terms").clone()),
            "a complete logical pattern must match both Poly and outer Sum storage"
        );
    }
}

proptest! {
    #[test]
    fn generated_builtin_patterns_preserve_registered_custom_calls(
        choice in 0_u8..=2,
    ) {
        let context = Context::new();
        let local = context.register_function("custom_sin", UserFunction::new(1..=1)).expect("the custom name is unbound");
        let host = context.register_function("host", UserFunction::new(2..=2)).expect("the custom name is unbound");
        let argument = match choice {
            0 => Expr::symbol("property_identity_x"),
            1 => Expr::symbol("property_identity_y"),
            _ => Expr::symbol("property_identity_z"),
        };
        let local_call = local.call(vec![argument.clone()]).expect("the custom function is unary");
        let input = host.call(vec![argument.sin(), local_call.clone()]).expect("the host accepts two arguments");
        let capture = Pattern::capture("property_builtin_argument");
        let rule = RewriteRule::new(
            capture.sin(),
            capture.cos(),
        )
        .expect("the generated builtin replacement is bound");
        prop_assert_eq!(
            rule.apply_all(&input, None, None),
            Ok(host.call(vec![argument.cos(), local_call]).expect("the host accepts two arguments")),
            "builtin matching leaves registered custom function calls untouched"
        );
    }
}
