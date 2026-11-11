//! Floating reassociation, sparse degrees, dot products, and backend operation
//! contracts.

use core::{
    ops::{Add, Div, Mul},
    slice::from_ref,
};

use alloc::{format, string::ToString, vec, vec::Vec};

use num_anafis::Number;
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::FixedMap,
    core::{Context, Expr},
};

use super::{
    super::{Combine, Compilation, Fold, Operation, ProgramData, Source},
    support::Compiled,
};

proptest! {
    #[test]
    #[cfg_attr(feature = "std", expect(clippy::suboptimal_flops, reason = "the independent Horner oracle keeps separate floating operations"))]
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
        let instructions = &native.program().data.instructions;
        let dot = instructions.iter().any(|instruction| {
            matches!(instruction.operation, Operation::Fold(Fold::Dot, _))
        });
        prop_assert!(dot);
        let fused = ProgramData::<f64>::fuse()
            || instructions
                .iter()
                .any(|instruction| matches!(instruction.operation, Operation::Fused(..)));
        let direct = expression
            .evaluate(symbols.iter().copied().zip(inputs.iter().copied()))
            .expect("bound inputs");
        let actual = native.evaluate(&inputs).expect("complete row");
        if fused {
            prop_assert!((actual - direct).abs() <= 1.0e-12 * direct.abs().max(1.0));
        } else {
            prop_assert_eq!(actual.to_bits(), direct.to_bits());
        }
        let columns: Vec<_> = inputs.iter().map(from_ref).collect();
        let mut output = [0.0];
        native.evaluate_batch(&columns, &mut output).expect("one row");
        let [batched] = output;
        prop_assert_eq!(batched.to_bits(), actual.to_bits());
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
        .any(|instruction| matches!(instruction.operation, Operation::Fused(..)));
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
            .any(|instruction| matches!(instruction.operation, Operation::Fused(..)))
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
        operands: Vec::new(),
        parameters: alloc::boxed::Box::new([]),
        results: vec![6],
        sources: Vec::new(),
        constants: vec![-1.0],
        parameter_map: FixedMap::default(),
        powers: FixedMap::default(),
        constant_reciprocals: FixedMap::default(),
        constant_bits: FixedMap::default(),
        exponent_constants: FixedMap::default(),
        denominators: FixedMap::default(),
        binary: FixedMap::default(),
        fused: FixedMap::default(),
        folds: FixedMap::default(),
        unit: None,
        initial: 0.0,
        trig_mask: 0,
    };
    let unit = compilation.coefficient(None).expect("native unit");
    assert_eq!(unit, 0);
    compilation.sources.extend([
        Source::Parameter(0),
        Source::Parameter(1),
        Source::Constant(0),
        Source::Computed(Operation::Combine(Combine::Multiply, [1, 2])),
        Source::Computed(Operation::Combine(Combine::Multiply, [4, 2])),
        Source::Computed(Operation::Combine(Combine::Multiply, [3, 4])),
    ]);
    compilation.optimize_arithmetic().expect("native unit");
    assert!(matches!(
        compilation.sources.get(6),
        Some(Source::Computed(Operation::NegatedMultiply([1, 2])))
    ));
}

#[test]
fn shared_linear_terms_constants_and_estrin_powers_avoid_duplicate_work() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let y = context.symb("y").expect("available identity");
    let z = context.symb("z").expect("available identity");

    // A shared linear product `2 * x` inside `cos(3 + 2 * x) + sin(2 * x)` stays shared.
    let linear_product = Mul::mul(2, x);
    let shared_linear = Add::add(
        Add::add(3, linear_product.clone()).cos(),
        linear_product.sin(),
    );
    let mut compiled_linear =
        Compiled::<f64>::compile(&shared_linear, &[x]).expect("shared linear expression");
    assert_eq!(
        compiled_linear
            .program()
            .data
            .instructions
            .iter()
            .filter(|instruction| {
                matches!(
                    instruction.operation,
                    Operation::Combine(Combine::Multiply, _) | Operation::Fused(..)
                )
            })
            .count(),
        1
    );
    assert_eq!(
        compiled_linear.evaluate(&[0.5]).expect("complete row"),
        4.0_f64.cos() + 1.0_f64.sin()
    );

    // `x^2 + x + 1` shares a single `1.0` constant register.
    let quadratic = Expr::sum(vec![x.pow(2), x.to_expr(), Expr::number(1)]);
    let compiled_quadratic = Compiled::<f64>::compile(&quadratic, &[x]).expect("quadratic");
    assert_eq!(compiled_quadratic.program().data.constants.len(), 1);

    // `1 / y + x / y` reuses `1 / y` directly without multiplying by `1.0`.
    let shared_reciprocal = Add::add(Div::div(Expr::number(1), y), Div::div(x, y));
    let mut compiled_reciprocal =
        Compiled::<f64>::compile(&shared_reciprocal, &[x, y]).expect("shared reciprocal");
    assert_eq!(compiled_reciprocal.program().data.constants.len(), 1);
    assert_eq!(
        compiled_reciprocal
            .evaluate(&[3.0, 2.0])
            .expect("complete row"),
        2.0
    );

    // Repeated constant divisors share one reciprocal constant; generic powers share one exponent constant.
    let constant_divisors = Expr::sum(vec![Div::div(x, 2), Div::div(y, 2), Div::div(z, 2)]);
    let compiled_divisors =
        Compiled::<f64>::compile(&constant_divisors, &[x, y, z]).expect("constant divisors");
    assert_eq!(compiled_divisors.program().data.constants.len(), 1);

    let sum_of_squares = Expr::sum(vec![x.pow(2), y.pow(2), z.pow(2)]);
    let compiled_squares =
        Compiled::<Number>::compile(&sum_of_squares, &[x, y, z]).expect("generic squares");
    assert_eq!(compiled_squares.program().data.constants.len(), 1);

    // Estrin records `x^2` and `x^4` in `powers` so external uses share them.
    let estrin_plus_sin = Add::add(
        Expr::sum(vec![
            Expr::number(1),
            x.to_expr(),
            x.pow(2),
            x.pow(3),
            x.pow(4),
        ]),
        x.pow(4).sin(),
    );
    let compiled_estrin = Compiled::<f64>::compile(&estrin_plus_sin, &[x]).expect("estrin");
    let squarings = compiled_estrin
        .program()
        .data
        .instructions
        .iter()
        .filter(|instruction| {
            matches!(
                instruction.operation,
                Operation::Combine(Combine::Multiply, [left, right]) if left == right
            )
        })
        .count();
    assert_eq!(squarings, 2);

    // `z - x * y` never leaves an unfused `NegatedMultiply` followed by `Add`.
    let subtracted_product = Add::add(z, Mul::mul(-1, Mul::mul(x, y)));
    let mut compiled_sub =
        Compiled::<f64>::compile(&subtracted_product, &[x, y, z]).expect("subtracted product");
    assert!(
        compiled_sub
            .program()
            .data
            .instructions
            .iter()
            .all(|instruction| !matches!(instruction.operation, Operation::NegatedMultiply(_)))
    );
    assert_eq!(
        compiled_sub
            .evaluate(&[2.0, 3.0, 10.0])
            .expect("complete row"),
        4.0
    );
}

#[test]
fn difference_of_products_eliminates_negated_multiply() {
    let context = Context::new().expect("context");
    let w = context.symb("w").expect("available identity");
    let x = context.symb("x").expect("available identity");
    let y = context.symb("y").expect("available identity");
    let z = context.symb("z").expect("available identity");
    let diff_of_products = Add::add(Mul::mul(w, z), Mul::mul(-1, Mul::mul(x, y)));
    let mut compiled_diff =
        Compiled::<f64>::compile(&diff_of_products, &[w, x, y, z]).expect("product difference");
    assert!(
        compiled_diff
            .program()
            .data
            .instructions
            .iter()
            .all(|instruction| !matches!(instruction.operation, Operation::NegatedMultiply(_)))
    );
    assert_eq!(
        compiled_diff
            .evaluate(&[5.0, 2.0, 3.0, 4.0])
            .expect("complete row"),
        14.0
    );
}

#[test]
fn wide_multivariate_compact_polynomial_matches_direct_evaluation() {
    let context = Context::new().expect("context");
    let shared = context.symb("shared").expect("available identity");
    let vars: Vec<_> = (0..24)
        .map(|index| {
            context
                .symb(&format!("wide_v_{index}"))
                .expect("available identity")
        })
        .collect();
    let mut terms = Vec::with_capacity(vars.len().saturating_mul(2));
    for (index, &var) in vars.iter().enumerate() {
        let weight = i64::try_from(index.saturating_add(1)).expect("small index");
        terms.push(Mul::mul(var.pow(2), shared.to_expr()));
        terms.push(Mul::mul(
            Expr::number(weight),
            Mul::mul(var.pow(3), shared.pow(2)),
        ));
    }
    let expression = Expr::sum(terms);
    let mut symbols = Vec::with_capacity(vars.len().saturating_add(1));
    symbols.push(shared);
    symbols.extend(&vars);
    let inputs: Vec<f64> = (0..symbols.len())
        .map(|index| 0.05 * f64::from(u32::try_from(index.saturating_add(1)).expect("small index")))
        .collect();
    let direct = expression
        .evaluate(symbols.iter().copied().zip(inputs.iter().copied()))
        .expect("direct evaluation");
    let mut compiled =
        Compiled::<f64>::compile(&expression, &symbols).expect("compiled polynomial");
    let actual = compiled.evaluate(&inputs).expect("compiled evaluation");
    assert!((actual - direct).abs() <= 1.0e-12 * direct.abs().max(1.0));
}

#[test]
fn signed_fused_variants_and_subtractions_execute_inline_across_scalar_and_batch_lanes() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let y = context.symb("y").expect("available identity");
    let z = context.symb("z").expect("available identity");

    let cases = [
        (Add::add(Mul::mul(x, y), z), (false, false), "fma", 11.0_f64),
        (
            Add::add(Mul::mul(x, y), Mul::mul(-1, z)),
            (false, true),
            "fms",
            1.0_f64,
        ),
        (
            Add::add(z, Mul::mul(-1, Mul::mul(x, y))),
            (true, false),
            "fnma",
            -1.0_f64,
        ),
        (
            Mul::mul(-1, Add::add(Mul::mul(x, y), z)),
            (true, true),
            "fnms",
            -11.0_f64,
        ),
    ];

    let xs: Vec<f64> = (0..33).map(|i| f64::from(i).mul_add(0.25, 2.0)).collect();
    let ys: Vec<f64> = (0..33).map(|i| f64::from(i).mul_add(-0.125, 3.0)).collect();
    let zs: Vec<f64> = (0..33).map(|i| f64::from(i).mul_add(0.5, 5.0)).collect();

    for (expression, (neg_prod, neg_add), mnemonic, expected) in cases {
        let mut compiled =
            Compiled::<f64>::compile(&expression, &[x, y, z]).expect("fused variant");
        assert_eq!(
            compiled.evaluate(&[2.0, 3.0, 5.0]).expect("scalar row"),
            expected
        );
        if ProgramData::<f64>::fuse() {
            assert!(
                compiled.program().data.instructions.iter().any(|inst| {
                    matches!(
                        inst.operation,
                        Operation::Fused(fused, _)
                            if fused.negate_product == neg_prod
                                && fused.negate_addend == neg_add
                    )
                }),
                "missing fused variant {mnemonic}"
            );
            assert!(
                compiled.disassemble().to_string().contains(mnemonic),
                "disassembly missing {mnemonic}"
            );
        }
        let mut batch_out = vec![0.0; xs.len()];
        compiled
            .evaluate_batch(&[&xs, &ys, &zs], &mut batch_out)
            .expect("batch evaluation");
        for (row, &actual) in batch_out.iter().enumerate() {
            let scalar = compiled
                .evaluate(&[
                    *xs.get(row).expect("x row"),
                    *ys.get(row).expect("y row"),
                    *zs.get(row).expect("z row"),
                ])
                .expect("scalar oracle");
            assert_eq!(actual.to_bits(), scalar.to_bits(), "{mnemonic} row {row}");
        }
    }
}

#[cfg(not(feature = "std"))]
#[test]
fn no_std_float_policy_disables_software_fma() {
    use super::super::EvaluationValue;

    const { assert!(!f32::FUSE) };
    const { assert!(!f64::FUSE) };
    assert!(!ProgramData::<f32>::fuse());
    assert!(!ProgramData::<f64>::fuse());
}

#[test]
fn dead_sources_from_custom_bodies_do_not_pollute_reciprocals_or_pairs_and_constants_deduplicate() {
    use crate::core::UserFunction;
    use alloc::sync::Arc;

    let context = Context::new().expect("context");
    let keep_first = context
        .register_function(
            "keep_first",
            UserFunction::new(2..=2).body(Arc::new(|args| args.first().expect("first").clone())),
        )
        .expect("unique function");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");
    let z = context.symb("z").expect("z");

    // Dead `z / y` must not turn the single live `x / y` into `x * (1 / y)`.
    let div_expr = keep_first
        .call(vec![Div::div(x, y), Div::div(z, y)])
        .expect("call");
    let mut div_prog = Compiled::<f64>::compile(&div_expr, &[x, y, z]).expect("div compile");
    assert_eq!(div_prog.instruction_count(), 1);
    assert!(matches!(
        div_prog
            .program()
            .data
            .instructions
            .first()
            .map(|i| &i.operation),
        Some(Operation::Divide(_))
    ));
    assert_eq!(div_prog.evaluate(&[6.0, 3.0, 9.0]).expect("eval"), 2.0);

    // Dead `sin(x)` must not pair with the single live `cos(x)` into `SinCos(x)`.
    let trig_expr = keep_first.call(vec![x.cos(), x.sin()]).expect("trig call");
    let mut trig_prog = Compiled::<f64>::compile(&trig_expr, &[x]).expect("trig compile");
    assert!(
        trig_prog
            .program()
            .data
            .instructions
            .iter()
            .all(|inst| !matches!(inst.operation, Operation::Pair(..)))
    );
    assert_eq!(
        trig_prog.evaluate(&[0.5]).expect("cos eval").to_bits(),
        0.5_f64.cos().to_bits()
    );

    // Distinct `Expr` representations of the same float bit pattern share one constant register.
    let dedup_expr = Add::add(Div::div(x, 2), Mul::mul(y, Expr::number(0.5)));
    let mut dedup_prog = Compiled::<f64>::compile(&dedup_expr, &[x, y]).expect("dedup compile");
    assert_eq!(dedup_prog.program().data.constants.len(), 1);
    assert_eq!(dedup_prog.evaluate(&[4.0, 6.0]).expect("eval"), 5.0);

    // Negation of a single-use difference `-(a - b)` folds directly to `b - a`.
    let neg_sub = Mul::mul(-1, Add::add(x.sin(), Mul::mul(-1, y.cos())));
    let mut neg_sub_prog = Compiled::<f64>::compile(&neg_sub, &[x, y]).expect("neg_sub");
    assert!(
        neg_sub_prog
            .program()
            .data
            .instructions
            .iter()
            .all(|inst| !matches!(inst.operation, Operation::Negate(_)))
    );
    let expected = -(0.4_f64.sin() - 0.7_f64.cos());
    assert!((neg_sub_prog.evaluate(&[0.4, 0.7]).expect("eval") - expected).abs() <= 1.0e-15);
}

macro_rules! define_fuse_policy_wrapper {
    ($name:ident, fuse: $fuse:literal) => {
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
        struct $name(f64);

        impl core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Display::fmt(&self.0, formatter)
            }
        }

        define_fuse_policy_wrapper!(@ops $name: Add => add, Sub => sub, Mul => mul, Div => div);

        impl core::ops::Neg for $name {
            type Output = Self;
            fn neg(self) -> Self {
                Self(-self.0)
            }
        }

        impl num_anafis::Numeric for $name {
            fn from_number(value: &Number) -> Option<Self> {
                f64::from_number(value).map(Self)
            }
            define_fuse_policy_wrapper!(@delegate
                unary: sin, cos, tan, cot, sec, csc, asin, acos, atan, acot, asec, acsc,
                    sinh, cosh, tanh, coth, sech, csch, asinh, acosh, atanh, acoth, acsch,
                    asech, exp, exp2, exp10, exp_m1, ln, ln_1p, log2, log10, sqrt, rsqrt,
                    recip, cbrt, abs, signum, re, im, conj, heaviside, floor, ceil, round, trunc,
                    fract, sinc, sin_pi, cos_pi, gamma, lgamma, digamma, trigamma, tetragamma, erf,
                    erfc, erfcx, erfi, dawson, faddeeva, zeta, elliptic_k, elliptic_e;
                binary: atan2, log, pow, hypot, max, min, modulo, rem, polygamma, beta, zeta_deriv,
                    bessel_j, bessel_y, bessel_i, bessel_k, hermite, lambert_w
            );
            fn mul_add(&self, m: &Self, a: &Self) -> Self {
                Self(num_anafis::Numeric::mul_add(&self.0, &m.0, &a.0))
            }
            fn clamp(&self, min: &Self, max: &Self) -> Self {
                Self(num_anafis::Numeric::clamp(&self.0, &min.0, &max.0))
            }
            fn assoc_legendre(&self, d: &Self, o: &Self) -> Self {
                Self(num_anafis::Numeric::assoc_legendre(&self.0, &d.0, &o.0))
            }
        }

        impl super::super::EvaluationValue for $name {
            const OPTIMIZE_FLOATS: bool = true;
            const FUSE: bool = $fuse;
        }
    };
    (@ops $name:ident: $($trait:ident => $method:ident),*) => {
        $(
            impl core::ops::$trait for $name {
                type Output = Self;
                fn $method(self, other: Self) -> Self {
                    Self(<f64 as core::ops::$trait>::$method(self.0, other.0))
                }
            }
            impl core::ops::$trait<&Self> for $name {
                type Output = Self;
                fn $method(self, other: &Self) -> Self {
                    Self(<f64 as core::ops::$trait>::$method(self.0, other.0))
                }
            }
        )*
    };
    (@delegate unary: $($u:ident),*; binary: $($b:ident),*) => {
        $(fn $u(&self) -> Self { Self(num_anafis::Numeric::$u(&self.0)) })*
        $(fn $b(&self, other: &Self) -> Self { Self(num_anafis::Numeric::$b(&self.0, &other.0)) })*
    };
}

define_fuse_policy_wrapper!(OptNoFuse, fuse: false);
define_fuse_policy_wrapper!(OptForceFuse, fuse: true);

#[test]
fn custom_evaluation_value_fuse_policy_is_respected_by_vm_and_compiler() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");
    let z = context.symb("z").expect("z");
    let expr = Add::add(Mul::mul(x, y), z);

    // `FUSE = false` must never emit `Operation::Fused`, even on x86_64 hosts with FMA.
    assert!(!ProgramData::<OptNoFuse>::fuse());
    let mut no_fuse = Compiled::<OptNoFuse>::compile(&expr, &[x, y, z]).expect("no fuse");
    assert!(
        no_fuse
            .program()
            .data
            .instructions
            .iter()
            .all(|inst| !matches!(inst.operation, Operation::Fused(..)))
    );
    assert_eq!(
        no_fuse
            .evaluate(&[OptNoFuse(2.0), OptNoFuse(3.0), OptNoFuse(5.0)])
            .expect("eval"),
        OptNoFuse(11.0)
    );

    // `FUSE = true` must emit and execute `Operation::Fused` across both scalar and batch lanes.
    assert!(ProgramData::<OptForceFuse>::fuse());
    let mut force_fuse = Compiled::<OptForceFuse>::compile(&expr, &[x, y, z]).expect("force fuse");
    assert!(
        force_fuse
            .program()
            .data
            .instructions
            .iter()
            .any(|inst| matches!(inst.operation, Operation::Fused(..)))
    );
    assert_eq!(
        force_fuse
            .evaluate(&[OptForceFuse(2.0), OptForceFuse(3.0), OptForceFuse(5.0)])
            .expect("eval"),
        OptForceFuse(11.0)
    );
    let xs = [OptForceFuse(2.0), OptForceFuse(4.0), OptForceFuse(-1.5)];
    let ys = [OptForceFuse(3.0), OptForceFuse(5.0), OptForceFuse(2.0)];
    let zs = [OptForceFuse(5.0), OptForceFuse(-2.0), OptForceFuse(7.0)];
    let mut out = [OptForceFuse(0.0); 3];
    force_fuse
        .evaluate_batch(&[&xs, &ys, &zs], &mut out)
        .expect("batch eval");
    assert_eq!(
        out,
        [OptForceFuse(11.0), OptForceFuse(18.0), OptForceFuse(4.0)]
    );
}
