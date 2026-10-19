//! Collision-safe weak interning with atomic lookup and publication.

use core::hash::{BuildHasher, Hash};

use alloc::{sync::Arc, vec::Vec};

use crate::collections::HashMap;

use super::{FixedState, Interner, Mutex, SHARDS};

impl<Value: ?Sized + Hash + Eq> Interner<Value> {
    /// Create an empty index.
    pub const fn new() -> Self {
        Self {
            shards: [const { Mutex::new(HashMap::with_hasher(FixedState::with_seed(0))) }; SHARDS],
        }
    }

    /// Allocate only when no equal live value exists.
    pub fn intern(&self, value: Value) -> Arc<Value>
    where
        Value: Sized,
    {
        let hash = Self::fingerprint(&value);
        let mut rejected = Vec::new();
        let selected = {
            let mut buckets = self.shard(hash).lock();
            let bucket = buckets.entry(hash).or_default();
            bucket.retain(|entry| entry.strong_count() != 0);
            let mut found = None;
            for entry in bucket.iter() {
                if let Some(candidate) = entry.upgrade() {
                    if candidate.as_ref() == &value {
                        found = Some(candidate);
                        break;
                    }
                    // A rejected candidate can become the last strong owner. Its
                    // destructor must run after the index lock is released.
                    rejected.push(candidate);
                }
            }
            #[expect(
                clippy::option_if_let_else,
                reason = "a closure could drop the owned candidate under the index lock and reenter the interner"
            )]
            let result = if let Some(existing) = found {
                existing
            } else {
                let fresh = Arc::new(value);
                bucket.push(Arc::downgrade(&fresh));
                fresh
            };
            drop(buckets);
            result
        };
        for candidate in rejected {
            let rejected_hash = Self::fingerprint(candidate.as_ref());
            drop(candidate);
            self.prune(rejected_hash);
        }
        selected
    }

    /// Remove expired entries from the given fingerprint bucket.
    pub fn prune(&self, hash: u64) {
        let mut buckets = self.shard(hash).lock();
        if let Some(bucket) = buckets.get_mut(&hash) {
            bucket.retain(|entry| entry.strong_count() != 0);
            if bucket.is_empty() {
                buckets.remove(&hash);
            } else if bucket.capacity() > bucket.len().saturating_mul(4).max(8) {
                // Return collision-bucket storage after a burst of expired
                // entries while retaining a small spare capacity for churn.
                bucket.shrink_to_fit();
            }
        }
        if buckets.is_empty() {
            *buckets = HashMap::with_hasher(FixedState::with_seed(0));
        } else if buckets.len().saturating_mul(4) < buckets.capacity() {
            // Geometric shrinking returns peak index storage without rehashing
            // on every deletion; total shrinking work is amortized linear.
            buckets.shrink_to_fit();
        }
    }

    /// Compute a lookup key without storing it in the value.
    pub fn fingerprint<Key: ?Sized + Hash>(value: &Key) -> u64 {
        FixedState::with_seed(0).hash_one(value)
    }
}
