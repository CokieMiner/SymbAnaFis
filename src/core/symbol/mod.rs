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

// Submodules
mod interned;
mod math_methods;
mod operators;
pub(crate) mod registry;

// Re-exports
pub use interned::InternedSymbol;
pub use registry::{
    clear_symbols, lookup_by_id, remove_symbol, symb, symb_get,
    symb_interned, symb_new, symbol_count, symbol_exists, symbol_names,
};

use std::sync::Arc;

use slotmap::{DefaultKey, Key};

use crate::core::known_symbols as ks;
use crate::Expr;

// Re-export ArcExprExt from operators
pub use operators::ArcExprExt;

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
pub struct Symbol(pub(crate) DefaultKey); // The key from the SlotMap - lightweight and Copy!

impl Symbol {
    /// Create a new anonymous symbol (always succeeds)
    ///
    /// Anonymous symbols have a unique ID but no string name.
    /// They cannot be retrieved by name and are useful for intermediate computations.
    #[must_use]
    pub fn anon() -> Self {
        registry::symb_anon()
    }

    /// Create a Symbol from its unique ID.
    ///
    /// This is used when a symbol needs to be reconstructed from a stored ID.
    #[inline]
    #[must_use]
    pub fn from_id(id: u64) -> Self {
        Self(registry::key_from_id(id))
    }

    /// Get the symbol's unique ID as a u64 for display or FFI.
    #[inline]
    #[must_use]
    pub fn id(&self) -> u64 {
        self.0.data().as_ffi()
    }

    /// Get the symbol's internal key.
    #[inline]
    #[must_use]
    pub const fn key(&self) -> DefaultKey {
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
        lookup_by_id(self.id()).and_then(|s| s.name_arc())
    }

    /// Convert to an Expr
    #[must_use]
    pub fn to_expr(&self) -> Expr {
        // Look up the InternedSymbol from registry
        lookup_by_id(self.id()).map_or_else(
            || Expr::from_interned(InternedSymbol::new_anon_with_key(self.0)),
            Expr::from_interned,
        )
    }

    /// Raise to a power (Copy means no clone needed)
    pub fn pow(&self, exp: impl Into<Expr>) -> Expr {
        Expr::pow_static(self.to_expr(), exp.into())
    }

    // === Parametric special functions ===

    /// Polygamma function: ψ^(n)(x)
    /// `x.polygamma(n)` → `polygamma(n, x)`
    pub fn polygamma(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.polygamma),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Beta function: B(a, b)
    pub fn beta(&self, other: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.beta),
            vec![self.to_expr(), other.into()],
        )
    }

    /// Bessel function of the first kind: `J_n(x)`
    pub fn besselj(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.besselj),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Bessel function of the second kind: `Y_n(x)`
    pub fn bessely(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.bessely),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Modified Bessel function of the first kind: `I_n(x)`
    pub fn besseli(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.besseli),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Modified Bessel function of the second kind: `K_n(x)`
    pub fn besselk(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.besselk),
            vec![n.into(), self.to_expr()],
        )
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
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.log),
            vec![base.into(), self.to_expr()],
        )
    }

    /// Two-argument arctangent: atan2(self, x) = angle to point (x, self)
    pub fn atan2(&self, x: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(ks::get_symbol(ks::KS.atan2), vec![self.to_expr(), x.into()])
    }

    /// Hermite polynomial `H_n(self)`
    pub fn hermite(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.hermite),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Associated Legendre polynomial `P_l^m(self)`
    pub fn assoc_legendre(&self, l: impl Into<Expr>, m: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.assoc_legendre),
            vec![l.into(), m.into(), self.to_expr()],
        )
    }

    /// Spherical harmonic `Y_l^m(theta`, phi) where self is theta
    pub fn spherical_harmonic(
        &self,
        l: impl Into<Expr>,
        m: impl Into<Expr>,
        phi: impl Into<Expr>,
    ) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.spherical_harmonic),
            vec![l.into(), m.into(), self.to_expr(), phi.into()],
        )
    }

    /// Alternative spherical harmonic notation `Y_l^m(theta`, phi)
    pub fn ynm(&self, l: impl Into<Expr>, m: impl Into<Expr>, phi: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.ynm),
            vec![l.into(), m.into(), self.to_expr(), phi.into()],
        )
    }

    /// Derivative of Riemann zeta function: zeta^(n)(self)
    pub fn zeta_deriv(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.zeta_deriv),
            vec![n.into(), self.to_expr()],
        )
    }
}

// Negation for Symbol
impl std::ops::Neg for Symbol {
    type Output = Expr;
    fn neg(self) -> Expr {
        self.to_expr().negate()
    }
}

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

// Arc<Expr> conversions (for CustomFn partials ergonomics)
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
