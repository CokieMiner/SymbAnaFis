//! Rule-set passes: the attempt budget, rule order, and error propagation.

use core::ops::{Add, Mul, Sub};

use alloc::{format, vec::Vec};

use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, Expr},
};

use super::super::{Pattern, RewriteError, RewriteRule, RuleSet};

/// A rule `a + b -> a` that matches any two additive operands.
fn absorbing_sum() -> RewriteRule {
    RewriteRule::new(
        Add::add(Pattern::capture("a", None), Pattern::capture("b", None)),
        Pattern::capture("a", None),
    )
    .expect("bound captures")
}

/// The triple-angle identity `3 sin(a) - 4 sin^3(a) -> sin(3a)`, whose pattern
/// is a sum of two products with exact coefficients.
fn triple_angle() -> RewriteRule {
    let a = Pattern::capture("a", None);
    RewriteRule::new(
        Sub::sub(Mul::mul(a.sin(), 3), Mul::mul(a.sin().pow(3), 4)),
        Mul::mul(&a, 3).sin(),
    )
    .expect("bound capture")
}

/// A sum of `count` distinct symbols, so no two operands are like terms.
fn wide_sum(count: usize) -> (Context, Expr) {
    let context = Context::new().expect("context");
    let terms = (0..count)
        .map(|index| {
            context
                .symb(&format!("wide_sum_{index}"))
                .expect("symbol")
                .to_expr()
        })
        .collect::<Vec<_>>();
    let expression = Expr::sum(terms);
    (context, expression)
}

proptest! {
    #[test]
    fn a_sum_wider_than_the_attempt_bound_is_still_rewritten(count in 2_usize..=64) {
        let (_context, expression) = wide_sum(count);
        let rules = RuleSet::new([absorbing_sum()]);
        // One attempt per operand is unavoidable, so a bound below the operand
        // count must not disable the rule.
        let rewritten = rules.apply(&expression, 1)?;
        prop_assert!(rewritten.node_count() < expression.node_count());
    }

    #[test]
    fn a_nested_pattern_scans_a_wide_sum_within_its_linear_allowance(count in 2_usize..=64) {
        let (context, expression) = wide_sum(count);
        let y = context.symb("nested_scan_y")?;
        let widened = Add::add(expression, y.sin());
        let rules = RuleSet::new([triple_angle()]);
        // Every operand expands the nested product group, so the allowance
        // covers the template's node count, not only the operand count.
        prop_assert_eq!(rules.apply(&widened, 1), Ok(widened));
    }

    #[test]
    fn a_zero_bound_disables_the_commutative_search(count in 2_usize..=8) {
        let (context, _expression) = wide_sum(count);
        let x = context.symb("zero_bound_x")?;
        let rule = RewriteRule::new(
            Pattern::capture("a", None).pow(2),
            Pattern::exact(0),
        )
        .expect("bound capture");
        let rules = RuleSet::new([rule]);
        // `x^4` needs the counted stage, which is a search, so a zero bound
        // exhausts it instead of matching.
        prop_assert_eq!(
            rules.apply(&x.pow(4), 0),
            Err(RewriteError::MatchLimitExceeded { limit: 0 })
        );
    }
}

#[test]
fn the_first_matching_rule_wins_and_later_rules_see_its_output() {
    let context = Context::new().expect("context");
    let x = context.symb("rule_order_x").expect("symbol");
    let y = context.symb("rule_order_y").expect("symbol");
    let first = RewriteRule::new(Pattern::exact(x), Pattern::exact(y)).expect("exact rule");
    let second = RewriteRule::new(Pattern::exact(y), Pattern::exact(2)).expect("exact rule");
    let rules = RuleSet::new([first, second]);
    // The pass is bottom-up, so the replacement of `x` is not revisited.
    assert_eq!(rules.apply(&x.to_expr(), usize::MAX), Ok(y.to_expr()));
    assert_eq!(rules.apply(&Add::add(x, 1), usize::MAX), Ok(Add::add(y, 1)));
}

#[test]
fn a_rule_whose_builtin_is_absent_below_the_node_is_skipped() {
    let context = Context::new().expect("context");
    let x = context.symb("rule_gate_x").expect("symbol");
    let capture = Pattern::capture("a", None);
    let rule = RewriteRule::new(capture.sin(), Pattern::exact(0)).expect("bound capture");
    let rules = RuleSet::new([rule]);
    // No `sin` occurs below `x + 1`, so the rule contributes nothing.
    assert_eq!(rules.apply(&Add::add(x, 1), usize::MAX), Ok(Add::add(x, 1)));
    assert_eq!(rules.apply(&x.sin(), usize::MAX), Ok(Expr::number(0)));
}

#[test]
fn an_empty_rule_set_returns_its_input() {
    let context = Context::new().expect("context");
    let x = context.symb("empty_rules_x").expect("symbol");
    let expression = Mul::mul(x, 2);
    let rules = RuleSet::new(Vec::new());
    assert_eq!(rules.apply(&expression, 0), Ok(expression));
}

#[test]
fn a_guard_rejection_leaves_the_node_unchanged() {
    let context = Context::new().expect("context");
    let x = context.symb("rule_guard_x").expect("symbol");
    let capture = Pattern::capture("a", None);
    let rule = RewriteRule::new(capture.pow(2), Pattern::exact(0)).expect("bound capture");
    let rules = RuleSet::new([rule]);
    let square = x.pow(2);
    // `RuleSet::apply` accepts every structural match, so the rule fires.
    assert_eq!(rules.apply(&square, usize::MAX), Ok(Expr::number(0)));
    let mut reject = |_: &HashMap<&str, Expr>| false;
    assert_eq!(
        RewriteRule::new(Pattern::capture("a", None).pow(2), Pattern::exact(0))
            .expect("bound capture")
            .apply_all(&square, usize::MAX, Some(&mut reject)),
        Ok(square)
    );
}

#[test]
fn a_capture_only_rule_keeps_the_logical_numbering_for_the_whole_set() {
    let context = Context::new().expect("context");
    let x = context.symb("mixed_numbering_x").expect("symbol");
    let capture = Pattern::capture("a", None);
    let builtin = RewriteRule::new(capture.sin(), Pattern::exact(0)).expect("bound capture");
    let plain = RewriteRule::new(Pattern::exact(x.pow(2)), Pattern::exact(1)).expect("exact rule");
    // `x^2 + 1` is compact storage over a symbol, so a set whose rules all call
    // a builtin numbers it as one leaf and the `x^2` term is never visited.
    let polynomial = Add::add(x.pow(2), 1);
    let builtin_only = RuleSet::new([builtin.clone()]);
    assert_eq!(
        builtin_only.apply(&polynomial, usize::MAX),
        Ok(polynomial.clone())
    );
    // The capture-only rule calls no builtin, so the set numbers logical nodes
    // and the term is visited even though the builtin rule cannot match it.
    let mixed = RuleSet::new([builtin, plain]);
    assert_eq!(mixed.apply(&polynomial, usize::MAX), Ok(Expr::number(2)));
    // The builtin rule still fires in the same set, at a node outside the
    // compact storage.
    let widened = Add::add(x.sin(), x.pow(2));
    assert_eq!(mixed.apply(&widened, usize::MAX), Ok(Expr::number(1)));
}
