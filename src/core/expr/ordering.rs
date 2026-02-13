//! Canonical ordering for expressions.
//!
//! Provides comparison functions for sorting expressions into canonical form.

use std::cmp::Ordering as CmpOrdering;

use super::{EXPR_ONE, Expr, ExprKind};

/// Compare expressions for canonical ordering.
/// Order: Numbers < Symbols (by power) < Sum < `FunctionCall` < Pow < Div
#[inline]
pub fn expr_cmp(a: &Expr, b: &Expr) -> CmpOrdering {
    // Fast identity check (pointer equality)
    if std::ptr::eq(a, b) {
        return CmpOrdering::Equal;
    }

    // Compare bases first to ensure adjacency of like terms (Phase 10 optimization)
    let base_a = get_base(a);
    let base_b = get_base(b);

    // If bases are different, use strict type comparison on bases
    if !std::ptr::eq(base_a, base_b) {
        let base_cmp = expr_cmp_type_strict(base_a, base_b);
        if base_cmp != CmpOrdering::Equal {
            return base_cmp;
        }
    }

    // Bases are equal (e.g., x vs x^2 vs 2*x)
    // Compare exponents next (e.g., x^2 > x)
    let exp_a = get_exponent(a);
    let exp_b = get_exponent(b);

    if !std::ptr::eq(exp_a, exp_b) {
        let exp_cmp = expr_cmp(exp_a, exp_b);
        if exp_cmp != CmpOrdering::Equal {
            return exp_cmp;
        }
    }

    // Exponents are equal (e.g., 2*x vs 3*x)
    // Compare coefficients last
    let coeff_a = get_coeff(a);
    let coeff_b = get_coeff(b);

    coeff_a.partial_cmp(&coeff_b).unwrap_or(CmpOrdering::Equal)
}

// Helper to get the sorting "base" of an expression.
// e.g., for `3*x^2`, the base is `x`. For `sin(t)`, the base is `sin(t)`.
#[inline]
/// Get the sorting base of an expression
fn get_base(e: &Expr) -> &Expr {
    match &e.kind {
        ExprKind::Pow(b, _) => b.as_ref(),
        ExprKind::Product(factors)
            if factors.len() == 2 && matches!(&factors[0].kind, ExprKind::Number(_)) =>
        {
            get_base(&factors[1])
        }
        ExprKind::Poly(p) => p.base(),
        _ => e,
    }
}

// Helper to get the sorting "exponent" of an expression.
// e.g., for `3*x^2`, the exponent is `2`. For `x`, it's `1`.
#[inline]
/// Get the sorting exponent of an expression
fn get_exponent(e: &Expr) -> &Expr {
    match &e.kind {
        ExprKind::Pow(_, exp) => exp.as_ref(),
        ExprKind::Product(factors)
            if factors.len() == 2 && matches!(&factors[0].kind, ExprKind::Number(_)) =>
        {
            get_exponent(&factors[1])
        }
        _ => &EXPR_ONE,
    }
}

// Helper to get the coefficient of an expression.
// e.g., for `3*x^2`, the coeff is `3.0`. For `x`, it's `1.0`.
#[inline]
/// Get the coefficient of an expression
fn get_coeff(e: &Expr) -> f64 {
    match &e.kind {
        ExprKind::Product(factors) if factors.len() == 2 => {
            if let ExprKind::Number(c) = &factors[0].kind {
                *c
            } else {
                1.0
            }
        }
        _ => 1.0,
    }
}

/// Fallback: Strict type comparisons for atomic terms
/// Order: Number < Symbol < Sum < `FunctionCall` < Pow < Div
/// Note: Product is not included because products are flattened during construction
pub fn expr_cmp_type_strict(a: &Expr, b: &Expr) -> CmpOrdering {
    use ExprKind::{Derivative, Div, FunctionCall, Number, Pow, Product, Sum, Symbol};
    match (&a.kind, &b.kind) {
        // 0. Symbols and Numbers come first (most common atomic types)
        (Number(x), Number(y)) => x.partial_cmp(y).unwrap_or(CmpOrdering::Equal),
        (Number(_), _) => CmpOrdering::Less,
        (_, Number(_)) => CmpOrdering::Greater,

        (Symbol(x), Symbol(y)) => x.cmp(y),
        (Symbol(_), _) => CmpOrdering::Less,
        (_, Symbol(_)) => CmpOrdering::Greater,

        // 1. Composite types (Sum, Product, etc.)
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

        // 3. Function calls (exp, sin, cos, etc.)
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

        // 4. Powers (complex expressions)
        (Pow(b1, e1), Pow(b2, e2)) => expr_cmp(b1, b2).then_with(|| expr_cmp(e1, e2)),
        (Pow(_, _), _) => CmpOrdering::Less,
        (_, Pow(_, _)) => CmpOrdering::Greater,

        // 5. Divisions come last (most complex)
        (Div(l1, r1), Div(l2, r2)) => expr_cmp(l1, l2).then_with(|| expr_cmp(r1, r2)),
        (Div(_, _), _) => CmpOrdering::Less,
        (_, Div(_, _)) => CmpOrdering::Greater,

        // Products shouldn't appear as factors (they're flattened), but handle gracefully
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

        // Derivatives
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

        (ExprKind::Poly(p1), ExprKind::Poly(p2)) => {
            expr_cmp(p1.base(), p2.base()).then_with(|| {
                p1.terms().len().cmp(&p2.terms().len()).then_with(|| {
                    for (t1, t2) in p1.terms().iter().zip(p2.terms().iter()) {
                        let c = t1
                            .0
                            .cmp(&t2.0)
                            .then_with(|| t1.1.partial_cmp(&t2.1).unwrap_or(CmpOrdering::Equal));
                        if c != CmpOrdering::Equal {
                            return c;
                        }
                    }
                    CmpOrdering::Equal
                })
            })
        }
        (ExprKind::Poly(_), _) => CmpOrdering::Less,
        (_, ExprKind::Poly(_)) => CmpOrdering::Greater,
    }
}
