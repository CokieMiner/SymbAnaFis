//! Independent numerical evaluation through the core's approximate literal folding.

use core::convert::Infallible;

use crate::{Expr, ExprView, HashMap, Number, SimplificationSequence, Symbol};

pub fn simplify(sequence: &SimplificationSequence, expression: &Expr) -> Expr {
    sequence
        .simplify(expression)
        .expect("bounded test expressions simplify")
        .into_expression()
}

pub fn evaluate(expression: &Expr, substitutions: &HashMap<Symbol<'_>, Expr>) -> Number {
    let input = expression.substitute_symbols(substitutions);
    let result = input
        .map::<Infallible>(|node| {
            Ok(match node.view() {
                ExprView::Number(number) => Some(Expr::number(number.to_float())),
                ExprView::ImaginaryUnit => Some(Expr::number(Number::i().to_float())),
                ExprView::Pi => Some(Expr::number(Number::pi())),
                ExprView::Euler => Some(Expr::number(Number::e())),
                ExprView::Symbol(_)
                | ExprView::Function { .. }
                | ExprView::Sum(_)
                | ExprView::Product(_)
                | ExprView::Div(..)
                | ExprView::Pow(..)
                | ExprView::Derivative { .. } => None,
            })
        })
        .expect("evaluation mapping is infallible");
    result
        .as_number()
        .expect("all test operations have approximate numeric arguments")
        .clone()
}
