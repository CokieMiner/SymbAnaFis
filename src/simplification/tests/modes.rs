//! Builtin rule tags, assumed domains, and caller-owned custom transformations.

use core::ops::{Add, Div, Mul, Neg};

use alloc::sync::Arc;

use crate::{
    DomainSafety, Expr, ExprView, Pattern, RewriteRule, SimplificationSequence, SimplificationStep,
};

use super::simplify;

#[test]
fn preserving_and_assumed_modes_select_distinct_builtin_contracts() {
    assert!(DomainSafety::Preserving.permits(DomainSafety::Preserving));
    assert!(!DomainSafety::Preserving.permits(DomainSafety::AssumptionDependent));
    assert!(DomainSafety::AssumptionDependent.permits(DomainSafety::Preserving));
    let x = Expr::symbol("mode_x");
    let preserving = SimplificationSequence::compact();
    let assumed = preserving
        .clone()
        .domain_safety(DomainSafety::AssumptionDependent);
    let cases = [
        (Div::div(&x, &x), Expr::number(1)),
        (Div::div(Expr::number(0), &x), Expr::number(0)),
        (x.pow(0), Expr::number(1)),
        (x.exp().ln(), x.clone()),
        (x.ln().exp(), x.clone()),
        (x.sin().asin(), x.clone()),
        (x.cos().acos(), x.clone()),
        (x.tan().atan(), x.clone()),
        (x.pow(2).sqrt(), x.abs()),
        (x.pow(3).cbrt(), x.clone()),
        (x.log(&x), Expr::number(1)),
    ];
    for (input, expected) in cases {
        assert_eq!(simplify(&preserving, &input), input);
        assert_eq!(simplify(&assumed, &input), expected);
    }
    assert_eq!(simplify(&preserving, &Div::div(&x, &x)), Div::div(&x, &x));
}

#[test]
fn real_even_powers_keep_absolute_values_under_fractional_outer_powers() {
    let x = Expr::symbol("even_fractional_power_x");
    let quarter = Div::div(Expr::number(1), 4);
    let preserving = SimplificationSequence::new().then(SimplificationStep::Algebraic);
    let input = x.pow(4).pow(&quarter);
    assert_eq!(simplify(&preserving, &input), input);
    let assumed = preserving
        .clone()
        .domain_safety(DomainSafety::AssumptionDependent);
    assert_eq!(simplify(&assumed, &input), x.abs());
    let real = Neg::neg(Add::add(x.abs(), 1));
    assert_eq!(
        simplify(&preserving, &real.pow(4).pow(&quarter)),
        Add::add(x.abs(), 1)
    );
}

#[test]
fn known_false_facts_are_never_promoted_to_assumptions() {
    let program =
        SimplificationSequence::compact().domain_safety(DomainSafety::AssumptionDependent);
    let zero = Expr::number(0);
    let pole = Div::div(Expr::number(1), &zero);
    assert_eq!(
        simplify(&program, &Div::div(&zero, &zero)),
        Div::div(&zero, &zero)
    );
    assert_eq!(
        simplify(&program, &Mul::mul(&zero, &pole)),
        Mul::mul(&zero, &pole)
    );
    assert_eq!(simplify(&program, &pole.pow(0)), pole.pow(0));
}

#[test]
fn custom_rules_and_maps_keep_the_callers_contract_in_both_modes() {
    let x = Expr::symbol("caller_mode_x");
    let rewrite =
        RewriteRule::new(Pattern::exact(x.sin()), Pattern::exact(x.cos())).expect("exact rule");
    for safety in [DomainSafety::Preserving, DomainSafety::AssumptionDependent] {
        let program = SimplificationSequence::new()
            .domain_safety(safety)
            .then(rewrite.clone());
        assert_eq!(simplify(&program, &x.sin()), x.cos());
        let original = x.clone();
        let mapped =
            SimplificationSequence::new()
                .domain_safety(safety)
                .then(SimplificationStep::Map(Arc::new(move |node| {
                    (node == &original).then(|| Expr::number(0))
                })));
        assert_eq!(simplify(&mapped, &x), Expr::number(0));
    }
}

#[test]
fn common_exponents_and_symbolic_negative_powers_have_guarded_forms() {
    let x = Expr::symbol("common_power_x");
    let y = Expr::symbol("common_power_y");
    let a = Expr::symbol("common_power_a");
    let preserving = SimplificationSequence::new().then(SimplificationStep::Algebraic);
    let assumed = preserving
        .clone()
        .domain_safety(DomainSafety::AssumptionDependent);
    let input = Mul::mul(x.pow(&a), y.pow(&a));
    assert_eq!(simplify(&preserving, &input), input);
    assert_eq!(simplify(&assumed, &input), Mul::mul(&x, &y).pow(&a));
    let quotient = Div::div(x.pow(&a), y.pow(&a));
    assert_eq!(simplify(&preserving, &quotient), quotient);
    assert_eq!(simplify(&assumed, &quotient), Div::div(&x, &y).pow(&a));
    let positive_x = Add::add(x.abs(), 1);
    let positive_y = Add::add(y.abs(), 1);
    assert_eq!(
        simplify(
            &preserving,
            &Mul::mul(positive_x.pow(&a), positive_y.pow(&a))
        ),
        Mul::mul(&positive_x, &positive_y).pow(&a)
    );
    let inverse = x.exp().pow(Neg::neg(&a));
    assert_eq!(
        simplify(&preserving, &inverse),
        Div::div(Expr::number(1), x.exp().pow(&a))
    );
}

#[test]
fn mixed_fraction_addition_and_general_logarithms_are_available() {
    let x = Expr::symbol("additional_rule_x");
    let y = Expr::symbol("additional_rule_y");
    let fractions = SimplificationSequence::new()
        .then(SimplificationStep::Algebraic)
        .score_with(|expr| usize::from(matches!(expr.view(), ExprView::Sum(_))));
    let input = Add::add(&x, Div::div(Expr::number(1), &y));
    assert_eq!(
        simplify(&fractions, &input),
        Div::div(Add::add(Mul::mul(&x, &y), 1), &y)
    );
    let logs = SimplificationSequence::compact().score_with(|expr| {
        expr.fold(0_usize, |count, node| {
            count.saturating_add(usize::from(matches!(node.view(), ExprView::Pow(..))
                || matches!(node.view(), ExprView::Function { name, .. } if matches!(name.as_str(), "sqrt" | "cbrt"))))
        })
    });
    let real = x.abs();
    let positive = Add::add(&real, 1);
    assert_eq!(
        simplify(&logs, &positive.pow(3).log(7)),
        Mul::mul(positive.log(7), 3)
    );
    assert_eq!(simplify(&logs, &real.pow(2).ln()), Mul::mul(real.ln(), 2));
    assert_eq!(simplify(&logs, &x.sqrt().ln()), Div::div(x.ln(), 2));
}
