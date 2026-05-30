use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use crate::error::NumAnafisError;

use super::super::float_ops;
use super::super::int_math;
use super::super::scalar::{Scalar, ScalarRepr};

// ============================================================================
// GeneratorSet
// ============================================================================

/// A sorted, duplicate-free set of `(generator_id, metric)` pairs defining an algebra.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
pub struct GeneratorSet {
    entries: Vec<(u32, i8)>,
    pub(crate) cayley_signs: alloc::sync::Arc<[i8]>,
}

impl PartialEq for GeneratorSet {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Eq for GeneratorSet {}

impl core::hash::Hash for GeneratorSet {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.entries.hash(state);
    }
}

fn compute_cayley_signs(entries: &[(u32, i8)]) -> alloc::sync::Arc<[i8]> {
    let n = entries.len();
    if n > 7 {
        return alloc::sync::Arc::from(Vec::new());
    }
    let count = 1 << n;
    let mut table = alloc::vec![0; count * count];
    let dummy = GeneratorSet {
        entries: entries.to_vec(),
        cayley_signs: alloc::sync::Arc::from(Vec::new()),
    };
    for a in 0..count {
        for b in 0..count {
            if let Some((_, factor)) = mul_blades(&dummy, a, b) {
                table[a * count + b] = factor;
            }
        }
    }
    alloc::sync::Arc::from(table)
}

impl GeneratorSet {
    /// Empty set — represents the pure-scalar subalgebra.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            cayley_signs: compute_cayley_signs(&[]),
        }
    }

    /// Build from a pre-sorted, non-duplicate list of `(id, metric)` pairs.
    ///
    /// # Panics
    /// Panics if the entries are not sorted by `id` or contain duplicates.
    #[must_use]
    pub fn from_sorted(entries: Vec<(u32, i8)>) -> Self {
        assert!(
            entries.windows(2).all(|w| w[0].0 < w[1].0),
            "GeneratorSet entries must be sorted by id with no duplicates"
        );
        Self {
            cayley_signs: compute_cayley_signs(&entries),
            entries,
        }
    }

    /// Number of active generators.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when there are no generators (scalar subalgebra).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Generator ID at bit position `i`.
    #[must_use]
    pub fn id_at(&self, i: usize) -> u32 {
        self.entries[i].0
    }

    /// Metric factor (+1, −1, or 0) of the generator at bit position `i`.
    #[must_use]
    pub fn metric_at(&self, i: usize) -> i8 {
        self.entries[i].1
    }

    /// Compute the union of `self` and `other`.
    ///
    /// Returns `(union, perm_self, perm_other)` where `perm_x[i]` is the bit position of
    /// `x`'s i-th generator in the union.
    #[must_use]
    pub fn union_with(&self, other: &Self) -> (Self, Vec<u8>, Vec<u8>) {
        use core::cmp::Ordering;
        let mut union = Vec::with_capacity(self.entries.len() + other.entries.len());
        let mut perm_a = vec![0_u8; self.entries.len()];
        let mut perm_b = vec![0_u8; other.entries.len()];

        let (mut ia, mut ib) = (0_usize, 0_usize);
        let mut pos: u8 = 0;

        while ia < self.entries.len() && ib < other.entries.len() {
            let (aid, am) = self.entries[ia];
            let (bid, bm) = other.entries[ib];

            match aid.cmp(&bid) {
                Ordering::Less => {
                    union.push((aid, am));
                    perm_a[ia] = pos;
                    ia += 1;
                }
                Ordering::Equal => {
                    debug_assert_eq!(am, bm, "same generator ID must have the same metric");
                    union.push((aid, am));
                    perm_a[ia] = pos;
                    perm_b[ib] = pos;
                    ia += 1;
                    ib += 1;
                }
                Ordering::Greater => {
                    union.push((bid, bm));
                    perm_b[ib] = pos;
                    ib += 1;
                }
            }
            pos += 1;
        }
        for (idx, &e) in self.entries[ia..].iter().enumerate() {
            union.push(e);
            perm_a[ia + idx] = pos;
            pos += 1;
        }
        for (idx, &e) in other.entries[ib..].iter().enumerate() {
            union.push(e);
            perm_b[ib + idx] = pos;
            pos += 1;
        }

        let cayley_signs = compute_cayley_signs(&union);
        (
            Self {
                entries: union,
                cayley_signs,
            },
            perm_a,
            perm_b,
        )
    }
}

// ============================================================================
// CliffordCoeffs — dense coefficient storage
// ============================================================================

/// Maximum generators for inline (stack-resident) coefficient storage.
pub const INLINE_GENERATOR_LIMIT: u8 = 5;
/// Inline coefficient count = 2^5 = 32.
pub const INLINE_COEFF_COUNT: usize = 32;

#[allow(
    clippy::large_enum_variant,
    reason = "Inline path is intentionally stack-resident for n ≤ 5."
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
pub enum CliffordCoeffs {
    Inline([Scalar; INLINE_COEFF_COUNT]),
    Heap(Vec<Scalar>),
}

// ============================================================================
// CliffordNumber
// ============================================================================

/// A dense multivector in a Clifford algebra with labeled generators.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
pub struct CliffordNumber {
    pub(crate) coeffs: CliffordCoeffs,
    pub(crate) gens: GeneratorSet,
}

// ============================================================================
// Private free helpers (crate-visible to submodules)
// ============================================================================

#[inline]
pub fn scalar_zero() -> Scalar {
    Scalar::from_int(int_math::zero())
}

#[inline]
pub fn scalar_one() -> Scalar {
    Scalar(ScalarRepr::Int(
        int_math::from_i64(1).expect("constant 1 always fits"),
    ))
}

#[inline]
pub fn scalar_nan() -> Scalar {
    Scalar::from_float_raw(float_ops::nan())
}

pub fn coeff_count(n: u8) -> Result<usize, NumAnafisError> {
    1_usize
        .checked_shl(u32::from(n))
        .ok_or(NumAnafisError::ActiveGeneratorsTooLargeForPlatform { active: n })
}

pub fn coeff_count_unchecked(n: u8) -> usize {
    coeff_count(n).expect("generator count was already validated")
}

/// Permute a blade index from an old generator ordering into a new (union) ordering.
pub fn permute_blade(blade_old: usize, perm: &[u8]) -> usize {
    let mut new = 0_usize;
    for (i, &p) in perm.iter().enumerate() {
        if (blade_old >> i) & 1 == 1 {
            new |= 1_usize << usize::from(p);
        }
    }
    new
}

/// Compute the result blade and sign factor for the geometric product of blades `a` × `b`.
#[allow(
    clippy::unreachable,
    reason = "Metric invariant checked by construction"
)]
pub fn mul_blades(gens: &GeneratorSet, a: usize, b: usize) -> Option<(usize, i8)> {
    let n = gens.len();

    let mut swaps = 0_u32;
    for i in 0..n {
        if (b >> i) & 1 == 1 {
            swaps += (a >> (i + 1)).count_ones();
        }
    }
    let mut factor: i8 = if swaps & 1 == 0 { 1 } else { -1 };

    let overlap = a & b;
    if overlap != 0 {
        for i in 0..n {
            if (overlap >> i) & 1 == 1 {
                match gens.metric_at(i) {
                    1 => {}
                    -1 => factor = -factor,
                    0 => return None,
                    _ => unreachable!("metric must be +1, -1, or 0"),
                }
            }
        }
    }
    Some((a ^ b, factor))
}

/// Human-readable name for a generator ID.
pub const fn gen_name(id: u32) -> &'static str {
    use super::gen_id;
    match id {
        gen_id::E1 => "e1",
        gen_id::E2 => "e2",
        gen_id::E3 => "e3",
        gen_id::E_PLUS => "e+",
        gen_id::E_MINUS => "e-",
        _ => "e?",
    }
}

/// Basis blade label for display.
pub fn blade_label(gens: &GeneratorSet, blade: usize) -> String {
    let mut label = String::new();
    for i in 0..gens.len() {
        if (blade >> i) & 1 == 1 {
            let name = gen_name(gens.id_at(i));
            if name == "e?" {
                label.push_str(&gens.id_at(i).to_string());
            } else {
                label.push_str(name);
            }
        }
    }
    label
}
