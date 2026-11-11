//! Builtin argument order and backend precision contracts.

use alloc::vec;

use num_anafis::{Number, Numeric};
use proptest::{prop_assert_eq, proptest};

use crate::core::{Context, Expr, Function};

use super::support::Compiled;

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(24))]
    #[test]
    fn multi_argument_kernels_keep_symbolic_argument_positions(
        value in 0.2_f64..=0.8,
        degree in 0.2_f64..=2.0,
        order in -0.4_f64..=0.4,
    ) {
        let context = Context::new()?;
        let x = context.symb("x").expect("available identity");
        let n = context.symb("n").expect("available identity");
        let m = context.symb("m").expect("available identity");
        let value_number = Number::from_float(value);
        let degree_number = Number::from_float(degree);
        let order_number = Number::from_float(order);
        let cases = [
            (x.hermite(n), value_number.hermite(&degree_number), Numeric::hermite(&value, &degree)),
            (x.besselj(n), value_number.bessel_j(&degree_number), Numeric::bessel_j(&value, &degree)),
            (x.log(n), value_number.log(&degree_number), Numeric::log(&value, &degree)),
            (x.hypot(n), value_number.hypot(&degree_number), Numeric::hypot(&value, &degree)),
            (x.max(n), value_number.max(&degree_number), Numeric::max(&value, &degree)),
            (x.min(n), value_number.min(&degree_number), Numeric::min(&value, &degree)),
            (x.modulo(n), value_number.modulo(&degree_number), Numeric::modulo(&value, &degree)),
            (x.rem(n), value_number.rem(&degree_number), Numeric::rem(&value, &degree)),
            (x.assoc_legendre(n, m), value_number.assoc_legendre(&degree_number, &order_number), Numeric::assoc_legendre(&value, &degree, &order)),
            (x.clamp(m, n), value_number.clamp(&order_number, &degree_number), Numeric::clamp(&value, &order, &degree)),
        ];
        for (expression, expected_number, expected_native) in cases {
            let mut general = Compiled::<Number>::compile(&expression, &[x, n, m]).expect("bound parameters");
            let actual = general.evaluate(&[value_number.clone(), degree_number.clone(), order_number.clone()]).expect("complete row");
            prop_assert_eq!(Expr::number(actual), Expr::number(expected_number));
            let mut native = Compiled::<f64>::compile(&expression, &[x, n, m]).expect("real literals");
            let result = native.evaluate(&[value, degree, order]).expect("complete row");
            prop_assert_eq!(result.to_bits(), expected_native.to_bits());
        }
    }
}

#[test]
fn every_builtin_signature_compiles_on_both_scalar_families() {
    let context = Context::new().expect("context");
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
        "re",
        "im",
        "conj",
        "heaviside",
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
        "hypot",
        "max",
        "min",
        "mod",
        "rem",
        "assoc_legendre",
        "clamp",
    ];
    for name in names {
        let function = Function::builtin(name).expect("known builtin");
        let arity = *function.arity().start();
        let expression = function
            .call(vec![x.to_expr(); arity])
            .expect("catalogue arity");
        assert!(
            Compiled::<Number>::compile(&expression, &[x]).is_ok(),
            "{name}"
        );
        assert!(
            Compiled::<f64>::compile(&expression, &[x]).is_ok(),
            "{name}"
        );
    }
}
