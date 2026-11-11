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

use crate::collections::{Entry, FixedMap, HashMap};

use super::{
    Expr, Function, FunctionDefinition, Pattern, PatternNode, ReplacementProgram, RewriteError,
};

/// The distinct nodes of a pattern DAG, children before parents.
///
/// Each node lists its children as positions of earlier descendants, so values
/// computed in this order are read by position. The root is never a child, so
/// it is kept apart from the descendants.
pub struct TemplateOrder<'pattern> {
    pub descendants: Vec<TemplateVisit<'pattern>>,
    pub root: TemplateVisit<'pattern>,
}

/// Several numbered roots sharing one discovery, children before parents.
pub struct TemplateOrderMany<'pattern> {
    pub descendants: Vec<TemplateVisit<'pattern>>,
    /// The position of each root in `descendants`, in the order given.
    pub roots: Vec<usize>,
}

/// A numbered pattern node and the positions of its children, in template order.
pub struct TemplateVisit<'pattern> {
    pub pattern: &'pattern Pattern,
    pub children: Vec<usize>,
}

/// Replacement instructions in children-before-parents order.
///
/// Built only by [`Template::compile`]: every operand position names an earlier
/// instruction, and the root may name any of `operands`.
#[derive(Clone)]
pub struct Template {
    pub operands: Vec<ReplacementOp>,
    pub root: ReplacementOp,
}

#[derive(Clone)]
pub enum ReplacementOp {
    Capture(usize),
    Exact(Expr),
    Function { name: Function, args: Vec<usize> },
    Sum(Vec<usize>),
    Product(Vec<usize>),
    Div(usize, usize),
    Pow(usize, usize),
}

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

impl ReplacementProgram {
    /// Build the destination of an accepted match; constructors normalize
    /// every result. Returns `None` when `slots` lacks a capture of the
    /// template; a successful match binds every capture of its pattern.
    ///
    /// `values` is scratch space for template instructions, and `bindings` is a
    /// lazily allocated named-capture map reused across callback invocations in
    /// the pass.
    pub fn instantiate<'rule>(
        &self,
        slots: &[Option<Expr>],
        capture_names: &'rule [Arc<str>],
        values: &mut Vec<Expr>,
        bindings: &mut Option<HashMap<&'rule str, Expr>>,
    ) -> Option<Expr> {
        match self {
            Self::Template(template) => template.instantiate(slots, values),
            Self::Callback(callback) | Self::ConcreteCallback { callback, .. } => {
                let map =
                    bindings.get_or_insert_with(|| HashMap::with_capacity(capture_names.len()));
                map.clear();
                for (name, value) in capture_names.iter().zip(slots) {
                    if let Some(bound) = value {
                        map.insert(name.as_ref(), bound.clone());
                    }
                }
                Some(callback(map))
            }
            Self::Concrete(entry) => Some(entry.to().clone()),
        }
    }
}

impl Template {
    /// Compile `root` into numbered instructions whose captures index
    /// `capture_names`.
    pub fn compile(root: &Pattern, capture_names: &[Arc<str>]) -> Result<Self, RewriteError> {
        let order = PatternNode::template_order(root);
        let mut operands = Vec::with_capacity(order.descendants.len());
        for visit in order.descendants {
            operands.push(ReplacementOp::compile(visit, capture_names)?);
        }
        let root_operation = ReplacementOp::compile(order.root, capture_names)?;
        Ok(Self {
            operands,
            root: root_operation,
        })
    }

    /// Evaluate compiled instructions from dense capture `slots`, reusing
    /// `values` as scratch space.
    pub fn instantiate(&self, slots: &[Option<Expr>], values: &mut Vec<Expr>) -> Option<Expr> {
        values.clear();
        values.reserve(self.operands.len());
        for operation in &self.operands {
            // SAFETY: operands name earlier instructions, and `values` holds
            // one value per earlier instruction.
            let value = unsafe { operation.instantiate(values, slots) }?;
            values.push(value);
        }
        // SAFETY: the root names instructions of `operands`, and `values`
        // holds one value per instruction of `operands`.
        unsafe { self.root.instantiate(values, slots) }
    }
}

impl ReplacementOp {
    /// The instruction for one numbered template node.
    fn compile(visit: TemplateVisit<'_>, capture_names: &[Arc<str>]) -> Result<Self, RewriteError> {
        let TemplateVisit { pattern, children } = visit;
        Ok(match pattern.node() {
            PatternNode::Capture { name, .. } => {
                let Some(slot) = capture_names
                    .iter()
                    .position(|bound| Arc::ptr_eq(bound, name) || bound.as_ref() == name.as_ref())
                else {
                    return Err(RewriteError::UnboundCapture(name.as_ref().into()));
                };
                Self::Capture(slot)
            }
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
    unsafe fn instantiate(&self, values: &[Expr], slots: &[Option<Expr>]) -> Option<Expr> {
        let operand = |position: &usize| {
            // SAFETY: the caller provides a value for every named position.
            unsafe { values.get_unchecked(*position) }.clone()
        };
        Some(match self {
            Self::Capture(slot) => slots.get(*slot)?.as_ref()?.clone(),
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
        discovery.slots.push(Slot {
            pattern: root,
            children: Vec::new(),
            expanded: false,
            position: 0,
        });
        discovery.expand(0);
        discovery.drain();
        let root_visit = discovery.finish(0);
        TemplateOrder {
            descendants: discovery.descendants,
            root: root_visit,
        }
    }

    /// Number the distinct nodes of every root in one discovery, children
    /// before parents.
    ///
    /// A flattened operand list would otherwise pay one discovery, and its
    /// index map and three vectors, per operand. A node shared by two roots
    /// takes one slot, so it is lowered once.
    pub fn template_order_many(roots: &[Pattern]) -> TemplateOrderMany<'_> {
        let mut discovery = Discovery {
            index: FixedMap::default(),
            slots: Vec::new(),
            pending: Vec::new(),
            descendants: Vec::new(),
        };
        let mut root_slots = Vec::with_capacity(roots.len());
        for root in roots {
            match discovery.index.entry(from_ref(root.node())) {
                Entry::Occupied(known) => root_slots.push(*known.get()),
                Entry::Vacant(vacant) => {
                    let slot = discovery.slots.len();
                    vacant.insert(slot);
                    discovery.slots.push(Slot {
                        pattern: root,
                        children: Vec::new(),
                        expanded: false,
                        position: 0,
                    });
                    root_slots.push(slot);
                }
            }
        }
        discovery
            .pending
            .extend(root_slots.iter().map(|&slot| (slot, false)));
        discovery.drain();
        let positions = root_slots
            .iter()
            // SAFETY: a root slot is an index of `slots`, and every root
            // finished in `drain` above.
            .map(|&slot| unsafe { discovery.slots.get_unchecked(slot) }.position)
            .collect();
        TemplateOrderMany {
            descendants: discovery.descendants,
            roots: positions,
        }
    }
}

impl<'pattern> Discovery<'pattern> {
    /// Expand pending slots and append finished visits in postorder.
    fn drain(&mut self) {
        while let Some((slot, finished)) = self.pending.pop() {
            if finished {
                let visit = self.finish(slot);
                self.descendants.push(visit);
                // SAFETY: pending slots index `slots`, which only grows.
            } else if !unsafe { self.slots.get_unchecked(slot) }.expanded {
                // The finish marker goes below the children, so they finish first.
                self.pending.push((slot, true));
                self.expand(slot);
            }
        }
    }

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
        // Exact capacity, so a leaf gets a vector that never allocates.
        let mut children = Vec::with_capacity(
            many.len()
                .saturating_add(usize::from(first.is_some()))
                .saturating_add(usize::from(second.is_some())),
        );
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
                    self.slots.push(Slot {
                        pattern: child,
                        children: Vec::new(),
                        expanded: false,
                        position: 0,
                    });
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
