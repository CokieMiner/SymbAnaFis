//! Ordered matching and logical operand constraints.

use core::ptr::from_ref;

use alloc::{borrow::Cow, vec, vec::Vec};

use crate::collections::{HashMap, HashSet};

use super::{
    AdditiveOperands, AlgebraicOperands, Expr, ExprView, MatchChildren, MatchInput, Pattern,
    PatternNode,
};

/// Match every template constraint, sharing captured expression handles.
impl PatternNode {
    pub fn match_ordered<'pattern>(
        pattern: &'pattern Pattern,
        expression: &Expr,
    ) -> Option<HashMap<&'pattern str, Expr>> {
        let mut captured: HashMap<&str, Expr> = HashMap::new();
        // A repeated template/input pair has the same structural result. The
        // expression handle is owned by this set so projected logical terms
        // cannot be reclaimed and have their address reused while matching.
        // Captures are monotonic: a successful expansion only adds bindings,
        // and later bindings can only validate or conflict with those names.
        let mut expanded = HashSet::new();
        let mut pending = vec![(pattern, MatchInput::Borrowed(expression))];
        while let Some((template, input)) = pending.pop() {
            let node = match &input {
                MatchInput::Borrowed(node) => *node,
                MatchInput::Owned(node) => node,
            };
            let template_node = template.node();
            let cacheable = matches!(
                template_node,
                Self::Function { .. }
                    | Self::Sum(_)
                    | Self::Product(_)
                    | Self::Div(..)
                    | Self::Pow(..)
            );
            if cacheable && expanded.contains(&(from_ref(template_node), node.clone())) {
                continue;
            }
            match input {
                MatchInput::Borrowed(borrowed) => {
                    let children = Self::match_node(template_node, borrowed, &mut captured, false)?;
                    if cacheable {
                        expanded.insert((from_ref(template_node), borrowed.clone()));
                    }
                    children.append_with(|item| pending.push(item), MatchInput::Borrowed);
                }
                MatchInput::Owned(owned) => {
                    let children = Self::match_node(template_node, &owned, &mut captured, false)?;
                    if cacheable {
                        expanded.insert((from_ref(template_node), owned.clone()));
                    }
                    children.append_with(
                        |item| pending.push(item),
                        |child| MatchInput::Owned(child.clone()),
                    );
                }
            }
        }
        Some(captured)
    }

    /// Walk each unique template node in children-before-parent order.
    pub fn template_order(root: &Pattern) -> Vec<&Pattern> {
        let mut visited = HashSet::new();
        let mut pending = vec![(root, false)];
        let mut result = Vec::new();
        while let Some((template, ready)) = pending.pop() {
            if ready {
                result.push(template);
            } else if visited.insert(from_ref(template.node())) {
                pending.push((template, true));
                pending.extend(Self::pattern_children(template).map(|child| (child, false)));
            }
        }
        result
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
                if commutative
                    && let Some((first, rest)) = args.split_first()
                    && !rest.is_empty()
                    && let Self::Exact(coefficient) = first.node()
                    && !matches!(node.view(), ExprView::Product(values) if values.first().is_some_and(|value|matches!(value.view(), ExprView::Number(_) | ExprView::ImaginaryUnit)))
                    && let Some(quotient) = AlgebraicOperands::divide_scalar(node, coefficient)
                {
                    let values = if let ExprView::Product(values) = quotient.view() {
                        values.to_vec()
                    } else {
                        vec![quotient]
                    };
                    return Some(MatchChildren::Projected(rest, values));
                }
                if let ExprView::Product(values) = node.view()
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

    /// Borrow template edges without allocating.
    fn pattern_children(pattern: &Pattern) -> impl Iterator<Item = &Pattern> {
        let (many, first, second): (&[Pattern], Option<&Pattern>, Option<&Pattern>) =
            match pattern.node() {
                Self::Capture { .. } | Self::Exact(_) => (&[], None, None),
                Self::Function { args, .. } | Self::Sum(args) | Self::Product(args) => {
                    (args, None, None)
                }
                Self::Div(left, right) | Self::Pow(left, right) => (&[], Some(left), Some(right)),
            };
        many.iter().chain(first).chain(second)
    }
}

impl<'pattern, 'expr> MatchChildren<'pattern, 'expr> {
    pub fn append_with<'input>(
        self,
        mut push: impl FnMut((&'pattern Pattern, MatchInput<'input>)),
        mut wrap: impl FnMut(&'expr Expr) -> MatchInput<'input>,
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
                    push((pattern, MatchInput::Owned(value)));
                }
            }
        }
    }
}
