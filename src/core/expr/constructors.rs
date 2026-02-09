//! Expression constructors.
//!
//! Provides all constructor methods for building expressions.

use std::cmp::Ordering as CmpOrdering;
use std::sync::Arc;

use super::{EPSILON, Expr, ExprKind, compute_expr_hash, expr_cmp, next_id};
use crate::core::symbol::{InternedSymbol, symb_interned};

impl Expr {
    /// Create a new expression with fresh ID
    #[inline]
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
    pub fn into_kind(mut self) -> ExprKind {
        std::mem::replace(&mut self.kind, ExprKind::Number(0.0))
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
    #[inline]
    #[must_use]
    pub fn number(n: f64) -> Self {
        Self::new(ExprKind::Number(n))
    }

    /// Create a symbol expression (auto-interned)
    pub fn symbol(s: impl AsRef<str>) -> Self {
        Self::new(ExprKind::Symbol(symb_interned(s.as_ref())))
    }

    /// Create from an already-interned symbol
    #[inline]
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
    /// Flattens nested sums and sorts terms into a canonical order.
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
            if matches!(t.kind, ExprKind::Sum(_) | ExprKind::Number(_)) {
                match t.into_kind() {
                    ExprKind::Sum(inner) => flat.extend(inner),
                    ExprKind::Number(n) => numeric_sum += n,
                    // Invariant protected by if guard
                    _ => {}
                }
            } else {
                flat.push(Arc::new(t));
            }
        }

        // Add accumulated numeric constant (canonical order: numbers first)
        if numeric_sum.abs() > EPSILON {
            flat.push(Arc::new(Self::number(numeric_sum)));
        }

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

        finalize_sum(flat)
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

        // Fast-path: if no term is a sum or number, skip flattening loop
        if !terms
            .iter()
            .any(|t| matches!(t.kind, ExprKind::Sum(_) | ExprKind::Number(_)))
        {
            return finalize_sum(terms);
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
                        if let ExprKind::Sum(inner) = expr.into_kind() {
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

        // Add accumulated numeric constant (canonical order: numbers first)
        if numeric_sum.abs() > EPSILON {
            flat.push(Arc::new(Self::number(numeric_sum)));
        }

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

        finalize_sum(flat)
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
        Self::product_from_arcs(factors.into_iter().map(Arc::new).collect())
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

        // Fast-path: if no term is a product or number, skip flattening/folding loop
        if !factors
            .iter()
            .any(|f| matches!(f.kind, ExprKind::Product(_) | ExprKind::Number(_)))
        {
            let mut flat = factors;
            // Sort for canonical form
            flat.sort_unstable_by(|a, b| {
                if Arc::ptr_eq(a, b) {
                    CmpOrdering::Equal
                } else {
                    expr_cmp(a, b)
                }
            });
            return Self::new(ExprKind::Product(flat));
        }

        // Flatten nested products and fold numbers
        let mut flat: Vec<Arc<Self>> = Vec::with_capacity(factors.len());
        let mut numeric_prod: f64 = 1.0;

        for f in factors {
            match &f.kind {
                ExprKind::Product(_) => {
                    // Try to unwrap to avoid cloning inner factors
                    match Arc::try_unwrap(f) {
                        Ok(expr) => {
                            if let ExprKind::Product(inner) = expr.into_kind() {
                                flat.extend(inner);
                            }
                        }
                        Err(arc) => {
                            if let ExprKind::Product(inner) = &arc.kind {
                                flat.extend(inner.iter().cloned());
                            }
                        }
                    }
                }
                ExprKind::Number(n) => {
                    if *n == 0.0 {
                        return Self::number(0.0);
                    }
                    numeric_prod *= *n;
                }
                _ => flat.push(f),
            }
        }

        // Add numeric coefficient (canonical order: numbers first)
        if (numeric_prod - 1.0).abs() > EPSILON {
            flat.push(Arc::new(Self::number(numeric_prod)));
        }

        if flat.is_empty() {
            return Self::number(1.0);
        }
        if flat.len() == 1 {
            return Arc::try_unwrap(flat.pop().expect("Vec must have exactly one element"))
                .unwrap_or_else(|arc| (*arc).clone());
        }

        // Sort for canonical form
        flat.sort_unstable_by(|a, b| {
            if Arc::ptr_eq(a, b) {
                CmpOrdering::Equal
            } else {
                expr_cmp(a, b)
            }
        });
        Self::new(ExprKind::Product(flat))
    }

    // -------------------------------------------------------------------------
    // Binary operation constructors (for legacy compatibility during migration)
    // -------------------------------------------------------------------------

    /// Create addition: a + b → Sum([a, b])
    #[must_use]
    pub fn add_expr(left: Self, right: Self) -> Self {
        // Optimization: 0 + x = x, x + 0 = x
        if left.is_zero_num() {
            return right;
        }
        if right.is_zero_num() {
            return left;
        }
        // Optimization: Constant folding n + m = (n + m)
        if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
            return Self::number(l + r);
        }
        Self::sum(vec![left, right])
    }

    /// Create subtraction: a - b → Sum([a, Product([-1, b])])
    ///
    /// Inline optimization: If both operands are numbers, computes the result directly.
    #[must_use]
    pub fn sub_expr(left: Self, right: Self) -> Self {
        // Inline constant folding: n - m = (n - m)
        if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
            return Self::number(l - r);
        }
        // 0 - x = -x
        if left.is_zero_num() {
            return right.negate();
        }
        // x - 0 = x
        if right.is_zero_num() {
            return left;
        }
        let neg_right = Self::product(vec![Self::number(-1.0), right]);
        Self::sum(vec![left, neg_right])
    }

    /// Create multiplication: a * b → Product([a, b])
    ///
    /// Inline optimizations:
    /// - `0 * x = 0`, `x * 0 = 0`
    /// - `1 * x = x`, `x * 1 = x`
    /// - `n * m = (n * m)` constant folding
    /// - Poly * Poly fast path when bases match
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
        // Inline constant folding: n * m = (n * m)
        if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
            return Self::number(l * r);
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
    ///
    /// Inline optimizations:
    /// - `x / x = 1` (when x equals x structurally)
    /// - `m / n = result` when both are numbers and m % n == 0 (exact integer division)
    /// - `x / 1 = x`
    /// - `0 / x = 0` (when x is not zero)
    ///
    /// Note: Preserves exact fractions like `3/2` (only folds when remainder is 0).
    #[must_use]
    pub fn div_expr(left: Self, right: Self) -> Self {
        // x / x = 1 (structural equality check)
        if left == right && !left.is_zero_num() {
            return Self::number(1.0);
        }
        // m / n = result when both are numbers and m % n == 0
        if let Some(m) = left.as_number()
            && let Some(n) = right.as_number()
            && n != 0.0
            && m % n == 0.0
        {
            return Self::number(m / n);
        }
        // x / 1 = x
        if right.is_one_num() {
            return left;
        }
        // 0 / x = 0
        if left.is_zero_num() && !right.is_zero_num() {
            return Self::number(0.0);
        }
        Self::new(ExprKind::Div(Arc::new(left), Arc::new(right)))
    }

    /// Create division from Arc operands (avoids cloning if Arc ref count is 1)
    ///
    /// Inline optimizations:
    /// - `x / x = 1` (when x equals x structurally)
    /// - `x / 1 = x`
    /// - `0 / x = 0`
    #[must_use]
    pub fn div_from_arcs(left: Arc<Self>, right: Arc<Self>) -> Self {
        // x / x = 1 (structural equality check via hash, then full comparison)
        if left.hash == right.hash && *left == *right && !left.is_zero_num() {
            return Self::number(1.0);
        }
        // x / 1 = x
        if right.is_one_num() {
            return Arc::try_unwrap(left).unwrap_or_else(|arc| (*arc).clone());
        }
        // 0 / x = 0
        if left.is_zero_num() && !right.is_zero_num() {
            return Self::number(0.0);
        }
        Self::new(ExprKind::Div(left, right))
    }

    /// Create power expression (static constructor form)
    ///
    /// For the method form `expr.pow(exp)`, see the `pow` method on Expr.
    ///
    /// Inline optimizations:
    /// - `x^0 → 1`
    /// - `x^1 → x`
    /// - `1^x → 1`
    /// - `0^n → 0` (for positive n)
    /// - `n^m → result` only when both are integers and exponent is positive
    ///
    /// Note: Does NOT fold non-integer results like `2^0.5` to preserve `sqrt(2)`.
    #[must_use]
    pub fn pow_static(base: Self, exponent: Self) -> Self {
        // x^0 = 1
        if exponent.is_zero_num() {
            return Self::number(1.0);
        }
        // x^1 = x
        if exponent.is_one_num() {
            return base;
        }
        // 1^x = 1
        if base.is_one_num() {
            return Self::number(1.0);
        }
        // 0^n = 0 (for positive n)
        if base.is_zero_num()
            && let Some(n) = exponent.as_number()
            && n > 0.0
        {
            return Self::number(0.0);
        }
        // Constant folding: n^m only when both are integers and exponent >= 1
        // This preserves sqrt(2), cbrt(3), etc. as symbolic
        if let (Some(b), Some(e)) = (base.as_number(), exponent.as_number()) {
            // Only fold if exponent is a positive integer
            if e >= 1.0 && e.fract().abs() < EPSILON {
                let result = b.powf(e);
                // Only fold if result is also an integer (or very close)
                if result.fract().abs() < EPSILON {
                    return Self::number(result.round());
                }
            }
        }
        Self::new(ExprKind::Pow(Arc::new(base), Arc::new(exponent)))
    }

    /// Create power from Arc operands (avoids cloning if Arc ref count is 1)
    ///
    /// Inline optimizations:
    /// - `x^0 → 1`
    /// - `x^1 → x`
    /// - `1^x → 1`
    /// - `0^n → 0` (for positive n)
    #[must_use]
    pub fn pow_from_arcs(base: Arc<Self>, exponent: Arc<Self>) -> Self {
        // x^0 = 1
        if exponent.is_zero_num() {
            return Self::number(1.0);
        }
        // x^1 = x
        if exponent.is_one_num() {
            return Arc::try_unwrap(base).unwrap_or_else(|arc| (*arc).clone());
        }
        // 1^x = 1
        if base.is_one_num() {
            return Self::number(1.0);
        }
        // 0^n = 0 (for positive n)
        if base.is_zero_num()
            && let Some(n) = exponent.as_number()
            && n > 0.0
        {
            return Self::number(0.0);
        }
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

/// Finalize a sum expression from a flattened list of terms
fn finalize_sum(mut flat: Vec<Arc<Expr>>) -> Expr {
    let len = flat.len();
    if len == 2 {
        // Use canonical comparison for binary case
        let cmp = expr_cmp(&flat[0], &flat[1]);
        if cmp == CmpOrdering::Greater {
            flat.swap(0, 1);
        }

        // Check for poly merge (e.g. x + x, x + x^2)
        let h1 = get_poly_base_hash(&flat[0]);
        let h2 = get_poly_base_hash(&flat[1]);
        if h1.is_some() && h1 == h2 && h1 != Some(0) {
            let mut poly = crate::core::poly::Polynomial::try_from_expr(&flat[0])
                .expect("base hash match should guarantee poly compatibility");
            let next_poly = crate::core::poly::Polynomial::try_from_expr(&flat[1])
                .expect("base hash match should guarantee poly compatibility");
            poly.try_add_assign(&next_poly);
            if poly.is_zero() {
                return Expr::number(0.0);
            }
            return Expr::poly(poly);
        }

        return Expr::new(ExprKind::Sum(flat));
    }

    // Sort for canonical ordering
    flat.sort_unstable_by(|a, b| {
        if Arc::ptr_eq(a, b) {
            CmpOrdering::Equal
        } else {
            expr_cmp(a, b)
        }
    });

    let mut result: Vec<Arc<Expr>> = Vec::with_capacity(flat.len());
    let mut it = flat.into_iter().peekable();

    while let Some(term) = it.next() {
        let h = get_poly_base_hash(&term);

        if let Some(bh) = h
            && bh != 0
            && it
                .peek()
                .is_some_and(|next| get_poly_base_hash(next) == Some(bh))
        {
            // Group found! Merge into a Polynomial
            let mut poly = crate::core::poly::Polynomial::try_from_expr(&term)
                .expect("Poly conversion failed");
            while it
                .peek()
                .is_some_and(|next| get_poly_base_hash(next) == Some(bh))
            {
                let next_term = it.next().expect("Iterator peeked successfully");
                poly.try_add_assign(
                    &crate::core::poly::Polynomial::try_from_expr(&next_term)
                        .expect("Poly conversion failed"),
                );
            }
            if !poly.is_zero() {
                result.push(Arc::new(Expr::poly(poly)));
            }
            continue;
        }
        result.push(term);
    }

    if result.is_empty() {
        return Expr::number(0.0);
    }
    if result.len() == 1 {
        return Arc::try_unwrap(result.pop().expect("result cannot be empty"))
            .unwrap_or_else(|arc| (*arc).clone());
    }

    Expr::new(ExprKind::Sum(result))
}

/// Extract the polynomial base structural hash from a term.
/// For polynomial-like terms (x, x^2, 3*x^3, sin(x)^2, 2*cos(x)), returns a hash
/// that identifies the base expression so adjacent terms with the same base can be merged.
/// Returns None if term is not polynomial-like.
fn get_poly_base_hash(expr: &Expr) -> Option<u64> {
    match &expr.kind {
        // Symbol x or FunctionCall like sin(x) → use its structural hash
        ExprKind::Symbol(_) | ExprKind::FunctionCall { .. } => Some(expr.hash),

        // Already a Poly -> use its base's hash
        ExprKind::Poly(p) => Some(p.base().hash),

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
