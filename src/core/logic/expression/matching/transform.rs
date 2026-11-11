//! Memoized DAG transformation and substitution.

#![expect(
    unsafe_code,
    reason = "a substitution frame holds one value per stored edge, so edge ordinals need no bounds checks"
)]

use core::{convert::Infallible, mem::replace};

use alloc::{borrow::Cow, vec::Vec};

use crate::collections::FixedMap;

use super::{
    ConcreteMatcher, ExactSources, Expr, MatcherHints, Node, PartialSelection, Polynomial,
    Postorder, Selection, SimplePolyFilter, StoredForm, Substitution, Substitutions,
};

/// Direct-mapped stack cache slots for raised base powers `(position, exponent)`.
const POWER_CACHE_SLOTS: usize = 64;

/// Stack cache of materialized `Node::Pow` handles keyed by `(position, exponent)`.
type PowerCache = [(u32, u32, Option<Expr>); POWER_CACHE_SLOTS];

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
    /// Merge only the affected monomial indices of a `simple_bases` polynomial.
    FilteredPoly { affected: Vec<usize> },
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
        // non-literal edge is rebuilt once without projecting compact terms or
        // indexing compact polynomial coefficients.
        if let Node::Symbol(_) = self.from().node() {
            let Ok(substituted) = Postorder::symbolic(expression)
                .map(|node| Ok::<_, Infallible>((node == self.from()).then(|| self.to().clone())));
            return substituted;
        }
        let matcher = ConcreteMatcher::new(ExactSources::One(self.clone()));
        let hints = matcher.hints();
        SubstitutionPass::run(expression, Some(&hints), |node| matcher.select(node))
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
            let Ok(substituted) = Postorder::symbolic(expression).map(|node| {
                Ok::<_, Infallible>(if let Node::Symbol(_) = node.node() {
                    self.get(node).cloned()
                } else {
                    None
                })
            });
            return substituted;
        }
        let matcher = ConcreteMatcher::new(ExactSources::Many(self));
        let hints = matcher.hints();
        SubstitutionPass::run(expression, Some(&hints), |node| matcher.select(node))
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

    /// Replace every occurrence of `from` in `self` by `to`.
    ///
    /// Shorthand for `Substitution::new(from.into(), to.into()).apply_all(self)`.
    /// Matching uses canonical identity rather than symbol names, so symbols
    /// from different contexts stay distinct. Inside sums and products, `from`
    /// also matches disjoint associative and exact power groups; destinations
    /// are not substituted again, and differentiation variables are unchanged.
    #[must_use]
    pub fn substitute(&self, from: impl Into<Self>, to: impl Into<Self>) -> Self {
        Substitution::new(from.into(), to.into()).apply_all(self)
    }
}

impl<'root> SubstitutionPass<'root> {
    /// Select each distinct logical node once, parents before children. A
    /// whole match replaces the node, a partial match rebuilds its selection
    /// from the values of its inputs, and an unmatched node is rebuilt from
    /// the values of its logical children (skipping unaffected compact
    /// polynomial projections when `hints` proves none can match).
    pub fn run(
        root: &'root Expr,
        hints: Option<&MatcherHints>,
        mut select: impl FnMut(&Expr) -> Option<Selection>,
    ) -> Expr {
        let mut pass = Self {
            memo: FixedMap::default(),
            pending: Vec::new(),
            spare: Vec::new(),
        };
        let mut frame = match pass.enter(Step::Visit(Cow::Borrowed(root)), hints, &mut select) {
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
                match pass.enter(step, hints, &mut select) {
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
        hints: Option<&MatcherHints>,
        select: &mut impl FnMut(&Expr) -> Option<Selection>,
    ) -> Entered<'root> {
        let input = match step {
            Step::Visit(input) => input,
            Step::Rebuild(input) => return self.schedule(input, false, hints),
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
            None => self.schedule(input, true, hints),
        }
    }

    /// Schedule the logical children of `input`, which is then rebuilt from
    /// their values and memoized when `memoize` holds.
    fn schedule(
        &mut self,
        input: Cow<'root, Expr>,
        memoize: bool,
        hints: Option<&MatcherHints>,
    ) -> Entered<'root> {
        if let Some(active_hints) = hints {
            if let Node::Poly(polynomial) = input.node()
                && polynomial.simple_bases
                && let Some(outcome) =
                    self.schedule_simple_poly(&input, polynomial, memoize, active_hints)
            {
                return outcome;
            }
            // When no multiplicative or power rule exists, or for a sum with a
            // compact group when no additive rule exists, stored edges suffice.
            let stored_suffices = !active_hints.has_monomial
                || (matches!(input.node(), Node::Sum(_)) && !active_hints.has_additive);
            if stored_suffices {
                return Entered::Frame(self.schedule_stored(input, memoize));
            }
        }
        let floor = self.pending.len();
        if Node::projects(&input) {
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
            Entered::Frame(Frame {
                input,
                plan: Plan::Projected,
                floor,
                values: self.spare.pop().unwrap_or_default(),
                memoize,
            })
        } else {
            Entered::Frame(self.schedule_stored(input, memoize))
        }
    }

    fn schedule_stored(&mut self, input: Cow<'root, Expr>, memoize: bool) -> Frame<'root> {
        let floor = self.pending.len();
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
        Frame {
            input,
            plan: Plan::Stored,
            floor,
            values: self.spare.pop().unwrap_or_default(),
            memoize,
        }
    }

    fn schedule_simple_poly(
        &mut self,
        input: &Cow<'root, Expr>,
        polynomial: &Polynomial,
        memoize: bool,
        hints: &MatcherHints,
    ) -> Option<Entered<'root>> {
        let keep_unchanged = |pass: &mut Self| -> Entered<'root> {
            let kept = Expr::clone(input);
            if memoize {
                pass.memo.insert(kept.clone(), kept.clone());
            }
            Entered::Value(kept)
        };
        let req_groups = match &hints.poly_filter {
            SimplePolyFilter::None => return Some(keep_unchanged(self)),
            SimplePolyFilter::Any => return None,
            SimplePolyFilter::Sources(groups) => groups,
        };
        let mut resolved: Vec<Vec<(u32, u32)>> = Vec::new();
        for group in req_groups {
            let mut mapped_group = Vec::with_capacity(group.len());
            let mut present = true;
            for (req_base, min_exp) in group {
                if let Some(pos) = polynomial.bases.iter().position(|base| base == req_base)
                    && let Ok(pos_u32) = u32::try_from(pos)
                {
                    mapped_group.push((pos_u32, *min_exp));
                } else {
                    present = false;
                    break;
                }
            }
            if present && !mapped_group.is_empty() {
                resolved.push(mapped_group);
            }
        }
        if resolved.is_empty() {
            return Some(keep_unchanged(self));
        }
        let mut affected = Vec::new();
        for (index, (_, term_powers)) in polynomial.terms().enumerate() {
            let matches_any = resolved.iter().any(|group| {
                group.iter().all(|&(req_pos, min_exp)| {
                    term_powers
                        .iter()
                        .any(|&(pos, exp)| pos == req_pos && exp >= min_exp)
                })
            });
            if matches_any {
                affected.push(index);
            }
        }
        if affected.is_empty() {
            return Some(keep_unchanged(self));
        }
        if affected.len() == polynomial.coefficients.len() {
            return None;
        }
        let floor = self.pending.len();
        let mut power_cache: PowerCache = [const { (0, 0, None) }; POWER_CACHE_SLOTS];
        let mut skip = affected.iter().copied().peekable();
        for (index, (coeff, term_powers)) in polynomial.terms().enumerate() {
            if skip.next_if_eq(&index).is_some() {
                let monomial = polynomial.materialize_term(coeff, term_powers, &mut power_cache);
                self.pending.push(Step::Visit(Cow::Owned(monomial)));
            }
        }
        Some(Entered::Frame(Frame {
            input: input.clone(),
            plan: Plan::FilteredPoly { affected },
            floor,
            values: self.spare.pop().unwrap_or_default(),
            memoize,
        }))
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
            Plan::FilteredPoly { affected } => {
                if let Node::Poly(polynomial) = input.node() {
                    polynomial.merge_substituted_terms(&affected, values)
                } else {
                    Expr::sum(values)
                }
            }
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
