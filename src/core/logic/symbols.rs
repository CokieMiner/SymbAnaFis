//! Permanent global identities and reclaimable context-owned local metadata.

#![expect(
    clippy::self_named_module_files,
    reason = "registry and metadata owners keep fields private while their direct children implement storage operations"
)]

use core::hash::{Hash, Hasher};

use alloc::sync::{Arc, Weak};

use crate::collections::HashMap;

use super::{FixedState, IdSequence, Interner, Mutex, Symbol, SymbolError};

const SHARDS: usize = 16;
static NEXT_ID: IdSequence = IdSequence::new();
static INDEX: SymbolIndex = SymbolIndex::new();

/// Registry-owned global symbol storage, independent of local contexts.
pub static SYMBOLS: SymbolRegistry = SymbolRegistry::new();

/// Immutable symbol metadata. The directory retains global allocations;
/// local allocations are owned only by their context and referencing expressions.
#[derive(Debug)]
pub struct SymbolData {
    id: usize,
    name: Option<Arc<str>>,
}

#[derive(Debug)]
struct SymbolIndex {
    shards: [Mutex<HashMap<usize, MetadataEntry, FixedState>>; SHARDS],
}

#[derive(Debug)]
enum MetadataEntry {
    Permanent(Arc<SymbolData>),
    Local(Weak<SymbolData>),
}

#[derive(Debug)]
struct Storage {
    names: HashMap<Arc<str>, Arc<SymbolData>, FixedState>,
    anonymous: HashMap<usize, Arc<SymbolData>, FixedState>,
}

/// Sharded bindings for global names and registered anonymous identities.
#[derive(Debug)]
pub struct SymbolRegistry {
    shards: [Mutex<Storage>; SHARDS],
}

impl SymbolData {
    /// Allocate metadata for a local context without a global name binding.
    ///
    /// # Errors
    /// Returns ID exhaustion.
    pub fn local(name: Arc<str>) -> Result<Arc<Self>, SymbolError> {
        let data = Self::allocate(Some(name))?;
        INDEX.publish_local(&data);
        Ok(data)
    }

    /// Resolve a live ID through the metadata directory.
    ///
    /// # Errors
    /// Returns an invalid-ID error for an absent or reclaimed identity.
    pub fn lookup(id: usize) -> Result<Arc<Self>, SymbolError> {
        INDEX.lookup(id).ok_or(SymbolError::InvalidId(id))
    }

    /// Validate a global ID and copy its handle without cloning metadata.
    /// A local ID cannot become static.
    pub fn lookup_permanent(id: usize) -> Result<Symbol<'static>, SymbolError> {
        INDEX.lookup_permanent(id).ok_or(SymbolError::InvalidId(id))
    }

    /// Resolve a validated handle without a fallible conversion boundary.
    pub fn for_symbol(symbol: Symbol<'_>) -> Arc<Self> {
        // Global handles are published permanently. Local handles borrow a
        // context retaining the ID or an expression retaining this allocation.
        // Such owners cannot be dropped/reset while a borrowed handle is used.
        INDEX
            .lookup(symbol.id())
            .expect("symbol lifetime retains its published metadata")
    }

    fn permanent(name: Option<Arc<str>>) -> Result<Arc<Self>, SymbolError> {
        let data = Self::allocate(name)?;
        INDEX.publish_permanent(&data);
        Ok(data)
    }

    fn allocate(name: Option<Arc<str>>) -> Result<Arc<Self>, SymbolError> {
        let id = NEXT_ID.next().ok_or(SymbolError::IdExhausted)?;
        Ok(Arc::new(Self { id, name }))
    }

    /// Share the name allocation with its owner's name index.
    pub fn name_arc(&self) -> Option<Arc<str>> {
        self.name.as_ref().map(Arc::clone)
    }

    /// Borrow a named symbol's text.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the never-reused identity.
    pub const fn id(&self) -> usize {
        self.id
    }
}

impl PartialEq for SymbolData {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for SymbolData {}
impl Hash for SymbolData {
    fn hash<HasherType: Hasher>(&self, state: &mut HasherType) {
        self.id.hash(state);
    }
}

impl SymbolIndex {
    #[expect(
        clippy::indexing_slicing,
        reason = "shard_index returns a value in 0..16 for this sixteen-element array"
    )]
    const fn shard(&self, id: usize) -> &Mutex<HashMap<usize, MetadataEntry, FixedState>> {
        &self.shards[shard_index(id)]
    }
}
impl SymbolRegistry {
    fn name_shard(&self, name: &str) -> &Mutex<Storage> {
        let hash = Interner::<str>::fingerprint(name);
        self.id_shard(usize::from(hash.to_le_bytes()[0]))
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "shard_index returns a value in 0..16 for this sixteen-element array"
    )]
    const fn id_shard(&self, id: usize) -> &Mutex<Storage> {
        &self.shards[shard_index(id)]
    }
}

const fn shard_index(key: usize) -> usize {
    key & 15
}

mod directory;
mod registry;

#[cfg(test)]
mod tests;
