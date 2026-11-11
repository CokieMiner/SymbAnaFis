//! Children-before-parents template numbering and compiled destination instantiation.
//!
//! Discovery gives each distinct node a slot with one hash lookup per edge.
//! A node finishes after all of its children, so every child position names
//! an earlier descendant, and values computed in this order are read by
//! position during compilation and instantiation.
//!
//! Invariant used by the unchecked reads below: every slot index stored in a
//! slot or on the pending stack indexes `slots`, which only grows, and every
//! compiled operand position names an earlier instruction in `values`.

#![expect(
    unsafe_code,
    reason = "slot indices and compiled operand positions index their allocation; see the module invariant"
)]

use core::{mem::take, ops::Div, ptr::from_ref};

use alloc::{sync::Arc, vec::Vec};

use crate::collections::{Entry, FixedMap, FixedSet, HashMap};

use super::{
    Expr, FunctionDefinition, Pattern, PatternNode, Replacement, ReplacementOp, ReplacementProgram,
    RewriteError, Template, TemplateOrder, TemplateVisit,
};

struct Slot<'pattern> {
    pattern: &'pattern Pattern,
    /// The slots of its children; replaced by their positions when it finishes.
    children: Vec<usize>,
    expanded: bool,
    /// The postorder position, assigned when the node finishes.
    position: usize,
}

struct Discovery<'pattern> {
    index: FixedMap<*const PatternNode, usize>,
    slots: Vec<Slot<'pattern>>,
    pending: Vec<(usize, bool)>,
    descendants: Vec<TemplateVisit<'pattern>>,
}

impl From<Pattern> for Replacement {
    fn from(pattern: Pattern) -> Self {
        Self::Pattern(pattern)
    }
}

impl ReplacementProgram {
    /// Compile template captures once; callbacks remain dormant until acceptance.
    pub fn compile(
        replacement: Replacement,
        captures: &FixedSet<&str>,
    ) -> Result<Self, RewriteError> {
        let root = match replacement {
            Replacement::Pattern(pattern) => pattern,
            Replacement::Callback(callback) => return Ok(Self::Callback(callback)),
        };
        let order = PatternNode::template_order(&root);
        let mut operands = Vec::with_capacity(order.descendants.len());
        for visit in order.descendants {
            operands.push(ReplacementOp::compile(visit, captures)?);
        }
        let root_operation = ReplacementOp::compile(order.root, captures)?;
        Ok(Self::Template(Template {
            operands,
            root: root_operation,
        }))
    }

    /// Build the destination of an accepted match; constructors normalize
    /// every result. Returns `None` when `captured` lacks a capture of the
    /// template; a successful match binds every capture of its pattern.
    pub fn instantiate(&self, captured: &HashMap<&str, Expr>) -> Option<Expr> {
        let template = match self {
            Self::Template(template) => template,
            Self::Callback(callback) | Self::ConcreteCallback { callback, .. } => {
                return Some(callback(captured));
            }
            Self::Concrete(entry) => return Some(entry.to().clone()),
        };
        let mut values = Vec::with_capacity(template.operands.len());
        for operation in &template.operands {
            // SAFETY: operands name earlier instructions, and `values` holds
            // one value per earlier instruction.
            let value = unsafe { operation.instantiate(&values, captured) }?;
            values.push(value);
        }
        // SAFETY: the root names instructions of `operands`, and `values`
        // holds one value per instruction of `operands`.
        unsafe { template.root.instantiate(&values, captured) }
    }
}

impl ReplacementOp {
    /// The instruction for one numbered template node.
    fn compile(visit: TemplateVisit<'_>, captures: &FixedSet<&str>) -> Result<Self, RewriteError> {
        let TemplateVisit { pattern, children } = visit;
        Ok(match pattern.node() {
            PatternNode::Capture { name, .. } if !captures.contains(name.as_ref()) => {
                return Err(RewriteError::UnboundCapture(name.as_ref().into()));
            }
            PatternNode::Capture { name, .. } => Self::Capture(Arc::clone(name)),
            PatternNode::Exact(value) => Self::Exact(value.clone()),
            PatternNode::Function { name, .. } => Self::Function {
                name: name.clone(),
                args: children,
            },
            PatternNode::Sum(_) => Self::Sum(children),
            PatternNode::Product(_) => Self::Product(children),
            // SAFETY: a binary template has two children.
            PatternNode::Div(..) => unsafe {
                Self::Div(*children.get_unchecked(0), *children.get_unchecked(1))
            },
            // SAFETY: a binary template has two children.
            PatternNode::Pow(..) => unsafe {
                Self::Pow(*children.get_unchecked(0), *children.get_unchecked(1))
            },
        })
    }

    /// The value of this instruction, or `None` for an unbound capture.
    ///
    /// # Safety
    /// `values` holds a value for every instruction position this one names.
    unsafe fn instantiate(&self, values: &[Expr], captured: &HashMap<&str, Expr>) -> Option<Expr> {
        let operand = |position: &usize| {
            // SAFETY: the caller provides a value for every named position.
            unsafe { values.get_unchecked(*position) }.clone()
        };
        Some(match self {
            Self::Capture(name) => captured.get(name.as_ref())?.clone(),
            Self::Exact(value) => value.clone(),
            Self::Function { name, args } => {
                FunctionDefinition::construct(name.clone(), args.iter().map(operand).collect())
            }
            Self::Sum(args) => Expr::sum(args.iter().map(operand).collect()),
            Self::Product(args) => Expr::product(args.iter().map(operand).collect()),
            Self::Div(left, right) => Div::div(operand(left), operand(right)),
            Self::Pow(base, exponent) => operand(base).pow(operand(exponent)),
        })
    }
}

impl PatternNode {
    /// Number the distinct nodes of `root`, children before parents.
    pub fn template_order(root: &Pattern) -> TemplateOrder<'_> {
        let mut discovery = Discovery {
            index: FixedMap::default(),
            slots: Vec::new(),
            pending: Vec::new(),
            descendants: Vec::new(),
        };
        discovery.index.insert(from_ref(root.node()), 0);
        discovery.slots.push(Slot::new(root));
        discovery.expand(0);
        while let Some((slot, finished)) = discovery.pending.pop() {
            if finished {
                let visit = discovery.finish(slot);
                discovery.descendants.push(visit);
                // SAFETY: pending slots index `slots`, which only grows.
            } else if !unsafe { discovery.slots.get_unchecked(slot) }.expanded {
                // The finish marker goes below the children, so they finish first.
                discovery.pending.push((slot, true));
                discovery.expand(slot);
            }
        }
        let root_visit = discovery.finish(0);
        TemplateOrder {
            descendants: discovery.descendants,
            root: root_visit,
        }
    }
}

impl<'pattern> Slot<'pattern> {
    const fn new(pattern: &'pattern Pattern) -> Self {
        Self {
            pattern,
            children: Vec::new(),
            expanded: false,
            position: 0,
        }
    }
}

impl<'pattern> Discovery<'pattern> {
    /// Assign slots to the children of `slot` and schedule the new ones.
    fn expand(&mut self, slot: usize) {
        // SAFETY: `slot` is a pending or root slot, an index of `slots`.
        let node = unsafe { self.slots.get_unchecked_mut(slot) };
        node.expanded = true;
        let pattern = node.pattern;
        let (many, first, second): (&[Pattern], Option<&Pattern>, Option<&Pattern>) =
            match pattern.node() {
                PatternNode::Capture { .. } | PatternNode::Exact(_) => (&[], None, None),
                PatternNode::Function { args, .. }
                | PatternNode::Sum(args)
                | PatternNode::Product(args) => (args, None, None),
                PatternNode::Div(left, right) | PatternNode::Pow(left, right) => {
                    (&[], Some(left), Some(right))
                }
            };
        let mut children = Vec::with_capacity(many.len().saturating_add(2));
        for child in many.iter().chain(first).chain(second) {
            match self.index.entry(from_ref(child.node())) {
                Entry::Occupied(known) => {
                    let child_slot = *known.get();
                    children.push(child_slot);
                    // SAFETY: indexed slots are indices of `slots`.
                    if !unsafe { self.slots.get_unchecked(child_slot) }.expanded {
                        self.pending.push((child_slot, false));
                    }
                }
                Entry::Vacant(vacant) => {
                    let child_slot = self.slots.len();
                    vacant.insert(child_slot);
                    self.slots.push(Slot::new(child));
                    children.push(child_slot);
                    self.pending.push((child_slot, false));
                }
            }
        }
        // SAFETY: `slot` is an index of `slots`, which only grew above.
        unsafe { self.slots.get_unchecked_mut(slot) }.children = children;
    }

    /// Replace the child slots of `slot` by their positions and give it the
    /// next position.
    fn finish(&mut self, slot: usize) -> TemplateVisit<'pattern> {
        // SAFETY: `slot` is a pending or root slot, an index of `slots`.
        let node = unsafe { self.slots.get_unchecked_mut(slot) };
        node.position = self.descendants.len();
        let pattern = node.pattern;
        let mut children = take(&mut node.children);
        for child in &mut children {
            // SAFETY: child slots index `slots`; a child finished before its
            // parent, so its position is assigned.
            *child = unsafe { self.slots.get_unchecked(*child) }.position;
        }
        TemplateVisit { pattern, children }
    }
}
