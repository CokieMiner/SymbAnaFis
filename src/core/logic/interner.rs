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

use crate::collections::HashTable;

use super::{FixedState, Mutex};

const SHARDS: usize = 16;

/// A value's fingerprint and a weak handle to that value.
type Entry<Value> = (u64, Weak<Value>);

/// A weak index of live canonical values, split into independently locked shards.
///
/// Entries never keep a value alive. Each entry stores the fingerprint of its
/// value, so the table can rehash entries whose value has already been dropped.
#[derive(Debug)]
pub struct Interner<Value: ?Sized> {
    shards: [Mutex<HashTable<Entry<Value>>>; SHARDS],
}

impl<Value: ?Sized> Interner<Value> {
    /// Select a shard from fingerprint bits 32 to 35. The table uses the low
    /// bits for bucket positions and the top seven bits for control tags.
    fn shard(&self, hash: u64) -> &Mutex<HashTable<Entry<Value>>> {
        // SAFETY: a remainder modulo `SHARDS` indexes one of the shards.
        unsafe {
            self.shards
                .get_unchecked(usize::from(hash.to_le_bytes()[4]) % SHARDS)
        }
    }
}

impl<Value: ?Sized + Hash + Eq> Interner<Value> {
    /// Create an empty index without allocating.
    pub const fn new() -> Self {
        Self {
            shards: [const { Mutex::new(HashTable::new()) }; SHARDS],
        }
    }

    /// Return the live value equal to `value`, or publish `value` as a new one.
    pub fn intern(&self, value: Value) -> Arc<Value>
    where
        Value: Sized,
    {
        let hash = Self::fingerprint(&value);
        // An upgraded colliding value can become its own last strong owner. Its
        // destructor reenters the index, so it runs after the lock is released.
        let mut rejected = Vec::new();
        let mut table = self.shard(hash).lock();
        let mut found = None;
        // The probe stops at the first live equal value; it upgrades each
        // colliding candidate once and keeps unequal ones for release.
        table.find(hash, |(stored, entry)| {
            if *stored != hash {
                return false;
            }
            let Some(candidate) = entry.upgrade() else {
                return false;
            };
            if *candidate == value {
                found = Some(candidate);
                return true;
            }
            rejected.push(candidate);
            false
        });
        let selected = found.unwrap_or_else(|| {
            let fresh = Arc::new(value);
            table.insert_unique(hash, (hash, Arc::downgrade(&fresh)), |(stored, _)| *stored);
            fresh
        });
        drop(table);
        if !rejected.is_empty() {
            drop(rejected);
            self.prune(hash);
        }
        selected
    }

    /// Remove the expired entry of the value at `address`, if it is still
    /// indexed, and release excess storage.
    ///
    /// A concurrent prune may have removed that entry and freed its address
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
        if table.is_empty() {
            *table = HashTable::new();
        } else if table.len().saturating_mul(4) < table.capacity() {
            table.shrink_to_fit(|(stored, _)| *stored);
        }
    }

    /// Remove every expired entry with this fingerprint and release excess storage.
    pub fn prune(&self, hash: u64) {
        let mut table = self.shard(hash).lock();
        // An expired weak handle owns only its allocation; dropping it under the
        // lock runs no value destructor.
        while let Ok(entry) = table.find_entry(hash, |(stored, weak)| {
            *stored == hash && weak.strong_count() == 0
        }) {
            drop(entry.remove());
        }
        if table.is_empty() {
            *table = HashTable::new();
        } else if table.len().saturating_mul(4) < table.capacity() {
            // Shrinking only after the load falls below one quarter keeps the
            // total rehashing work linear in the number of removals.
            table.shrink_to_fit(|(stored, _)| *stored);
        }
    }

    /// Hash a value with the fixed seed shared by every index.
    pub fn fingerprint<Key: ?Sized + Hash>(value: &Key) -> u64 {
        FixedState::with_seed(0).hash_one(value)
    }
}
