//! Memoized postorder traversal with explicit custom-body expansion frames.

use alloc::{borrow::ToOwned, vec, vec::Vec};

use crate::{
    collections::{HashMap, HashSet},
    core::{EvaluationView, Expr, ExprView, Function, FunctionDefinition},
    errors::EvalError,
};

/// Own traversal frames and active function identities for one reduction pass.
pub struct Traversal {
    active: HashSet<Function>,
    pending: Vec<Pending>,
}

enum Pending {
    Enter(Expr),
    Finish(Expr),
    Body {
        call: Expr,
        body: Expr,
        function: Function,
    },
}

impl Traversal {
    /// Reduce each stored node once, expanding registered custom bodies iteratively.
    /// The reducer receives builtin calls and ordinary scalar nodes after their children.
    pub fn walk<Value: Clone>(
        expression: &Expr,
        values: &mut HashMap<Expr, Value>,
        mut reduce: impl FnMut(&Expr, &HashMap<Expr, Value>) -> Result<Value, EvalError>,
    ) -> Result<Value, EvalError> {
        let mut traversal = Self {
            active: HashSet::new(),
            pending: vec![Pending::Enter(expression.clone())],
        };
        while let Some(frame) = traversal.pending.pop() {
            match frame {
                Pending::Enter(node) => {
                    if values.contains_key(&node) {
                        continue;
                    }
                    if matches!(node.view(), ExprView::Derivative { .. }) {
                        return Err(EvalError::UnevaluatedDerivative);
                    }
                    traversal.pending.push(Pending::Finish(node.clone()));
                    traversal.pending.extend(
                        EvaluationView::new(&node)
                            .children()
                            .cloned()
                            .map(Pending::Enter),
                    );
                }
                Pending::Finish(node) => {
                    if let ExprView::Function { name, args } = node.view()
                        && FunctionDefinition::get(name).is_none()
                    {
                        if !traversal.active.insert(name.clone()) {
                            return Err(EvalError::RecursiveFunction {
                                name: name.as_str().to_owned(),
                            });
                        }
                        let callback = name
                            .definition()
                            .and_then(|definition| definition.body_fn())
                            .ok_or_else(|| EvalError::MissingBody {
                                name: name.as_str().to_owned(),
                            })?;
                        let body = callback(args);
                        traversal.pending.push(Pending::Body {
                            call: node.clone(),
                            body: body.clone(),
                            function: name.clone(),
                        });
                        traversal.pending.push(Pending::Enter(body));
                    } else {
                        let value = reduce(&node, values)?;
                        values.insert(node, value);
                    }
                }
                Pending::Body {
                    call,
                    body,
                    function,
                } => {
                    values.insert(call, values[&body].clone());
                    traversal.active.remove(&function);
                }
            }
        }
        Ok(values[expression].clone())
    }
}
