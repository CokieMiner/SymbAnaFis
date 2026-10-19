//! Commutative subset matching, transactional captures, and bounded search.

use alloc::{format, vec, vec::Vec};

use proptest::{arbitrary::any, prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, UserFunction},
};

use super::super::{Expr, ExprView, Pattern, RewriteError, RewriteRule};

fn expression(sum: bool, operands: Vec<Expr>) -> Expr {
    if sum {
        Expr::sum(operands)
    } else {
        Expr::product(operands)
    }
}

fn template(sum: bool, operands: Vec<Pattern>) -> Pattern {
    if sum {
        Pattern::sum(operands)
    } else {
        Pattern::product(operands)
    }
}

proptest! {
    #[test]
    fn literal_patterns_find_subsets_in_any_template_order(
        sum in any::<bool>(),
        remainder_count in 1_usize..=10,
        reverse in any::<bool>(),
        coefficient in 2_i64..=20,
    ) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let u = context.symb("u")?.to_expr();
        let mut patterns = vec![Pattern::exact(&x), Pattern::exact(&y)];
        if reverse { patterns.reverse(); }
        let rule = RewriteRule::new(template(sum, patterns), Pattern::exact(&u))
            .expect("literal replacements have no captures");
        let mut remaining: Vec<_> = (0..remainder_count)
            .map(|index| context.symb(&format!("r{index}")).map(|symbol| symbol.to_expr()))
            .collect::<Result<_, _>>()?;
        remaining.push(Expr::number(coefficient));
        let mut original = remaining.clone();
        original.extend([x, y]);
        remaining.push(u);
        let input = expression(sum, original);
        let expected = expression(sum, remaining);
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(expected.clone()));
        prop_assert_eq!(rule.apply_all(&input, Some(4096), None), Ok(expected));
    }

    #[test]
    fn capture_groups_replace_every_disjoint_occurrence_and_preserve_the_tail(
        sum in any::<bool>(),
        operand_count in 3_usize..=12,
    ) {
        let context = Context::new();
        let operands: Vec<_> = (0..operand_count)
            .map(|index| context.symb(&format!("x{index}")).map(|symbol| symbol.to_expr()))
            .collect::<Result<_, _>>()?;
        let input = expression(sum, operands);
        let canonical = if sum {
            input.children().collect()
        } else if let ExprView::Product(factors) = input.view() {
            factors.to_vec()
        } else {
            return Err(proptest::test_runner::TestCaseError::fail("the generated input is a product"));
        };
        let a = Pattern::capture("a", None);
        let b = Pattern::capture("b", None);
        let packed = context.register_function("packed", UserFunction::new(2..=2)).expect("the custom name is unbound");
        let rule = RewriteRule::new(template(sum, vec![a.clone(), b.clone()]), Pattern::apply(packed.clone(), vec![a, b]).expect("the function accepts two patterns"))
            .expect("the replacement captures are bound");
        let (groups, remainder) = canonical.as_chunks::<2>();
        let mut expected = Vec::new();
        for [first, second] in groups {
            expected.push(packed.call(vec![first.clone(), second.clone()]).expect("the function accepts two arguments"));
        }
        expected.extend(remainder.iter().cloned());
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(expression(sum, expected)));
    }

    #[test]
    fn failed_assignments_roll_back_captures_before_trying_another_group(remainder_count in 0_usize..=8) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let capture = Pattern::capture("a", None);
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("the name is unbound");
        let g = context.register_function("g", UserFunction::new(1..=1)).expect("the name is unbound");
        let h = context.register_function("h", UserFunction::new(1..=1)).expect("the name is unbound");
        let rule = RewriteRule::new(
            Pattern::sum(vec![Pattern::apply(f.clone(), vec![capture.clone()]).expect("f is unary"), Pattern::apply(g.clone(), vec![capture.clone()]).expect("g is unary")]),
            Pattern::apply(h.clone(), vec![capture]).expect("h is unary"),
        ).expect("the replacement capture is bound");
        let remaining: Vec<_> = (0..remainder_count)
            .map(|index| context.symb(&format!("z{index}")).expect("symbol allocation").to_expr())
            .chain([f.call(vec![x]).expect("f is unary")])
            .collect();
        let mut original = remaining.clone();
        original.extend([f.call(vec![y.clone()]).expect("f is unary"), g.call(vec![y.clone()]).expect("g is unary")]);
        let mut expected = remaining;
        expected.push(h.call(vec![y]).expect("h is unary"));
        prop_assert_eq!(rule.apply_all(&Expr::sum(original), None, None), Ok(Expr::sum(expected)));
    }

    #[test]
    fn nested_associative_templates_match_permutations_but_require_all_operands(sum in any::<bool>()) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let z = context.symb("z")?.to_expr();
        let replacement = context.symb("replacement")?.to_expr();
        let host = context.register_function("host", UserFunction::new(1..=1)).expect("the custom name is unbound");
        let rule = RewriteRule::new(
            Pattern::apply(host.clone(), vec![template(sum, vec![Pattern::exact(&y), Pattern::exact(&x)])]).expect("the host is unary"),
            Pattern::exact(&replacement),
        ).expect("literal replacements have no captures");
        prop_assert_eq!(rule.apply_all(&host.call(vec![expression(sum, vec![x.clone(), y.clone()])]).expect("the host is unary"), None, None), Ok(replacement));
        let larger = host.call(vec![expression(sum, vec![x, y, z])]).expect("the host is unary");
        prop_assert_eq!(rule.apply_all(&larger, None, None).expect("unrestricted matching cannot exhaust a limit"), larger);
    }

    #[test]
    fn guarded_matching_can_search_another_assignment(sum in any::<bool>(), chosen_index in 0_usize..=3) {
        let context = Context::new();
        let values: Vec<_> = (0..4)
            .map(|index| context.symb(&format!("x{index}")).map(|symbol| symbol.to_expr()))
            .collect::<Result<_, _>>()?;
        let chosen = values.get(chosen_index).expect("the generated index is bounded").clone();
        let input = expression(sum, values);
        let canonical = if sum {
            input.children().collect()
        } else if let ExprView::Product(factors) = input.view() {
            factors.to_vec()
        } else {
            return Err(proptest::test_runner::TestCaseError::fail("the input is a product"));
        };
        let other = canonical.iter().find(|value| *value != &chosen)
            .expect("four distinct operands include an alternative");
        let replacement = context.symb("replacement")?.to_expr();
        let rule = RewriteRule::new(
            template(sum, vec![Pattern::capture("a", None), Pattern::capture("b", None)]), Pattern::exact(&replacement),
        ).expect("literal replacements have no captures");
        let mut expected: Vec<_> = canonical.iter()
            .filter(|value| *value != &chosen && *value != other)
            .cloned().collect();
        expected.push(replacement);
        prop_assert_eq!(
            rule.apply_all(&input, None, Some(&mut |captures: &HashMap<&str, Expr>| captures.get("a") == Some(&chosen))),
            Ok(expression(sum, expected)),
        );
    }

    #[test]
    fn common_factor_rewrites_preserve_bounded_integer_evaluation(
        a_value in -10_i64..=10,
        b_value in -10_i64..=10,
        c_value in -10_i64..=10,
        constant in -10_i64..=10,
    ) {
        let context = Context::new();
        let a = context.symb("a")?;
        let b = context.symb("b")?;
        let c = context.symb("c")?;
        let pa = Pattern::capture("a", None);
        let pb = Pattern::capture("b", None);
        let pc = Pattern::capture("c", None);
        let rule = RewriteRule::new(
            Pattern::sum(vec![Pattern::product(vec![pa.clone(), pb.clone()]), Pattern::product(vec![pa.clone(), pc.clone()])]),
            Pattern::product(vec![pa, Pattern::sum(vec![pb, pc])]),
        ).expect("all replacement captures are bound");
        let input = Expr::sum(vec![Expr::product(vec![a.to_expr(), b.to_expr()]), Expr::product(vec![a.to_expr(), c.to_expr()]), Expr::number(constant)]);
        let bindings = HashMap::from([(a, Expr::number(a_value)), (b, Expr::number(b_value)), (c, Expr::number(c_value))]);
        let result = rule.apply_all(&input, None, None)
            .expect("unrestricted matching cannot exhaust a limit");
        prop_assert_eq!(result.substitute_symbols(&bindings), input.substitute_symbols(&bindings));
    }

    #[test]
    fn sum_patterns_match_projected_polynomial_groups(low in 2_i64..=4, high in 6_i64..=8, other in 10_i64..=12) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let u = context.symb("u")?.to_expr();
        let input = Expr::sum(vec![x.pow(low), x.pow(high), x.pow(other)]);
        prop_assert!(matches!(input.view(), ExprView::Sum(_)));
        let rule = RewriteRule::new(Pattern::sum(vec![Pattern::exact(x.pow(high)), Pattern::exact(x.pow(low))]), Pattern::exact(&u))
            .expect("literal replacements have no captures");
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(Expr::sum(vec![u, x.pow(other)])));
    }
}

#[test]
fn bounded_search_reports_exhaustion_and_ordered_matches_need_no_candidates() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation").to_expr();
    let y = context.symb("y").expect("symbol allocation").to_expr();
    let z = context.symb("z").expect("symbol allocation").to_expr();
    let u = context.symb("u").expect("symbol allocation").to_expr();
    let rule = RewriteRule::new(
        Pattern::sum(vec![Pattern::exact(&x), Pattern::exact(&y)]),
        Pattern::exact(&u),
    )
    .expect("literal replacements have no captures");
    let input = Expr::sum(vec![x.clone(), y.clone(), z.clone()]);
    assert_eq!(
        rule.apply_all(&input, Some(0), None),
        Err(RewriteError::MatchLimitExceeded { limit: 0 })
    );
    assert_eq!(
        rule.apply_all(&input, None, None),
        Ok(Expr::sum(vec![u.clone(), z]))
    );
    assert_eq!(rule.apply_all(&Expr::sum(vec![x, y]), Some(0), None), Ok(u));
}
