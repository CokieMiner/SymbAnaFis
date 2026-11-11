//! Assumed domains, explicit poles, literal boundaries, and caller-owned rewrites.

use core::ops::{Add, Div, Mul, Neg};

use alloc::{sync::Arc, vec};

use num_anafis::{Number, RationalType};

use crate::{
    Context, DomainSafety, Expr, ExprView, Pattern, RewriteRule, SimplificationSequence,
    SimplificationStep, UserFunction,
};

use super::simplify;

#[test]
fn unknown_poles_and_principal_branches_are_preserved() {
    let x = Expr::symbol("guarded_sequence_x").expect("symbol");
    let one = Expr::number(1);
    let sequence = SimplificationSequence::compact();
    for input in vec![
        &x / &x,
        Expr::number(0) / &x,
        x.pow(0),
        x.exp().ln(),
        x.sin().asin(),
        x.cos().acos(),
        x.tan().atan(),
        x.pow(2).sqrt(),
        x.pow(3).cbrt(),
        x.abs().pow(2),
        x.ln().exp(),
        &one / x.tan(),
        &one / x.tanh(),
        &one / (&one / &x),
    ] {
        assert_eq!(simplify(&sequence, &input), input);
    }
    let input = (&x * x.sin()) / &x;
    let result = simplify(&sequence, &input).substitute("guarded_sequence_x", &Expr::number(0));
    assert!(result.fold(false, |found, node| found
        || matches!(node.view(), ExprView::Div(_, denominator) if denominator.is_zero_num())));
}

#[test]
fn proven_real_positive_and_nonzero_inputs_enable_guarded_identities() {
    let x = Expr::symbol("proved_sequence_x").expect("symbol");
    let sequence = SimplificationSequence::compact();
    let real = x.abs();
    let positive = &real + 1;
    let nonzero = x.exp();
    assert_eq!(simplify(&sequence, &(&nonzero / &nonzero)), Expr::number(1));
    assert_eq!(simplify(&sequence, &positive.ln().exp()), positive);
    assert_eq!(simplify(&sequence, &real.exp().ln()), real);
    assert_eq!(simplify(&sequence, &real.pow(2).sqrt()), real);
    assert_eq!(
        simplify(&sequence, &x.ln().exp().pow(0)),
        x.ln().exp().pow(0)
    );
}

#[test]
fn float_payloads_and_approximate_exponents_do_not_acquire_exact_representations() {
    let x = Expr::symbol("literal_sequence_x").expect("symbol");
    let sequence = SimplificationSequence::compact();
    for value in [
        0.0,
        -0.0,
        f64::from_bits(1),
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0x7ff8_0000_0000_1234),
    ] {
        let input = Expr::number(value);
        assert_eq!(simplify(&sequence, &input), input);
    }
    let approximate = x.abs().pow(2.0).sqrt();
    assert_eq!(simplify(&sequence, &approximate), approximate);
    let approximate_zero = x.exp().pow(-0.0);
    assert_eq!(simplify(&sequence, &approximate_zero), approximate_zero);
    let signed_zero = Expr::number(-0.0) / x.exp();
    assert_eq!(simplify(&sequence, &signed_zero), signed_zero);
    let half = Expr::number(Number::from_rational(RationalType::new(1, 2)));
    let nested = x.pow(&half).pow(&half);
    assert_eq!(simplify(&sequence, &nested), nested);
}

#[test]
fn user_function_names_do_not_acquire_builtin_identities() {
    let context = Context::new().expect("context");
    let function = context
        .register_function("custom_sin", UserFunction::new(1..=1))
        .expect("custom function identity");
    let x = context.symb("x").expect("symbol");
    let input = function.call(vec![x.to_expr()]).expect("unary call").pow(2) + x.cos().pow(2);
    let sequence = SimplificationSequence::new().then(SimplificationStep::Trigonometric);
    assert_eq!(simplify(&sequence, &input), input);
}

#[test]
fn native_approximate_coefficients_stay_under_the_core_division_policy() {
    let x = Expr::symbol("float_coefficient_x").expect("symbol");
    let y = Expr::symbol("float_coefficient_y").expect("symbol");
    let approximate = (Expr::number(3.0) * &x) / (Expr::number(3.0) * &y);
    assert_eq!(
        simplify(&SimplificationSequence::compact(), &approximate),
        approximate
    );
}

#[test]
fn positive_and_nonzero_bases_authorize_general_power_combination() {
    let x = Expr::symbol("power_base_x").expect("symbol");
    let a = Expr::symbol("power_exponent_a").expect("symbol");
    let b = Expr::symbol("power_exponent_b").expect("symbol");
    let base = x.exp();
    let sequence = SimplificationSequence::new().then(SimplificationStep::Algebraic);
    assert_eq!(
        simplify(&sequence, &(base.pow(&a) * base.pow(&b))),
        base.pow(&a + &b)
    );
    assert_eq!(
        simplify(&sequence, &(&base * base.pow(&a))),
        base.pow(&a + 1)
    );
    assert_eq!(
        simplify(&sequence, &(base.pow(&a) / base.pow(&b))),
        base.pow(&a - &b)
    );
    assert_eq!(simplify(&sequence, &base.pow(&a).pow(2)), base.pow(2 * &a));
    let unknown = x.pow(&a).pow(&b);
    assert_eq!(simplify(&sequence, &unknown), unknown);
}

#[test]
fn exponent_capacity_and_approximate_nesting_are_retained() {
    let base = Expr::symbol("bounded_exponent_base").expect("symbol").exp();
    let sequence = SimplificationSequence::new().then(SimplificationStep::Algebraic);
    let overflowing_sum = base.pow(i64::MAX) * &base;
    assert_eq!(simplify(&sequence, &overflowing_sum), overflowing_sum);
    let overflowing_product = base.pow(i64::MAX).pow(2);
    assert_eq!(
        simplify(&sequence, &overflowing_product),
        overflowing_product
    );
    let approximate = base.pow(0.3).pow(2);
    assert_eq!(simplify(&sequence, &approximate), approximate);
}

#[test]
fn preserving_and_assumed_modes_select_distinct_builtin_contracts() {
    let x = Expr::symbol("mode_x").expect("symbol");
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
    let x = Expr::symbol("even_fractional_power_x").expect("symbol");
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
    let x = Expr::symbol("caller_mode_x").expect("symbol");
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
    let x = Expr::symbol("common_power_x").expect("symbol");
    let y = Expr::symbol("common_power_y").expect("symbol");
    let a = Expr::symbol("common_power_a").expect("symbol");
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
fn quotients_cancel_common_power_counts_of_admitted_bases() {
    let x = Expr::symbol("cancel_count_x").expect("symbol");
    let y = Expr::symbol("cancel_count_y").expect("symbol");
    let z = Expr::symbol("cancel_count_z").expect("symbol");
    let preserving = SimplificationSequence::new().then(SimplificationStep::Algebraic);
    let assumed = preserving
        .clone()
        .domain_safety(DomainSafety::AssumptionDependent);
    // Counts cancel partially on both sides; the literal and the unmatched
    // factors are kept as they are.
    let quotient = Div::div(
        Expr::product(vec![Expr::number(3), x.pow(5), y.clone(), z.sin()]),
        Expr::product(vec![x.pow(2), y.pow(3), z.clone()]),
    );
    assert_eq!(
        simplify(&assumed, &quotient),
        Div::div(
            Expr::product(vec![Expr::number(3), x.pow(3), z.sin()]),
            Mul::mul(y.pow(2), &z),
        )
    );
    // Preserving mode cancels only the base proved nonzero.
    let positive = Add::add(x.abs(), 1);
    let guarded = Div::div(Mul::mul(positive.pow(4), &y), Mul::mul(&positive, y.pow(2)));
    assert_eq!(
        simplify(&preserving, &guarded),
        Div::div(Mul::mul(positive.pow(3), &y), y.pow(2))
    );
    assert_eq!(simplify(&assumed, &guarded), Div::div(positive.pow(3), &y));
    // Without a common base nothing changes.
    let coprime = Div::div(Mul::mul(x.pow(2), z.sin()), Mul::mul(&y, &z));
    assert_eq!(simplify(&assumed, &coprime), coprime);
}

#[test]
fn mixed_fraction_addition_and_general_logarithms_are_available() {
    let x = Expr::symbol("additional_rule_x").expect("symbol");
    let y = Expr::symbol("additional_rule_y").expect("symbol");
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
