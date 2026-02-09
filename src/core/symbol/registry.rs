//! Global symbol registry management.
//!
//! Contains the global registries and public API functions for symbol management.
//! This implementation uses sharding to minimize lock contention and `FxHash` for
//! high-performance mapping.

use std::sync::{Mutex, RwLock};

use rustc_hash::{FxHashMap, FxHasher};
use slotmap::{DefaultKey, SlotMap};
use std::hash::Hasher;

use super::interned::InternedSymbol;
use super::{Symbol, SymbolError};

// ============================================================================
// Public API Functions for ID/Key Conversion and Anonymous Symbols
// ============================================================================

/// Create a `DefaultKey` from a 64-bit ID.
///
/// This is the reverse of `key.data().as_ffi()`.
#[inline]
pub fn key_from_id(id: u64) -> DefaultKey {
    slotmap::KeyData::from_ffi(id).into()
}

/// Create a new anonymous symbol.
#[inline]
#[must_use]
pub fn symb_anon() -> Symbol {
    let key = REGISTRY
        .id_to_data
        .write()
        .expect("Global ID registry poisoned")
        .insert_with_key(InternedSymbol::new_anon);
    Symbol(key)
}

/// Create a new named symbol that is only registered by ID, not by name.
/// This is for use in isolated contexts.
#[must_use]
pub fn symb_new_isolated(name: &str) -> Symbol {
    let key = REGISTRY
        .id_to_data
        .write()
        .expect("Global ID registry poisoned")
        .insert_with_key(|k| InternedSymbol::new_named(name, k));
    Symbol(key)
}

// ============================================================================
// Global Symbol Registry
// ============================================================================

/// Number of shards for the symbol registry
const NUM_SHARDS: usize = 16;

/// A shard of the symbol registry for name-to-key mapping
struct RegistryShard {
    /// Mapping from symbol names to their keys
    name_to_symbol_key: FxHashMap<String, DefaultKey>,
}

/// Unified symbol registry storage
struct SymbolRegistry {
    /// Shards for Name -> Symbol mapping to reduce contention
    shards: [Mutex<RegistryShard>; NUM_SHARDS],
    /// ID -> Symbol mapping using `SlotMap` for memory efficiency and safe key generation
    id_to_data: RwLock<SlotMap<DefaultKey, InternedSymbol>>,
}

impl SymbolRegistry {
    /// Create a new symbol registry
    fn new() -> Self {
        let shards: [Mutex<RegistryShard>; NUM_SHARDS] = std::array::from_fn(|_| {
            Mutex::new(RegistryShard {
                name_to_symbol_key: FxHashMap::default(),
            })
        });

        Self {
            shards,
            id_to_data: RwLock::new(SlotMap::with_key()),
        }
    }

    /// # Panics
    ///
    /// Panics if the global registry hash cannot be computed.
    fn get_shard(&self, name: &str) -> &Mutex<RegistryShard> {
        // Use FxHasher for sharding to stay consistent and fast
        let mut hasher = FxHasher::default();
        std::hash::Hash::hash(name, &mut hasher);
        let hash = hasher.finish();

        // Truncation is safe/expected here as we only need the low bits for sharding (hash % 16)
        #[allow(
            clippy::cast_possible_truncation,
            reason = "Truncation is safe/expected here as we only need the low bits for sharding (hash % 16)"
        )]
        let shard_idx = (hash as usize) % NUM_SHARDS;
        &self.shards[shard_idx]
    }
}

/// Global registry for symbols
static REGISTRY: std::sync::LazyLock<SymbolRegistry> =
    std::sync::LazyLock::new(SymbolRegistry::new);

thread_local! {
    // Thread-local cache to avoid global lock contention on frequently accessed symbols
    static ID_CACHE: std::cell::RefCell<FxHashMap<DefaultKey, InternedSymbol>> = std::cell::RefCell::new(FxHashMap::default());

    // Thread-local cache for name → Symbol lookups (hot path in parsing/construction)
    static NAME_CACHE: std::cell::RefCell<FxHashMap<String, Symbol>> = std::cell::RefCell::new(FxHashMap::default());
}

/// Look up `InternedSymbol` by its u64 ID.
///
/// # Panics
///
/// Panics if the global ID registry lock is poisoned.
pub fn lookup_by_id(id: u64) -> Option<InternedSymbol> {
    let key = key_from_id(id);

    // Try TLS cache first to avoid RwLock contention
    if let Some(s) = ID_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return Some(s);
    }

    // Fallback to global registry
    let symbol = REGISTRY
        .id_to_data
        .read()
        .expect("Global ID registry poisoned")
        .get(key)
        .cloned();

    // Populate cache if found
    if let Some(ref s) = symbol {
        ID_CACHE.with(|cache| {
            cache.borrow_mut().insert(key, s.clone());
        });
    }

    symbol
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Create a new named symbol (errors if name already registered)
///
/// # Errors
/// Returns `SymbolError::DuplicateName` if a symbol with this name already exists.
///
/// # Panics
///
/// Panics if any global registry lock is poisoned.
pub fn symb_new(name: &str) -> Result<Symbol, SymbolError> {
    let shard_lock = REGISTRY.get_shard(name);
    let mut shard = shard_lock
        .lock()
        .expect("Global symbol registry shard poisoned");

    if shard.name_to_symbol_key.contains_key(name) {
        return Err(SymbolError::DuplicateName(name.to_owned()));
    }

    let key = REGISTRY
        .id_to_data
        .write()
        .expect("Global ID registry poisoned")
        .insert_with_key(|k| InternedSymbol::new_named(name, k));
    shard.name_to_symbol_key.insert(name.to_owned(), key);
    drop(shard);

    Ok(Symbol(key))
}

/// Get an existing symbol by name
///
/// # Errors
/// Returns `SymbolError::NotFound` if the symbol name is not registered.
///
/// # Panics
///
/// Panics if the global registry shard lock is poisoned.
pub fn symb_get(name: &str) -> Result<Symbol, SymbolError> {
    let shard_lock = REGISTRY.get_shard(name);
    let shard = shard_lock
        .lock()
        .expect("Global symbol registry shard poisoned");

    shard
        .name_to_symbol_key
        .get(name)
        .map(|&key| Symbol(key))
        .ok_or_else(|| SymbolError::NotFound(name.to_owned()))
}

/// Check if a symbol exists
///
/// # Panics
///
/// Panics if the global registry shard lock is poisoned.
pub fn symbol_exists(name: &str) -> bool {
    let shard_lock = REGISTRY.get_shard(name);
    let shard = shard_lock
        .lock()
        .expect("Global symbol registry shard poisoned");
    shard.name_to_symbol_key.contains_key(name)
}

/// Create or get a Symbol
///
/// # Panics
///
/// Panics if any global registry lock is poisoned.
#[must_use]
pub fn symb(name: &str) -> Symbol {
    // Fast path: check TLS cache first (no locks, no allocations for common symbols)
    if let Some(sym) = NAME_CACHE.with(|cache| cache.borrow().get(name).copied()) {
        return sym;
    }

    // Slow path: acquire shard lock
    let shard_lock = REGISTRY.get_shard(name);
    let mut shard = shard_lock
        .lock()
        .expect("Global symbol registry shard poisoned");

    // Check if the symbol already exists.
    if let Some(&key) = shard.name_to_symbol_key.get(name) {
        let sym = Symbol(key);
        // Populate TLS cache for future lookups
        drop(shard); // Release lock before TLS operation
        NAME_CACHE.with(|cache| {
            cache.borrow_mut().insert(name.to_owned(), sym);
        });
        return sym;
    }

    // If not, create it.
    let key = REGISTRY
        .id_to_data
        .write()
        .expect("Global ID registry poisoned")
        .insert_with_key(|k| InternedSymbol::new_named(name, k));
    shard.name_to_symbol_key.insert(name.to_owned(), key);
    drop(shard);

    let sym = Symbol(key);
    // Populate TLS cache
    NAME_CACHE.with(|cache| {
        cache.borrow_mut().insert(name.to_owned(), sym);
    });
    sym
}

/// Get or create an interned symbol
///
/// # Panics
///
/// Panics if the global registry shard lock is poisoned.
pub fn symb_interned(name: &str) -> InternedSymbol {
    let symbol = symb(name);
    lookup_by_id(symbol.id()).expect("Just-created symbol should always be found")
}

/// Remove a symbol from the global registry
///
/// Returns `true` if the symbol existed and was removed, `false` otherwise.
///
/// # Panics
///
/// Panics if any global registry lock is poisoned.
pub fn remove_symbol(name: &str) -> bool {
    let shard_lock = REGISTRY.get_shard(name);
    let mut shard = shard_lock
        .lock()
        .expect("Global symbol registry shard poisoned");

    shard.name_to_symbol_key.remove(name).is_some_and(|key| {
        // Explicitly drop shard lock before taking id_data lock to avoid deadlocks
        drop(shard);
        REGISTRY
            .id_to_data
            .write()
            .expect("Global ID registry poisoned")
            .remove(key)
            .is_some()
    })
}

/// Clear all symbols from the global registry
///
/// # Panics
///
/// Panics if any global registry lock is poisoned.
pub fn clear_symbols() {
    // Clear TLS caches first
    ID_CACHE.with(|cache| cache.borrow_mut().clear());
    NAME_CACHE.with(|cache| cache.borrow_mut().clear());

    for shard_lock in &REGISTRY.shards {
        let mut shard = shard_lock
            .lock()
            .expect("Global symbol registry shard poisoned");
        shard.name_to_symbol_key.clear();
    }

    let mut id_data = REGISTRY
        .id_to_data
        .write()
        .expect("Global ID registry poisoned");
    id_data.clear();
}

/// Get the number of registered symbols
///
/// # Panics
///
/// Panics if any global registry shard lock is poisoned.
pub fn symbol_count() -> usize {
    let id_data = REGISTRY
        .id_to_data
        .read()
        .expect("Global ID registry poisoned");
    id_data.len()
}

/// Get a list of all registered symbol names (unsorted for performance)
///
/// # Panics
///
/// Panics if any global registry shard lock is poisoned.
#[must_use]
pub fn symbol_names() -> Vec<String> {
    let mut names = Vec::new();
    for shard_lock in &REGISTRY.shards {
        let shard = shard_lock
            .lock()
            .expect("Global symbol registry shard poisoned");
        names.extend(shard.name_to_symbol_key.keys().cloned());
    }
    // Removed sorting - caller can sort if needed
    // This avoids O(n log n) cost for use cases that don't need ordering
    names
}
