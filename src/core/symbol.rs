//! Type-safe Symbol and operator overloading for ergonomic expression building
//!
//! # Symbol Interning
//!
//! Symbols are interned globally for O(1) equality comparisons. Each unique symbol name
//! exists exactly once in memory, and all references share the same ID.
//!
//! # Example
//! ```
//! use symb_anafis::{symb, symb_new, symb_get, clear_symbols};
//!
//! // Create or get a symbol (doesn't error on existing)
//! let x = symb("doc_example_x");
//!
//! // Get the same symbol again
//! let x2 = symb("doc_example_x");
//! assert_eq!(x.id(), x2.id());  // Same symbol!
//!
//! // Anonymous symbol (always succeeds)
//! use symb_anafis::Symbol;
//! let temp = Symbol::anon();
//! assert!(temp.name().is_none());
//! ```

use crate::Expr;
use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

// ============================================================================
// Global Symbol Registry
// ============================================================================

/// Global counter for symbol IDs (shared across modules)
pub static NEXT_SYMBOL_ID: AtomicU64 = AtomicU64::new(1);

/// Global symbol registry: name -> `InternedSymbol`
static SYMBOL_REGISTRY: std::sync::LazyLock<RwLock<HashMap<String, InternedSymbol>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

/// Reverse lookup: ID -> `InternedSymbol` (for Copy Symbol to find its data)
static ID_REGISTRY: std::sync::LazyLock<RwLock<HashMap<u64, InternedSymbol>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

fn get_registry_mut() -> std::sync::RwLockWriteGuard<'static, HashMap<String, InternedSymbol>> {
    SYMBOL_REGISTRY.write().unwrap()
}

fn get_id_registry_mut() -> std::sync::RwLockWriteGuard<'static, HashMap<u64, InternedSymbol>> {
    ID_REGISTRY.write().unwrap()
}

fn get_id_registry_read() -> std::sync::RwLockReadGuard<'static, HashMap<u64, InternedSymbol>> {
    ID_REGISTRY.read().unwrap()
}

fn get_registry_read() -> std::sync::RwLockReadGuard<'static, HashMap<String, InternedSymbol>> {
    SYMBOL_REGISTRY.read().unwrap()
}

fn with_registry_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, InternedSymbol>, &mut HashMap<u64, InternedSymbol>) -> R,
{
    let mut registry = get_registry_mut();
    let mut id_registry = get_id_registry_mut();
    f(&mut registry, &mut id_registry)
}

/// Look up `InternedSymbol` by ID (for Symbol -> Expr conversion and `known_symbols`)
pub fn lookup_by_id(id: u64) -> Option<InternedSymbol> {
    let guard = get_id_registry_read();
    guard.get(&id).cloned()
}

/// Register an `InternedSymbol` in the ID registry (for Context integration)
pub fn register_in_id_registry(id: u64, interned: InternedSymbol) {
    let mut id_registry = get_id_registry_mut();
    id_registry.insert(id, interned);
}

// ============================================================================
// Symbol Error Type
// ============================================================================

/// Errors that can occur during symbol operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolError {
    /// Attempted to create a symbol with a name that's already registered
    DuplicateName(String),
    /// Attempted to get a symbol that doesn't exist
    NotFound(String),
}

impl std::fmt::Display for SymbolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateName(name) => {
                write!(
                    f,
                    "Symbol '{name}' is already registered. Use symb_get() to retrieve it."
                )
            }
            Self::NotFound(name) => {
                write!(
                    f,
                    "Symbol '{name}' not found. Use symb() to create it first."
                )
            }
        }
    }
}

impl std::error::Error for SymbolError {}

// ============================================================================
// Interned Symbol (Internal)
// ============================================================================

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
    fn new_named(name: &str) -> Self {
        Self {
            id: NEXT_SYMBOL_ID.fetch_add(1, Ordering::Relaxed),
            name: Some(Arc::from(name)),
        }
    }

    /// Create a new anonymous interned symbol
    fn new_anon() -> Self {
        Self {
            id: NEXT_SYMBOL_ID.fetch_add(1, Ordering::Relaxed),
            name: None,
        }
    }

    /// Create a new named symbol for Context (uses external counter for isolation).
    pub(crate) fn new_named_for_context(name: &str, counter: &AtomicU64) -> Self {
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

// ============================================================================
// Public Symbol Type
// ============================================================================

/// Type-safe symbol for building expressions ergonomically
///
/// Symbols are interned - each unique name exists exactly once, and all
/// references share the same ID for O(1) equality comparisons.
///
/// **This type is `Copy`** - you can use it in expressions without `.clone()`:
/// ```
/// use symb_anafis::symb;
/// let a = symb("symbol_doc_a");
/// let expr = a + a;  // Works! No clone needed.
/// assert!(format!("{}", expr).contains("symbol_doc_a"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(u64); // Just the ID - lightweight and Copy!

impl Symbol {
    /// Create a Symbol from a raw ID.
    ///
    /// This is used by Context to create symbols from its isolated registry.
    #[inline]
    pub(crate) const fn from_id(id: u64) -> Self {
        Self(id)
    }

    /// Create a new anonymous symbol (always succeeds)
    ///
    /// Anonymous symbols have a unique ID but no string name.
    /// They cannot be retrieved by name and are useful for intermediate computations.
    #[must_use] 
    pub fn anon() -> Self {
        let interned = InternedSymbol::new_anon();
        // Optimization: Don't register anonymous symbols in the global registry.
        // This avoids:
        // 1. Global Write Lock contention (performance)
        // 2. Memory leaks (registry growing indefinitely with temp symbols)
        //
        // Symbol::to_expr() and name() handle missing registry entries gracefully
        // by assuming they are anonymous.
        Self(interned.id())
    }

    /// Get the symbol's unique ID
    #[inline]
    #[must_use] 
    pub const fn id(&self) -> u64 {
        self.0
    }

    /// Get the name of the symbol (None for anonymous symbols)
    ///
    /// Note: This clones the name string. For frequent access, consider `name_arc()`.
    #[must_use] 
    pub fn name(&self) -> Option<String> {
        self.name_arc().map(|arc| arc.to_string())
    }

    /// Get the name as an `Arc<str>` (avoiding String allocation)
    #[must_use] 
    pub fn name_arc(&self) -> Option<Arc<str>> {
        lookup_by_id(self.0).and_then(|s| s.name)
    }

    /// Convert to an Expr
    #[must_use] 
    pub fn to_expr(&self) -> Expr {
        // Look up the InternedSymbol from registry
        lookup_by_id(self.0).map_or_else(|| Expr::from_interned(InternedSymbol {
            id: self.0,
            name: None,
        }), Expr::from_interned)
    }

    /// Raise to a power (Copy means no clone needed)
    pub fn pow(&self, exp: impl Into<Expr>) -> Expr {
        Expr::pow_static(self.to_expr(), exp.into())
    }

    // === Parametric special functions ===

    /// Polygamma function: ψ^(n)(x)
    /// `x.polygamma(n)` → `polygamma(n, x)`
    pub fn polygamma(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("polygamma", vec![n.into(), self.to_expr()])
    }

    /// Beta function: B(a, b)
    pub fn beta(&self, other: impl Into<Expr>) -> Expr {
        Expr::func_multi("beta", vec![self.to_expr(), other.into()])
    }

    /// Bessel function of the first kind: `J_n(x)`
    pub fn besselj(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("besselj", vec![n.into(), self.to_expr()])
    }

    /// Bessel function of the second kind: `Y_n(x)`
    pub fn bessely(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("bessely", vec![n.into(), self.to_expr()])
    }

    /// Modified Bessel function of the first kind: `I_n(x)`
    pub fn besseli(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("besseli", vec![n.into(), self.to_expr()])
    }

    /// Modified Bessel function of the second kind: `K_n(x)`
    pub fn besselk(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("besselk", vec![n.into(), self.to_expr()])
    }

    /// Logarithm with arbitrary base: `x.log(base)` → `log(base, x)`
    ///
    /// # Example
    /// ```
    /// use symb_anafis::symb;
    /// let x = symb("log_example_x");
    /// let log_base_2 = x.log(2.0);  // log(2, x)
    /// assert_eq!(format!("{}", log_base_2), "log(2, log_example_x)");
    /// ```
    pub fn log(&self, base: impl Into<Expr>) -> Expr {
        Expr::func_multi("log", vec![base.into(), self.to_expr()])
    }

    /// Two-argument arctangent: atan2(self, x) = angle to point (x, self)
    pub fn atan2(&self, x: impl Into<Expr>) -> Expr {
        Expr::func_multi("atan2", vec![self.to_expr(), x.into()])
    }

    /// Hermite polynomial `H_n(self)`
    pub fn hermite(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("hermite", vec![n.into(), self.to_expr()])
    }

    /// Associated Legendre polynomial `P_l^m(self)`
    pub fn assoc_legendre(&self, l: impl Into<Expr>, m: impl Into<Expr>) -> Expr {
        Expr::func_multi("assoc_legendre", vec![l.into(), m.into(), self.to_expr()])
    }

    /// Spherical harmonic `Y_l^m(theta`, phi) where self is theta
    pub fn spherical_harmonic(
        &self,
        l: impl Into<Expr>,
        m: impl Into<Expr>,
        phi: impl Into<Expr>,
    ) -> Expr {
        Expr::func_multi(
            "spherical_harmonic",
            vec![l.into(), m.into(), self.to_expr(), phi.into()],
        )
    }

    /// Alternative spherical harmonic notation `Y_l^m(theta`, phi)
    pub fn ynm(&self, l: impl Into<Expr>, m: impl Into<Expr>, phi: impl Into<Expr>) -> Expr {
        Expr::func_multi("ynm", vec![l.into(), m.into(), self.to_expr(), phi.into()])
    }

    /// Derivative of Riemann zeta function: zeta^(n)(self)
    pub fn zeta_deriv(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("zeta_deriv", vec![n.into(), self.to_expr()])
    }
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Create a new named symbol (errors if name already registered)
///
/// Use this to define your mathematical variables. Each name can only be
/// registered once - attempting to register the same name twice will error.
///
/// # Example
/// ```
/// use symb_anafis::{symb_new, symbol_exists};
/// // Use unique names to avoid conflicts with other tests
/// if !symbol_exists("symb_new_doc_x") {
///     let x = symb_new("symb_new_doc_x").unwrap();
///     let y = symb_new("symb_new_doc_y").unwrap();
///     let expr = x.pow(2.0) + y;
///     assert!(format!("{}", expr).contains("symb_new_doc"));
/// }
/// ```
///
/// # Errors
/// Returns `SymbolError::DuplicateName` if a symbol with this name already exists.
pub fn symb_new(name: &str) -> Result<Symbol, SymbolError> {
    with_registry_mut(|registry, id_registry| {
        if registry.contains_key(name) {
            return Err(SymbolError::DuplicateName(name.to_string()));
        }
        let interned = InternedSymbol::new_named(name);
        let id = interned.id();
        registry.insert(name.to_string(), interned.clone());
        id_registry.insert(id, interned);
        Ok(Symbol(id))
    })
}

/// Get an existing symbol by name (errors if not found)
///
/// Use this to retrieve a symbol that was previously created with `symb_new()`.
/// The returned symbol shares the same ID as the original.
///
/// # Example
/// ```
/// use symb_anafis::{symb, symb_get};
/// let x = symb("symb_get_doc_x");
/// let x2 = symb_get("symb_get_doc_x").unwrap();
/// assert_eq!(x.id(), x2.id());  // Same symbol!
/// ```
///
/// # Errors
/// Returns `SymbolError::NotFound` if no symbol with this name exists.
pub fn symb_get(name: &str) -> Result<Symbol, SymbolError> {
    let guard = get_registry_read();
    guard.get(name).map_or_else(|| Err(SymbolError::NotFound(name.to_string())), |interned| Ok(Symbol(interned.id())))
}

/// Check if a symbol name is already registered
#[must_use] 
pub fn symbol_exists(name: &str) -> bool {
    let guard = get_registry_read();
    guard.contains_key(name)
}

/// Get the number of registered symbols in the global registry
///
/// # Example
/// ```
/// use symb_anafis::{symb, symbol_count};
/// symb("symbol_count_doc_x");
/// symb("symbol_count_doc_y");
/// assert!(symbol_count() >= 2);
/// ```
#[must_use] 
pub fn symbol_count() -> usize {
    let guard = get_registry_read();
    guard.len()
}

/// List all symbol names in the global registry
///
/// # Example
/// ```
/// use symb_anafis::{symb, symbol_names};
/// symb("symbol_names_doc_x");
/// let names = symbol_names();
/// assert!(names.contains(&"symbol_names_doc_x".to_string()));
/// ```
#[must_use] 
pub fn symbol_names() -> Vec<String> {
    let guard = get_registry_read();
    guard.keys().cloned().collect()
}

/// Clear all registered symbols from the global registry
///
/// This resets the symbol table. Use with caution - any existing Symbol
/// instances will still work but won't match newly created ones by name lookup.
pub fn clear_symbols() {
    get_registry_mut().clear();
    get_id_registry_mut().clear();
}

/// Remove a specific symbol from the registry
///
/// Returns true if the symbol was removed, false if it didn't exist.
/// This completely removes the symbol from both name and ID registries.
/// Use with caution - any existing Symbol instances will become invalid.
///
/// # Warning
/// Removing a symbol does NOT reset the global ID counter. Creating a new symbol
/// with the same name afterwards will result in a **different** ID.
/// ```rust
/// # use symb_anafis::{symb, remove_symbol};
/// let x1 = symb("x"); // ID=1
/// remove_symbol("x");
/// let x2 = symb("x"); // ID=2 (x1 != x2)
/// ```
/// This can lead to subtle bugs where two "x" variables are mathematically distinct.
///
/// # Example
/// ```
/// use symb_anafis::{symb, symbol_exists, remove_symbol};
/// symb("remove_symbol_doc_x");
/// assert!(symbol_exists("remove_symbol_doc_x"));
/// remove_symbol("remove_symbol_doc_x");
/// assert!(!symbol_exists("remove_symbol_doc_x"));
/// ```
#[must_use] 
pub fn remove_symbol(name: &str) -> bool {
    with_registry_mut(|name_registry, id_registry| {
        name_registry.remove(name).is_some_and(|interned| {
            // Also remove from ID registry to prevent inconsistent state
            id_registry.remove(&interned.id());
            true
        })
    })
}

/// Create or get a Symbol (main public API)
///
/// This is the primary way to create symbols. It gets an existing symbol with the given
/// name or creates a new one if it doesn't exist.
///
/// # Example
/// ```
/// use symb_anafis::symb;
/// let x = symb("x");
/// let y = symb("y");
/// let expr = x + y;
/// ```
#[must_use] 
pub fn symb(name: &str) -> Symbol {
    let interned = symb_interned(name);
    Symbol(interned.id())
}

/// Get or create an interned symbol (internal use for parser)
///
/// Unlike `symb()`, this does NOT error on duplicates - it returns the existing symbol.
/// This is used by the parser to ensure parsed expressions use the same symbols.
pub fn symb_interned(name: &str) -> InternedSymbol {
    with_registry_mut(|registry, id_registry| {
        if let Some(existing) = registry.get(name) {
            return existing.clone();
        }
        let interned = InternedSymbol::new_named(name);
        let id = interned.id();
        registry.insert(name.to_string(), interned.clone());
        id_registry.insert(id, interned.clone());
        interned
    })
}

// ============================================================================
// Macro for generating math function methods
// ============================================================================

/// Generate math function methods for a type (taking &self for Symbol, self for Expr)
macro_rules! impl_math_functions_ref {
    ($type:ty, $converter:expr, $($fn_name:ident => $func_str:literal),* $(,)?) => {
        impl $type {
            $(
                #[doc = concat!("Apply the `", $func_str, "` function to this expression.")]
                pub fn $fn_name(&self) -> Expr {
                    Expr::func($func_str, $converter(self))
                }
            )*
        }
    };
}

/// Generate math function methods for Expr (consumes self)
macro_rules! impl_math_functions_owned {
    ($type:ty, $converter:expr, $($fn_name:ident => $func_str:literal),* $(,)?) => {
        impl $type {
            $(
                #[doc = concat!("Apply the `", $func_str, "` function to this expression.")]
                #[must_use]
                pub fn $fn_name(self) -> Expr {
                    Expr::func($func_str, $converter(self))
                }
            )*
        }
    };
}

macro_rules! math_function_list_ref {
    ($macro_name:ident, $type:ty, $converter:expr) => {
        $macro_name!($type, $converter,
            // Trigonometric functions
            sin => "sin", cos => "cos", tan => "tan",
            cot => "cot", sec => "sec", csc => "csc",
            // Inverse trigonometric functions
            asin => "asin", acos => "acos", atan => "atan",
            acot => "acot", asec => "asec", acsc => "acsc",
            // Hyperbolic functions
            sinh => "sinh", cosh => "cosh", tanh => "tanh",
            coth => "coth", sech => "sech", csch => "csch",
            // Inverse hyperbolic functions
            asinh => "asinh", acosh => "acosh", atanh => "atanh",
            acoth => "acoth", asech => "asech", acsch => "acsch",
            // Exponential and logarithmic functions
            exp => "exp", ln => "ln",
            log10 => "log10", log2 => "log2",
            // Root functions
            sqrt => "sqrt", cbrt => "cbrt",
            // Rounding functions
            floor => "floor", ceil => "ceil", round => "round",
            // Special functions (single-argument only)
            abs => "abs", signum => "signum", sinc => "sinc",
            erf => "erf", erfc => "erfc", gamma => "gamma",
            digamma => "digamma", trigamma => "trigamma", tetragamma => "tetragamma",
            zeta => "zeta", lambertw => "lambertw",
            elliptic_k => "elliptic_k", elliptic_e => "elliptic_e",
            exp_polar => "exp_polar"
        );
    };
}

macro_rules! math_function_list_owned {
    ($macro_name:ident, $type:ty, $converter:expr) => {
        $macro_name!($type, $converter,
            // Trigonometric functions
            sin => "sin", cos => "cos", tan => "tan",
            cot => "cot", sec => "sec", csc => "csc",
            // Inverse trigonometric functions
            asin => "asin", acos => "acos", atan => "atan",
            acot => "acot", asec => "asec", acsc => "acsc",
            // Hyperbolic functions
            sinh => "sinh", cosh => "cosh", tanh => "tanh",
            coth => "coth", sech => "sech", csch => "csch",
            // Inverse hyperbolic functions
            asinh => "asinh", acosh => "acosh", atanh => "atanh",
            acoth => "acoth", asech => "asech", acsch => "acsch",
            // Exponential and logarithmic functions
            exp => "exp", ln => "ln",
            log10 => "log10", log2 => "log2",
            // Root functions
            sqrt => "sqrt", cbrt => "cbrt",
            // Rounding functions
            floor => "floor", ceil => "ceil", round => "round",
            // Special functions (single-argument only)
            abs => "abs", signum => "signum", sinc => "sinc",
            erf => "erf", erfc => "erfc", gamma => "gamma",
            digamma => "digamma", trigamma => "trigamma", tetragamma => "tetragamma",
            zeta => "zeta", lambertw => "lambertw",
            elliptic_k => "elliptic_k", elliptic_e => "elliptic_e",
            exp_polar => "exp_polar"
        );
    };
}

// Apply to Symbol (takes &self, converts via to_expr())
math_function_list_ref!(impl_math_functions_ref, Symbol, |s: &Symbol| s.to_expr());

// Apply to Expr (consumes self)
math_function_list_owned!(impl_math_functions_owned, Expr, |e: Expr| e);

// Convert Symbol to Expr
impl From<Symbol> for Expr {
    fn from(s: Symbol) -> Self {
        s.to_expr()
    }
}

// Convert f64 to Expr
impl From<f64> for Expr {
    fn from(n: f64) -> Self {
        Self::number(n)
    }
}

// Convert i32 to Expr
impl From<i32> for Expr {
    fn from(n: i32) -> Self {
        Self::number(f64::from(n))
    }
}

// ============================================================================
// Operator Overloading
// ============================================================================

macro_rules! impl_binary_ops {
    (symbol_ops $lhs:ty, $rhs:ty, $to_lhs:expr, $to_rhs:expr) => {
        impl Add<$rhs> for $lhs {
            type Output = Expr;
            fn add(self, rhs: $rhs) -> Expr {
                Expr::add_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Sub<$rhs> for $lhs {
            type Output = Expr;
            fn sub(self, rhs: $rhs) -> Expr {
                Expr::sub_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Mul<$rhs> for $lhs {
            type Output = Expr;
            fn mul(self, rhs: $rhs) -> Expr {
                Expr::mul_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
        impl Div<$rhs> for $lhs {
            type Output = Expr;
            fn div(self, rhs: $rhs) -> Expr {
                Expr::div_expr($to_lhs(self), $to_rhs(rhs))
            }
        }
    };
}

// Symbol operations (value)
impl_binary_ops!(symbol_ops Symbol, Symbol, |s: Symbol| s.to_expr(), |r: Symbol| r.to_expr());
impl_binary_ops!(symbol_ops Symbol, Expr, |s: Symbol| s.to_expr(), |r: Expr| r);
impl_binary_ops!(symbol_ops Symbol, f64, |s: Symbol| s.to_expr(), |r: f64| Expr::number(r));

// &Symbol operations (reference)
impl_binary_ops!(
    symbol_ops & Symbol,
    &Symbol,
    |s: &Symbol| s.to_expr(),
    |r: &Symbol| r.to_expr()
);
impl_binary_ops!(
    symbol_ops & Symbol,
    Symbol,
    |s: &Symbol| s.to_expr(),
    |r: Symbol| r.to_expr()
);
impl_binary_ops!(
    symbol_ops & Symbol,
    Expr,
    |s: &Symbol| s.to_expr(),
    |r: Expr| r
);
impl_binary_ops!(
    symbol_ops & Symbol,
    f64,
    |s: &Symbol| s.to_expr(),
    |r: f64| Expr::number(r)
);

// Expr operations
impl_binary_ops!(symbol_ops Expr, Expr, |s: Expr| s, |r: Expr| r);
impl_binary_ops!(symbol_ops Expr, Symbol, |s: Expr| s, |r: Symbol| r.to_expr());
impl_binary_ops!(symbol_ops Expr, &Symbol, |s: Expr| s, |r: &Symbol| r.to_expr());
impl_binary_ops!(symbol_ops Expr, f64, |s: Expr| s, |r: f64| Expr::number(r));
impl_binary_ops!(symbol_ops Expr, &Expr, |s: Expr| s, |r: &Expr| r.clone());

// Symbol + &Symbol and Symbol + &Expr
impl_binary_ops!(symbol_ops Symbol, &Symbol, |s: Symbol| s.to_expr(), |r: &Symbol| r.to_expr());
impl_binary_ops!(symbol_ops Symbol, &Expr, |s: Symbol| s.to_expr(), |r: &Expr| r.clone());

// &Expr operations (reference - allows &expr + &expr without explicit .clone())
impl_binary_ops!(
    symbol_ops & Expr,
    &Expr,
    |e: &Expr| e.clone(),
    |r: &Expr| r.clone()
);
impl_binary_ops!(symbol_ops & Expr, Expr, |e: &Expr| e.clone(), |r: Expr| r);
impl_binary_ops!(
    symbol_ops & Expr,
    Symbol,
    |e: &Expr| e.clone(),
    |r: Symbol| r.to_expr()
);
impl_binary_ops!(
    symbol_ops & Expr,
    &Symbol,
    |e: &Expr| e.clone(),
    |r: &Symbol| r.to_expr()
);
impl_binary_ops!(symbol_ops & Expr, f64, |e: &Expr| e.clone(), |r: f64| {
    Expr::number(r)
});

// f64 on left side
impl Add<Expr> for f64 {
    type Output = Expr;
    fn add(self, rhs: Expr) -> Expr {
        Expr::add_expr(Expr::number(self), rhs)
    }
}

impl Mul<Expr> for f64 {
    type Output = Expr;
    fn mul(self, rhs: Expr) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs)
    }
}

impl Sub<Expr> for f64 {
    type Output = Expr;
    fn sub(self, rhs: Expr) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs)
    }
}

impl Div<Expr> for f64 {
    type Output = Expr;
    fn div(self, rhs: Expr) -> Expr {
        Expr::div_expr(Expr::number(self), rhs)
    }
}

impl Sub<Symbol> for f64 {
    type Output = Expr;
    fn sub(self, rhs: Symbol) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Mul<Symbol> for f64 {
    type Output = Expr;
    fn mul(self, rhs: Symbol) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Add<Symbol> for f64 {
    type Output = Expr;
    fn add(self, rhs: Symbol) -> Expr {
        Expr::add_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Div<Symbol> for f64 {
    type Output = Expr;
    fn div(self, rhs: Symbol) -> Expr {
        Expr::div_expr(Expr::number(self), rhs.to_expr())
    }
}

// f64 on left side with &Expr
impl Add<&Expr> for f64 {
    type Output = Expr;
    fn add(self, rhs: &Expr) -> Expr {
        Expr::add_expr(Expr::number(self), rhs.clone())
    }
}

impl Mul<&Expr> for f64 {
    type Output = Expr;
    fn mul(self, rhs: &Expr) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs.clone())
    }
}

impl Sub<&Expr> for f64 {
    type Output = Expr;
    fn sub(self, rhs: &Expr) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs.clone())
    }
}

impl Div<&Expr> for f64 {
    type Output = Expr;
    fn div(self, rhs: &Expr) -> Expr {
        Expr::div_expr(Expr::number(self), rhs.clone())
    }
}

// f64 on left side with &Symbol
impl Add<&Symbol> for f64 {
    type Output = Expr;
    fn add(self, rhs: &Symbol) -> Expr {
        Expr::add_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Sub<&Symbol> for f64 {
    type Output = Expr;
    fn sub(self, rhs: &Symbol) -> Expr {
        Expr::sub_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Mul<&Symbol> for f64 {
    type Output = Expr;
    fn mul(self, rhs: &Symbol) -> Expr {
        Expr::mul_expr(Expr::number(self), rhs.to_expr())
    }
}

impl Div<&Symbol> for f64 {
    type Output = Expr;
    fn div(self, rhs: &Symbol) -> Expr {
        Expr::div_expr(Expr::number(self), rhs.to_expr())
    }
}

// =============================================================================
// i32 operators (Symbol/Expr on left side)
// =============================================================================

// Symbol + i32
impl_binary_ops!(symbol_ops Symbol, i32, |s: Symbol| s.to_expr(), |r: i32| Expr::number(f64::from(r)));
impl_binary_ops!(
    symbol_ops & Symbol,
    i32,
    |s: &Symbol| s.to_expr(),
    |r: i32| Expr::number(f64::from(r))
);

// Expr + i32
impl_binary_ops!(symbol_ops Expr, i32, |s: Expr| s, |r: i32| Expr::number(f64::from(r)));
impl_binary_ops!(symbol_ops & Expr, i32, |e: &Expr| e.clone(), |r: i32| {
    Expr::number(f64::from(r))
});

// i32 on left side with Symbol
impl Add<Symbol> for i32 {
    type Output = Expr;
    fn add(self, rhs: Symbol) -> Expr {
        Expr::add_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

impl Sub<Symbol> for i32 {
    type Output = Expr;
    fn sub(self, rhs: Symbol) -> Expr {
        Expr::sub_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

impl Mul<Symbol> for i32 {
    type Output = Expr;
    fn mul(self, rhs: Symbol) -> Expr {
        Expr::mul_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

impl Div<Symbol> for i32 {
    type Output = Expr;
    fn div(self, rhs: Symbol) -> Expr {
        Expr::div_expr(Expr::number(f64::from(self)), rhs.to_expr())
    }
}

// i32 on left side with Expr
impl Add<Expr> for i32 {
    type Output = Expr;
    fn add(self, rhs: Expr) -> Expr {
        Expr::add_expr(Expr::number(f64::from(self)), rhs)
    }
}

impl Sub<Expr> for i32 {
    type Output = Expr;
    fn sub(self, rhs: Expr) -> Expr {
        Expr::sub_expr(Expr::number(f64::from(self)), rhs)
    }
}

impl Mul<Expr> for i32 {
    type Output = Expr;
    fn mul(self, rhs: Expr) -> Expr {
        Expr::mul_expr(Expr::number(f64::from(self)), rhs)
    }
}

impl Div<Expr> for i32 {
    type Output = Expr;
    fn div(self, rhs: Expr) -> Expr {
        Expr::div_expr(Expr::number(f64::from(self)), rhs)
    }
}

// i32 on left side with &Expr
impl Add<&Expr> for i32 {
    type Output = Expr;
    fn add(self, rhs: &Expr) -> Expr {
        Expr::add_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

impl Sub<&Expr> for i32 {
    type Output = Expr;
    fn sub(self, rhs: &Expr) -> Expr {
        Expr::sub_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

impl Mul<&Expr> for i32 {
    type Output = Expr;
    fn mul(self, rhs: &Expr) -> Expr {
        Expr::mul_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

impl Div<&Expr> for i32 {
    type Output = Expr;
    fn div(self, rhs: &Expr) -> Expr {
        Expr::div_expr(Expr::number(f64::from(self)), rhs.clone())
    }
}

// =============================================================================
// Arc<Expr> conversions (for CustomFn partials ergonomics)
// =============================================================================

impl From<Arc<Self>> for Expr {
    fn from(arc: Arc<Self>) -> Self {
        Arc::try_unwrap(arc).unwrap_or_else(|arc| (*arc).clone())
    }
}

impl From<&Arc<Self>> for Expr {
    fn from(arc: &Arc<Self>) -> Self {
        (**arc).clone()
    }
}

// =============================================================================
// Arc<Expr> ergonomic operations via extension trait
// =============================================================================
// Note: Due to Rust's orphan rules, we cannot:
// - Implement traits like `Add<Arc<Expr>> for f64` (both are foreign types)
// - Add inherent methods to `Arc<Expr>` (foreign type)
//
// Instead, we provide an extension trait `ArcExprExt` with methods like:
//   args[0].pow(2.0), args[0].sin(), args[0].cos(), etc.
//
// Recommended patterns in UserFunction body/partial closures:
//   .body(|args| args[0].pow(2.0) + 1.0)           // method chaining via trait
//   .body(|args| 2.0 * Expr::from(&args[0]))       // using From trait
//   .body(|args| args[0].sin() * args[1].cos())    // multiple args

/// Extension trait for `Arc<Expr>` providing ergonomic math operations.
///
/// This trait is automatically in scope when using the library, allowing
/// you to call methods like `.pow()`, `.sin()`, `.cos()` directly on `Arc<Expr>`.
///
/// # Example
/// ```
/// use symb_anafis::{UserFunction, Expr, ArcExprExt};
/// use std::sync::Arc;
///
/// // f(x) = x^2 + sin(x)
/// let f = UserFunction::new(1..=1)
///     .body(|args| args[0].pow(2.0) + args[0].sin());
/// ```
pub trait ArcExprExt {
    /// Raise to a power: `args[0].pow(2.0)` → `args[0]^2`
    fn pow(&self, exp: impl Into<Expr>) -> Expr;

    // Trigonometric
    /// Sine function: sin(x)
    fn sin(&self) -> Expr;
    /// Cosine function: cos(x)
    fn cos(&self) -> Expr;
    /// Tangent function: tan(x)
    fn tan(&self) -> Expr;
    /// Cotangent function: cot(x)
    fn cot(&self) -> Expr;
    /// Secant function: sec(x)
    fn sec(&self) -> Expr;
    /// Cosecant function: csc(x)
    fn csc(&self) -> Expr;

    // Inverse trigonometric
    /// Arcsine function: asin(x)
    fn asin(&self) -> Expr;
    /// Arccosine function: acos(x)
    fn acos(&self) -> Expr;
    /// Arctangent function: atan(x)
    fn atan(&self) -> Expr;
    /// Arccotangent function: acot(x)
    fn acot(&self) -> Expr;
    /// Arcsecant function: asec(x)
    fn asec(&self) -> Expr;
    /// Arccosecant function: acsc(x)
    fn acsc(&self) -> Expr;

    // Hyperbolic
    /// Hyperbolic sine: sinh(x)
    fn sinh(&self) -> Expr;
    /// Hyperbolic cosine: cosh(x)
    fn cosh(&self) -> Expr;
    /// Hyperbolic tangent: tanh(x)
    fn tanh(&self) -> Expr;
    /// Hyperbolic cotangent: coth(x)
    fn coth(&self) -> Expr;
    /// Hyperbolic secant: sech(x)
    fn sech(&self) -> Expr;
    /// Hyperbolic cosecant: csch(x)
    fn csch(&self) -> Expr;

    // Inverse hyperbolic
    /// Inverse hyperbolic sine: asinh(x)
    fn asinh(&self) -> Expr;
    /// Inverse hyperbolic cosine: acosh(x)
    fn acosh(&self) -> Expr;
    /// Inverse hyperbolic tangent: atanh(x)
    fn atanh(&self) -> Expr;
    /// Inverse hyperbolic cotangent: acoth(x)
    fn acoth(&self) -> Expr;
    /// Inverse hyperbolic secant: asech(x)
    fn asech(&self) -> Expr;
    /// Inverse hyperbolic cosecant: acsch(x)
    fn acsch(&self) -> Expr;

    // Exponential/logarithmic
    /// Exponential function: exp(x) = e^x
    fn exp(&self) -> Expr;
    /// Natural logarithm: ln(x)
    fn ln(&self) -> Expr;
    /// Logarithm with arbitrary base: `x.log(base)` → `log(base, x)`
    fn log(&self, base: impl Into<Expr>) -> Expr;
    /// Base-10 logarithm: log10(x)
    fn log10(&self) -> Expr;
    /// Base-2 logarithm: log2(x)
    fn log2(&self) -> Expr;
    /// Square root: sqrt(x)
    fn sqrt(&self) -> Expr;
    /// Cube root: cbrt(x)
    fn cbrt(&self) -> Expr;

    // Rounding
    /// Floor function: floor(x)
    fn floor(&self) -> Expr;
    /// Ceiling function: ceil(x)
    fn ceil(&self) -> Expr;
    /// Round to nearest integer: round(x)
    fn round(&self) -> Expr;

    // Special functions
    /// Absolute value: abs(x)
    fn abs(&self) -> Expr;
    /// Sign function: signum(x)
    fn signum(&self) -> Expr;
    /// Sinc function: sin(x)/x
    fn sinc(&self) -> Expr;
    /// Error function: erf(x)
    fn erf(&self) -> Expr;
    /// Complementary error function: erfc(x)
    fn erfc(&self) -> Expr;
    /// Gamma function: Γ(x)
    fn gamma(&self) -> Expr;
    /// Digamma function: ψ(x)
    fn digamma(&self) -> Expr;
    /// Trigamma function: ψ₁(x)
    fn trigamma(&self) -> Expr;
    /// Tetragamma function: ψ₂(x)
    fn tetragamma(&self) -> Expr;
    /// Riemann zeta function: ζ(x)
    fn zeta(&self) -> Expr;
    /// Lambert W function
    fn lambertw(&self) -> Expr;
    /// Complete elliptic integral of the first kind: K(x)
    fn elliptic_k(&self) -> Expr;
    /// Complete elliptic integral of the second kind: E(x)
    fn elliptic_e(&self) -> Expr;
    /// Exponential with polar representation
    fn exp_polar(&self) -> Expr;
}

impl ArcExprExt for Arc<Expr> {
    fn pow(&self, exp: impl Into<Expr>) -> Expr {
        Expr::pow_static(Expr::from(self), exp.into())
    }

    // Trigonometric
    fn sin(&self) -> Expr {
        Expr::func("sin", Expr::from(self))
    }
    fn cos(&self) -> Expr {
        Expr::func("cos", Expr::from(self))
    }
    fn tan(&self) -> Expr {
        Expr::func("tan", Expr::from(self))
    }
    fn cot(&self) -> Expr {
        Expr::func("cot", Expr::from(self))
    }
    fn sec(&self) -> Expr {
        Expr::func("sec", Expr::from(self))
    }
    fn csc(&self) -> Expr {
        Expr::func("csc", Expr::from(self))
    }

    // Inverse trigonometric
    fn asin(&self) -> Expr {
        Expr::func("asin", Expr::from(self))
    }
    fn acos(&self) -> Expr {
        Expr::func("acos", Expr::from(self))
    }
    fn atan(&self) -> Expr {
        Expr::func("atan", Expr::from(self))
    }
    fn acot(&self) -> Expr {
        Expr::func("acot", Expr::from(self))
    }
    fn asec(&self) -> Expr {
        Expr::func("asec", Expr::from(self))
    }
    fn acsc(&self) -> Expr {
        Expr::func("acsc", Expr::from(self))
    }

    // Hyperbolic
    fn sinh(&self) -> Expr {
        Expr::func("sinh", Expr::from(self))
    }
    fn cosh(&self) -> Expr {
        Expr::func("cosh", Expr::from(self))
    }
    fn tanh(&self) -> Expr {
        Expr::func("tanh", Expr::from(self))
    }
    fn coth(&self) -> Expr {
        Expr::func("coth", Expr::from(self))
    }
    fn sech(&self) -> Expr {
        Expr::func("sech", Expr::from(self))
    }
    fn csch(&self) -> Expr {
        Expr::func("csch", Expr::from(self))
    }

    // Inverse hyperbolic
    fn asinh(&self) -> Expr {
        Expr::func("asinh", Expr::from(self))
    }
    fn acosh(&self) -> Expr {
        Expr::func("acosh", Expr::from(self))
    }
    fn atanh(&self) -> Expr {
        Expr::func("atanh", Expr::from(self))
    }
    fn acoth(&self) -> Expr {
        Expr::func("acoth", Expr::from(self))
    }
    fn asech(&self) -> Expr {
        Expr::func("asech", Expr::from(self))
    }
    fn acsch(&self) -> Expr {
        Expr::func("acsch", Expr::from(self))
    }

    // Exponential/logarithmic
    fn exp(&self) -> Expr {
        Expr::func("exp", Expr::from(self))
    }
    fn ln(&self) -> Expr {
        Expr::func("ln", Expr::from(self))
    }
    fn log(&self, base: impl Into<Expr>) -> Expr {
        Expr::func_multi("log", vec![base.into(), Expr::from(self)])
    }
    fn log10(&self) -> Expr {
        Expr::func("log10", Expr::from(self))
    }
    fn log2(&self) -> Expr {
        Expr::func("log2", Expr::from(self))
    }
    fn sqrt(&self) -> Expr {
        Expr::func("sqrt", Expr::from(self))
    }
    fn cbrt(&self) -> Expr {
        Expr::func("cbrt", Expr::from(self))
    }

    // Rounding
    fn floor(&self) -> Expr {
        Expr::func("floor", Expr::from(self))
    }
    fn ceil(&self) -> Expr {
        Expr::func("ceil", Expr::from(self))
    }
    fn round(&self) -> Expr {
        Expr::func("round", Expr::from(self))
    }

    // Special functions
    fn abs(&self) -> Expr {
        Expr::func("abs", Expr::from(self))
    }
    fn signum(&self) -> Expr {
        Expr::func("signum", Expr::from(self))
    }
    fn sinc(&self) -> Expr {
        Expr::func("sinc", Expr::from(self))
    }
    fn erf(&self) -> Expr {
        Expr::func("erf", Expr::from(self))
    }
    fn erfc(&self) -> Expr {
        Expr::func("erfc", Expr::from(self))
    }
    fn gamma(&self) -> Expr {
        Expr::func("gamma", Expr::from(self))
    }
    fn digamma(&self) -> Expr {
        Expr::func("digamma", Expr::from(self))
    }
    fn trigamma(&self) -> Expr {
        Expr::func("trigamma", Expr::from(self))
    }
    fn tetragamma(&self) -> Expr {
        Expr::func("tetragamma", Expr::from(self))
    }
    fn zeta(&self) -> Expr {
        Expr::func("zeta", Expr::from(self))
    }
    fn lambertw(&self) -> Expr {
        Expr::func("lambertw", Expr::from(self))
    }
    fn elliptic_k(&self) -> Expr {
        Expr::func("elliptic_k", Expr::from(self))
    }
    fn elliptic_e(&self) -> Expr {
        Expr::func("elliptic_e", Expr::from(self))
    }
    fn exp_polar(&self) -> Expr {
        Expr::func("exp_polar", Expr::from(self))
    }
}

// --- Expr with Arc<Expr> (Expr on left side works due to local type) ---
impl Add<Arc<Self>> for Expr {
    type Output = Self;
    fn add(self, rhs: Arc<Self>) -> Self {
        Self::add_expr(self, Self::from(rhs))
    }
}

impl Add<&Arc<Self>> for Expr {
    type Output = Self;
    fn add(self, rhs: &Arc<Self>) -> Self {
        Self::add_expr(self, Self::from(rhs))
    }
}

impl Sub<Arc<Self>> for Expr {
    type Output = Self;
    fn sub(self, rhs: Arc<Self>) -> Self {
        Self::sub_expr(self, Self::from(rhs))
    }
}

impl Sub<&Arc<Self>> for Expr {
    type Output = Self;
    fn sub(self, rhs: &Arc<Self>) -> Self {
        Self::sub_expr(self, Self::from(rhs))
    }
}

impl Mul<Arc<Self>> for Expr {
    type Output = Self;
    fn mul(self, rhs: Arc<Self>) -> Self {
        Self::mul_expr(self, Self::from(rhs))
    }
}

impl Mul<&Arc<Self>> for Expr {
    type Output = Self;
    fn mul(self, rhs: &Arc<Self>) -> Self {
        Self::mul_expr(self, Self::from(rhs))
    }
}

impl Div<Arc<Self>> for Expr {
    type Output = Self;
    fn div(self, rhs: Arc<Self>) -> Self {
        Self::div_expr(self, Self::from(rhs))
    }
}

impl Div<&Arc<Self>> for Expr {
    type Output = Self;
    fn div(self, rhs: &Arc<Self>) -> Self {
        Self::div_expr(self, Self::from(rhs))
    }
}

impl Neg for Symbol {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::mul_expr(Expr::number(-1.0), self.to_expr())
    }
}

impl Neg for Expr {
    type Output = Self;
    fn neg(self) -> Self {
        Self::mul_expr(Self::number(-1.0), self)
    }
}

impl Neg for &Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::mul_expr(Expr::number(-1.0), self.clone())
    }
}

impl Neg for &Symbol {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::mul_expr(Expr::number(-1.0), self.to_expr())
    }
}

// ============================================================================
// Expr Methods
// ============================================================================

impl Expr {
    /// Raise to a power (method form, consumes self)
    ///
    /// This is the ergonomic method form for chaining. Accepts any type
    /// that can be converted to Expr (f64, i32, Symbol, Expr).
    ///
    /// # Example
    /// ```
    /// use symb_anafis::symb;
    /// let x = symb("pow_method_x");
    /// let expr = x.sin().pow(2.0);  // sin(x)^2
    /// assert_eq!(format!("{}", expr), "sin(pow_method_x)^2");
    /// ```
    #[inline]
    #[must_use]
    pub fn pow(self, exp: impl Into<Self>) -> Self {
        Self::pow_static(self, exp.into())
    }

    // === Parametric special functions ===

    /// Polygamma function: ψ^(n)(x)
    #[must_use]
    pub fn polygamma(self, n: impl Into<Self>) -> Self {
        Self::func_multi("polygamma", vec![n.into(), self])
    }

    /// Beta function: B(a, b)
    #[must_use]
    pub fn beta(self, other: impl Into<Self>) -> Self {
        Self::func_multi("beta", vec![self, other.into()])
    }

    /// Bessel function of the first kind: `J_n(x)`
    #[must_use]
    pub fn besselj(self, n: impl Into<Self>) -> Self {
        Self::func_multi("besselj", vec![n.into(), self])
    }

    /// Bessel function of the second kind: `Y_n(x)`
    #[must_use]
    pub fn bessely(self, n: impl Into<Self>) -> Self {
        Self::func_multi("bessely", vec![n.into(), self])
    }

    /// Modified Bessel function of the first kind: `I_n(x)`
    #[must_use]
    pub fn besseli(self, n: impl Into<Self>) -> Self {
        Self::func_multi("besseli", vec![n.into(), self])
    }

    /// Modified Bessel function of the second kind: `K_n(x)`
    #[must_use]
    pub fn besselk(self, n: impl Into<Self>) -> Self {
        Self::func_multi("besselk", vec![n.into(), self])
    }

    /// Derivative of Riemann zeta function: ζ^(n)(self)
    #[must_use]
    pub fn zeta_deriv(self, n: impl Into<Self>) -> Self {
        Self::func_multi("zeta_deriv", vec![n.into(), self])
    }

    /// Logarithm with arbitrary base: `x.log(base)` → `log(base, x)`
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{symb, Expr};
    /// let x = symb("log_expr_example");
    /// let expr = x.sin() + Expr::number(1.0);
    /// let log_base_10 = expr.log(10.0);  // log(10, sin(x) + 1)
    /// assert!(format!("{}", log_base_10).contains("log"));
    /// ```
    #[must_use]
    pub fn log(self, base: impl Into<Self>) -> Self {
        Self::func_multi("log", vec![base.into(), self])
    }

    /// Hermite polynomial `H_n(self)`
    #[must_use]
    pub fn hermite(self, n: impl Into<Self>) -> Self {
        Self::func_multi("hermite", vec![n.into(), self])
    }

    /// Associated Legendre polynomial `P_l^m(self)`
    #[must_use]
    pub fn assoc_legendre(self, l: impl Into<Self>, m: impl Into<Self>) -> Self {
        Self::func_multi("assoc_legendre", vec![l.into(), m.into(), self])
    }

    /// Spherical harmonic `Y_l^m(theta`, phi) where self is theta
    #[must_use]
    pub fn spherical_harmonic(
        self,
        l: impl Into<Self>,
        m: impl Into<Self>,
        phi: impl Into<Self>,
    ) -> Self {
        Self::func_multi(
            "spherical_harmonic",
            vec![l.into(), m.into(), self, phi.into()],
        )
    }

    /// Alternative spherical harmonic notation `Y_l^m(theta`, phi)
    #[must_use]
    pub fn ynm(self, l: impl Into<Self>, m: impl Into<Self>, phi: impl Into<Self>) -> Self {
        Self::func_multi("ynm", vec![l.into(), m.into(), self, phi.into()])
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests use unique symbol names to avoid interference from parallel execution.
    // The global registry is shared, so each test uses a unique prefix.

    #[test]
    fn test_symb_new_creates_new_symbol() {
        let x = symb_new("test_create_x").unwrap();
        assert_eq!(x.name(), Some("test_create_x".to_string()));
    }

    #[test]
    fn test_symb_new_errors_on_duplicate() {
        let name = "test_dup_x";
        // First try to create, if exists that's fine (from previous run)
        let _ = symb_new(name);
        // Second attempt should fail
        let result = symb_new(name);
        assert!(matches!(result, Err(SymbolError::DuplicateName(_))));
    }

    #[test]
    fn test_symb_get_returns_existing() {
        let name = "test_get_existing";
        let x = symb(name); // Use sym to ensure it exists
        let x2 = symb_get(name).unwrap();
        assert_eq!(x.id(), x2.id());
    }

    #[test]
    fn test_symb_get_errors_if_not_found() {
        let result = symb_get("test_nonexistent_unique_name_12345");
        assert!(matches!(result, Err(SymbolError::NotFound(_))));
    }

    #[test]
    fn test_symbol_exists() {
        let name = "test_exists_x";
        let _ = symb(name); // Ensure it exists
        assert!(symbol_exists(name));
    }

    #[test]
    fn test_anonymous_symbol() {
        let a1 = Symbol::anon();
        let a2 = Symbol::anon();
        assert_ne!(a1.id(), a2.id()); // Different IDs
        assert_eq!(a1.name(), None);
        assert_eq!(a2.name(), None);
    }

    #[test]
    fn test_sym_legacy_compatibility() {
        let x = symb("test_legacy_x");
        let x2 = symb("test_legacy_x"); // Should NOT error
        assert_eq!(x.id(), x2.id());
    }

    #[test]
    fn test_symbol_equality() {
        let name = "test_equality_x";
        let x1 = symb(name); // Use sym to ensure it exists  
        let x2 = symb_get(name).unwrap();
        assert_eq!(x1, x2); // Same ID, equal
    }

    #[test]
    fn test_symbol_arithmetic() {
        let x = symb("test_arith_x2");
        let y = symb("test_arith_y2");

        // No clone() needed - Symbol is Copy!
        let sum = x + y;
        assert_eq!(format!("{}", sum), "test_arith_x2 + test_arith_y2");

        let scaled = 2.0 * x;
        assert_eq!(format!("{}", scaled), "2*test_arith_x2");
    }

    #[test]
    fn test_symbol_power() {
        let x = symb("test_pow_x2");
        let squared = x.pow(2.0);
        assert_eq!(format!("{}", squared), "test_pow_x2^2");
    }

    #[test]
    fn test_symbol_functions() {
        let x = symb("test_fn_x2");
        // No clone() needed - methods take &self!
        assert_eq!(format!("{}", x.sin()), "sin(test_fn_x2)");
        assert_eq!(format!("{}", x.cos()), "cos(test_fn_x2)");
        assert_eq!(format!("{}", x.exp()), "exp(test_fn_x2)");
        assert_eq!(format!("{}", x.ln()), "ln(test_fn_x2)");

        // Can now use x multiple times without clone!
        let res = x.cos() + x.sin();
        // Sorts alphabetically: cos before sin
        assert_eq!(format!("{}", res), "cos(test_fn_x2) + sin(test_fn_x2)");
    }

    #[test]
    fn test_symbol_copy_operators() {
        // This is the key test: Symbol is Copy, so a + a works!
        let a = symb("test_copy_a");

        // a + a - uses the same symbol twice without .clone()
        let expr = a + a;
        assert!(format!("{}", expr).contains("test_copy_a"));

        // Multiple uses in complex expression
        let expr2 = a * a + a;
        assert!(format!("{}", expr2).contains("test_copy_a"));

        // Mixed with functions (which take &self)
        let expr3 = a.sin() + a.cos() + a;
        assert!(format!("{}", expr3).contains("test_copy_a"));
    }

    // =========================================================================
    // Comprehensive Expression Building Tests
    // =========================================================================

    #[test]
    fn test_symbol_with_f64() {
        let x = symb("test_f64_x");

        // Symbol + f64
        let e1 = x + 1.5;
        assert!(format!("{}", e1).contains("test_f64_x"));

        // f64 + Symbol
        let e2 = 2.5 + x;
        assert!(format!("{}", e2).contains("test_f64_x"));

        // All operations
        let _ = x - 1.0;
        let _ = x * 2.0;
        let _ = x / 3.0;
        let _ = 1.0 - x;
        let _ = 2.0 * x;
        let _ = 3.0 / x;
    }

    #[test]
    fn test_symbol_with_i32() {
        let x = symb("test_i32_x");

        // Symbol + i32
        let e1 = x + 1;
        assert!(format!("{}", e1).contains("test_i32_x"));

        // i32 + Symbol
        let e2 = 2 + x;
        assert!(format!("{}", e2).contains("test_i32_x"));

        // All operations
        let _ = x - 1;
        let _ = x * 2;
        let _ = x / 3;
        let _ = 1 - x;
        let _ = 2 * x;
        let _ = 3 / x;
    }

    #[test]
    fn test_symbol_ref_with_f64() {
        let x = symb("test_ref_f64_x");

        // &Symbol + f64
        let e1 = &x + 1.5;
        assert!(format!("{}", e1).contains("test_ref_f64_x"));

        // All operations
        let _ = &x - 1.0;
        let _ = &x * 2.0;
        let _ = &x / 3.0;
    }

    #[test]
    fn test_symbol_ref_with_i32() {
        let x = symb("test_ref_i32_x");

        // &Symbol + i32
        let e1 = &x + 1;
        assert!(format!("{}", e1).contains("test_ref_i32_x"));

        // All operations
        let _ = &x - 1;
        let _ = &x * 2;
        let _ = &x / 3;
    }

    #[test]
    fn test_expr_with_f64() {
        let x = symb("test_expr_f64_x");
        let expr = x.sin();

        // Expr + f64
        let e1 = expr.clone() + 1.5;
        assert!(format!("{}", e1).contains("sin"));

        // f64 + Expr
        let e2 = 2.5 + expr.clone();
        assert!(format!("{}", e2).contains("sin"));

        // All operations
        let _ = expr.clone() - 1.0;
        let _ = expr.clone() * 2.0;
        let _ = expr.clone() / 3.0;
        let _ = 1.0 - expr.clone();
        let _ = 2.0 * expr.clone();
        let _ = 3.0 / expr;
    }

    #[test]
    fn test_expr_with_i32() {
        let x = symb("test_expr_i32_x");
        let expr = x.cos();

        // Expr + i32
        let e1 = expr.clone() + 1;
        assert!(format!("{}", e1).contains("cos"));

        // i32 + Expr
        let e2 = 2 + expr.clone();
        assert!(format!("{}", e2).contains("cos"));

        // All operations
        let _ = expr.clone() - 1;
        let _ = expr.clone() * 2;
        let _ = expr.clone() / 3;
        let _ = 1 - expr.clone();
        let _ = 2 * expr.clone();
        let _ = 3 / expr;
    }

    #[test]
    fn test_expr_ref_with_f64() {
        let x = symb("test_eref_f64_x");
        let expr = x.exp();

        // &Expr + f64
        let e1 = &expr + 1.5;
        assert!(format!("{}", e1).contains("exp"));

        // f64 + &Expr
        let e2 = 2.5 + &expr;
        assert!(format!("{}", e2).contains("exp"));

        // All operations
        let _ = &expr - 1.0;
        let _ = &expr * 2.0;
        let _ = &expr / 3.0;
        let _ = 1.0 - &expr;
        let _ = 2.0 * &expr;
        let _ = 3.0 / &expr;
    }

    #[test]
    fn test_expr_ref_with_i32() {
        let x = symb("test_eref_i32_x");
        let expr = x.ln();

        // &Expr + i32
        let e1 = &expr + 1;
        assert!(format!("{}", e1).contains("ln"));

        // i32 + &Expr
        let e2 = 2 + &expr;
        assert!(format!("{}", e2).contains("ln"));

        // All operations
        let _ = &expr - 1;
        let _ = &expr * 2;
        let _ = &expr / 3;
        let _ = 1 - &expr;
        let _ = 2 * &expr;
        let _ = 3 / &expr;
    }

    #[test]
    fn test_symbol_with_symbol() {
        let x = symb("test_ss_x");
        let y = symb("test_ss_y");

        // Symbol + Symbol
        let e1 = x + y;
        assert!(format!("{}", e1).contains("test_ss_x"));
        assert!(format!("{}", e1).contains("test_ss_y"));

        // &Symbol + Symbol
        let e2 = x + y;
        assert!(format!("{}", e2).contains("test_ss_x"));

        // Symbol + &Symbol
        let e3 = x + y;
        assert!(format!("{}", e3).contains("test_ss_y"));

        // &Symbol + &Symbol
        let e4 = x + y;
        assert!(format!("{}", e4).contains("test_ss_x"));
        assert!(format!("{}", e4).contains("test_ss_y"));
    }

    #[test]
    fn test_symbol_with_expr() {
        let x = symb("test_se_x");
        let y = symb("test_se_y");
        let expr = y.sin();

        // Symbol + Expr
        let e1 = x + expr.clone();
        assert!(format!("{}", e1).contains("test_se_x"));
        assert!(format!("{}", e1).contains("sin"));

        // Expr + Symbol
        let e2 = expr.clone() + x;
        assert!(format!("{}", e2).contains("test_se_x"));

        // &Symbol + Expr
        let e3 = x + expr.clone();
        assert!(format!("{}", e3).contains("test_se_x"));

        // Symbol + &Expr
        let e4 = x + &expr;
        assert!(format!("{}", e4).contains("sin"));
    }

    #[test]
    fn test_expr_with_expr() {
        let x = symb("test_ee_x");
        let y = symb("test_ee_y");
        let expr1 = x.sin();
        let expr2 = y.cos();

        // Expr + Expr
        let e1 = expr1.clone() + expr2.clone();
        assert!(format!("{}", e1).contains("sin"));
        assert!(format!("{}", e1).contains("cos"));

        // &Expr + &Expr
        let e2 = &expr1 + &expr2;
        assert!(format!("{}", e2).contains("sin"));

        // &Expr + Expr
        let e3 = &expr1 + expr2.clone();
        assert!(format!("{}", e3).contains("sin"));

        // Expr + &Expr
        let e4 = expr1 + &expr2;
        assert!(format!("{}", e4).contains("cos"));
    }

    #[test]
    fn test_mixed_complex_expression() {
        let x = symb("test_mix_x");
        let y = symb("test_mix_y");

        // Complex expression mixing all types
        // (x + 1) * (y - 2.0) + 3 * x.sin()
        let expr = (x + 1) * (y - 2.0) + 3 * x.sin();
        let s = format!("{}", expr);
        assert!(s.contains("test_mix_x"));
        assert!(s.contains("test_mix_y"));
        assert!(s.contains("sin"));
    }

    #[test]
    fn test_negation() {
        let x = symb("test_neg_x");
        let expr = x.sin();

        // -Symbol
        let e1 = -x;
        assert!(format!("{}", e1).contains("test_neg_x"));

        // -&Symbol
        let e2 = -&x;
        assert!(format!("{}", e2).contains("test_neg_x"));

        // -Expr
        let e3 = -expr.clone();
        assert!(format!("{}", e3).contains("sin"));

        // -&Expr
        let e4 = -&expr;
        assert!(format!("{}", e4).contains("sin"));
    }

    #[test]
    fn test_power_with_all_types() {
        let x = symb("test_pow_all_x");

        // Symbol.pow(f64)
        let e1 = x.pow(2.0);
        assert!(format!("{}", e1).contains("^2"));

        // Symbol.pow(i32)
        let e2 = x.pow(3);
        assert!(format!("{}", e2).contains("^3"));

        // Symbol.pow(Expr)
        let y = symb("test_pow_all_y");
        let e3 = x.pow(y);
        assert!(format!("{}", e3).contains("test_pow_all_y"));

        // Expr.pow(f64)
        let expr = x.sin();
        let e4 = expr.pow(2.0);
        assert!(format!("{}", e4).contains("sin"));
    }
}
