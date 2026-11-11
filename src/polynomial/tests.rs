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

use crate::core::{Expr, Symbol};

use super::{Monomial, PolynomialGcd, PolynomialInput, PolynomialWork, SparsePolynomial};

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
                first.sum(second, work).ok()?,
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

#[test]
fn multivariate_gcd_round_trips_through_coprime_cofactors() {
    let mut work = PolynomialWork::new(usize::MAX);
    let common = polynomial(3, &[(1, &[2, 0, 0]), (1, &[0, 1, 1]), (2, &[])], &mut work);
    let first = polynomial(3, &[(1, &[1, 0, 0]), (1, &[0, 1, 0]), (1, &[])], &mut work);
    let second = polynomial(3, &[(1, &[0, 0, 1]), (2, &[])], &mut work);
    let left = common
        .checked_mul(&first, &mut work)
        .expect("work")
        .expect("capacity");
    let right = common
        .checked_mul(&second, &mut work)
        .expect("work")
        .expect("capacity");
    let result = PolynomialGcd::cancel(&left, &right, &mut work)
        .expect("work")
        .expect("certified");
    let (_, removed) = result.common.into_primitive();
    let (_, expected) = common.into_primitive();
    assert!(removed == expected, "the GCD comes back");
}

/// The content is the gcd of the coefficients, signed by the leading one, and
/// coefficients that only fit `i128` still reduce to native quotients.
#[test]
fn integer_terms_reduce_to_a_primitive_part_in_either_width() {
    let square = Monomial::from_indexed(1, [(0, 2)]).expect("one variable");
    let linear = Monomial::from_indexed(1, [(0, 1)]).expect("one variable");
    let unit = Monomial::from_indexed(1, []).expect("one variable");
    let narrow = SparsePolynomial::from_integers(
        1,
        vec![
            (square.clone(), -6),
            (linear.clone(), 4),
            (unit.clone(), -2),
        ],
        NonZeroU64::MIN,
    )
    .expect("capacity");
    let (content, primitive) = narrow.into_primitive();
    assert!(
        content == RationalType::from_integer(-2),
        "the content is the gcd, signed by the leading coefficient"
    );
    assert!(
        primitive
            .terms()
            .iter()
            .map(|(_, value)| *value)
            .eq([3, -2, 1]),
        "the primitive part divides every coefficient by the content"
    );

    let wide = SparsePolynomial::from_integers(
        1,
        vec![
            (square.clone(), 1_i128 << 70),
            (linear, (1_i128 << 70) | (1_i128 << 40)),
            (unit.clone(), 1_i128 << 70),
        ],
        NonZeroU64::MIN,
    )
    .expect("capacity");
    let (wide_content, wide_primitive) = wide.into_primitive();
    assert!(
        wide_content == RationalType::from_integer(1_i64 << 40),
        "the content is the gcd of coefficients beyond native width"
    );
    assert!(
        wide_primitive.terms().iter().map(|(_, value)| *value).eq([
            1_i64 << 30,
            (1_i64 << 30) | 1,
            1_i64 << 30
        ]),
        "quotients of wide coefficients fit native width"
    );

    // A content of $2^{63}$ has no `i64` magnitude, so it takes the wide path
    // instead of declining.
    let extreme = SparsePolynomial::from_integers(
        1,
        vec![(square, i128::from(i64::MIN)), (unit, 1_i128 << 63)],
        NonZeroU64::MIN,
    )
    .expect("capacity");
    let (extreme_content, extreme_primitive) = extreme.into_primitive();
    assert!(
        extreme_content == RationalType::from_integer(i64::MIN),
        "the content is the signed gcd at the native boundary"
    );
    assert!(
        extreme_primitive
            .terms()
            .iter()
            .map(|(_, value)| *value)
            .eq([1, -1]),
        "the primitive part divides by a content beyond native magnitude"
    );
}

#[test]
fn free_scale_images_round_trip_through_coprime_cofactors() {
    // Two groups of two monomials each, so the image scales come from the
    // block recurrences rather than from a single leading monomial.
    let mut work = PolynomialWork::new(usize::MAX);
    let common = polynomial(
        3,
        &[(1, &[1, 1, 0]), (1, &[1, 0, 1]), (1, &[0, 1, 1]), (1, &[])],
        &mut work,
    );
    let first = polynomial(3, &[(1, &[2, 0, 0]), (1, &[0, 1, 0]), (3, &[])], &mut work);
    let second = polynomial(3, &[(1, &[0, 0, 2]), (2, &[1, 0, 0]), (5, &[])], &mut work);
    let left = common
        .checked_mul(&first, &mut work)
        .expect("work")
        .expect("capacity");
    let right = common
        .checked_mul(&second, &mut work)
        .expect("work")
        .expect("capacity");
    let result = PolynomialGcd::cancel(&left, &right, &mut work)
        .expect("work")
        .expect("certified");
    let (_, removed) = result.common.into_primitive();
    let (_, expected) = common.into_primitive();
    assert!(removed == expected, "the GCD comes back");
}

#[test]
fn degree_bounds_cover_asymmetric_inputs() {
    // The left input reaches degree four in `x` and two in `y` while the right
    // reaches one in each, so the per-variable power tables must be sized from
    // the larger degree of the pair.
    let mut work = PolynomialWork::new(usize::MAX);
    let common = polynomial(2, &[(1, &[1, 1]), (1, &[])], &mut work);
    let first = polynomial(2, &[(1, &[3, 0]), (1, &[0, 2]), (1, &[])], &mut work);
    let second = polynomial(2, &[(1, &[1, 0]), (1, &[0, 1]), (1, &[])], &mut work);
    check(&common, &first, &second, &mut work);
}

#[test]
fn a_padding_slot_is_not_a_variable() {
    // Two variables occupy the first two slots of the inline vector; the last
    // two are padding and must not accept an exponent.
    assert!(Monomial::from_indexed(2, [(0, 1), (1, 2)]).is_some());
    for index in 2..4 {
        assert!(
            Monomial::from_indexed(2, [(index, 1)]).is_none(),
            "index {index} is padding for two variables"
        );
    }
    // A repeated variable and a zero exponent stay rejected.
    assert!(Monomial::from_indexed(2, [(0, 1), (0, 2)]).is_none());
    assert!(Monomial::from_indexed(2, [(0, 0)]).is_none());
    // The heap representation has no padding, so the same bound applies.
    assert!(Monomial::from_indexed(5, [(4, 3)]).is_some());
    assert!(Monomial::from_indexed(5, [(5, 3)]).is_none());
}

#[test]
fn squaring_and_powers_match_repeated_multiplication() {
    let mut work = PolynomialWork::new(usize::MAX);
    // Dense univariate square takes the single-line Kronecker convolution path.
    let dense_terms: Vec<(i64, Vec<u32>)> = (0_u32..96)
        .map(|index| (i64::from(index.rem_euclid(7)) - 3, vec![index]))
        .collect();
    let borrowed: Vec<(i64, &[u32])> = dense_terms
        .iter()
        .map(|(value, powers)| (*value, powers.as_slice()))
        .collect();
    let dense = polynomial(1, &borrowed, &mut work);
    let square = dense
        .checked_mul(&dense, &mut work)
        .expect("work")
        .expect("capacity");
    assert!(
        Some(square) == cartesian(&dense, &dense, &work),
        "single-line Kronecker squaring matches cartesian multiplication"
    );

    // Sparse inline square with large exponent gap takes packed `u128` heap merge.
    let sparse = polynomial(
        3,
        &[(2, &[500, 0, 0]), (-3, &[0, 500, 0]), (5, &[0, 0, 500])],
        &mut work,
    );
    let cube = sparse
        .checked_pow(3, &mut work)
        .expect("work")
        .expect("capacity");
    let expected = cartesian(&sparse, &sparse, &work)
        .and_then(|sq| cartesian(&sq, &sparse, &work))
        .expect("capacity");
    assert!(
        cube == expected,
        "sparse packed merge cube matches cartesian"
    );

    // Single-term power scales exponents and rational content directly.
    let monomial = SparsePolynomial::from_terms(
        2,
        vec![(
            Monomial::from_indexed(2, [(0, 2), (1, 3)]).expect("valid"),
            RationalType::from_parts(-3, 2),
        )],
        &mut work,
    )
    .expect("work")
    .expect("capacity");
    let fifth = monomial
        .checked_pow(5, &mut work)
        .expect("work")
        .expect("capacity");
    assert!(
        fifth.terms()
            == [(
                Monomial::from_indexed(2, [(0, 10), (1, 15)]).expect("valid"),
                1
            )],
        "monomial power scales exponents"
    );
    assert!(
        fifth.into_primitive().0 == RationalType::from_parts(-243, 32),
        "monomial power raises rational content"
    );
}

#[test]
fn univariate_gcd_fast_path_handles_coprime_deflated_and_crt_cases() {
    let mut work = PolynomialWork::new(usize::MAX);
    // Coprime univariate pair proves unit GCD on the first prime.
    let left = polynomial(1, &[(1, &[3]), (2, &[1]), (1, &[])], &mut work);
    let right = polynomial(1, &[(1, &[2]), (1, &[])], &mut work);
    let coprime = PolynomialGcd::cancel(&left, &right, &mut work)
        .expect("work")
        .expect("certified");
    assert!(
        coprime.common.is_constant(),
        "coprime univariate GCD is one"
    );

    // Univariate pair requiring two-prime CRT lifting.
    let big_common = polynomial(1, &[(1, &[2]), (1 << 55, &[1]), (3, &[])], &mut work);
    let f1 = polynomial(1, &[(1, &[1]), (1, &[])], &mut work);
    let f2 = polynomial(1, &[(1, &[1]), (-2, &[])], &mut work);
    check(&big_common, &f1, &f2, &mut work);
}

#[test]
fn input_lowering_and_to_expr_preserve_compact_polynomials_with_constants() {
    let x = Symbol::intern("poly_test_x").expect("symbol").to_expr();
    let y = Symbol::intern("poly_test_y").expect("symbol").to_expr();
    // `(x^3 + 2x^2 + x + 1)` has a constant term alongside a `Node::Poly` group.
    let numerator = x.pow(3) + Expr::number(2) * x.pow(2) + x.clone() + Expr::number(1);
    let denominator = x.clone() + Expr::number(1);
    let mut work = PolynomialWork::new(usize::MAX);
    let (top, bottom, bases) = PolynomialInput::pair(&numerator, &denominator, &mut work)
        .expect("work")
        .expect("lowered");
    assert_eq!(top.to_expr(&bases), Some(numerator));
    assert_eq!(bottom.to_expr(&bases), Some(denominator));

    // `scalar_leaves` lowers compact + constant sums and round-trips through `to_expr`.
    let factor = x.pow(2) + y.clone() + Expr::number(1);
    let (polys, leaf_bases) = PolynomialInput::scalar_leaves(&[factor]).expect("scalar leaves");
    let first = polys.first().expect("one factor");
    let squared = first
        .checked_pow(2, &mut work)
        .expect("work")
        .expect("capacity");
    let expanded = squared.to_expr(&leaf_bases).expect("to_expr");
    let expected = x.pow(4)
        + Expr::number(2) * x.pow(2) * y.clone()
        + Expr::number(2) * x.pow(2)
        + y.pow(2)
        + Expr::number(2) * y
        + Expr::number(1);
    assert_eq!(
        expanded, expected,
        "expanded polynomial with constant round-trips"
    );
}

#[test]
fn heap_monomial_split_and_multiply_preserve_exponents() {
    let mut work = PolynomialWork::new(usize::MAX);
    let poly = polynomial(
        6,
        &[
            (3, &[2, 1, 0, 4, 3, 2]),
            (-5, &[1, 3, 0, 2, 3, 5]),
            (7, &[4, 2, 0, 2, 4, 2]),
        ],
        &mut work,
    );
    let (factor, primitive) = poly.split_monomial(&mut work).expect("work");
    assert_eq!(factor.as_slice(), &[1, 1, 0, 2, 3, 2]);
    let restored = primitive
        .multiply_monomial(&factor, &mut work)
        .expect("work");
    assert_eq!(
        restored, poly,
        "multiplying back the split factor restores the polynomial"
    );

    // Degree-pruned inline and general division rejects degree-exceeding divisors
    // immediately and divides exact multivariate products across inline and heap shapes.
    let a = polynomial(3, &[(2, &[4, 0, 0]), (-2, &[0, 4, 0])], &mut work);
    let inexact = polynomial(3, &[(1, &[1, 0, 0]), (-1, &[0, 0, 5])], &mut work);
    assert!(a.divide(&inexact, &mut work).expect("work").is_none());
    let divisor = polynomial(3, &[(1, &[2, 0, 0]), (1, &[0, 2, 0])], &mut work);
    let quotient = a.divide(&divisor, &mut work).expect("work").expect("exact");
    let expected_q = polynomial(3, &[(2, &[2, 0, 0]), (-2, &[0, 2, 0])], &mut work);
    assert_eq!(quotient, expected_q);
}
