//! Permanent global metadata and weak local entries with final-owner retirement.

use alloc::sync::Arc;

use super::{
    FixedState, HashMap, INDEX, MetadataEntry, Mutex, SHARDS, Symbol, SymbolData, SymbolIndex,
};

impl Drop for SymbolData {
    fn drop(&mut self) {
        // Permanent entries hold a strong owner and never reach this destructor.
        // Local entries contain only Weak handles; removing one cannot reenter
        // the destructor or release another symbol while this lock is held.
        INDEX.retire(self.id);
    }
}

impl SymbolIndex {
    pub const fn new() -> Self {
        Self {
            shards: [const { Mutex::new(HashMap::with_hasher(FixedState::with_seed(0))) }; SHARDS],
        }
    }

    pub fn publish_permanent(&self, data: &Arc<SymbolData>) {
        self.shard(data.id)
            .lock()
            .insert(data.id, MetadataEntry::Permanent(Arc::clone(data)));
    }

    pub fn publish_local(&self, data: &Arc<SymbolData>) {
        self.shard(data.id)
            .lock()
            .insert(data.id, MetadataEntry::Local(Arc::downgrade(data)));
    }

    pub fn lookup(&self, id: usize) -> Option<Arc<SymbolData>> {
        self.shard(id)
            .lock()
            .get(&id)
            .and_then(|entry| match entry {
                MetadataEntry::Permanent(data) => Some(Arc::clone(data)),
                MetadataEntry::Local(data) => data.upgrade(),
            })
    }

    pub fn lookup_permanent(&self, id: usize) -> Option<Symbol<'static>> {
        self.shard(id)
            .lock()
            .get(&id)
            .and_then(|entry| match entry {
                MetadataEntry::Permanent(data) => Some(data.permanent_symbol()),
                MetadataEntry::Local(_) => None,
            })
    }

    fn retire(&self, id: usize) {
        let mut entries = self.shard(id).lock();
        entries.remove(&id);
        if entries.is_empty() {
            *entries = HashMap::with_hasher(FixedState::with_seed(0));
        } else if entries.capacity() > entries.len().saturating_mul(4).max(64) {
            entries.shrink_to_fit();
        }
    }
}
