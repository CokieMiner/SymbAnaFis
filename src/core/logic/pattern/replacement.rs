//! Validated destination compilation and canonical expression construction.

use core::{ops::Div, ptr::from_ref};

use alloc::{sync::Arc, vec::Vec};

use crate::collections::{HashMap, HashSet};

use super::{
    Expr, FunctionDefinition, Pattern, PatternNode, Replacement, ReplacementOp, ReplacementProgram,
    RewriteError,
};

impl From<Pattern> for Replacement {
    fn from(pattern: Pattern) -> Self {
        Self::Pattern(pattern)
    }
}

impl ReplacementProgram {
    /// Compile template captures once; callbacks remain dormant until acceptance.
    pub fn compile(
        replacement: Replacement,
        captures: &HashSet<&str>,
    ) -> Result<Self, RewriteError> {
        let root = match replacement {
            Replacement::Pattern(pattern) => pattern,
            Replacement::Callback(callback) => return Ok(Self::Callback(callback)),
        };
        let mut instructions = Vec::new();
        let mut indices = HashMap::new();
        for template in PatternNode::template_order(&root) {
            let index = |child: &Pattern| {
                *indices
                    .get(&from_ref(child.node()))
                    .expect("compiled children precede their parent")
            };
            let operation = match template.node() {
                PatternNode::Capture { name, .. } if !captures.contains(name.as_ref()) => {
                    return Err(RewriteError::UnboundCapture(name.as_ref().into()));
                }
                PatternNode::Capture { name, .. } => ReplacementOp::Capture(Arc::clone(name)),
                PatternNode::Exact(value) => ReplacementOp::Exact(value.clone()),
                PatternNode::Function { name, args } => ReplacementOp::Function {
                    name: name.clone(),
                    args: args.iter().map(index).collect(),
                },
                PatternNode::Sum(args) => ReplacementOp::Sum(args.iter().map(index).collect()),
                PatternNode::Product(args) => {
                    ReplacementOp::Product(args.iter().map(index).collect())
                }
                PatternNode::Div(left, right) => ReplacementOp::Div(index(left), index(right)),
                PatternNode::Pow(base, exponent) => {
                    ReplacementOp::Pow(index(base), index(exponent))
                }
            };
            indices.insert(from_ref(template.node()), instructions.len());
            instructions.push(operation);
        }
        Ok(Self::Template(instructions))
    }

    /// Execute only accepted assignments; constructors normalize every result.
    pub fn instantiate(&self, captured: &HashMap<&str, Expr>) -> Expr {
        let replacement = match self {
            Self::Template(instructions) => instructions,
            Self::Callback(callback) | Self::ConcreteCallback { callback, .. } => {
                return callback(captured);
            }
            Self::Concrete(entry) => return entry.to().clone(),
        };
        let mut values: Vec<Expr> = Vec::with_capacity(replacement.len());
        for operation in replacement {
            let mapped = |index: &usize| {
                values
                    .get(*index)
                    .expect("compiled template operands precede their operation")
                    .clone()
            };
            let expression = match operation {
                ReplacementOp::Capture(name) => captured
                    .get(name.as_ref())
                    .expect("validated captures are bound by a successful match")
                    .clone(),
                ReplacementOp::Exact(value) => value.clone(),
                ReplacementOp::Function { name, args } => {
                    FunctionDefinition::construct(name.clone(), args.iter().map(mapped).collect())
                }
                ReplacementOp::Sum(args) => Expr::sum(args.iter().map(mapped).collect()),
                ReplacementOp::Product(args) => Expr::product(args.iter().map(mapped).collect()),
                ReplacementOp::Div(left, right) => Div::div(mapped(left), mapped(right)),
                ReplacementOp::Pow(base, exponent) => mapped(base).pow(mapped(exponent)),
            };
            values.push(expression);
        }
        values
            .pop()
            .expect("compiled replacement includes its root")
    }
}
