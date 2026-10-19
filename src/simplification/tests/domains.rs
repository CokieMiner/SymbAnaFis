//! Explicit poles, branch counterexamples, and literal representation boundaries.

use alloc::vec;

use num_anafis::{Number, RationalType};

use crate::{Context, Expr, ExprView, SimplificationSequence, SimplificationStep};

use super::simplify;

#[test]
fn unknown_poles_and_principal_branches_are_preserved() {
    let x = Expr::symbol("guarded_sequence_x");
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
    let x = Expr::symbol("proved_sequence_x");
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
    let x = Expr::symbol("literal_sequence_x");
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
    let context = Context::new();
    let function = context
        .register_function("custom_sin", crate::UserFunction::new(1..=1))
        .expect("custom function identity");
    let x = context.symb("x").expect("symbol");
    let input = function.call(vec![x.to_expr()]).expect("unary call").pow(2) + x.cos().pow(2);
    let sequence = SimplificationSequence::new().then(SimplificationStep::Trigonometric);
    assert_eq!(simplify(&sequence, &input), input);
}

#[test]
fn native_approximate_coefficients_stay_under_the_core_division_policy() {
    let x = Expr::symbol("float_coefficient_x");
    let y = Expr::symbol("float_coefficient_y");
    let approximate = (Expr::number(3.0) * &x) / (Expr::number(3.0) * &y);
    assert_eq!(
        simplify(&SimplificationSequence::compact(), &approximate),
        approximate
    );
}

#[test]
fn positive_and_nonzero_bases_authorize_general_power_combination() {
    let x = Expr::symbol("power_base_x");
    let a = Expr::symbol("power_exponent_a");
    let b = Expr::symbol("power_exponent_b");
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
    let base = Expr::symbol("bounded_exponent_base").exp();
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
