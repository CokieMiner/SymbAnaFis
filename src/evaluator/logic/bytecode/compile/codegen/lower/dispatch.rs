use crate::Expr;
use crate::core::error::DiffError;
use crate::core::expr::ExprKind;
use super::super::super::vir::VReg;
use super::super::super::vir::node::NodeData;
use rustc_hash::FxHashMap;

use super::super::super::Compiler;

impl Compiler {
    pub(in crate::evaluator::logic::bytecode::compile) fn compile_nonconst_node(
        &mut self,
        expr: &Expr,
        node_map: &FxHashMap<*const Expr, NodeData>,
    ) -> Result<VReg, DiffError> {
        match &expr.kind {
            ExprKind::Number(_) => Err(DiffError::UnsupportedExpression(
                "Numerical values are unreachable here".to_owned(),
            )),
            ExprKind::Symbol(s) => self.compile_symbol_node(s),
            ExprKind::Sum(terms) => self.compile_sum_node(terms, node_map),
            ExprKind::Product(factors) => self.compile_product_node(factors, node_map),
            ExprKind::Div(num, den) => self.compile_div_node(num.as_ref(), den.as_ref(), node_map),
            ExprKind::Pow(base, exp) => {
                self.compile_pow_node(base.as_ref(), exp.as_ref(), node_map)
            }
            ExprKind::FunctionCall { name, args } => {
                self.compile_function_node(name, args, node_map)
            }
            ExprKind::Poly(poly) => self.compile_poly_node(poly, node_map),
            ExprKind::Derivative { .. } => Err(DiffError::UnsupportedExpression(
                "Derivatives cannot be numerically evaluated - simplify first".to_owned(),
            )),
        }
    }
}
