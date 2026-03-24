//! Public API for end-users of the library.
//!
//! These items are part of the stable external interface:
//! - [`Symbol`] — the ergonomic, `Copy` symbol handle
//! - [`SymbolError`] — error type for registry operations
//! - Registry functions visible to crate consumers

use std::sync::Arc;

use slotmap::{DefaultKey, Key};

use crate::Expr;
use crate::core::known_symbols as ks;

use super::logic::interned::InternedSymbol;
use super::logic::registry::{lookup_by_id, symb_anon};

// ============================================================================
// Public re-exports
// ============================================================================

pub use super::logic::operators::ArcExprExt;
pub use super::logic::registry::{
    clear_symbols, remove_symbol, symb, symb_get, symb_new, symbol_count, symbol_exists,
    symbol_names,
};

// ============================================================================
// SymbolError
// ============================================================================

/// Errors that can occur during symbol operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolError {
    /// Attempted to create a symbol with a name that's already registered.
    DuplicateName(String),
    /// Attempted to get a symbol that doesn't exist.
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
// Symbol
// ============================================================================

/// Type-safe, `Copy` handle to an interned symbol.
///
/// Symbols are interned — each unique name exists exactly once in memory, and
/// all handles share the same ID for O(1) equality comparisons.
///
/// ```
/// use symb_anafis::symb;
/// let a = symb("symbol_doc_a");
/// let expr = a + a;  // Copy — no clone needed
/// assert!(format!("{}", expr).contains("symbol_doc_a"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(pub(crate) DefaultKey);

impl Symbol {
    /// Create a new anonymous symbol (unique ID, no string name).
    #[must_use]
    pub fn anon() -> Self {
        symb_anon()
    }

    /// Reconstruct a Symbol from a previously obtained ID.
    #[inline]
    #[must_use]
    pub fn from_id(id: u64) -> Self {
        Self(super::logic::registry::key_from_id(id))
    }

    /// The symbol's unique integer ID.
    #[inline]
    #[must_use]
    pub fn id(&self) -> u64 {
        self.0.data().as_ffi()
    }

    /// The symbol's internal slotmap key.
    #[inline]
    #[must_use]
    pub const fn key(&self) -> DefaultKey {
        self.0
    }

    /// The symbol's name, or `None` for anonymous symbols.
    ///
    /// Prefer [`name_arc`](Self::name_arc) in hot paths to avoid allocation.
    #[must_use]
    pub fn name(&self) -> Option<String> {
        self.name_arc().map(|arc| arc.to_string())
    }

    /// The symbol's name as an `Arc<str>` (no allocation on subsequent calls).
    #[must_use]
    pub fn name_arc(&self) -> Option<Arc<str>> {
        lookup_by_id(self.id()).and_then(|s| s.name_arc())
    }

    /// Convert to an `Expr`.
    #[must_use]
    pub fn to_expr(&self) -> Expr {
        lookup_by_id(self.id()).map_or_else(
            || Expr::from_interned(InternedSymbol::new_anon_with_key(self.0)),
            Expr::from_interned,
        )
    }

    /// Raise to a power: `self ^ exp`.
    pub fn pow(&self, exp: impl Into<Expr>) -> Expr {
        Expr::pow_static(self.to_expr(), exp.into())
    }

    // =========================================================================
    // Parametric special functions
    // =========================================================================

    /// Polygamma ψ^(n)(self)
    pub fn polygamma(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.polygamma),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Beta function B(self, other)
    pub fn beta(&self, other: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.beta),
            vec![self.to_expr(), other.into()],
        )
    }

    /// Bessel `J_n(self)`
    pub fn besselj(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.besselj),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Bessel `Y_n(self)`
    pub fn bessely(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.bessely),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Modified Bessel `I_n(self)`
    pub fn besseli(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.besseli),
            vec![n.into(), self.to_expr()],
        )
    }

    /// Modified Bessel `K_n(self)`
    pub fn besselk(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.besselk),
            vec![n.into(), self.to_expr()],
        )
    }

    /// `log(base, self)`
    ///
    /// ```
    /// use symb_anafis::symb;
    /// let x = symb("log_example_x");
    /// assert_eq!(format!("{}", x.log(2.0)), "log(2, log_example_x)");
    /// ```
    pub fn log(&self, base: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.log),
            vec![base.into(), self.to_expr()],
        )
    }

    /// `atan2(self, x)`
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

    /// Associated Legendre `P_l^m(self)`
    pub fn assoc_legendre(&self, l: impl Into<Expr>, m: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.assoc_legendre),
            vec![l.into(), m.into(), self.to_expr()],
        )
    }

    /// Spherical harmonic `Y_l^m(self=θ`, φ)
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

    /// `Y_l^m` alias for [`spherical_harmonic`](Self::spherical_harmonic)
    pub fn ynm(&self, l: impl Into<Expr>, m: impl Into<Expr>, phi: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.ynm),
            vec![l.into(), m.into(), self.to_expr(), phi.into()],
        )
    }

    /// Riemann zeta derivative ζ^(n)(self)
    pub fn zeta_deriv(&self, n: impl Into<Expr>) -> Expr {
        Expr::func_multi_symbol(
            ks::get_symbol(ks::KS.zeta_deriv),
            vec![n.into(), self.to_expr()],
        )
    }
}

// ============================================================================
// Trait implementations
// ============================================================================

impl std::ops::Neg for Symbol {
    type Output = Expr;
    fn neg(self) -> Expr {
        self.to_expr().negate()
    }
}

impl From<Symbol> for Expr {
    fn from(s: Symbol) -> Self {
        s.to_expr()
    }
}

impl From<f64> for Expr {
    fn from(n: f64) -> Self {
        Self::number(n)
    }
}

impl From<i32> for Expr {
    fn from(n: i32) -> Self {
        Self::number(f64::from(n))
    }
}

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
