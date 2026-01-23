//! Expression constructors.
//!
//! Provides all constructor methods for building expressions.

use std::sync::Arc;

use super::{EPSILON, Expr, ExprKind, compute_expr_hash, expr_cmp, next_id};
use crate::core::symbol::{InternedSymbol, symb_interned};

impl Expr {
    /// Create a new expression with fresh ID
    #[must_use]
    pub fn new(kind: ExprKind) -> Self {
        let hash = compute_expr_hash(&kind);
        Self {
            id: next_id(),
            hash,
            kind,
        }
    }

    /// Get the unique ID of the expression
    #[inline]
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Get the structural hash of the expression
    #[inline]
    #[must_use]
    pub const fn structural_hash(&self) -> u64 {
        self.hash
    }

    /// Consume the expression and return its kind
    #[inline]
    #[must_use]
    pub fn into_kind(self) -> ExprKind {
        self.kind
    }

    // -------------------------------------------------------------------------
    // Accessor methods
    // -------------------------------------------------------------------------

    /// Check if expression is a constant number and return its value
    #[inline]
    #[must_use]
    pub const fn as_number(&self) -> Option<f64> {
        match &self.kind {
            ExprKind::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Check if this expression is the number zero (with tolerance)
    #[inline]
    pub fn is_zero_num(&self) -> bool {
        self.as_number().is_some_and(crate::core::traits::is_zero)
    }

    /// Check if this expression is the number one (with tolerance)
    #[inline]
    pub fn is_one_num(&self) -> bool {
        self.as_number().is_some_and(crate::core::traits::is_one)
    }

    /// Check if this expression is the number negative one (with tolerance)
    #[inline]
    pub fn is_neg_one_num(&self) -> bool {
        self.as_number()
            .is_some_and(crate::core::traits::is_neg_one)
    }

    // -------------------------------------------------------------------------
    // Basic constructors
    // -------------------------------------------------------------------------

    /// Create a number expression
    #[must_use]
    pub fn number(n: f64) -> Self {
        Self::new(ExprKind::Number(n))
    }

    /// Create a symbol expression (auto-interned)
    pub fn symbol(s: impl AsRef<str>) -> Self {
        Self::new(ExprKind::Symbol(symb_interned(s.as_ref())))
    }

    /// Create from an already-interned symbol
    pub(crate) fn from_interned(interned: InternedSymbol) -> Self {
        Self::new(ExprKind::Symbol(interned))
    }

    /// Create a function call from an already-interned symbol (single argument)
    pub(crate) fn func_symbol(name: InternedSymbol, arg: Self) -> Self {
        Self::new(ExprKind::FunctionCall {
            name,
            args: vec![Arc::new(arg)],
        })
    }

    /// Create a polynomial expression directly
    #[must_use]
    pub fn poly(p: crate::core::poly::Polynomial) -> Self {
        // Empty polynomial is 0
        if p.terms().is_empty() {
            return Self::number(0.0);
        }
        // Single constant term (pow=0) is just a number
        if p.terms().len() == 1 && p.terms()[0].0 == 0 {
            return Self::number(p.terms()[0].1);
        }
        Self::new(ExprKind::Poly(p))
    }

    // -------------------------------------------------------------------------
    // N-ary Sum constructor (smart - flattens and sorts)
    // -------------------------------------------------------------------------

    /// Create a sum expression from terms.
    /// Flattens nested sums. Sorting and like-term combination is deferred to simplification
    /// for performance (avoids O(N²) cascade during differentiation).
    ///
    /// Auto-optimization: If 3+ terms form a pure polynomial (only numbers, symbols,
    /// products of coeff*symbol^n), converts to Poly for O(N) differentiation.
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use).
    #[must_use]
    pub fn sum(terms: Vec<Self>) -> Self {
        if terms.is_empty() {
            return Self::number(0.0);
        }
        if terms.len() == 1 {
            return terms
                .into_iter()
                .next()
                .expect("Vec must have at least one element");
        }

        let mut flat: Vec<Arc<Self>> = Vec::with_capacity(terms.len());
        let mut numeric_sum: f64 = 0.0;

        for t in terms {
            match t.kind {
                ExprKind::Sum(inner) => flat.extend(inner),
                ExprKind::Poly(poly) => {
                    // Flatten Poly into its terms (they'll be merged later)
                    for term_expr in poly.to_expr_terms() {
                        flat.push(Arc::new(term_expr));
                    }
                }
                ExprKind::Number(n) => numeric_sum += n, // Combine numbers immediately
                _ => flat.push(Arc::new(t)),
            }
        }

        // Add accumulated numeric constant at the BEGINNING (canonical order: numbers first)
        // Build new vector with capacity to avoid O(n) insert
        let flat = if numeric_sum.abs() > EPSILON {
            let mut with_num = Vec::with_capacity(flat.len() + 1);
            with_num.push(Arc::new(Self::number(numeric_sum)));
            with_num.extend(flat);
            with_num
        } else {
            flat
        };

        if flat.is_empty() {
            return Self::number(0.0);
        }
        if flat.len() == 1 {
            return Arc::try_unwrap(
                flat.into_iter()
                    .next()
                    .expect("flat must have exactly one element"),
            )
            .unwrap_or_else(|arc| (*arc).clone());
        }

        // Streaming incremental Poly building: if term has same base as last, combine into Poly
        // Since terms are canonically sorted, same-base terms are always adjacent
        if flat.len() >= 2 {
            let mut result: Vec<Arc<Self>> = Vec::new();
            let mut last_base: Option<u64> = None;

            for term in flat {
                let current_base = get_poly_base_hash(&term);

                if current_base.is_some() && current_base == last_base && !result.is_empty() {
                    // Same base as last term - try to merge
                    let last_arc = result.pop().expect("result cannot be empty here");

                    // Unwrap locally to get ownership (or clone if shared)
                    let mut last_expr = Arc::try_unwrap(last_arc).unwrap_or_else(|a| (*a).clone());

                    // Try to parse current term as polynomial
                    // Optimization: if term is simple, this is cheap
                    if let Some(term_poly) = crate::core::poly::Polynomial::try_from_expr(&term) {
                        // Case A: Last is already a Poly - try in-place add
                        if let ExprKind::Poly(ref mut p) = last_expr.kind {
                            if p.try_add_assign(&term_poly) {
                                result.push(Arc::new(last_expr));
                                continue;
                            }
                        }
                        // Case B: Last is not a Poly yet, or merge failed (shouldn't if bases match)
                        // Try to convert last to Poly and then merge
                        else if let Some(dest_poly) =
                            crate::core::poly::Polynomial::try_from_expr(&last_expr)
                        {
                            // Create new Poly from last
                            let mut new_poly = dest_poly; // Move
                            if new_poly.try_add_assign(&term_poly) {
                                result.push(Arc::new(Self::poly(new_poly)));
                                continue;
                            }
                        }
                    }

                    // Fallback: could not merge. Push both separately.
                    // (Should not happen if get_poly_base_hash is correct)
                    result.push(Arc::new(last_expr));
                }
                result.push(term);
                last_base = current_base;
            }

            if result.len() == 1 {
                return Arc::try_unwrap(
                    result.pop().expect("result must have exactly one element"),
                )
                .unwrap_or_else(|arc| (*arc).clone());
            }
            return Self::new(ExprKind::Sum(result));
        }

        Self::new(ExprKind::Sum(flat))
    }

    /// Create sum from Arc terms (flattens only, sorting deferred to simplification)
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use).
    #[must_use]
    pub fn sum_from_arcs(terms: Vec<Arc<Self>>) -> Self {
        if terms.is_empty() {
            return Self::number(0.0);
        }
        if terms.len() == 1 {
            return Arc::try_unwrap(
                terms
                    .into_iter()
                    .next()
                    .expect("Vec must have exactly one element"),
            )
            .unwrap_or_else(|arc| (*arc).clone());
        }

        // Flatten nested sums and combine numbers
        let mut flat: Vec<Arc<Self>> = Vec::with_capacity(terms.len());
        let mut numeric_sum: f64 = 0.0;

        for t in terms {
            // Check for Number
            if let ExprKind::Number(n) = t.kind {
                numeric_sum += n;
                continue;
            }

            // Check for Sum (flattening)
            if matches!(t.kind, ExprKind::Sum(_)) {
                // Try to unwrap to avoid cloning inner vector elements
                match Arc::try_unwrap(t) {
                    Ok(expr) => {
                        if let ExprKind::Sum(inner) = expr.kind {
                            flat.extend(inner);
                        }
                    }
                    Err(arc) => {
                        if let ExprKind::Sum(inner) = &arc.kind {
                            flat.extend(inner.iter().cloned());
                        }
                    }
                }
                continue;
            }

            // Default: push the Arc directly (move, don't clone)
            flat.push(t);
        }

        // Add accumulated numeric constant at the BEGINNING (canonical order: numbers first)
        // Build new vector with capacity to avoid O(n) insert
        let flat = if numeric_sum.abs() > EPSILON {
            let mut with_num = Vec::with_capacity(flat.len() + 1);
            with_num.push(Arc::new(Self::number(numeric_sum)));
            with_num.extend(flat);
            with_num
        } else {
            flat
        };

        if flat.is_empty() {
            return Self::number(0.0);
        }
        if flat.len() == 1 {
            return Arc::try_unwrap(
                flat.into_iter()
                    .next()
                    .expect("flat must have exactly one element"),
            )
            .unwrap_or_else(|arc| (*arc).clone());
        }

        Self::new(ExprKind::Sum(flat))
    }

    // -------------------------------------------------------------------------
    // N-ary Product constructor (smart - flattens and sorts)
    // -------------------------------------------------------------------------

    /// Create a product expression from factors.
    /// Flattens nested products. Sorting deferred to simplification.
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use).
    #[must_use]
    pub fn product(factors: Vec<Self>) -> Self {
        if factors.is_empty() {
            return Self::number(1.0);
        }
        if factors.len() == 1 {
            return factors
                .into_iter()
                .next()
                .expect("Vec must have exactly one element");
        }

        let mut flat: Vec<Arc<Self>> = Vec::with_capacity(factors.len());
        let mut numeric_prod: f64 = 1.0;

        for f in factors {
            match f.kind {
                ExprKind::Product(inner) => flat.extend(inner),
                ExprKind::Number(n) => {
                    if n == 0.0 {
                        return Self::number(0.0); // Early exit for zero
                    }
                    numeric_prod *= n;
                }
                _ => flat.push(Arc::new(f)),
            }
        }

        // Add numeric coefficient at the BEGINNING if not 1.0 (canonical order: numbers first)
        // Build new vector with capacity to avoid O(n) insert
        let flat = if (numeric_prod - 1.0).abs() > EPSILON {
            let mut with_coeff = Vec::with_capacity(flat.len() + 1);
            with_coeff.push(Arc::new(Self::number(numeric_prod)));
            with_coeff.extend(flat);
            with_coeff
        } else {
            flat
        };

        if flat.is_empty() {
            return Self::number(1.0);
        }
        if flat.len() == 1 {
            return Arc::try_unwrap(
                flat.into_iter()
                    .next()
                    .expect("flat must have exactly one element"),
            )
            .unwrap_or_else(|arc| (*arc).clone());
        }

        Self::new(ExprKind::Product(flat))
    }

    /// Create product from Arc factors (flattens and sorts for canonical form)
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use).
    #[must_use]
    pub fn product_from_arcs(factors: Vec<Arc<Self>>) -> Self {
        if factors.is_empty() {
            return Self::number(1.0);
        }
        if factors.len() == 1 {
            return Arc::try_unwrap(
                factors
                    .into_iter()
                    .next()
                    .expect("Vec must have exactly one element"),
            )
            .unwrap_or_else(|arc| (*arc).clone());
        }

        // Flatten nested products
        let mut flat: Vec<Arc<Self>> = Vec::with_capacity(factors.len());
        for f in factors {
            match &f.kind {
                ExprKind::Product(inner) => flat.extend(inner.clone()),
                _ => flat.push(f),
            }
        }

        if flat.len() == 1 {
            return Arc::try_unwrap(flat.pop().expect("Vec must have exactly one element"))
                .unwrap_or_else(|arc| (*arc).clone());
        }

        // Sort for canonical form
        flat.sort_by(|a, b| expr_cmp(a, b));
        Self::new(ExprKind::Product(flat))
    }

    // -------------------------------------------------------------------------
    // Binary operation constructors (for legacy compatibility during migration)
    // -------------------------------------------------------------------------

    /// Create addition: a + b → Sum([a, b])
    #[must_use]
    pub fn add_expr(left: Self, right: Self) -> Self {
        Self::sum(vec![left, right])
    }

    /// Create subtraction: a - b → Sum([a, Product([-1, b])])
    #[must_use]
    pub fn sub_expr(left: Self, right: Self) -> Self {
        let neg_right = Self::product(vec![Self::number(-1.0), right]);
        Self::sum(vec![left, neg_right])
    }

    /// Create multiplication: a * b → Product([a, b])
    ///
    /// Optimization: If both operands are Poly with same base, use fast O(N*M) polynomial multiplication
    #[must_use]
    pub fn mul_expr(left: Self, right: Self) -> Self {
        // Optimization: 0 * x = 0, x * 0 = 0
        if left.is_zero_num() || right.is_zero_num() {
            return Self::number(0.0);
        }
        // Optimization: 1 * x = x, x * 1 = x
        if left.is_one_num() {
            return right;
        }
        if right.is_one_num() {
            return left;
        }

        // Fast path: Poly * Poly with same base uses polynomial multiplication
        if let (ExprKind::Poly(p1), ExprKind::Poly(p2)) = (&left.kind, &right.kind) {
            // Only use polynomial mul if bases match
            if p1.base() == p2.base() {
                let result = p1.mul(p2);
                return Self::poly(result);
            }
            // Different bases: fall through to Product
        }

        Self::product(vec![left, right])
    }

    /// Create multiplication from Arc operands (avoids Expr cloning)
    #[must_use]
    pub fn mul_from_arcs(factors: Vec<Arc<Self>>) -> Self {
        Self::product_from_arcs(factors)
    }

    /// Unwrap an `Arc<Expr>` without cloning if refcount is 1
    #[inline]
    #[must_use]
    pub fn unwrap_arc(arc: Arc<Self>) -> Self {
        Arc::try_unwrap(arc).unwrap_or_else(|a| (*a).clone())
    }

    /// Create division
    #[must_use]
    pub fn div_expr(left: Self, right: Self) -> Self {
        Self::new(ExprKind::Div(Arc::new(left), Arc::new(right)))
    }

    /// Create division from Arc operands (avoids cloning if Arc ref count is 1)
    #[must_use]
    pub fn div_from_arcs(left: Arc<Self>, right: Arc<Self>) -> Self {
        Self::new(ExprKind::Div(left, right))
    }

    /// Create power expression (static constructor form)
    ///
    /// For the method form `expr.pow(exp)`, see the `pow` method on Expr.
    #[must_use]
    pub fn pow_static(base: Self, exponent: Self) -> Self {
        Self::new(ExprKind::Pow(Arc::new(base), Arc::new(exponent)))
    }

    /// Create power from Arc operands (avoids cloning if Arc ref count is 1)
    #[must_use]
    pub fn pow_from_arcs(base: Arc<Self>, exponent: Arc<Self>) -> Self {
        Self::new(ExprKind::Pow(base, exponent))
    }

    /// Create a function call expression (single argument)
    ///
    /// Accepts `Expr`, `Arc<Expr>`, or `&Arc<Expr>` as the content parameter.
    pub fn func(name: impl AsRef<str>, content: impl Into<Self>) -> Self {
        Self::new(ExprKind::FunctionCall {
            name: symb_interned(name.as_ref()),
            args: vec![Arc::new(content.into())],
        })
    }

    /// Create a multi-argument function call
    pub fn func_multi(name: impl AsRef<str>, args: Vec<Self>) -> Self {
        Self::new(ExprKind::FunctionCall {
            name: symb_interned(name.as_ref()),
            args: args.into_iter().map(Arc::new).collect(),
        })
    }

    /// Create a multi-argument function call using `InternedSymbol` (more efficient)
    pub(crate) fn func_multi_symbol(name: InternedSymbol, args: Vec<Self>) -> Self {
        Self::new(ExprKind::FunctionCall {
            name,
            args: args.into_iter().map(Arc::new).collect(),
        })
    }

    /// Create a function call from Arc arguments (avoids cloning)
    pub fn func_multi_from_arcs(name: impl AsRef<str>, args: Vec<Arc<Self>>) -> Self {
        Self::new(ExprKind::FunctionCall {
            name: symb_interned(name.as_ref()),
            args,
        })
    }

    /// Create a function call from Arc arguments using `InternedSymbol` (most efficient)
    pub(crate) fn func_multi_from_arcs_symbol(name: InternedSymbol, args: Vec<Arc<Self>>) -> Self {
        Self::new(ExprKind::FunctionCall { name, args })
    }

    /// Create a function call with explicit arguments using array syntax
    pub fn call<const N: usize>(name: impl AsRef<str>, args: [Self; N]) -> Self {
        Self::func_multi(name, args.into())
    }

    /// Create a partial derivative expression
    pub fn derivative(inner: Self, var: impl AsRef<str>, order: u32) -> Self {
        Self::new(ExprKind::Derivative {
            inner: Arc::new(inner),
            var: symb_interned(var.as_ref()),
            order,
        })
    }

    /// Create a partial derivative expression with an already-interned symbol
    pub(crate) fn derivative_interned(inner: Self, var: InternedSymbol, order: u32) -> Self {
        Self::new(ExprKind::Derivative {
            inner: Arc::new(inner),
            var,
            order,
        })
    }

    // -------------------------------------------------------------------------
    // Negation helper
    // -------------------------------------------------------------------------

    /// Negate this expression: -x = Product([-1, x])
    #[must_use]
    pub fn negate(self) -> Self {
        Self::product(vec![Self::number(-1.0), self])
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Extract the polynomial base structural hash from a term.
/// For polynomial-like terms (x, x^2, 3*x^3, sin(x)^2, 2*cos(x)), returns a hash
/// that identifies the base expression so adjacent terms with the same base can be merged.
/// Returns None if term is not polynomial-like.
fn get_poly_base_hash(expr: &Expr) -> Option<u64> {
    match &expr.kind {
        // Symbol x or FunctionCall like sin(x) → use its structural hash
        ExprKind::Symbol(_) | ExprKind::FunctionCall { .. } => Some(expr.hash),

        // base^n where n is a positive integer → base's hash
        ExprKind::Pow(base, exp) => {
            if let ExprKind::Number(n) = &exp.kind
                && *n >= 1.0
                && n.fract().abs() < EPSILON
            {
                return Some(base.hash);
            }
            None
        }

        // c*base or c*base^n → extract the non-numeric base's hash
        ExprKind::Product(factors) => {
            let mut base_hash = None;
            for f in factors {
                match &f.kind {
                    ExprKind::Number(_) => {} // Skip coefficients
                    ExprKind::Symbol(_) | ExprKind::FunctionCall { .. } => {
                        if base_hash.is_some() {
                            return None; // Multiple bases
                        }
                        base_hash = Some(f.hash);
                    }
                    ExprKind::Pow(b, exp) => {
                        if let ExprKind::Number(n) = &exp.kind
                            && *n >= 1.0
                            && n.fract().abs() < EPSILON
                        {
                            if base_hash.is_some() {
                                return None; // Multiple bases
                            }
                            base_hash = Some(b.hash);
                            continue;
                        }
                        return None;
                    }
                    _ => return None, // Other complex expressions
                }
            }
            base_hash
        }

        // Constants have "base" 0 (can combine with any polynomial)
        ExprKind::Number(_) => Some(0),

        _ => None,
    }
}
