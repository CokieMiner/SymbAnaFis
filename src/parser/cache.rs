//! Bounded spelling and power caches for one parse.

use core::hash::BuildHasher;

use alloc::vec::Vec;

use crate::{collections::FixedState, core::Expr, errors::ParseError};

use super::Value;

// Reuse repeated names and literals with a bounded open-addressed table. The
// table is sized from the source, so a short source gets a small one; the
// largest holds 256 live entries in a half-full table, which stays in the
// first-level data cache while covering typical large expressions.
const CACHED_SPELLINGS: usize = 256;
const CACHE_SLOTS: usize = CACHED_SPELLINGS * 2;
const POWER_SLOTS: usize = 1024;
const MIN_SLOTS: usize = 4;

/// The slot count for a source of `len` bytes: half the bytes, at least
/// [`MIN_SLOTS`] and at most [`CACHE_SLOTS`], rounded up to a power of two so
/// the mask stays exact.
pub fn slots_for(len: usize) -> usize {
    len.saturating_div(2)
        .clamp(MIN_SLOTS, CACHE_SLOTS)
        .checked_next_power_of_two()
        .unwrap_or(CACHE_SLOTS)
}

struct Entry<'source> {
    text: &'source str,
    positive: Expr,
    negative: Option<Expr>,
}

/// The expressions of spellings already resolved during one parse. Names and
/// literals have separate tables, so one kind cannot exhaust the other's.
///
/// The first entry is held inline and the table is allocated only on the
/// second distinct entry, so single-variable and single-literal parses never
/// allocate a table. It admits at most [`CACHED_SPELLINGS`] entries and then
/// stops growing so the bounded table stays in the first-level cache.
pub struct Cache<'source> {
    first: Option<Entry<'source>>,
    slots: Vec<Option<Entry<'source>>>,
    /// One less than the power-of-two slot count.
    mask: usize,
    /// The live entries the table admits, half its slots.
    limit: usize,
    len: usize,
}

/// Bounded open-addressed cache of `(base, exponent)` powers for one parse.
pub struct PowerCache {
    first: Option<(Expr, Expr, Expr)>,
    slots: Vec<Option<(Expr, Expr, Expr)>>,
    mask: usize,
    limit: usize,
    len: usize,
}

impl<'source> Cache<'source> {
    pub const fn new(slots: usize) -> Self {
        Self {
            first: None,
            slots: Vec::new(),
            mask: slots.saturating_sub(1),
            limit: slots.saturating_div(2),
            len: 0,
        }
    }

    /// The cached expression of `text`, or else the value of `resolve`,
    /// cached when it is an expression and fewer than the table's limit of
    /// entries are live. When `want_negative` is set, the entry's negated
    /// literal is also cached on first request.
    ///
    /// The probe ends because at least half of the slots stay vacant. Slots
    /// follow the FNV-1a hash of `text` (G. Fowler, L. C. Noll, K.-P. Vo,
    /// IETF draft-eastlake-fnv).
    pub fn resolve(
        &mut self,
        text: &'source str,
        want_negative: bool,
        resolve: impl FnOnce() -> Result<Value, ParseError>,
    ) -> Result<Value, ParseError> {
        if self.slots.is_empty() {
            if let Some(entry) = &mut self.first
                && entry.text == text
            {
                return Ok(Self::value_from(entry, want_negative));
            }
            let value = resolve()?;
            if self.len < self.limit
                && let Value::Expression(expression) = &value
            {
                let mut entry = Entry {
                    text,
                    positive: expression.clone(),
                    negative: None,
                };
                let output = Self::value_from(&mut entry, want_negative);
                if let Some(first_entry) = self.first.take() {
                    self.slots.resize_with(self.mask.saturating_add(1), || None);
                    self.insert_fresh(first_entry);
                    self.insert_fresh(entry);
                    self.len = 2;
                } else {
                    self.first = Some(entry);
                    self.len = 1;
                }
                return Ok(output);
            }
            return Ok(Self::uncached_value(value, want_negative));
        }
        let mut slot = Self::hash(text, self.mask);
        let vacant = loop {
            match self.slots.get_mut(slot) {
                Some(Some(entry)) if entry.text == text => {
                    return Ok(Self::value_from(entry, want_negative));
                }
                Some(Some(_)) => slot = slot.saturating_add(1) & self.mask,
                Some(None) | None => break slot,
            }
        };
        let value = resolve()?;
        if self.len < self.limit
            && let Value::Expression(expression) = &value
        {
            let mut entry = Entry {
                text,
                positive: expression.clone(),
                negative: None,
            };
            let output = Self::value_from(&mut entry, want_negative);
            if let Some(slot_entry) = self.slots.get_mut(vacant) {
                *slot_entry = Some(entry);
                self.len = self.len.saturating_add(1);
            }
            return Ok(output);
        }
        Ok(Self::uncached_value(value, want_negative))
    }

    fn value_from(entry: &mut Entry<'source>, want_negative: bool) -> Value {
        if want_negative {
            if entry.negative.is_none() {
                entry.negative = Value::negated_literal(&entry.positive);
            }
            Value::Literal {
                positive: entry.positive.clone(),
                negative: entry.negative.clone(),
            }
        } else {
            Value::Expression(entry.positive.clone())
        }
    }

    fn uncached_value(value: Value, want_negative: bool) -> Value {
        if want_negative && let Value::Expression(positive) = value {
            let negative = Value::negated_literal(&positive);
            Value::Literal { positive, negative }
        } else {
            value
        }
    }

    fn hash(text: &str, mask: usize) -> usize {
        let mut hash = 0x811c_9dc5_usize;
        for byte in text.bytes() {
            hash ^= usize::from(byte);
            hash = hash.wrapping_mul(0x0100_0193);
        }
        hash & mask
    }

    fn insert_fresh(&mut self, entry: Entry<'source>) {
        let mut slot = Self::hash(entry.text, self.mask);
        while let Some(Some(_)) = self.slots.get(slot) {
            slot = slot.saturating_add(1) & self.mask;
        }
        if let Some(target) = self.slots.get_mut(slot) {
            *target = Some(entry);
        }
    }
}

impl PowerCache {
    pub fn new(source_len: usize) -> Self {
        let slots = source_len
            .saturating_div(2)
            .clamp(MIN_SLOTS, POWER_SLOTS)
            .checked_next_power_of_two()
            .unwrap_or(POWER_SLOTS);
        Self {
            first: None,
            slots: Vec::new(),
            mask: slots.saturating_sub(1),
            limit: slots.saturating_div(2),
            len: 0,
        }
    }

    /// The cached result of `base.pow(exponent)`, or a newly constructed power
    /// cached while fewer than half the table's slots are occupied.
    #[inline]
    pub fn resolve(&mut self, base: Expr, exponent: Expr) -> Expr {
        if self.slots.is_empty() {
            if let Some((cached_base, cached_exp, cached_power)) = &self.first
                && *cached_base == base
                && *cached_exp == exponent
            {
                return cached_power.clone();
            }
            if self.len < self.limit {
                let power = base.pow(exponent.clone());
                let entry = (base, exponent, power.clone());
                if let Some(first_entry) = self.first.take() {
                    self.slots.resize_with(self.mask.saturating_add(1), || None);
                    self.insert_fresh(first_entry);
                    self.insert_fresh(entry);
                    self.len = 2;
                } else {
                    self.first = Some(entry);
                    self.len = 1;
                }
                return power;
            }
            return base.pow(exponent);
        }
        let mut slot = Self::hash(&base, &exponent, self.mask);
        let vacant = loop {
            match self.slots.get(slot) {
                Some(Some((cached_base, cached_exp, cached_power)))
                    if *cached_base == base && *cached_exp == exponent =>
                {
                    return cached_power.clone();
                }
                Some(Some(_)) => slot = slot.saturating_add(1) & self.mask,
                Some(None) | None => break slot,
            }
        };
        let power = base.pow(exponent.clone());
        if self.len < self.limit {
            if let Some(target) = self.slots.get_mut(vacant) {
                *target = Some((base, exponent, power.clone()));
                self.len = self.len.saturating_add(1);
            }
        } else if let Some(target) = self.slots.get_mut(Self::hash(&base, &exponent, self.mask)) {
            *target = Some((base, exponent, power.clone()));
        }
        power
    }

    #[inline]
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "only the low bits masked by `mask` are used as a slot index"
    )]
    fn hash(base: &Expr, exponent: &Expr, mask: usize) -> usize {
        (FixedState::default().hash_one((base, exponent)) as usize) & mask
    }

    fn insert_fresh(&mut self, entry: (Expr, Expr, Expr)) {
        let mut slot = Self::hash(&entry.0, &entry.1, self.mask);
        while let Some(Some(_)) = self.slots.get(slot) {
            slot = slot.saturating_add(1) & self.mask;
        }
        if let Some(target) = self.slots.get_mut(slot) {
            *target = Some(entry);
        }
    }
}
