//! Compiler node types and expression-tree pattern helpers.
//!
//! [`NodeData`] and [`NodeValue`] track per-node information (virtual register,
//! constant value if known, expensiveness) during the iterative compilation walk.
//! The free functions are pattern-matching helpers that inspect the AST to
//! identify instruction-fusion opportunities.

use super::registry::CONST_FOLD_MAP;
use super::vir::VReg;
use crate::core::known_symbols::KS;
use crate::EPSILON;
use crate::{Expr, core::ExprKind};
use rustc_hash::FxHashMap;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub enum NodeValue {
    Runtime(VReg),
    Constant { vreg: VReg, value: f64 },
}

impl NodeValue {
    pub const fn vreg(self) -> VReg {
        match self {
            Self::Runtime(vreg) | Self::Constant { vreg, .. } => vreg,
        }
    }

    pub const fn const_val(self) -> Option<f64> {
        match self {
            Self::Runtime(..) => None,
            Self::Constant { value, .. } => Some(value),
        }
    }
}

#[derive(Clone, Copy)]
pub struct NodeData {
    pub value: NodeValue,
    pub is_expensive: bool,
}

impl NodeData {
    pub const fn runtime(vreg: VReg, is_expensive: bool) -> Self {
        Self {
            value: NodeValue::Runtime(vreg),
            is_expensive,
        }
    }

    pub const fn constant(vreg: VReg, value: f64, is_expensive: bool) -> Self {
        Self {
            value: NodeValue::Constant { vreg, value },
            is_expensive,
        }
    }

    pub const fn vreg(self) -> VReg {
        self.value.vreg()
    }

    pub const fn const_val(self) -> Option<f64> {
        self.value.const_val()
    }
}

pub fn const_from_map(node_map: &FxHashMap<*const Expr, NodeData>, expr: &Expr) -> Option<f64> {
    node_map
        .get(&std::ptr::from_ref(expr))
        .and_then(|data| data.const_val())
}

pub fn compute_expensive_from_children(
    expr: &Expr,
    node_map: &FxHashMap<*const Expr, NodeData>,
) -> bool {
    match &expr.kind {
        ExprKind::FunctionCall { .. } | ExprKind::Div(..) | ExprKind::Poly(_) => true,
        ExprKind::Pow(base, exp) => {
            if const_from_map(node_map, exp.as_ref())
                .is_none_or(|n| (n - n.round()).abs() > EPSILON)
            {
                true
            } else {
                !matches!(base.kind, ExprKind::Number(_))
            }
        }
        ExprKind::Sum(terms) | ExprKind::Product(terms) => match terms.len().cmp(&2) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Equal => terms.iter().any(|t| {
                if node_map
                    .get(&Arc::as_ptr(t))
                    .is_some_and(|data| data.is_expensive)
                {
                    return true;
                }
                !matches!(t.kind, ExprKind::Number(_) | ExprKind::Symbol(_))
            }),
            std::cmp::Ordering::Less => terms.iter().any(|t| {
                node_map
                    .get(&Arc::as_ptr(t))
                    .is_some_and(|data| data.is_expensive)
            }),
        },
        _ => false,
    }
}

pub fn compute_const_from_children(
    expr: &Expr,
    node_map: &FxHashMap<*const Expr, NodeData>,
) -> Option<f64> {
    match &expr.kind {
        ExprKind::Number(n) => Some(*n),
        ExprKind::Symbol(s) => crate::core::known_symbols::get_constant_value_by_id(s.id()),
        ExprKind::Sum(terms) => {
            let mut sum = 0.0;
            for t in terms {
                sum += const_from_map(node_map, t.as_ref())?;
            }
            Some(sum)
        }
        ExprKind::Product(factors) => {
            let mut product = 1.0;
            for f in factors {
                product *= const_from_map(node_map, f.as_ref())?;
            }
            Some(product)
        }
        ExprKind::Div(num, den) => {
            Some(const_from_map(node_map, num.as_ref())? / const_from_map(node_map, den.as_ref())?)
        }
        ExprKind::Pow(base, exp) => Some(
            const_from_map(node_map, base.as_ref())?.powf(const_from_map(node_map, exp.as_ref())?),
        ),
        ExprKind::FunctionCall { name, args } => match args.len() {
            1 => {
                let x = const_from_map(node_map, args[0].as_ref())?;
                CONST_FOLD_MAP.get(&name.id()).map(|f| f(x))
            }
            2 => {
                let a = const_from_map(node_map, args[0].as_ref())?;
                let b = const_from_map(node_map, args[1].as_ref())?;
                let id = name.id();
                let ks = &*KS;
                if id == ks.atan2 {
                    Some(a.atan2(b))
                } else if id == ks.log {
                    Some(b.log(a))
                } else {
                    None
                }
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn product_two_vregs(
    term: &Expr,
    node_map: &FxHashMap<*const Expr, NodeData>,
) -> Option<(VReg, VReg)> {
    if let ExprKind::Product(factors) = &term.kind
        && factors.len() == 2
    {
        if factors.iter().any(|f| {
            const_from_map(node_map, f.as_ref()).is_some_and(|n| (n + 1.0).abs() < EPSILON)
        }) {
            return None;
        }
        let a = node_map
            .get(&Arc::as_ptr(&factors[0]))
            .map(|data| data.vreg())?;
        let b = node_map
            .get(&Arc::as_ptr(&factors[1]))
            .map(|data| data.vreg())?;
        return Some((a, b));
    }
    None
}

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
    let mut neg_idx = None;
    for (i, f) in factors.iter().enumerate() {
        if const_from_map(node_map, f.as_ref()).is_some_and(|n| (n + 1.0).abs() < EPSILON) {
            if neg_idx.is_some() {
                return None;
            }
            neg_idx = Some(i);
        }
    }
    let neg_idx = neg_idx?;
    let mut iter = factors
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != neg_idx)
        .map(|(_, f)| f);
    let a = node_map
        .get(&Arc::as_ptr(iter.next()?))
        .map(|data| data.vreg())?;
    let b = node_map
        .get(&Arc::as_ptr(iter.next()?))
        .map(|data| data.vreg())?;
    Some((a, b))
}

pub fn exp_call_arg(expr: &Expr) -> Option<&Expr> {
    if let ExprKind::FunctionCall { name, args } = &expr.kind
        && name.id() == KS.exp
        && args.len() == 1
    {
        return Some(args[0].as_ref());
    }
    None
}

pub fn pow2_base<'expr>(
    expr: &'expr Expr,
    node_map: &FxHashMap<*const Expr, NodeData>,
) -> Option<&'expr Expr> {
    if let ExprKind::Pow(base, exp) = &expr.kind
        && const_from_map(node_map, exp.as_ref()).is_some_and(|n| (n - 2.0).abs() < EPSILON)
    {
        return Some(base.as_ref());
    }
    None
}

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

    let is_neg_one = |expr: &Expr| -> bool {
        const_from_map(node_map, expr).is_some_and(|n| (n + 1.0).abs() < EPSILON)
    };

    if let Some(arg) = exp_call_arg(a)
        && is_neg_one(b)
    {
        return Some(arg);
    }
    if let Some(arg) = exp_call_arg(b)
        && is_neg_one(a)
    {
        return Some(arg);
    }
    None
}

pub fn exp_sqr_arg(
    arg: &Expr,
    node_map: &FxHashMap<*const Expr, NodeData>,
) -> Option<(VReg, bool)> {
    if let Some(base) = pow2_base(arg, node_map) {
        let base_v = node_map
            .get(&std::ptr::from_ref(base))
            .map(|data| data.vreg())?;
        return Some((base_v, false));
    }

    if let ExprKind::Product(factors) = &arg.kind
        && factors.len() == 2
    {
        let (_neg_idx, other_idx) = if const_from_map(node_map, factors[0].as_ref())
            .is_some_and(|n| (n + 1.0).abs() < EPSILON)
        {
            (0, 1)
        } else if const_from_map(node_map, factors[1].as_ref())
            .is_some_and(|n| (n + 1.0).abs() < EPSILON)
        {
            (1, 0)
        } else {
            return None;
        };
        if let Some(base) = pow2_base(factors[other_idx].as_ref(), node_map) {
            let base_v = node_map
                .get(&std::ptr::from_ref(base))
                .map(|data| data.vreg())?;
            return Some((base_v, true));
        }
    }

    None
}
