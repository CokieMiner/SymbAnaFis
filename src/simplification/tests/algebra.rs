//! Polynomial transformations and exact regression properties.

use core::ops::{Add, Div, Mul, Sub};

use alloc::{format, vec};

use proptest::{
    collection, prelude::ProptestConfig, prop_assert, prop_assert_eq, prop_assume, prop_oneof,
    proptest,
};

use crate::{
    Context, Expr, ExprView, SimplificationSequence, SimplificationStep, Substitution,
    Substitutions, Symbol,
};

use super::simplify;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn expansion_preserves_integer_polynomials_and_eliminates_distributable_nodes(
        first in -3_i64..=3, second in -3_i64..=3, constant in -3_i64..=3,
        degree in 2_i64..=5, x_value in -3_i64..=3, y_value in -3_i64..=3,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let polynomial = Expr::sum(vec![Mul::mul(x, first), Mul::mul(y, second), Expr::number(constant)]);
        let input = Mul::mul(polynomial.pow(degree), Add::add(x, y));
        let program = SimplificationSequence::new().then(SimplificationStep::Expand)
            .target(crate::SimplificationTarget::Expanded);
        let result = simplify(&program, &input);
        let substitutions = Substitutions::new([(x.into(), Expr::number(x_value)), (y.into(), Expr::number(y_value))])?;
        prop_assert_eq!(substitutions.apply_all(&input), substitutions.apply_all(&result));
        prop_assert!(result.fold(true, |valid, node| valid && match node.view() {
            ExprView::Product(factors) => !factors.iter().any(|factor| matches!(factor.view(), ExprView::Sum(_))),
            ExprView::Pow(base, exponent) => !matches!(base.view(), ExprView::Sum(_)) || !exponent.as_number().is_some_and(|value| value.is_int() && value.to_int() > 1),
            ExprView::Number(_) | ExprView::ImaginaryUnit | ExprView::Pi | ExprView::Euler
            | ExprView::Symbol(_) | ExprView::Function { .. } | ExprView::Sum(_)
            | ExprView::Div(..) | ExprView::Derivative { .. } => true,
        }), "expanded output contains no distributable arithmetic nodes");
    }

    #[test]
    fn expansion_equals_canonical_distribution(
        left in collection::vec((prop_oneof![-4_i64..=-1, 1_i64..=4], 0_i64..=2, 0_i64..=2), 1..=4),
        right in collection::vec((prop_oneof![-4_i64..=-1, 1_i64..=4], 0_i64..=2, 0_i64..=2), 2..=4),
        degree in 1_i64..=3, kind in 0_u8..3,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        // Symbols and pi take the sparse polynomial path; powers of e become
        // `exp` calls and take the expression path.
        let base = match kind {
            0 => context.symb("y")?.to_expr(),
            1 => Expr::pi(),
            _ => Expr::e(),
        };
        // A zero exponent is omitted: `x^0` keeps the exclusion at zero.
        let build = |terms: &[(i64, i64, i64)]| Expr::sum(terms.iter().map(|&(coefficient, first, second)| {
            let mut factors = vec![Expr::number(coefficient)];
            factors.extend((first > 0).then(|| x.pow(first)));
            factors.extend((second > 0).then(|| base.pow(second)));
            Expr::product(factors)
        }).collect());
        let distribute = |product: &Expr, factor: &Expr| {
            let terms = |operand: &Expr| if matches!(operand.view(), ExprView::Sum(_)) { operand.children().cloned().collect() } else { vec![operand.clone()] };
            let factor_terms: alloc::vec::Vec<Expr> = terms(factor);
            Expr::sum(terms(product).iter().flat_map(|term| factor_terms.iter().map(move |other| Mul::mul(term, other))).collect())
        };
        // Canonical sums keep zero-coefficient terms; polynomial products drop
        // them, which is exact for the finite bases they accept.
        let nonzero = |sum: &Expr| if matches!(sum.view(), ExprView::Sum(_)) {
            Expr::sum(sum.children().filter(|term| !(term.is_zero_num() || matches!(term.view(),
                ExprView::Product(factors) if factors.first().is_some_and(Expr::is_zero_num)))).cloned().collect())
        } else {
            sum.clone()
        };
        let (first, second) = (build(&left), build(&right));
        // A power of a single term is canonical construction, not expansion.
        prop_assume!(matches!(second.view(), ExprView::Sum(_)));
        let mut expected = first.clone();
        for _ in 0..degree {
            expected = distribute(&expected, &second);
        }
        let program = SimplificationSequence::new().then(SimplificationStep::Expand)
            .target(crate::SimplificationTarget::Expanded);
        let output = simplify(&program, &Mul::mul(&first, second.pow(degree)));
        prop_assert_eq!(nonzero(&output), nonzero(&expected));
    }

    #[test]
    fn factoring_extracts_common_powers_and_numeric_gcd_without_symbolic_division(
        first in 1_i64..=6, second in 1_i64..=6, degree in 1_i64..=5,
        x_value in -3_i64..=3, y_value in -3_i64..=3,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let input = Add::add(Mul::mul(Mul::mul(x.pow(degree), y), first), Mul::mul(x.pow(degree), second));
        let output = simplify(&SimplificationSequence::factored(), &input);
        let substitutions = Substitutions::new([(x.into(), Expr::number(x_value)), (y.into(), Expr::number(y_value))])?;
        prop_assert_eq!(substitutions.apply_all(&input), substitutions.apply_all(&output));
        prop_assert!(!output.fold(false, |found, node| found || matches!(node.view(), ExprView::Div(..))));
        prop_assert_eq!(&output, &simplify(&SimplificationSequence::factored(), &output));
    }

    #[test]
    fn factoring_preserves_exact_values_of_sparse_polynomials(
        terms in collection::vec((-6_i64..=6, 0_i64..=3, 0_i64..=3, 0_i64..=2), 1..=8),
        x_value in -3_i64..=3, y_value in -3_i64..=3, z_value in -3_i64..=3,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let z = context.symb("z")?;
        let input = Expr::sum(terms.iter().map(|&(coefficient, first, second, third)| {
            Expr::product(vec![Expr::number(coefficient), x.pow(first), y.pow(second), z.pow(third)])
        }).collect());
        let substitutions = Substitutions::new([(x.into(), Expr::number(x_value)), (y.into(), Expr::number(y_value)), (z.into(), Expr::number(z_value))])?;
        let expected = substitutions.apply_all(&input);
        for sequence in [SimplificationSequence::factored(), SimplificationSequence::compilation()] {
            let output = simplify(&sequence, &input);
            prop_assert_eq!(substitutions.apply_all(&output), expected.clone(), "output={}", output);
        }
    }

    #[test]
    fn fraction_addition_preserves_exact_values_and_denominator_exclusions(
        a in -9_i64..=9, b in 1_i64..=9, c in -9_i64..=9, d in 1_i64..=9,
    ) {
        let context = Context::new()?;
        let numerator = context.symb("a")?;
        let denominator = context.symb("b")?;
        let other_top = context.symb("c")?;
        let other_bottom = context.symb("d")?;
        let input = Add::add(Div::div(numerator, denominator), Div::div(other_top, other_bottom));
        let sequence = SimplificationSequence::new().then(SimplificationStep::Algebraic)
            .score_with(|node| node.fold(0_usize, |count, part| count.saturating_add(usize::from(matches!(part.view(), ExprView::Div(..))))));
        let output = simplify(&sequence, &input);
        let substitutions = Substitutions::new([(numerator.into(), Expr::number(a)), (denominator.into(), Expr::number(b)), (other_top.into(), Expr::number(c)), (other_bottom.into(), Expr::number(d))])?;
        prop_assert_eq!(substitutions.apply_all(&input), substitutions.apply_all(&output));
        for symbol in [denominator, other_bottom] {
            let excluded = Substitution::new(symbol.into(), Expr::number(0)).apply_all(&output);
            prop_assert_eq!(excluded.properties().get(crate::Property::Finite), crate::Truth::False, "both original zero exclusions survive fraction addition");
        }
    }
}

#[test]
fn factoring_extracts_all_global_symbolic_powers_in_one_pass() {
    let context = Context::new().expect("context");
    let x = context.symb("batch_factor_x").expect("symbol");
    let y = context.symb("batch_factor_y").expect("symbol");
    let z = context.symb("batch_factor_z").expect("symbol");
    let u = context.symb("batch_factor_u").expect("symbol");
    let input = Expr::sum(vec![
        Mul::mul(Mul::mul(x.pow(2), &y), &z),
        Mul::mul(Mul::mul(x.pow(3), y.pow(2)), &u),
        Mul::mul(Mul::mul(x.pow(2), &y), z.pow(2)),
    ]);
    let expected = Mul::mul(
        Mul::mul(x.pow(2), &y),
        Expr::sum(vec![
            Mul::mul(z.to_expr(), Add::add(z, 1)),
            Mul::mul(x, Mul::mul(y, &u)),
        ]),
    );
    let sequence = SimplificationSequence::factored();
    assert_eq!(simplify(&sequence, &input), simplify(&sequence, &expected));
}

#[test]
fn perfect_powers_and_differences_have_factored_presets() {
    let context = Context::new().expect("context");
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
fn sum_and_difference_of_cubes_are_factored() {
    let x = Symbol::intern("cube_x").expect("symbol");
    let y = Symbol::intern("cube_y").expect("symbol");
    let expanded = SimplificationSequence::expanded();
    let factored = SimplificationSequence::factored();
    for (source, expected) in [
        (
            Add::add(x.pow(3), y.pow(3)),
            Mul::mul(
                Add::add(&x, &y),
                Add::add(Sub::sub(x.pow(2), Mul::mul(&x, &y)), y.pow(2)),
            ),
        ),
        (
            Sub::sub(x.pow(3), y.pow(3)),
            Mul::mul(
                Sub::sub(&x, &y),
                Add::add(Add::add(x.pow(2), Mul::mul(&x, &y)), y.pow(2)),
            ),
        ),
    ] {
        let result = simplify(&factored, &source);
        assert!(matches!(result.view(), ExprView::Product(_)));
        assert_eq!(simplify(&expanded, &result), simplify(&expanded, &expected));
    }
}

#[test]
fn power_collection_finishes_long_products_in_one_pass() {
    let context = Context::new().expect("context");
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
