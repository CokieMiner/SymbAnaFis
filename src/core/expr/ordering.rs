//! Canonical ordering for expressions.
//!
//! Provides comparison functions for sorting expressions into canonical form.

use std::cmp::Ordering as CmpOrdering;

use super::{EXPR_ONE, Expr, ExprKind};

/// Compare expressions for canonical ordering.
/// Order: Numbers < Symbols (alphabetical) < Functions < Sum < Product < Div < Pow
pub fn expr_cmp(a: &Expr, b: &Expr) -> CmpOrdering {
    use ExprKind::{Number, Pow, Product};

    // Helper: Extract sort key (Base, Exponent, Coefficient)
    // Returns: (Base, Exponent, Coefficient, IsAtomic)
    // Note: Exponent is Option<&Expr> (None means 1), Coefficient is f64
    fn extract_key(e: &Expr) -> (&Expr, Option<&Expr>, f64, bool) {
        match &e.kind {
            // Case: x^2 -> Base x, Exp 2, Coeff 1
            Pow(b, exp) => (b.as_ref(), Some(exp.as_ref()), 1.0, false),

            // Case: 2*x -> Base x, Exp 1, Coeff 2 (Only if Product starts with Number)
            Product(factors) if factors.len() == 2 => {
                if let Number(n) = &factors[0].kind {
                    (&factors[1], None, *n, false)
                } else {
                    (e, None, 1.0, true)
                }
            }
            // Case: x -> Base x, Exp 1, Coeff 1
            _ => (e, None, 1.0, true),
        }
    }

    // 1. Numbers always come first
    if let (Number(x), Number(y)) = (&a.kind, &b.kind) {
        return x.partial_cmp(y).unwrap_or(CmpOrdering::Equal);
    }
    if matches!(a.kind, Number(_)) {
        return CmpOrdering::Less;
    }
    if matches!(b.kind, Number(_)) {
        return CmpOrdering::Greater;
    }

    let (base_a, exp_a, coeff_a, atomic_a) = extract_key(a);
    let (base_b, exp_b, coeff_b, atomic_b) = extract_key(b);

    // 2. If both are atomic (e.g., Symbol vs Symbol), use strict type sorting fallback
    // This prevents infinite recursion (comparing x vs x)
    if atomic_a && atomic_b {
        return expr_cmp_type_strict(a, b);
    }

    // 3. Compare Bases (Recursively)
    // Recursion is safe because at least one is composite (smaller depth)
    let base_cmp = expr_cmp(base_a, base_b);
    if base_cmp != CmpOrdering::Equal {
        return base_cmp;
    }

    // logic: 1 vs 2 -> Less
    // logic: 1 vs 1 -> Equal
    // logic: 2 vs 1 -> Greater

    // If one has explicit exponent and one implied 1:
    // x (1) vs x^2 (2) -> 1 < 2 -> Less
    // Use statically cached EXPR_ONE to avoid allocations in this hot path
    match (exp_a, exp_b) {
        (Some(e_a), Some(e_b)) => {
            let exp_cmp = expr_cmp(e_a, e_b);
            if exp_cmp != CmpOrdering::Equal {
                return exp_cmp;
            }
        }
        (Some(e_a), None) => {
            // Compare expr e_a vs 1.0 (using static cached EXPR_ONE)
            let exp_cmp = expr_cmp(e_a, &EXPR_ONE);
            if exp_cmp != CmpOrdering::Equal {
                return exp_cmp;
            }
        }
        (None, Some(e_b)) => {
            // Compare 1.0 vs e_b (using static cached EXPR_ONE)
            let exp_cmp = expr_cmp(&EXPR_ONE, e_b);
            if exp_cmp != CmpOrdering::Equal {
                return exp_cmp;
            }
        }
        (None, None) => {} // Both 1
    }

    // 5. Compare Coefficients (1 < 2)
    // x vs 2x -> 1 < 2 -> Less -> x, 2x
    coeff_a.partial_cmp(&coeff_b).unwrap_or(CmpOrdering::Equal)
}

/// Fallback: Original strict type comparisons for atomic terms
pub fn expr_cmp_type_strict(a: &Expr, b: &Expr) -> CmpOrdering {
    use ExprKind::{Derivative, Div, FunctionCall, Pow, Product, Sum, Symbol};
    match (&a.kind, &b.kind) {
        (Symbol(x), Symbol(y)) => x.as_ref().cmp(y.as_ref()),
        (Symbol(_), _) => CmpOrdering::Less,
        (_, Symbol(_)) => CmpOrdering::Greater,

        (FunctionCall { name: n1, args: a1 }, FunctionCall { name: n2, args: a2 }) => {
            n1.cmp(n2).then_with(|| {
                for (x, y) in a1.iter().zip(a2.iter()) {
                    match expr_cmp(x, y) {
                        CmpOrdering::Equal => {}
                        other => return other,
                    }
                }
                a1.len().cmp(&a2.len())
            })
        }
        (FunctionCall { .. }, _) => CmpOrdering::Less,
        (_, FunctionCall { .. }) => CmpOrdering::Greater,

        (Sum(t1), Sum(t2)) => t1.len().cmp(&t2.len()).then_with(|| {
            for (x, y) in t1.iter().zip(t2.iter()) {
                match expr_cmp(x, y) {
                    CmpOrdering::Equal => {}
                    other => return other,
                }
            }
            CmpOrdering::Equal
        }),
        (Sum(_), _) => CmpOrdering::Less,
        (_, Sum(_)) => CmpOrdering::Greater,

        // Products are handled as atomics if they don't match the "Coeff * Rest" pattern
        (Product(f1), Product(f2)) => f1.len().cmp(&f2.len()).then_with(|| {
            for (x, y) in f1.iter().zip(f2.iter()) {
                match expr_cmp(x, y) {
                    CmpOrdering::Equal => {}
                    other => return other,
                }
            }
            CmpOrdering::Equal
        }),
        (Product(_), _) => CmpOrdering::Less,
        (_, Product(_)) => CmpOrdering::Greater,

        (Div(l1, r1), Div(l2, r2)) => expr_cmp(l1, l2).then_with(|| expr_cmp(r1, r2)),
        (Div(_, _), _) => CmpOrdering::Less,
        (_, Div(_, _)) => CmpOrdering::Greater,

        (Pow(b1, e1), Pow(b2, e2)) => expr_cmp(b1, b2).then_with(|| expr_cmp(e1, e2)),
        (Pow(_, _), _) => CmpOrdering::Less,
        (_, Pow(_, _)) => CmpOrdering::Greater,

        (
            Derivative {
                inner: i1,
                var: v1,
                order: o1,
            },
            Derivative {
                inner: i2,
                var: v2,
                order: o2,
            },
        ) => v1
            .cmp(v2)
            .then_with(|| o1.cmp(o2))
            .then_with(|| expr_cmp(i1, i2)),

        _ => CmpOrdering::Equal, // Should be covered by match arms above but safe fallback
    }
}
