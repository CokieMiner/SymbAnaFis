//! Synchronization primitives and cache-line padding for internal locks.

mod mutex;

pub use mutex::{CachePadded, Mutex};

#[cfg(test)]
mod tests;
