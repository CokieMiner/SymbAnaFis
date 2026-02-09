//! Abstract Syntax Tree for mathematical expressions.
//!
//! This module defines:
//! - `Expr` - The central AST node type
//! - `ExprKind` - The variants of expression nodes (Number, Symbol, Function, etc.)
//! - Formatting and display traits (Display, LaTeX, etc.)
//!
//! # Architecture
//!
//! The expression system uses several key design decisions for performance:
//!
//! ## N-ary Sum/Product
//! Instead of binary `Add(left, right)`, we use N-ary `Sum(Vec<Arc<Expr>>)`.
//! This allows efficient simplification without deep recursion:
//! - `a + b + c + d` is `Sum([a, b, c, d])` not `Add(Add(Add(a,b),c),d)`
//! - Flattening happens automatically in constructors
//! - Like-term combination is O(N) instead of O(N²)
//!
//! ## Structural Hashing
//! Each `Expr` has a pre-computed `hash` field for O(1) equality rejection.
//! Two expressions with different hashes are definitely not equal, avoiding
//! expensive recursive comparisons in the common case.
//!
//! ## Symbol Interning
//! Variables use [`InternedSymbol`] with numeric IDs for O(1) equality comparison
//! instead of O(N) string comparison.
//!
//! ## Polynomial Optimization
//! When 3+ terms form a pure polynomial (e.g., `x² + 2x + 1`), they are
//! automatically converted to [`Poly`](ExprKind::Poly) for O(N) differentiation
//! instead of O(N²) term-by-term processing.
//!
//! # Usage
//!
//! ```
//! use symb_anafis::{symb, Expr};
//!
//! // Create expressions using symbols and operators
//! let x = symb("x");
//! let expr = x.pow(2.0) + x.sin();  // x² + sin(x)
//!
//! // Or use constructors directly
//! let sum = Expr::sum(vec![Expr::symbol("a"), Expr::symbol("b")]);
//! ```

// Submodules
mod analysis;
mod constructors;
mod evaluate;
mod hash;
mod ordering;

use rustc_hash::FxHasher;
use std::hash::Hasher;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::symbol::InternedSymbol;

// Re-exports from submodules
pub use hash::compute_expr_hash;
pub use ordering::expr_cmp;

/// Type alias for custom evaluation functions map
pub type CustomEvalMap =
    std::collections::HashMap<String, std::sync::Arc<dyn Fn(&[f64]) -> Option<f64> + Send + Sync>>;

// Re-export EPSILON from traits for backward compatibility
pub use crate::core::traits::EPSILON;

// =============================================================================
// EXPRESSION ID COUNTER AND CACHED CONSTANTS
// =============================================================================

/// Global counter for generating unique expression IDs
static EXPR_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate the next unique expression ID
pub fn next_id() -> u64 {
    EXPR_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Cached hash for number 1.0 (used in comparisons)
pub(super) static EXPR_ONE_HASH: std::sync::LazyLock<u64> =
    std::sync::LazyLock::new(|| compute_expr_hash(&ExprKind::Number(1.0)));

/// Cached Expr for number 1.0 to avoid repeated allocations in comparison hot path
pub(super) static EXPR_ONE: std::sync::LazyLock<Expr> = std::sync::LazyLock::new(|| Expr {
    id: 0, // ID doesn't matter for comparison (not used in eq/hash)
    hash: *EXPR_ONE_HASH,
    kind: ExprKind::Number(1.0),
});

/// Cached Arc<Expr> for 0.0, used during Drop to swap out children without allocation
static DUMMY_ARC: std::sync::LazyLock<Arc<Expr>> = std::sync::LazyLock::new(|| {
    Arc::new(Expr {
        id: 0,
        hash: compute_expr_hash(&ExprKind::Number(0.0)),
        kind: ExprKind::Number(0.0),
    })
});

// =============================================================================
// EXPR - The main expression type
// =============================================================================

/// A symbolic mathematical expression.
///
/// This is the core type representing mathematical expressions in AST form.
/// Expressions can be constants, variables, function calls, arithmetic operations,
/// or derivatives. They support operator overloading for convenient construction.
///
/// # Example
/// ```
/// use symb_anafis::{symb, Expr};
///
/// let x = symb("x");
/// let expr = x.pow(2.0) + x.sin();  // x² + sin(x)
/// ```
#[derive(Debug, Clone)]
pub struct Expr {
    /// Unique ID for debugging and caching (not used in equality comparisons)
    pub(crate) id: u64,
    /// Structural hash for O(1) equality rejection (Phase 7b optimization)
    pub(crate) hash: u64,
    /// The kind of expression (structure)
    pub(crate) kind: ExprKind,
}

impl Deref for Expr {
    type Target = ExprKind;
    fn deref(&self) -> &Self::Target {
        &self.kind
    }
}

// Structural equality based on KIND only (with hash fast-reject)
impl PartialEq for Expr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Fast reject: different hashes mean definitely not equal
        if self.hash != other.hash {
            return false;
        }
        // Slow path: verify structural equality (handles hash collisions)
        self.kind == other.kind
    }
}

impl Eq for Expr {}

impl std::hash::Hash for Expr {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Use pre-computed hash directly
        self.hash.hash(state);
    }
}

// =============================================================================
// EXPRKIND - N-ary Sum/Product architecture
// =============================================================================

/// The kind (structure) of an expression node.
///
/// This enum defines all possible expression types in the AST.
/// Most variants use `Arc<Expr>` for efficient sharing and cloning.
#[derive(Debug, Clone, PartialEq)]
#[allow(
    private_interfaces,
    reason = "InternedSymbol is pub(crate) but exposed here for pattern matching"
)]
pub enum ExprKind {
    /// Constant number (e.g., 3.14, 1e10)
    Number(f64),

    /// Variable or constant symbol (e.g., "x", "a", "pi")
    /// Uses `InternedSymbol` for O(1) equality comparisons
    Symbol(InternedSymbol),

    /// Function call (built-in or custom).
    /// Args use `Arc<Expr>` for consistency with Sum/Product.
    FunctionCall {
        /// The function name as an interned symbol.
        name: InternedSymbol,
        /// The function arguments.
        args: Vec<Arc<Expr>>,
    },

    /// N-ary sum: a + b + c + ...
    /// Stored flat and sorted into a canonical order during construction.
    /// Subtraction is represented as: a - b = Sum([a, Product([-1, b])])
    Sum(Vec<Arc<Expr>>),

    /// N-ary product: a * b * c * ...
    /// Stored flat. Sorting/canonicalization is deferred to simplification.
    Product(Vec<Arc<Expr>>),

    /// Division (binary - not associative)
    Div(Arc<Expr>, Arc<Expr>),

    /// Exponentiation (binary - not associative)
    Pow(Arc<Expr>, Arc<Expr>),

    /// Partial derivative notation: ∂^order/∂var^order of inner expression
    ///
    /// Uses `InternedSymbol` for the variable to enable O(1) comparison
    /// instead of O(N) string comparison.
    Derivative {
        /// The expression being differentiated.
        inner: Arc<Expr>,
        /// The variable to differentiate with respect to (interned for O(1) comparison).
        var: InternedSymbol,
        /// The order of differentiation (1 = first derivative, 2 = second, etc.)
        order: u32,
    },

    /// Polynomial in sparse representation (coefficient * powers)
    /// Used for efficient polynomial operations (differentiation, multiplication)
    Poly(crate::core::poly::Polynomial),
}

// =============================================================================
// DROP IMPLEMENTATION - Iterative drop to prevent stack overflow
// =============================================================================

impl Drop for Expr {
    fn drop(&mut self) {
        fn drain_children(kind: &mut ExprKind, queue: &mut Vec<Arc<Expr>>) {
            match kind {
                ExprKind::FunctionCall { args, .. } => {
                    queue.extend(std::mem::take(args));
                }
                ExprKind::Sum(terms) => {
                    queue.extend(std::mem::take(terms));
                }
                ExprKind::Product(factors) => {
                    queue.extend(std::mem::take(factors));
                }
                ExprKind::Div(left, right) => {
                    let dummy = Arc::clone(&DUMMY_ARC);
                    queue.push(std::mem::replace(left, Arc::clone(&dummy)));
                    queue.push(std::mem::replace(right, Arc::clone(&dummy)));
                }
                ExprKind::Pow(base, exp) => {
                    let dummy = Arc::clone(&DUMMY_ARC);
                    queue.push(std::mem::replace(base, Arc::clone(&dummy)));
                    queue.push(std::mem::replace(exp, Arc::clone(&dummy)));
                }
                ExprKind::Derivative { inner, .. } => {
                    let dummy = DUMMY_ARC.clone();
                    queue.push(std::mem::replace(inner, dummy));
                }
                ExprKind::Poly(poly) => {
                    queue.push(poly.take_base());
                }
                ExprKind::Number(_) | ExprKind::Symbol(_) => {}
            }
        }

        let mut work_queue = Vec::new();
        drain_children(&mut self.kind, &mut work_queue);

        while let Some(child_arc) = work_queue.pop() {
            if let Ok(mut child_expr) = Arc::try_unwrap(child_arc) {
                drain_children(&mut child_expr.kind, &mut work_queue);
            }
        }
    }
}

// =============================================================================
// HASH FOR EXPRKIND
// =============================================================================

impl std::hash::Hash for ExprKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Number(n) => {
                // Normalize -0.0 to 0.0 before hashing
                let normalized = if *n == 0.0 { 0.0 } else { *n };
                normalized.to_bits().hash(state);
            }
            Self::Symbol(s) => s.hash(state),
            Self::FunctionCall { name, args } => {
                name.hash(state);
                args.hash(state);
            }
            Self::Sum(terms) => {
                // Commutative hash: hash of the sum of children hashes
                let mut sum_hash: u64 = 0;
                for t in terms {
                    sum_hash = sum_hash.wrapping_add(t.hash);
                }
                sum_hash.hash(state);
            }
            Self::Product(factors) => {
                // Commutative hash: hash of the sum of children hashes
                let mut prod_hash: u64 = 0;
                for f in factors {
                    prod_hash = prod_hash.wrapping_add(f.hash);
                }
                prod_hash.hash(state);
            }
            Self::Div(l, r) | Self::Pow(l, r) => {
                l.hash(state);
                r.hash(state);
            }
            Self::Derivative { inner, var, order } => {
                inner.hash(state);
                var.hash(state);
                order.hash(state);
            }
            Self::Poly(poly) => {
                poly.base().hash.hash(state);
                // Commutative hash for terms
                let mut terms_hash: u64 = 0;
                for &(pow, coeff) in poly.terms() {
                    let mut term_hasher = FxHasher::default();
                    coeff.to_bits().hash(&mut term_hasher);
                    pow.hash(&mut term_hasher);
                    terms_hash = terms_hash.wrapping_add(term_hasher.finish());
                }
                terms_hash.hash(state);
            }
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_flattening() {
        let x = Expr::symbol("x");
        let y = Expr::symbol("y");
        let z = Expr::symbol("z");

        // (x + y) + z should flatten to Sum or Poly (3+ terms may become Poly)
        let inner = Expr::sum(vec![x, y]);
        let outer = Expr::sum(vec![inner, z]);

        match &outer.kind {
            ExprKind::Sum(terms) => assert_eq!(terms.len(), 3),
            ExprKind::Poly(poly) => assert_eq!(poly.terms().len(), 3),
            _ => panic!("Expected Sum or Poly"),
        }
    }

    #[test]
    fn test_product_flattening() {
        let a = Expr::symbol("a");
        let b = Expr::symbol("b");
        let c = Expr::symbol("c");

        // (a * b) * c should flatten to Product([a, b, c])
        let inner = Expr::product(vec![a, b]);
        let outer = Expr::product(vec![inner, c]);

        match &outer.kind {
            ExprKind::Product(factors) => assert_eq!(factors.len(), 3),
            _ => panic!("Expected Product"),
        }
    }

    #[test]
    fn test_subtraction_as_sum() {
        let x = Expr::symbol("x");
        let y = Expr::symbol("y");

        // x - y = Sum([x, Product([-1, y])])
        let result = Expr::sub_expr(x, y);

        match &result.kind {
            ExprKind::Sum(terms) => {
                assert_eq!(terms.len(), 2);
            }
            _ => panic!("Expected Sum from subtraction"),
        }
    }
}
