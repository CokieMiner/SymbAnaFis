//! Memoized DAG transformation and substitution.

#![expect(
    unsafe_code,
    reason = "a substitution frame holds one value per stored edge, so edge ordinals need no bounds checks"
)]

use core::{convert::Infallible, mem::replace};

use alloc::{borrow::Cow, vec::Vec};

use crate::collections::FixedMap;

use super::{
    ConcreteMatcher, ExactSources, Expr, Node, PartialSelection, Postorder, Selection, StoredForm,
    Substitution, Substitutions,
};

/// The shared state of one substitution pass.
///
/// A frame's children are the pending steps above its floor. Each child
/// delivers exactly one value to the frame, directly or when its own frame
/// finishes, so a frame whose floor is reached holds one value per child.
pub struct SubstitutionPass<'root> {
    memo: FixedMap<Expr, Expr>,
    pending: Vec<Step<'root>>,
    /// Emptied value buffers, reused by later frames.
    spare: Vec<Vec<Expr>>,
}

/// One pending child of a substitution frame.
pub enum Step<'root> {
    /// Select the node, or reuse its memoized value.
    Visit(Cow<'root, Expr>),
    /// Rebuild the node from its logical children without selecting it again.
    Rebuild(Cow<'root, Expr>),
}

/// A node of a substitution pass whose children are being visited.
struct Frame<'root> {
    input: Cow<'root, Expr>,
    plan: Plan,
    /// The number of pending steps below this node's children.
    floor: usize,
    /// One value per child visited so far, in reverse order of scheduling.
    values: Vec<Expr>,
    memoize: bool,
}

/// How a frame combines the values of its children.
enum Plan {
    /// Rebuild from the values of the stored edges.
    Stored,
    /// Sum the values of the logical terms.
    Projected,
    /// Rebuild the selection with each input replaced by its value.
    Partial(PartialSelection),
}

/// The result of entering a step.
enum Entered<'root> {
    /// The step's value, known without visiting children.
    Value(Expr),
    /// A frame whose children were scheduled.
    Frame(Frame<'root>),
}

impl Substitution {
    /// Replace every occurrence of the source in `expression` by the destination.
    ///
    /// A matched node is replaced whole, without visiting its descendants.
    /// Inside sums and products the source also matches disjoint groups: exact
    /// power groups that preserve principal branches and zero exclusions, common
    /// additive factors, and numeric coefficient quotients. Compact polynomial
    /// terms match through their logical form. Every edge to a shared node gets
    /// the same result; destinations and generated coefficients and exponents
    /// are not matched again.
    #[must_use]
    pub fn apply_all(&self, expression: &Expr) -> Expr {
        if self.from() == self.to() {
            return expression.clone();
        }
        // A symbol matches only itself and forms no implicit group, so every
        // stored edge is rebuilt once without projecting compact terms.
        if let Node::Symbol(_) = self.from().node() {
            let Ok(substituted) = Postorder::stored(expression)
                .map(|node| Ok::<_, Infallible>((node == self.from()).then(|| self.to().clone())));
            return substituted;
        }
        let matcher = ConcreteMatcher::new(ExactSources::One(self.clone()));
        SubstitutionPass::run(expression, |node| matcher.select(node))
    }
}

impl Substitutions {
    /// Apply all entries simultaneously, matching each original node once.
    ///
    /// A matched parent takes precedence over its descendants. Destinations and
    /// rebuilt nodes are not matched in the same pass, and unchanged nodes keep
    /// their pointers.
    #[must_use]
    pub fn apply_all(&self, expression: &Expr) -> Expr {
        if self
            .iter()
            .all(|(source, destination)| source == destination)
        {
            return expression.clone();
        }
        if self
            .iter()
            .all(|(source, _)| matches!(source.node(), Node::Symbol(_)))
        {
            let Ok(substituted) = Postorder::stored(expression).map(|node| {
                Ok::<_, Infallible>(if let Node::Symbol(_) = node.node() {
                    self.get(node).cloned()
                } else {
                    None
                })
            });
            return substituted;
        }
        let matcher = ConcreteMatcher::new(ExactSources::Many(self));
        SubstitutionPass::run(expression, |node| matcher.select(node))
    }
}

impl Expr {
    /// Transform each distinct logical node once, children before parents.
    ///
    /// The callback receives the node rebuilt from its transformed children and
    /// returns a replacement, or `None` to keep it. Compact polynomial terms are
    /// visited as ordinary terms, and replacements are not traversed. Unchanged
    /// nodes keep their pointers.
    ///
    /// # Errors
    /// Returns the first callback error. The partial output is discarded;
    /// callback side effects are not rolled back.
    ///
    /// # Panics
    /// Propagates callback panics.
    pub fn map<Error>(
        &self,
        transform: impl FnMut(&Self) -> Result<Option<Self>, Error>,
    ) -> Result<Self, Error> {
        Postorder::logical(self).map(transform)
    }

    /// Replace every symbol named `name`, from any context, by `replacement`.
    /// Differentiation variables are unchanged; a [`Substitution`] selects a
    /// single identity.
    #[must_use]
    pub fn substitute(&self, name: &str, replacement: &Self) -> Self {
        let Ok(substituted) = Postorder::stored(self).map(|node| {
            Ok::<_, Infallible>(
                if let Node::Symbol(data) = node.node()
                    && data.name() == Some(name)
                {
                    Some(replacement.clone())
                } else {
                    None
                },
            )
        });
        substituted
    }
}

impl<'root> SubstitutionPass<'root> {
    /// Select each distinct logical node once, parents before children. A
    /// whole match replaces the node, a partial match rebuilds its selection
    /// from the values of its inputs, and an unmatched node is rebuilt from
    /// the values of its logical children.
    pub fn run(root: &'root Expr, mut select: impl FnMut(&Expr) -> Option<Selection>) -> Expr {
        let mut pass = Self {
            memo: FixedMap::default(),
            pending: Vec::new(),
            spare: Vec::new(),
        };
        let mut frame = match pass.enter(Step::Visit(Cow::Borrowed(root)), &mut select) {
            Entered::Value(value) => return value,
            Entered::Frame(frame) => frame,
        };
        let mut parents = Vec::new();
        loop {
            let child = if pass.pending.len() > frame.floor {
                pass.pending.pop()
            } else {
                None
            };
            if let Some(step) = child {
                match pass.enter(step, &mut select) {
                    Entered::Value(value) => frame.values.push(value),
                    Entered::Frame(nested) => parents.push(replace(&mut frame, nested)),
                }
                continue;
            }
            let value = pass.finish(frame);
            let Some(parent) = parents.pop() else {
                return value;
            };
            frame = parent;
            frame.values.push(value);
        }
    }

    /// Reuse the value of a visited node, select it, or schedule its children.
    fn enter(
        &mut self,
        step: Step<'root>,
        select: &mut impl FnMut(&Expr) -> Option<Selection>,
    ) -> Entered<'root> {
        let input = match step {
            Step::Visit(input) => input,
            Step::Rebuild(input) => return Entered::Frame(self.schedule(input, false)),
        };
        if let Some(value) = self.memo.get(&*input) {
            return Entered::Value(value.clone());
        }
        match select(&input) {
            Some(Selection::Whole(replacement)) => {
                self.memo.insert(input.into_owned(), replacement.clone());
                Entered::Value(replacement)
            }
            Some(Selection::Partial(mut selection)) => {
                let floor = self.pending.len();
                selection.for_each_input(&mut |child| {
                    // A residual power may keep this node as its base. Its
                    // children are original; its new power is not.
                    self.pending.push(if *child == *input {
                        Step::Rebuild(input.clone())
                    } else {
                        Step::Visit(Cow::Owned(child.clone()))
                    });
                });
                Entered::Frame(Frame {
                    input,
                    plan: Plan::Partial(selection),
                    floor,
                    values: self.spare.pop().unwrap_or_default(),
                    memoize: true,
                })
            }
            None => Entered::Frame(self.schedule(input, true)),
        }
    }

    /// Schedule the logical children of `input`, which is then rebuilt from
    /// their values and memoized when `memoize` holds.
    fn schedule(&mut self, input: Cow<'root, Expr>, memoize: bool) -> Frame<'root> {
        let floor = self.pending.len();
        let plan = if Node::projects(&input) {
            match &input {
                Cow::Borrowed(stored) => self.pending.extend(
                    stored
                        .children()
                        .map(|term| Step::Visit(Cow::Borrowed(term))),
                ),
                Cow::Owned(owned) => self.pending.extend(
                    owned
                        .children()
                        .cloned()
                        .map(|term| Step::Visit(Cow::Owned(term))),
                ),
            }
            Plan::Projected
        } else {
            match &input {
                Cow::Borrowed(stored) => self.pending.extend(
                    stored
                        .stored_children()
                        .map(|child| Step::Visit(Cow::Borrowed(child))),
                ),
                Cow::Owned(owned) => self.pending.extend(
                    owned
                        .stored_children()
                        .cloned()
                        .map(|child| Step::Visit(Cow::Owned(child))),
                ),
            }
            Plan::Stored
        };
        Frame {
            input,
            plan,
            floor,
            values: self.spare.pop().unwrap_or_default(),
            memoize,
        }
    }

    /// Combine the values of a frame's children into the node's value.
    fn finish(&mut self, frame: Frame<'root>) -> Expr {
        let Frame {
            input,
            plan,
            mut values,
            memoize,
            ..
        } = frame;
        // Children were pushed in order and popped from the top.
        values.reverse();
        // Buffers whose values are only read return to the pool; consumed
        // buffers become the operands of the rebuilt node.
        let value = match plan {
            Plan::Stored => {
                let rebuilt = Node::rebuild_with(&input, |ordinal, _| {
                    // SAFETY: the frame holds one value per stored edge, and
                    // `rebuild_with` passes ordinals of stored edges.
                    unsafe { values.get_unchecked(ordinal) }
                });
                values.clear();
                self.spare.push(values);
                rebuilt
            }
            Plan::Projected if values.iter().eq(input.children()) => {
                values.clear();
                self.spare.push(values);
                Expr::clone(&input)
            }
            Plan::Projected => Expr::sum(values),
            Plan::Partial(mut selection) => {
                let mut mapped = values.into_iter();
                selection.for_each_input(&mut |child| {
                    if let Some(value) = mapped.next() {
                        *child = value;
                    }
                });
                selection.rebuild()
            }
        };
        if memoize {
            self.memo.insert(input.into_owned(), value.clone());
        }
        value
    }
}
