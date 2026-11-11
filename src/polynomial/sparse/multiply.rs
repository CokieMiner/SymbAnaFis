//! Sparse polynomial products and integer powers.
//!
//! Products use a direct shift for monomials, dense accumulation, a convolution
//! of the Kronecker line, or a heap merge, whichever the shape makes cheapest.

#![expect(
    unsafe_code,
    reason = "heap entries name rows and columns pushed only below their lengths"
)]

use core::{mem::replace, ptr::eq};

use alloc::{
    collections::{BinaryHeap, binary_heap::PeekMut},
    vec,
    vec::Vec,
};

use super::{
    HeapEntry, Kronecker, Monomial, PolynomialLimitExceeded, PolynomialWork, SparsePolynomial,
    Terms,
};

/// Term products per slot from which a convolution of the Kronecker line
/// costs less than accumulating every product (measured crossover).
const LINE_PRODUCTS: usize = 32;
/// Fewest slots for which planning a convolution pays (measured).
const LINE_SLOTS: usize = 64;

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
    ) -> Result<Option<Self>, PolynomialLimitExceeded> {
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
        // A one-term primitive polynomial is a single monomial with unit
        // coefficient, so multiplying by it shifts every exponent directly.
        if let [(powers, 1)] = rows {
            if powers.is_zero() {
                return Ok(Some(Self {
                    content,
                    terms: columns.to_vec(),
                    dimensions,
                }));
            }
            work.product(1, columns.len())?;
            let mut terms = Vec::with_capacity(columns.len());
            for (column_powers, column_value) in columns {
                terms.push((column_powers.sum(powers, work)?, *column_value));
            }
            return Ok(Some(Self {
                content,
                terms,
                dimensions,
            }));
        }
        work.product(rows.len(), columns.len())?;
        let products = rows.len().saturating_mul(columns.len());
        let levels = rows
            .len()
            .checked_ilog2()
            .map_or(0, |log| log.saturating_mul(4));
        let budget = products.saturating_mul(usize::try_from(levels).unwrap_or(0));
        let (box_layout, inline_safe) = Kronecker::analyze(rows, columns, dimensions);
        let product = 'product: {
            if let Some(layout) = box_layout {
                let slots = layout.slots();
                if slots >= LINE_SLOTS
                    && slots.saturating_mul(LINE_PRODUCTS) <= products
                    && let Some(lines) = layout.lines()
                {
                    break 'product layout.convolve(lines, work.workspace());
                }
                if slots <= budget {
                    break 'product layout.accumulate();
                }
            }
            if inline_safe {
                break 'product Self::merge_inline(rows, columns, work)?;
            }
            Self::merge_general(rows, columns, work)?
        };
        Ok(product.map(|terms| Self {
            content,
            terms,
            dimensions,
        }))
    }

    /// Binary exponentiation `self^exponent`, or `None` when a content or
    /// coefficient leaves native capacity.
    pub fn checked_pow(
        &self,
        exponent: u32,
        work: &mut PolynomialWork,
    ) -> Result<Option<Self>, PolynomialLimitExceeded> {
        if exponent == 0 {
            return Ok(Some(Self::one(self.dimensions)));
        }
        if exponent == 1 || self.terms.is_empty() {
            return Ok(Some(self.clone()));
        }
        // A one-term polynomial scales its exponents and raises its content
        // directly instead of allocating logarithmic intermediate products.
        if let [(powers, 1)] = self.terms.as_slice() {
            work.spend(self.dimensions.saturating_add(1))?;
            let Some(content) = (|| {
                let mut factor = self.content.clone();
                let mut remaining = exponent;
                while remaining & 1 == 0 {
                    factor = factor.checked_mul(&factor)?;
                    remaining >>= 1;
                }
                remaining >>= 1;
                if remaining == 0 {
                    return Some(factor);
                }
                let mut acc = factor.clone();
                while remaining != 0 {
                    factor = factor.checked_mul(&factor)?;
                    if remaining & 1 != 0 {
                        acc = acc.checked_mul(&factor)?;
                    }
                    remaining >>= 1;
                }
                Some(acc)
            })() else {
                return Ok(None);
            };
            let scaled = powers.scale(exponent, work)?;
            return Ok(Some(Self {
                content,
                terms: vec![(scaled, 1)],
                dimensions: self.dimensions,
            }));
        }
        let mut base = self.clone();
        let mut count = exponent;
        while count & 1 == 0 {
            let Some(square) = base.checked_mul(&base, work)? else {
                return Ok(None);
            };
            base = square;
            count >>= 1;
        }
        count >>= 1;
        if count == 0 {
            return Ok(Some(base));
        }
        let mut result = base.clone();
        while count != 0 {
            let Some(square) = base.checked_mul(&base, work)? else {
                return Ok(None);
            };
            base = square;
            if count & 1 != 0 {
                let Some(product) = result.checked_mul(&base, work)? else {
                    return Ok(None);
                };
                result = product;
            }
            count >>= 1;
        }
        Ok(Some(result))
    }

    /// Packed `u128` heap merge for inline polynomials (`dimensions <= 4`)
    /// whose coordinate sums cannot carry across 32-bit lanes, packing once
    /// and visiting only the upper triangle when squaring.
    fn merge_inline(
        rows: &Terms,
        columns: &Terms,
        work: &PolynomialWork,
    ) -> Result<Option<Vec<(Monomial, i64)>>, PolynomialLimitExceeded> {
        let symmetric = eq(rows, columns);
        let Some(row_packed) = rows
            .iter()
            .map(|(powers, value)| Some((powers.pack_inline()?, *value)))
            .collect::<Option<Vec<_>>>()
        else {
            return Self::merge_general(rows, columns, work);
        };
        let col_storage;
        let col_packed: &[(u128, i64)] = if symmetric {
            &row_packed
        } else {
            let Some(packed) = columns
                .iter()
                .map(|(powers, value)| Some((powers.pack_inline()?, *value)))
                .collect::<Option<Vec<_>>>()
            else {
                return Self::merge_general(rows, columns, work);
            };
            col_storage = packed;
            &col_storage
        };
        Self::merge_rows(
            symmetric,
            row_packed.len(),
            col_packed.len(),
            |row, column| {
                // SAFETY: `merge_rows` passes indices below the respective lengths.
                let (&(left, _), &(right, _)) = unsafe {
                    (
                        row_packed.get_unchecked(row),
                        col_packed.get_unchecked(column),
                    )
                };
                Ok(left.wrapping_add(right))
            },
            |row, column| {
                // SAFETY: `merge_rows` passes indices below the respective lengths.
                let (&(_, left), &(_, right)) = unsafe {
                    (
                        row_packed.get_unchecked(row),
                        col_packed.get_unchecked(column),
                    )
                };
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "factors of magnitude at most 2^63 have a product within 127 bits"
                )]
                {
                    i128::from(left) * i128::from(right)
                }
            },
            Monomial::from_packed_inline,
        )
    }

    /// General heap merge for arbitrary-dimension polynomials.
    fn merge_general(
        rows: &Terms,
        columns: &Terms,
        work: &PolynomialWork,
    ) -> Result<Option<Vec<(Monomial, i64)>>, PolynomialLimitExceeded> {
        Self::merge_rows(
            eq(rows, columns),
            rows.len(),
            columns.len(),
            |row, column| {
                // SAFETY: `merge_rows` passes indices below the respective lengths.
                let ((left, _), (right, _)) =
                    unsafe { (rows.get_unchecked(row), columns.get_unchecked(column)) };
                left.sum(right, work)
            },
            |row, column| {
                // SAFETY: `merge_rows` passes indices below the respective lengths.
                let ((_, left), (_, right)) =
                    unsafe { (rows.get_unchecked(row), columns.get_unchecked(column)) };
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "factors of magnitude at most 2^63 have a product within 127 bits"
                )]
                {
                    i128::from(*left) * i128::from(*right)
                }
            },
            |monomial| monomial,
        )
    }

    /// Primitive product in descending order by a lazy max-heap merge of its
    /// rows: $O(nm \log n)$ time and $O(n)$ heap space (Johnson 1974; Monagan
    /// and Pearce 2009). When `symmetric` is true (`col_count == row_count`),
    /// only the upper triangle `column >= row` is visited, doubling off-diagonal
    /// products.
    fn merge_rows<Key: Ord>(
        symmetric: bool,
        row_count: usize,
        col_count: usize,
        mut key: impl FnMut(usize, usize) -> Result<Key, PolynomialLimitExceeded>,
        mut term_product: impl FnMut(usize, usize) -> i128,
        mut finish: impl FnMut(Key) -> Monomial,
    ) -> Result<Option<Vec<(Monomial, i64)>>, PolynomialLimitExceeded> {
        if row_count == 0 || col_count == 0 {
            return Ok(Some(Vec::new()));
        }
        let mut heap = BinaryHeap::with_capacity(row_count);
        heap.push(HeapEntry {
            powers: key(0, 0)?,
            row: 0,
            column: 0,
        });
        let mut product = Vec::with_capacity(row_count.saturating_add(col_count).saturating_sub(1));
        let mut run: Option<(Key, i128)> = None;
        loop {
            let Some(mut top) = heap.peek_mut() else {
                break;
            };
            let (row, column) = (top.row, top.column);
            let raw = term_product(row, column);
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "factors below 2^63 have raw product below 2^126, so doubling fits i128"
            )]
            let value = if symmetric && row != column {
                raw * 2
            } else {
                raw
            };
            let following = column.saturating_add(1);
            let powers = if following < col_count {
                let successor = HeapEntry {
                    powers: key(row, following)?,
                    row,
                    column: following,
                };
                let current = replace(&mut *top, successor);
                drop(top);
                current.powers
            } else {
                PeekMut::pop(top).powers
            };
            let trigger = if symmetric { row.saturating_add(1) } else { 0 };
            if column == trigger {
                let below = row.saturating_add(1);
                if below < row_count {
                    let next_col = if symmetric { below } else { 0 };
                    heap.push(HeapEntry {
                        powers: key(below, next_col)?,
                        row: below,
                        column: next_col,
                    });
                }
            }
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
                        product.push((finish(last), coefficient));
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
            product.push((finish(last), coefficient));
        }
        Ok(Some(product))
    }
}
