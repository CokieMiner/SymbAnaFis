//! Iterative release of deep template structures.

use alloc::{sync::Arc, vec::Vec};

use super::{Pattern, PatternNode};

impl Drop for Pattern {
    fn drop(&mut self) {
        let Some(owner) = self.0.take() else {
            return;
        };
        if let Some(template) = Arc::into_inner(owner) {
            let mut pending = Vec::new();
            release(template, &mut pending);
            while let Some(child) = pending.pop() {
                if let Some(node) = Arc::into_inner(child) {
                    release(node, &mut pending);
                }
            }
        }
    }
}

fn release(node: PatternNode, pending: &mut Vec<Arc<PatternNode>>) {
    match node {
        PatternNode::Capture(_) | PatternNode::Exact(_) => {}
        PatternNode::Function { args, .. }
        | PatternNode::Sum(args)
        | PatternNode::Product(args) => {
            for child in args {
                enqueue(child, pending);
            }
        }
        PatternNode::Div(left, right) | PatternNode::Pow(left, right) => {
            enqueue(left, pending);
            enqueue(right, pending);
        }
    }
}

fn enqueue(mut child: Pattern, pending: &mut Vec<Arc<PatternNode>>) {
    if let Some(owner) = child.0.take() {
        pending.push(owner);
    }
}
