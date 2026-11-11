//! Term-table ownership and greedy multivariate Horner grouping.
//!
//! An inverted index built by counting and a lazily refreshed priority queue
//! group a set of $F$ factor occurrences in $O(F \log F)$. A nested set has
//! lower total degree than its parent, and the sets in progress live on an
//! explicit stack, so one call reaches the complete nested form without
//! recursion on the call stack.

#![expect(
    unsafe_code,
    reason = "base indices, incidence spans, and power positions are assigned by the vectors they index"
)]

use core::mem::replace;

use alloc::{collections::BinaryHeap, vec, vec::Vec};

use crate::{collections::FixedMap, core::Expr};

/// A term: its numeric coefficient, `None` for the exact unit, and its powers
/// as base indices with positive multiplicities, one per base in no particular
/// order.
#[derive(Default)]
pub struct Term {
    pub coefficient: Option<Expr>,
    pub powers: Vec<(usize, i64)>,
}

/// The distinct bases of one factored sum.
pub struct Factoring {
    bases: Vec<Base>,
    index: FixedMap<Expr, usize>,
    identities: bool,
    changed: bool,
}

/// A distinct base of the factored sum.
struct Base {
    expression: Expr,
    /// The number of its occurrences in the outermost sum.
    occurrences: usize,
    /// Zero outside [`Factoring::level`], which first counts the base's terms
    /// in the set being indexed here and then stores its local index.
    slot: usize,
}

/// One base of a set being grouped.
struct Local {
    /// Its index in [`Factoring::bases`].
    global: usize,
    /// The span of its incidences.
    start: usize,
    end: usize,
    /// The number of ungrouped terms that contain it.
    live: usize,
}

/// A set of terms whose sum is being grouped.
///
/// While the set is grouped, its terms name their bases by position in
/// `locals`; a term leaves with global indices when it joins a group.
pub struct Level {
    terms: Vec<Option<Term>>,
    /// `(term, power position)`, by local base and then term.
    incidences: Vec<(usize, usize)>,
    locals: Vec<Local>,
    /// Coverage, occurrences in the outermost sum, and local base. Locals
    /// follow global order, so equal keys up to the local prefer the base read
    /// last. A popped entry whose coverage has dropped is pushed again with its
    /// current coverage.
    queue: BinaryHeap<(usize, usize, usize)>,
    /// Factors removed from every term, which multiply the finished sum.
    enclosing: Vec<Expr>,
    /// Products of an extracted power and its finished group.
    grouped: Vec<Expr>,
    /// The global base and multiplicity extracted for the group in progress.
    extracted: (usize, i64),
}

/// A set reduced before grouping, or its finished expression.
pub enum Start {
    Done(Expr),
    Grouping(Level),
}

impl Factoring {
    /// Start a factoring context over an empty base table.
    pub fn new(identities: bool) -> Self {
        Self {
            bases: Vec::new(),
            index: FixedMap::default(),
            identities,
            changed: false,
        }
    }

    /// Whether binomial identities are enabled for this factoring run.
    pub const fn has_identities(&self) -> bool {
        self.identities
    }

    /// Whether any factor, binomial identity, or grouping changed the sum.
    pub const fn is_changed(&self) -> bool {
        self.changed
    }

    /// Record whether a factoring step changed the expression.
    pub const fn mark_changed(&mut self, changed: bool) {
        self.changed |= changed;
    }

    /// Intern `base`, incrementing its outermost occurrence count.
    pub fn base(&mut self, base: &Expr) -> usize {
        if let Some(&index) = self.index.get(base) {
            // SAFETY: an indexed base was pushed at its index.
            let entry = unsafe { self.bases.get_unchecked_mut(index) };
            entry.occurrences = entry.occurrences.saturating_add(1);
            return index;
        }
        let index = self.bases.len();
        self.bases.push(Base {
            expression: base.clone(),
            occurrences: 1,
            slot: 0,
        });
        self.index.insert(base.clone(), index);
        index
    }

    /// Index the bases of a set and queue those shared by several terms.
    ///
    /// A counting pass gives each base its coverage, the distinct bases in
    /// ascending global order become the locals, and a second pass places each
    /// incidence in its base's run in term order: $O(F + D \log D)$ for $F$
    /// factor occurrences over $D$ distinct bases.
    pub fn level(&mut self, mut terms: Vec<Term>, enclosing: Vec<Expr>) -> Level {
        let mut distinct = Vec::new();
        for term in &terms {
            for &(global, _) in &term.powers {
                // SAFETY: every base index was assigned by `Factoring::base`.
                let base = unsafe { self.bases.get_unchecked_mut(global) };
                if base.slot == 0 {
                    distinct.push(global);
                }
                base.slot = base.slot.saturating_add(1);
            }
        }
        distinct.sort_unstable();
        let mut locals = Vec::with_capacity(distinct.len());
        let mut shared = Vec::new();
        let mut start = 0_usize;
        for (local, &global) in distinct.iter().enumerate() {
            // SAFETY: as above.
            let base = unsafe { self.bases.get_unchecked_mut(global) };
            // A term holds one power per base, so the count is the coverage.
            let coverage = replace(&mut base.slot, local);
            if coverage > 1 {
                shared.push((coverage, base.occurrences, local));
            }
            locals.push(Local {
                global,
                start,
                end: start,
                live: coverage,
            });
            start = start.saturating_add(coverage);
        }
        let mut incidences = vec![(0, 0); start];
        for (index, term) in terms.iter_mut().enumerate() {
            for (position, power) in term.powers.iter_mut().enumerate() {
                // SAFETY: as above; the slot now holds an index of `locals`.
                power.0 = unsafe { self.bases.get_unchecked(power.0) }.slot;
                // SAFETY: as above.
                let run = unsafe { locals.get_unchecked_mut(power.0) };
                // SAFETY: each base receives exactly `coverage` incidences, so
                // `end` stays below the start of the next run.
                *unsafe { incidences.get_unchecked_mut(run.end) } = (index, position);
                run.end = run.end.saturating_add(1);
            }
        }
        for &global in &distinct {
            // SAFETY: as above.
            unsafe { self.bases.get_unchecked_mut(global) }.slot = 0;
        }
        Level {
            terms: terms.into_iter().map(Some).collect(),
            incidences,
            locals,
            queue: BinaryHeap::from(shared),
            enclosing,
            grouped: Vec::new(),
            extracted: (0, 0),
        }
    }

    /// Materialize a `Term` back into a product expression.
    pub fn term(&self, term: &Term) -> Expr {
        Expr::product(
            term.coefficient
                .iter()
                .cloned()
                .chain(
                    term.powers
                        .iter()
                        .map(|&(base, count)| self.power(base, count)),
                )
                .collect(),
        )
    }

    /// Materialize `base^count` from the base table.
    pub fn power(&self, base: usize, count: i64) -> Expr {
        // SAFETY: every base index was assigned by `Factoring::base`.
        let expression = &unsafe { self.bases.get_unchecked(base) }.expression;
        if count == 1 {
            expression.clone()
        } else {
            expression.pow(count)
        }
    }
}

impl Level {
    /// Global base index and extracted multiplicity for the active child group.
    pub const fn extracted(&self) -> (usize, i64) {
        self.extracted
    }

    /// Append a completed factor-group product to this level.
    pub fn push_grouped(&mut self, expression: Expr) {
        self.grouped.push(expression);
    }

    /// Take the terms of the most frequent base with that base reduced by its
    /// least multiplicity, or `None` when no base is shared by two terms.
    pub fn next(&mut self) -> Option<Vec<Term>> {
        while let Some((coverage, occurrences, local)) = self.queue.pop() {
            // SAFETY: queued indices name locals.
            let base = unsafe { self.locals.get_unchecked(local) };
            if base.live != coverage {
                if base.live > 1 {
                    self.queue.push((base.live, occurrences, local));
                }
                continue;
            }
            let global = base.global;
            // SAFETY: a local's span was recorded within `incidences`.
            let run = unsafe { self.incidences.get_unchecked(base.start..base.end) };
            let least = run
                .iter()
                .filter_map(|&(index, position)| {
                    // SAFETY: incidences name terms and positions in their powers.
                    let term = unsafe { self.terms.get_unchecked(index) }.as_ref()?;
                    // SAFETY: as above.
                    Some(unsafe { term.powers.get_unchecked(position) }.1)
                })
                .min()?;
            let mut group = Vec::with_capacity(coverage);
            for &(index, position) in run {
                // SAFETY: incidences name terms.
                let Some(mut term) = unsafe { self.terms.get_unchecked_mut(index) }.take() else {
                    continue;
                };
                for &(other, _) in &term.powers {
                    // SAFETY: the powers of an ungrouped term name locals.
                    let entry = unsafe { self.locals.get_unchecked_mut(other) };
                    entry.live = entry.live.saturating_sub(1);
                }
                // SAFETY: incidences name positions in their term's powers.
                let extracted = unsafe { term.powers.get_unchecked_mut(position) };
                extracted.1 = extracted.1.saturating_sub(least);
                // Powers are unordered, so the last power fills the place of
                // an exhausted one unless it was the exhausted one.
                if extracted.1 == 0
                    && let Some(last) = term.powers.pop()
                    && let Some(place) = term.powers.get_mut(position)
                {
                    *place = last;
                }
                for power in &mut term.powers {
                    // SAFETY: the powers of an ungrouped term name locals.
                    power.0 = unsafe { self.locals.get_unchecked(power.0) }.global;
                }
                group.push(term);
            }
            self.extracted = (global, least);
            return Some(group);
        }
        None
    }

    /// The sum of the grouped products and the remaining terms, times the
    /// enclosing factors.
    pub fn finish(mut self, factoring: &Factoring) -> Expr {
        for mut term in self.terms.into_iter().flatten() {
            for power in &mut term.powers {
                // SAFETY: the powers of an ungrouped term name locals.
                power.0 = unsafe { self.locals.get_unchecked(power.0) }.global;
            }
            self.grouped.push(factoring.term(&term));
        }
        let sum = Expr::sum(self.grouped);
        if self.enclosing.is_empty() {
            sum
        } else {
            self.enclosing.push(sum);
            Expr::product(self.enclosing)
        }
    }
}
