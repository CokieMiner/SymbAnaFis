//! Normalized arithmetic templates and exact mixed operands.

use alloc::{format, vec, vec::Vec};

use num_anafis::Number;
use proptest::{prelude::prop, prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, UserFunction},
};

use super::super::{Expr, ExprView, Pattern, PatternNode, RewriteRule};

proptest! {
    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bounded coefficients exercise normalized pattern and symbol arithmetic"
    )]
    fn named_captures_collect_coefficients_and_keep_different_names_separate(
        first in -8_i32..=8,
        second in -8_i32..=8,
        value in -8_i64..=8,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let a = Pattern::capture("a", None);
        let other_a = Pattern::capture("a", None);
        let pattern = first * &a + second * &other_a;
        let rule = RewriteRule::new(pattern, Pattern::exact(u))
            .expect("literal replacements require no captures");
        let input = first * x + second * x;
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(u.to_expr()));

        let b = Pattern::capture("b", None);
        let args_function = context.register_function("args", UserFunction::new(2..=2))
            .expect("the custom name is unbound");
        let body = first * &a + second * &b;
        let instantiate = RewriteRule::new(
            Pattern::apply(args_function.clone(), vec![a, b]).expect("the function accepts two patterns"), body,
        ).expect("both replacement captures occur in the function pattern");
        let args = args_function.call(vec![x.to_expr(), Expr::number(value)]).expect("the function accepts two arguments");
        prop_assert_eq!(instantiate.apply_all(&args, None, None), Ok(first * x + Expr::number(second) * value));
    }

    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "positive exponents total at most ten and symbolic division preserves unknown zero bases"
    )]
    fn repeated_positive_powers_collect_without_cancelling_unknown_bases(
        first in 1_i64..=5,
        second in 1_i64..=5,
        value in -8_i64..=8,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let a = Pattern::capture("a", None);
        let rule = RewriteRule::new(a.pow(first) * a.pow(second), Pattern::exact(u))
            .expect("literal replacements require no captures");
        prop_assert_eq!(rule.apply_all(&x.pow(first + second), None, None), Ok(u.to_expr()));

        let function = context.register_function("value", UserFunction::new(1..=1))
            .expect("the custom name is unbound");
        let retain = RewriteRule::new(
            Pattern::apply(function.clone(), vec![a.clone()]).expect("the function is unary"), &a / &a,
        ).expect("the division capture occurs in the function pattern");
        let result = retain.apply_all(&function.call(vec![x.to_expr()]).expect("the function is unary"), None, None)
            .expect("unrestricted matching cannot exhaust a limit");
        prop_assert!(matches!(result.view(), ExprView::Div(_, _)));
        prop_assert_eq!(result.substitute_symbol(x, &Expr::number(value)), Expr::number(value) / Expr::number(value));
    }

    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bounded nonzero divisors and integer inputs exercise symbolic operator construction"
    )]
    fn arithmetic_replacements_agree_with_expression_construction(
        coefficient in -8_i32..=8,
        divisor in 1_i32..=8,
        exponent in 1_i64..=5,
        x_value in -8_i64..=8,
        y_value in -8_i64..=8,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let a = Pattern::capture("a", None);
        let b = Pattern::capture("b", None);
        let replacement = (coefficient * &a - &b) / divisor + (-&a).pow(exponent);
        let args_function = context.register_function("args", UserFunction::new(2..=2))
            .expect("the custom name is unbound");
        let rule = RewriteRule::new(
            Pattern::apply(args_function.clone(), vec![a, b]).expect("the function accepts two patterns"), replacement,
        ).expect("all arithmetic captures occur in the function pattern");
        let input = args_function.call(vec![x.to_expr(), y.to_expr()]).expect("the function accepts two arguments");
        let result = rule.apply_all(&input, None, None)
            .expect("unrestricted matching cannot exhaust a limit");
        let expected = (coefficient * x - y) / divisor + (-x).pow(exponent);
        prop_assert_eq!(&result, &expected);
        let values = HashMap::from([(x, Expr::number(x_value)), (y, Expr::number(y_value))]);
        prop_assert_eq!(result.substitute_symbols(&values), expected.substitute_symbols(&values));
    }

    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "generated symbolic sums exercise operator flattening and exact subset replacement"
    )]
    fn vector_and_operator_groups_flatten_and_preserve_subset_matching(
        remainder in prop::collection::vec(1_i32..=9, 0..=5),
        reverse in prop::bool::ANY,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let z = context.symb("z")?;
        let u = context.symb("u")?;
        let a = Pattern::capture("a", None);
        let mut terms = vec![Pattern::exact(x), Pattern::exact(y), Pattern::exact(z)];
        if reverse { terms.reverse(); }
        let vector = Pattern::sum(terms);
        let chained = x + (y + Pattern::exact(z));
        let rest: Vec<_> = remainder.iter().enumerate()
            .map(|(index, coefficient)| context.symb(&format!("r{index}")).map(|symbol| *coefficient * symbol))
            .collect::<Result<_, _>>()?;
        let input = Expr::sum(rest.iter().cloned().chain([x.to_expr(), y.to_expr(), z.to_expr()]).collect());
        let expected = Expr::sum(rest.into_iter().chain([u.to_expr()]).collect());
        for template in [vector, chained] {
            let rule = RewriteRule::new(template, Pattern::exact(u))
                .expect("literal replacements require no captures");
            prop_assert_eq!(rule.apply_all(&input, None, None), Ok(expected.clone()));
        }
        let rule = RewriteRule::new(u + &a, 2 * &a)
            .expect("the replacement capture occurs in the input sum");
        prop_assert_eq!(rule.apply_all(&(u + x), None, None), Ok(2 * x));
    }

    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "bounded powers and coefficients exercise logical polynomial pattern construction"
    )]
    fn polynomial_templates_keep_captures_in_logical_monomials(
        low in 2_i64..=4,
        high in 6_i64..=8,
        coefficient in 1_i32..=5,
        constant in -5_i32..=5,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let a = Pattern::capture("a", None);
        let pattern = a.pow(low) + coefficient * a.pow(high);
        prop_assert!(matches!(pattern.node(), PatternNode::Sum(_)));
        let rule = RewriteRule::new(pattern, a)
            .expect("the polynomial base capture occurs in its monomials");
        let input = x.pow(low) + coefficient * x.pow(high) + constant;
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(x + constant));
        let literal = RewriteRule::new(
            Pattern::sum(vec![Pattern::exact(x.pow(low)), Pattern::exact(x.pow(high))]),
            Pattern::exact(u),
        ).expect("literal replacements require no captures");
        prop_assert_eq!(literal.apply_all(&(x.pow(low) + x.pow(high) + constant), None, None), Ok(u + constant));
    }

    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        clippy::op_ref,
        reason = "borrowed and owned operand forms are intentionally compared through overloaded symbolic arithmetic"
    )]
    fn mixed_concrete_operands_preserve_context_identity(value in 1_i32..=9) {
        let first = Context::new();
        let second = Context::new();
        let x = first.symb("x")?;
        let other_x = second.symb("x")?;
        let y = first.symb("y")?;
        let a = Pattern::capture("a", None);
        let x_expr = x.to_expr();
        let literal = Number::from(value);
        let replacements = [
            x + &a, &x + &a, &x_expr + &a,
            x_expr.clone() + a.clone(),
            &a + x, &a + &x_expr,
            &literal * &a, literal.clone() * &a,
            i64::from(value) * &a, f64::from(value) * &a,
        ];
        let expected_results = [
            x + y, x + y, x + y, x + y, x + y, x + y,
            Expr::number(literal.clone()) * y,
            Expr::number(literal) * y,
            Expr::number(i64::from(value)) * y,
            Expr::number(f64::from(value)) * y,
        ];
        let function = first.register_function("f", UserFunction::new(1..=1))
            .expect("the custom name is unbound");
        for (replacement, expected) in replacements.into_iter().zip(expected_results) {
            let rule = RewriteRule::new(Pattern::apply(function.clone(), vec![a.clone()]).expect("the function is unary"), replacement)
                .expect("the mixed replacement capture occurs in the input function");
            prop_assert_eq!(rule.apply_all(&function.call(vec![y.to_expr()]).expect("the function is unary"), None, None), Ok(expected));
        }
        let rule = RewriteRule::new(x + &a, a)
            .expect("the replacement capture occurs in the input sum");
        let input = other_x + y;
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(input));
        prop_assert_eq!(rule.apply_all(&(x + y), None, None), Ok(y.to_expr()));
    }

    #[test]
    fn distinct_argument_captures_may_be_omitted_from_the_result(
        x_value in -8_i64..=8,
        y_value in -8_i64..=8,
    ) {
        let a = Pattern::capture("a", None);
        let context = Context::new();
        let function = context.register_function("first", UserFunction::new(2..=2))
            .expect("the custom name is unbound");
        let rule = RewriteRule::new(
            Pattern::apply(function.clone(), vec![a.clone(), Pattern::capture("b", None)]).expect("the function accepts two patterns"), a,
        ).expect("the retained argument is captured");
        let x = Expr::number(x_value);
        let y = Expr::number(y_value);
        prop_assert_eq!(rule.apply_all(&function.call(vec![x.clone(), y]).expect("the function accepts two arguments"), None, None), Ok(x));
    }
}

#[test]
fn undefined_zero_bases_and_empty_groups_preserve_expression_contracts() {
    let a = Pattern::capture("a", None);
    assert!(matches!((&a - &a).node(), PatternNode::Product(_)));
    assert!(matches!((&a / &a).node(), PatternNode::Div(_, _)));
    assert!(matches!(a.pow(0).node(), PatternNode::Pow(_, _)));
    assert!(
        matches!(Pattern::sum(Vec::new()).node(), PatternNode::Exact(value) if value.is_zero_num())
    );
    assert!(
        matches!(Pattern::product(Vec::new()).node(), PatternNode::Exact(value) if value.is_one_num())
    );
}
