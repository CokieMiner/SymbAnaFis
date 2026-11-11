//! Commutative operand groups, candidate generation, and lazy capture partitions.

use core::ops::Range;

use alloc::{borrow::Cow, boxed::Box, vec::Vec};

use super::{
    AlgebraicOperands, CountedSlot, Expr, Heads, OperandDemand, OperandMatch, Pattern, PatternNode,
    PowerTemplate, Taken,
};

/// The operands a group has taken: explicit positions, or counted units.
#[derive(Clone, Debug)]
pub enum Selected {
    Explicit(Taken),
    Counted(Vec<OperandMatch>),
}

#[derive(Clone, Debug)]
pub enum Task<'pattern, 'input> {
    Match(&'pattern Pattern, Expr),
    Group(Group<'pattern, 'input>),
}

#[derive(Clone, Debug)]
pub struct Group<'pattern, 'input> {
    pub patterns: &'pattern [Pattern],
    pub values: GroupValues<'input>,
    pub selected: Selected,
    pub next: usize,
    pub projection: i64,
    /// Ranks of the next grouped partition; boxed so a choice is at most
    /// 128 bytes and moves without a `memcpy` call.
    pub combination: Option<Box<[usize]>>,
    pub sum: bool,
    pub start: usize,
    pub root: bool,
}

#[derive(Clone, Debug)]
pub enum GroupValues<'input> {
    /// Original operands; those consumed by earlier matches start selected.
    Borrowed(&'input [Expr]),
    /// Operands an expansion appended to `MatchWork::projected`.
    Owned(Range<usize>),
    Counted(&'input AlgebraicOperands),
}

pub enum Assignment<'pattern> {
    Explicit(usize),
    Group(Vec<usize>),
    Counted(OperandMatch),
    Projected {
        operand: OperandMatch,
        base: &'pattern Pattern,
    },
}

/// How a template draws operands from a group, fixed while one assignment
/// tries its candidates.
pub struct Draw<'pattern> {
    power: Option<PowerTemplate<'pattern>>,
    basic: OperandDemand,
    /// Whether one operand can supply several candidates in turn.
    multiple: bool,
    /// The heads an operand must have to be worth a candidate.
    heads: Heads,
    /// The heads that must occur among the factors of a candidate.
    factors: Heads,
}

impl<'input> Group<'_, 'input> {
    /// How `pattern` draws operands from this group. A counted product group
    /// decomposes powers for an exact power template.
    pub fn draw<'pattern>(&self, pattern: &'pattern Pattern) -> Draw<'pattern> {
        let power = if !self.sum && matches!(self.values, GroupValues::Counted(_)) {
            pattern.node().power_template()
        } else {
            None
        };
        let basic = pattern.node().operand_demand(self.sum, power.as_ref());
        Draw {
            multiple: matches!(pattern.node(), PatternNode::Capture { .. })
                || power.is_some() && matches!(basic, OperandDemand::Power(_)),
            power,
            basic,
            heads: pattern.node().operand_heads(),
            factors: pattern.node().required_factors(),
        }
    }

    /// Try individual operands, counted blocks, then grouped capture partitions.
    /// An existing operand is lent, so a rejected candidate costs no count.
    /// `draw` is `self.draw(pattern)`, `appended` holds the operands that
    /// expansions appended for nested templates, and `slot_heads` holds the
    /// head set of each counted operand when the caller built them.
    pub fn candidate<'pattern, 'values>(
        &mut self,
        pattern: &'pattern Pattern,
        draw: &Draw<'pattern>,
        appended: &'values [Expr],
        slot_heads: Option<&[CountedSlot]>,
    ) -> Option<(Cow<'values, Expr>, Assignment<'pattern>)>
    where
        'input: 'values,
    {
        // Ranges name appended operands that backtracking has not truncated,
        // so the fallback is never taken.
        let explicit: &'values [Expr] = match &self.values {
            GroupValues::Borrowed(values) => values,
            GroupValues::Owned(range) => appended.get(range.clone()).unwrap_or_default(),
            GroupValues::Counted(_) => &[],
        };
        if !self.root
            && self.patterns.len() == 1
            && matches!(pattern.node(), PatternNode::Capture { .. })
        {
            return self.grouped(true, explicit);
        }
        let count = match &self.values {
            GroupValues::Counted(values) => values.originals().len(),
            GroupValues::Borrowed(_) | GroupValues::Owned(_) => explicit.len(),
        };
        let Draw {
            power,
            basic,
            multiple,
            heads,
            factors,
        } = draw;
        let literal = if let PatternNode::Exact(value) = pattern.node() {
            Some(value)
        } else {
            None
        };
        while self.next < count {
            let index = self.next;
            if let GroupValues::Counted(values) = &self.values {
                let demand = match (pattern.node(), *basic) {
                    (PatternNode::Capture { .. }, _) if self.projection == 0 => {
                        OperandDemand::Whole
                    }
                    (PatternNode::Capture { .. }, _) => OperandDemand::Split(self.projection),
                    (_, OperandDemand::Power(exponent)) if power.is_some() => {
                        let Some(block) = self.projection.checked_add(1) else {
                            self.next = index.saturating_add(1);
                            self.projection = 0;
                            continue;
                        };
                        OperandDemand::Root { exponent, block }
                    }
                    _ => *basic,
                };
                // A candidate the pattern cannot accept is rejected by
                // `compatible` after it is built, and building it interns a
                // coefficient and a base for nothing. The slot's head sets
                // decide the same rejection before the value exists, so a wide
                // polynomial costs two bit tests per operand per rule.
                if !heads.accepts(Heads::PRODUCT) && values.builds_product(index, demand)
                    || slot_heads
                        .and_then(|sets| sets.get(index))
                        .is_some_and(|slot| {
                            !heads.accepts(slot.value) || !slot.factors.covers(*factors)
                        })
                {
                    self.next = index.saturating_add(1);
                    self.projection = 0;
                    continue;
                }
                let projected = if let Selected::Counted(used) = &self.selected {
                    values.candidate(
                        index,
                        demand,
                        literal,
                        used.iter(),
                        power.as_ref().is_some_and(|source| source.inverse),
                    )
                } else {
                    None
                };
                if let Some((value, consumed)) = projected {
                    if *multiple {
                        self.projection = self.projection.saturating_add(1);
                    } else {
                        self.next = index.saturating_add(1);
                    }
                    let assignment = if let Some(source) = power {
                        Assignment::Projected {
                            operand: consumed,
                            base: source.base,
                        }
                    } else {
                        Assignment::Counted(consumed)
                    };
                    return Some((Cow::Owned(value), assignment));
                }
                self.next = index.saturating_add(1);
                self.projection = 0;
            } else {
                self.next = index.saturating_add(1);
                if let Some(value) = explicit.get(index)
                    && heads.accepts(Heads::of(value))
                    && matches!(&self.selected, Selected::Explicit(indices) if !indices.contains(index))
                {
                    return Some((Cow::Borrowed(value), Assignment::Explicit(index)));
                }
            }
        }
        if matches!(pattern.node(), PatternNode::Capture { .. }) {
            self.grouped(false, explicit)
        } else {
            None
        }
    }

    /// A capture partition of the explicit operands `values`; counted groups
    /// select units, not partitions.
    fn grouped<'values>(
        &mut self,
        complete: bool,
        values: &'values [Expr],
    ) -> Option<(Cow<'values, Expr>, Assignment<'static>)> {
        let Selected::Explicit(used) = &self.selected else {
            return None;
        };
        // The exact length is known, so the vector never grows: a growth path
        // would reload the set's pointer and length on every push.
        let mut available = Vec::with_capacity(values.len());
        for index in 0..values.len() {
            if !used.contains(index) {
                available.push(index);
            }
        }
        if available.is_empty() {
            return None;
        }
        let indices = if complete {
            if self.combination.is_some() {
                return None;
            }
            self.combination = Some(Box::default());
            available
        } else {
            let max = available
                .len()
                .saturating_sub(self.patterns.len().saturating_sub(1));
            if max < 2 {
                return None;
            }
            let ranks = self.combination.get_or_insert_with(|| Box::new([0, 1]));
            if ranks.is_empty() {
                return None;
            }
            let mut indices = Vec::with_capacity(ranks.len());
            for rank in ranks.iter() {
                indices.push(available.get(*rank).copied()?);
            }
            let mut advanced = false;
            for position in (0..ranks.len()).rev() {
                let limit = available
                    .len()
                    .saturating_sub(ranks.len().saturating_sub(position));
                if let Some(rank) = ranks.get_mut(position)
                    && *rank < limit
                {
                    *rank = rank.saturating_add(1);
                    let mut next = *rank;
                    for tail in ranks.iter_mut().skip(position.saturating_add(1)) {
                        next = next.saturating_add(1);
                        *tail = next;
                    }
                    advanced = true;
                    break;
                }
            }
            if !advanced {
                let length = ranks.len().saturating_add(1);
                *ranks = if length <= max {
                    (0..length).collect()
                } else {
                    Box::default()
                };
            }
            indices
        };
        // Built in place: a `collect` temporary would be copied into the
        // argument slot with a `memcpy` call.
        let mut operands = Vec::with_capacity(indices.len());
        for &index in &indices {
            if let Some(value) = values.get(index) {
                operands.push(value.clone());
            }
        }
        let value = if self.sum {
            Expr::sum(operands)
        } else {
            Expr::product(operands)
        };
        Some((Cow::Owned(value), Assignment::Group(indices)))
    }
}
