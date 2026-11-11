//! Principal-branch counterexamples and square-root factor extraction.
//!
//! `roots` and `logarithms` return the output of one `Roots` or `Exponential`
//! step, whichever its score.

use core::ops::{Add, Div, Mul, Sub};

use alloc::vec;

use num_anafis::Number;
use proptest::{bool, prop_assert, proptest};

use crate::{
    Context, DomainSafety, Expr, ExprView, SimplificationSequence, SimplificationStep,
    Substitutions,
};

use super::{evaluate, simplify};

fn step_output(step: SimplificationStep, input: &Expr, safety: DomainSafety) -> Expr {
    // Any changed candidate outranks the input.
    let original = input.clone();
    let sequence = SimplificationSequence::new()
        .then(step)
        .domain_safety(safety)
        .score_with(move |candidate| usize::from(*candidate == original));
    simplify(&sequence, input)
}

fn roots(input: &Expr, safety: DomainSafety) -> Expr {
    step_output(SimplificationStep::Roots, input, safety)
}

fn logarithms(input: &Expr, safety: DomainSafety) -> Expr {
    step_output(SimplificationStep::Exponential, input, safety)
}

proptest! {
    #[test]
    fn literal_root_products_and_logarithm_sums_keep_their_principal_value(
        first in (-2_i64..=2, -2_i64..=2),
        second in (-2_i64..=2, -2_i64..=2),
        third in (-2_i64..=2, -2_i64..=2),
        assumed in bool::ANY,
    ) {
        // Literal arguments decide the winding, so neither mode assumes it.
        let literal = |(real, imaginary): (i64, i64)| {
            Expr::number(Number::from_complex(Number::from(real), Number::from(imaginary)))
        };
        let (a, b, c) = (literal(first), literal(second), literal(third));
        let safety = if assumed { DomainSafety::AssumptionDependent } else { DomainSafety::Preserving };
        let substitutions = Substitutions::default();
        let pair = Mul::mul(a.sqrt(), b.sqrt());
        let mut inputs = vec![(pair.clone(), true), (Mul::mul(&pair, c.sqrt()), true)];
        if third != (0, 0) {
            inputs.push((Div::div(a.sqrt(), c.sqrt()), true));
            inputs.push((Div::div(&pair, c.sqrt()), true));
        }
        if [first, second, third].iter().all(|value| *value != (0, 0)) {
            let sum = Add::add(a.ln(), b.ln());
            inputs.push((sum.clone(), false));
            inputs.push((Add::add(&sum, c.ln()), false));
            inputs.push((Sub::sub(&sum, c.ln()), false));
            inputs.push((Sub::sub(a.log2(), c.log2()), false));
        }
        for (input, root) in inputs {
            let output = if root { roots(&input, safety) } else { logarithms(&input, safety) };
            let before = evaluate(&input, &substitutions);
            let after = evaluate(&output, &substitutions);
            prop_assert!(before.approx_eq_number(&after, &Number::from_float(1.0e-12)), "{input} -> {output}: {before} != {after}");
        }
    }

    #[test]
    fn square_factors_leave_roots_of_real_bases_and_square_coefficients(
        power in 2_i64..=5,
        coefficient in 0_usize..6,
        offset in -2.0_f64..2.0,
        real in -2.0_f64..2.0,
        imaginary in -2.0_f64..2.0,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        // |x| - 2 is real with either sign; y is any finite complex value.
        let base = Sub::sub(x.to_expr().abs(), 2);
        let scale = [(1, 4), (-1, 4), (9, 1), (-9, 1), (4, 9), (2, 1)]
            .get(coefficient)
            .map_or_else(|| Expr::number(1), |(top, bottom)| Div::div(Expr::number(*top), Expr::number(*bottom)));
        let input = Expr::product(vec![scale, base.pow(power), y.to_expr()]).sqrt();
        let output = roots(&input, DomainSafety::Preserving);
        let substitutions = Substitutions::new([
            (x.into(), Expr::number(offset)),
            (y.into(), Expr::number(Number::from_complex(Number::from_float(real), Number::from_float(imaginary)))),
        ])?;
        let before = evaluate(&input, &substitutions);
        let after = evaluate(&output, &substitutions);
        prop_assert!(before.approx_eq_number(&after, &Number::from_float(1.0e-12)), "{input} -> {output}: {before} != {after}");
    }
}

#[test]
fn only_stored_square_roots_combine() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol").to_expr();
    let y = context.symb("y").expect("symbol").to_expr();
    let positive = x.abs() + 1;
    for input in [
        Expr::pi() * positive.sqrt(),
        (Expr::pi() * &y).sqrt(),
        (2 * Expr::pi() * y.pow(2)).sqrt(),
    ] {
        assert_eq!(roots(&input, DomainSafety::Preserving), input);
    }
    assert_eq!(
        roots(
            &(Expr::pi().sqrt() * positive.sqrt()),
            DomainSafety::Preserving
        ),
        (Expr::pi() * &positive).sqrt()
    );
    // A real negative x refutes x*sqrt(y) = |x|*sqrt(y).
    let unmatched = &x * y.sqrt();
    assert_eq!(
        roots(&unmatched, DomainSafety::AssumptionDependent),
        unmatched
    );
    let density = context
        .parse("exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)")
        .expect("valid density formula");
    let expanded = context
        .parse("exp(-(x^2 - 2*mu*x + mu^2) / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)")
        .expect("valid density formula");
    assert_eq!(
        simplify(&SimplificationSequence::expanded(), &density),
        expanded
    );
}

#[test]
fn branch_identities_follow_the_principal_phase_sum() {
    let x = Expr::symbol("phase_root_x").expect("symbol");
    let y = Expr::symbol("phase_root_y").expect("symbol");
    let preserving = DomainSafety::Preserving;
    let assumed = DomainSafety::AssumptionDependent;
    let positive = x.abs() + 1;
    let other = y.abs() + 1;
    let (negative, opposite) = (-x.abs() - 1, -y.abs() - 1);
    let turn = Expr::product(vec![Expr::number(2), Expr::pi(), Expr::i()]);
    // One nonnegative argument fixes the phase sum to the other phase.
    assert_eq!(
        roots(&(positive.sqrt() * y.sqrt()), preserving),
        (&positive * &y).sqrt()
    );
    assert_eq!(
        logarithms(&(x.ln() + Expr::number(2).ln()), preserving),
        (2 * &x).ln()
    );
    // Unknown phases join only by assumption.
    let unknown = x.sqrt() * y.sqrt();
    assert_eq!(roots(&unknown, preserving), unknown);
    assert_eq!(roots(&unknown, assumed), (&x * &y).sqrt());
    let unknown_sum = x.ln() + y.ln();
    assert_eq!(logarithms(&unknown_sum, preserving), unknown_sum);
    assert_eq!(logarithms(&unknown_sum, assumed), (&x * &y).ln());
    // Two negative arguments sum to 2*pi, which fixes the winding at one.
    for safety in [preserving, assumed] {
        assert_eq!(
            roots(&(negative.sqrt() * opposite.sqrt()), safety),
            -(&negative * &opposite).sqrt()
        );
        assert_eq!(
            logarithms(&(negative.ln() + opposite.ln()), safety),
            (&negative * &opposite).ln() + &turn
        );
    }
    // Equal coefficients combine as one group.
    assert_eq!(
        logarithms(&(2 * positive.ln() + 2 * other.ln()), preserving),
        2 * (&positive * &other).ln()
    );
    // Arguments that differ by a positive factor have one phase, which
    // cancels; a subtracted argument still needs a nonzero proof.
    let nonzero = y.exp();
    let difference = (2 * &nonzero).ln() - nonzero.ln();
    let combined = logarithms(&difference, preserving);
    assert!(
        matches!(combined.view(), ExprView::Function { name, .. } if name.as_str() == "ln"),
        "{combined}"
    );
    assert_eq!(
        simplify(&SimplificationSequence::compact(), &difference),
        Expr::number(2).ln()
    );
    let unproved = (2 * &x).ln() - x.ln();
    assert_eq!(logarithms(&unproved, preserving), unproved);
    assert_eq!(logarithms(&unproved, assumed), (2 * &x / &x).ln());
    // A quotient subtracts phases; a negative real denominator reaches -pi.
    let two = Expr::number(2);
    assert_eq!(
        roots(&(x.sqrt() / two.sqrt()), preserving),
        (&x / &two).sqrt()
    );
    let reciprocal = two.sqrt() / x.sqrt();
    assert_eq!(roots(&reciprocal, preserving), reciprocal);
    assert_eq!(roots(&reciprocal, assumed), (&two / &x).sqrt());
}

#[test]
fn roots_of_products_extract_every_square_factor() {
    let x = Expr::symbol("square_factor_x").expect("symbol");
    let y = Expr::symbol("square_factor_y").expect("symbol");
    let preserving = DomainSafety::Preserving;
    // A positive base leaves without an absolute value; any power splits.
    assert_eq!(
        roots(&(&y * Expr::pi().pow(3)).sqrt(), preserving),
        Expr::pi() * (Expr::pi() * &y).sqrt()
    );
    // Square coefficients leave with their magnitude; the sign stays inside.
    assert_eq!(roots(&(4 * &y).sqrt(), preserving), 2 * y.sqrt());
    assert_eq!(roots(&(-4 * &y).sqrt(), preserving), 2 * (-&y).sqrt());
    let unknown = (x.pow(2) * &y).sqrt();
    assert_eq!(roots(&unknown, preserving), unknown);
    assert_eq!(
        roots(&unknown, DomainSafety::AssumptionDependent),
        x.abs() * y.sqrt()
    );
}
