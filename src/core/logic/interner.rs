//! Sharded weak hash-consing of canonical shared values.
//!
//! Hash-consing stores each distinct live value once and returns the shared
//! instance for every equal request (E. Goto, "Monocopy and associative
//! algorithms in an extended Lisp", Univ. of Tokyo, TR 74-03, 1974).

#![expect(
    unsafe_code,
    reason = "a shard index is a remainder modulo the shard count, so it needs no bounds check"
)]

use core::{
    hash::{BuildHasher, Hash},
    ptr::addr_eq,
};

use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};

use crate::{
    collections::HashTable,
    sync::{CachePadded, Mutex},
};

use super::FixedState;

const SHARDS: usize = 16;
const MIN_SHRINK_CAPACITY: usize = 64;

/// A value's fingerprint and a weak handle to that value.
type Entry<Value> = (u64, Weak<Value>);

/// A weak index of live canonical values, split into independently locked shards.
///
/// Entries never keep a value alive. Each entry stores the fingerprint of its
/// value, so the table can rehash entries whose value has already been dropped.
#[derive(Debug)]
pub struct Interner<Value: ?Sized> {
    shards: [CachePadded<Mutex<HashTable<Entry<Value>>>>; SHARDS],
}

impl<Value: ?Sized> Interner<Value> {
    /// Select a shard from fingerprint bits 32 to 35. The table uses the low
    /// bits for bucket positions and the top seven bits for control tags.
    #[inline]
    fn shard(&self, hash: u64) -> &Mutex<HashTable<Entry<Value>>> {
        // SAFETY: a remainder modulo `SHARDS` indexes one of the shards.
        unsafe {
            self.shards
                .get_unchecked(usize::from(hash.to_le_bytes()[4]) % SHARDS)
        }
    }

    fn shrink_if_sparse(table: &mut HashTable<Entry<Value>>) {
        if table.is_empty() && table.capacity() > MIN_SHRINK_CAPACITY {
            *table = HashTable::new();
        } else if table.capacity() > MIN_SHRINK_CAPACITY
            && table.len().saturating_mul(4) < table.capacity()
        {
            // Shrinking only after the load falls below one quarter and capacity
            // exceeds `MIN_SHRINK_CAPACITY` keeps total rehashing work linear
            // while avoiding churn on small warm shards.
            table.shrink_to_fit(|(stored, _)| *stored);
        }
    }
}

impl<Value: ?Sized + Hash + Eq> Interner<Value> {
    /// Create an empty index without allocating.
    pub const fn new() -> Self {
        Self {
            shards: [const { CachePadded::new(Mutex::new(HashTable::new())) }; SHARDS],
        }
    }

    /// Return the live value matching `matches(&candidate, &input)` under a
    /// precomputed fingerprint `hash`, or construct and publish one via
    /// `build(input)` on a cache miss.
    ///
    /// Callers must ensure `hash == Self::fingerprint(&value)` and that
    /// `matches(candidate, &input)` agrees with `*candidate == build(input)`
    /// so concurrent lookups and [`Self::remove`] address the same shard and
    /// bucket hash.
    pub fn intern<Input>(
        &self,
        hash: u64,
        input: Input,
        matches: impl Fn(&Value, &Input) -> bool,
        build: impl FnOnce(Input) -> Value,
    ) -> Arc<Value>
    where
        Value: Sized,
    {
        // An upgraded colliding value can become its own last strong owner. Its
        // destructor reenters the index, so it runs after the lock is released.
        let mut rejected = Vec::new();
        let mut table = self.shard(hash).lock();
        let mut found = None;
        // The probe stops at the first live matching value; it upgrades each
        // colliding candidate once and keeps non-matching ones for release.
        table.find(hash, |(stored, entry)| {
            if *stored != hash {
                return false;
            }
            let Some(candidate) = entry.upgrade() else {
                return false;
            };
            if matches(&candidate, &input) {
                found = Some(candidate);
                return true;
            }
            rejected.push(candidate);
            false
        });
        let selected = found.unwrap_or_else(|| {
            let fresh = Arc::new(build(input));
            table.insert_unique(hash, (hash, Arc::downgrade(&fresh)), |(stored, _)| *stored);
            fresh
        });
        drop(table);
        if !rejected.is_empty() {
            drop(rejected);
            let mut shard = self.shard(hash).lock();
            // An expired handle owns only its allocation, so no value
            // destructor runs under the lock.
            shard.retain(|(_, weak)| weak.strong_count() > 0);
            Self::shrink_if_sparse(&mut shard);
            drop(shard);
        }
        selected
    }

    /// Remove the expired entry of the value at `address`, if it is still
    /// indexed, and release excess storage.
    ///
    /// A concurrent sweep may have removed that entry and freed its address
    /// for an equal value, so only an expired entry at that address matches;
    /// a live value is never unindexed.
    pub fn remove(&self, hash: u64, address: *const Value) {
        let mut table = self.shard(hash).lock();
        if let Ok(entry) = table.find_entry(hash, |(stored, weak)| {
            *stored == hash && addr_eq(weak.as_ptr(), address) && weak.strong_count() == 0
        }) {
            // An expired weak handle owns only its allocation; dropping it under
            // the lock runs no value destructor.
            drop(entry.remove());
        }
        Self::shrink_if_sparse(&mut table);
        drop(table);
    }

    /// Hash a value with the fixed seed shared by every index.
    pub fn fingerprint<Key: ?Sized + Hash>(value: &Key) -> u64 {
        FixedState::with_seed(0).hash_one(value)
    }
}
