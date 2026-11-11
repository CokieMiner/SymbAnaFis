//! Compact and forest numberings differentiate as the logical form does.

use core::ops::Div;

use alloc::{vec, vec::Vec};

use proptest::{collection, prelude::*};

use crate::{
    core::{Context, Expr, Postorder, Symbol},
    diff::Differentiation,
};

/// A polynomial in `x`, `y`, and \(\pi\) with the given term coefficients and
/// powers; `halves` makes each coefficient the float \(c/2\). A zero power
/// omits its base, so compact storage holds scalar bases only.
fn polynomial(terms: &[(i64, i64, i64, i64)], x: Symbol<'_>, y: Symbol<'_>, halves: bool) -> Expr {
    Expr::sum(
        terms
            .iter()
            .map(|&(coefficient, x_power, y_power, pi_power)| {
                let mut factors = vec![if halves {
                    Expr::number(f64::from(i32::try_from(coefficient).unwrap_or(0)) * 0.5)
                } else {
                    Expr::number(coefficient)
                }];
                for (base, power) in [
                    (x.to_expr(), x_power),
                    (y.to_expr(), y_power),
                    (Expr::pi(), pi_power),
                ] {
                    if power > 0 {
                        factors.push(base.pow(power));
                    }
                }
                Expr::product(factors)
            })
            .collect(),
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn compact_numbering_differentiates_as_the_logical_form(
        terms in collection::vec((-6_i64..=6, 0_i64..=5, 0_i64..=4, 0_i64..=2), 1..=6),
        shape in 0_u8..6,
        halves in proptest::bool::ANY,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let scalar = polynomial(&terms, x, y, halves);
        let opaque = Expr::sum(
            terms
                .iter()
                .map(|&(coefficient, x_power, y_power, _)| {
                    Expr::product(vec![Expr::number(coefficient), x.sin().pow(x_power), y.pow(y_power)])
                })
                .collect(),
        );
        let input = match shape {
            0 => scalar,
            1 => Expr::sum(vec![scalar, x.cos()]),
            2 => Expr::product(vec![scalar.sin(), scalar]),
            3 => Div::div(scalar.clone(), Expr::sum(vec![scalar, y.to_expr()])),
            4 => opaque,
            _ => Expr::sum(vec![opaque, scalar]),
        };
        for variable in [x, y] {
            let compact = Differentiation::new(variable).evaluate(&Postorder::compact(&input));
            let logical = Differentiation::new(variable).evaluate(&Postorder::logical(&input));
            prop_assert_eq!(compact, logical);
        }
    }

    #[test]
    fn forest_columns_equal_per_row_gradients(
        first in collection::vec((-6_i64..=6, 0_i64..=5, 0_i64..=4, 0_i64..=2), 1..=5),
        second in collection::vec((-6_i64..=6, 0_i64..=5, 0_i64..=4, 0_i64..=2), 1..=5),
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let shared = polynomial(&first, x, y, false);
        let rows = [shared.clone(), Expr::product(vec![shared.sin(), polynomial(&second, x, y, true)]), shared];
        let expected: Vec<_> = rows.iter().map(|row| row.gradient(&[x, y])).collect();
        prop_assert_eq!(Expr::jacobian(&rows, &[x, y]), expected);
    }
}
