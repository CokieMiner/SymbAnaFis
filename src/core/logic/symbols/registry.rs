//! Never-reused identities, and symbol metadata: permanent global identities
//! and context-owned local ones.

#![expect(
    unsafe_code,
    reason = "shard indices are remainders modulo the shard count, so they need no bounds check"
)]

use core::{
    hash::{Hash, Hasher},
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};

use crate::collections::FixedMap;

use super::{FixedState, Interner, Mutex, Symbol, SymbolError};

const SHARDS: usize = 16;
static NEXT_ID: IdSequence = IdSequence::new();
static INDEX: SymbolIndex = SymbolIndex::new();

/// Registry-owned global symbol storage, independent of local contexts.
pub static SYMBOLS: SymbolRegistry = SymbolRegistry::new();

/// A checked sequence of identities for symbols and contexts; exhaustion
/// leaves its last value unchanged.
#[derive(Debug)]
pub struct IdSequence(AtomicUsize);

/// Immutable symbol metadata, always owned through an `Arc`. Global metadata
/// is permanent; local metadata is owned by its context and by the expressions
/// that reference it.
#[derive(Debug)]
pub struct SymbolData {
    id: usize,
    name: Option<Arc<str>>,
}

/// Permanent global metadata by identity, for reconstructing global handles.
#[derive(Debug)]
struct SymbolIndex {
    shards: [Mutex<FixedMap<usize, &'static Arc<SymbolData>>>; SHARDS],
}

#[derive(Debug)]
struct Storage {
    names: FixedMap<Arc<str>, &'static Arc<SymbolData>>,
    anonymous: FixedMap<usize, &'static Arc<SymbolData>>,
}

/// Sharded bindings for global names and registered anonymous identities.
#[derive(Debug)]
pub struct SymbolRegistry {
    shards: [Mutex<Storage>; SHARDS],
}

impl IdSequence {
    /// Start at one, reserving zero as an unallocated identity.
    pub const fn new() -> Self {
        Self(AtomicUsize::new(1))
    }

    /// Allocate an identity without wrapping or reusing previous values.
    pub fn next(&self) -> Option<usize> {
        let mut current = self.0.load(Ordering::Relaxed);
        loop {
            let successor = current.checked_add(1)?;
            match self.0.compare_exchange_weak(
                current,
                successor,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(allocated) => return Some(allocated),
                Err(actual) => current = actual,
            }
        }
    }
}

impl SymbolData {
    /// Allocate metadata owned by a context, outside the global index.
    ///
    /// # Errors
    /// Returns [`SymbolError::IdExhausted`] when no identity can be allocated.
    pub fn local(name: Arc<str>) -> Result<Arc<Self>, SymbolError> {
        let id = NEXT_ID.next().ok_or(SymbolError::IdExhausted)?;
        Ok(Arc::new(Self {
            id,
            name: Some(name),
        }))
    }

    /// The global handle with identity `id`.
    ///
    /// # Errors
    /// Returns [`SymbolError::InvalidId`] for an identity that is not global.
    pub fn lookup_permanent(id: usize) -> Result<Symbol<'static>, SymbolError> {
        INDEX.lookup(id).ok_or(SymbolError::InvalidId(id))
    }

    /// Allocate permanent metadata, register it, and then publish it by
    /// identity. The leaked `Arc` is the strong owner that keeps it alive for
    /// the whole process. Registration precedes publication, so no handle
    /// found by identity can be unregistered before its registration.
    fn permanent(
        name: Option<Arc<str>>,
        register: impl FnOnce(&'static Arc<Self>),
    ) -> Result<&'static Arc<Self>, SymbolError> {
        let id = NEXT_ID.next().ok_or(SymbolError::IdExhausted)?;
        let data: &'static Arc<Self> = Box::leak(Box::new(Arc::new(Self { id, name })));
        register(data);
        INDEX.publish(data);
        Ok(data)
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
    fn shard(&self, id: usize) -> &Mutex<FixedMap<usize, &'static Arc<SymbolData>>> {
        // SAFETY: a remainder modulo `SHARDS` indexes one of the shards.
        unsafe { self.shards.get_unchecked(id % SHARDS) }
    }

    pub const fn new() -> Self {
        Self {
            shards: [const { Mutex::new(FixedMap::with_hasher(FixedState::with_seed(0))) }; SHARDS],
        }
    }

    pub fn publish(&self, data: &'static Arc<SymbolData>) {
        self.shard(data.id).lock().insert(data.id, data);
    }

    pub fn lookup(&self, id: usize) -> Option<Symbol<'static>> {
        self.shard(id)
            .lock()
            .get(&id)
            .copied()
            .map(SymbolData::symbol)
    }
}

impl SymbolRegistry {
    /// Create an independent binding registry with sixteen name shards.
    pub const fn new() -> Self {
        Self {
            shards: [const { Mutex::new(Storage::new()) }; SHARDS],
        }
    }

    /// Get or create a global name under its publication lock.
    pub fn intern(&self, name: &str) -> Result<&'static Arc<SymbolData>, SymbolError> {
        let mut storage = self.name_shard(name).lock();
        if let Some(existing) = storage.names.get(name) {
            return Ok(existing);
        }
        let text: Arc<str> = Arc::from(name);
        let data = SymbolData::permanent(Some(Arc::clone(&text)), |data| {
            storage.names.insert(text, data);
        })?;
        drop(storage);
        Ok(data)
    }

    /// Register a new name, rejecting duplicates in this registry.
    pub fn register(&self, name: &str) -> Result<&'static Arc<SymbolData>, SymbolError> {
        let mut storage = self.name_shard(name).lock();
        if storage.names.contains_key(name) {
            return Err(SymbolError::DuplicateName(name.into()));
        }
        let text: Arc<str> = Arc::from(name);
        let data = SymbolData::permanent(Some(Arc::clone(&text)), |data| {
            storage.names.insert(text, data);
        })?;
        drop(storage);
        Ok(data)
    }

    /// Register an anonymous symbol with permanent identity metadata.
    pub fn anonymous(&self) -> Result<&'static Arc<SymbolData>, SymbolError> {
        SymbolData::permanent(None, |data| {
            self.id_shard(data.id)
                .lock()
                .anonymous
                .insert(data.id, data);
        })
    }

    /// Query a global name without creating it.
    pub fn get(&self, name: &str) -> Result<Symbol<'static>, SymbolError> {
        self.name_shard(name)
            .lock()
            .names
            .get(name)
            .copied()
            .map(SymbolData::symbol)
            .ok_or_else(|| SymbolError::NotFound(name.into()))
    }

    /// Remove a global name binding without invalidating its identity.
    pub fn remove(&self, name: &str) -> bool {
        let removed = {
            let mut storage = self.name_shard(name).lock();
            let removed = storage.names.remove(name);
            Storage::compact(&mut storage.names);
            removed
        };
        removed.is_some()
    }

    /// Remove the global or anonymous registration of `data`. The metadata
    /// stays available, and context bindings are unaffected.
    pub fn unregister(&self, data: &SymbolData) -> bool {
        let id = data.id;
        let removed = data.name().map_or_else(
            || {
                let mut storage = self.id_shard(id).lock();
                let removed = storage.anonymous.remove(&id);
                Storage::compact(&mut storage.anonymous);
                removed
            },
            |name| {
                let mut storage = self.name_shard(name).lock();
                if storage
                    .names
                    .get(name)
                    .is_some_and(|registered| registered.id == id)
                {
                    let removed = storage.names.remove(name);
                    Storage::compact(&mut storage.names);
                    removed
                } else {
                    None
                }
            },
        );
        removed.is_some()
    }

    /// Count current global names and anonymous registrations.
    pub fn len(&self) -> usize {
        // All operations acquiring multiple shards use this order. Single-name
        // operations hold one shard, then the directory; there is no reverse path.
        // `array::map` locks the shards in index order.
        self.shards
            .each_ref()
            .map(Mutex::lock)
            .iter()
            .map(|storage| storage.names.len().saturating_add(storage.anonymous.len()))
            .sum()
    }

    /// List global names in unspecified order.
    pub fn names(&self) -> Vec<String> {
        self.shards
            .each_ref()
            .map(Mutex::lock)
            .iter()
            .flat_map(|storage| storage.names.keys().map(|name| String::from(&**name)))
            .collect()
    }

    fn name_shard(&self, name: &str) -> &Mutex<Storage> {
        let hash = Interner::<str>::fingerprint(name);
        self.id_shard(usize::from(hash.to_le_bytes()[0]))
    }

    fn id_shard(&self, id: usize) -> &Mutex<Storage> {
        // SAFETY: a remainder modulo `SHARDS` indexes one of the shards.
        unsafe { self.shards.get_unchecked(id % SHARDS) }
    }
}

impl Storage {
    const fn new() -> Self {
        Self {
            names: FixedMap::with_hasher(FixedState::with_seed(0)),
            anonymous: FixedMap::with_hasher(FixedState::with_seed(0)),
        }
    }

    fn compact<Key: Eq + Hash, Value>(bindings: &mut FixedMap<Key, Value>) {
        if bindings.is_empty() {
            *bindings = FixedMap::with_hasher(FixedState::with_seed(0));
        } else if bindings.capacity() > bindings.len().saturating_mul(4).max(64) {
            bindings.shrink_to_fit();
        }
    }
}
