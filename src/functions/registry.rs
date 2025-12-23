use crate::Expr;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::{Arc, OnceLock};

/// Definition of a mathematical function including its evaluation and differentiation logic.
///
/// This struct defines how a function behaves numerically (via `eval`) and symbolically
/// (via `derivative`). It is used by the function registry to look up function behavior
/// during evaluation and differentiation.
///
/// # Creating Custom Functions
///
/// To create a custom function, use the `new()` constructor:
///
/// ```
/// use symb_anafis::{Expr, FunctionDefinition};
/// use std::sync::Arc;
///
/// let my_square = FunctionDefinition::new(
///     "my_square",
///     1..=1,  // exactly 1 argument
///     |args| Some(args[0] * args[0]),  // numerical evaluation
///     |args, derivs| {
///         // d/dx[my_square(u)] = 2*u * u'
///         let u = &args[0];
///         let u_prime = &derivs[0];
///         Expr::product(vec![
///             Expr::number(2.0),
///             (**u).clone(),
///             u_prime.clone(),
///         ])
///     },
/// );
/// ```
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
    /// Returns the total derivative dA/dx = sum( (dA/d_arg_i) * (d_arg_i/dx) )
    pub(crate) derivative: fn(&[Arc<Expr>], &[Expr]) -> Expr,
}

impl FunctionDefinition {
    /// Create a new function definition.
    ///
    /// # Arguments
    /// * `name` - Static string name of the function (e.g., "sin", "my_func")
    /// * `arity` - Range of acceptable argument counts (e.g., `1..=1` for unary, `2..=3` for 2-3 args)
    /// * `eval` - Numerical evaluation: takes argument values, returns computed result (or `None` for undefined)
    /// * `derivative` - Symbolic differentiation: takes Arc arguments and their derivatives, returns combined derivative
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{Expr, FunctionDefinition};
    /// use std::sync::Arc;
    ///
    /// // Define cube(x) = x³ with derivative 3x²·x'
    /// let cube_fn = FunctionDefinition::new(
    ///     "cube",
    ///     1..=1,
    ///     |args| Some(args[0].powi(3)),
    ///     |args, derivs| {
    ///         // 3 * x^2 * x'
    ///         Expr::product(vec![
    ///             Expr::number(3.0),
    ///             Expr::pow((**args.get(0).unwrap()).clone(), Expr::number(2.0)),
    ///             derivs[0].clone(),
    ///         ])
    ///     },
    /// );
    /// assert_eq!(cube_fn.name(), "cube");
    /// ```
    pub fn new(
        name: &'static str,
        arity: RangeInclusive<usize>,
        eval: fn(&[f64]) -> Option<f64>,
        derivative: fn(&[Arc<Expr>], &[Expr]) -> Expr,
    ) -> Self {
        Self {
            name,
            arity,
            eval,
            derivative,
        }
    }

    /// Get the function name
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Get the arity range (acceptable number of arguments)
    pub fn arity(&self) -> &RangeInclusive<usize> {
        &self.arity
    }

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
