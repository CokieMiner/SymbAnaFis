//! Interned symbol implementation.
//!
//! Contains the `InternedSymbol` type that is stored in the global registry.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::registry::NEXT_SYMBOL_ID;

/// An interned symbol - the actual data stored in the registry
///
/// This is Clone-cheap because it only contains a u64 and an Arc.
#[derive(Debug, Clone)]
pub struct InternedSymbol {
    id: u64,
    name: Option<Arc<str>>,
}

impl InternedSymbol {
    /// Create a new named interned symbol
    pub(crate) fn new_named(name: &str) -> Self {
        Self {
            id: NEXT_SYMBOL_ID.fetch_add(1, Ordering::Relaxed),
            name: Some(Arc::from(name)),
        }
    }

    /// Create a new anonymous interned symbol
    pub(crate) fn new_anon() -> Self {
        Self {
            id: NEXT_SYMBOL_ID.fetch_add(1, Ordering::Relaxed),
            name: None,
        }
    }

    /// Create an anonymous symbol with a specific ID (for Symbol -> Expr when not in registry)
    pub(crate) const fn new_anon_with_id(id: u64) -> Self {
        Self { id, name: None }
    }

    /// Create a new named symbol for Context (uses external counter for isolation).
    pub(crate) fn new_named_for_context(
        name: &str,
        counter: &std::sync::atomic::AtomicU64,
    ) -> Self {
        Self {
            id: counter.fetch_add(1, Ordering::Relaxed),
            name: Some(Arc::from(name)),
        }
    }

    /// Get the symbol's unique ID
    #[inline]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Get the symbol's name (None for anonymous symbols)
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the name as Arc<str> (for cloning)
    pub fn name_arc(&self) -> Option<Arc<str>> {
        self.name.clone()
    }

    /// Get the name as &str (empty for anonymous symbols)
    #[inline]
    pub fn as_str(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }
}

// O(1) equality comparison using ID only
impl PartialEq for InternedSymbol {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for InternedSymbol {}

// Hash by ID for O(1) HashMap operations
impl std::hash::Hash for InternedSymbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash by ID for consistency with PartialEq (which now uses ID only)
        self.id.hash(state);
    }
}

// Allow display for debugging and error messages
impl std::fmt::Display for InternedSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(n) => write!(f, "{n}"),
            None => write!(f, "${}", self.id),
        }
    }
}

// Allow conversion to &str for APIs that need it
impl AsRef<str> for InternedSymbol {
    fn as_ref(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }
}

// Support ordering for canonical forms
impl PartialOrd for InternedSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InternedSymbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare by name first, then by ID for anonymous symbols
        match (&self.name, &other.name) {
            (Some(a), Some(b)) => a.cmp(b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => self.id.cmp(&other.id),
        }
    }
}
