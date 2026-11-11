//! Certified GCD cancellation and checked sparse arithmetic.
//!
//! Helpers: `polynomial` builds an integer polynomial from exponent vectors;
//! `cartesian` multiplies by collecting every term product; `canonical`
//! checks the content and primitive-part invariants; `check` cancels
//! `G*P / (G*Q)` and verifies the cross product and that `G` divides the
//! removed factor.

use core::num::NonZeroU64;

use alloc::{vec, vec::Vec};

use num_anafis::RationalType;
use proptest::prelude::*;

use super::{Monomial, PolynomialGcd, PolynomialWork, SparsePolynomial};

fn polynomial(
    dimensions: usize,
    terms: &[(i64, &[u32])],
    work: &mut PolynomialWork,
) -> SparsePolynomial {
    SparsePolynomial::from_terms(
        dimensions,
        terms
            .iter()
            .map(|(value, powers)| {
                let monomial =
                    Monomial::from_fn(dimensions, |index| powers.get(index).copied().unwrap_or(0));
                (monomial, RationalType::from_integer(*value))
            })
            .collect(),
        work,
    )
    .expect("work")
    .expect("capacity")
}

fn cartesian(
    left: &SparsePolynomial,
    right: &SparsePolynomial,
    work: &PolynomialWork,
) -> Option<SparsePolynomial> {
    let dimensions = left.dimensions();
    let (left_content, left_primitive) = left.clone().into_primitive();
    let (right_content, right_primitive) = right.clone().into_primitive();
    let mut pairs = Vec::new();
    for (first, first_value) in left_primitive.terms() {
        for (second, second_value) in right_primitive.terms() {
            pairs.push((
                first.sum(second, dimensions, work).ok()?,
                i128::from(*first_value).checked_mul(i128::from(*second_value))?,
            ));
        }
    }
    SparsePolynomial::from_integers(dimensions, pairs, NonZeroU64::MIN)?
        .checked_scale(&left_content.checked_mul(&right_content)?)
}

fn canonical(polynomial: &SparsePolynomial) -> bool {
    let terms = polynomial.terms();
    let mut common = 0_u64;
    for (_, value) in terms {
        let mut remainder = value.unsigned_abs();
        while let Some(next) = common.checked_rem(remainder) {
            common = remainder;
            remainder = next;
        }
    }
    terms.is_sorted_by(|(first, _), (second, _)| first > second)
        && terms
            .iter()
            .all(|(_, value)| *value != 0 && *value != i64::MIN)
        && terms
            .first()
            .is_none_or(|(_, lead)| *lead > 0 && common == 1)
}

fn check(
    common: &SparsePolynomial,
    first: &SparsePolynomial,
    second: &SparsePolynomial,
    work: &mut PolynomialWork,
) {
    let left = common
        .checked_mul(first, work)
        .expect("work")
        .expect("capacity");
    let right = common
        .checked_mul(second, work)
        .expect("work")
        .expect("capacity");
    let result = PolynomialGcd::cancel(&left, &right, work)
        .expect("work")
        .expect("certified");
    let cross_left = left
        .checked_mul(&result.denominator, work)
        .expect("work")
        .expect("capacity");
    let cross_right = right
        .checked_mul(&result.numerator, work)
        .expect("work")
        .expect("capacity");
    assert!(cross_left == cross_right, "P*Q' = Q*P'");
    for polynomial in [&result.common, &result.numerator, &result.denominator] {
        assert!(canonical(polynomial), "results keep the canonical form");
    }
    let (_, removed) = result.common.into_primitive();
    let (_, expected) = common.clone().into_primitive();
    assert!(
        removed.divide(&expected, work).expect("work").is_some(),
        "the removed factor is a multiple of the common factor"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn random_common_factors_are_removed(
        dimensions in 1_usize..=4,
        factors in prop::array::uniform3(prop::collection::vec(
            (-9_i64..=9, prop::array::uniform4(0_u32..=3)),
            1..=4,
        )),
    ) {
        let mut work = PolynomialWork::new(usize::MAX);
        let [common, first, second] = factors.map(|terms| {
            let mut borrowed: Vec<(i64, &[u32])> =
                terms.iter().map(|(value, powers)| (*value, &powers[..])).collect();
            // A unit constant term keeps every factor nonzero.
            borrowed.push((1, &[]));
            let result = polynomial(dimensions, &borrowed, &mut work);
            if result.is_zero() {
                SparsePolynomial::one(dimensions)
            } else {
                result
            }
        });
        check(&common, &first, &second, &mut work);
    }

    /// Small shapes take the dense box or the heap merge.
    #[test]
    fn products_equal_their_collected_cartesian_terms(
        dimensions in 1_usize..=6,
        factors in prop::array::uniform2((
            prop::collection::vec((-9_i64..=9, prop::collection::vec(0_u32..=3, 6)), 0..=12),
            1_i64..=6,
        )),
    ) {
        let mut work = PolynomialWork::new(usize::MAX);
        let [left, right] = factors.map(|(terms, denominator)| {
            let rational: Vec<_> = terms
                .iter()
                .map(|(value, powers)| {
                    let monomial = Monomial::from_fn(dimensions, |index| {
                        powers.get(index).copied().unwrap_or(0)
                    });
                    let coefficient = RationalType::from_integer(*value)
                        .checked_div(&RationalType::from_integer(denominator))
                        .expect("nonzero denominator");
                    (monomial, coefficient)
                })
                .collect();
            SparsePolynomial::from_terms(dimensions, rational, &mut work)
                .expect("work")
                .expect("capacity")
        });
        let product = left.checked_mul(&right, &mut work).expect("work");
        prop_assert!(product == cartesian(&left, &right, &work));
        for polynomial in [Some(&left), Some(&right), product.as_ref()] {
            prop_assert!(polynomial.is_some_and(canonical));
        }
    }

    /// Dense univariate and bivariate lines take the convolution, with signed
    /// coefficients that may leave native capacity.
    #[test]
    fn dense_lines_convolve_like_collected_terms(
        dimensions in 1_usize..=2,
        factors in prop::array::uniform2(prop::collection::vec(-(1_i64 << 40)..=(1_i64 << 40), 100..=160)),
        magnitude in 0_u32..=40,
    ) {
        let mut work = PolynomialWork::new(usize::MAX);
        let [left, right] = factors.map(|values| {
            let terms: Vec<(i64, Vec<u32>)> = values
                .iter()
                .zip(0_u32..)
                .map(|(value, index)| {
                    let powers = if dimensions == 1 {
                        vec![index]
                    } else {
                        vec![index.div_euclid(11), index.rem_euclid(11)]
                    };
                    let scaled = value.checked_shr(40_u32.saturating_sub(magnitude)).unwrap_or(0);
                    (scaled, powers)
                })
                .collect();
            let borrowed: Vec<(i64, &[u32])> =
                terms.iter().map(|(value, powers)| (*value, &powers[..])).collect();
            polynomial(dimensions, &borrowed, &mut work)
        });
        let product = left.checked_mul(&right, &mut work).expect("work");
        prop_assert!(product == cartesian(&left, &right, &work));
        prop_assert!(product.as_ref().is_none_or(canonical));
    }

    #[test]
    fn sums_equal_their_rational_terms(
        addends in prop::collection::vec(
            prop::collection::vec((-9_i64..=9, 1_i64..=12, 0_u32..=4), 0..=6),
            0..=5,
        ),
    ) {
        let mut work = PolynomialWork::new(usize::MAX);
        let mut all = Vec::new();
        let mut polynomials = Vec::new();
        for terms in &addends {
            let rational: Vec<_> = terms
                .iter()
                .map(|(numerator, denominator, power)| {
                    let coefficient = RationalType::from_integer(*numerator)
                        .checked_div(&RationalType::from_integer(*denominator))
                        .expect("nonzero denominator");
                    (Monomial::from_fn(1, |_| *power), coefficient)
                })
                .collect();
            all.extend(rational.iter().cloned());
            polynomials.push(
                SparsePolynomial::from_terms(1, rational, &mut work)
                    .expect("work")
                    .expect("capacity"),
            );
        }
        let borrowed: Vec<&SparsePolynomial> = polynomials.iter().collect();
        let sum = SparsePolynomial::sum(1, &borrowed, &mut work).expect("work");
        let expected = SparsePolynomial::from_terms(1, all, &mut work).expect("work");
        prop_assert!(sum == expected);
        prop_assert!(sum.as_ref().is_some_and(canonical));
    }
}

#[test]
fn coefficient_overflow_declines_instead_of_panicking() {
    let mut work = PolynomialWork::new(1_000_000);
    let large = SparsePolynomial::constant(RationalType::from_integer(i64::MAX), 1, &mut work)
        .expect("constant");
    let shifted = polynomial(1, &[(1, &[1]), (i64::MAX, &[])], &mut work);
    assert!(
        shifted
            .checked_mul(&shifted, &mut work)
            .expect("work")
            .is_none()
    );
    assert!(large.checked_pow(2, &mut work).expect("work").is_none());
    assert!(
        SparsePolynomial::sum(1, &[&large, &large], &mut work)
            .expect("work")
            .is_none()
    );
}

#[test]
fn nonmonomial_leading_coefficients_use_free_image_scales() {
    // Every variable has a two-term leading coefficient in `xy + xz + yz + 1`,
    // so sparse images need the scales of de Kleine, Monagan, and Wittkopf.
    let mut work = PolynomialWork::new(usize::MAX);
    let common = polynomial(
        3,
        &[(1, &[1, 1, 0]), (1, &[1, 0, 1]), (1, &[0, 1, 1]), (1, &[])],
        &mut work,
    );
    let first = polynomial(
        3,
        &[(1, &[2, 0, 0]), (2, &[0, 1, 0]), (3, &[0, 0, 2]), (3, &[])],
        &mut work,
    );
    let second = polynomial(3, &[(1, &[1, 0, 1]), (1, &[0, 2, 0]), (5, &[])], &mut work);
    check(&common, &first, &second, &mut work);
}

#[test]
fn coefficients_beyond_one_prime_combine_two_images() {
    let mut work = PolynomialWork::new(usize::MAX);
    let common = polynomial(2, &[(1, &[1, 0]), (1 << 55, &[0, 1]), (3, &[])], &mut work);
    let first = polynomial(2, &[(1, &[1, 0]), (1, &[])], &mut work);
    let second = polynomial(2, &[(1, &[0, 1]), (1, &[])], &mut work);
    check(&common, &first, &second, &mut work);
}

#[test]
fn exponent_strides_deflate_before_interpolation() {
    let mut work = PolynomialWork::new(usize::MAX);
    let common = polynomial(1, &[(1, &[2]), (-1, &[])], &mut work);
    let first = polynomial(1, &[(1, &[4]), (1, &[2]), (1, &[])], &mut work);
    let second = polynomial(1, &[(1, &[2]), (1, &[])], &mut work);
    check(&common, &first, &second, &mut work);
}

#[test]
fn coprime_inputs_keep_a_unit_factor() {
    let mut work = PolynomialWork::new(usize::MAX);
    let left = polynomial(
        3,
        &[(1, &[5, 1, 0]), (1, &[3, 0, 2]), (1, &[0, 1, 1]), (3, &[])],
        &mut work,
    );
    let right = polynomial(
        3,
        &[(1, &[4, 0, 1]), (1, &[0, 3, 1]), (1, &[1, 2, 0]), (2, &[])],
        &mut work,
    );
    let result = PolynomialGcd::cancel(&left, &right, &mut work)
        .expect("work")
        .expect("certified");
    assert!(result.common.is_constant());
}

#[test]
fn larger_trivariate_factors_cancel() {
    let mut work = PolynomialWork::new(usize::MAX);
    let common = polynomial(
        3,
        &[
            (1, &[3, 0, 0]),
            (1, &[0, 3, 0]),
            (1, &[0, 0, 3]),
            (1, &[1, 1, 1]),
            (1, &[]),
        ],
        &mut work,
    )
    .checked_pow(2, &mut work)
    .expect("work")
    .expect("capacity");
    let first = polynomial(
        3,
        &[(1, &[1, 0, 0]), (2, &[0, 1, 0]), (3, &[0, 0, 1])],
        &mut work,
    );
    let second = polynomial(3, &[(1, &[1, 1, 0]), (1, &[0, 0, 2]), (1, &[])], &mut work)
        .checked_pow(2, &mut work)
        .expect("work")
        .expect("capacity");
    check(&common, &first, &second, &mut work);
}
