//! Template arithmetic through canonical expression construction.

use core::{
    convert::Infallible,
    ops::{Div, Sub},
    ptr::from_ref,
};

use alloc::{sync::Arc, vec::Vec};

use super::{
    AdditiveOperands, Arithmetic, CaptureConditions, Expr, ExprView, FunctionDefinition, HashMap,
    Normalization, Pattern, PatternNode, UserFunction,
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
        let mut memo: HashMap<*const PatternNode, Expr> = HashMap::new();
        for pattern in order {
            let child = |template: &Pattern| {
                memo.get(&from_ref(template.node()))
                    .expect("template children precede their parent")
                    .clone()
            };
            let expression = match pattern.node() {
                PatternNode::Capture { name, predicates } => {
                    if let Some((existing, conditions)) = self.captures.get_mut(name) {
                        for condition in predicates.iter() {
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
                PatternNode::Function { name, args } => {
                    FunctionDefinition::construct(name.clone(), args.iter().map(child).collect())
                }
                PatternNode::Sum(args) => Expr::sum(args.iter().map(child).collect()),
                PatternNode::Product(args) => Expr::product(args.iter().map(child).collect()),
                PatternNode::Div(left, right) => Div::div(child(left), child(right)),
                PatternNode::Pow(base, exponent) => child(base).pow(child(exponent)),
            };
            memo.insert(from_ref(pattern.node()), expression);
        }
        memo.remove(&from_ref(root.node()))
            .expect("template order includes its root")
    }

    fn lift(&self, root: &Expr) -> Pattern {
        let mut memo: HashMap<Expr, (Pattern, bool)> = HashMap::new();
        let transformed = root.map(|node| {
            let child = |expression: &Expr| {
                memo.get(expression)
                    .expect("logical children precede their parent")
                    .0
                    .clone()
            };
            let marker = if let ExprView::Function { name, args } = node.view()
                && args.is_empty()
            {
                self.markers.get(name)
            } else {
                None
            };
            let contains_marker = marker.is_some()
                || node
                    .children()
                    .any(|expression| memo.get(&expression).is_some_and(|(_, captured)| *captured));
            let group =
                node == root && matches!(node.view(), ExprView::Sum(_) | ExprView::Product(_));
            let template = marker.map_or_else(
                || {
                    if !contains_marker && !group {
                        return Pattern::exact(node.clone());
                    }
                    let kind = match node.view() {
                        ExprView::Function { name, args } => PatternNode::Function {
                            name: name.clone(),
                            args: args.iter().map(child).collect(),
                        },
                        ExprView::Sum(_) => PatternNode::Sum(
                            node.additive_operands()
                                .expect("a logical sum has ordered additive operands")
                                .iter()
                                .map(child)
                                .collect(),
                        ),
                        ExprView::Product(args) => {
                            PatternNode::Product(args.iter().map(child).collect())
                        }
                        ExprView::Div(left, right) => PatternNode::Div(child(left), child(right)),
                        ExprView::Pow(base, exponent) => {
                            PatternNode::Pow(child(base), child(exponent))
                        }
                        ExprView::Number(_)
                        | ExprView::ImaginaryUnit
                        | ExprView::Pi
                        | ExprView::Euler
                        | ExprView::Symbol(_)
                        | ExprView::Derivative { .. } => PatternNode::Exact(node.clone()),
                    };
                    Pattern(Some(Arc::new(kind)))
                },
                |name| {
                    let predicates = match self
                        .captures
                        .get(name)
                        .map(|(_, conditions)| conditions.as_slice())
                    {
                        None | Some([]) => CaptureConditions::Unrestricted,
                        Some([predicate]) => CaptureConditions::One(Arc::clone(predicate)),
                        Some(conditions) => CaptureConditions::Many(conditions.to_vec()),
                    };
                    Pattern(Some(Arc::new(PatternNode::Capture {
                        name: Arc::clone(name),
                        predicates,
                    })))
                },
            );
            memo.insert(node.clone(), (template, contains_marker));
            Ok::<_, Infallible>(None)
        });
        match transformed {
            Ok(_) => {
                memo.remove(root)
                    .expect("logical traversal includes its root")
                    .0
            }
            Err(never) => match never {},
        }
    }
}
