//! Exact sparse polynomial division by degree-bounded max-heap merging.

#![expect(
    unsafe_code,
    reason = "heap entries name quotient rows and trailing divisor columns pushed only below their lengths"
)]

use core::mem::replace;

use alloc::{
    collections::{BinaryHeap, binary_heap::PeekMut},
    vec::Vec,
};

use super::{
    HeapEntry, Monomial, PolynomialLimitExceeded, PolynomialWork, SparsePolynomial, Terms,
};

/// Sign-bit mask across four packed 32-bit exponent lanes: a lane difference
/// of operands below $2^{31}$ sets its bit 31 if and only if that lane borrows.
const INLINE_SIGN_MASK: u128 = 0x8000_0000_8000_0000_8000_0000_8000_0000;
const INLINE_LANE_LIMIT: u32 = 1 << 31;

impl SparsePolynomial {
    /// The exact quotient `self / divisor` of nonzero polynomials, or `None`
    /// when the division leaves a remainder or a content or coefficient
    /// beyond native capacity.
    ///
    /// A quotient of primitive polynomials is primitive, and its leading
    /// coefficient is the positive quotient of the leads. Every exact quotient
    /// satisfies $\deg_{i}(Q) = \deg_{i}(A) - \deg_{i}(B)$ in every variable
    /// $i$, which prunes inexact divisors upfront and bounds every admitted
    /// quotient term during the max-heap merge (Johnson 1974; Monagan and
    /// Pearce 2007).
    pub fn divide(
        &self,
        divisor: &Self,
        work: &mut PolynomialWork,
    ) -> Result<Option<Self>, PolynomialLimitExceeded> {
        let dimensions = self.dimensions;
        let Some(((lead_powers, lead_value), rest)) = divisor.terms.split_first() else {
            return Ok(None);
        };
        let lead_value = *lead_value;
        let Some(content) = self.content.checked_div(&divisor.content) else {
            return Ok(None);
        };
        let Some((first_powers, first_value)) = self.terms.first() else {
            return Ok(Some(Self::zero(dimensions)));
        };
        if rest.is_empty() && lead_value == 1 {
            if lead_powers.is_zero() {
                return Ok(Some(Self {
                    content,
                    terms: self.terms.clone(),
                    dimensions,
                }));
            }
            let mut terms = Vec::with_capacity(self.terms.len());
            for (powers, value) in &self.terms {
                let Some(reduced) = powers.difference(lead_powers) else {
                    return Ok(None);
                };
                terms.push((reduced, *value));
            }
            return Ok(Some(Self {
                content,
                terms,
                dimensions,
            }));
        }
        if first_powers.difference(lead_powers).is_none()
            || first_value.checked_rem(lead_value) != Some(0)
        {
            return Ok(None);
        }
        if let (Some((last_powers, last_value)), Some((tail_powers, tail_value))) =
            (self.terms.last(), rest.last())
            && (last_powers.difference(tail_powers).is_none()
                || last_value.checked_rem(*tail_value) != Some(0))
        {
            return Ok(None);
        }
        let Some((q_bounds, inline_safe)) =
            Self::quotient_bounds(dimensions, &self.terms, lead_powers, rest)
        else {
            return Ok(None);
        };
        let Some(quotient) = (if inline_safe {
            Self::divide_inline(&self.terms, lead_powers, lead_value, rest, &q_bounds, work)?
        } else {
            Self::divide_general(&self.terms, lead_powers, lead_value, rest, &q_bounds, work)?
        }) else {
            return Ok(None);
        };
        Ok(Some(if quotient.is_empty() {
            Self::zero(dimensions)
        } else {
            Self {
                content,
                terms: quotient,
                dimensions,
            }
        }))
    }

    /// Computes per-variable degree bounds `deg(dividend) - deg(divisor)`,
    /// returning `None` if `deg(divisor) > deg(dividend)` in any variable, plus
    /// whether `dimensions <= 4` and all dividend degrees lie below $2^{31}$.
    fn quotient_bounds(
        dimensions: usize,
        dividend: &Terms,
        lead_powers: &Monomial,
        rest: &Terms,
    ) -> Option<(Monomial, bool)> {
        let mut inline_a = [0_u32; 4];
        let mut inline_b = [0_u32; 4];
        let mut heap_a = Vec::new();
        let mut heap_b = Vec::new();
        let (deg_a, deg_b): (&mut [u32], &mut [u32]) = if let (Some(a), Some(b)) = (
            inline_a.get_mut(..dimensions),
            inline_b.get_mut(..dimensions),
        ) {
            (a, b)
        } else {
            heap_a.resize(dimensions, 0);
            heap_b.resize(dimensions, 0);
            (&mut heap_a, &mut heap_b)
        };
        for (powers, _) in dividend {
            for (max, &power) in deg_a.iter_mut().zip(powers.as_slice()) {
                *max = (*max).max(power);
            }
        }
        for (max, &power) in deg_b.iter_mut().zip(lead_powers.as_slice()) {
            *max = (*max).max(power);
        }
        for (powers, _) in rest {
            for (max, &power) in deg_b.iter_mut().zip(powers.as_slice()) {
                *max = (*max).max(power);
            }
        }
        let mut inline_safe = dimensions <= 4;
        for (a, &b) in deg_a.iter_mut().zip(&*deg_b) {
            if *a >= INLINE_LANE_LIMIT {
                inline_safe = false;
            }
            *a = a.checked_sub(b)?;
        }
        let bounds = Monomial::from_fn(dimensions, |idx| deg_a.get(idx).copied().unwrap_or(0));
        Some((bounds, inline_safe))
    }

    /// Allocation-free `u128` specialization of [`Self::divide_rows`].
    fn divide_inline(
        terms: &Terms,
        lead_powers: &Monomial,
        lead_value: i64,
        rest: &Terms,
        q_bounds: &Monomial,
        work: &mut PolynomialWork,
    ) -> Result<Option<Vec<(Monomial, i64)>>, PolynomialLimitExceeded> {
        let (Some(lead_packed), Some(bound_packed)) =
            (lead_powers.pack_inline(), q_bounds.pack_inline())
        else {
            return Self::divide_general(terms, lead_powers, lead_value, rest, q_bounds, work);
        };
        let Some(rest_packed) = rest
            .iter()
            .map(|(powers, value)| Some((powers.pack_inline()?, *value)))
            .collect::<Option<Vec<_>>>()
        else {
            return Self::divide_general(terms, lead_powers, lead_value, rest, q_bounds, work);
        };
        Self::divide_rows(
            terms
                .iter()
                .map(|(powers, value)| (powers.pack_inline().unwrap_or(0), *value)),
            terms.len(),
            lead_value,
            rest_packed.len(),
            |col| {
                // SAFETY: `divide_rows` passes `col < rest_packed.len()`.
                unsafe { rest_packed.get_unchecked(col).1 }
            },
            |current| {
                let reduced = current.wrapping_sub(lead_packed);
                ((reduced & INLINE_SIGN_MASK) == 0
                    && (bound_packed.wrapping_sub(reduced) & INLINE_SIGN_MASK) == 0)
                    .then_some(reduced)
            },
            |q_pow, col, _work| {
                // SAFETY: `divide_rows` passes `col < rest_packed.len()`.
                let tail_pow = unsafe { rest_packed.get_unchecked(col).0 };
                Ok(q_pow.wrapping_add(tail_pow))
            },
            Monomial::from_packed_inline,
            work,
        )
    }

    /// Arbitrary-dimension specialization of [`Self::divide_rows`].
    fn divide_general(
        terms: &Terms,
        lead_powers: &Monomial,
        lead_value: i64,
        rest: &Terms,
        q_bounds: &Monomial,
        work: &mut PolynomialWork,
    ) -> Result<Option<Vec<(Monomial, i64)>>, PolynomialLimitExceeded> {
        Self::divide_rows(
            terms.iter().map(|(powers, value)| (powers.clone(), *value)),
            terms.len(),
            lead_value,
            rest.len(),
            |col| {
                // SAFETY: `divide_rows` passes `col < rest.len()`.
                unsafe { rest.get_unchecked(col).1 }
            },
            |current| {
                let reduced = current.difference(lead_powers)?;
                q_bounds.difference(&reduced)?;
                Some(reduced)
            },
            |q_pow, col, w| {
                // SAFETY: `divide_rows` passes `col < rest.len()`.
                let tail_pow = unsafe { &rest.get_unchecked(col).0 };
                q_pow.sum(tail_pow, w)
            },
            |monomial| monomial,
            work,
        )
    }

    /// Core max-heap exact division engine parameterized by key representation.
    #[expect(
        clippy::too_many_arguments,
        reason = "the generic Johnson division loop takes key/coefficient closures for inline u128 and general Monomial representations"
    )]
    fn divide_rows<Key: Ord>(
        dividend_iter: impl Iterator<Item = (Key, i64)>,
        capacity: usize,
        lead_value: i64,
        rest_len: usize,
        rest_coeff: impl Fn(usize) -> i64,
        reduce_key: impl Fn(Key) -> Option<Key>,
        sum_key: impl Fn(&Key, usize, &mut PolynomialWork) -> Result<Key, PolynomialLimitExceeded>,
        mut finish: impl FnMut(Key) -> Monomial,
        work: &mut PolynomialWork,
    ) -> Result<Option<Vec<(Monomial, i64)>>, PolynomialLimitExceeded> {
        let lead = i128::from(lead_value);
        let mut quotient: Vec<(Key, i64)> = Vec::with_capacity(capacity);
        let mut heap = BinaryHeap::<HeapEntry<Key>>::new();
        let mut dividend = dividend_iter.peekable();
        loop {
            let (current, sum) = match (dividend.peek(), heap.peek()) {
                (None, None) => break,
                (Some((term_powers, _)), heap_top)
                    if heap_top.is_none_or(|top| term_powers > &top.powers) =>
                {
                    let Some((next_term, term_value)) = dividend.next() else {
                        break;
                    };
                    (next_term, i128::from(term_value))
                }
                _ => {
                    let Some((current, product)) = Self::advance_divide_top(
                        &mut heap,
                        &quotient,
                        rest_len,
                        &rest_coeff,
                        &sum_key,
                        work,
                    )?
                    else {
                        break;
                    };
                    let base = dividend
                        .next_if(|(term, _)| *term == current)
                        .map_or(0_i128, |(_, value)| i128::from(value));
                    let Some(mut sum) = base.checked_sub(product) else {
                        return Ok(None);
                    };
                    while heap.peek().is_some_and(|top| top.powers == current) {
                        let Some((_, term_product)) = Self::advance_divide_top(
                            &mut heap,
                            &quotient,
                            rest_len,
                            &rest_coeff,
                            &sum_key,
                            work,
                        )?
                        else {
                            break;
                        };
                        let Some(difference) = sum.checked_sub(term_product) else {
                            return Ok(None);
                        };
                        sum = difference;
                    }
                    if sum == 0 {
                        continue;
                    }
                    (current, sum)
                }
            };
            let Some(reduced) = reduce_key(current) else {
                return Ok(None);
            };
            let Some(value) = (if lead_value == 1 {
                Self::native(sum)
            } else if let Ok(narrow) = i64::try_from(sum) {
                narrow
                    .checked_rem(lead_value)
                    .filter(|&rem| rem == 0)
                    .and_then(|_| narrow.checked_div(lead_value))
                    .filter(|&q| q != i64::MIN)
            } else {
                sum.checked_rem(lead)
                    .filter(|&rem| rem == 0)
                    .and_then(|_| sum.checked_div(lead))
                    .and_then(Self::native)
            }) else {
                return Ok(None);
            };
            if rest_len > 0 {
                work.spend(1)?;
                heap.push(HeapEntry {
                    powers: sum_key(&reduced, 0, work)?,
                    row: quotient.len(),
                    column: 0,
                });
            }
            quotient.push((reduced, value));
        }
        Ok(Some(
            quotient
                .into_iter()
                .map(|(powers, value)| (finish(powers), value))
                .collect(),
        ))
    }

    #[inline]
    fn advance_divide_top<Key: Ord>(
        heap: &mut BinaryHeap<HeapEntry<Key>>,
        quotient: &[(Key, i64)],
        rest_len: usize,
        rest_coeff: &impl Fn(usize) -> i64,
        sum_key: &impl Fn(&Key, usize, &mut PolynomialWork) -> Result<Key, PolynomialLimitExceeded>,
        work: &mut PolynomialWork,
    ) -> Result<Option<(Key, i128)>, PolynomialLimitExceeded> {
        let Some(mut top) = heap.peek_mut() else {
            return Ok(None);
        };
        let (row, column) = (top.row, top.column);
        // SAFETY: an entry is pushed only with `row < quotient.len()`.
        let (factor_powers, factor) = unsafe { quotient.get_unchecked(row) };
        let term = rest_coeff(column);
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "factors of magnitude at most 2^63 have a product within 127 bits"
        )]
        let product = i128::from(*factor) * i128::from(term);
        let following = column.saturating_add(1);
        let powers = if following < rest_len {
            work.spend(1)?;
            let entry = HeapEntry {
                powers: sum_key(factor_powers, following, work)?,
                row,
                column: following,
            };
            let popped = replace(&mut *top, entry);
            drop(top);
            popped.powers
        } else {
            PeekMut::pop(top).powers
        };
        Ok(Some((powers, product)))
    }
}
