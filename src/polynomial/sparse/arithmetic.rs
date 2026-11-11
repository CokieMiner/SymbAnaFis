//! Sparse polynomial products and exact division.
//!
//! Products use dense accumulation, a convolution of the Kronecker line, or a
//! heap merge, whichever the shape makes cheapest.

#![expect(
    unsafe_code,
    reason = "product numerals stay below the slot count of their box, and heap entries name rows, columns, and terms pushed only below their lengths"
)]

use core::{iter::repeat_n, mem::replace, num::NonZeroUsize};

use alloc::{
    collections::{BinaryHeap, binary_heap::PeekMut},
    vec,
    vec::Vec,
};

use ntt_anafis::ConvPlan;

use crate::errors::SimplifyError;

use super::{Monomial, PolynomialWork, SparsePolynomial};

/// Most slots a box of product coefficients holds: 64 MiB of 128-bit sums.
const BOX_SLOTS: u32 = 1 << 22;
/// Term products per slot from which a convolution of the Kronecker line
/// costs less than accumulating every product (measured crossover).
const LINE_PRODUCTS: usize = 32;
/// Fewest slots for which planning a convolution pays (measured).
const LINE_SLOTS: usize = 64;

type Terms = [(Monomial, i64)];

impl SparsePolynomial {
    /// The product, or `None` when the content or a coefficient leaves native
    /// capacity.
    ///
    /// Contents multiply, and by Gauss's lemma the product of the primitive
    /// parts is primitive with a positive leading coefficient. The shorter
    /// factor gives the $n$ rows of the $nm$ term products. Of the $s$ slots
    /// of the box of exponent bounds, a convolution of the Kronecker line
    /// runs when $nm \ge 32 s$, dense accumulation when
    /// $s \le 4 nm \lfloor\log_{2} n\rfloor$, about what a heap merge spends,
    /// and a heap merge otherwise; the thresholds are measured crossovers.
    pub fn checked_mul(
        &self,
        other: &Self,
        work: &mut PolynomialWork,
    ) -> Result<Option<Self>, SimplifyError> {
        let dimensions = self.dimensions;
        let (rows, columns) = if self.terms.len() <= other.terms.len() {
            (self.terms.as_slice(), other.terms.as_slice())
        } else {
            (other.terms.as_slice(), self.terms.as_slice())
        };
        if rows.is_empty() {
            return Ok(Some(Self::zero(dimensions)));
        }
        let Some(content) = self.content.checked_mul(&other.content) else {
            return Ok(None);
        };
        // The primitive part of a constant is one.
        for (unit, kept) in [(rows, columns), (columns, rows)] {
            if let [(powers, 1)] = unit
                && powers.as_slice().iter().all(|power| *power == 0)
            {
                return Ok(Some(Self {
                    content,
                    terms: kept.to_vec(),
                    dimensions,
                }));
            }
        }
        work.product(rows.len(), columns.len())?;
        let products = rows.len().saturating_mul(columns.len());
        let levels = rows
            .len()
            .checked_ilog2()
            .map_or(0, |log| log.saturating_mul(4));
        let budget = products.saturating_mul(usize::try_from(levels).unwrap_or(0));
        let product = 'product: {
            if let Some(layout) = Kronecker::new(rows, columns, dimensions) {
                if layout.slots >= LINE_SLOTS
                    && layout.slots.saturating_mul(LINE_PRODUCTS) <= products
                    && let Some(lines) = layout.lines()
                {
                    break 'product layout.convolve(lines);
                }
                if layout.slots <= budget {
                    break 'product layout.accumulate();
                }
            }
            Self::merge(rows, columns, dimensions, work)?
        };
        Ok(product.map(|terms| Self {
            content,
            terms,
            dimensions,
        }))
    }

    pub fn checked_pow(
        &self,
        exponent: u32,
        work: &mut PolynomialWork,
    ) -> Result<Option<Self>, SimplifyError> {
        let mut result = Self::one(self.dimensions);
        let mut base = self.clone();
        let mut count = exponent;
        while count != 0 {
            if count & 1 != 0 {
                let Some(product) = result.checked_mul(&base, work)? else {
                    return Ok(None);
                };
                result = product;
            }
            count >>= 1;
            if count != 0 {
                let Some(square) = base.checked_mul(&base, work)? else {
                    return Ok(None);
                };
                base = square;
            }
        }
        Ok(Some(result))
    }

    /// The exact quotient `self / divisor` of nonzero polynomials, or `None`
    /// when the division leaves a remainder or a content or coefficient
    /// beyond native capacity.
    ///
    /// A quotient of primitive polynomials is primitive, and its leading
    /// coefficient is the positive quotient of the leads. The quotient terms
    /// appear in descending order. A max-heap merges the products of the
    /// quotient so far with the divisor's trailing terms, so each remainder
    /// term is formed once and nothing is stored beyond one heap entry per
    /// quotient term (Johnson 1974; Monagan and Pearce 2007).
    pub fn divide(
        &self,
        divisor: &Self,
        work: &mut PolynomialWork,
    ) -> Result<Option<Self>, SimplifyError> {
        let dimensions = self.dimensions;
        let Some(((lead_powers, lead_value), rest)) = divisor.terms.split_first() else {
            return Ok(None);
        };
        let Some(content) = self.content.checked_div(&divisor.content) else {
            return Ok(None);
        };
        let lead = i128::from(*lead_value);
        let mut quotient: Vec<(Monomial, i64)> = Vec::new();
        let mut heap = BinaryHeap::<(Monomial, usize, usize)>::new();
        let mut dividend = self.terms.iter().peekable();
        loop {
            let next = match (dividend.peek(), heap.peek()) {
                (None, None) => break,
                (Some((powers, _)), None) | (None, Some((powers, _, _))) => powers.clone(),
                (Some((first, _)), Some((second, _, _))) => first.max(second).clone(),
            };
            let mut sum = dividend
                .next_if(|(powers, _)| *powers == next)
                .map_or(0, |(_, value)| i128::from(*value));
            loop {
                let Some(mut top) = heap.peek_mut() else {
                    break;
                };
                if top.0 != next {
                    break;
                }
                let (row, column) = (top.1, top.2);
                // SAFETY: an entry is pushed only with a row below
                // `quotient.len()` and a column below `rest.len()`.
                let ((factor_powers, factor), (_, term)) =
                    unsafe { (quotient.get_unchecked(row), rest.get_unchecked(column)) };
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "factors of magnitude at most 2^63 have a product within 127 bits"
                )]
                let product = i128::from(*factor) * i128::from(*term);
                let Some(difference) = sum.checked_sub(product) else {
                    return Ok(None);
                };
                sum = difference;
                // The next product of the row replaces the top, so the heap
                // sifts once per product.
                let following = column.saturating_add(1);
                if let Some((successor, _)) = rest.get(following) {
                    work.spend(1)?;
                    let entry = (
                        factor_powers.sum(successor, dimensions, work)?,
                        row,
                        following,
                    );
                    *top = entry;
                } else {
                    PeekMut::pop(top);
                }
            }
            if sum == 0 {
                continue;
            }
            let Some(powers) = next.difference(lead_powers) else {
                return Ok(None);
            };
            let Some(value) = sum
                .checked_rem(lead)
                .filter(|remainder| *remainder == 0)
                .and_then(|_| sum.checked_div(lead))
                .and_then(Self::native)
            else {
                return Ok(None);
            };
            if let Some((first, _)) = rest.first() {
                work.spend(1)?;
                heap.push((powers.sum(first, dimensions, work)?, quotient.len(), 0));
            }
            quotient.push((powers, value));
        }
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

    /// The primitive product formed in descending order by a heap merge of
    /// its rows.
    ///
    /// A max-heap holds, for each term of the shorter factor, its next
    /// product with the longer factor; a row enters after the row above it
    /// starts, so every waiting product is at most a heap entry. Equal
    /// monomials leave the heap together and are added at once, and only one
    /// entry per row is stored: $O(nm \log n)$ time and $O(n)$ space beyond
    /// the product (Johnson 1974; Monagan and Pearce 2009).
    fn merge(
        rows: &Terms,
        columns: &Terms,
        dimensions: usize,
        work: &PolynomialWork,
    ) -> Result<Option<Vec<(Monomial, i64)>>, SimplifyError> {
        let (Some((first, _)), Some((left, _))) = (rows.first(), columns.first()) else {
            return Ok(Some(Vec::new()));
        };
        let mut heap = BinaryHeap::with_capacity(rows.len());
        heap.push((first.sum(left, dimensions, work)?, 0_usize, 0_usize));
        let mut product = Vec::new();
        let mut run: Option<(Monomial, i128)> = None;
        loop {
            let Some(mut top) = heap.peek_mut() else {
                break;
            };
            let (row, column) = (top.1, top.2);
            // SAFETY: an entry is pushed only with a row below `rows.len()`
            // and a column below `columns.len()`.
            let ((row_powers, row_value), (_, column_value)) =
                unsafe { (rows.get_unchecked(row), columns.get_unchecked(column)) };
            // The next product of the row replaces the top, so the heap
            // sifts once per product.
            let following = column.saturating_add(1);
            let (powers, _, _) = if let Some((next, _)) = columns.get(following) {
                let successor = (row_powers.sum(next, dimensions, work)?, row, following);
                let current = replace(&mut *top, successor);
                drop(top);
                current
            } else {
                PeekMut::pop(top)
            };
            if column == 0 {
                let below = row.saturating_add(1);
                if let Some((next, _)) = rows.get(below) {
                    heap.push((next.sum(left, dimensions, work)?, below, 0));
                }
            }
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "factors of magnitude at most 2^63 have a product within 127 bits"
            )]
            let value = i128::from(*row_value) * i128::from(*column_value);
            match &mut run {
                Some((last, sum)) if *last == powers => {
                    let Some(total) = sum.checked_add(value) else {
                        return Ok(None);
                    };
                    *sum = total;
                }
                _ => {
                    if let Some((last, sum)) = run.replace((powers, value))
                        && sum != 0
                    {
                        let Some(coefficient) = Self::native(sum) else {
                            return Ok(None);
                        };
                        product.push((last, coefficient));
                    }
                }
            }
        }
        if let Some((last, sum)) = run
            && sum != 0
        {
            let Some(coefficient) = Self::native(sum) else {
                return Ok(None);
            };
            product.push((last, coefficient));
        }
        Ok(Some(product))
    }
}

/// Two factors with the exponent bounds of their product, whose box holds
/// at most [`BOX_SLOTS`] slots.
///
/// An exponent vector within the bounds is the mixed-radix numeral of its
/// slot, variable zero most significant (Kronecker substitution). The digits
/// of a product's exponents are the sums of its factors' digits, at most the
/// bounds, so no digit carries: numerals add as exponents do and ascend in
/// lexicographic order.
struct Kronecker<'terms> {
    rows: &'terms Terms,
    columns: &'terms Terms,
    /// The sums of the factors' exponent maxima, by variable.
    bounds: Vec<u32>,
    /// The product of the bounds plus one, the number of numerals.
    slots: usize,
    dimensions: usize,
}

impl<'terms> Kronecker<'terms> {
    fn new(rows: &'terms Terms, columns: &'terms Terms, dimensions: usize) -> Option<Self> {
        let mut bounds = vec![0_u32; dimensions];
        let mut maxima = vec![0_u32; dimensions];
        for (factor, target) in [(rows, &mut bounds), (columns, &mut maxima)] {
            for (powers, _) in factor {
                for (maximum, power) in target.iter_mut().zip(powers.as_slice()) {
                    *maximum = (*maximum).max(*power);
                }
            }
        }
        let mut slots = 1_usize;
        for (bound, maximum) in bounds.iter_mut().zip(&maxima) {
            *bound = bound.checked_add(*maximum)?;
            slots = slots.checked_mul(usize::try_from(*bound).ok()?.checked_add(1)?)?;
        }
        (slots <= usize::try_from(BOX_SLOTS).unwrap_or(usize::MAX)).then_some(Self {
            rows,
            columns,
            bounds,
            slots,
            dimensions,
        })
    }

    /// The slot of an exponent vector of either factor or the product.
    fn numeral(&self, powers: &Monomial) -> Option<usize> {
        let mut slot = 0_usize;
        for (power, bound) in powers.as_slice().iter().zip(&self.bounds) {
            let (digit, radix) = (usize::try_from(*power).ok()?, usize::try_from(*bound).ok()?);
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "every exponent is at most its bound, so the numeral stays below the slot count"
            )]
            {
                slot = slot * (radix + 1) + digit;
            }
        }
        Some(slot)
    }

    /// Accumulate every term product into its slot: $O(nm + s)$.
    fn accumulate(&self) -> Option<Vec<(Monomial, i64)>> {
        let mut sums = vec![0_i128; self.slots];
        let row_slots = self
            .rows
            .iter()
            .map(|(powers, _)| self.numeral(powers))
            .collect::<Option<Vec<_>>>()?;
        let column_slots = self
            .columns
            .iter()
            .map(|(powers, _)| self.numeral(powers))
            .collect::<Option<Vec<_>>>()?;
        for (row_slot, (_, row_value)) in row_slots.iter().zip(self.rows) {
            for (column_slot, (_, column_value)) in column_slots.iter().zip(self.columns) {
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "a product's numeral is the sum of its factors' numerals and below the slot count; factors of magnitude at most 2^63 have a product within 127 bits"
                )]
                let (slot, value) = (
                    row_slot + column_slot,
                    i128::from(*row_value) * i128::from(*column_value),
                );
                // SAFETY: `slot` is below the slot count, the length of
                // `sums`.
                let sum = unsafe { sums.get_unchecked_mut(slot) };
                *sum = sum.checked_add(value)?;
            }
        }
        self.readout(sums.into_iter().rev().map(Some))
    }

    /// Both factors' Kronecker lines, each coefficient at its numeral as a
    /// two's complement limb and zero elsewhere, with the plan of their signed
    /// convolution; `None` when a coefficient does not fit a signed limb or
    /// the transform has no plan.
    fn lines(&self) -> Option<(ConvPlan, [Vec<usize>; 2])> {
        let mut lines = [Vec::new(), Vec::new()];
        let mut maxima = [0_usize; 2];
        for ((line, maximum), factor) in lines
            .iter_mut()
            .zip(&mut maxima)
            .zip([self.rows, self.columns])
        {
            let (lead, _) = factor.first()?;
            *line = vec![0; self.numeral(lead)?.checked_add(1)?];
            for (powers, value) in factor {
                *maximum = (*maximum).max(usize::try_from(value.unsigned_abs()).ok()?);
                let slot = self.numeral(powers)?;
                // SAFETY: terms descend in lexicographic order, which numerals
                // preserve, so the lead's numeral bounds every other one.
                *unsafe { line.get_unchecked_mut(slot) } =
                    isize::try_from(*value).ok()?.cast_unsigned();
            }
        }
        let [row_line, column_line] = lines;
        let [row_maximum, column_maximum] = maxima;
        let plan = ConvPlan::new(
            row_line.len(),
            column_line.len(),
            row_maximum,
            column_maximum,
            true,
        )?;
        Some((plan, [row_line, column_line]))
    }

    /// Convolve the factors' Kronecker lines: $O(s \log s)$. Each product
    /// coefficient is read back from its two's complement block of limbs,
    /// most significant limb first; one beyond `i128` declines the product.
    fn convolve(
        &self,
        (plan, [row_line, column_line]): (ConvPlan, [Vec<usize>; 2]),
    ) -> Option<Vec<(Monomial, i64)>> {
        let mut convolution = vec![0_usize; plan.output_len()];
        let mut scratch = vec![0_usize; plan.scratch_len()];
        if !plan.convolve(&mut convolution, &row_line, &column_line, &mut scratch) {
            return None;
        }
        let stride = NonZeroUsize::new(plan.stride())?;
        let radix = 1_i128.checked_shl(usize::BITS)?;
        let blocks = convolution.len().div_euclid(stride.get());
        let values = convolution.chunks_exact(stride.get()).rev().map(|block| {
            let (top, rest) = block.split_last()?;
            let mut value = i128::try_from(top.cast_signed()).ok()?;
            for &limb in rest.iter().rev() {
                value = value
                    .checked_mul(radix)?
                    .checked_add(i128::try_from(limb).ok()?)?;
            }
            Some(value)
        });
        let empty = self.slots.checked_sub(blocks)?;
        self.readout(repeat_n(Some(0), empty).chain(values))
    }

    /// The nonzero sums of the slots, read from the last slot down while an
    /// odometer tracks their exponents; `None` for an unreadable sum.
    fn readout(&self, sums: impl Iterator<Item = Option<i128>>) -> Option<Vec<(Monomial, i64)>> {
        let mut digits = self.bounds.clone();
        let mut terms = Vec::new();
        for slot in sums {
            let sum = slot?;
            if sum != 0 {
                let powers = Monomial::from_fn(self.dimensions, |index| {
                    digits.get(index).copied().unwrap_or(0)
                });
                terms.push((powers, SparsePolynomial::native(sum)?));
            }
            for (digit, bound) in digits.iter_mut().zip(&self.bounds).rev() {
                if let Some(next) = digit.checked_sub(1) {
                    *digit = next;
                    break;
                }
                *digit = *bound;
            }
        }
        Some(terms)
    }
}
