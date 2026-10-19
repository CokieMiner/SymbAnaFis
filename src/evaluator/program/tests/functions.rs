//! Builtin argument order and backend precision contracts.

use alloc::vec;

use num_anafis::{F64Ext, Number};
use proptest::{prop_assert_eq, proptest};

use crate::{Context, Evaluator, Expr, Function};

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(24))]
    #[test]
    fn multi_argument_kernels_keep_symbolic_argument_positions(
        value in 0.2_f64..=0.8,
        degree in 0.2_f64..=2.0,
        order in -0.4_f64..=0.4,
        angle in 0.2_f64..=0.8,
    ) {
        let context = Context::new();
        let x = context.symb("x").expect("available identity");
        let n = context.symb("n").expect("available identity");
        let m = context.symb("m").expect("available identity");
        let p = context.symb("p").expect("available identity");
        let value_number = Number::from_float(value);
        let degree_number = Number::from_float(degree);
        let order_number = Number::from_float(order);
        let angle_number = Number::from_float(angle);
        let cases = [
            (x.hermite(n), value_number.hermite(&degree_number), F64Ext::hermite(value, degree)),
            (x.besselj(n), value_number.bessel_j(&degree_number), F64Ext::bessel_j(value, degree)),
            (x.log(n), value_number.log(&degree_number), F64Ext::log(value, degree)),
            (x.assoc_legendre(n, m), value_number.assoc_legendre(&degree_number, &order_number), F64Ext::assoc_legendre(value, degree, order)),
            (x.spherical_harmonic(n, m, p), value_number.spherical_harmonic(&degree_number, &order_number, &angle_number), F64Ext::spherical_harmonic(value, degree, order, angle)),
        ];
        for (expression, expected_number, expected_native) in cases {
            let mut general = Evaluator::<Number>::compile(&expression, &[x, n, m, p]).expect("bound parameters");
            let actual = general.evaluate(&[value_number.clone(), degree_number.clone(), order_number.clone(), angle_number.clone()]).expect("complete row");
            prop_assert_eq!(Expr::number(actual), Expr::number(expected_number));
            let mut native = Evaluator::<f64>::compile(&expression, &[x, n, m, p]).expect("real literals");
            let result = native.evaluate(&[value, degree, order, angle]).expect("complete row");
            prop_assert_eq!(result.to_bits(), expected_native.to_bits());
        }
    }
}

#[test]
fn every_builtin_signature_compiles_on_both_scalar_families() {
    let context = Context::new();
    let x = context.symb("x").expect("available identity");
    let names = [
        "sin",
        "cos",
        "tan",
        "cot",
        "sec",
        "csc",
        "asin",
        "acos",
        "atan",
        "acot",
        "asec",
        "acsc",
        "sinh",
        "cosh",
        "tanh",
        "coth",
        "sech",
        "csch",
        "asinh",
        "acosh",
        "atanh",
        "acoth",
        "asech",
        "acsch",
        "exp",
        "exp_m1",
        "exp_neg",
        "ln",
        "ln_1p",
        "log10",
        "log2",
        "sqrt",
        "cbrt",
        "floor",
        "ceil",
        "round",
        "trunc",
        "fract",
        "abs",
        "signum",
        "sinc",
        "erf",
        "erfc",
        "gamma",
        "lgamma",
        "digamma",
        "trigamma",
        "tetragamma",
        "zeta",
        "elliptic_k",
        "elliptic_e",
        "polygamma",
        "besselj",
        "bessely",
        "besseli",
        "besselk",
        "zeta_deriv",
        "log",
        "hermite",
        "lambertw",
        "beta",
        "atan2",
        "assoc_legendre",
        "spherical_harmonic",
    ];
    for name in names {
        let function = Function::builtin(name).expect("known builtin");
        let arity = *function.arity().start();
        let expression = function
            .call(vec![x.to_expr(); arity])
            .expect("catalogue arity");
        assert!(
            Evaluator::<Number>::compile(&expression, &[x]).is_ok(),
            "{name}"
        );
        assert!(
            Evaluator::<f64>::compile(&expression, &[x]).is_ok(),
            "{name}"
        );
    }
}
