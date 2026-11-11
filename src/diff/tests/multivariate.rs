//! Coordinate ordering, forest memoization, and independent polynomial oracles.

use core::{
    ops::{Add, Mul},
    sync::atomic::{AtomicUsize, Ordering},
};

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
        let outputs = vec![input.clone(), Expr::product(vec![x.to_expr(), y.to_expr()]), input.clone()];
        let jacobian = Expr::jacobian(&outputs, &[y, x]);
        for (row, output) in jacobian.iter().zip(&outputs) {
            prop_assert_eq!(row, &output.gradient(&[y, x]));
        }
        let directional = input.directional_derivative(&[(x, Expr::number(3)), (y, Expr::number(-2))]);
        let expected_dir = expected_x
            .saturating_mul(3)
            .saturating_add(expected_y.saturating_mul(-2));
        prop_assert_eq!(evaluate(&directional), Expr::number(expected_dir));
        let hessian = input.hessian(&[y, x]);
        let trace = Expr::sum(vec![
            hessian.first().and_then(|row| row.first()).expect("H_00").clone(),
            hessian.get(1).and_then(|row| row.get(1)).expect("H_11").clone(),
        ]);
        prop_assert_eq!(input.laplacian(&[y, x]), trace);
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

#[test]
fn directional_derivative_and_mixed_partial_edges() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let y = context.symb("y").expect("symbol");
    let vx = context.symb("vx").expect("symbol").to_expr();
    let vy = context.symb("vy").expect("symbol").to_expr();

    let nonlinear = Expr::sum(vec![Mul::mul(x, y).sin(), x.pow(2), x.pow(3)]);
    assert_eq!(nonlinear.directional_derivative(&[]), Expr::number(0));
    assert_eq!(
        nonlinear.directional_derivative(&[(x, Expr::number(1)), (x, Expr::number(-1))]),
        Expr::number(0)
    );
    assert_eq!(
        nonlinear.directional_derivative(&[(x, Expr::number(1))]),
        nonlinear.clone().derivative(x, 1)
    );
    let combined_dir =
        nonlinear.directional_derivative(&[(x, vx.clone()), (y, vy.clone()), (x, vx.clone())]);
    assert_eq!(
        combined_dir,
        nonlinear.directional_derivative(&[(x, Mul::mul(2, &vx)), (y, vy.clone())])
    );

    let unknown = context
        .register_function("unknown_dir", UserFunction::new(2..=2))
        .expect("unknown");
    let call = unknown.call(vec![x.into(), y.into()]).expect("call");
    assert_eq!(
        call.directional_derivative(&[(x, vx.clone()), (y, vy.clone())]),
        Add::add(
            Mul::mul(Expr::derivative(call.clone(), x, 1), &vx),
            Mul::mul(Expr::derivative(call.clone(), y, 1), &vy),
        )
    );
    assert_eq!(
        unknown
            .call(vec![x.into(), Expr::number(1)])
            .expect("call")
            .directional_derivative(&[(y, vy)]),
        Expr::number(0)
    );

    let poly = Mul::mul(x.pow(3), y.pow(2));
    assert_eq!(poly.clone().partial_derivatives(&[]), poly);
    assert_eq!(
        poly.clone().partial_derivatives(&[(x, 2), (y, 1)]),
        Expr::product(vec![Expr::number(12), x.into(), y.into()])
    );
    assert_eq!(
        poly.partial_derivatives(&[(x, 4), (y, 50)]),
        Expr::number(0)
    );
    assert_eq!(
        call.clone().partial_derivatives(&[(x, 1), (y, 1)]),
        Expr::derivative(Expr::derivative(call, x, 1), y, 1)
    );
}

#[test]
fn laplacian_repeated_variable_dedup_and_symbol_fast_paths() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let y = context.symb("y").expect("symbol");

    let cubic_sum = Add::add(x.pow(3), y.pow(2));
    assert_eq!(cubic_sum.laplacian(&[]), Expr::number(0));
    assert_eq!(cubic_sum.laplacian(&[x, y]), Add::add(Mul::mul(6, x), 2));
    assert_eq!(cubic_sum.laplacian(&[x, x]), Mul::mul(12, x));

    let unknown = context
        .register_function("unknown_laplacian", UserFunction::new(2..=2))
        .expect("unknown");
    let call = unknown.call(vec![x.into(), y.into()]).expect("call");
    assert_eq!(
        call.laplacian(&[x, y]),
        Add::add(
            Expr::derivative(call.clone(), x, 2),
            Expr::derivative(call, y, 2),
        )
    );

    let count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&count);
    let tracked = context
        .register_function(
            "tracked_dedup",
            UserFunction::new(1..=1)
                .partial(
                    0,
                    Arc::new(move |_| {
                        counter.fetch_add(1, Ordering::Relaxed);
                        Expr::number(1)
                    }),
                )
                .expect("partial"),
        )
        .expect("tracked");
    let tracked_call = tracked.call(vec![x.into()]).expect("call");
    assert_eq!(
        tracked_call.gradient(&[x, x, x]),
        vec![Expr::number(1), Expr::number(1), Expr::number(1)]
    );
    assert_eq!(count.load(Ordering::Relaxed), 1);
    assert_eq!(
        Expr::jacobian(&[tracked_call], &[x, x]),
        vec![vec![Expr::number(1), Expr::number(1)]]
    );
    assert_eq!(count.load(Ordering::Relaxed), 2);

    assert_eq!(x.derivative(x, 0), x.to_expr());
    assert_eq!(x.derivative(x, 1), Expr::number(1));
    assert_eq!(x.derivative(x, 2), Expr::number(0));
    assert_eq!(x.derivative(y, 1), Expr::number(0));
    assert_eq!(
        x.gradient(&[y, x, x]),
        vec![Expr::number(0), Expr::number(1), Expr::number(1)]
    );
    assert_eq!(
        x.hessian(&[x, y]),
        vec![
            vec![Expr::number(0), Expr::number(0)],
            vec![Expr::number(0), Expr::number(0)],
        ]
    );
}
