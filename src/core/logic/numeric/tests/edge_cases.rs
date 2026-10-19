//! Focused numeric boundary regressions.

use alloc::format;

use num_anafis::{Number, Real};

use crate::core::{Expr, ExprView, Symbol};

#[test]
fn constants_have_distinct_nodes_from_named_variables_and_float_approximations() {
    assert_eq!(Expr::pi(), Expr::pi());
    assert_eq!(Expr::e(), Expr::e());
    assert_eq!(Expr::i(), Expr::number(Number::i()));
    assert!(matches!(Expr::pi().view(), ExprView::Pi));
    assert!(matches!(Expr::e().view(), ExprView::Euler));
    assert!(matches!(Expr::i().view(), ExprView::ImaginaryUnit));

    for (constant, variable) in [(Expr::pi(), "pi"), (Expr::e(), "e"), (Expr::i(), "i")] {
        assert_ne!(constant, Symbol::intern(variable).to_expr());
        assert!(constant.variables().is_empty());
    }
    assert_ne!(Expr::pi(), Expr::number(core::f64::consts::PI));
    assert_eq!(Expr::pi().to_latex(), "\\pi");
}

#[test]
fn exact_and_approximate_tiers_keep_structural_identity() {
    assert_ne!(Expr::number(1), Expr::number(1.0));
    assert_ne!(Expr::number(0), Expr::number(0.0));
    assert_ne!(
        Expr::number(Number::from_int(9_007_199_254_740_993)),
        Expr::number(9_007_199_254_740_992.0)
    );
}

#[test]
fn native_overflow_and_minimum_components_have_explicit_contracts() {
    let limit = Expr::number(i64::MAX);
    assert_eq!(&limit + 1, Expr::number(i64::MIN));
    assert_eq!(
        Expr::number(1_i64 << 32) * Expr::number(1_i64 << 32),
        Expr::number(0)
    );
    assert_eq!(Expr::number(2).pow(63), Expr::number(i64::MIN));
    assert_eq!(Expr::number(i64::MIN) * -1, Expr::number(i64::MIN));
    assert_eq!(
        format!("{}", Expr::number(i64::MIN)),
        "-9223372036854775808"
    );

    let minimum = Expr::number(Number::from_complex(
        Number::one(),
        Number::from_int(i64::MIN),
    ));
    assert_eq!(format!("{minimum}"), "1 - 9223372036854775808*i");
}

#[test]
fn exact_negative_square_roots_keep_the_imaginary_unit() {
    assert_eq!(Expr::number(-4).sqrt(), 2 * Expr::i());
}

#[test]
fn exact_origin_cells_and_inexact_calls_stay_symbolic() {
    assert_eq!(Expr::number(0).sin(), Expr::number(0));
    assert_eq!(Expr::number(0).cos(), Expr::number(1));
    assert_eq!(Expr::number(0).exp_m1(), Expr::number(0));
}

#[test]
fn zero_denominators_and_approximate_underflow_remain_operations() {
    let tiny = Expr::number(f64::from_bits(1));
    assert!(matches!((&tiny / 2.0).view(), ExprView::Div(..)));
    assert!(matches!((&tiny * &tiny).view(), ExprView::Product(_)));
    assert!(matches!(tiny.pow(2).view(), ExprView::Pow(..)));
    assert!(matches!(
        Expr::number(1.0)
            .as_number()
            .and_then(|number| number.extract_complex().ok())
            .map(|parts| parts.re),
        Some(Real::Float(_))
    ));
}
