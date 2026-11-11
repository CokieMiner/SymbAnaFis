//! The explicit operand positions one group has taken.

use alloc::vec::Vec;

/// The number of positions one word holds, as a shift.
const SHIFT: u32 = 6;

/// The mask of one position within its word.
const MASK: usize = (1 << SHIFT) - 1;

/// The explicit operand positions a group has taken, as a growable bit set.
///
/// Membership is one word read, so a rule that consumes a wide sum does not
/// rescan the taken prefix for every remaining operand. The first word is held
/// inline, so a set of at most 64 positions clones without allocating, and the
/// rest grows to the highest position taken: a group that takes one early
/// operand of a wide sum allocates nothing, and a group that consumes the whole
/// sum allocates one bit per operand.
#[derive(Clone, Debug, Default)]
pub struct Taken {
    /// The word holding positions `0..64`.
    first: u64,
    /// The words holding positions `64..`, so position `p` is bit `p & MASK` of
    /// `rest[(p >> SHIFT) - 1]`.
    rest: Vec<u64>,
    count: usize,
}

impl Taken {
    /// The number of distinct positions taken.
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Whether `position` is taken.
    pub fn contains(&self, position: usize) -> bool {
        let word = position >> SHIFT;
        let mask = 1 << (position & MASK);
        if word == 0 {
            self.first & mask != 0
        } else {
            self.rest
                .get(word.saturating_sub(1))
                .is_some_and(|held| held & mask != 0)
        }
    }

    /// Take `position`, growing the set to reach it.
    ///
    /// The word is looked up first, so the common case is one bounds check and
    /// one bit test; growth is out of line.
    pub fn insert(&mut self, position: usize) {
        let word = position >> SHIFT;
        let mask = 1 << (position & MASK);
        if word == 0 {
            if self.first & mask == 0 {
                self.first |= mask;
                self.count = self.count.saturating_add(1);
            }
            return;
        }
        let Some(slot) = self.rest.get_mut(word.saturating_sub(1)) else {
            self.grow(word, mask);
            return;
        };
        if *slot & mask == 0 {
            *slot |= mask;
            self.count = self.count.saturating_add(1);
        }
    }

    /// Take the bit of `word`, which the set does not hold yet.
    ///
    /// The words the set gains start clear, so the new word holds this bit
    /// alone. A set that reaches one word further pushes it, which is a store;
    /// a set that jumps several words zeroes the gap.
    #[cold]
    fn grow(&mut self, word: usize, mask: u64) {
        if self.rest.len().saturating_add(1) == word {
            self.rest.push(mask);
        } else {
            self.rest.resize(word, 0);
            if let Some(slot) = self.rest.get_mut(word.saturating_sub(1)) {
                *slot = mask;
            }
        }
        self.count = self.count.saturating_add(1);
    }

    /// Take every position of `positions`.
    pub fn extend(&mut self, positions: impl IntoIterator<Item = usize>) {
        for position in positions {
            self.insert(position);
        }
    }

    /// The first position at or after `from` that is not taken.
    ///
    /// One word is tested per 64 positions, so a wide sum whose taken prefix is
    /// long costs one step per word rather than one per operand. Positions past
    /// the highest one taken are free, so the answer is never below `from` and
    /// may exceed the set's growth.
    #[expect(
        clippy::as_conversions,
        reason = "a bit index below 64 widens losslessly to a slice index"
    )]
    pub fn first_free(&self, from: usize) -> usize {
        let word = from >> SHIFT;
        if word == 0 {
            let free = !self.first & (u64::MAX << (from & MASK));
            if free != 0 {
                return free.trailing_zeros() as usize;
            }
        }
        // Positions below `from` are not candidates, so the word holding it is
        // masked; every later word is tested whole.
        let start = word.saturating_sub(1);
        let offset = if word == 0 { 0 } else { from & MASK };
        for (index, held) in self.rest.iter().enumerate().skip(start) {
            let free = if index == start {
                !held & (u64::MAX << offset)
            } else {
                !held
            };
            if free != 0 {
                return index
                    .saturating_add(1)
                    .saturating_mul(1 << SHIFT)
                    .saturating_add(free.trailing_zeros() as usize);
            }
        }
        // Every position the set reaches is taken, so the first free one is the
        // first past its growth.
        from.max(self.rest.len().saturating_add(1).saturating_mul(1 << SHIFT))
    }
}
