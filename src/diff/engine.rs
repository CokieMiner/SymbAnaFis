//! Reusable, identity-bound differentiation of logical expression forests.

#![expect(
    clippy::self_named_module_files,
    reason = "the differentiation workspace owns its arithmetic and function implementations"
)]

use alloc::vec::Vec;

use crate::{
    collections::HashMap,
    core::{Expr, ExprView, Symbol},
};

/// A workspace belongs to one borrowed symbol identity. Completed partials are
/// reused across output roots, including shared logical polynomial monomials.
pub struct Differentiation<'variable> {
    variable: Symbol<'variable>,
    values: HashMap<Expr, Partial>,
    pending: Vec<(Expr, bool)>,
    scratch: Vec<Expr>,
}

enum Partial {
    Evaluated(Expr),
    Retained(Expr),
}

impl<'variable> Differentiation<'variable> {
    pub fn new(variable: Symbol<'variable>) -> Self {
        Self {
            variable,
            values: HashMap::new(),
            pending: Vec::new(),
            scratch: Vec::new(),
        }
    }

    /// Complete original logical children before their parents, without revisiting
    /// previously completed subtrees. An unsupported root returns `None`.
    pub fn evaluate(
        &mut self,
        expression: &Expr,
        unresolved: impl Fn(&Expr) -> Expr,
    ) -> Option<Expr> {
        self.pending.push((expression.clone(), false));
        while let Some((node, ready)) = self.pending.pop() {
            if let Some(partial) = self.values.get(&node) {
                if node == *expression {
                    return match partial {
                        Partial::Evaluated(value) => Some(value.clone()),
                        Partial::Retained(_) => None,
                    };
                }
                continue;
            }
            if !ready {
                self.pending.push((node.clone(), true));
                self.pending
                    .extend(node.children().map(|child| (child, false)));
                continue;
            }
            let partial = self
                .reduce(&node)
                .map_or_else(|| Partial::Retained(unresolved(&node)), Partial::Evaluated);
            let root = node == *expression;
            let result = if root {
                match &partial {
                    Partial::Evaluated(value) => Some(value.clone()),
                    Partial::Retained(_) => None,
                }
            } else {
                None
            };
            self.values.insert(node, partial);
            if root {
                return result;
            }
        }
        None
    }

    fn reduce(&mut self, node: &Expr) -> Option<Expr> {
        let get = |child: &Expr| {
            // Enter/finish frames on an immutable acyclic DAG complete every
            // logical child first. Owning keys retain projected monomials.
            match self
                .values
                .get(child)
                .expect("logical children precede their differentiation parent")
            {
                Partial::Evaluated(value) | Partial::Retained(value) => value.clone(),
            }
        };
        match node.view() {
            ExprView::Number(value) => value.is_finite().then(|| Expr::number(0)),
            ExprView::ImaginaryUnit | ExprView::Pi | ExprView::Euler => Some(Expr::number(0)),
            ExprView::Symbol(symbol) => Some(Expr::number(i32::from(symbol == self.variable))),
            ExprView::Sum(terms) => Some(Expr::sum(
                terms
                    .map(|term| get(&term))
                    .filter(|term| !term.is_zero_num())
                    .collect(),
            )),
            ExprView::Product(factors) => {
                self.scratch.clear();
                self.scratch.extend(factors.iter().map(get));
                Some(Self::product(factors, &self.scratch))
            }
            ExprView::Div(numerator, denominator) => Some(Self::quotient(
                numerator,
                denominator,
                get(numerator),
                get(denominator),
            )),
            ExprView::Pow(base, exponent) => Self::power(base, exponent, get(base), get(exponent)),
            ExprView::Function { name, args } => {
                self.scratch.clear();
                self.scratch.extend(args.iter().map(get));
                Self::function(name, args, &self.scratch)
            }
            ExprView::Derivative { inner, .. } => get(inner).is_zero_num().then(|| Expr::number(0)),
        }
    }
}

mod arithmetic;
mod elementary;
mod functions;
mod special;
