//! Memoized postorder traversal with explicit custom-body expansion frames.

use alloc::{
    borrow::{Cow, ToOwned},
    vec,
    vec::Vec,
};

use crate::{
    collections::{FixedMap, FixedSet},
    core::{BuiltinFunction, Expr, ExprView, Function, StoredForm},
    errors::EvalError,
};

/// Own traversal frames and active function identities for one reduction pass.
///
/// Frames borrow the nodes of the traversed DAG; only the nodes of an expanded
/// custom body, which the DAG does not hold, are owned.
pub struct Traversal<'expr> {
    active: FixedSet<Function>,
    pending: Vec<Pending<'expr>>,
}

enum Pending<'expr> {
    Enter(Cow<'expr, Expr>),
    Finish(Cow<'expr, Expr>),
    Body {
        call: Cow<'expr, Expr>,
        body: Expr,
        function: Function,
    },
}

impl<'expr> Traversal<'expr> {
    /// Reduce each stored node once, expanding registered custom bodies iteratively.
    /// The reducer receives builtin calls and ordinary scalar nodes after their children.
    pub fn walk<Value: Clone>(
        expression: &'expr Expr,
        values: &mut FixedMap<Expr, Value>,
        mut reduce: impl FnMut(&Expr, &FixedMap<Expr, Value>) -> Result<Value, EvalError>,
    ) -> Result<Value, EvalError> {
        let mut traversal = Self {
            active: FixedSet::default(),
            pending: vec![Pending::Enter(Cow::Borrowed(expression))],
        };
        while let Some(frame) = traversal.pending.pop() {
            match frame {
                Pending::Enter(node) => {
                    if values.contains_key(&*node) {
                        continue;
                    }
                    if matches!(node.view(), ExprView::Derivative { .. }) {
                        return Err(EvalError::UnevaluatedDerivative);
                    }
                    // The node finishes after the children pushed above it;
                    // children with a value are not pushed.
                    match node {
                        Cow::Borrowed(borrowed) => {
                            traversal
                                .pending
                                .push(Pending::Finish(Cow::Borrowed(borrowed)));
                            traversal.pending.extend(
                                borrowed
                                    .stored_children()
                                    .filter(|child| !values.contains_key(*child))
                                    .map(|child| Pending::Enter(Cow::Borrowed(child))),
                            );
                        }
                        Cow::Owned(owned) => {
                            traversal
                                .pending
                                .push(Pending::Finish(Cow::Owned(owned.clone())));
                            traversal.pending.extend(
                                owned
                                    .stored_children()
                                    .filter(|child| !values.contains_key(*child))
                                    .map(|child| Pending::Enter(Cow::Owned(child.clone()))),
                            );
                        }
                    }
                }
                Pending::Finish(node) => {
                    if let ExprView::Function { name, args } = node.view()
                        && BuiltinFunction::of(name).is_none()
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
                        let function = name.clone();
                        traversal.pending.push(Pending::Body {
                            call: node,
                            body: body.clone(),
                            function,
                        });
                        traversal.pending.push(Pending::Enter(Cow::Owned(body)));
                    } else {
                        let value = reduce(&node, values)?;
                        values.insert(node.into_owned(), value);
                    }
                }
                Pending::Body {
                    call,
                    body,
                    function,
                } => {
                    values.insert(call.into_owned(), values[&body].clone());
                    traversal.active.remove(&function);
                }
            }
        }
        Ok(values[expression].clone())
    }
}
