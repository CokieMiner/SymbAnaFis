//! Floating reassociation, sparse degrees, dot products, and backend operation
//! contracts.

use core::{
    ops::{Add, Div, Mul},
    slice::from_ref,
};

use alloc::{format, vec, vec::Vec};

use num_anafis::Number;
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::FixedMap,
    core::{Context, Expr},
};

use super::{
    super::{Compilation, Fold, Operation, ProgramData, Source},
    support::Compiled,
};

proptest! {
    #[test]
    #[expect(clippy::suboptimal_flops, reason = "the independent Horner oracle keeps separate floating operations")]
    fn dense_and_sparse_polynomials_agree_with_a_scalar_oracle(
        coefficients in proptest::collection::vec(-6_i32..=6, 2..34),
        input in -0.9_f64..=0.9,
        stride in 1_u32..=7,
    ) {
        let context = Context::new()?;
        let x = context.symb("x").expect("available identity");
        let terms: Vec<_> = coefficients.iter().enumerate().map(|(index, coefficient)| {
            let degree = u32::try_from(index).expect("at most thirty-three coefficients").checked_mul(stride).expect("small degree");
            Mul::mul(*coefficient, x.pow(i64::from(degree)))
        }).collect();
        let expression = Expr::sum(terms);
        let mut evaluator = Compiled::<f64>::compile(&expression, &[x]).expect("real polynomial");
        let coordinate = input.powi(i32::try_from(stride).expect("small stride"));
        let expected = coefficients.iter().rev().fold(0.0, |value, coefficient| value * coordinate + f64::from(*coefficient));
        let actual = evaluator.evaluate(&[input]).expect("complete row");
        prop_assert!((actual - expected).abs() < 2.0e-12 * expected.abs().max(1.0));
    }

    #[test]
    fn dot_products_round_each_product_and_sum_in_stored_order(
        factors in proptest::collection::vec((-1.0e3_f64..=1.0e3, -1.0e3_f64..=1.0e3), 8..=24),
    ) {
        let context = Context::new()?;
        let symbols: Vec<_> = (0..factors.len().saturating_mul(2))
            .map(|index| context.symb(&format!("dot_{index}")).expect("available identity"))
            .collect();
        let (left, right) = symbols.split_at(factors.len());
        let expression = Expr::sum(left.iter().zip(right).map(|(a, b)| Mul::mul(*a, *b)).collect());
        let inputs: Vec<f64> = factors.iter().map(|(a, _)| *a).chain(factors.iter().map(|(_, b)| *b)).collect();
        let mut native = Compiled::<f64>::compile(&expression, &symbols).expect("real inputs");
        let dot = native.program().data.instructions.iter().any(|instruction| {
            matches!(instruction.operation, Operation::Fold(Fold::Dot, _))
        });
        prop_assert!(dot);
        let direct = expression
            .evaluate(symbols.iter().copied().zip(inputs.iter().copied()))
            .expect("bound inputs");
        let actual = native.evaluate(&inputs).expect("complete row");
        prop_assert_eq!(actual.to_bits(), direct.to_bits());
        let columns: Vec<_> = inputs.iter().map(from_ref).collect();
        let mut output = [0.0];
        native.evaluate_batch(&columns, &mut output).expect("one row");
        let [batched] = output;
        prop_assert_eq!(batched.to_bits(), direct.to_bits());
    }
}

#[test]
fn sparse_large_degrees_do_not_allocate_by_degree() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let expression = Add::add(
        x.pow(i64::from(u32::MAX)),
        x.pow(i64::from(u32::MAX.saturating_sub(2))),
    );
    let mut evaluator = Compiled::<f64>::compile(&expression, &[x]).expect("sparse polynomial");
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
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let y = context.symb("y").expect("available identity");
    let z = context.symb("z").expect("available identity");
    let expression = Add::add(Div::div(x, y), Div::div(z, y));
    let mut native = Compiled::<f64>::compile(&expression, &[x, y, z]).expect("real inputs");
    assert_eq!(
        native
            .program()
            .data
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
    let general = Compiled::<Number>::compile(&expression, &[x, y, z]).expect("general inputs");
    assert_eq!(
        general
            .program()
            .data
            .instructions
            .iter()
            .filter(|instruction| matches!(instruction.operation, Operation::Divide(_)))
            .count(),
        2
    );
}

#[test]
fn approximate_coefficients_follow_number_arithmetic_in_generic_programs() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let expression = Add::add(Mul::mul(Expr::number(1.5), x.pow(2)), x.pow(3));
    let mut evaluator = Compiled::<Number>::compile(&expression, &[x]).expect("scalar polynomial");
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
    let context = Context::new().expect("context");
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
    let mut native = Compiled::<f64>::compile(&expression, &[x, y, z]).expect("real inputs");
    let fused = native
        .program()
        .data
        .instructions
        .iter()
        .any(|instruction| matches!(instruction.operation, Operation::Fused(_)));
    let actual = native.evaluate(&[left, right, -1.0]).expect("complete row");
    if fused {
        assert_eq!(actual, -5.551_115_123_125_783e-17);
    } else {
        assert_eq!(actual, direct);
    }
    let mut general = Compiled::<Number>::compile(&expression, &[x, y, z]).expect("general inputs");
    assert!(
        !general
            .program()
            .data
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

#[test]
fn dot_products_keep_their_unit_when_paired_functions_renumber_sources() {
    let context = Context::new().expect("context");
    let symbols: Vec<_> = (0..13)
        .map(|index| {
            context
                .symb(&format!("paired_dot_{index}"))
                .expect("available identity")
        })
        .collect();
    let (factors, rest) = symbols.split_at(12);
    let (left, right) = factors.split_at(6);
    let angle = Expr::from(rest.first().copied().expect("one angle"));
    let mut terms: Vec<_> = left
        .iter()
        .zip(right)
        .map(|(a, b)| Mul::mul(*a, *b))
        .collect();
    terms.extend([angle.sin(), angle.cos()]);
    let expression = Expr::sum(terms);
    let inputs: Vec<f64> = (1..=13).map(|index| 0.25 * f64::from(index)).collect();
    let mut native = Compiled::<f64>::compile(&expression, &symbols).expect("real inputs");
    let instructions = &native.program().data.instructions;
    assert!(
        instructions
            .iter()
            .any(|instruction| matches!(instruction.operation, Operation::Fold(Fold::Dot, _)))
    );
    assert!(
        instructions
            .iter()
            .any(|instruction| matches!(instruction.operation, Operation::Pair(..)))
    );
    let direct = expression
        .evaluate(symbols.iter().copied().zip(inputs.iter().copied()))
        .expect("bound inputs");
    let actual = native.evaluate(&inputs).expect("complete row");
    assert!((actual - direct).abs() <= 1.0e-12 * direct.abs());
}

#[test]
fn unreachable_readers_leave_a_product_single_use() {
    // Sources: the unit, x, y, -1, p = x*y, an unread q = p*y, and the result
    // -1*p. Only the result reads p, so the negation folds into it.
    let mut compilation = Compilation {
        program: ProgramData {
            instructions: Vec::new(),
            operands: Vec::new(),
            parameters: Vec::new(),
            result: 6,
            constants: Vec::new(),
            register_count: 0,
            initial: 0.0,
        },
        sources: Vec::new(),
        constants: vec![-1.0],
        parameters: FixedMap::default(),
        powers: FixedMap::default(),
        denominators: FixedMap::default(),
        unit: None,
    };
    let unit = compilation.coefficient(None).expect("native unit");
    assert_eq!(unit, 0);
    compilation.sources.extend([
        Source::Parameter(0),
        Source::Parameter(1),
        Source::Constant(0),
        Source::Computed(Operation::Multiply([1, 2])),
        Source::Computed(Operation::Multiply([4, 2])),
        Source::Computed(Operation::Multiply([3, 4])),
    ]);
    compilation.optimize_arithmetic().expect("native unit");
    assert!(matches!(
        compilation.sources.get(6),
        Some(Source::Computed(Operation::NegatedMultiply([1, 2])))
    ));
}
