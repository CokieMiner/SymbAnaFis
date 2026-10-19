//! Algebraic derivative properties.

use core::ops::{Add, Div, Mul, Sub};

use alloc::{vec, vec::Vec};

use proptest::{collection, prop_assert, prop_assert_eq, proptest};

use crate::core::{Context, Expr, ExprView};

proptest! {
    #[test]
    fn exact_polynomial_powers_differentiate(n in 1_u32..=16, coefficient in -16_i32..=16) {
        let context = Context::new();
        let x = context.symb("x")?;
        let input = Mul::mul(coefficient, x.pow(i64::from(n)));
        let expected = if n == 1 {
            Expr::number(coefficient)
        } else {
            Mul::mul(Mul::mul(coefficient, Expr::number(i64::from(n))), x.pow(i64::from(n.saturating_sub(1))))
        };
        prop_assert_eq!(Expr::derivative(input, x, 1), expected);
    }

    #[test]
    fn sparse_multivariate_polynomials_follow_an_independent_integer_oracle(
        terms in collection::vec((-4_i64..=4, 1_u32..=5, 1_u32..=4), 2..=10),
        x_value in -3_i64..=3,
        y_value in -3_i64..=3,
        order in 0_u32..=7,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let input = Expr::sum(terms.iter().map(|(coefficient, dx, dy)| Expr::product(vec![
            Expr::number(*coefficient), x.pow(i64::from(*dx)), y.pow(i64::from(*dy)),
        ])).collect());
        let derivative = Expr::derivative(input, x, order);
        let actual = derivative.substitute_symbol(x, &Expr::number(x_value))
            .substitute_symbol(y, &Expr::number(y_value));
        let mut expected = 0_i64;
        for (coefficient, dx, dy) in terms {
            if let Some(power) = dx.checked_sub(order) {
                let mut weight = coefficient;
                for step in 0..order {
                    weight = weight.checked_mul(i64::from(dx.saturating_sub(step)))
                        .expect("bounded falling factorial fits i64");
                }
                let value = weight.checked_mul(x_value.checked_pow(power).expect("bounded power"))
                    .and_then(|first| first.checked_mul(y_value.checked_pow(dy)?))
                    .expect("bounded monomial fits i64");
                expected = expected.checked_add(value).expect("ten bounded terms fit i64");
            }
        }
        prop_assert_eq!(actual, Expr::number(expected));
    }

    #[test]
    fn balanced_product_rules_preserve_regular_zero_factors(count in 11_i32..=18) {
        let context = Context::new();
        let x = context.symb("x")?;
        let factors = (0..count).map(|offset| Sub::sub(x, offset)).collect::<Vec<_>>();
        let derivative = Expr::derivative(Expr::product(factors), x, 1);
        let actual = derivative.substitute_symbol(x, &Expr::number(0));
        let expected = (1..count).fold(1_i64, |product, offset| product
            .checked_mul(i64::from(offset).saturating_neg()).expect("17 factorial fits i64"));
        prop_assert_eq!(actual, Expr::number(expected));
        prop_assert!(!derivative.fold(false, |found, node| found || matches!(node.view(), ExprView::Div(..))));
    }

    #[test]
    fn reciprocal_derivatives_use_positive_denominator_powers(n in 1_i64..=12) {
        let context = Context::new();
        let x = context.symb("x")?;
        let derivative = Expr::derivative(Div::div(1, x.pow(n)), x, 1);
        let at_two = derivative.substitute_symbol(x, &Expr::number(2));
        let expected = Div::div(Expr::number(n.saturating_neg()), Expr::number(2).pow(n.saturating_add(1)));
        prop_assert_eq!(at_two, expected);
    }
}

#[test]
fn scalar_rules_do_not_run_a_separate_simplifier() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let input = Expr::sum(vec![x.to_expr(), x.pow(2), x.pow(3)]);
    let expected = Expr::sum(vec![Expr::number(1), Mul::mul(2, x), Mul::mul(3, x.pow(2))]);
    assert_eq!(Expr::derivative(input, x, 1), expected);
    assert_eq!(Expr::derivative(x.pow(1), x, 1), Expr::number(1));
    assert_eq!(Expr::derivative(x.pow(2), x, 2), Expr::number(2));
    let power = x.pow(x);
    let derivative = Expr::derivative(power, x, 1);
    let expected_power = Add::add(
        Mul::mul(x, x.pow(Sub::sub(x.to_expr(), 1))),
        Mul::mul(x.pow(x), x.ln()),
    );
    assert_eq!(derivative, expected_power);
}
