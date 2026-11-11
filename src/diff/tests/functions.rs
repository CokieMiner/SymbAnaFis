//! Elementary chain rules and symbolic fallback contracts.

use core::{
    f64::consts,
    ops::{Add, Div, Mul, Neg, Sub},
};

use alloc::{string::ToString, vec};

use num_anafis::Number;
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::core::{
    BuiltinFunction, Context, Expr, ExprView, Function, FunctionDefinition, Substitution, Symbol,
};

proptest! {
    #[test]
    fn elementary_slopes_agree_with_complex_finite_differences(
        kind in 0_usize..32,
        real in 0.2_f64..0.8,
        imaginary in 0.25_f64..0.75,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let names = ["sin", "cos", "tan", "cot", "sec", "csc", "asin", "acos", "atan", "acot", "asec", "acsc",
            "sinh", "cosh", "tanh", "coth", "sech", "csch", "asinh", "acosh", "atanh", "acoth", "asech", "acsch",
            "exp", "exp_m1", "ln", "ln_1p", "log2", "log10", "sqrt", "cbrt"];
        let function = Function::builtin(names.get(kind).expect("index bounded by catalogue length"))
            .expect("builtin identity");
        let input = function.call(vec![Add::add(x.pow(2), 1)])?;
        let derivative = Expr::derivative(input.clone(), x, 1);
        let point = Number::from_complex(Number::from_float(real), Number::from_float(imaginary));
        let step = Number::from_float(1e-5);
        let upper = evaluate(&input, x, Add::add(&point, &step));
        let lower = evaluate(&input, x, Sub::sub(&point, &step));
        let reference: Number = Div::div(Sub::sub(upper, lower), Mul::mul(Number::from_int(2), step));
        let actual = evaluate(&derivative, x, point);
        let error = Sub::sub(&actual, &reference).abs().to_f64();
        let tolerance = Mul::mul(2e-7_f64, Add::add(1.0, reference.abs().to_f64()));
        prop_assert!(error <= tolerance, "{}: error {error}, tolerance {tolerance}", function.as_str());
    }

    #[test]
    fn special_slopes_and_multivariate_arguments_agree_with_finite_differences(
        kind in 0_usize..16,
        real in 0.4_f64..0.8,
        imaginary in 0.3_f64..0.7,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let input = match kind {
            0 => x.erf(), 1 => x.erfc(), 2 => x.gamma(), 3 => x.lgamma(),
            4 => x.digamma(), 5 => x.trigamma(), 6 => x.tetragamma(),
            7 => x.beta(Add::add(x.pow(2), 1)),
            8 => Add::add(x.pow(2), 3).log(Add::add(x, 2)),
            9 => x.lambertw(0),
            10 => x.besselj(3), 11 => x.bessely(2),
            12 => x.besseli(1), 13 => x.besselk(2),
            14 => x.hermite(4),
            _ => x.atan2(Add::add(x.pow(2), 1)),
        };
        let derivative = Expr::derivative(input.clone(), x, 1);
        let point = Number::from_complex(Number::from_float(real), Number::from_float(imaginary));
        let step = Number::from_float(1e-5);
        let upper = evaluate(&input, x, Add::add(&point, &step));
        let lower = evaluate(&input, x, Sub::sub(&point, &step));
        let reference = Div::div(Sub::sub(upper, lower), Mul::mul(Number::from_int(2), step));
        let actual = evaluate(&derivative, x, point);
        let error = Sub::sub(&actual, &reference).abs().to_f64();
        let tolerance = Mul::mul(2e-6_f64, Add::add(1.0, reference.abs().to_f64()));
        prop_assert!(error <= tolerance, "special family {kind}: error {error}, tolerance {tolerance}");
    }

    #[test]
    fn special_integer_orders_advance_without_representational_overflow(order in 0_i64..=20) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let expected = x.polygamma(order.saturating_add(1));
        prop_assert_eq!(Expr::derivative(x.polygamma(order), x, 1), expected);
        prop_assert_eq!(Expr::derivative(x.zeta_deriv(order), x, 1), x.zeta_deriv(order.saturating_add(1)));
        let hermite = Expr::derivative(x.hermite(order), x, 1);
        let expected_hermite = if order == 0 { Expr::number(0) } else {
            Mul::mul(Expr::number(order.saturating_mul(2)), x.hermite(order.saturating_sub(1)))
        };
        prop_assert_eq!(hermite, expected_hermite);
    }

    #[test]
    fn bessel_recurrences_retain_exact_halves_and_constant_orders(order in -8_i64..=8) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        for name in ["besselj", "bessely", "besseli", "besselk"] {
            let function = Function::builtin(name).expect("Bessel identity");
            let input = function.call(vec![Expr::number(order), x.into()])?;
            let previous = function.call(vec![Expr::number(order.saturating_sub(1)), x.into()])?;
            let next = function.call(vec![Expr::number(order.saturating_add(1)), x.into()])?;
            let numerator = match name {
                "besselj" | "bessely" => Sub::sub(previous, next),
                "besseli" => Add::add(previous, next),
                _ => Neg::neg(Add::add(previous, next)),
            };
            prop_assert_eq!(Expr::derivative(input, x, 1), Div::div(numerator, 2));
        }
    }
}

#[test]
fn supported_nodes_evaluate_to_their_slope() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    assert_eq!(Expr::derivative(x.sin(), x, 1), x.cos());
    assert_eq!(Expr::derivative(x.sin(), x, 2), -x.sin());
    assert_eq!(Expr::derivative(x.exp(), x, u32::MAX), x.exp());
    assert_eq!(Expr::derivative(x.abs(), x, 1), x.signum());
    assert_eq!(Expr::derivative(x.floor(), x, 1), Expr::number(0));
}

#[test]
fn standard_branch_formulas_keep_their_composed_square_roots() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let expected = Div::div(1, Mul::mul(Sub::sub(x, 1).sqrt(), Add::add(x, 1).sqrt()));
    assert_eq!(Expr::derivative(x.acosh(), x, 1), expected);
    assert_eq!(
        Expr::derivative(x.sqrt(), x, 1),
        Div::div(1, Mul::mul(2, x.sqrt()))
    );
    let lambert = x.lambertw(0);
    let expected_w = Div::div((-&lambert).exp(), Add::add(&lambert, 1));
    let derivative_w = Expr::derivative(lambert, x, 1);
    assert_eq!(derivative_w, expected_w);
    assert_eq!(
        Substitution::new(x.into(), Expr::number(0)).apply_all(&derivative_w),
        Expr::number(1)
    );
}

#[test]
fn isolated_singularities_keep_their_slope() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    assert_eq!(Expr::derivative(x.abs(), x, 1), x.signum());
    assert_eq!(Expr::derivative(x.signum(), x, 1), Expr::number(0));
    assert_eq!(
        Expr::derivative(x.sinc(), x, 1),
        Div::div(Sub::sub(Mul::mul(x, x.cos()), x.sin()), x.pow(2))
    );
    assert_eq!(
        Expr::derivative(x.elliptic_k(), x, 1),
        Div::div(
            Sub::sub(Div::div(x.elliptic_e(), Sub::sub(1, x)), x.elliptic_k()),
            Mul::mul(2, x)
        )
    );
    assert_eq!(
        Expr::derivative(x.elliptic_e(), x, 1),
        Div::div(Sub::sub(x.elliptic_e(), x.elliptic_k()), Mul::mul(2, x))
    );
    let current = x.assoc_legendre(2, 1);
    let previous = x.assoc_legendre(1, 1);
    assert_eq!(
        Expr::derivative(current.clone(), x, 1),
        Div::div(
            Sub::sub(Mul::mul(Mul::mul(2, x), current), Mul::mul(3, previous)),
            Sub::sub(x.pow(2), 1)
        )
    );
}

#[test]
fn step_functions_differentiate_almost_everywhere() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    // The step functions are constant on every open interval between their
    // discontinuities, so their slope is zero there and `fract` has slope one.
    for input in [x.floor(), x.ceil(), x.round(), x.trunc()] {
        assert_eq!(Expr::derivative(input, x, 1), Expr::number(0));
    }
    assert_eq!(Expr::derivative(x.fract(), x, 1), Expr::number(1));
}

#[test]
fn varying_orders_remain_deferred() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let y = context.symb("y").expect("symbol allocation");
    for input in [
        y.besselj(x),
        x.besselj(x),
        x.polygamma(i64::MAX),
        x.zeta_deriv(i64::MAX),
        x.hermite(0.5),
        x.hermite(y),
        x.besselj(i64::MIN),
        x.besselj(i64::MAX),
        x.assoc_legendre(x, 1),
        x.assoc_legendre(2, x),
    ] {
        let result = Expr::derivative(input.clone(), x, 1);
        assert!(matches!(result.view(), ExprView::Derivative { inner, .. } if inner == &input));
    }
}

#[test]
fn every_builtin_either_differentiates_or_is_explicitly_deferred() {
    // Every builtin has a slope on its regular domain. A builtin whose slope
    // has no single formula must be listed here, so that adding one without a
    // derivative fails this test instead of silently deferring.
    const DEFERRED: &[BuiltinFunction] = &[];
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    for builtin in BuiltinFunction::ALL {
        let (before, after) = builtin.method_arity();
        let mut arguments = vec![Expr::number(2); before];
        arguments.push(x.to_expr());
        arguments.extend(vec![Expr::number(2); after]);
        let call = FunctionDefinition::construct(builtin.function(), arguments);
        let derivative = Expr::derivative(call.clone(), x, 1);
        let deferred =
            matches!(derivative.view(), ExprView::Derivative { inner, .. } if inner == &call);
        assert_eq!(
            deferred,
            DEFERRED.contains(&builtin),
            "`{}` must {} a slope",
            builtin.as_str(),
            if DEFERRED.contains(&builtin) {
                "defer"
            } else {
                "produce"
            }
        );
    }
}

#[test]
fn real_branch_guards_and_legendre_and_discrete_edges() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let nonpositive = Neg::neg(x.abs());
    let nonnegative = x.abs();
    assert_eq!(
        Expr::derivative(nonnegative.abs(), x, 1),
        Expr::derivative(nonnegative, x, 1)
    );
    assert_eq!(
        Expr::derivative(nonpositive.abs(), x, 1),
        Neg::neg(Expr::derivative(nonpositive, x, 1))
    );

    let non_real = Add::add(x.abs(), Expr::i());
    for input in [
        non_real.abs(),
        non_real.signum(),
        non_real.floor(),
        non_real.ceil(),
        non_real.round(),
        non_real.trunc(),
        non_real.fract(),
    ] {
        let result = Expr::derivative(input.clone(), x, 1);
        assert!(matches!(result.view(), ExprView::Derivative { inner, .. } if inner == &input));
    }

    assert_eq!(
        Expr::derivative(x.assoc_legendre(0, 0), x, 1),
        Expr::number(0)
    );
    for (degree, order) in [(1, 1), (2, 2), (1, -1), (2, -2)] {
        let call = x.assoc_legendre(degree, order);
        let expected = Div::div(Mul::mul(Mul::mul(degree, x), &call), Sub::sub(x.pow(2), 1));
        assert_eq!(Expr::derivative(call, x, 1), expected);
    }

    let approx_imaginary_order =
        Number::from_complex(Number::from_int(2), Number::from_float(-0.0));
    let call = x.hermite(approx_imaginary_order);
    let deferred = Expr::derivative(call.clone(), x, 1);
    assert!(matches!(deferred.view(), ExprView::Derivative { inner, .. } if inner == &call));
}

#[test]
fn new_elementary_and_piecewise_builtins_close_under_higher_order_derivatives() {
    use crate::core::Property;

    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let r = context.symb("r").expect("r");
    r.set_properties(&[Property::Real]).expect("real");

    // Complex component derivatives for real parameter x:
    // d^n/dx^n re(e^{i x}) = re(i^n e^{i x}), etc.
    let wave = Mul::mul(Expr::i(), x).exp();
    let test_point = Number::from_float(0.7);
    let step = Number::from_float(1e-5);
    for (expr, name) in [
        (wave.re(), "re(exp(i*x))"),
        (wave.im(), "im(exp(i*x))"),
        (wave.conj(), "conj(exp(i*x))"),
        (Add::add(x, 1).hypot(Add::add(x.pow(2), 2)), "hypot"),
        (x.pow(2).max(Mul::mul(3, x)), "max"),
        (x.pow(2).min(Mul::mul(3, x)), "min"),
        (
            x.pow(2).clamp(Sub::sub(x, 1), Add::add(Mul::mul(2, x), 1)),
            "clamp",
        ),
        (Add::add(x.pow(2), 3).modulo(Add::add(x, 2)), "mod"),
        (Add::add(x.pow(2), 3).rem(Add::add(x, 2)), "rem"),
        (Sub::sub(x, 2).heaviside(), "heaviside"),
    ] {
        // Every builtin closes cleanly to 1st, 2nd, and 3rd derivatives without deferring.
        let mut current = expr;
        for order in 1..=3 {
            let next = Expr::derivative(current.clone(), x, 1);
            assert!(
                !matches!(next.view(), ExprView::Derivative { .. }),
                "{name} order {order} should not defer"
            );
            let upper = evaluate(&current, x, Add::add(&test_point, &step));
            let lower = evaluate(&current, x, Sub::sub(&test_point, &step));
            let fd = Div::div(Sub::sub(upper, lower), Mul::mul(Number::from_int(2), &step));
            let actual = evaluate(&next, x, test_point.clone());
            let error = Sub::sub(&actual, &fd).abs().to_f64();
            let tolerance = Mul::mul(1e-5_f64, Add::add(1.0, fd.abs().to_f64()));
            assert!(
                error <= tolerance,
                "{name} order {order}: actual={actual}, fd={fd}, error={error}"
            );
            current = next;
        }
    }

    // Real-valued arguments simplify re/im/conj derivatives directly.
    assert_eq!(Expr::derivative(r.pow(2).re(), r, 1), Mul::mul(2, r));
    assert_eq!(Expr::derivative(r.pow(2).im(), r, 1), Expr::number(0));
    assert_eq!(Expr::derivative(r.pow(2).conj(), r, 1), Mul::mul(2, r));

    // Constant finite real bounds eliminate the nested `max(u, lo)` in `d/dx clamp(u, lo, hi)`.
    let unit_clamp_deriv = Expr::derivative(x.clamp(0, 1), x, 1);
    assert_eq!(
        unit_clamp_deriv,
        Mul::mul(Sub::sub(1, x).heaviside(), x.heaviside())
    );
    assert!(!unit_clamp_deriv.to_string().contains("max"));
    assert_eq!(Expr::derivative(x.clamp(2, 2), x, 1), Expr::number(0));
    assert_eq!(Expr::derivative(x.clamp(5, 2), x, 1), Expr::number(0));
    let quad_clamp_deriv = Expr::derivative(x.pow(2).clamp(-1, 3), x, 1);
    assert!(!quad_clamp_deriv.to_string().contains("max"));
    assert_eq!(
        quad_clamp_deriv,
        Mul::mul(
            Sub::sub(3, x.pow(2)).heaviside(),
            Mul::mul(Add::add(x.pow(2), 1).heaviside(), Mul::mul(2, x))
        )
    );

    // Real-only piecewise/remainder builtins defer on proved non-real arguments.
    let non_real = Add::add(x.abs(), Expr::i());
    for input in [
        non_real.heaviside(),
        non_real.max(x),
        non_real.min(x),
        non_real.clamp(0, 1),
        non_real.modulo(2),
        non_real.rem(2),
    ] {
        let result = Expr::derivative(input.clone(), x, 1);
        assert!(matches!(result.view(), ExprView::Derivative { inner, .. } if inner == &input));
    }

    // Hypot is analytic on complex inputs too.
    let hypot_expr = x.hypot(Add::add(x.pow(2), 1));
    let hypot_deriv = Expr::derivative(hypot_expr.clone(), x, 1);
    let z_point = Number::from_complex(Number::from_float(0.6), Number::from_float(0.4));
    let upper = evaluate(&hypot_expr, x, Add::add(&z_point, &step));
    let lower = evaluate(&hypot_expr, x, Sub::sub(&z_point, &step));
    let reference = Div::div(Sub::sub(upper, lower), Mul::mul(Number::from_int(2), &step));
    let actual = evaluate(&hypot_deriv, x, z_point);
    let error = Sub::sub(&actual, &reference).abs().to_f64();
    assert!(error <= 1e-6 * (1.0 + reference.abs().to_f64()));
}

fn evaluate(expression: &Expr, variable: Symbol<'_>, point: Number) -> Number {
    let input = Substitution::new(variable.into(), Expr::number(point)).apply_all(expression);
    let resolved = input
        .map(|node| {
            Ok::<_, ()>(match node.view() {
                ExprView::Number(value) => Some(Expr::number(value.to_float())),
                ExprView::Pi => Some(Expr::number(consts::PI)),
                ExprView::Euler => Some(Expr::number(consts::E)),
                ExprView::ImaginaryUnit
                | ExprView::Symbol(_)
                | ExprView::Function { .. }
                | ExprView::Sum(_)
                | ExprView::Product(_)
                | ExprView::Div(..)
                | ExprView::Pow(..)
                | ExprView::Derivative { .. } => None,
            })
        })
        .expect("constant projection cannot fail");
    resolved
        .as_number()
        .expect("finite numerical result")
        .clone()
}
