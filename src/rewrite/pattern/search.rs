//! Iterative commutative search with transactional bindings and pending work.

use core::{mem::take, ptr::from_ref};

use alloc::{borrow::Cow, vec::Vec};

use crate::collections::{Entry, FixedSet, HashMap};

use super::{
    AlgebraicOperands, Assignment, BuiltinFunction, Choice, ExactIdentity, Expansion, Expr,
    ExprView, Group, GroupValues, MatchChildren, MatchResult, MatchWork, NonZeroU64, OperandDemand,
    Pattern, PatternNode, PowerTemplate, ReplacementProgram, RewriteError, Search, Selected, Task,
    Undo,
};

impl<'pattern, 'input> Search<'pattern, 'input> {
    pub fn new(grounds: &'pattern [(Pattern, ReplacementProgram)]) -> Self {
        Self {
            pending: Vec::new(),
            popped: Vec::new(),
            projected: Vec::new(),
            captured: HashMap::new(),
            expanded: FixedSet::default(),
            undo: Vec::new(),
            choices: Vec::new(),
            grounds,
        }
    }

    /// Search from `initial`. The initial task is not stored, so a search that
    /// fails before scheduling more work allocates nothing.
    pub fn run(
        &mut self,
        initial: Task<'pattern, 'input>,
        work: &mut MatchWork,
        guard: &mut impl FnMut(&HashMap<&str, Expr>) -> bool,
    ) -> Result<Option<MatchResult<'pattern>>, RewriteError> {
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
                    let matched = self.expand(pattern, &input);
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
                        } else if guard(&self.captured) {
                            return Ok(Some(MatchResult {
                                captures: take(&mut self.captured),
                                selected: group.selected,
                            }));
                        } else {
                            false
                        }
                    } else {
                        self.assign(group, work)?
                    }
                }
                None => {
                    if guard(&self.captured) {
                        return Ok(Some(MatchResult {
                            captures: take(&mut self.captured),
                            selected: Selected::Explicit(Vec::new()),
                        }));
                    }
                    false
                }
            };
            if !matched && !self.backtrack() {
                return Ok(None);
            }
        }
    }

    fn assign(
        &mut self,
        mut group: Group<'pattern, 'input>,
        work: &mut MatchWork,
    ) -> Result<bool, RewriteError> {
        let Some((pattern, rest)) = group.patterns.split_first() else {
            return Ok(false);
        };
        let draw = group.draw(pattern);
        let ground = self.ground(pattern, &group);
        if let Some(position) = ground {
            group.next = position;
        }
        while let Some((candidate, assignment)) = group.candidate(pattern, &draw, &self.projected) {
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
            if !self.compatible(template, &candidate) {
                continue;
            }
            // Only a compatible candidate is owned by the search.
            let value = candidate.into_owned();
            let mut selected = group.selected.clone();
            match (&mut selected, assignment) {
                (Selected::Explicit(indices), Assignment::Explicit(index)) => {
                    indices.push(index);
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
                if let Entry::Vacant(entry) = self.captured.entry(name.as_ref()) {
                    entry.insert(value);
                    self.undo.push(Undo::Capture(name));
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

    fn expand(&mut self, pattern: &'pattern Pattern, input: &Expr) -> bool {
        // Captures and exact templates are never cached.
        let key = from_ref(pattern.node());
        let cacheable = matches!(
            pattern.node(),
            PatternNode::Function { .. }
                | PatternNode::Sum(_)
                | PatternNode::Product(_)
                | PatternNode::Div(..)
                | PatternNode::Pow(..)
        );
        if cacheable && self.expanded.contains(&Expansion(key, input)) {
            return true;
        }
        let capture = if let PatternNode::Capture { name, .. } = pattern.node()
            && !self.captured.contains_key(name.as_ref())
        {
            Some(name.as_ref())
        } else {
            None
        };
        let Some(children) =
            PatternNode::match_node(pattern.node(), input, &mut self.captured, true)
        else {
            return false;
        };
        let transactional = !self.choices.is_empty();
        if transactional && let Some(name) = capture {
            self.undo.push(Undo::Capture(name));
        }
        if cacheable {
            if transactional {
                self.undo.push(Undo::Expanded(key, input.clone()));
            }
            self.expanded.insert((key, input.clone()));
        }
        if matches!(
            pattern.node(),
            PatternNode::Sum(_) | PatternNode::Product(_)
        ) {
            let start = self.projected.len();
            let patterns = match children {
                MatchChildren::Many(patterns, values) => {
                    self.projected.extend_from_slice(values);
                    patterns
                }
                MatchChildren::Projected(patterns, values) => {
                    self.projected.extend(values);
                    patterns
                }
                MatchChildren::Done | MatchChildren::Pair(..) => return false,
            };
            if transactional {
                self.undo.extend([Undo::Projected(start), Undo::Pushed]);
            }
            self.pending.push(Task::Group(Group {
                patterns,
                values: GroupValues::Owned(start..self.projected.len()),
                selected: Selected::Explicit(Vec::new()),
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
    ///
    /// A rigid template has no sum or product, whose operands match
    /// commutatively, and is not a bare capture, which may take a group of
    /// operands. It matches structurally through functions, quotients,
    /// powers, exact values, and bound captures, so the only operand it can
    /// match is the canonical expression it instantiates to, found by identity
    /// in one pass instead of one attempt per operand. Rigidity is a property
    /// of the template alone, so the rule compiles each rigid template once,
    /// and an unbound capture fails only its instantiation.
    fn ground(&self, pattern: &Pattern, group: &Group<'_, '_>) -> Option<usize> {
        let (_, template) = self
            .grounds
            .iter()
            .find(|(rigid, _)| from_ref(rigid.node()) == from_ref(pattern.node()))?;
        let values: &[Expr] = match &group.values {
            GroupValues::Borrowed(values) => values,
            GroupValues::Owned(range) => self.projected.get(range.clone())?,
            GroupValues::Counted(_) => return None,
        };
        let Selected::Explicit(selected) = &group.selected else {
            return None;
        };
        let instance = template.instantiate(&self.captured)?;
        Some(
            values
                .iter()
                .enumerate()
                .skip(group.next)
                .find(|(index, value)| **value == instance && !selected.contains(index))
                .map_or(usize::MAX, |(index, _)| index),
        )
    }

    /// Reject a candidate before any choice is recorded. Function, quotient,
    /// and power templates require the same head as `PatternNode::match_node`,
    /// so a mismatched head would fail its expansion anyway.
    fn compatible(&self, pattern: &Pattern, value: &Expr) -> bool {
        match pattern.node() {
            PatternNode::Exact(expected) => expected == value,
            PatternNode::Capture { name, predicates } => {
                self.captured
                    .get(name.as_ref())
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

    fn backtrack(&mut self) -> bool {
        let Some(choice) = self.choices.pop() else {
            return false;
        };
        while self.undo.len() > choice.checkpoint {
            match self.undo.pop() {
                Some(Undo::Capture(name)) => {
                    self.captured.remove(name);
                }
                Some(Undo::Expanded(template, input)) => {
                    self.expanded.remove(&(template, input));
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
                Some(Undo::Projected(length)) => self.projected.truncate(length),
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

impl PatternNode {
    /// Prove a fixed exponent or coefficient before projecting a virtual operand.
    pub fn operand_demand(&self, sum: bool, power: Option<&PowerTemplate<'_>>) -> OperandDemand {
        if let Some(source) = power {
            return if source.denominator == NonZeroU64::MIN {
                OperandDemand::Power(source.numerator)
            } else {
                OperandDemand::Rational {
                    numerator: source.numerator,
                    denominator: source.denominator,
                }
            };
        }
        match self {
            Self::Exact(value) => AlgebraicOperands::demand(value, sum),
            Self::Pow(..) if !sum => OperandDemand::Whole,
            Self::Product(factors) if sum => factors
                .first()
                .and_then(|factor| {
                    if let Self::Exact(value) = factor.node() {
                        AlgebraicOperands::integer(value).and_then(i64::checked_abs)
                    } else {
                        None
                    }
                })
                .filter(|count| *count > 0)
                .map_or(OperandDemand::Atom, OperandDemand::Copies),
            Self::Capture { .. }
            | Self::Function { .. }
            | Self::Sum(_)
            | Self::Product(_)
            | Self::Div(..)
            | Self::Pow(..) => OperandDemand::Atom,
        }
    }

    /// Expose only a fixed exact exponent, retaining the template's principal base.
    pub fn power_template(&self) -> Option<PowerTemplate<'_>> {
        let reciprocal = match self {
            Self::Div(unit, denominator) if matches!(unit.node(), Self::Exact(value) if value.is_exact_one()) => {
                Some(denominator)
            }
            Self::Capture { .. }
            | Self::Exact(_)
            | Self::Function { .. }
            | Self::Sum(_)
            | Self::Product(_)
            | Self::Div(..)
            | Self::Pow(..) => None,
        };
        let node = reciprocal.map_or(self, Pattern::node);
        if reciprocal.is_some()
            && matches!(node, Self::Div(unit, _) if matches!(unit.node(), Self::Exact(value) if value.is_exact_one()))
        {
            return None;
        }
        let projected = match node {
            Self::Pow(base, exponent) => {
                let ratio = if let Self::Exact(value) = exponent.node() {
                    AlgebraicOperands::ratio(value)
                } else {
                    None
                };
                ratio.map(|(numerator, denominator)| (base, numerator, denominator))
            }
            Self::Function { name, args } if *name == BuiltinFunction::Sqrt.function() => {
                let [base] = args.as_slice() else { return None };
                Some((base, 1, NonZeroU64::new(2)?))
            }
            Self::Capture { .. }
            | Self::Exact(_)
            | Self::Function { .. }
            | Self::Sum(_)
            | Self::Product(_)
            | Self::Div(..) => None,
        };
        let Some((base, signed, denominator)) = projected else {
            return reciprocal.map(|base| PowerTemplate {
                base,
                numerator: 1,
                denominator: NonZeroU64::MIN,
                inverse: true,
            });
        };
        if signed < 0 && reciprocal.is_some() {
            return None;
        }
        let numerator = signed.checked_abs()?;
        (numerator > 0).then_some(PowerTemplate {
            base,
            numerator,
            denominator,
            inverse: signed < 0 || reciprocal.is_some(),
        })
    }
}
