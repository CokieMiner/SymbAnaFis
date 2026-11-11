//! Never-reused identities, and symbol metadata: permanent global identities
//! and context-owned local ones.

#![expect(
    unsafe_code,
    reason = "shard indices are bounded by the shard count and permanent symbol allocations stay live for the process lifetime"
)]

#[cfg(target_has_atomic = "64")]
use core::sync::atomic::AtomicU64;
use core::{
    hash::{Hash, Hasher},
    ptr::{NonNull, null_mut},
    sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering},
};

use alloc::{string::String, sync::Arc, vec::Vec};

use crate::{
    collections::FixedMap,
    sync::{CachePadded, Mutex},
};

use super::{Facts, FixedState, Interner, Symbol, SymbolError};

const SHARDS: usize = 16;
static NEXT_ID: IdSequence = IdSequence::new();
static INDEX: SymbolIndex = SymbolIndex::new();

/// Registry-owned global symbol storage, independent of local contexts.
pub static SYMBOLS: SymbolRegistry = SymbolRegistry::new();

/// A checked sequence of identities for symbols and contexts; exhaustion
/// leaves its last value unchanged.
#[derive(Debug)]
pub struct IdSequence(AtomicUsize);

/// Symbol metadata, always owned through an `Arc`. Global metadata is
/// permanent; local metadata is owned by its context and by the expressions
/// that reference it.
#[derive(Debug)]
pub struct SymbolData {
    id: usize,
    name: Option<Arc<str>>,
    facts: AtomicU32,
    is_permanent: bool,
    cached_expr: AtomicPtr<()>,
    #[cfg(target_has_atomic = "64")]
    node_hash: AtomicU64,
}

/// Permanent global metadata by identity, for reconstructing global handles.
#[derive(Debug)]
struct SymbolIndex {
    shards: [CachePadded<Mutex<FixedMap<usize, Symbol<'static>>>>; SHARDS],
}

#[derive(Debug)]
struct Storage {
    names: FixedMap<Arc<str>, Symbol<'static>>,
    anonymous: FixedMap<usize, Symbol<'static>>,
}

/// Sharded bindings for global names and registered anonymous identities.
#[derive(Debug)]
pub struct SymbolRegistry {
    shards: [CachePadded<Mutex<Storage>>; SHARDS],
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
    pub fn local(name: Option<Arc<str>>) -> Result<Arc<Self>, SymbolError> {
        let id = NEXT_ID.next().ok_or(SymbolError::IdExhausted)?;
        Ok(Arc::new(Self {
            id,
            name,
            facts: AtomicU32::new(Facts::SYMBOL_DEFAULT.pack()),
            is_permanent: false,
            cached_expr: AtomicPtr::new(null_mut()),
            #[cfg(target_has_atomic = "64")]
            node_hash: AtomicU64::new(0),
        }))
    }

    /// The global handle with identity `id`.
    ///
    /// # Errors
    /// Returns [`SymbolError::InvalidId`] for an identity that is not global.
    pub fn lookup_permanent(id: usize) -> Result<Symbol<'static>, SymbolError> {
        INDEX
            .shard(id)
            .lock()
            .get(&id)
            .copied()
            .ok_or(SymbolError::InvalidId(id))
    }

    /// Allocate permanent metadata, register it, and then publish it by
    /// identity. The leaked `Arc` is the strong owner that keeps it alive for
    /// the whole process. Registration precedes publication, so no handle
    /// found by identity can be unregistered before its registration.
    fn permanent(
        name: Option<Arc<str>>,
        register: impl FnOnce(Symbol<'static>),
    ) -> Result<Symbol<'static>, SymbolError> {
        let id = NEXT_ID.next().ok_or(SymbolError::IdExhausted)?;
        let raw = Arc::into_raw(Arc::new(Self {
            id,
            name,
            facts: AtomicU32::new(Facts::SYMBOL_DEFAULT.pack()),
            is_permanent: true,
            cached_expr: AtomicPtr::new(null_mut()),
            #[cfg(target_has_atomic = "64")]
            node_hash: AtomicU64::new(0),
        }))
        .cast_mut();
        // SAFETY: `Arc::into_raw` returns a non-null pointer with full `Arc`
        // provenance whose strong count starts at one and is never decremented,
        // keeping the allocation live for `'static`.
        let symbol = unsafe { Self::permanent_symbol(NonNull::new_unchecked(raw)) };
        register(symbol);
        INDEX.shard(id).lock().insert(id, symbol);
        Ok(symbol)
    }

    /// Read the currently declared domain facts of this symbol.
    pub fn facts(&self) -> Facts {
        Facts::unpack(self.facts.load(Ordering::Acquire))
    }

    /// Replace the declared domain facts of this symbol and advance the global
    /// property epoch when they change.
    pub fn set_facts(&self, facts: Facts) {
        Facts::write_symbol_facts(&self.facts, facts);
    }

    /// Borrow a named symbol's text.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the never-reused identity.
    #[inline]
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Cached raw `Stored` handle slot when this metadata is permanent (`'static`).
    #[inline]
    pub const fn permanent_expr_cache(&self) -> Option<&AtomicPtr<()>> {
        if self.is_permanent {
            Some(&self.cached_expr)
        } else {
            None
        }
    }

    /// Cached `Node::Symbol` fingerprint (`0` until computed).
    #[cfg(target_has_atomic = "64")]
    #[inline]
    pub const fn node_hash(&self) -> &AtomicU64 {
        &self.node_hash
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
    fn shard(&self, id: usize) -> &Mutex<FixedMap<usize, Symbol<'static>>> {
        // SAFETY: a remainder modulo `SHARDS` indexes one of the shards.
        unsafe { self.shards.get_unchecked(id % SHARDS) }
    }

    pub const fn new() -> Self {
        Self {
            shards: [const {
                CachePadded::new(Mutex::new(FixedMap::with_hasher(FixedState::with_seed(0))))
            }; SHARDS],
        }
    }
}

impl SymbolRegistry {
    /// Create an independent binding registry with sixteen name shards.
    pub const fn new() -> Self {
        Self {
            shards: [const { CachePadded::new(Mutex::new(Storage::new())) }; SHARDS],
        }
    }

    /// Get or create a global name under its publication lock.
    pub fn intern(&self, name: &str) -> Result<Symbol<'static>, SymbolError> {
        let mut storage = self.name_shard(name).lock();
        if let Some(existing) = storage.names.get(name) {
            return Ok(*existing);
        }
        let text: Arc<str> = Arc::from(name);
        let symbol = SymbolData::permanent(Some(Arc::clone(&text)), |symbol| {
            storage.names.insert(text, symbol);
        })?;
        drop(storage);
        Ok(symbol)
    }

    /// Register a new name, rejecting duplicates in this registry.
    pub fn register(&self, name: &str) -> Result<Symbol<'static>, SymbolError> {
        let mut storage = self.name_shard(name).lock();
        if storage.names.contains_key(name) {
            return Err(SymbolError::DuplicateName(name.into()));
        }
        let text: Arc<str> = Arc::from(name);
        let symbol = SymbolData::permanent(Some(Arc::clone(&text)), |symbol| {
            storage.names.insert(text, symbol);
        })?;
        drop(storage);
        Ok(symbol)
    }

    /// Register an anonymous symbol with permanent identity metadata.
    pub fn anonymous(&self) -> Result<Symbol<'static>, SymbolError> {
        SymbolData::permanent(None, |symbol| {
            self.id_shard(symbol.id())
                .lock()
                .anonymous
                .insert(symbol.id(), symbol);
        })
    }

    /// Query a global name without creating it.
    pub fn get(&self, name: &str) -> Option<Symbol<'static>> {
        self.name_shard(name).lock().names.get(name).copied()
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
                    .is_some_and(|registered| registered.id() == id)
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
            .map(|shard| shard.lock())
            .iter()
            .map(|storage| storage.names.len().saturating_add(storage.anonymous.len()))
            .sum()
    }

    /// List global names in lexicographic order.
    pub fn names(&self) -> Vec<String> {
        let snapshot: Vec<Arc<str>> = {
            let guards = self.shards.each_ref().map(|shard| shard.lock());
            guards
                .iter()
                .flat_map(|storage| storage.names.keys().map(Arc::clone))
                .collect()
        };
        let mut names: Vec<String> = snapshot
            .into_iter()
            .map(|name| String::from(&*name))
            .collect();
        names.sort_unstable();
        names
    }

    fn name_shard(&self, name: &str) -> &Mutex<Storage> {
        let hash = Interner::<str>::fingerprint(name);
        // Select the shard from bits 32..35, matching `Interner::shard`, so
        // SwissTable low bucket bits remain independent of the shard index.
        self.id_shard(usize::from(hash.to_le_bytes()[4]))
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
