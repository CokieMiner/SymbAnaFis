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

/// Own traversal frames and active function identities across one or more roots.
///
/// Frames and the value map borrow the nodes of the traversed DAG without
/// touching their atomic reference counts; only the nodes of an expanded custom
/// body, which the input DAG does not hold, are owned.
#[derive(Debug, Default)]
pub struct Traversal<'expr> {
    active: FixedSet<Function>,
    pending: Vec<Pending<'expr>>,
}

#[derive(Debug)]
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
    /// Construct a reusable traversal state with preallocated frame capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            active: FixedSet::default(),
            pending: Vec::with_capacity(32),
        }
    }

    /// Reduce each stored node reachable from `expression` once, reusing this
    /// traversal's frame stack and `values` map across multiple roots.
    ///
    /// # Errors
    /// Returns [`EvalError`] for unevaluated derivatives, missing or recursive
    /// custom bodies, or errors returned by `reduce`.
    pub fn walk_root<Value: Clone>(
        &mut self,
        expression: &'expr Expr,
        values: &mut FixedMap<Cow<'expr, Expr>, Value>,
        mut reduce: impl FnMut(&Expr, &FixedMap<Cow<'expr, Expr>, Value>) -> Result<Value, EvalError>,
    ) -> Result<Value, EvalError> {
        self.active.clear();
        self.pending.clear();
        if let Some(existing) = values.get(expression) {
            return Ok(existing.clone());
        }
        self.pending.push(Pending::Enter(Cow::Borrowed(expression)));
        while let Some(frame) = self.pending.pop() {
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
                            self.pending.push(Pending::Finish(Cow::Borrowed(borrowed)));
                            self.pending.extend(
                                borrowed
                                    .stored_children()
                                    .filter(|child| !values.contains_key(*child))
                                    .map(|child| Pending::Enter(Cow::Borrowed(child))),
                            );
                        }
                        Cow::Owned(owned) => {
                            self.pending
                                .push(Pending::Finish(Cow::Owned(owned.clone())));
                            self.pending.extend(
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
                        if !self.active.insert(name.clone()) {
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
                        if let Some(recursive) = body_calls_active(&body, args, &self.active) {
                            return Err(EvalError::RecursiveFunction {
                                name: recursive.as_str().to_owned(),
                            });
                        }
                        let function = name.clone();
                        self.pending.push(Pending::Body {
                            call: node,
                            body: body.clone(),
                            function,
                        });
                        self.pending.push(Pending::Enter(Cow::Owned(body)));
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
                    let evaluated = values
                        .get(&body)
                        .cloned()
                        .ok_or(EvalError::InvalidProgram)?;
                    values.insert(call, evaluated);
                    self.active.remove(&function);
                }
            }
        }
        values
            .get(expression)
            .cloned()
            .ok_or(EvalError::InvalidProgram)
    }
}

/// Detect whether `body` directly invokes any function currently in `active`
/// outside the arguments passed to the expanding call, regardless of whether
/// the callee node was previously memoized in `values`.
fn body_calls_active(
    body: &Expr,
    call_args: &[Expr],
    active: &FixedSet<Function>,
) -> Option<Function> {
    let mut visited: FixedSet<&Expr> = FixedSet::default();
    let mut arg_nodes: Option<FixedSet<&Expr>> = None;
    let mut stack = vec![body];
    while let Some(node) = stack.pop() {
        if call_args.contains(node) || !visited.insert(node) {
            continue;
        }
        if let ExprView::Function { name, .. } = node.view()
            && active.contains(name)
        {
            let args_set = arg_nodes.get_or_insert_with(|| {
                let mut nodes: FixedSet<&Expr> = FixedSet::default();
                let mut arg_stack: Vec<&Expr> = call_args.iter().collect();
                while let Some(arg_node) = arg_stack.pop() {
                    if nodes.insert(arg_node) {
                        arg_stack.extend(arg_node.stored_children());
                    }
                }
                nodes
            });
            if !args_set.contains(node) {
                return Some(name.clone());
            }
            continue;
        }
        stack.extend(node.stored_children());
    }
    None
}
