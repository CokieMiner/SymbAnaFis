//! Canonical construction from validated replacement instructions.

use core::ops::Div;

use alloc::vec::Vec;

use crate::collections::HashMap;

use super::{Expr, FunctionDefinition, ReplacementOp};

impl ReplacementOp {
    pub fn instantiate(replacement: &[Self], captured: &HashMap<&str, Expr>) -> Expr {
        let mut values: Vec<Expr> = Vec::with_capacity(replacement.len());
        for operation in replacement {
            let mapped = |index: &usize| {
                values
                    .get(*index)
                    .expect("compiled template operands precede their operation")
                    .clone()
            };
            let expression = match operation {
                Self::Capture(name) => captured
                    .get(name.as_ref())
                    .expect("validated captures are bound by a successful match")
                    .clone(),
                Self::Exact(value) => value.clone(),
                Self::Function { name, args } => {
                    FunctionDefinition::construct(name.clone(), args.iter().map(mapped).collect())
                }
                Self::Sum(args) => Expr::sum(args.iter().map(mapped).collect()),
                Self::Product(args) => Expr::product(args.iter().map(mapped).collect()),
                Self::Div(left, right) => Div::div(mapped(left), mapped(right)),
                Self::Pow(base, exponent) => mapped(base).pow(mapped(exponent)),
            };
            values.push(expression);
        }
        values
            .pop()
            .expect("compiled replacement includes its root")
    }
}
