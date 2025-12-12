//! Type-safe Symbol and operator overloading for ergonomic expression building
//!
//! # Symbol Interning
//!
//! Symbols are interned globally for O(1) equality comparisons. Each unique symbol name
//! exists exactly once in memory, and all references share the same ID.
//!
//! # Example
//! ```ignore
//! use symb_anafis::{sym, symb_new, symb_get, clear_symbols};
//!
//! // Create a new symbol (errors if name already registered)
//! let x = symb_new("x").unwrap();
//!
//! // Get existing symbol (errors if not found)
//! let x2 = symb_get("x").unwrap();
//! assert_eq!(x.id(), x2.id());  // Same symbol!
//!
//! // Anonymous symbol (always succeeds)
//! let temp = Symbol::anon();
//!
//! // Clear registry for fresh start
//! clear_symbols();
//! ```

use crate::Expr;
use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

// ============================================================================
// Global Symbol Registry
// ============================================================================

/// Global counter for symbol IDs
static NEXT_SYMBOL_ID: AtomicU64 = AtomicU64::new(1);

/// Global symbol registry: name -> InternedSymbol
static SYMBOL_REGISTRY: RwLock<Option<HashMap<String, InternedSymbol>>> = RwLock::new(None);

/// Reverse lookup: ID -> InternedSymbol (for Copy Symbol to find its data)
static ID_REGISTRY: RwLock<Option<HashMap<u64, InternedSymbol>>> = RwLock::new(None);

fn get_registry_mut()
-> std::sync::RwLockWriteGuard<'static, Option<HashMap<String, InternedSymbol>>> {
    SYMBOL_REGISTRY.write().unwrap()
}

fn get_id_registry_mut()
-> std::sync::RwLockWriteGuard<'static, Option<HashMap<u64, InternedSymbol>>> {
    ID_REGISTRY.write().unwrap()
}

fn get_id_registry_read()
-> std::sync::RwLockReadGuard<'static, Option<HashMap<u64, InternedSymbol>>> {
    ID_REGISTRY.read().unwrap()
}

fn get_registry_read()
-> std::sync::RwLockReadGuard<'static, Option<HashMap<String, InternedSymbol>>> {
    SYMBOL_REGISTRY.read().unwrap()
}

fn with_registry_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, InternedSymbol>, &mut HashMap<u64, InternedSymbol>) -> R,
{
    let mut guard = get_registry_mut();
    let mut id_guard = get_id_registry_mut();
    let registry = guard.get_or_insert_with(HashMap::new);
    let id_registry = id_guard.get_or_insert_with(HashMap::new);
    f(registry, id_registry)
}

/// Look up InternedSymbol by ID (for Symbol -> Expr conversion)
fn lookup_by_id(id: u64) -> Option<InternedSymbol> {
    let guard = get_id_registry_read();
    guard.as_ref().and_then(|r| r.get(&id).cloned())
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
            SymbolError::DuplicateName(name) => {
                write!(
                    f,
                    "Symbol '{}' is already registered. Use symb_get() to retrieve it.",
                    name
                )
            }
            SymbolError::NotFound(name) => {
                write!(
                    f,
                    "Symbol '{}' not found. Use symb() to create it first.",
                    name
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
        InternedSymbol {
            id: NEXT_SYMBOL_ID.fetch_add(1, Ordering::Relaxed),
            name: Some(Arc::from(name)),
        }
    }

    /// Create a new anonymous interned symbol
    fn new_anon() -> Self {
        InternedSymbol {
            id: NEXT_SYMBOL_ID.fetch_add(1, Ordering::Relaxed),
            name: None,
        }
    }

    /// Get the symbol's unique ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the symbol's name (None for anonymous symbols)
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the symbol's name or a generated name for anonymous symbols
    pub fn display_name(&self) -> String {
        match &self.name {
            Some(n) => n.to_string(),
            None => format!("${}", self.id),
        }
    }
}

// O(1) equality comparison using ID
impl PartialEq for InternedSymbol {
    fn eq(&self, other: &Self) -> bool {
        if self.id == other.id {
            return true;
        }
        // Fallback to name comparison for symbols created from different paths
        // (e.g., parser-created vs API-created before interning was unified)
        match (&self.name, &other.name) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for InternedSymbol {}

// Hash by ID for O(1) HashMap operations
impl std::hash::Hash for InternedSymbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // For consistency with PartialEq, we hash by name if present
        // This ensures that symbols with same name have same hash
        match &self.name {
            Some(n) => n.hash(state),
            None => self.id.hash(state),
        }
    }
}

// Allow comparison with &str for ergonomic pattern matching
impl PartialEq<str> for InternedSymbol {
    fn eq(&self, other: &str) -> bool {
        self.name.as_deref() == Some(other)
    }
}

impl PartialEq<&str> for InternedSymbol {
    fn eq(&self, other: &&str) -> bool {
        self.name.as_deref() == Some(*other)
    }
}

impl PartialEq<InternedSymbol> for str {
    fn eq(&self, other: &InternedSymbol) -> bool {
        other.name.as_deref() == Some(self)
    }
}

impl PartialEq<InternedSymbol> for &str {
    fn eq(&self, other: &InternedSymbol) -> bool {
        other.name.as_deref() == Some(*self)
    }
}

// Allow display for debugging and error messages
impl std::fmt::Display for InternedSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(n) => write!(f, "{}", n),
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
/// ```ignore
/// let a = symb("a");
/// let expr = a + a;  // Works! No clone needed.
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Symbol(u64); // Just the ID - lightweight and Copy!

impl Symbol {
    /// Create a new anonymous symbol (always succeeds)
    ///
    /// Anonymous symbols have a unique ID but no string name.
    /// They cannot be retrieved by name and are useful for intermediate computations.
    pub fn anon() -> Self {
        let interned = InternedSymbol::new_anon();
        let id = interned.id();
        // Register in ID registry for lookup
        let mut id_guard = get_id_registry_mut();
        let id_registry = id_guard.get_or_insert_with(HashMap::new);
        id_registry.insert(id, interned);
        Symbol(id)
    }

    /// Get the symbol's unique ID
    pub fn id(&self) -> u64 {
        self.0
    }

    /// Get the name of the symbol (None for anonymous symbols)
    pub fn name(&self) -> Option<&'static str> {
        // Look up in registry - we leak the string to get 'static lifetime
        // This is safe because symbol names live for the program duration
        lookup_by_id(self.0).and_then(|s| {
            s.name.as_ref().map(|arc| {
                // Leak to get 'static - symbols are never deallocated in practice
                let leaked: &'static str = Box::leak(arc.to_string().into_boxed_str());
                leaked
            })
        })
    }

    /// Get the name as an owned String (avoids the leak in name())
    pub fn name_owned(&self) -> Option<String> {
        lookup_by_id(self.0).and_then(|s| s.name.as_ref().map(|arc| arc.to_string()))
    }

    /// Convert to an Expr
    pub fn to_expr(&self) -> Expr {
        // Look up the InternedSymbol from registry
        if let Some(interned) = lookup_by_id(self.0) {
            Expr::from_interned(interned)
        } else {
            // Fallback: create anonymous symbol expression
            // This shouldn't happen in normal use
            Expr::from_interned(InternedSymbol::new_anon())
        }
    }

    /// Raise to a power (Copy means no clone needed)
    pub fn pow(&self, exp: impl Into<Expr>) -> Expr {
        Expr::pow(self.to_expr(), exp.into())
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

    /// Bessel function of the first kind: J_n(x)
    pub fn besselj(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("besselj", vec![n.into(), self.to_expr()])
    }

    /// Bessel function of the second kind: Y_n(x)
    pub fn bessely(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("bessely", vec![n.into(), self.to_expr()])
    }

    /// Modified Bessel function of the first kind: I_n(x)
    pub fn besseli(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("besseli", vec![n.into(), self.to_expr()])
    }

    /// Modified Bessel function of the second kind: K_n(x)
    pub fn besselk(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("besselk", vec![n.into(), self.to_expr()])
    }
}

// Allow Symbol to be used where &str is expected
// Note: This uses the name() method which handles 'static lifetime via leak
impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        self.name().unwrap_or("")
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
/// ```ignore
/// let x = symb_new("x")?;
/// let y = symb_new("y")?;
/// let expr = x.pow(2) + y;  // No clone needed!
/// ```
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
/// ```ignore
/// let x = symb_new("x")?;
/// // ... later ...
/// let x2 = symb_get("x")?;
/// assert_eq!(x.id(), x2.id());  // Same symbol!
/// ```
pub fn symb_get(name: &str) -> Result<Symbol, SymbolError> {
    let guard = get_registry_read();
    match guard.as_ref().and_then(|r| r.get(name)) {
        Some(interned) => Ok(Symbol(interned.id())),
        None => Err(SymbolError::NotFound(name.to_string())),
    }
}

/// Check if a symbol name is already registered
pub fn symbol_exists(name: &str) -> bool {
    let guard = get_registry_read();
    guard.as_ref().is_some_and(|r| r.contains_key(name))
}

/// Clear all registered symbols from the global registry
///
/// This resets the symbol table. Use with caution - any existing Symbol
/// instances will still work but won't match newly created ones by name lookup.
pub fn clear_symbols() {
    let mut guard = get_registry_mut();
    let mut id_guard = get_id_registry_mut();
    if let Some(registry) = guard.as_mut() {
        registry.clear();
    }
    if let Some(id_registry) = id_guard.as_mut() {
        id_registry.clear();
    }
}

/// Remove a specific symbol from the registry
///
/// Returns true if the symbol was removed, false if it didn't exist.
/// Use with caution - any existing Symbol instances with this name will
/// still work but won't match newly created ones.
///
/// # Example
/// ```ignore
/// let x = symb("x");
/// assert!(symbol_exists("x"));
/// remove_symbol("x");
/// assert!(!symbol_exists("x"));
/// ```
pub fn remove_symbol(name: &str) -> bool {
    let mut guard = get_registry_mut();
    if let Some(registry) = guard.as_mut() {
        registry.remove(name).is_some()
    } else {
        false
    }
}

/// Get or create an interned symbol (internal use for parser)
///
/// Unlike `symb()`, this does NOT error on duplicates - it returns the existing symbol.
/// This is used by the parser to ensure parsed expressions use the same symbols.
pub(crate) fn get_or_intern(name: &str) -> InternedSymbol {
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

// Legacy compatibility - use sym as alias that doesn't require error handling
// by using get_or_intern internally

/// Convenience function to create or get a Symbol (legacy compatibility)
///
/// This is a convenience wrapper that uses get-or-create semantics.
/// For strict control, use `symb()` and `symb_get()` instead.
///
/// Note: Unlike `symb()`, this does NOT error on duplicate names.
pub fn symb(name: &str) -> Symbol {
    let interned = get_or_intern(name);
    Symbol(interned.id())
}

// ============================================================================
// Macro for generating math function methods
// ============================================================================

/// Generate math function methods for a type (taking &self for Symbol, self for Expr)
macro_rules! impl_math_functions_ref {
    ($type:ty, $converter:expr, $($fn_name:ident => $func_str:literal),* $(,)?) => {
        impl $type {
            $(
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
                pub fn $fn_name(self) -> Expr {
                    Expr::func($func_str, $converter(self))
                }
            )*
        }
    };
}

// Define the function list once
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
            exp => "exp", ln => "ln", log => "log",
            log10 => "log10", log2 => "log2",
            // Root functions
            sqrt => "sqrt", cbrt => "cbrt",
            // Special functions (single-argument only)
            abs => "abs", sign => "sign", sinc => "sinc",
            erf => "erf", erfc => "erfc", gamma => "gamma",
            digamma => "digamma", trigamma => "trigamma",
            zeta => "zeta", lambertw => "lambertw",
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
            exp => "exp", ln => "ln", log => "log",
            log10 => "log10", log2 => "log2",
            // Root functions
            sqrt => "sqrt", cbrt => "cbrt",
            // Special functions (single-argument only)
            abs => "abs", sign => "sign", sinc => "sinc",
            erf => "erf", erfc => "erfc", gamma => "gamma",
            digamma => "digamma", trigamma => "trigamma",
            zeta => "zeta", lambertw => "lambertw",
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
        Expr::number(n)
    }
}

// Convert i32 to Expr
impl From<i32> for Expr {
    fn from(n: i32) -> Self {
        Expr::number(n as f64)
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

// Negation
impl Neg for Symbol {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::mul_expr(Expr::number(-1.0), self.to_expr())
    }
}

impl Neg for Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::mul_expr(Expr::number(-1.0), self)
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
    /// Raise to a power
    #[inline]
    pub fn pow_expr(self, exp: impl Into<Expr>) -> Expr {
        Expr::pow(self, exp.into())
    }

    /// Alias for pow_expr
    #[inline]
    pub fn pow_of(self, exp: impl Into<Expr>) -> Expr {
        self.pow_expr(exp)
    }

    // === Parametric special functions ===

    /// Polygamma function: ψ^(n)(x)
    pub fn polygamma(self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("polygamma", vec![n.into(), self])
    }

    /// Beta function: B(a, b)
    pub fn beta(self, other: impl Into<Expr>) -> Expr {
        Expr::func_multi("beta", vec![self, other.into()])
    }

    /// Bessel function of the first kind: J_n(x)
    pub fn besselj(self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("besselj", vec![n.into(), self])
    }

    /// Bessel function of the second kind: Y_n(x)
    pub fn bessely(self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("bessely", vec![n.into(), self])
    }

    /// Modified Bessel function of the first kind: I_n(x)
    pub fn besseli(self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("besseli", vec![n.into(), self])
    }

    /// Modified Bessel function of the second kind: K_n(x)
    pub fn besselk(self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi("besselk", vec![n.into(), self])
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
        assert_eq!(x.name(), Some("test_create_x"));
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
        assert_eq!(format!("{}", scaled), "2test_arith_x2");
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
        let combined = x.sin() + x.cos();
        assert_eq!(format!("{}", combined), "sin(test_fn_x2) + cos(test_fn_x2)");
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
}
