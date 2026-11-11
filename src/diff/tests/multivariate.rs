//! Coordinate ordering, forest memoization, and independent polynomial oracles.

use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::{sync::Arc, vec};

use proptest::prelude::*;

use crate::core::{Context, Expr, Substitutions, UserFunction};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn polynomial_gradient_and_hessian_follow_independent_formulas(
        quadratic_x in -5_i64..=5, mixed in -5_i64..=5, quadratic_y in -5_i64..=5,
        linear_x in -5_i64..=5, linear_y in -5_i64..=5,
        value_x in -4_i64..=4, value_y in -4_i64..=4,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let input = Expr::sum(vec![
            Expr::product(vec![x.pow(2), Expr::number(quadratic_x)]),
            Expr::product(vec![x.to_expr(), Expr::number(mixed), y.to_expr()]),
            Expr::product(vec![y.pow(2), Expr::number(quadratic_y)]),
            Expr::product(vec![x.to_expr(), Expr::number(linear_x)]),
            Expr::product(vec![y.to_expr(), Expr::number(linear_y)]),
        ]);
        let actual = input.gradient(&[y, x]);
        let point = Substitutions::new([
            (x.into(), Expr::number(value_x)),
            (y.into(), Expr::number(value_y)),
        ])?;
        let evaluate = |expression: &Expr| point.apply_all(expression);
        let expected_x = quadratic_x.saturating_mul(2).saturating_mul(value_x)
            .saturating_add(mixed.saturating_mul(value_y)).saturating_add(linear_x);
        let expected_y = quadratic_y.saturating_mul(2).saturating_mul(value_y)
            .saturating_add(mixed.saturating_mul(value_x)).saturating_add(linear_y);
        prop_assert_eq!(actual.iter().map(evaluate).collect::<alloc::vec::Vec<_>>(),
            vec![Expr::number(expected_y), Expr::number(expected_x)]);
        prop_assert_eq!(input.hessian(&[y, x]), vec![
            vec![Expr::number(quadratic_y.saturating_mul(2)), Expr::number(mixed)],
            vec![Expr::number(mixed), Expr::number(quadratic_x.saturating_mul(2))],
        ]);
        let outputs = vec![input.clone(), Expr::product(vec![x.to_expr(), y.to_expr()]), input];
        let jacobian = Expr::jacobian(&outputs, &[y, x]);
        for (row, output) in jacobian.iter().zip(&outputs) {
            prop_assert_eq!(row, &output.gradient(&[y, x]));
        }
    }
}

#[test]
fn coordinate_identity_repetitions_and_empty_shapes_are_explicit() {
    let context = Context::new().expect("context");
    let other_context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let other_x = other_context.symb("x").expect("symbol");
    let input = x.pow(2);
    assert_eq!(
        input.gradient(&[other_x, x, x]),
        vec![Expr::number(0), 2 * x, 2 * x]
    );
    assert_eq!(input.gradient(&[]).len(), 0);
    assert_eq!(input.hessian(&[]).len(), 0);
    assert_eq!(Expr::jacobian(&[], &[x]).len(), 0);
    assert_eq!(Expr::jacobian(&[input], &[]), vec![vec![]]);
}

#[test]
fn jacobian_reuses_shared_callbacks_across_output_roots() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&count);
    let function = context
        .register_function(
            "shared",
            UserFunction::new(1..=1)
                .partial(
                    0,
                    Arc::new(move |_| {
                        counter.fetch_add(1, Ordering::Relaxed);
                        Expr::number(1)
                    }),
                )
                .expect("valid partial"),
        )
        .expect("function");
    let call = function.call(vec![x.to_expr()]).expect("unary call");
    let outputs = vec![call.clone(), call.pow(2), call.clone() + x, call];
    let jacobian = Expr::jacobian(&outputs, &[x]);
    assert_eq!(count.load(Ordering::Relaxed), 1);
    assert_eq!(jacobian.first(), Some(&vec![Expr::number(1)]));
}

#[test]
fn unresolved_mixed_partials_keep_their_operator_order_and_owners() {
    let (input, original_x, original_y) = {
        let context = Context::new().expect("context");
        let x = context.symb("x").expect("symbol");
        let y = context.symb("y").expect("symbol");
        let function = context
            .register_function("unknown", UserFunction::new(2..=2))
            .expect("function");
        (
            function.call(vec![x.to_expr(), y.to_expr()]).expect("call"),
            x.to_expr(),
            y.to_expr(),
        )
    };
    let x = original_x.as_symbol().expect("owning symbol expression");
    let y = original_y.as_symbol().expect("owning symbol expression");
    let expected_xy = Expr::derivative(Expr::derivative(input.clone(), x, 1), y, 1);
    let expected_yx = Expr::derivative(Expr::derivative(input.clone(), y, 1), x, 1);
    assert_ne!(expected_xy, expected_yx);
    assert_eq!(
        input.hessian(&[x, y]),
        vec![
            vec![Expr::derivative(input.clone(), x, 2), expected_xy],
            vec![expected_yx, Expr::derivative(input, y, 2)],
        ]
    );
}

#[test]
fn jacobian_trait_method_works_on_arrays_vectors_slices_and_single_expressions() {
    use crate::Jacobian;

    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let y = context.symb("y").expect("symbol");

    let f1 = x.pow(2) + x * y;
    let f2 = x * y;

    let expected = vec![vec![2 * x + y, x.to_expr()], vec![y.to_expr(), x.to_expr()]];

    // Array [Expr; 2]
    assert_eq!([f1.clone(), f2.clone()].jacobian(&[x, y]), expected);

    // Vec<Expr>
    let vec_outputs = vec![f1, f2];
    assert_eq!(vec_outputs.jacobian(&[x, y]), expected);

    // Slice &[Expr]
    assert_eq!(vec_outputs.as_slice().jacobian(&[x, y]), expected);
}
