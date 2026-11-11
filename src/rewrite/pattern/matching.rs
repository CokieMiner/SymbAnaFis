//! Ordered matching, logical operand constraints, lazy partitions, and exact power decompositions.

use core::ptr::from_ref;

use alloc::{borrow::Cow, boxed::Box, vec, vec::Vec};

use crate::collections::{Equivalent, FixedSet, HashMap};

use super::{
    AdditiveOperands, AlgebraicOperands, Assignment, Draw, Expansion, Expr, ExprView, Group,
    GroupValues, MatchChildren, OperandDemand, Pattern, PatternNode, Selected,
};

/// The borrowed key hashes its fields in tuple order, so it finds the owned
/// pair it equals.
impl Equivalent<(*const PatternNode, Expr)> for Expansion<'_> {
    fn equivalent(&self, key: &(*const PatternNode, Expr)) -> bool {
        self.0 == key.0 && *self.1 == key.1
    }
}

/// Match every template constraint, sharing captured expression handles.
impl PatternNode {
    pub fn match_ordered<'pattern>(
        pattern: &'pattern Pattern,
        expression: &Expr,
    ) -> Option<HashMap<&'pattern str, Expr>> {
        let mut captured: HashMap<&str, Expr> = HashMap::new();
        // The root is matched before any scratch storage is allocated, so a
        // mismatched root allocates nothing. No template node is its own
        // descendant, so the root pair is never met again and is not cached.
        let mut pending = Vec::new();
        Self::match_node(pattern.node(), expression, &mut captured, false)?
            .append_with(|item| pending.push(item), Cow::Borrowed);
        // A repeated template/input pair has the same structural result. The
        // expression handle is owned by this set so projected logical terms
        // cannot be reclaimed and have their address reused while matching.
        // Captures are monotonic: a successful expansion only adds bindings,
        // and later bindings can only validate or conflict with those names.
        let mut expanded = FixedSet::default();
        while let Some((template, input)) = pending.pop() {
            let node: &Expr = &input;
            let template_node = template.node();
            let cacheable = matches!(
                template_node,
                Self::Function { .. }
                    | Self::Sum(_)
                    | Self::Product(_)
                    | Self::Div(..)
                    | Self::Pow(..)
            );
            if cacheable && expanded.contains(&Expansion(from_ref(template_node), node)) {
                continue;
            }
            match input {
                Cow::Borrowed(borrowed) => {
                    let children = Self::match_node(template_node, borrowed, &mut captured, false)?;
                    if cacheable {
                        expanded.insert((from_ref(template_node), borrowed.clone()));
                    }
                    children.append_with(|item| pending.push(item), Cow::Borrowed);
                }
                Cow::Owned(owned) => {
                    let children = Self::match_node(template_node, &owned, &mut captured, false)?;
                    if cacheable {
                        expanded.insert((from_ref(template_node), owned.clone()));
                    }
                    children
                        .append_with(|item| pending.push(item), |child| Cow::Owned(child.clone()));
                }
            }
        }
        Some(captured)
    }

    pub fn match_node<'pattern, 'expr>(
        template: &'pattern Self,
        node: &'expr Expr,
        captured: &mut HashMap<&'pattern str, Expr>,
        commutative: bool,
    ) -> Option<MatchChildren<'pattern, 'expr>> {
        match template {
            Self::Capture { name, predicates } => {
                if let Some(existing) = captured.get(name.as_ref())
                    && existing != node
                {
                    return None;
                }
                if !predicates.iter().all(|predicate| predicate(node)) {
                    return None;
                }
                captured.entry(name).or_insert_with(|| node.clone());
                Some(MatchChildren::Done)
            }
            Self::Exact(expected) => (expected == node).then_some(MatchChildren::Done),
            Self::Function { name, args } => {
                if let ExprView::Function {
                    name: actual,
                    args: values,
                } = node.view()
                    && name == actual
                    && args.len() == values.len()
                {
                    Some(MatchChildren::Many(args, values))
                } else {
                    None
                }
            }
            Self::Sum(args) => {
                let terms = node.additive_operands()?;
                if args.len() != terms.len() && !(commutative && args.len() <= terms.len()) {
                    return None;
                }
                match terms {
                    Cow::Borrowed(values) => Some(MatchChildren::Many(args, values)),
                    Cow::Owned(values) => Some(MatchChildren::Projected(args, values)),
                }
            }
            Self::Product(args) => {
                let factors = if let ExprView::Product(values) = node.view() {
                    Some(values)
                } else {
                    None
                };
                if commutative
                    && let Some((first, rest)) = args.split_first()
                    && !rest.is_empty()
                    && let Self::Exact(coefficient) = first.node()
                    && !factors.and_then(<[Expr]>::first).is_some_and(|value| {
                        matches!(value.view(), ExprView::Number(_) | ExprView::ImaginaryUnit)
                    })
                    && let Some(quotient) = AlgebraicOperands::divide_scalar(node, coefficient)
                {
                    let values = if let ExprView::Product(values) = quotient.view() {
                        values.to_vec()
                    } else {
                        vec![quotient]
                    };
                    return Some(MatchChildren::Projected(rest, values));
                }
                if let Some(values) = factors
                    && (args.len() == values.len() || commutative && args.len() <= values.len())
                {
                    Some(MatchChildren::Many(args, values))
                } else {
                    None
                }
            }
            Self::Div(left, right) => {
                if let ExprView::Div(a, b) = node.view() {
                    Some(MatchChildren::Pair((left, a), (right, b)))
                } else {
                    None
                }
            }
            Self::Pow(base, exponent) => {
                if let ExprView::Pow(a, b) = node.view() {
                    Some(MatchChildren::Pair((base, a), (exponent, b)))
                } else {
                    None
                }
            }
        }
    }
}

impl<'pattern, 'expr> MatchChildren<'pattern, 'expr> {
    pub fn append_with<'input>(
        self,
        mut push: impl FnMut((&'pattern Pattern, Cow<'input, Expr>)),
        mut wrap: impl FnMut(&'expr Expr) -> Cow<'input, Expr>,
    ) {
        match self {
            Self::Done => {}
            Self::Many(patterns, values) => {
                for (pattern, value) in patterns.iter().zip(values) {
                    push((pattern, wrap(value)));
                }
            }
            Self::Pair((left, a), (right, b)) => {
                push((left, wrap(a)));
                push((right, wrap(b)));
            }
            Self::Projected(patterns, values) => {
                for (pattern, value) in patterns.iter().zip(values) {
                    push((pattern, Cow::Owned(value)));
                }
            }
        }
    }
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
        }
    }

    /// Try individual operands, counted blocks, then grouped capture partitions.
    /// An existing operand is lent, so a rejected candidate costs no count.
    /// `draw` is `self.draw(pattern)`, and `appended` holds the operands that
    /// expansions appended for nested templates.
    pub fn candidate<'pattern, 'values>(
        &mut self,
        pattern: &'pattern Pattern,
        draw: &Draw<'pattern>,
        appended: &'values [Expr],
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
                    && matches!(&self.selected, Selected::Explicit(indices) if !indices.contains(&index))
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
        let available: Vec<_> = (0..values.len())
            .filter(|index| !used.contains(index))
            .collect();
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
                *ranks = if length <= max {
                    (0..length).collect()
                } else {
                    Box::default()
                };
            }
            indices
        };
        let operands = indices
            .iter()
            .filter_map(|index| values.get(*index).cloned())
            .collect();
        let value = if self.sum {
            Expr::sum(operands)
        } else {
            Expr::product(operands)
        };
        Some((Cow::Owned(value), Assignment::Group(indices)))
    }
}
