//! Iterative commutative search with transactional bindings and pending work.

use core::ptr::from_ref;

use alloc::vec::Vec;

use hashbrown::hash_map::Entry;

use crate::collections::{HashMap, HashSet};

use super::{
    AlgebraicOperands, Assignment, BuiltinFunction, Choice, Expr, Group, GroupValues,
    MatchChildren, MatchInput, MatchResult, MatchWork, NonZeroU64, OperandDemand, Pattern,
    PatternNode, PowerTemplate, RewriteError, Search, Selected, Task, Undo,
};

impl<'pattern, 'input> Search<'pattern, 'input> {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            captured: HashMap::new(),
            expanded: HashSet::new(),
            undo: Vec::new(),
            choices: Vec::new(),
        }
    }

    pub fn run(
        mut self,
        work: &mut MatchWork,
        guard: &mut impl FnMut(&HashMap<&str, Expr>) -> bool,
    ) -> Result<Option<MatchResult<'pattern>>, RewriteError> {
        loop {
            let task = self.pending.pop();
            if !self.choices.is_empty()
                && let Some(pending) = &task
            {
                self.undo.push(Undo::Popped(pending.clone()));
            }
            let matched = match task {
                Some(Task::Match(pattern, input)) => self.expand(pattern, &input),
                Some(Task::Group(group)) => {
                    if group.patterns.is_empty() {
                        if !group.root {
                            let complete = match (&group.values, &group.selected) {
                                (GroupValues::Owned(values), Selected::Explicit(indices)) => {
                                    indices.len() == values.iter().filter(|(_, keep)| *keep).count()
                                }
                                (GroupValues::Borrowed(values), Selected::Explicit(indices)) => {
                                    indices.len() == values.iter().filter(|(_, keep)| *keep).count()
                                }
                                _ => false,
                            };
                            if complete {
                                continue;
                            }
                            false
                        } else if guard(&self.captured) {
                            return Ok(Some(MatchResult {
                                captures: self.captured,
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
                            captures: self.captured,
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
        while let Some((value, assignment)) = group.candidate(pattern) {
            if let Some(remaining) = &mut work.remaining {
                *remaining = remaining
                    .checked_sub(1)
                    .ok_or(RewriteError::MatchLimitExceeded { limit: work.limit })?;
            }
            let template = match &assignment {
                Assignment::Projected { base, .. } => *base,
                Assignment::Explicit(_) | Assignment::Group(_) | Assignment::Counted(_) => pattern,
            };
            if !self.compatible(template, &value) {
                continue;
            }
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
                self.pending
                    .extend([Task::Group(continuation), Task::Match(template, value)]);
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn expand(&mut self, pattern: &'pattern Pattern, input: &Expr) -> bool {
        let cacheable = matches!(
            pattern.node(),
            PatternNode::Function { .. }
                | PatternNode::Sum(_)
                | PatternNode::Product(_)
                | PatternNode::Div(..)
                | PatternNode::Pow(..)
        );
        let key = (from_ref(pattern.node()), input.clone());
        if cacheable && self.expanded.contains(&key) {
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
            self.expanded.insert(key.clone());
            if transactional {
                self.undo.push(Undo::Expanded(key.0, key.1));
            }
        }
        if matches!(
            pattern.node(),
            PatternNode::Sum(_) | PatternNode::Product(_)
        ) {
            let (patterns, values) = match children {
                MatchChildren::Many(patterns, values) => (
                    patterns,
                    values.iter().cloned().map(|value| (value, true)).collect(),
                ),
                MatchChildren::Projected(patterns, values) => (
                    patterns,
                    values.into_iter().map(|value| (value, true)).collect(),
                ),
                MatchChildren::Done | MatchChildren::Pair(..) => return false,
            };
            if transactional {
                self.undo.push(Undo::Pushed);
            }
            self.pending.push(Task::Group(Group {
                patterns,
                values: GroupValues::Owned(values),
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
                    let child = match value {
                        MatchInput::Borrowed(node) => node.clone(),
                        MatchInput::Owned(node) => node,
                    };
                    if transactional {
                        self.undo.push(Undo::Pushed);
                    }
                    self.pending.push(Task::Match(template, child));
                },
                |node| MatchInput::Owned(node.clone()),
            );
        }
        true
    }

    fn compatible(&self, pattern: &Pattern, value: &Expr) -> bool {
        match pattern.node() {
            PatternNode::Exact(expected) => expected == value,
            PatternNode::Capture { name, predicates } => {
                self.captured
                    .get(name.as_ref())
                    .is_none_or(|existing| existing == value)
                    && predicates.iter().all(|predicate| predicate(value))
            }
            PatternNode::Function { .. }
            | PatternNode::Sum(_)
            | PatternNode::Product(_)
            | PatternNode::Div(..)
            | PatternNode::Pow(..) => true,
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
                Some(Undo::Popped(task)) => self.pending.push(task),
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
            Self::Div(unit, denominator) if matches!(unit.node(), Self::Exact(value) if value.is_one_num()) => {
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
            && matches!(node, Self::Div(unit, _) if matches!(unit.node(), Self::Exact(value) if value.is_one_num()))
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
