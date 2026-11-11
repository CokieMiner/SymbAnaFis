//! Template arithmetic through canonical expression construction.

#![expect(
    unsafe_code,
    reason = "numbered children name earlier nodes, so their values need no bounds checks"
)]

use core::{
    convert::Infallible,
    ops::{Div, Sub},
};

use alloc::{boxed::Box, sync::Arc, vec::Vec};

use super::{
    AdditiveOperands, Arithmetic, Expr, ExprView, FunctionDefinition, Mapped, Normalization,
    Pattern, PatternNode, Postorder, TemplateVisit, UserFunction,
};

impl PatternNode {
    pub fn normalize(arithmetic: Arithmetic) -> Pattern {
        let mut state = Normalization::default();
        let sum = matches!(&arithmetic, Arithmetic::Sum(_));
        let normalized = match arithmetic {
            Arithmetic::Sum(inputs) | Arithmetic::Product(inputs) => {
                let mut flat = Vec::new();
                for input in inputs {
                    match input.node() {
                        Self::Sum(children) if sum => flat.extend(children.iter().cloned()),
                        Self::Product(children) if !sum => flat.extend(children.iter().cloned()),
                        Self::Capture { .. }
                        | Self::Exact(_)
                        | Self::Function { .. }
                        | Self::Sum(_)
                        | Self::Product(_)
                        | Self::Div(_, _)
                        | Self::Pow(_, _) => flat.push(input),
                    }
                }
                let lowered = flat.iter().map(|input| state.lower(input)).collect();
                if sum {
                    Expr::sum(lowered)
                } else {
                    Expr::product(lowered)
                }
            }
            Arithmetic::Sub(left, right) => Sub::sub(state.lower(&left), state.lower(&right)),
            Arithmetic::Div(left, right) => Div::div(state.lower(&left), state.lower(&right)),
            Arithmetic::Pow(base, exponent) => state.lower(&base).pow(state.lower(&exponent)),
        };
        state.lift(&normalized)
    }
}

impl Normalization {
    pub fn lower(&mut self, root: &Pattern) -> Expr {
        let order = PatternNode::template_order(root);
        let mut values = Vec::with_capacity(order.descendants.len());
        for visit in &order.descendants {
            // SAFETY: children name earlier descendants, and `values` holds
            // one value per earlier descendant.
            let value = unsafe { self.lower_visit(visit, &values) };
            values.push(value);
        }
        // SAFETY: the root's children are descendants, and `values` holds one
        // value per descendant.
        unsafe { self.lower_visit(&order.root, &values) }
    }

    /// Lower one numbered template node from the lowered values of its children.
    ///
    /// # Safety
    /// `values` holds a value for every child position of `visit`.
    unsafe fn lower_visit(&mut self, visit: &TemplateVisit<'_>, values: &[Expr]) -> Expr {
        let children = visit.children.iter().map(|&position| {
            // SAFETY: the caller provides a value for every child position.
            unsafe { values.get_unchecked(position) }.clone()
        });
        match visit.pattern.node() {
            PatternNode::Capture { name, predicates } => {
                if let Some((existing, conditions)) = self.captures.get_mut(name) {
                    for condition in predicates {
                        if !conditions.iter().any(|old| Arc::ptr_eq(old, condition)) {
                            conditions.push(Arc::clone(condition));
                        }
                    }
                    existing.clone()
                } else {
                    let marker =
                        FunctionDefinition::local(Arc::clone(name), UserFunction::new(0..=0));
                    self.markers.insert(marker.clone(), Arc::clone(name));
                    let expression = FunctionDefinition::construct(marker, Vec::new());
                    self.captures.insert(
                        Arc::clone(name),
                        (expression.clone(), predicates.iter().cloned().collect()),
                    );
                    expression
                }
            }
            PatternNode::Exact(expression) => expression.clone(),
            PatternNode::Function { name, .. } => {
                FunctionDefinition::construct(name.clone(), children.collect())
            }
            PatternNode::Sum(_) => Expr::sum(children.collect()),
            PatternNode::Product(_) => Expr::product(children.collect()),
            PatternNode::Div(..) => {
                // SAFETY: a binary template has two children, and the caller
                // provides their values.
                let (left, right) = unsafe {
                    (
                        values.get_unchecked(*visit.children.get_unchecked(0)),
                        values.get_unchecked(*visit.children.get_unchecked(1)),
                    )
                };
                Div::div(left.clone(), right.clone())
            }
            PatternNode::Pow(..) => {
                // SAFETY: a binary template has two children, and the caller
                // provides their values.
                let (base, exponent) = unsafe {
                    (
                        values.get_unchecked(*visit.children.get_unchecked(0)),
                        values.get_unchecked(*visit.children.get_unchecked(1)),
                    )
                };
                base.pow(exponent.clone())
            }
        }
    }

    fn lift(&self, root: &Expr) -> Pattern {
        let Ok((lifted, _)) =
            Postorder::logical(root).evaluate(|node: &Mapped<'_, '_, (Pattern, bool)>| {
                let expression = node.expression();
                let marker = if let ExprView::Function { name, args } = expression.view()
                    && args.is_empty()
                {
                    self.markers.get(name)
                } else {
                    None
                };
                let contains_marker =
                    marker.is_some() || node.values().any(|(_, captured)| *captured);
                let group = expression == root
                    && matches!(expression.view(), ExprView::Sum(_) | ExprView::Product(_));
                let template = marker.map_or_else(
                    || {
                        if !contains_marker && !group {
                            return Pattern::exact(expression.clone());
                        }
                        let children = node.values().map(|(child, _)| child.clone());
                        Pattern::from_node(match expression.view() {
                            ExprView::Function { name, .. } => PatternNode::Function {
                                name: name.clone(),
                                args: children.collect(),
                            },
                            ExprView::Sum(_) => {
                                // Ordered sum templates list operands as `additive_operands` does.
                                let mut terms: Vec<_> =
                                    expression.children().zip(children).collect();
                                terms.sort_unstable_by(|(left, _), (right, _)| {
                                    left.additive_order(right)
                                });
                                PatternNode::Sum(
                                    terms.into_iter().map(|(_, child)| child).collect(),
                                )
                            }
                            ExprView::Product(_) => PatternNode::Product(children.collect()),
                            ExprView::Div(..) => {
                                // SAFETY: a quotient has two edges.
                                let (left, right) = unsafe { (node.child(0), node.child(1)) };
                                PatternNode::Div(left.0.clone(), right.0.clone())
                            }
                            ExprView::Pow(..) => {
                                // SAFETY: a power has two edges.
                                let (base, exponent) = unsafe { (node.child(0), node.child(1)) };
                                PatternNode::Pow(base.0.clone(), exponent.0.clone())
                            }
                            ExprView::Number(_)
                            | ExprView::ImaginaryUnit
                            | ExprView::Pi
                            | ExprView::Euler
                            | ExprView::Symbol(_)
                            | ExprView::Derivative { .. } => PatternNode::Exact(expression.clone()),
                        })
                    },
                    |name| {
                        let predicates = self
                            .captures
                            .get(name)
                            .map_or_else(Box::default, |(_, conditions)| {
                                conditions.as_slice().into()
                            });
                        Pattern::from_node(PatternNode::Capture {
                            name: Arc::clone(name),
                            predicates,
                        })
                    },
                );
                Ok::<_, Infallible>((template, contains_marker))
            });
        lifted
    }
}
