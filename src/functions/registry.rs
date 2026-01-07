use crate::Expr;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::{Arc, OnceLock};

/// Definition of a mathematical function including its evaluation and differentiation logic.
///
/// This struct defines how a function behaves numerically (via `eval`) and symbolically
/// (via `derivative`). It is used by the function registry to look up function behavior
/// during evaluation and differentiation.
#[derive(Clone, Debug)]
pub struct FunctionDefinition {
    /// Canonical name of the function (e.g., "sin", "besselj")
    pub(crate) name: &'static str,

    /// Acceptable argument count (arity)
    pub(crate) arity: RangeInclusive<usize>,

    /// Numerical evaluation function
    pub(crate) eval: fn(&[f64]) -> Option<f64>,

    /// Symbolic differentiation function
    /// Arguments: (args of the function call as Arc, derivatives of the arguments)
    /// Returns the total derivative dA/dx = sum( (`dA/d_arg_i`) * (`d_arg_i/dx`) )
    pub(crate) derivative: fn(&[Arc<Expr>], &[Expr]) -> Expr,
}

impl FunctionDefinition {
    /// Helper to check if argument count is valid
    #[inline]
    pub(crate) fn validate_arity(&self, args: usize) -> bool {
        self.arity.contains(&args)
    }
}

use crate::core::symbol::{InternedSymbol, symb_interned};

/// Static registry storing all function definitions
/// Maps symbol ID -> `FunctionDefinition` for fast O(1) lookup
static REGISTRY: OnceLock<HashMap<u64, FunctionDefinition>> = OnceLock::new();

/// Initialize the registry with all function definitions
fn init_registry() -> HashMap<u64, FunctionDefinition> {
    let mut map = HashMap::with_capacity(70);

    // Populate from definitions
    for def in crate::functions::definitions::all_definitions() {
        // Intern the name to get its ID
        let sym = symb_interned(def.name);
        map.insert(sym.id(), def);
    }

    map
}

/// Central registry for getting function definitions
pub struct Registry;

impl Registry {
    /// Get a function definition by symbol - O(1) lookup using ID
    pub(crate) fn get_by_symbol(sym: &InternedSymbol) -> Option<&'static FunctionDefinition> {
        REGISTRY.get_or_init(init_registry).get(&sym.id())
    }
}
