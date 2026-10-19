//! Elementary chain rules and symbolic fallback contracts.

use core::{
    f64::consts,
    ops::{Add, Div, Mul, Neg, Sub},
};

use alloc::vec;

use num_anafis::Number;
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::core::{Context, Expr, ExprView, Function, Symbol};

proptest! {
    #[test]
    fn elementary_slopes_agree_with_complex_finite_differences(
        kind in 0_usize..33,
        real in 0.2_f64..0.8,
        imaginary in 0.25_f64..0.75,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let names = ["sin", "cos", "tan", "cot", "sec", "csc", "asin", "acos", "atan", "acot", "asec", "acsc",
            "sinh", "cosh", "tanh", "coth", "sech", "csch", "asinh", "acosh", "atanh", "acoth", "asech", "acsch",
            "exp", "exp_m1", "exp_neg", "ln", "ln_1p", "log2", "log10", "sqrt", "cbrt"];
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
        kind in 0_usize..17,
        real in 0.4_f64..0.8,
        imaginary in 0.3_f64..0.7,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let input = match kind {
            0 => x.erf(), 1 => x.erfc(), 2 => x.gamma(), 3 => x.lgamma(),
            4 => x.digamma(), 5 => x.trigamma(), 6 => x.tetragamma(),
            7 => x.beta(Add::add(x.pow(2), 1)),
            8 => Add::add(x.pow(2), 3).log(Add::add(x, 2)),
            9 => x.lambertw(0),
            10 => x.besselj(3), 11 => x.bessely(2),
            12 => x.besseli(1), 13 => x.besselk(2),
            14 => x.hermite(4), 15 => x.atan2(Add::add(x.pow(2), 1)),
            _ => Expr::number(0.7).spherical_harmonic(2, 1, x),
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
        let context = Context::new();
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
        let context = Context::new();
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
fn supported_nodes_evaluate_and_nonsmooth_nodes_remain_symbolic() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    assert_eq!(Expr::derivative(x.sin(), x, 1), x.cos());
    assert_eq!(Expr::derivative(x.sin(), x, 2), -x.sin());
    assert_eq!(Expr::derivative(x.exp(), x, u32::MAX), x.exp());
    assert!(matches!(
        Expr::derivative(x.abs(), x, 1).view(),
        ExprView::Derivative { order: 1, .. }
    ));
}

#[test]
fn standard_branch_formulas_keep_their_composed_square_roots() {
    let context = Context::new();
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
        derivative_w.substitute_symbol(x, &Expr::number(0)),
        Expr::number(1)
    );
}

#[test]
fn nonsmooth_functions_removable_poles_and_varying_orders_remain_deferred() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let y = context.symb("y").expect("symbol allocation");
    for input in [
        x.abs(),
        x.signum(),
        x.floor(),
        x.ceil(),
        x.round(),
        x.trunc(),
        x.fract(),
        x.sinc(),
        x.elliptic_k(),
        x.elliptic_e(),
        y.besselj(x),
        x.besselj(x),
        x.polygamma(i64::MAX),
        x.zeta_deriv(i64::MAX),
        x.hermite(0.5),
        x.hermite(y),
        x.besselj(i64::MIN),
        x.besselj(i64::MAX),
        x.assoc_legendre(2, 1),
    ] {
        let result = Expr::derivative(input.clone(), x, 1);
        assert!(matches!(result.view(), ExprView::Derivative { inner, .. } if inner == &input));
    }
}

fn evaluate(expression: &Expr, variable: Symbol<'_>, point: Number) -> Number {
    let input = expression.substitute_symbol(variable, &Expr::number(point));
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
