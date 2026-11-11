//! Iterative commutative search with transactional bindings and pending work.

use core::ptr::from_ref;

use alloc::{borrow::Cow, sync::Arc, vec::Vec};

use crate::collections::{FixedSet, HashMap};

use super::{
    Assignment, CountedSlot, Expansion, Expr, ExprView, Group, GroupValues, MatchChildren,
    MatchWork, Pattern, PatternNode, RewriteError, Selected, Taken, Task, Template,
};

pub struct Search<'pattern, 'input> {
    pending: Vec<Task<'pattern, 'input>>,
    /// Tasks popped under an open choice, restored in reverse by `Undo::Popped`.
    popped: Vec<Task<'pattern, 'input>>,
    undo: Vec<Undo>,
    choices: Vec<Choice<'pattern, 'input>>,
    /// Distinct capture names of the active rule, indexing `MatchWork::slots`.
    capture_names: &'pattern [Arc<str>],
    /// The rule's compiled rigid operand templates, keyed by template address.
    grounds: &'pattern [(usize, Template)],
    /// One slot per counted operand, when the caller built them; a slot whose
    /// head sets the pattern rejects is skipped before its value is built.
    heads: Option<&'input [CountedSlot]>,
    /// The template nodes whose expansions can repeat; see `RewriteRule`.
    repeats: &'pattern FixedSet<usize>,
}

#[derive(Debug)]
enum Undo {
    Capture(usize),
    Expanded(*const PatternNode, Expr),
    Pushed,
    /// Restore the last task of `Search::popped`.
    Popped,
    /// Truncate `MatchWork::projected` to this length.
    Projected(usize),
}

#[derive(Debug)]
struct Choice<'pattern, 'input> {
    checkpoint: usize,
    retry: Group<'pattern, 'input>,
}

impl<'pattern, 'input> Search<'pattern, 'input> {
    pub const fn new(
        capture_names: &'pattern [Arc<str>],
        grounds: &'pattern [(usize, Template)],
        heads: Option<&'input [CountedSlot]>,
        repeats: &'pattern FixedSet<usize>,
    ) -> Self {
        Self {
            pending: Vec::new(),
            popped: Vec::new(),
            undo: Vec::new(),
            choices: Vec::new(),
            capture_names,
            grounds,
            heads,
            repeats,
        }
    }

    /// Search from `initial`, reusing scratch space in `self` and `work` across
    /// calls. Accepted capture bindings remain in `work.slots` on `Ok(Some(_))`.
    pub fn run<'rule>(
        &mut self,
        initial: Task<'pattern, 'input>,
        work: &mut MatchWork<'rule>,
        guard: &mut impl FnMut(&[Option<Expr>], &mut Option<HashMap<&'rule str, Expr>>) -> bool,
    ) -> Result<Option<Selected>, RewriteError> {
        self.pending.clear();
        self.popped.clear();
        self.undo.clear();
        self.choices.clear();
        work.slots.clear();
        work.slots.resize(self.capture_names.len(), None);
        work.projected.clear();
        if !self.repeats.is_empty() {
            work.expanded.clear();
        }
        let mut next = Some(initial);
        loop {
            let task = next.take().or_else(|| self.pending.pop());
            let transactional = !self.choices.is_empty();
            // Markers alone order restoration: the marker precedes the entries
            // this task records, and the task joins `popped` once it is no
            // longer borrowed.
            if transactional && task.is_some() {
                self.undo.push(Undo::Popped);
            }
            let matched = match task {
                Some(Task::Match(pattern, input)) => {
                    let matched = self.expand(pattern, &input, work);
                    if transactional {
                        self.popped.push(Task::Match(pattern, input));
                    }
                    matched
                }
                Some(Task::Group(group)) => {
                    if transactional {
                        self.popped.push(Task::Group(group.clone()));
                    }
                    if group.patterns.is_empty() {
                        if !group.root {
                            let complete = match (&group.values, &group.selected) {
                                (GroupValues::Owned(range), Selected::Explicit(indices)) => {
                                    indices.len() == range.len()
                                }
                                (GroupValues::Borrowed(values), Selected::Explicit(indices)) => {
                                    indices.len() == values.len()
                                }
                                _ => false,
                            };
                            if complete {
                                continue;
                            }
                            false
                        } else if guard(&work.slots, &mut work.bindings) {
                            return Ok(Some(group.selected));
                        } else {
                            false
                        }
                    } else {
                        self.assign(group, work)?
                    }
                }
                None => {
                    if guard(&work.slots, &mut work.bindings) {
                        return Ok(Some(Selected::Explicit(Taken::default())));
                    }
                    false
                }
            };
            if !matched && !self.backtrack(work) {
                return Ok(None);
            }
        }
    }

    fn assign(
        &mut self,
        mut group: Group<'pattern, 'input>,
        work: &mut MatchWork<'_>,
    ) -> Result<bool, RewriteError> {
        let Some((pattern, rest)) = group.patterns.split_first() else {
            return Ok(false);
        };
        let draw = group.draw(pattern);
        let ground = self.ground(pattern, &group, work);
        if let Some(position) = ground {
            group.next = position;
        }
        while let Some((candidate, assignment)) =
            group.candidate(pattern, &draw, &work.projected, self.heads)
        {
            if ground.is_some() {
                // The operand equal to the instantiation is the only candidate.
                group.next = usize::MAX;
            }
            work.remaining = work
                .remaining
                .checked_sub(1)
                .ok_or(RewriteError::MatchLimitExceeded { limit: work.limit })?;
            let template = match &assignment {
                Assignment::Projected { base, .. } => *base,
                Assignment::Explicit(_) | Assignment::Group(_) | Assignment::Counted(_) => pattern,
            };
            if !self.compatible(template, &candidate, &work.slots) {
                continue;
            }
            // Only a compatible candidate is owned by the search.
            let value = candidate.into_owned();
            let mut selected = group.selected.clone();
            match (&mut selected, assignment) {
                (Selected::Explicit(indices), Assignment::Explicit(index)) => {
                    indices.insert(index);
                }
                (Selected::Explicit(indices), Assignment::Group(assigned)) => {
                    indices.extend(assigned);
                }
                (
                    Selected::Counted(operands),
                    Assignment::Counted(operand) | Assignment::Projected { operand, .. },
                ) => {
                    operands.push(operand);
                }
                _ => {
                    return Ok(false);
                }
            }
            let continuation = Group {
                patterns: rest,
                values: group.values.clone(),
                selected,
                next: group.start,
                projection: 0,
                combination: None,
                sum: group.sum,
                start: group.start,
                root: group.root,
            };
            self.choices.push(Choice {
                checkpoint: self.undo.len(),
                retry: group,
            });
            if let PatternNode::Capture { name, .. } = template.node() {
                if let Some(slot) = self
                    .capture_names
                    .iter()
                    .position(|bound| Arc::ptr_eq(bound, name) || bound.as_ref() == name.as_ref())
                    && let Some(entry) = work.slots.get_mut(slot)
                    && entry.is_none()
                {
                    *entry = Some(value);
                    self.undo.push(Undo::Capture(slot));
                }
                self.undo.push(Undo::Pushed);
                self.pending.push(Task::Group(continuation));
            } else {
                // The new choice checkpoints both pushes before either task runs.
                self.undo.extend([Undo::Pushed, Undo::Pushed]);
                // Separate pushes write each task in place; an array of both
                // would be copied with a `memcpy` call.
                self.pending.push(Task::Group(continuation));
                self.pending.push(Task::Match(template, value));
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn expand(
        &mut self,
        pattern: &'pattern Pattern,
        input: &Expr,
        work: &mut MatchWork<'_>,
    ) -> bool {
        // Captures and exact templates are never cached, and a node reached
        // through one parent is expanded once per input, so its pairs never
        // repeat.
        let key = from_ref(pattern.node());
        let cacheable = !self.repeats.is_empty()
            && self.repeats.contains(&key.addr())
            && matches!(
                pattern.node(),
                PatternNode::Function { .. }
                    | PatternNode::Sum(_)
                    | PatternNode::Product(_)
                    | PatternNode::Div(..)
                    | PatternNode::Pow(..)
            );
        if cacheable && work.expanded.contains(&Expansion(key, input)) {
            return true;
        }
        let capture = if let PatternNode::Capture { name, .. } = pattern.node()
            && let Some(slot) = self
                .capture_names
                .iter()
                .position(|bound| Arc::ptr_eq(bound, name) || bound.as_ref() == name.as_ref())
            && work.slots.get(slot).is_some_and(Option::is_none)
        {
            Some(slot)
        } else {
            None
        };
        let Some(children) = PatternNode::match_node(
            pattern.node(),
            input,
            self.capture_names,
            &mut work.slots,
            true,
            None,
        ) else {
            return false;
        };
        let transactional = !self.choices.is_empty();
        if transactional && let Some(slot) = capture {
            self.undo.push(Undo::Capture(slot));
        }
        if cacheable {
            if transactional {
                self.undo.push(Undo::Expanded(key, input.clone()));
            }
            work.expanded.insert((key, input.clone()));
        }
        if matches!(
            pattern.node(),
            PatternNode::Sum(_) | PatternNode::Product(_)
        ) {
            let start = work.projected.len();
            let patterns = match children {
                MatchChildren::Many(patterns, values) => {
                    work.projected.extend_from_slice(values);
                    patterns
                }
                MatchChildren::Projected(patterns, values) => {
                    work.projected.extend(values);
                    patterns
                }
                MatchChildren::Done | MatchChildren::Pair(..) => return false,
            };
            if transactional {
                self.undo.extend([Undo::Projected(start), Undo::Pushed]);
            }
            self.pending.push(Task::Group(Group {
                patterns,
                values: GroupValues::Owned(start..work.projected.len()),
                selected: Selected::Explicit(Taken::default()),
                next: 0,
                projection: 0,
                combination: None,
                sum: matches!(pattern.node(), PatternNode::Sum(_)),
                start: 0,
                root: false,
            }));
        } else {
            children.append_with(
                |(template, value)| {
                    if transactional {
                        self.undo.push(Undo::Pushed);
                    }
                    self.pending.push(Task::Match(template, value.into_owned()));
                },
                Cow::Borrowed,
            );
        }
        true
    }

    /// The cursor from which `pattern` draws its only candidate when it is
    /// rigid and every capture it names is bound: the position of the
    /// available explicit operand equal to its instantiation, or past the end
    /// when there is none. `None` when the template is not rigid, not bound,
    /// or draws counted operands.
    fn ground(
        &self,
        pattern: &Pattern,
        group: &Group<'_, '_>,
        work: &mut MatchWork<'_>,
    ) -> Option<usize> {
        if self.grounds.is_empty() {
            return None;
        }
        let values: &[Expr] = match &group.values {
            GroupValues::Borrowed(values) => values,
            GroupValues::Owned(range) => work.projected.get(range.clone())?,
            GroupValues::Counted(_) => return None,
        };
        let Selected::Explicit(selected) = &group.selected else {
            return None;
        };
        let address = from_ref(pattern.node()).addr();
        let (_, template) = self.grounds.iter().find(|(rigid, _)| *rigid == address)?;
        let instance = template.instantiate(&work.slots, &mut work.values)?;
        Some(
            values
                .iter()
                .enumerate()
                .skip(group.next)
                .find(|(index, value)| **value == instance && !selected.contains(*index))
                .map_or(usize::MAX, |(index, _)| index),
        )
    }

    /// Reject a candidate before any choice is recorded. Function, quotient,
    /// and power templates require the same head as `PatternNode::match_node`,
    /// so a mismatched head would fail its expansion anyway.
    fn compatible(&self, pattern: &Pattern, value: &Expr, slots: &[Option<Expr>]) -> bool {
        match pattern.node() {
            PatternNode::Exact(expected) => expected == value,
            PatternNode::Capture { name, predicates } => {
                let Some(slot) = self
                    .capture_names
                    .iter()
                    .position(|bound| Arc::ptr_eq(bound, name) || bound.as_ref() == name.as_ref())
                else {
                    return false;
                };
                slots
                    .get(slot)
                    .and_then(Option::as_ref)
                    .is_none_or(|existing| existing == value)
                    && predicates.iter().all(|predicate| predicate(value))
            }
            PatternNode::Function { name, args } => matches!(
                value.view(),
                ExprView::Function { name: actual, args: values }
                    if name == actual && args.len() == values.len()
            ),
            PatternNode::Div(..) => matches!(value.view(), ExprView::Div(..)),
            PatternNode::Pow(..) => matches!(value.view(), ExprView::Pow(..)),
            PatternNode::Sum(_) | PatternNode::Product(_) => true,
        }
    }

    fn backtrack(&mut self, work: &mut MatchWork<'_>) -> bool {
        let Some(choice) = self.choices.pop() else {
            return false;
        };
        while self.undo.len() > choice.checkpoint {
            match self.undo.pop() {
                Some(Undo::Capture(slot)) => {
                    if let Some(entry) = work.slots.get_mut(slot) {
                        *entry = None;
                    }
                }
                Some(Undo::Expanded(template, input)) => {
                    work.expanded.remove(&(template, input));
                }
                Some(Undo::Pushed) => {
                    drop(self.pending.pop());
                }
                Some(Undo::Popped) => {
                    if let Some(task) = self.popped.pop() {
                        self.pending.push(task);
                    }
                }
                // Expansions append and are undone in reverse, so this drops
                // exactly the operands of the undone expansion.
                Some(Undo::Projected(length)) => work.projected.truncate(length),
                None => break,
            }
        }
        if !self.choices.is_empty() {
            self.undo.push(Undo::Pushed);
        }
        self.pending.push(Task::Group(choice.retry));
        true
    }
}
