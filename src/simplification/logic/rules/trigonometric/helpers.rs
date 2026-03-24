use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::KS;
use crate::core::symbol::InternedSymbol;

/// Extract the trig function symbol and its single argument.
pub fn get_trig_function(expr: &Expr) -> Option<(InternedSymbol, Expr)> {
    if let AstKind::FunctionCall { name, args } = &expr.kind
        && args.len() == 1
    {
        match name {
            n if n.id() == KS.sin
                || n.id() == KS.cos
                || n.id() == KS.tan
                || n.id() == KS.cot
                || n.id() == KS.sec
                || n.id() == KS.csc =>
            {
                Some((name.clone(), (*args[0]).clone()))
            }
            _ => None,
        }
    } else {
        None
    }
}
