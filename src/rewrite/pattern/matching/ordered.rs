//! Ordered structural matching and single-node template expansion.

use core::ptr::from_ref;

use alloc::{borrow::Cow, sync::Arc, vec, vec::Vec};

use crate::collections::{Equivalent, FixedSet};

use super::{AdditiveOperands, AlgebraicOperands, Expr, ExprView, MatchWork, Pattern, PatternNode};

#[derive(Debug)]
pub enum MatchChildren<'pattern, 'expr> {
    Done,
    Many(&'pattern [Pattern], &'expr [Expr]),
    Pair(
        (&'pattern Pattern, &'expr Expr),
        (&'pattern Pattern, &'expr Expr),
    ),
    Projected(&'pattern [Pattern], Vec<Expr>),
}

/// A borrowed `(template, input)` expansion key, so a lookup takes no handle.
#[derive(Hash)]
pub struct Expansion<'input>(pub *const PatternNode, pub &'input Expr);

/// The borrowed key hashes its fields in tuple order, so it finds the owned
/// pair it equals.
impl Equivalent<(*const PatternNode, Expr)> for Expansion<'_> {
    fn equivalent(&self, key: &(*const PatternNode, Expr)) -> bool {
        self.0 == key.0 && *self.1 == key.1
    }
}

impl PatternNode {
    /// Match the template against the whole node, in order, storing bindings in
    /// `work.slots`.
    ///
    /// `projection` is the node's ordered operands when the caller already
    /// holds them. A sum with a compact group projects its terms on every call,
    /// so a caller that tries several rules at one node passes the projection
    /// it built once instead of rebuilding it per rule.
    pub fn match_ordered<'pattern, 'expr>(
        pattern: &'pattern Pattern,
        expression: &'expr Expr,
        projection: Option<&'expr [Expr]>,
        names: &[Arc<str>],
        repeats: &FixedSet<usize>,
        work: &mut MatchWork<'_>,
    ) -> bool {
        let slots = &mut work.slots;
        slots.clear();
        slots.resize(names.len(), None);
        // The root is matched before any scratch storage is allocated, so a
        // mismatched root allocates nothing. No template node is its own
        // descendant, so the root pair is never met again and is not cached.
        let Some(root) =
            Self::match_node(pattern.node(), expression, names, slots, false, projection)
        else {
            return false;
        };
        if matches!(root, MatchChildren::Done) {
            return true;
        }
        // Small templates fit on the stack without allocating a pending `Vec`.
        let mut stack: [Option<(&'pattern Pattern, Cow<'expr, Expr>)>; 8] = [const { None }; 8];
        let mut depth = 0_usize;
        let mut overflow: Vec<(&'pattern Pattern, Cow<'expr, Expr>)> = Vec::new();
        root.append_with(
            |item| {
                if let Some(slot) = stack.get_mut(depth) {
                    *slot = Some(item);
                    depth = depth.saturating_add(1);
                } else {
                    overflow.push(item);
                }
            },
            Cow::Borrowed,
        );
        // Only DAG patterns with shared template nodes populate `work.expanded`.
        let track_repeats = !repeats.is_empty();
        let expanded = &mut work.expanded;
        if track_repeats {
            expanded.clear();
        }
        while let Some((template, input)) = overflow.pop().or_else(|| {
            depth = depth.checked_sub(1)?;
            stack.get_mut(depth).and_then(Option::take)
        }) {
            let node = template.node();
            let cacheable = track_repeats
                && repeats.contains(&from_ref(node).addr())
                && matches!(
                    node,
                    Self::Function { .. }
                        | Self::Sum(_)
                        | Self::Product(_)
                        | Self::Div(..)
                        | Self::Pow(..)
                );
            if cacheable && expanded.contains(&Expansion(from_ref(node), &input)) {
                continue;
            }
            let mut push = |item: (&'pattern Pattern, Cow<'expr, Expr>)| {
                if let Some(slot) = stack.get_mut(depth) {
                    *slot = Some(item);
                    depth = depth.saturating_add(1);
                } else {
                    overflow.push(item);
                }
            };
            match input {
                Cow::Borrowed(borrowed) => {
                    let Some(children) =
                        Self::match_node(node, borrowed, names, slots, false, None)
                    else {
                        return false;
                    };
                    if cacheable {
                        expanded.insert((from_ref(node), borrowed.clone()));
                    }
                    children.append_with(&mut push, Cow::Borrowed);
                }
                Cow::Owned(owned) => {
                    let Some(children) = Self::match_node(node, &owned, names, slots, false, None)
                    else {
                        return false;
                    };
                    if cacheable {
                        expanded.insert((from_ref(node), owned.clone()));
                    }
                    children.append_with(&mut push, |child| Cow::Owned(child.clone()));
                }
            }
        }
        true
    }

    pub fn match_node<'pattern, 'expr>(
        template: &'pattern Self,
        node: &'expr Expr,
        capture_names: &[Arc<str>],
        slots: &mut [Option<Expr>],
        commutative: bool,
        projection: Option<&'expr [Expr]>,
    ) -> Option<MatchChildren<'pattern, 'expr>> {
        match template {
            Self::Capture { name, predicates } => {
                let slot = capture_names.iter().position(|bound| {
                    Arc::ptr_eq(bound, name) || bound.as_ref() == name.as_ref()
                })?;
                let entry = slots.get_mut(slot)?;
                if let Some(existing) = entry.as_ref()
                    && existing != node
                {
                    return None;
                }
                if !predicates.iter().all(|predicate| predicate(node)) {
                    return None;
                }
                if entry.is_none() {
                    *entry = Some(node.clone());
                }
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
                let terms = match projection {
                    Some(values) => Cow::Borrowed(values),
                    None => node.additive_operands()?,
                };
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
