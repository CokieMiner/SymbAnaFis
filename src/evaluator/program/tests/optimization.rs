//! Floating reassociation, sparse degrees, and backend operation contracts.

use core::ops::{Add, Div, Mul};

use alloc::vec::Vec;

use num_anafis::Number;
use proptest::{prop_assert, proptest};

use crate::{Context, Evaluator, Expr};

use super::super::Operation;

proptest! {
    #[test]
    #[expect(clippy::suboptimal_flops, reason = "the independent Horner oracle keeps separate floating operations")]
    fn dense_and_sparse_polynomials_agree_with_a_scalar_oracle(
        coefficients in proptest::collection::vec(-6_i32..=6, 2..34),
        input in -0.9_f64..=0.9,
        stride in 1_u32..=7,
    ) {
        let context = Context::new();
        let x = context.symb("x").expect("available identity");
        let terms: Vec<_> = coefficients.iter().enumerate().map(|(index, coefficient)| {
            let degree = u32::try_from(index).expect("at most thirty-three coefficients").checked_mul(stride).expect("small degree");
            Mul::mul(*coefficient, x.pow(i64::from(degree)))
        }).collect();
        let expression = Expr::sum(terms);
        let mut evaluator = Evaluator::<f64>::compile(&expression, &[x]).expect("real polynomial");
        let coordinate = input.powi(i32::try_from(stride).expect("small stride"));
        let expected = coefficients.iter().rev().fold(0.0, |value, coefficient| value * coordinate + f64::from(*coefficient));
        let actual = evaluator.evaluate(&[input]).expect("complete row");
        prop_assert!((actual - expected).abs() < 2.0e-12 * expected.abs().max(1.0));
    }
}

#[test]
fn sparse_large_degrees_do_not_allocate_by_degree() {
    let context = Context::new();
    let x = context.symb("x").expect("available identity");
    let expression = Add::add(
        x.pow(i64::from(u32::MAX)),
        x.pow(i64::from(u32::MAX.saturating_sub(2))),
    );
    let mut evaluator = Evaluator::<f64>::compile(&expression, &[x]).expect("sparse polynomial");
    assert!(evaluator.instruction_count() < 70);
    assert!(evaluator.workspace_size() < 10);
    for (input, expected) in [(0.0, 0.0), (1.0, 2.0), (-1.0, -2.0)] {
        assert_eq!(
            evaluator.evaluate(&[input]).expect("complete row"),
            expected
        );
    }
}

#[test]
fn repeated_denominators_share_only_native_reciprocals() {
    let context = Context::new();
    let x = context.symb("x").expect("available identity");
    let y = context.symb("y").expect("available identity");
    let z = context.symb("z").expect("available identity");
    let expression = Add::add(Div::div(x, y), Div::div(z, y));
    let mut native = Evaluator::<f64>::compile(&expression, &[x, y, z]).expect("real inputs");
    assert_eq!(
        native
            .program
            .instructions
            .iter()
            .filter(|instruction| matches!(instruction.operation, Operation::Divide(_)))
            .count(),
        1
    );
    assert_eq!(
        native.evaluate(&[2.0, 4.0, 6.0]).expect("complete row"),
        2.0
    );
    assert!(
        native
            .evaluate(&[2.0, 0.0, 6.0])
            .expect("complete row")
            .is_infinite()
    );
    assert!(
        native
            .evaluate(&[0.0, 0.0, 0.0])
            .expect("complete row")
            .is_nan()
    );
    let general = Evaluator::<Number>::compile(&expression, &[x, y, z]).expect("general inputs");
    assert_eq!(
        general
            .program
            .instructions
            .iter()
            .filter(|instruction| matches!(instruction.operation, Operation::Divide(_)))
            .count(),
        2
    );
}

#[test]
fn approximate_coefficients_follow_number_arithmetic_in_generic_programs() {
    let context = Context::new();
    let x = context.symb("x").expect("available identity");
    let expression = Add::add(Mul::mul(Expr::number(1.5), x.pow(2)), x.pow(3));
    let mut evaluator = Evaluator::<Number>::compile(&expression, &[x]).expect("scalar polynomial");
    let expected = Expr::number(14.0);
    assert_eq!(
        Expr::number(
            evaluator
                .evaluate(&[Number::from_int(2)])
                .expect("complete row")
        ),
        expected
    );
    assert_eq!(
        Expr::number(
            expression
                .evaluate([(x, Number::from_int(2))])
                .expect("bound input")
        ),
        expected
    );
}

#[test]
fn fused_native_programs_and_generic_programs_have_explicit_rounding_contracts() {
    let context = Context::new();
    let x = context.symb("x").expect("available identity");
    let y = context.symb("y").expect("available identity");
    let z = context.symb("z").expect("available identity");
    let expression = Add::add(Mul::mul(x, y), z);
    let left = 1.000_000_007_450_580_6;
    let right = 0.999_999_992_549_419_4;
    let direct = expression
        .evaluate([(x, left), (y, right), (z, -1.0)])
        .expect("bound inputs");
    assert_eq!(direct, 0.0);
    let mut native = Evaluator::<f64>::compile(&expression, &[x, y, z]).expect("real inputs");
    let fused = native
        .program
        .instructions
        .iter()
        .any(|instruction| matches!(instruction.operation, Operation::Fused(_)));
    let actual = native.evaluate(&[left, right, -1.0]).expect("complete row");
    if fused {
        assert_eq!(actual, -5.551_115_123_125_783e-17);
    } else {
        assert_eq!(actual, direct);
    }
    let mut general =
        Evaluator::<Number>::compile(&expression, &[x, y, z]).expect("general inputs");
    assert!(
        !general
            .program
            .instructions
            .iter()
            .any(|instruction| matches!(instruction.operation, Operation::Fused(_)))
    );
    assert_eq!(
        Expr::number(
            general
                .evaluate(&[
                    Number::from_float(left),
                    Number::from_float(right),
                    Number::from_float(-1.0)
                ])
                .expect("complete row")
        ),
        Expr::number(0.0)
    );
}
