//! Allocation-free exponent vectors for polynomials in at most four variables
//! and shared work accounting with reusable convolution storage.

use core::cmp::Ordering;

use alloc::{vec, vec::Vec};

use ntt_anafis::ConvWorkspace;

/// All keys in one polynomial have the same dimension and representation.
/// Inline vectors pad unused variables with zeros, preserving lexicographic
/// order and componentwise arithmetic without a separate length or allocation.
///
/// Monomials are ordered lexicographically, inline before heap vectors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Monomial {
    powers: Powers,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Powers {
    Inline([u32; 4]),
    Heap(Vec<u32>),
}

/// Sparse polynomial conversion or arithmetic exhausted its work or exponent bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolynomialLimitExceeded {
    /// Configured work budget that was exhausted.
    pub limit: usize,
}

/// Shared work accounting and reusable convolution arena for sparse polynomial
/// conversion and arithmetic.
#[derive(Debug)]
pub struct PolynomialWork {
    remaining: usize,
    limit: usize,
    workspace: ConvWorkspace,
}

impl Ord for Monomial {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.powers, &other.powers) {
            (Powers::Inline(left), Powers::Inline(right)) => {
                Self::pack_array(left).cmp(&Self::pack_array(right))
            }
            (Powers::Heap(left), Powers::Heap(right)) => left.cmp(right),
            (Powers::Inline(_), Powers::Heap(_)) => Ordering::Less,
            (Powers::Heap(_), Powers::Inline(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for Monomial {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Monomial {
    /// The zero exponent vector in `dimensions` variables.
    pub fn zero(dimensions: usize) -> Self {
        Self {
            powers: if dimensions <= 4 {
                Powers::Inline([0; 4])
            } else {
                Powers::Heap(vec![0; dimensions])
            },
        }
    }

    /// A one-variable inline monomial $x_{0}^{d}$ of degree $d$.
    #[inline]
    pub const fn univariate(degree: u32) -> Self {
        Self {
            powers: Powers::Inline([degree, 0, 0, 0]),
        }
    }

    /// The unit basis exponent vector for `variable` in `dimensions` variables.
    pub fn variable(dimensions: usize, variable: usize) -> Self {
        Self::from_fn(dimensions, |index| u32::from(index == variable))
    }

    /// The exponent vector whose power at each of the first `dimensions`
    /// variables is `power(variable)`; padding variables stay zero.
    pub fn from_fn(dimensions: usize, mut power: impl FnMut(usize) -> u32) -> Self {
        if dimensions <= 4 {
            let mut inline = [0_u32; 4];
            for (index, slot) in inline.iter_mut().enumerate().take(dimensions) {
                *slot = power(index);
            }
            Self {
                powers: Powers::Inline(inline),
            }
        } else {
            Self {
                powers: Powers::Heap((0..dimensions).map(power).collect()),
            }
        }
    }

    /// The exponent vector with `power` at each named variable, or `None` when
    /// a variable is out of range, repeated, or zero.
    ///
    /// A padding slot is not a variable: writing one would break the
    /// lexicographic order and drop the exponent from the rendered term.
    pub fn from_indexed(
        dimensions: usize,
        powers: impl IntoIterator<Item = (usize, u32)>,
    ) -> Option<Self> {
        let mut result = Self::zero(dimensions);
        for (index, power) in powers {
            if index >= dimensions || power == 0 {
                return None;
            }
            let slot = result.as_mut_slice().get_mut(index)?;
            if *slot != 0 {
                return None;
            }
            *slot = power;
        }
        Some(result)
    }

    /// Unpack four big-endian base-$2^{32}$ exponents into an inline monomial.
    #[inline]
    pub fn from_packed_inline(bits: u128) -> Self {
        let mask = u128::from(u32::MAX);
        Self {
            powers: Powers::Inline([
                u32::try_from(bits >> 96).unwrap_or(0),
                u32::try_from((bits >> 64) & mask).unwrap_or(0),
                u32::try_from((bits >> 32) & mask).unwrap_or(0),
                u32::try_from(bits & mask).unwrap_or(0),
            ]),
        }
    }

    /// Whether every exponent is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        match &self.powers {
            Powers::Inline(powers) => *powers == [0; 4],
            Powers::Heap(powers) => powers.iter().all(|&power| power == 0),
        }
    }

    /// The exponent of `variable`, zero beyond the stored variables.
    #[inline]
    pub fn power(&self, variable: usize) -> u32 {
        self.as_slice().get(variable).copied().unwrap_or(0)
    }

    /// Pack an inline monomial's four exponents into one big-endian base-$2^{32}$
    /// integer, or `None` for a heap monomial.
    #[inline]
    pub fn pack_inline(&self) -> Option<u128> {
        match &self.powers {
            Powers::Inline(powers) => Some(Self::pack_array(powers)),
            Powers::Heap(_) => None,
        }
    }

    /// Whether `self` with `variable` zeroed equals `prefix`, which must
    /// already have exponent zero in `variable`.
    #[inline]
    pub fn matches_prefix(&self, prefix: &Self, variable: usize) -> bool {
        self.assert_compatible(prefix);
        if let (Powers::Inline(left), Powers::Inline(right)) = (&self.powers, &prefix.powers) {
            let mut masked = *left;
            if let Some(slot) = masked.get_mut(variable) {
                *slot = 0;
            }
            masked == *right
        } else {
            self.as_slice()
                .iter()
                .zip(prefix.as_slice())
                .enumerate()
                .all(|(index, (&left, &right))| index == variable || left == right)
        }
    }

    /// All stored exponents, including zero-padded trailing slots when inline.
    #[inline]
    pub fn as_slice(&self) -> &[u32] {
        match &self.powers {
            Powers::Inline(powers) => powers,
            Powers::Heap(powers) => powers,
        }
    }

    /// Mutable access to stored exponents.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        match &mut self.powers {
            Powers::Inline(powers) => powers,
            Powers::Heap(powers) => powers,
        }
    }

    /// Componentwise sum, checking `u32` exponent overflow.
    #[inline]
    pub fn sum(
        &self,
        other: &Self,
        work: &PolynomialWork,
    ) -> Result<Self, PolynomialLimitExceeded> {
        self.assert_compatible(other);
        let powers = if let (Powers::Inline([l0, l1, l2, l3]), Powers::Inline([r0, r1, r2, r3])) =
            (&self.powers, &other.powers)
        {
            // Padding slots are zero, so adding them never overflows and keeps them zero.
            Powers::Inline([
                work.exponent_sum(*l0, *r0)?,
                work.exponent_sum(*l1, *r1)?,
                work.exponent_sum(*l2, *r2)?,
                work.exponent_sum(*l3, *r3)?,
            ])
        } else {
            let mut sum = Vec::with_capacity(self.as_slice().len());
            for (&left, &right) in self.as_slice().iter().zip(other.as_slice()) {
                sum.push(work.exponent_sum(left, right)?);
            }
            Powers::Heap(sum)
        };
        Ok(Self { powers })
    }

    /// Add `other` componentwise in place without reallocating a heap vector.
    #[inline]
    pub fn add_assign(
        &mut self,
        other: &Self,
        work: &PolynomialWork,
    ) -> Result<(), PolynomialLimitExceeded> {
        self.assert_compatible(other);
        for (slot, &rhs) in self.as_mut_slice().iter_mut().zip(other.as_slice()) {
            *slot = work.exponent_sum(*slot, rhs)?;
        }
        Ok(())
    }

    /// Multiply every exponent by `factor`, checking native exponent capacity.
    #[inline]
    pub fn scale(
        &self,
        factor: u32,
        work: &PolynomialWork,
    ) -> Result<Self, PolynomialLimitExceeded> {
        let powers = match &self.powers {
            Powers::Inline([p0, p1, p2, p3]) => Powers::Inline([
                work.exponent_mul(*p0, factor)?,
                work.exponent_mul(*p1, factor)?,
                work.exponent_mul(*p2, factor)?,
                work.exponent_mul(*p3, factor)?,
            ]),
            Powers::Heap(slice) => {
                let mut scaled = Vec::with_capacity(slice.len());
                for &power in slice {
                    scaled.push(work.exponent_mul(power, factor)?);
                }
                Powers::Heap(scaled)
            }
        };
        Ok(Self { powers })
    }

    /// Componentwise difference `self - other`, or `None` on coordinate underflow.
    #[inline]
    pub fn difference(&self, other: &Self) -> Option<Self> {
        self.assert_compatible(other);
        let powers = if let (Powers::Inline([l0, l1, l2, l3]), Powers::Inline([r0, r1, r2, r3])) =
            (&self.powers, &other.powers)
        {
            Powers::Inline([
                l0.checked_sub(*r0)?,
                l1.checked_sub(*r1)?,
                l2.checked_sub(*r2)?,
                l3.checked_sub(*r3)?,
            ])
        } else {
            let mut difference = Vec::with_capacity(self.as_slice().len());
            for (&left, &right) in self.as_slice().iter().zip(other.as_slice()) {
                difference.push(left.checked_sub(right)?);
            }
            Powers::Heap(difference)
        };
        Some(Self { powers })
    }

    /// Return the componentwise minimum exponent vector.
    ///
    /// This is the largest monomial dividing both inputs. It is used to remove
    /// common monomial factors before the polynomial remainder sequence.
    pub fn minimum(&self, other: &Self) -> Self {
        self.assert_compatible(other);
        let mut result = self.clone();
        result.min_assign(other);
        result
    }

    /// Replace `self` with its componentwise minimum with `other`, returning
    /// whether every exponent is now zero.
    #[inline]
    pub fn min_assign(&mut self, other: &Self) -> bool {
        self.assert_compatible(other);
        let mut combined = 0_u32;
        if let (Powers::Inline(left), Powers::Inline(right)) = (&mut self.powers, &other.powers) {
            for (slot, &rhs) in left.iter_mut().zip(right) {
                *slot = (*slot).min(rhs);
                combined |= *slot;
            }
        } else {
            for (slot, &rhs) in self.as_mut_slice().iter_mut().zip(other.as_slice()) {
                *slot = (*slot).min(rhs);
                combined |= *slot;
            }
        }
        combined == 0
    }

    /// Subtract a common monomial factor whose exponents do not exceed `self`.
    pub fn remove_factor(&mut self, factor: &Self) {
        self.assert_compatible(factor);
        for (slot, &divisor) in self.as_mut_slice().iter_mut().zip(factor.as_slice()) {
            *slot = slot.saturating_sub(divisor);
        }
    }

    /// Separate one variable from its coefficient monomial. Absent variables
    /// have degree zero; the GCD driver selects axes below the shared dimension.
    #[inline]
    pub fn separate(&self, variable: usize) -> (u32, Self) {
        let mut reduced = self.clone();
        let degree = reduced.as_mut_slice().get_mut(variable).map_or(0, |slot| {
            let value = *slot;
            *slot = 0;
            value
        });
        (degree, reduced)
    }

    /// Shift a validated basis variable, checking native exponent capacity.
    #[inline]
    pub fn shift(
        &mut self,
        variable: usize,
        count: u32,
        work: &PolynomialWork,
    ) -> Result<(), PolynomialLimitExceeded> {
        if let Some(slot) = self.as_mut_slice().get_mut(variable) {
            *slot = work.exponent_sum(*slot, count)?;
        }
        Ok(())
    }

    /// Four exponents in lexicographic order as one integer in base $2^{32}$.
    #[inline]
    fn pack_array(&[first, second, third, fourth]: &[u32; 4]) -> u128 {
        (u128::from(first) << 96)
            | (u128::from(second) << 64)
            | (u128::from(third) << 32)
            | u128::from(fourth)
    }

    /// Assert that two operands share one representation and dimension, which
    /// the componentwise kernels require.
    fn assert_compatible(&self, other: &Self) {
        debug_assert!(
            matches!(
                (&self.powers, &other.powers),
                (Powers::Inline(_), Powers::Inline(_)) | (Powers::Heap(_), Powers::Heap(_))
            ),
            "operands share one representation"
        );
        debug_assert_eq!(
            self.as_slice().len(),
            other.as_slice().len(),
            "operands share one dimension"
        );
    }
}

impl PolynomialWork {
    /// Create a work budget of `limit` units with an empty convolution workspace.
    pub const fn new(limit: usize) -> Self {
        Self {
            remaining: limit,
            limit,
            workspace: ConvWorkspace::new(),
        }
    }

    /// Reusable dense-convolution workspace retained across products in this pass.
    #[inline]
    pub const fn workspace(&mut self) -> &mut ConvWorkspace {
        &mut self.workspace
    }

    /// Charge `count` work units against the remaining budget.
    #[inline]
    pub fn spend(&mut self, count: usize) -> Result<(), PolynomialLimitExceeded> {
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or(PolynomialLimitExceeded { limit: self.limit })?;
        Ok(())
    }

    /// Charge `left * right` work units against the remaining budget.
    #[inline]
    pub fn product(&mut self, left: usize, right: usize) -> Result<(), PolynomialLimitExceeded> {
        let count = left
            .checked_mul(right)
            .ok_or(PolynomialLimitExceeded { limit: self.limit })?;
        self.spend(count)
    }

    /// Checked exponent sum bounded by `u32::MAX`.
    #[inline]
    pub fn exponent_sum(&self, left: u32, right: u32) -> Result<u32, PolynomialLimitExceeded> {
        left.checked_add(right)
            .ok_or(PolynomialLimitExceeded { limit: self.limit })
    }

    /// Checked exponent product bounded by `u32::MAX`.
    #[inline]
    pub fn exponent_mul(&self, base: u32, factor: u32) -> Result<u32, PolynomialLimitExceeded> {
        base.checked_mul(factor)
            .ok_or(PolynomialLimitExceeded { limit: self.limit })
    }
}
