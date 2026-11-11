//! Children-before-parents numbering of the distinct nodes of a DAG.
//!
//! Discovery gives each distinct node a slot with one hash lookup per edge;
//! every later step indexes vectors. Every stored edge is a strong handle and
//! the borrowed DAG is immutable, so until a projection is expanded a node
//! with one strong handle has one incoming edge, is met once, and takes a slot
//! without an index entry. A projection, built here or by another thread, adds
//! handles to existing nodes, so its first expansion indexes every slot met so
//! far and later nodes are always indexed. In an acyclic graph a node finishes
//! after all of its children, so every child position points to an earlier
//! descendant, and values computed in postorder are read by position.
//!
//! Invariants used by the unchecked reads below:
//! - every slot stored in `Discovery::children` or on the pending stack is an
//!   index of `Discovery::slots`, which only grows;
//! - every recorded span lies inside the list it was recorded from, which only
//!   grows;
//! - a node's links hold one position per child of its [`Expansion`], each
//!   below the position of the node itself.

#![expect(
    unsafe_code,
    reason = "indices assigned by the vectors they read need no bounds checks; see the invariants above"
)]

use core::{borrow::Borrow, sync::atomic::Ordering::Relaxed};

use alloc::{sync::Arc, vec::Vec};

use crate::collections::{Entry, FixedMap};

use super::{Expansion, Expr, Forest, Mapped, Node, Numbering, Postorder, Span, StoredForm, Visit};

/// A node met during discovery, indexed by its discovery slot.
struct Slot<'expr> {
    expression: &'expr Expr,
    /// The discovery slots of its children, as a span of `Discovery::children`.
    children: Span,
    expansion: Expansion,
    expanded: bool,
    /// The postorder position, assigned when the node finishes.
    position: usize,
}

#[derive(Default)]
struct Discovery<'expr> {
    /// The slots of the nodes met so far, except those met with one handle
    /// before any projection was expanded.
    index: FixedMap<*const Node, usize>,
    /// Whether a projected node was expanded.
    projected: bool,
    slots: Vec<Slot<'expr>>,
    children: Vec<usize>,
    pending: Vec<(usize, bool)>,
    scratch: Vec<&'expr Expr>,
    descendants: Vec<Visit<'expr>>,
    links: Vec<usize>,
}

impl<'expr> Postorder<'expr> {
    /// Number the nodes reachable through stored edges.
    pub fn stored(root: &'expr Expr) -> Self {
        Self::new(root, |_| Expansion::Stored)
    }

    /// Number the logical nodes: compact polynomial storage contributes its
    /// terms instead of its coefficients and bases.
    pub fn logical(root: &'expr Expr) -> Self {
        Self::new(root, |expression| {
            if Node::projects(expression) {
                Expansion::Projected
            } else {
                Expansion::Stored
            }
        })
    }

    /// Number the logical nodes, keeping compact storage whose bases are all
    /// symbols, \(\pi\), or \(e\) whole: such a polynomial is a leaf, and a sum
    /// holding it as its compact group numbers only its other operands. Other
    /// compact storage contributes its terms.
    pub fn compact(root: &'expr Expr) -> Self {
        Self::new(root, Expansion::compact)
    }

    /// Number as [`Self::compact`] does for a node map identified by the bit
    /// `stage`, whose transform depends on nothing but the node it receives.
    /// [`Self::map`] then records on every node it keeps that the map keeps
    /// it, and a node so recorded by any earlier run is a leaf that the map
    /// keeps without calling its transform or visiting its descendants.
    pub fn settled(root: &'expr Expr, stage: u32) -> Self {
        let mut order = Self::new(root, |expression| {
            if expression.0.settled.load(Relaxed) & stage == 0 {
                Expansion::compact(expression)
            } else {
                Expansion::Settled
            }
        });
        order.settles = stage;
        order
    }

    /// Number the nodes of several roots at once, as [`Self::compact`] does.
    pub fn forest(roots: &'expr [Expr]) -> Forest<'expr> {
        let mut discovery = Discovery::default();
        let mut root_slots = Vec::with_capacity(roots.len());
        for root in roots {
            let fresh = discovery.slots.len();
            let slot = *discovery.index.entry(root.pointer()).or_insert(fresh);
            if slot == fresh {
                discovery.slots.push(Slot::new(root));
            }
            root_slots.push(slot);
        }
        discovery
            .pending
            .extend(root_slots.iter().map(|&slot| (slot, false)));
        discovery.run(Expansion::compact);
        let positions = root_slots
            .iter()
            .map(|&slot| {
                // SAFETY: root slots index `slots`, and every slot has finished.
                unsafe { discovery.slots.get_unchecked(slot) }.position
            })
            .collect();
        Forest {
            numbering: Numbering {
                visits: discovery.descendants,
                links: discovery.links,
            },
            roots: positions,
        }
    }

    /// Number the product and quotient structure of `root`; other nodes are leaves.
    pub fn factors(root: &'expr Expr) -> Self {
        Self::new(root, |expression| {
            if matches!(expression.node(), Node::Product(_) | Node::Div(..)) {
                Expansion::Stored
            } else {
                Expansion::Leaf
            }
        })
    }

    /// The classifier is a type parameter, so each traversal kind gets its own
    /// discovery loop with a direct, inlinable classification.
    fn new(root: &'expr Expr, expansion: impl Fn(&Expr) -> Expansion + Copy) -> Self {
        // The root is never a child in an acyclic graph, so it needs no index
        // entry.
        let mut discovery = Discovery::default();
        discovery.slots.push(Slot::new(root));
        discovery.expand(0, expansion);
        discovery.run(expansion);
        Self {
            root: discovery.finish(0),
            numbering: Numbering {
                visits: discovery.descendants,
                links: discovery.links,
            },
            settles: 0,
        }
    }

    /// The descendants of the root, children before parents.
    pub fn descendants(&self) -> &[Visit<'expr>] {
        &self.numbering.visits
    }

    /// Compute one value per node, children before parents, from the values
    /// of each node's children, and return the root's value.
    ///
    /// # Errors
    /// Returns the first error of `value`; later nodes are not visited.
    pub fn evaluate<Value, Error>(
        &self,
        mut value: impl FnMut(&Mapped<'_, 'expr, Value>) -> Result<Value, Error>,
    ) -> Result<Value, Error> {
        let values = self.numbering.values(&mut value)?;
        let Numbering { visits, links } = &self.numbering;
        value(&Mapped {
            visit: &self.root,
            // SAFETY: the span was recorded from `links` when the root finished.
            links: unsafe { links.get_unchecked(self.root.links.start..self.root.links.end) },
            descendants: visits,
            values: &values,
        })
    }

    /// Transform each numbered node once, children before parents, and return
    /// the root's value. The callback receives the node rebuilt from its
    /// transformed children and returns a replacement, or `None` to keep it;
    /// leaves are passed unchanged and replacements are not traversed. Over a
    /// [`Self::settled`] numbering, settled nodes are kept without a call and
    /// every node whose value is itself is recorded as settled.
    ///
    /// # Errors
    /// Returns the first callback error; later nodes are not visited, and the
    /// nodes already kept stay recorded.
    pub fn map<Error>(
        &self,
        mut transform: impl FnMut(&Expr) -> Result<Option<Expr>, Error>,
    ) -> Result<Expr, Error> {
        self.evaluate(|node| {
            if matches!(node.visit.expansion, Expansion::Settled) {
                return Ok(node.expression().clone());
            }
            let rebuilt = node.rebuild();
            let value = transform(&rebuilt)?.unwrap_or(rebuilt);
            if self.settles != 0 && value == *node.expression() {
                value.0.settled.fetch_or(self.settles, Relaxed);
            }
            Ok(value)
        })
    }
}

impl<'expr> Forest<'expr> {
    /// Compute one value per node, children before parents, from the values
    /// of each node's children, and return the roots' values in root order.
    ///
    /// # Errors
    /// Returns the first error of `value`; later nodes are not visited.
    pub fn evaluate<Value: Clone, Error>(
        &self,
        mut value: impl FnMut(&Mapped<'_, 'expr, Value>) -> Result<Value, Error>,
    ) -> Result<Vec<Value>, Error> {
        let values = self.numbering.values(&mut value)?;
        Ok(self
            .roots
            .iter()
            .map(|&position| {
                // SAFETY: a root position names a visit, and `values` holds one
                // value per visit.
                unsafe { values.get_unchecked(position) }.clone()
            })
            .collect())
    }
}

impl<'expr> Numbering<'expr> {
    /// One value per visit, in visit order, each computed from the values of
    /// the visit's children.
    fn values<Value, Error>(
        &self,
        value: &mut impl FnMut(&Mapped<'_, 'expr, Value>) -> Result<Value, Error>,
    ) -> Result<Vec<Value>, Error> {
        let mut values = Vec::with_capacity(self.visits.len());
        for visit in &self.visits {
            let mapped = Mapped {
                visit,
                // SAFETY: the span was recorded from `links` when `visit` finished.
                links: unsafe { self.links.get_unchecked(visit.links.start..visit.links.end) },
                descendants: &self.visits,
                values: &values,
            };
            let computed = value(&mapped)?;
            values.push(computed);
        }
        Ok(values)
    }
}

impl Expansion {
    /// Keep compact storage in symbols and constants whole, as a leaf or as
    /// the group of its sum; project other compact storage into its terms.
    fn compact(expression: &Expr) -> Self {
        match expression.node() {
            Node::Poly(polynomial) if polynomial.simple_bases => Self::Leaf,
            Node::Poly(_) => Self::Projected,
            Node::Sum(terms) => match Node::compact_group(terms) {
                Some((_, group)) if group.simple_bases => Self::Grouped,
                Some(_) => Self::Projected,
                None => Self::Stored,
            },
            Node::Number(_)
            | Node::ImaginaryUnit
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Product(_)
            | Node::Div(..)
            | Node::Pow(..)
            | Node::Derivative { .. } => Self::Stored,
        }
    }
}

impl<'expr> Visit<'expr> {
    /// The visited expression.
    pub const fn expression(&self) -> &'expr Expr {
        self.expression
    }
}

impl<'run, 'expr: 'run, Value> Mapped<'run, 'expr, Value> {
    /// The visited expression.
    pub const fn expression(&self) -> &'expr Expr {
        self.visit.expression
    }

    /// The compact storage in symbols and constants that this numbering keeps
    /// whole at the node: the node itself when it is numbered as a leaf, or
    /// the compact group of a sum whose children are its other operands.
    pub fn group(&self) -> Option<&'run Expr> {
        let expression = self.expression();
        match (self.visit.expansion, expression.node()) {
            (Expansion::Leaf, Node::Poly(polynomial)) if polynomial.simple_bases => {
                Some(expression)
            }
            (Expansion::Grouped, Node::Sum(terms)) => {
                Node::compact_group(terms).map(|(group, _)| group)
            }
            (
                Expansion::Leaf
                | Expansion::Stored
                | Expansion::Projected
                | Expansion::Grouped
                | Expansion::Settled,
                _,
            ) => None,
        }
    }

    /// The values of the children, in the order of the node's [`Expansion`]:
    /// stored edges, logical terms, or a sum's operands without its group.
    pub fn values(&self) -> impl Iterator<Item = &'run Value> {
        let values = self.values;
        self.links.iter().map(move |&position| {
            // SAFETY: a link is the position of an earlier descendant, and
            // `values` holds one value for every earlier descendant.
            unsafe { values.get_unchecked(position) }
        })
    }

    /// The value of child number `ordinal`.
    ///
    /// # Safety
    /// `ordinal` is below the node's number of children in this numbering,
    /// which its [`Expansion`] gives.
    pub unsafe fn child(&self, ordinal: usize) -> &'run Value {
        // SAFETY: the links hold one position per edge, so `ordinal` indexes
        // them by the caller's contract; each position is below `values.len()`.
        unsafe {
            self.values
                .get_unchecked(*self.links.get_unchecked(ordinal))
        }
    }
}

impl<Value: Borrow<Expr>> Mapped<'_, '_, Value> {
    /// Rebuild the node from the mapped expressions of its children. A node
    /// whose children all map to themselves keeps its pointer, a leaf is
    /// unchanged, and a sum keeps its whole compact group.
    pub fn rebuild(&self) -> Expr {
        match self.visit.expansion {
            Expansion::Leaf | Expansion::Settled => self.expression().clone(),
            Expansion::Stored => Node::rebuild_with(self.expression(), |ordinal, original| {
                debug_assert!(
                    self.links
                        .get(ordinal)
                        .and_then(|&position| self.descendants.get(position))
                        .is_some_and(|visit| visit.expression() == original),
                    "edge ordinals follow `stored_children` order"
                );
                // SAFETY: `rebuild_with` passes ordinals of this node's stored
                // edges, which are below its edge count.
                unsafe { self.child(ordinal) }.borrow()
            }),
            Expansion::Projected | Expansion::Grouped => {
                let unchanged = self.links.iter().all(|&position| {
                    // SAFETY: a link is the position of an earlier descendant,
                    // and both lists hold one entry for every earlier descendant.
                    let (original, mapped) = unsafe {
                        (
                            self.descendants.get_unchecked(position),
                            self.values.get_unchecked(position),
                        )
                    };
                    mapped.borrow() == original.expression()
                });
                if unchanged {
                    return self.expression().clone();
                }
                Expr::sum(
                    self.values()
                        .map(|value| value.borrow().clone())
                        .chain(self.group().cloned())
                        .collect(),
                )
            }
        }
    }
}

impl<'expr> Slot<'expr> {
    const fn new(expression: &'expr Expr) -> Self {
        Self {
            expression,
            children: Span { start: 0, end: 0 },
            expansion: Expansion::Leaf,
            expanded: false,
            position: 0,
        }
    }
}

impl<'expr> Discovery<'expr> {
    /// Expand and finish pending slots until none remain.
    fn run(&mut self, expansion: impl Fn(&Expr) -> Expansion + Copy) {
        while let Some((slot, finished)) = self.pending.pop() {
            if finished {
                let visit = self.finish(slot);
                self.descendants.push(visit);
                // SAFETY: pending slots index `slots`, which only grows.
            } else if !unsafe { self.slots.get_unchecked(slot) }.expanded {
                // The finish marker goes below the children, so they finish first.
                self.pending.push((slot, true));
                self.expand(slot, expansion);
            }
        }
    }

    /// Assign slots to the children of `slot` and schedule the new ones.
    fn expand(&mut self, slot: usize, expansion: impl Fn(&Expr) -> Expansion) {
        // SAFETY: `slot` is a pending or root slot, an index of `slots`.
        let node = unsafe { self.slots.get_unchecked_mut(slot) };
        node.expanded = true;
        node.expansion = expansion(node.expression);
        let expression = node.expression;
        match node.expansion {
            Expansion::Leaf | Expansion::Settled => {}
            Expansion::Projected => {
                // Building or reading a projection can add edges to nodes
                // already met with one handle, so from here on every slot is
                // indexed.
                if !self.projected {
                    self.projected = true;
                    for (position, met) in self.slots.iter().enumerate() {
                        self.index
                            .entry(met.expression.pointer())
                            .or_insert(position);
                    }
                }
                self.scratch.extend(expression.children());
            }
            Expansion::Stored => self.scratch.extend(expression.stored_children()),
            Expansion::Grouped => self.scratch.extend(
                expression
                    .stored_children()
                    .filter(|term| !matches!(term.node(), Node::Poly(_))),
            ),
        }
        let start = self.children.len();
        for child in self.scratch.drain(..) {
            if !self.projected && Arc::strong_count(&child.0) == 1 {
                let child_slot = self.slots.len();
                self.slots.push(Slot::new(child));
                self.children.push(child_slot);
                self.pending.push((child_slot, false));
                continue;
            }
            match self.index.entry(child.pointer()) {
                Entry::Occupied(known) => {
                    let child_slot = *known.get();
                    self.children.push(child_slot);
                    // SAFETY: indexed slots are indices of `slots`.
                    if !unsafe { self.slots.get_unchecked(child_slot) }.expanded {
                        self.pending.push((child_slot, false));
                    }
                }
                Entry::Vacant(vacant) => {
                    let child_slot = self.slots.len();
                    vacant.insert(child_slot);
                    self.slots.push(Slot::new(child));
                    self.children.push(child_slot);
                    self.pending.push((child_slot, false));
                }
            }
        }
        let end = self.children.len();
        // SAFETY: `slot` is an index of `slots`, which only grew above.
        unsafe { self.slots.get_unchecked_mut(slot) }.children = Span { start, end };
    }

    /// Record the child positions of `slot` and give it the next position.
    fn finish(&mut self, slot: usize) -> Visit<'expr> {
        let start = self.links.len();
        // SAFETY: `slot` is a pending or root slot, an index of `slots`, and its
        // child span was recorded from `children` when it was expanded.
        let children = unsafe {
            let span = self.slots.get_unchecked(slot).children;
            self.children.get_unchecked(span.start..span.end)
        };
        for &child in children {
            // SAFETY: child slots index `slots`; a child finished before its
            // parent, so its position is assigned.
            self.links
                .push(unsafe { self.slots.get_unchecked(child) }.position);
        }
        // SAFETY: `slot` is an index of `slots`.
        let node = unsafe { self.slots.get_unchecked_mut(slot) };
        node.position = self.descendants.len();
        Visit {
            expression: node.expression,
            links: Span {
                start,
                end: self.links.len(),
            },
            expansion: node.expansion,
        }
    }
}
