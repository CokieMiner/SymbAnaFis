//! Global name publication, targeted removal, and consistent introspection.

use core::hash::Hash;

use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use super::{
    FixedState, HashMap, INDEX, Mutex, SHARDS, Storage, Symbol, SymbolData, SymbolError,
    SymbolRegistry,
};

impl SymbolRegistry {
    /// Create an independent binding registry with sixteen name shards.
    pub const fn new() -> Self {
        Self {
            shards: [const { Mutex::new(Storage::new()) }; SHARDS],
        }
    }

    /// Get or create a global name under its publication lock.
    pub fn intern(&self, name: &str) -> Result<Arc<SymbolData>, SymbolError> {
        let mut storage = self.name_shard(name).lock();
        if let Some(existing) = storage.names.get(name) {
            return Ok(Arc::clone(existing));
        }
        let text: Arc<str> = Arc::from(name);
        let data = SymbolData::permanent(Some(Arc::clone(&text)))?;
        storage.names.insert(text, Arc::clone(&data));
        drop(storage);
        Ok(data)
    }

    /// Register a new name, rejecting duplicates in this registry.
    pub fn register(&self, name: &str) -> Result<Arc<SymbolData>, SymbolError> {
        let mut storage = self.name_shard(name).lock();
        if storage.names.contains_key(name) {
            return Err(SymbolError::DuplicateName(name.into()));
        }
        let text: Arc<str> = Arc::from(name);
        let data = SymbolData::permanent(Some(Arc::clone(&text)))?;
        storage.names.insert(text, Arc::clone(&data));
        drop(storage);
        Ok(data)
    }

    /// Register an anonymous symbol with permanent identity metadata.
    pub fn anonymous(&self) -> Result<Arc<SymbolData>, SymbolError> {
        let data = SymbolData::allocate(None)?;
        let mut storage = self.id_shard(data.id).lock();
        INDEX.publish_permanent(&data);
        storage.anonymous.insert(data.id, Arc::clone(&data));
        drop(storage);
        Ok(data)
    }

    /// Query a global name without creating it.
    pub fn get(&self, name: &str) -> Result<Symbol<'static>, SymbolError> {
        self.name_shard(name)
            .lock()
            .names
            .get(name)
            .map(|data| data.permanent_symbol())
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

    /// Remove a matching global or anonymous registration. Identity metadata
    /// remains available, and local context bindings are unaffected.
    pub fn remove_id(&self, id: usize) -> bool {
        let Ok(data) = SymbolData::lookup(id) else {
            return false;
        };
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
        let guards: Vec<_> = self.shards.iter().map(Mutex::lock).collect();
        guards
            .iter()
            .map(|storage| storage.names.len().saturating_add(storage.anonymous.len()))
            .sum()
    }

    /// List global names in unspecified order.
    pub fn names(&self) -> Vec<String> {
        let guards: Vec<_> = self.shards.iter().map(Mutex::lock).collect();
        guards
            .iter()
            .flat_map(|storage| storage.names.keys().map(ToString::to_string))
            .collect()
    }
}

impl Storage {
    const fn new() -> Self {
        Self {
            names: HashMap::with_hasher(FixedState::with_seed(0)),
            anonymous: HashMap::with_hasher(FixedState::with_seed(0)),
        }
    }

    fn compact<Key: Eq + Hash, Value>(bindings: &mut HashMap<Key, Value, FixedState>) {
        if bindings.is_empty() {
            *bindings = HashMap::with_hasher(FixedState::with_seed(0));
        } else if bindings.capacity() > bindings.len().saturating_mul(4).max(64) {
            bindings.shrink_to_fit();
        }
    }
}
