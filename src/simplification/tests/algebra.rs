//! Polynomial transformations and exact regression properties.

use core::ops::{Add, Div, Mul};

use alloc::{format, vec};

use proptest::prelude::*;

use crate::{Context, Expr, ExprView, HashMap, SimplificationSequence, SimplificationStep};

use super::simplify;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn expansion_preserves_integer_polynomials_and_eliminates_distributable_nodes(
        first in -3_i64..=3, second in -3_i64..=3, constant in -3_i64..=3,
        degree in 2_i64..=5, x_value in -3_i64..=3, y_value in -3_i64..=3,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let polynomial = Expr::sum(vec![Mul::mul(x, first), Mul::mul(y, second), Expr::number(constant)]);
        let input = Mul::mul(polynomial.pow(degree), Add::add(x, y));
        let program = SimplificationSequence::new().then(SimplificationStep::Expand)
            .target(crate::SimplificationTarget::Expanded);
        let result = simplify(&program, &input);
        let substitutions = HashMap::from([(x, Expr::number(x_value)), (y, Expr::number(y_value))]);
        prop_assert_eq!(input.substitute_symbols(&substitutions), result.substitute_symbols(&substitutions));
        prop_assert!(result.fold(true, |valid, node| valid && match node.view() {
            ExprView::Product(factors) => !factors.iter().any(|factor| matches!(factor.view(), ExprView::Sum(_))),
            ExprView::Pow(base, exponent) => !matches!(base.view(), ExprView::Sum(_)) || !exponent.as_number().is_some_and(|value| value.is_int() && value.to_int() > 1),
            ExprView::Number(_) | ExprView::ImaginaryUnit | ExprView::Pi | ExprView::Euler
            | ExprView::Symbol(_) | ExprView::Function { .. } | ExprView::Sum(_)
            | ExprView::Div(..) | ExprView::Derivative { .. } => true,
        }), "expanded output contains no distributable arithmetic nodes");
    }

    #[test]
    fn factoring_extracts_common_powers_and_numeric_gcd_without_symbolic_division(
        first in 1_i64..=6, second in 1_i64..=6, degree in 1_i64..=5,
        x_value in -3_i64..=3, y_value in -3_i64..=3,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let input = Add::add(Mul::mul(Mul::mul(x.pow(degree), y), first), Mul::mul(x.pow(degree), second));
        let output = simplify(&SimplificationSequence::factored(), &input);
        let substitutions = HashMap::from([(x, Expr::number(x_value)), (y, Expr::number(y_value))]);
        prop_assert_eq!(input.substitute_symbols(&substitutions), output.substitute_symbols(&substitutions));
        prop_assert!(!output.fold(false, |found, node| found || matches!(node.view(), ExprView::Div(..))));
        prop_assert_eq!(&output, &simplify(&SimplificationSequence::factored(), &output));
    }

    #[test]
    fn fraction_addition_preserves_exact_values_and_denominator_exclusions(
        a in -9_i64..=9, b in 1_i64..=9, c in -9_i64..=9, d in 1_i64..=9,
    ) {
        let context = Context::new();
        let numerator = context.symb("a")?;
        let denominator = context.symb("b")?;
        let other_top = context.symb("c")?;
        let other_bottom = context.symb("d")?;
        let input = Add::add(Div::div(numerator, denominator), Div::div(other_top, other_bottom));
        let sequence = SimplificationSequence::new().then(SimplificationStep::Algebraic)
            .score_with(|node| node.fold(0_usize, |count, part| count.saturating_add(usize::from(matches!(part.view(), ExprView::Div(..))))));
        let output = simplify(&sequence, &input);
        let substitutions = HashMap::from([(numerator, Expr::number(a)), (denominator, Expr::number(b)), (other_top, Expr::number(c)), (other_bottom, Expr::number(d))]);
        prop_assert_eq!(input.substitute_symbols(&substitutions), output.substitute_symbols(&substitutions));
        for symbol in [denominator, other_bottom] {
            let excluded = output.substitute_symbol(symbol, &Expr::number(0));
            prop_assert_eq!(excluded.properties().get(crate::Property::Finite), crate::Truth::False, "both original zero exclusions survive fraction addition");
        }
    }
}

#[test]
fn perfect_powers_and_differences_have_factored_presets() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol");
    let y = context.symb("y").expect("symbol");
    for degree in [2, 3] {
        let power = Add::add(x, y).pow(degree);
        let expanded = simplify(&SimplificationSequence::expanded(), &power);
        assert_eq!(
            simplify(&SimplificationSequence::factored(), &expanded),
            power
        );
        for offset in [1, -1] {
            let shifted = Add::add(x, offset).pow(degree);
            let expanded_constant = simplify(&SimplificationSequence::expanded(), &shifted);
            assert_eq!(
                simplify(&SimplificationSequence::factored(), &expanded_constant),
                shifted
            );
        }
    }
    let difference = x.pow(2) - y.pow(2);
    let factored = (x - y) * (x + y);
    assert_eq!(
        simplify(&SimplificationSequence::factored(), &difference),
        factored
    );
}

#[test]
fn power_collection_finishes_long_products_in_one_pass() {
    let context = Context::new();
    let base = context.symb("base").expect("symbol").exp();
    let exponents = (0..40)
        .map(|index| {
            context
                .symb(&format!("exponent_{index}"))
                .expect("symbol")
                .to_expr()
        })
        .collect::<alloc::vec::Vec<_>>();
    let input = Expr::product(
        exponents
            .iter()
            .map(|exponent| base.pow(exponent))
            .collect(),
    );
    let program = SimplificationSequence::new()
        .then(SimplificationStep::Algebraic)
        .max_iterations(1);
    assert_eq!(simplify(&program, &input), base.pow(Expr::sum(exponents)));
}
