//! The Kronecker layout of a product: exponent bounds, numerals, and the
//! dense accumulation, convolution, and readout over them.

#![expect(
    unsafe_code,
    reason = "product numerals stay below the slot count of their box, and line entries name slots bounded by the lead's numeral"
)]

use core::{num::NonZeroUsize, ptr::eq};

use alloc::{vec, vec::Vec};

use ntt_anafis::{ConvPlan, ConvWorkspace, Sign};

use super::{Monomial, SparsePolynomial, Terms};

/// Most slots a box of product coefficients holds: 64 MiB of 128-bit sums.
const BOX_SLOTS: u32 = 1 << 22;

/// One or two prepared Kronecker lines for convolution.
///
/// Squaring passes the same slice twice so `ConvWorkspace::convolve` runs one
/// forward transform per prime.
pub enum KroneckerLines {
    Distinct([Vec<usize>; 2]),
    Square(Vec<usize>),
}

/// Two factors with the exponent and magnitude bounds of their product, whose
/// box holds at most [`BOX_SLOTS`] slots.
///
/// An exponent vector within the bounds is the mixed-radix numeral of its
/// slot, variable zero most significant (Kronecker substitution). The digits
/// of a product's exponents are the sums of its factors' digits, at most the
/// bounds, so no digit carries: numerals add as exponents do and ascend in
/// lexicographic order.
pub struct Kronecker<'terms> {
    rows: &'terms Terms,
    columns: &'terms Terms,
    /// The sums of the factors' exponent maxima, by variable; every bound plus
    /// one fits in `usize` because the slot product fits in `BOX_SLOTS`.
    bounds: Monomial,
    /// Largest coefficient magnitudes of `rows` and `columns`.
    max_row: u64,
    max_col: u64,
    /// The product of the bounds plus one, the number of numerals.
    slots: usize,
    dimensions: usize,
    /// Whether `rows` and `columns` alias the same factor (squaring).
    squaring: bool,
}

impl<'terms> Kronecker<'terms> {
    /// Inspects both factors once, returning the Kronecker layout when the box
    /// holds at most [`BOX_SLOTS`] slots, together with whether `dimensions <= 4`
    /// and every coordinate sum fits in `u32` without lane carry.
    pub fn analyze(
        rows: &'terms Terms,
        columns: &'terms Terms,
        dimensions: usize,
    ) -> (Option<Self>, bool) {
        let squaring = eq(rows, columns);
        let mut inline_bounds = [0_u32; 4];
        let mut inline_maxima = [0_u32; 4];
        let mut heap_bounds = Vec::new();
        let mut heap_maxima = Vec::new();
        let (bounds, maxima): (&mut [u32], &mut [u32]) = if let (Some(left), Some(right)) = (
            inline_bounds.get_mut(..dimensions),
            inline_maxima.get_mut(..dimensions),
        ) {
            (left, right)
        } else {
            heap_bounds.resize(dimensions, 0);
            heap_maxima.resize(dimensions, 0);
            (&mut heap_bounds, &mut heap_maxima)
        };
        let mut max_row = 0_u64;
        for (powers, value) in rows {
            max_row = max_row.max(value.unsigned_abs());
            for (maximum, &power) in bounds.iter_mut().zip(powers.as_slice()) {
                *maximum = (*maximum).max(power);
            }
        }
        let mut max_col = max_row;
        let mut sums_fit = true;
        if squaring {
            for bound in bounds.iter_mut() {
                if let Some(doubled) = bound.checked_mul(2) {
                    *bound = doubled;
                } else {
                    sums_fit = false;
                }
            }
        } else {
            max_col = 0_u64;
            for (powers, value) in columns {
                max_col = max_col.max(value.unsigned_abs());
                for (maximum, &power) in maxima.iter_mut().zip(powers.as_slice()) {
                    *maximum = (*maximum).max(power);
                }
            }
            for (bound, &maximum) in bounds.iter_mut().zip(&*maxima) {
                if let Some(sum) = bound.checked_add(maximum) {
                    *bound = sum;
                } else {
                    sums_fit = false;
                }
            }
        }
        let inline_safe = dimensions <= 4 && sums_fit;
        if !sums_fit {
            return (None, false);
        }
        let mut slots = 1_usize;
        for &bound in &*bounds {
            let Some(next) = usize::try_from(bound)
                .ok()
                .and_then(|b| b.checked_add(1))
                .and_then(|radix| slots.checked_mul(radix))
            else {
                return (None, inline_safe);
            };
            slots = next;
        }
        if slots > usize::try_from(BOX_SLOTS).unwrap_or(usize::MAX) {
            return (None, inline_safe);
        }
        let stored_bounds =
            Monomial::from_fn(dimensions, |index| bounds.get(index).copied().unwrap_or(0));
        (
            Some(Self {
                rows,
                columns,
                bounds: stored_bounds,
                max_row,
                max_col,
                slots,
                dimensions,
                squaring,
            }),
            inline_safe,
        )
    }

    /// The number of slots of the box of exponent bounds.
    pub const fn slots(&self) -> usize {
        self.slots
    }

    /// Accumulate every term product into its slot: $O(nm + s)$.
    ///
    /// Uses 64-bit slot accumulators whenever `min(n, m) * max_row * max_col <= i64::MAX`,
    /// halving memory traffic and avoiding per-product overflow branches.
    pub fn accumulate(&self) -> Option<Vec<(Monomial, i64)>> {
        let row_entries: Vec<(usize, i64)> = self
            .rows
            .iter()
            .map(|(powers, value)| (self.numeral(powers), *value))
            .collect();
        let column_entries: Vec<(usize, i64)> = if self.squaring {
            Vec::new()
        } else {
            self.columns
                .iter()
                .map(|(powers, value)| (self.numeral(powers), *value))
                .collect()
        };
        let capacity = self
            .slots
            .min(self.rows.len().saturating_mul(self.columns.len()));
        let terms = self.rows.len().min(self.columns.len());
        let fits_i64 = u128::try_from(terms)
            .ok()
            .and_then(|t| t.checked_mul(u128::from(self.max_row)))
            .and_then(|p| p.checked_mul(u128::from(self.max_col)))
            .is_some_and(|bound| bound <= u128::from(i64::MAX.unsigned_abs()));
        if fits_i64 {
            let mut sums = vec![0_i64; self.slots];
            if self.squaring {
                for (index, &(row_slot, row_value)) in row_entries.iter().enumerate() {
                    #[expect(
                        clippy::arithmetic_side_effects,
                        reason = "numerals sum below `self.slots`, and `fits_i64` bounds every partial sum within `[-i64::MAX, i64::MAX]`"
                    )]
                    {
                        // SAFETY: `row_slot + row_slot < self.slots`.
                        *unsafe { sums.get_unchecked_mut(row_slot + row_slot) } +=
                            row_value * row_value;
                        let twice_row = row_value * 2;
                        for &(col_slot, col_value) in row_entries.get(index.saturating_add(1)..)? {
                            // SAFETY: `row_slot + col_slot < self.slots`.
                            *unsafe { sums.get_unchecked_mut(row_slot + col_slot) } +=
                                twice_row * col_value;
                        }
                    }
                }
            } else {
                for &(row_slot, row_value) in &row_entries {
                    for &(col_slot, col_value) in &column_entries {
                        #[expect(
                            clippy::arithmetic_side_effects,
                            reason = "numerals sum below `self.slots`, and `fits_i64` bounds every partial sum within `[-i64::MAX, i64::MAX]`"
                        )]
                        {
                            // SAFETY: `row_slot + col_slot < self.slots`.
                            *unsafe { sums.get_unchecked_mut(row_slot + col_slot) } +=
                                row_value * col_value;
                        }
                    }
                }
            }
            return self.readout(sums.into_iter().rev().map(Some), self.slots, capacity);
        }
        self.accumulate_wide(&row_entries, &column_entries, capacity)
    }

    /// Both factors' Kronecker lines, each coefficient at its numeral as a
    /// two's complement limb and zero elsewhere, with the plan of their signed
    /// convolution; `None` when a coefficient does not fit a signed limb or
    /// the transform has no plan.
    pub fn lines(&self) -> Option<(ConvPlan, KroneckerLines)> {
        if self.squaring {
            let max_limb = usize::try_from(self.max_row).ok()?;
            let line = self.build_line(self.rows)?;
            let plan = ConvPlan::new(line.len(), line.len(), max_limb, max_limb, Sign::Signed)?;
            return Some((plan, KroneckerLines::Square(line)));
        }
        let row_max = usize::try_from(self.max_row).ok()?;
        let col_max = usize::try_from(self.max_col).ok()?;
        let row_line = self.build_line(self.rows)?;
        let column_line = self.build_line(self.columns)?;
        let plan = ConvPlan::new(
            row_line.len(),
            column_line.len(),
            row_max,
            col_max,
            Sign::Signed,
        )?;
        Some((plan, KroneckerLines::Distinct([row_line, column_line])))
    }

    /// Convolve the factors' Kronecker lines in `workspace`: $O(s \log s)$.
    /// Each product coefficient is read back from its two's complement block of
    /// limbs; one outside native primitive capacity (`|c| <= i64::MAX`)
    /// declines the product.
    pub fn convolve(
        &self,
        (plan, lines): (ConvPlan, KroneckerLines),
        workspace: &mut ConvWorkspace,
    ) -> Option<Vec<(Monomial, i64)>> {
        let mut convolution = vec![0_usize; plan.output_len()];
        workspace.prepare(plan);
        let ok = match &lines {
            KroneckerLines::Square(line) => workspace.convolve(&mut convolution, line, line),
            KroneckerLines::Distinct([row_line, column_line]) => {
                workspace.convolve(&mut convolution, row_line, column_line)
            }
        };
        if !ok {
            return None;
        }
        let stride = NonZeroUsize::new(plan.stride())?;
        let blocks = convolution.len().div_euclid(stride.get());
        let values = convolution.chunks_exact(stride.get()).rev().map(|block| {
            #[cfg(target_pointer_width = "64")]
            {
                let (&low, high) = block.split_first()?;
                let signed = low.cast_signed();
                let sign_extension = (signed >> 63).cast_unsigned();
                if high.iter().all(|&limb| limb == sign_extension) && signed != isize::MIN {
                    i64::try_from(signed).ok()
                } else {
                    None
                }
            }
            #[cfg(not(target_pointer_width = "64"))]
            {
                let (&top, rest) = block.split_last()?;
                let radix = 1_i128.checked_shl(usize::BITS)?;
                let mut value = i128::try_from(top.cast_signed()).ok()?;
                for &limb in rest.iter().rev() {
                    value = value
                        .checked_mul(radix)?
                        .checked_add(i128::try_from(limb).ok()?)?;
                }
                SparsePolynomial::native(value)
            }
        });
        self.readout(values, blocks, blocks)
    }

    /// Checked 128-bit slot accumulation when the static magnitude bound exceeds `i64::MAX`.
    fn accumulate_wide(
        &self,
        row_entries: &[(usize, i64)],
        column_entries: &[(usize, i64)],
        capacity: usize,
    ) -> Option<Vec<(Monomial, i64)>> {
        let mut sums = vec![0_i128; self.slots];
        if self.squaring {
            for (index, &(row_slot, row_value)) in row_entries.iter().enumerate() {
                let row_wide = i128::from(row_value);
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "numerals sum below `self.slots`; factors below 2^63 have square and doubled product strictly below 2^127"
                )]
                let (diag_slot, diag_value, twice_row) =
                    (row_slot + row_slot, row_wide * row_wide, row_wide * 2);
                // SAFETY: `diag_slot < self.slots`.
                let diag = unsafe { sums.get_unchecked_mut(diag_slot) };
                *diag = diag.checked_add(diag_value)?;
                for &(col_slot, col_value) in row_entries.get(index.saturating_add(1)..)? {
                    #[expect(
                        clippy::arithmetic_side_effects,
                        reason = "numerals sum below `self.slots`; `|2 * row * col| < 2^127`"
                    )]
                    let (slot, value) = (row_slot + col_slot, twice_row * i128::from(col_value));
                    // SAFETY: `slot < self.slots`.
                    let sum = unsafe { sums.get_unchecked_mut(slot) };
                    *sum = sum.checked_add(value)?;
                }
            }
        } else {
            for &(row_slot, row_value) in row_entries {
                let row_wide = i128::from(row_value);
                for &(col_slot, col_value) in column_entries {
                    #[expect(
                        clippy::arithmetic_side_effects,
                        reason = "numerals sum below `self.slots`; factors below 2^63 have a product within 127 bits"
                    )]
                    let (slot, value) = (row_slot + col_slot, row_wide * i128::from(col_value));
                    // SAFETY: `slot < self.slots`.
                    let sum = unsafe { sums.get_unchecked_mut(slot) };
                    *sum = sum.checked_add(value)?;
                }
            }
        }
        self.readout(
            sums.into_iter().rev().map(|sum| {
                if sum == 0 {
                    Some(0)
                } else {
                    SparsePolynomial::native(sum)
                }
            }),
            self.slots,
            capacity,
        )
    }

    fn build_line(&self, factor: &Terms) -> Option<Vec<usize>> {
        let (lead, _) = factor.first()?;
        let mut line = vec![0_usize; self.numeral(lead).checked_add(1)?];
        for (powers, value) in factor {
            let slot = self.numeral(powers);
            // SAFETY: terms descend in lexicographic order, which numerals
            // preserve, so the lead's numeral bounds every other one.
            *unsafe { line.get_unchecked_mut(slot) } =
                isize::try_from(*value).ok()?.cast_unsigned();
        }
        Some(line)
    }

    /// The nonzero coefficients of `blocks` slots read downward from slot
    /// `blocks - 1` to `0`, tracking exponents with a stack-inline odometer
    /// initialized directly from `blocks - 1`.
    fn readout(
        &self,
        coefficients: impl Iterator<Item = Option<i64>>,
        blocks: usize,
        capacity: usize,
    ) -> Option<Vec<(Monomial, i64)>> {
        if blocks == 0 {
            return Some(Vec::new());
        }
        if blocks > self.slots {
            return None;
        }
        if self.dimensions == 1 {
            let mut terms = Vec::with_capacity(capacity);
            let mut degree = u32::try_from(blocks.saturating_sub(1)).ok()?;
            for slot in coefficients {
                let value = slot?;
                if value != 0 {
                    terms.push((Monomial::univariate(degree), value));
                }
                degree = degree.saturating_sub(1);
            }
            return Some(terms);
        }
        let mut digits = Monomial::zero(self.dimensions);
        let mut remainder = blocks.saturating_sub(1);
        for (digit, &bound) in digits
            .as_mut_slice()
            .iter_mut()
            .zip(self.bounds.as_slice())
            .take(self.dimensions)
            .rev()
        {
            let radix = usize::try_from(bound).ok()?.checked_add(1)?;
            let coord = remainder.rem_euclid(radix);
            remainder = remainder.div_euclid(radix);
            *digit = u32::try_from(coord).ok()?;
        }
        let mut terms = Vec::with_capacity(capacity);
        for slot in coefficients {
            let value = slot?;
            if value != 0 {
                terms.push((digits.clone(), value));
            }
            for (digit, &bound) in digits
                .as_mut_slice()
                .iter_mut()
                .zip(self.bounds.as_slice())
                .take(self.dimensions)
                .rev()
            {
                if let Some(next) = digit.checked_sub(1) {
                    *digit = next;
                    break;
                }
                *digit = bound;
            }
        }
        Some(terms)
    }

    /// The slot of an exponent vector of either factor or the product.
    #[inline]
    fn numeral(&self, powers: &Monomial) -> usize {
        let mut slot = 0_usize;
        for (&power, &bound) in powers
            .as_slice()
            .iter()
            .zip(self.bounds.as_slice())
            .take(self.dimensions)
        {
            #[cfg_attr(
                target_pointer_width = "16",
                expect(
                    clippy::cast_possible_truncation,
                    reason = "Kronecker::analyze verified every bound + 1 <= BOX_SLOTS fits usize and every exponent is at most its bound"
                )
            )]
            #[expect(
                clippy::as_conversions,
                clippy::arithmetic_side_effects,
                reason = "Kronecker::analyze verified every bound + 1 <= BOX_SLOTS fits usize and every exponent is at most its bound"
            )]
            {
                slot = slot * (bound as usize + 1) + power as usize;
            }
        }
        slot
    }
}
