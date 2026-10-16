//! Allocator-backed hash collections used by expression queries and transforms.
//!
//! These types are identical with and without the `std` feature. Their default
//! `FoldHash` builder requires no operating-system entropy and is not a
//! cryptographic hash or a guarantee against adversarial collision attacks.

pub use hashbrown::{HashMap, HashSet};
