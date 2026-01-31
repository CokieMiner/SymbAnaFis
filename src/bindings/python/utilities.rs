//! Python bindings for utility functions
//!
//! This module provides utility functions for symbol management and introspection.

use crate::{clear_symbols, symbol_count, symbol_exists, symbol_names};
use pyo3::prelude::*;

/// Clear all registered symbols
#[pyfunction]
#[pyo3(name = "clear_symbols")]
pub fn py_clear_symbols() {
    clear_symbols();
}

/// Get the number of registered symbols
#[pyfunction]
#[pyo3(name = "symbol_count")]
pub fn py_symbol_count() -> usize {
    symbol_count()
}

/// Get all registered symbol names
#[pyfunction]
#[pyo3(name = "symbol_names")]
pub fn py_symbol_names() -> Vec<String> {
    symbol_names()
}

/// Check if a symbol exists
#[pyfunction]
#[pyo3(name = "symbol_exists")]
pub fn py_symbol_exists(name: &str) -> bool {
    symbol_exists(name)
}
