//! Global symbol registry management.
//!
//! Contains the global registries and public API functions for symbol management.
//! This implementation uses sharding to minimize lock contention and `FxHash` for
//! high-performance mapping.

use std::sync::RwLock;
use std::sync::atomic::AtomicU64;

use rustc_hash::{FxHashMap, FxHasher};
use std::hash::Hasher;

use super::interned::InternedSymbol;
use super::{Symbol, SymbolError};

// ============================================================================
// Global Symbol Registry
// ============================================================================

/// Global counter for symbol IDs (shared across modules)
pub static NEXT_SYMBOL_ID: AtomicU64 = AtomicU64::new(1);

const NUM_SHARDS: usize = 16;

struct RegistryShard {
    // Use FxHashMap for faster lookups with short symbol names
    name_to_symbol: FxHashMap<String, InternedSymbol>,
}

/// Unified symbol registry storage
struct SymbolRegistry {
    // Shards for Name -> Symbol mapping to reduce contention
    shards: [RwLock<RegistryShard>; NUM_SHARDS],
    // ID -> Symbol mapping (Sequential, O(1) lookup, using Vec instead of HashMap)
    id_to_data: RwLock<Vec<Option<InternedSymbol>>>,
}

impl SymbolRegistry {
    fn new() -> Self {
        let shards: [RwLock<RegistryShard>; NUM_SHARDS] = std::array::from_fn(|_| {
            RwLock::new(RegistryShard {
                name_to_symbol: FxHashMap::default(),
            })
        });

        Self {
            shards,
            id_to_data: RwLock::new(Vec::with_capacity(128)),
        }
    }

    /// # Panics
    ///
    /// Panics if the global registry hash cannot be computed.
    fn get_shard(&self, name: &str) -> &RwLock<RegistryShard> {
        // Use FxHasher for sharding to stay consistent and fast
        let mut hasher = FxHasher::default();
        std::hash::Hash::hash(name, &mut hasher);
        let hash = hasher.finish();

        // Truncation is safe/expected here as we only need the low bits for sharding (hash % 16)
        #[allow(clippy::cast_possible_truncation)]
        let shard_idx = (hash as usize) % NUM_SHARDS;
        &self.shards[shard_idx]
    }
}

/// Global registry for symbols
static REGISTRY: std::sync::LazyLock<SymbolRegistry> =
    std::sync::LazyLock::new(SymbolRegistry::new);

/// Look up `InternedSymbol` by ID (for Symbol -> Expr conversion and `known_symbols`)
///
/// # Panics
///
/// Panics if the global ID registry lock is poisoned.
pub fn lookup_by_id(id: u64) -> Option<InternedSymbol> {
    let guard = REGISTRY
        .id_to_data
        .read()
        .expect("Global ID registry poisoned");
    // Symbol IDs are sequential; 4 billion symbols would exceed system memory before 32-bit truncation
    #[allow(clippy::cast_possible_truncation)]
    let idx = id as usize;
    guard.get(idx).and_then(Clone::clone)
}

/// Register an `InternedSymbol` in the ID registry (for Context integration)
///
/// # Panics
///
/// Panics if the global ID registry lock is poisoned.
pub fn register_in_id_registry(id: u64, interned: InternedSymbol) {
    let mut guard = REGISTRY
        .id_to_data
        .write()
        .expect("Global ID registry poisoned");

    // Symbol IDs are sequential; 4 billion symbols would exceed system memory before 32-bit truncation
    #[allow(clippy::cast_possible_truncation)]
    let idx = id as usize;
    if guard.len() <= idx {
        let new_len = idx + 32;
        guard.resize(new_len, None);
    }

    guard[idx] = Some(interned);
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
        .write()
        .expect("Global symbol registry shard poisoned");

    if shard.name_to_symbol.contains_key(name) {
        return Err(SymbolError::DuplicateName(name.to_owned()));
    }

    let interned = InternedSymbol::new_named(name);
    let id = interned.id();

    // Consistency: Update both registries while holding the shard lock
    register_in_id_registry(id, interned.clone());
    shard.name_to_symbol.insert(name.to_owned(), interned);
    drop(shard); // Early drop to reduce contention

    Ok(Symbol(id))
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
        .read()
        .expect("Global symbol registry shard poisoned");

    shard
        .name_to_symbol
        .get(name)
        .map(|s| Symbol(s.id()))
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
        .read()
        .expect("Global symbol registry shard poisoned");
    shard.name_to_symbol.contains_key(name)
}

/// Create or get a Symbol
#[must_use]
pub fn symb(name: &str) -> Symbol {
    let interned = symb_interned(name);
    Symbol(interned.id())
}

/// Get or create an interned symbol
///
/// # Panics
///
/// Panics if the global registry shard lock is poisoned.
pub fn symb_interned(name: &str) -> InternedSymbol {
    let shard_lock = REGISTRY.get_shard(name);

    // Fast Path: Read Lock (common case)
    {
        let shard = shard_lock
            .read()
            .expect("Global symbol registry shard poisoned");
        if let Some(sym) = shard.name_to_symbol.get(name) {
            return sym.clone();
        }
    }

    // Slow Path: Write Lock
    let mut shard = shard_lock
        .write()
        .expect("Global symbol registry shard poisoned");

    // Double-check after acquiring write lock
    if let Some(sym) = shard.name_to_symbol.get(name) {
        return sym.clone();
    }

    let interned = InternedSymbol::new_named(name);
    let id = interned.id();

    // Establish valid state in ID registry before making it visible in Shard
    register_in_id_registry(id, interned.clone());
    shard
        .name_to_symbol
        .insert(name.to_owned(), interned.clone());
    drop(shard); // Early drop to reduce contention

    interned
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
        .write()
        .expect("Global symbol registry shard poisoned");

    // Remove from name mapping and get the symbol to know its ID
    let sym_opt = shard.name_to_symbol.remove(name);
    // Explicitly drop shard lock before taking id_data lock to avoid potential deadlocks
    drop(shard);

    sym_opt.is_some_and(|sym| {
        let mut id_data = REGISTRY
            .id_to_data
            .write()
            .expect("Global ID registry poisoned");

        // Symbol IDs are sequential; 4 billion symbols would exceed system memory before 32-bit truncation
        #[allow(clippy::cast_possible_truncation)]
        let idx = sym.id() as usize;
        if idx < id_data.len() {
            id_data[idx] = None;
        }
        true
    })
}

/// Clear all symbols from the global registry
///
/// # Panics
///
/// Panics if any global registry lock is poisoned.
pub fn clear_symbols() {
    for shard_lock in &REGISTRY.shards {
        let mut shard = shard_lock
            .write()
            .expect("Global symbol registry shard poisoned");
        shard.name_to_symbol.clear();
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
    REGISTRY
        .shards
        .iter()
        .map(|shard_lock| {
            shard_lock
                .read()
                .expect("Global symbol registry shard poisoned")
                .name_to_symbol
                .len()
        })
        .sum()
}

/// Get a list of all registered symbol names
///
/// # Panics
///
/// Panics if any global registry shard lock is poisoned.
#[must_use]
pub fn symbol_names() -> Vec<String> {
    let mut names = Vec::new();
    for shard_lock in &REGISTRY.shards {
        let shard = shard_lock
            .read()
            .expect("Global symbol registry shard poisoned");
        names.extend(shard.name_to_symbol.keys().cloned());
    }
    names.sort();
    names
}
