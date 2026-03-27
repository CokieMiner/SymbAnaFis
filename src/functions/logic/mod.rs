pub(super) mod definitions;
pub(super) mod registry;

// Staircase re-export — one hop up to api.rs
pub use registry::Registry;
