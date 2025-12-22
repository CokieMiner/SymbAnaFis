use crate::Expr;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::{Arc, OnceLock};

/// Definition of a mathematical function including its evaluation and differentiation logic
#[derive(Clone)]
pub(crate) struct FunctionDefinition {
    /// Canonical name of the function (e.g., "sin", "besselj")
    pub name: &'static str,

    /// Acceptable argument count (arity)
    pub arity: RangeInclusive<usize>,

    /// Numerical evaluation function
    pub eval: fn(&[f64]) -> Option<f64>,

    /// Symbolic differentiation function
    /// Arguments: (args of the function call as Arc, derivatives of the arguments)
    /// Returns the total derivative dA/dx = sum( (dA/d_arg_i) * (d_arg_i/dx) )
    pub derivative: fn(&[Arc<Expr>], &[Expr]) -> Expr,
}

impl FunctionDefinition {
    /// Helper to check if argument count is valid
    #[inline]
    pub(crate) fn validate_arity(&self, args: usize) -> bool {
        self.arity.contains(&args)
    }
}

use crate::core::symbol::{InternedSymbol, get_or_intern};

/// Static registry storing all function definitions
/// Maps symbol ID -> FunctionDefinition for fast O(1) lookup
static REGISTRY: OnceLock<HashMap<u64, FunctionDefinition>> = OnceLock::new();

/// Initialize the registry with all function definitions
fn init_registry() -> HashMap<u64, FunctionDefinition> {
    let mut map = HashMap::with_capacity(70);

    // Populate from definitions
    for def in crate::functions::definitions::all_definitions() {
        // Intern the name to get its ID
        let sym = get_or_intern(def.name);
        map.insert(sym.id(), def);
    }

    map
}

/// Central registry for getting function definitions
pub(crate) struct Registry;

impl Registry {
    /// Get a function definition by symbol - O(1) lookup using ID
    pub(crate) fn get_by_symbol(sym: &InternedSymbol) -> Option<&'static FunctionDefinition> {
        REGISTRY.get_or_init(init_registry).get(&sym.id())
    }
}
