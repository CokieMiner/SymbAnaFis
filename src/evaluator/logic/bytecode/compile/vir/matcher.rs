//! Pattern-matching helpers for AST-based instruction fusion.
//!
//! These functions inspect the AST to identify opportunities for specialized
//! instructions (e.g. FMA, Sinc, `ExpSqr`).

use super::node::{NodeData, const_from_map};
use super::types::VReg;
use crate::EPSILON;
use crate::core::known_symbols::KS;
use crate::core::{Expr, ExprKind};
use rustc_hash::FxHashMap;
use std::ptr::from_ref;
use std::sync::Arc;

/// Returns true if the expression is a constant approximately equal to -1.0.
#[inline]
pub fn is_const_neg_one(node_map: &FxHashMap<*const Expr, NodeData>, expr: &Expr) -> bool {
    const_from_map(node_map, expr).is_some_and(|n| (n + 1.0).abs() < EPSILON)
}

/// Returns true if the expression is a constant approximately equal to 2.0.
#[inline]
pub fn is_const_two(node_map: &FxHashMap<*const Expr, NodeData>, expr: &Expr) -> bool {
    const_from_map(node_map, expr).is_some_and(|n| (n - 2.0).abs() < EPSILON)
}

/// Identifies a product of two virtual registers: `a * b`.
pub fn product_two_vregs(
    term: &Expr,
    node_map: &FxHashMap<*const Expr, NodeData>,
) -> Option<(VReg, VReg)> {
    if let ExprKind::Product(factors) = &term.kind
        && factors.len() == 2
    {
        let d0 = node_map.get(&Arc::as_ptr(&factors[0]))?;
        let d1 = node_map.get(&Arc::as_ptr(&factors[1]))?;

        if is_const_neg_one(node_map, factors[0].as_ref())
            || is_const_neg_one(node_map, factors[1].as_ref())
        {
            return None;
        }

        return Some((d0.vreg(), d1.vreg()));
    }
    None
}

/// Identifies a negated product of two virtual registers: `-1 * a * b`.
pub fn negated_product_two_vregs(
    term: &Expr,
    node_map: &FxHashMap<*const Expr, NodeData>,
) -> Option<(VReg, VReg)> {
    let ExprKind::Product(factors) = &term.kind else {
        return None;
    };
    if factors.len() != 3 {
        return None;
    }

    let d0 = node_map.get(&Arc::as_ptr(&factors[0]))?;
    let d1 = node_map.get(&Arc::as_ptr(&factors[1]))?;
    let d2 = node_map.get(&Arc::as_ptr(&factors[2]))?;

    let mut neg_idx = None;
    let data = [d0, d1, d2];
    for (i, _d) in data.iter().enumerate() {
        if is_const_neg_one(node_map, factors[i].as_ref()) {
            if neg_idx.is_some() {
                return None;
            }
            neg_idx = Some(i);
        }
    }

    let neg_idx = neg_idx?;
    let mut iter = data
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != neg_idx)
        .map(|(_, d)| d.vreg());
    let a = iter.next()?;
    let b = iter.next()?;
    Some((a, b))
}

/// Checks if an expression is a call to `exp(arg)`.
#[inline]
pub fn exp_call_arg(expr: &Expr) -> Option<&Expr> {
    if let ExprKind::FunctionCall { name, args } = &expr.kind
        && name.id() == KS.exp
        && args.len() == 1
    {
        return Some(args[0].as_ref());
    }
    None
}

/// Checks if an expression is `base^2`.
#[inline]
pub fn pow2_base<'expr>(
    expr: &'expr Expr,
    node_map: &FxHashMap<*const Expr, NodeData>,
) -> Option<&'expr Expr> {
    if let ExprKind::Pow(base, exp) = &expr.kind
        && is_const_two(node_map, exp.as_ref())
    {
        return Some(base.as_ref());
    }
    None
}

/// Identifies the argument in `1 / (exp(x) - 1)`.
#[inline]
pub fn recip_expm1_arg<'expr>(
    den: &'expr Expr,
    node_map: &FxHashMap<*const Expr, NodeData>,
) -> Option<&'expr Expr> {
    let ExprKind::Sum(terms) = &den.kind else {
        return None;
    };
    if terms.len() != 2 {
        return None;
    }
    let a = terms[0].as_ref();
    let b = terms[1].as_ref();

    if let Some(arg) = exp_call_arg(a)
        && is_const_neg_one(node_map, b)
    {
        return Some(arg);
    }
    if let Some(arg) = exp_call_arg(b)
        && is_const_neg_one(node_map, a)
    {
        return Some(arg);
    }
    None
}

/// Identifies the argument and negativity in `exp(x^2)` or `exp(-x^2)`.
#[inline]
pub fn exp_sqr_arg(
    arg: &Expr,
    node_map: &FxHashMap<*const Expr, NodeData>,
) -> Option<(VReg, bool)> {
    if let Some(base) = pow2_base(arg, node_map) {
        let base_v = node_map.get(&from_ref(base)).map(|data| data.vreg())?;
        return Some((base_v, false));
    }

    if let ExprKind::Product(factors) = &arg.kind
        && factors.len() == 2
    {
        let (_neg_idx, other_idx) = if is_const_neg_one(node_map, factors[0].as_ref()) {
            (0, 1)
        } else if is_const_neg_one(node_map, factors[1].as_ref()) {
            (1, 0)
        } else {
            return None;
        };
        if let Some(base) = pow2_base(factors[other_idx].as_ref(), node_map) {
            let base_v = node_map.get(&from_ref(base)).map(|data| data.vreg())?;
            return Some((base_v, true));
        }
    }

    None
}
