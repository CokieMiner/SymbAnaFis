//! Generated arithmetic, shared-register, and batch comparisons.

use core::ops::{Add, Div, Mul};

use alloc::{vec, vec::Vec};

use num_anafis::{Number, RationalType};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{Context, Evaluator, Expr};

proptest! {
    #[test]
    #[expect(clippy::arithmetic_side_effects, reason = "generated coordinates are at most thirty and coefficients at most six, so each integer intermediate fits i64")]
    fn sparse_polynomials_and_quotients_preserve_exact_values(
        x_value in -30_i32..=30,
        y_value in -30_i32..=30,
        a in -6_i32..=6,
        b in -6_i32..=6,
        c in -6_i32..=6,
    ) {
        let context = Context::new();
        let x = context.symb("x").expect("available identity");
        let y = context.symb("y").expect("available identity");
        let numerator = Expr::sum(vec![Mul::mul(a, x.pow(2)), Mul::mul(Mul::mul(b, x), y), Mul::mul(c, y.pow(2))]);
        let expression = Div::div(numerator, Add::add(y.pow(2), 1));
        let square_x = i64::from(x_value).pow(2);
        let square_y = i64::from(y_value).pow(2);
        let expected_numerator = i64::from(a) * square_x + i64::from(b) * i64::from(x_value) * i64::from(y_value) + i64::from(c) * square_y;
        let expected = Number::from_rational(RationalType::new(expected_numerator, square_y.checked_add(1).expect("small positive denominator")));
        let mut exact = Evaluator::<Number>::compile(&expression, &[x, y]).expect("closed expression");
        let actual = exact.evaluate(&[Number::from_int(i64::from(x_value)), Number::from_int(i64::from(y_value))]).expect("complete row");
        prop_assert_eq!(&actual, &expected);
        let mut native = Evaluator::<f64>::compile(&expression, &[x, y]).expect("real literals");
        prop_assert_eq!(native.evaluate(&[f64::from(x_value), f64::from(y_value)]).expect("complete row").to_bits(), expected.to_f64().to_bits());
    }

    #[test]
    #[expect(clippy::suboptimal_flops, reason = "the independent scalar oracle preserves the expression's unfused multiply-add operations")]
    fn shared_transcendental_nodes_survive_register_reuse(
        left in -0.8_f64..=0.8,
        right in 0.1_f64..=0.9,
        weights in proptest::collection::vec(-4_i32..=4, 1..30),
    ) {
        let context = Context::new();
        let x = context.symb("x").expect("available identity");
        let y = context.symb("y").expect("available identity");
        let shared = Add::add(x.sin(), y.cos());
        let terms: Vec<_> = weights.iter().enumerate().map(|(index, weight)| {
            Add::add(Mul::mul(shared.clone().pow(2), *weight), Expr::number(i64::try_from(index).expect("short generator"))).sin()
        }).collect();
        let expression = Expr::sum(terms);
        let mut evaluator = Evaluator::<f64>::compile(&expression, &[x, y]).expect("closed real expression");
        let baseline = evaluator.evaluate(&[left, right]).expect("complete row");
        let changed = evaluator.evaluate(&[right, left]).expect("complete row");
        let repeated = evaluator.evaluate(&[left, right]).expect("complete row");
        prop_assert_eq!(baseline.to_bits(), repeated.to_bits());
        prop_assert!(baseline.is_finite() && changed.is_finite());
        let value = left.sin() + right.cos();
        let expected: f64 = weights.iter().enumerate().map(|(index, weight)| {
            (value.powi(2) * f64::from(*weight) + f64::from(u32::try_from(index).expect("short generator"))).sin()
        }).sum();
        prop_assert!((baseline - expected).abs() < 1.0e-11);
        prop_assert!(evaluator.workspace_size() <= evaluator.instruction_count().saturating_add(10));
    }

    #[test]
    fn scalar_batches_equal_individual_rows(
        xs in proptest::collection::vec(-10.0_f64..=10.0, 0..100),
        offset in -4.0_f64..=4.0,
    ) {
        let context = Context::new();
        let x = context.symb("x").expect("available identity");
        let expression = Add::add(x.pow(2), offset).sqrt();
        let mut evaluator = Evaluator::<f64>::compile(&expression, &[x]).expect("real expression");
        let expected: Vec<_> = xs.iter().map(|value| evaluator.evaluate(&[*value]).expect("complete row").to_bits()).collect();
        let mut output = vec![0.0; xs.len()];
        evaluator.evaluate_batch(&[&xs], &mut output).expect("equal column lengths");
        prop_assert_eq!(output.into_iter().map(f64::to_bits).collect::<Vec<_>>(), expected);
    }
}
