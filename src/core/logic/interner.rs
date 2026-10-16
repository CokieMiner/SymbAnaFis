//! Collision-safe, reclaimable indexing of canonical live payloads.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]

use alloc::{sync::Weak, vec::Vec};

use crate::collections::HashMap;

use super::{FixedState, Mutex};

const SHARDS: usize = 16;

type Buckets<Value> = HashMap<u64, Vec<Weak<Value>>, FixedState>;

/// A weak index; keys contain only a lookup fingerprint, never owned payloads.
#[derive(Debug)]
pub struct Interner<Value: ?Sized> {
    shards: [Mutex<Buckets<Value>>; SHARDS],
}

impl<Value: ?Sized> Interner<Value> {
    #[expect(
        clippy::indexing_slicing,
        reason = "the first byte exists in an eight-byte array and its low four bits select one of sixteen shards"
    )]
    fn shard(&self, hash: u64) -> &Mutex<Buckets<Value>> {
        &self.shards[usize::from(hash.to_le_bytes()[0] & 15)]
    }
}

mod index;

#[cfg(test)]
mod tests;
