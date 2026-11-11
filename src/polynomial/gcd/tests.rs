//! Sparse interpolation of a known support.
//!
//! Helpers: `monomial` builds an exponent vector from two exponents; `image`
//! evaluates one coefficient vector per group at the nodes `m^l` for
//! `l = 1, 2, ...`, returning one contiguous series per group.

use alloc::{vec, vec::Vec};

use ntt_anafis::{PrimeField, Residue};

use super::{Monomial, Shape};

fn monomial(first: u32, second: u32) -> Monomial {
    Monomial::from_fn(2, |index| if index == 0 { first } else { second })
}

/// `y_{g,l} = sum_i c_{g,i} m_{g,i}^l` for `l = 1..=count`, one contiguous
/// series per group.
fn image(field: PrimeField, groups: &[(&[Residue], &[Residue])], count: u64) -> Vec<Vec<Residue>> {
    groups
        .iter()
        .map(|(nodes, coefficients)| {
            (1..=count)
                .map(|power| {
                    nodes.iter().zip(*coefficients).fold(
                        Residue::ZERO,
                        |sum, (&node, &coefficient)| {
                            field.add(sum, field.mul(coefficient, field.pow(node, power)))
                        },
                    )
                })
                .collect()
        })
        .collect()
}

#[test]
fn a_single_leading_monomial_fixes_the_scales() {
    let Some(field) = PrimeField::catalog().next() else {
        return;
    };
    let beta = field.reduce_u64(7);
    // Descending support: `x^2` alone in the leading group, then `xy` and `x`.
    let support = [
        (monomial(2, 0), Residue::ONE),
        (monomial(1, 1), Residue::ONE),
        (monomial(1, 0), Residue::ONE),
    ];
    let shape = Shape::new(&support);
    assert!(shape.single, "the leading group holds one monomial");
    assert_eq!(shape.count, Some(2), "the widest block has two monomials");
    // The nodes are the monomials at `beta`: `y^0 = 1` and `y^1 = beta`.
    let nodes = [vec![Residue::ONE], vec![beta, Residue::ONE]];
    let (first, second, third) = (
        field.reduce_u64(3),
        field.reduce_u64(11),
        field.reduce_u64(5),
    );
    let rows = image(
        field,
        &[
            (&[Residue::ONE], &[first]),
            (&[beta, Residue::ONE], &[second, third]),
        ],
        2,
    );
    let terms = shape.solve(field, &nodes, &rows).expect("solved");
    let expected = [
        (monomial(2, 0), first),
        (monomial(1, 1), second),
        (monomial(1, 0), third),
    ];
    assert!(terms == expected, "the image recovers its coefficients");
}

#[test]
fn free_scales_recover_coefficients_up_to_leading_normalization() {
    let Some(field) = PrimeField::catalog().next() else {
        return;
    };
    let beta = field.reduce_u64(13);
    let beta_sq = field.mul(beta, beta);
    // Two groups of two monomials each: `(x^2 y + x^2) + (x y^2 + x)`.
    let support = [
        (monomial(2, 1), Residue::ONE),
        (monomial(2, 0), Residue::ONE),
        (monomial(1, 2), Residue::ONE),
        (monomial(1, 0), Residue::ONE),
    ];
    let shape = Shape::new(&support);
    assert!(!shape.single, "the leading group holds two monomials");
    let count = shape.count.expect("free scale count");
    let nodes = [vec![beta, Residue::ONE], vec![beta_sq, Residue::ONE]];
    let [c0, c1, c2, c3] = [
        field.reduce_u64(5),
        field.reduce_u64(7),
        field.reduce_u64(11),
        field.reduce_u64(17),
    ];
    let coeffs = [c0, c1, c2, c3];
    let mut series = image(
        field,
        &[
            (&[beta, Residue::ONE], &[c0, c1]),
            (&[beta_sq, Residue::ONE], &[c2, c3]),
        ],
        u64::try_from(count).expect("small count"),
    );
    // Normalize each image by its own leading group value, matching `Dense::gcd`.
    let lead_series = series.first().expect("leading series").clone();
    for (step, &lead_val) in lead_series.iter().enumerate() {
        let inv = field.inv(lead_val).expect("nonzero lead");
        for group in &mut series {
            if let Some(slot) = group.get_mut(step) {
                *slot = field.mul(*slot, inv);
            }
        }
    }
    let terms = shape.solve(field, &nodes, &series).expect("solved");
    assert_eq!(terms.len(), 4, "all four coefficients recover");
    let first_actual = terms.first().expect("first term").1;
    let scale = field.mul(first_actual, field.inv(c0).expect("nonzero"));
    for ((_, actual), &expected) in terms.iter().zip(&coeffs) {
        assert_eq!(
            *actual,
            field.mul(expected, scale),
            "free-scale solution matches true coefficients up to one scalar"
        );
    }
}
