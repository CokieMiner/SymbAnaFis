//! N-ary expression constructors (Sum, Product) with flattening and sorting.

use std::cmp::Ordering;
use std::sync::Arc;

use super::{EPSILON, EXPR_ONE, Expr, ExprKind, Polynomial, expr_cmp};

impl Expr {
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
                    _ => {}
                }
            } else {
                flat.push(Arc::new(t));
            }
        }

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
    /// Panics if internal invariants are violated (never in normal use).
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

        if !terms
            .iter()
            .any(|t| matches!(t.kind, ExprKind::Sum(_) | ExprKind::Number(_)))
        {
            return finalize_sum(terms);
        }

        let mut flat: Vec<Arc<Self>> = Vec::with_capacity(terms.len());
        let mut numeric_sum: f64 = 0.0;

        for t in terms {
            if let ExprKind::Number(n) = t.kind {
                numeric_sum += n;
                continue;
            }

            if matches!(t.kind, ExprKind::Sum(_)) {
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

            flat.push(t);
        }

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
    ///
    /// # Panics
    /// Panics if internal invariants are violated (never in normal use).
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
    /// Panics if internal invariants are violated (never in normal use).
    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "Complex flattening and sorting logic"
    )]
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

        if factors.len() == 2
            && matches!(factors[0].kind, ExprKind::Product(_))
            && matches!(factors[1].kind, ExprKind::Product(_))
            && let (ExprKind::Product(a_factors), ExprKind::Product(b_factors)) =
                (&factors[0].kind, &factors[1].kind)
        {
            let mut merged: Vec<Arc<Self>> = Vec::with_capacity(a_factors.len() + b_factors.len());
            let mut ai = 0;
            let mut bi = 0;
            while ai < a_factors.len() && bi < b_factors.len() {
                let ord = if Arc::ptr_eq(&a_factors[ai], &b_factors[bi]) {
                    Ordering::Equal
                } else {
                    expr_cmp(&a_factors[ai], &b_factors[bi])
                };
                if ord == Ordering::Greater {
                    merged.push(Arc::clone(&b_factors[bi]));
                    bi += 1;
                } else {
                    merged.push(Arc::clone(&a_factors[ai]));
                    ai += 1;
                }
            }
            merged.extend_from_slice(&a_factors[ai..]);
            merged.extend(b_factors[bi..].iter().map(Arc::clone));
            return finalize_product(merged);
        }

        if !factors
            .iter()
            .any(|f| matches!(f.kind, ExprKind::Product(_) | ExprKind::Number(_)))
        {
            let mut flat = factors;
            if flat
                .windows(2)
                .any(|w| !Arc::ptr_eq(&w[0], &w[1]) && expr_cmp(&w[0], &w[1]) == Ordering::Greater)
            {
                flat.sort_unstable_by(|a, b| {
                    if Arc::ptr_eq(a, b) {
                        Ordering::Equal
                    } else {
                        expr_cmp(a, b)
                    }
                });
            }
            return finalize_product(flat);
        }

        let mut flat: Vec<Arc<Self>> = Vec::with_capacity(factors.len());
        let mut numeric_prod: f64 = 1.0;

        for f in factors {
            match &f.kind {
                ExprKind::Product(_) => match Arc::try_unwrap(f) {
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
                },
                ExprKind::Number(n) => {
                    if *n == 0.0 {
                        return Self::number(0.0);
                    }
                    numeric_prod *= *n;
                }
                _ => flat.push(f),
            }
        }

        if (numeric_prod - 1.0).abs() > EPSILON {
            flat.push(Arc::new(Self::number(numeric_prod)));
        }

        if flat.is_empty() {
            return Self::number(1.0);
        }
        if flat.len() == 1 {
            return Arc::try_unwrap(flat.pop().expect("Vec not empty"))
                .unwrap_or_else(|arc| (*arc).clone());
        }

        if flat
            .windows(2)
            .any(|w| !Arc::ptr_eq(&w[0], &w[1]) && expr_cmp(&w[0], &w[1]) == Ordering::Greater)
        {
            flat.sort_unstable_by(|a, b| {
                if Arc::ptr_eq(a, b) {
                    Ordering::Equal
                } else {
                    expr_cmp(a, b)
                }
            });
        }

        finalize_product(flat)
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Finalize a sum expression from a flattened list of terms
fn finalize_sum(mut flat: Vec<Arc<Expr>>) -> Expr {
    let len = flat.len();
    if len == 2 {
        let cmp = expr_cmp(&flat[0], &flat[1]);
        if cmp == Ordering::Greater {
            flat.swap(0, 1);
        }

        let h1 = get_poly_base_hash(&flat[0]);
        let h2 = get_poly_base_hash(&flat[1]);
        if h1.is_some()
            && h1 == h2
            && h1 != Some(0)
            && let (Some(mut poly), Some(next_poly)) = (
                Polynomial::try_from_expr(&flat[0]),
                Polynomial::try_from_expr(&flat[1]),
            )
            && poly.try_add_assign(&next_poly)
        {
            if poly.is_zero() {
                return Expr::number(0.0);
            }
            return Expr::poly(poly);
        }
        return Expr::new(ExprKind::Sum(flat));
    }

    if flat
        .windows(2)
        .any(|w| !Arc::ptr_eq(&w[0], &w[1]) && expr_cmp(&w[0], &w[1]) == Ordering::Greater)
    {
        flat.sort_unstable_by(|a, b| {
            if Arc::ptr_eq(a, b) {
                Ordering::Equal
            } else {
                expr_cmp(a, b)
            }
        });
    }

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
            if let Some(mut poly) = Polynomial::try_from_expr(&term) {
                let mut unmerged: Vec<Arc<Expr>> = Vec::new();
                while it
                    .peek()
                    .is_some_and(|next| get_poly_base_hash(next) == Some(bh))
                {
                    let next_term = it.next().expect("Iterator peeked successfully");
                    if let Some(next_poly) = Polynomial::try_from_expr(&next_term) {
                        if !poly.try_add_assign(&next_poly) {
                            unmerged.push(next_term);
                        }
                    } else {
                        unmerged.push(next_term);
                    }
                }
                if !poly.is_zero() {
                    result.push(Arc::new(Expr::poly(poly)));
                }
                result.extend(unmerged);
                continue;
            }
            result.push(term);
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

fn get_poly_base_hash(expr: &Expr) -> Option<u64> {
    match &expr.kind {
        ExprKind::Symbol(_) | ExprKind::FunctionCall { .. } => Some(expr.structural_hash()),
        ExprKind::Poly(p) => Some(p.base().structural_hash()),
        ExprKind::Pow(base, exp) => {
            if let ExprKind::Number(n) = &exp.kind
                && *n >= 1.0
                && n.fract().abs() < EPSILON
            {
                return Some(base.structural_hash());
            }
            None
        }
        ExprKind::Product(factors) => {
            let mut base_hash = None;
            for f in factors {
                match &f.kind {
                    ExprKind::Number(_) => {}
                    ExprKind::Symbol(_) | ExprKind::FunctionCall { .. } => {
                        if base_hash.is_some() {
                            return None;
                        }
                        base_hash = Some(f.structural_hash());
                    }
                    ExprKind::Pow(b, exp) => {
                        if let ExprKind::Number(n) = &exp.kind
                            && *n >= 1.0
                            && n.fract().abs() < EPSILON
                        {
                            if base_hash.is_some() {
                                return None;
                            }
                            base_hash = Some(b.structural_hash());
                            continue;
                        }
                        return None;
                    }
                    _ => return None,
                }
            }
            base_hash
        }
        ExprKind::Number(_) => Some(0),
        _ => None,
    }
}

fn get_factor_base_and_exponent(expr: &Arc<Expr>) -> (Arc<Expr>, Expr) {
    match &expr.kind {
        ExprKind::Pow(base, exp) => (Arc::clone(base), (**exp).clone()),
        _ => (Arc::clone(expr), EXPR_ONE.clone()),
    }
}

fn get_product_base_hash(expr: &Expr) -> Option<u64> {
    match &expr.kind {
        ExprKind::Number(_) => None,
        ExprKind::Pow(base, _) => Some(base.structural_hash()),
        _ => Some(expr.structural_hash()),
    }
}

fn finalize_product(mut flat: Vec<Arc<Expr>>) -> Expr {
    if flat.is_empty() {
        return Expr::number(1.0);
    }
    if flat.len() == 1 {
        return Arc::try_unwrap(flat.pop().expect("Vec not empty"))
            .unwrap_or_else(|arc| (*arc).clone());
    }

    let mut result: Vec<Arc<Expr>> = Vec::with_capacity(flat.len());
    let mut it = flat.into_iter().peekable();

    while let Some(factor) = it.next() {
        let bh = get_product_base_hash(&factor);

        if let Some(bh_val) = bh
            && it
                .peek()
                .is_some_and(|next| get_product_base_hash(next) == Some(bh_val))
        {
            let (base, first_exp) = get_factor_base_and_exponent(&factor);
            let mut group_exps = vec![first_exp];
            let mut unmerged: Vec<Arc<Expr>> = Vec::new();

            while it
                .peek()
                .is_some_and(|next| get_product_base_hash(next) == Some(bh_val))
            {
                let next_factor = it.next().expect("Iterator peeked successfully");
                let (next_base, next_exp) = get_factor_base_and_exponent(&next_factor);
                if *next_base == *base {
                    group_exps.push(next_exp);
                } else {
                    unmerged.push(next_factor);
                }
            }

            let total_exp = Expr::sum(group_exps);
            result.push(Arc::new(Expr::pow_static(
                Arc::try_unwrap(base).unwrap_or_else(|arc| (*arc).clone()),
                total_exp,
            )));
            result.extend(unmerged);
            continue;
        }
        result.push(factor);
    }

    if result.len() == 1 {
        return Arc::try_unwrap(result.pop().expect("result not empty"))
            .unwrap_or_else(|arc| (*arc).clone());
    }

    Expr::new(ExprKind::Product(result))
}
