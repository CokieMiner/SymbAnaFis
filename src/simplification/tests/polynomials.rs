//! Exact multivariate GCD reductions, retained poles, and bounded arithmetic.

use core::ops::{Add, Div, Mul};

use proptest::{prelude::ProptestConfig, prop_assert_eq, proptest};

use crate::{
    DomainSafety, Expr, Property, SimplificationSequence, SimplificationStep, SimplifyError, Truth,
};

use super::simplify;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn expanded_multivariate_common_factors_cancel_exactly(
        a in 1_i64..=4, b in 1_i64..=4, c in -3_i64..=3,
        d in 1_i64..=4, e in 1_i64..=4,
    ) {
        let x = Expr::symbol("generated_gcd_x")?;
        let y = Expr::symbol("generated_gcd_y")?;
        let common = Add::add(Add::add(Mul::mul(&x, a), Mul::mul(&y, b)), c);
        let numerator = Add::add(&x, d);
        let denominator = Add::add(y.pow(2), e);
        let expansion = SimplificationSequence::expanded();
        let top = simplify(&expansion, &Mul::mul(&common, &numerator));
        let bottom = simplify(&expansion, &Mul::mul(&common, &denominator));
        let program = SimplificationSequence::new().then(SimplificationStep::Algebraic).then(SimplificationStep::Factor)
            .domain_safety(DomainSafety::AssumptionDependent).polynomial_limit(1_000_000);
        prop_assert_eq!(simplify(&program, &Div::div(top, bottom)), Div::div(numerator, denominator));
    }
}

#[test]
fn polynomial_gcd_finds_hidden_univariate_and_multivariate_factors() {
    let x = Expr::symbol("gcd_x").expect("symbol");
    let y = Expr::symbol("gcd_y").expect("symbol");
    let expansion = SimplificationSequence::expanded();
    let reduction = SimplificationSequence::new()
        .then(SimplificationStep::Algebraic)
        .domain_safety(DomainSafety::AssumptionDependent);
    for common in [
        Add::add(Add::add(x.pow(3), &x), 1),
        Add::add(Add::add(&x, &y), 1),
        Add::add(x.pow(2), y.pow(2)),
    ] {
        let numerator = Add::add(&x, 2);
        let denominator = Add::add(y.pow(2), 3);
        let top = simplify(&expansion, &Mul::mul(&common, &numerator));
        let bottom = simplify(&expansion, &Mul::mul(&common, &denominator));
        assert_eq!(
            simplify(&reduction, &Div::div(top, bottom)),
            Div::div(numerator, denominator)
        );
    }
}

#[test]
fn polynomial_gcd_uses_monic_univariate_remainders() {
    let x = Expr::symbol("univariate_gcd_x").expect("symbol");
    let common = Add::add(Add::add(x.pow(4), Mul::mul(x.pow(2), 3)), 1);
    let numerator = Add::add(&x, 2);
    let denominator = Add::add(x.pow(3), 5);
    let expansion = SimplificationSequence::expanded();
    let top = simplify(&expansion, &Mul::mul(&common, &numerator));
    let bottom = simplify(&expansion, &Mul::mul(&common, &denominator));
    let reduction = SimplificationSequence::new()
        .then(SimplificationStep::Algebraic)
        .domain_safety(DomainSafety::AssumptionDependent);
    assert_eq!(
        simplify(&reduction, &Div::div(top, bottom)),
        Div::div(numerator, denominator)
    );
}

#[test]
fn polynomial_gcd_extracts_common_monomials_before_remainders() {
    let x = Expr::symbol("monomial_gcd_x").expect("symbol");
    let y = Expr::symbol("monomial_gcd_y").expect("symbol");
    let common = Mul::mul(&x, &y);
    let numerator = Add::add(&x, 1);
    let denominator = Add::add(&y, 2);
    let expansion = SimplificationSequence::expanded();
    let top = simplify(&expansion, &Mul::mul(&common, &numerator));
    let bottom = simplify(&expansion, &Mul::mul(&common, &denominator));
    let reduction = SimplificationSequence::new()
        .then(SimplificationStep::Algebraic)
        .domain_safety(DomainSafety::AssumptionDependent);
    assert_eq!(
        simplify(&reduction, &Div::div(top, bottom)),
        Div::div(numerator, denominator)
    );
}

#[test]
fn preserving_polynomial_cancellation_requires_a_nonzero_removed_factor() {
    let x = Expr::symbol("guarded_gcd_x").expect("symbol");
    let z = Expr::symbol("guarded_gcd_z").expect("symbol");
    let preserving = SimplificationSequence::new().then(SimplificationStep::Algebraic);
    let source = Div::div(Add::add(x.pow(2), -1), Add::add(&x, -1));
    assert_eq!(simplify(&preserving, &source), source);
    let assumed = preserving
        .clone()
        .domain_safety(DomainSafety::AssumptionDependent);
    assert_eq!(simplify(&assumed, &source), Add::add(&x, 1));
    assert_ne!(
        source
            .substitute("guarded_gcd_x", &Expr::number(1))
            .properties()
            .get(Property::Finite),
        Truth::True
    );
    let common = z.exp();
    let top = Add::add(Mul::mul(&common, &x), &common);
    let bottom = Add::add(Mul::mul(&common, &x), Mul::mul(&common, 2));
    assert_eq!(
        simplify(&preserving, &Div::div(top, bottom)),
        Div::div(Add::add(&x, 1), Add::add(&x, 2))
    );
}

#[test]
fn polynomial_budget_failure_is_transactional() {
    let x = Expr::symbol("bounded_gcd_x").expect("symbol");
    let input = Div::div(Add::add(x.pow(3), -1), Add::add(x.pow(2), -1));
    let sequence = SimplificationSequence::new()
        .then(SimplificationStep::Algebraic)
        .domain_safety(DomainSafety::AssumptionDependent)
        .polynomial_limit(0);
    assert!(matches!(
        sequence.simplify(&input),
        Err(SimplifyError::PolynomialLimitExceeded { limit: 0 })
    ));
    assert_eq!(
        input,
        Div::div(Add::add(x.pow(3), -1), Add::add(x.pow(2), -1))
    );
}

#[test]
fn polynomial_gcd_supports_heap_backed_monomials() {
    let first = Expr::symbol("wide_gcd_x").expect("symbol");
    let second = Expr::symbol("wide_gcd_y").expect("symbol");
    let third = Expr::symbol("wide_gcd_z").expect("symbol");
    let fourth = Expr::symbol("wide_gcd_u").expect("symbol");
    let fifth = Expr::symbol("wide_gcd_v").expect("symbol");
    let common = Expr::sum(alloc::vec![
        first.clone(),
        second,
        third,
        fourth,
        fifth.clone(),
        Expr::number(1)
    ]);
    let numerator = Add::add(&first, 2);
    let denominator = Add::add(&fifth, 3);
    let expansion = SimplificationSequence::expanded();
    let input = Div::div(
        simplify(&expansion, &Mul::mul(&common, &numerator)),
        simplify(&expansion, &Mul::mul(&common, &denominator)),
    );
    let program = SimplificationSequence::new()
        .then(SimplificationStep::Algebraic)
        .domain_safety(DomainSafety::AssumptionDependent)
        .polynomial_limit(1_000_000);
    assert_eq!(simplify(&program, &input), Div::div(numerator, denominator));
}

#[test]
fn polynomial_gcd_extracts_repeated_multivariate_content() {
    let x = Expr::symbol("content_gcd_x").expect("symbol");
    let y = Expr::symbol("content_gcd_y").expect("symbol");
    let z = Expr::symbol("content_gcd_z").expect("symbol");
    let factor = Add::add(Add::add(Add::add(&x, &y), &z), 1);
    let common = Mul::mul(Add::add(&y, 1), Mul::mul(&factor, &factor));
    let numerator = Add::add(&x, 2);
    let denominator = Add::add(y.pow(2), 3);
    let expansion = SimplificationSequence::expanded();
    let top = simplify(&expansion, &Mul::mul(&common, &numerator));
    let bottom = simplify(&expansion, &Mul::mul(&common, &denominator));
    let reduction = SimplificationSequence::new()
        .then(SimplificationStep::Algebraic)
        .domain_safety(DomainSafety::AssumptionDependent)
        .polynomial_limit(1_000_000);
    assert_eq!(
        simplify(&reduction, &Div::div(top, bottom)),
        Div::div(numerator, denominator)
    );
}

#[test]
fn polynomial_gcd_retains_coprime_high_term_quotients() {
    let x = Expr::symbol("coprime_gcd_x").expect("symbol");
    let y = Expr::symbol("coprime_gcd_y").expect("symbol");
    let z = Expr::symbol("coprime_gcd_z").expect("symbol");
    let top = Expr::sum(alloc::vec![
        Mul::mul(x.pow(5), &y),
        Mul::mul(x.pow(3), z.pow(2)),
        Mul::mul(&y, &z),
        Expr::number(3),
    ]);
    let bottom = Expr::sum(alloc::vec![
        Mul::mul(x.pow(4), &z),
        Mul::mul(y.pow(3), &z),
        Mul::mul(&x, y.pow(2)),
        Expr::number(2),
    ]);
    let input = Div::div(top, bottom);
    let reduction = SimplificationSequence::new()
        .then(SimplificationStep::Algebraic)
        .domain_safety(DomainSafety::AssumptionDependent)
        .polynomial_limit(1_000_000);
    assert_eq!(simplify(&reduction, &input), input);
}

#[test]
fn polynomial_gcd_cancels_large_coefficients_and_degrees() {
    let x = Expr::symbol("modular_gcd_x").expect("symbol");
    let y = Expr::symbol("modular_gcd_y").expect("symbol");
    let z = Expr::symbol("modular_gcd_z").expect("symbol");
    let expansion = SimplificationSequence::expanded();
    let reduction = SimplificationSequence::new()
        .then(SimplificationStep::Algebraic)
        .domain_safety(DomainSafety::AssumptionDependent);
    // Coefficients near 10^18 exceed every rational remainder sequence.
    let linear = Add::add(Add::add(Mul::mul(&x, 1_000_003), Mul::mul(&y, 999_983)), 7);
    let cases = [
        (
            linear.pow(2),
            Mul::mul(&linear, Add::add(&x, 1)),
            Add::add(&y, 1),
        ),
        (
            Expr::sum(alloc::vec![
                x.pow(3),
                y.pow(3),
                z.pow(3),
                Mul::mul(Mul::mul(&x, &y), &z),
                Expr::number(1),
            ]),
            Add::add(Add::add(&x, Mul::mul(&y, 2)), Mul::mul(&z, 3)),
            Add::add(Add::add(Mul::mul(&x, &y), z.pow(2)), 1).pow(2),
        ),
    ];
    for (common, numerator, denominator) in cases {
        let top = simplify(&expansion, &Mul::mul(&common, &numerator));
        let bottom = simplify(&expansion, &Mul::mul(&common, &denominator));
        assert_eq!(
            simplify(&reduction, &Div::div(top, bottom)),
            Div::div(
                simplify(&expansion, &numerator),
                simplify(&expansion, &denominator)
            )
        );
    }
}
