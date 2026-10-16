//! Lazy commutative partitions and positive-power decompositions.

use alloc::{vec, vec::Vec};

use super::{Assignment, Expr, Group, GroupValues, OperandDemand, Pattern, PatternNode, Selected};

impl Group<'_, '_> {
    /// Try individual operands, counted blocks, then grouped capture partitions.
    pub fn candidate(&mut self, pattern: &Pattern) -> Option<(Expr, Assignment)> {
        if !self.root
            && self.patterns.len() == 1
            && matches!(pattern.node(), PatternNode::Capture(_))
        {
            return self.grouped(true);
        }
        let count = match &self.values {
            GroupValues::Borrowed(values) => values.len(),
            GroupValues::Owned(values) => values.len(),
            GroupValues::Counted(values) => values.originals().len(),
        };
        while self.next < count {
            let index = self.next;
            if let GroupValues::Counted(values) = &self.values {
                let basic = pattern.node().operand_demand(self.sum);
                let multiple = matches!(pattern.node(), PatternNode::Capture(_))
                    || matches!(pattern.node(), PatternNode::Pow(..))
                        && matches!(basic, OperandDemand::Power(_));
                let demand = match (pattern.node(), basic) {
                    (PatternNode::Capture(_), _) if self.projection == 0 => OperandDemand::Whole,
                    (PatternNode::Capture(_), _) => OperandDemand::Split(self.projection),
                    (PatternNode::Pow(..), OperandDemand::Power(exponent)) => OperandDemand::Root {
                        exponent,
                        block: self.projection.saturating_add(1),
                    },
                    _ => basic,
                };
                let literal = if let PatternNode::Exact(value) = pattern.node() {
                    Some(value)
                } else {
                    None
                };
                let projected = if let Selected::Counted(used) = &self.selected {
                    values.candidate(index, demand, literal, used.iter(), false)
                } else {
                    None
                };
                if let Some((value, consumed)) = projected {
                    if multiple {
                        self.projection = self.projection.saturating_add(1);
                    } else {
                        self.next = index.saturating_add(1);
                    }
                    return Some((value, Assignment::Counted(consumed)));
                }
                self.next = index.saturating_add(1);
                self.projection = 0;
            } else {
                self.next = index.saturating_add(1);
                let values = match &self.values {
                    GroupValues::Borrowed(values) => *values,
                    GroupValues::Owned(values) => values.as_ref(),
                    GroupValues::Counted(_) => return None,
                };
                if let Some((value, true)) = values.get(index)
                    && matches!(&self.selected, Selected::Explicit(indices) if !indices.contains(&index))
                {
                    return Some((value.clone(), Assignment::Explicit(index)));
                }
            }
        }
        if matches!(pattern.node(), PatternNode::Capture(_)) {
            self.grouped(false)
        } else {
            None
        }
    }

    fn grouped(&mut self, complete: bool) -> Option<(Expr, Assignment)> {
        let values = match &self.values {
            GroupValues::Borrowed(values) => *values,
            GroupValues::Owned(values) => values.as_ref(),
            GroupValues::Counted(_) => return None,
        };
        let Selected::Explicit(used) = &self.selected else {
            return None;
        };
        let available: Vec<_> = values
            .iter()
            .enumerate()
            .filter_map(|(index, (_, keep))| (*keep && !used.contains(&index)).then_some(index))
            .collect();
        if available.is_empty() {
            return None;
        }
        let indices = if complete {
            if self.combination.is_some() {
                return None;
            }
            self.combination = Some(Vec::new());
            available
        } else {
            let max = available
                .len()
                .saturating_sub(self.patterns.len().saturating_sub(1));
            if max < 2 {
                return None;
            }
            let ranks = self.combination.get_or_insert_with(|| vec![0, 1]);
            if ranks.is_empty() {
                return None;
            }
            let indices = ranks
                .iter()
                .map(|rank| available.get(*rank).copied())
                .collect::<Option<Vec<_>>>()?;
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
                if length <= max {
                    *ranks = (0..length).collect();
                } else {
                    ranks.clear();
                }
            }
            indices
        };
        let operands = indices
            .iter()
            .filter_map(|index| values.get(*index).map(|(value, _)| value.clone()))
            .collect();
        let value = if self.sum {
            Expr::sum(operands)
        } else {
            Expr::product(operands)
        };
        Some((value, Assignment::Group(indices)))
    }
}
