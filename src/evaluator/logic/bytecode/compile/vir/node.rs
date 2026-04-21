//! Compiler node types and core constant folding logic.
//!
//! [`NodeData`] and [`NodeValue`] track per-node information (virtual register,
//! constant value if known) during the iterative compilation walk.

use super::registry::CONST_FOLD_MAP;
use super::types::VReg;
use crate::core::known_symbols::KS;
use crate::core::known_symbols::get_constant_value_by_id;
use crate::core::{Expr, ExprKind};
use rustc_hash::FxHashMap;
use std::ptr::from_ref;

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
}

impl NodeData {
    pub const fn runtime(vreg: VReg) -> Self {
        Self {
            value: NodeValue::Runtime(vreg),
        }
    }

    pub const fn constant(vreg: VReg, value: f64) -> Self {
        Self {
            value: NodeValue::Constant { vreg, value },
        }
    }

    pub const fn vreg(self) -> VReg {
        self.value.vreg()
    }

    pub const fn const_val(self) -> Option<f64> {
        self.value.const_val()
    }
}

/// Look up a constant value for an expression if it exists in the node map.
#[inline]
pub fn const_from_map(node_map: &FxHashMap<*const Expr, NodeData>, expr: &Expr) -> Option<f64> {
    node_map
        .get(&from_ref(expr))
        .and_then(|data| data.const_val())
}

/// Returns true if the expression node should be considered for Common Subexpression Elimination.
pub const fn compute_is_cse_candidate(
    expr: &Expr,
    _node_map: &FxHashMap<*const Expr, NodeData>,
) -> bool {
    // Any node that maps to an instruction is considered eligible for CSE.
    // Symbols and Numbers simply map to Param/Const virtual registers without emitting instructions.
    !matches!(expr.kind, ExprKind::Number(_) | ExprKind::Symbol(_))
}

/// Recursively computes the constant value of an expression based on its children's known values.
pub fn compute_const_from_children(
    expr: &Expr,
    node_map: &FxHashMap<*const Expr, NodeData>,
) -> Option<f64> {
    match &expr.kind {
        ExprKind::Number(n) => Some(*n),
        ExprKind::Symbol(s) => get_constant_value_by_id(s.id()),
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
                product *= const_from_map(node_map, (*f).as_ref())?;
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
