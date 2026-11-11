//! Allocator-backed hash collections shared by every subsystem.
//!
//! The types are identical with and without the `std` feature. The default
//! `FoldHash` builder needs no operating-system entropy; it is neither a
//! cryptographic hash nor a defense against adversarial collisions.
//!
//! Internal scratch tables use [`FixedMap`] and [`FixedSet`], whose hasher
//! has a fixed seed. The default builder derives a seed for every new map
//! from a process-wide atomic; a fixed seed makes construction free, so
//! short-lived traversal tables cost only their allocation. Their keys are
//! node pointers and other internal data, never untrusted input.

pub use foldhash::fast::FixedState;
pub use hashbrown::{Equivalent, HashMap, HashSet, HashTable, hash_map::Entry};

/// A hash map with a fixed hasher seed; build it with `FixedMap::default()`.
pub type FixedMap<Key, Value> = HashMap<Key, Value, FixedState>;

/// A hash set with a fixed hasher seed; build it with `FixedSet::default()`.
pub type FixedSet<Value> = HashSet<Value, FixedState>;
