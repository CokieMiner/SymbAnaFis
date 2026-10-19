//! Iterative destruction of expression DAG nodes.

use alloc::{sync::Arc, vec::Vec};

use super::{Expr, Interner, NODES, Node};

impl Drop for Expr {
    fn drop(&mut self) {
        let Some(owner) = self.0.take() else {
            return;
        };
        // All expression owners use into_inner: exactly one racing final owner
        // receives the value, and none falls back to a recursive Arc destructor.
        if let Some(node) = Arc::into_inner(owner) {
            let mut pending = Vec::new();
            release(node, &mut pending);
            while let Some(child) = pending.pop() {
                if let Some(child_node) = Arc::into_inner(child) {
                    release(child_node, &mut pending);
                }
            }
        }
    }
}

fn release(node: Node, pending: &mut Vec<Arc<Node>>) {
    // Its allocation has zero strong owners. Prune expired weak entries before
    // releasing children, while their canonical pointers still form the key.
    NODES.prune(Interner::<Node>::fingerprint(&node));
    match node {
        Node::Number(_) | Node::ImaginaryUnit | Node::Pi | Node::Euler | Node::Symbol(_) => {}
        Node::Function { args, .. } | Node::Sum(args) | Node::Product(args) => {
            for child in args {
                enqueue(child, pending);
            }
        }
        Node::Poly(polynomial) => {
            for term in polynomial.terms {
                enqueue(term.coefficient, pending);
                for (base, _) in term.powers {
                    enqueue(base, pending);
                }
            }
        }
        Node::Div(left, right) | Node::Pow(left, right) => {
            enqueue(left, pending);
            enqueue(right, pending);
        }
        Node::Derivative { inner, .. } => enqueue(inner, pending),
    }
}

fn enqueue(mut child: Expr, pending: &mut Vec<Arc<Node>>) {
    if let Some(owner) = child.0.take() {
        pending.push(owner);
    }
}
