//! The stored form of a node: borrowed edges and compact monomials,
//! reachability, and rebuilding.

use core::{iter::FusedIterator, num::NonZeroU64, slice::Iter};

use alloc::{vec, vec::Vec};

use crate::collections::FixedSet;

use super::{ExactIdentity, Expr, FunctionDefinition, Node, StoredForm};

/// The stored edges of one node: its operand vector, its binary operands, or
/// the bases and then the coefficients of a compact polynomial.
#[derive(Clone, Default)]
pub struct Edges<'expr> {
    operands: Iter<'expr, Expr>,
    first: Option<&'expr Expr>,
    second: Option<&'expr Expr>,
    coefficients: Iter<'expr, Expr>,
}

impl Node {
    /// Whether the logical children of `expression` are the terms of compact
    /// polynomial storage rather than its stored edges.
    pub fn projects(expression: &Expr) -> bool {
        match expression.node() {
            Self::Poly(_) => true,
            Self::Sum(terms) => Self::compact_group(terms).is_some(),
            Self::Number(_)
            | Self::ImaginaryUnit
            | Self::Pi
            | Self::Euler
            | Self::Symbol(_)
            | Self::Function { .. }
            | Self::Product(_)
            | Self::Div(..)
            | Self::Pow(..)
            | Self::Derivative { .. } => false,
        }
    }

    /// Strict descendants in postorder, each distinct node once; the root is excluded.
    pub fn descendants(root: &Expr) -> Vec<&Expr> {
        let mut visited = FixedSet::default();
        let mut pending: Vec<_> = root.stored_children().map(|child| (child, false)).collect();
        let mut result = Vec::new();
        while let Some((expression, ready)) = pending.pop() {
            if ready {
                result.push(expression);
            } else if visited.insert(expression) {
                pending.push((expression, true));
                pending.extend(expression.stored_children().map(|child| (child, false)));
            }
        }
        result
    }

    /// Whether a distinct node satisfies `predicate`, stopping at the first match.
    pub fn any(root: &Expr, mut predicate: impl FnMut(&Expr) -> bool) -> bool {
        let mut visited = FixedSet::default();
        let mut pending = vec![root];
        while let Some(expression) = pending.pop() {
            if visited.insert(expression) {
                if predicate(expression) {
                    return true;
                }
                pending.extend(expression.stored_children());
            }
        }
        false
    }

    /// Rebuild `original` from its mapped children, reading each child once.
    /// `get` receives a child's ordinal among the stored edges, in
    /// `stored_children` order, and the child itself. A node whose children all
    /// map to themselves keeps its pointer.
    pub fn rebuild_with<'mapped>(
        original: &Expr,
        get: impl Fn(usize, &Expr) -> &'mapped Expr,
    ) -> Expr {
        let unchanged = || original.clone();
        match original.node() {
            Self::Number(_) | Self::ImaginaryUnit | Self::Pi | Self::Euler | Self::Symbol(_) => {
                unchanged()
            }
            Self::Function { name, args } => Self::mapped_operands(args, &get)
                .map_or_else(unchanged, |mapped| {
                    FunctionDefinition::construct(name.clone(), mapped)
                }),
            Self::Sum(args) => Self::mapped_operands(args, &get).map_or_else(unchanged, Expr::sum),
            Self::Product(args) => {
                Self::mapped_operands(args, &get).map_or_else(unchanged, Expr::product)
            }
            Self::Div(left, right) => match (get(0, left), get(1, right)) {
                (mapped_left, mapped_right) if mapped_left == left && mapped_right == right => {
                    unchanged()
                }
                (mapped_left, mapped_right) => {
                    Self::normalize_quotient(mapped_left.clone(), mapped_right.clone())
                }
            },
            Self::Pow(base, exponent) => match (get(0, base), get(1, exponent)) {
                (mapped_base, mapped_exponent)
                    if mapped_base == base && mapped_exponent == exponent =>
                {
                    unchanged()
                }
                (mapped_base, mapped_exponent) => {
                    Self::normalize_power(mapped_base.clone(), mapped_exponent.clone())
                }
            },
            Self::Derivative { inner, var, order } => match get(0, inner) {
                mapped if mapped == inner => unchanged(),
                mapped => Expr::derivative(mapped.clone(), var.symbol(), order.get()),
            },
            Self::Poly(polynomial) => {
                if original
                    .stored_children()
                    .enumerate()
                    .all(|(ordinal, child)| child == get(ordinal, child))
                {
                    unchanged()
                } else {
                    Expr::sum(Self::mapped_polynomial(polynomial, |ordinal, child| {
                        get(ordinal, child).clone()
                    }))
                }
            }
        }
    }

    /// The mapped operands, or `None` when every operand maps to itself. Each
    /// operand is looked up once; the unchanged prefix is cloned directly.
    fn mapped_operands<'mapped>(
        operands: &[Expr],
        get: &impl Fn(usize, &Expr) -> &'mapped Expr,
    ) -> Option<Vec<Expr>> {
        let mut mapped: Option<Vec<Expr>> = None;
        for (index, operand) in operands.iter().enumerate() {
            let replacement = get(index, operand);
            if let Some(rebuilt) = &mut mapped {
                rebuilt.push(replacement.clone());
            } else if replacement != operand {
                let mut rebuilt = Vec::with_capacity(operands.len());
                rebuilt.extend(operands.iter().take(index).cloned());
                rebuilt.push(replacement.clone());
                mapped = Some(rebuilt);
            }
        }
        mapped
    }
}

impl StoredForm for Expr {
    fn stored_children(&self) -> impl Iterator<Item = &Self> {
        let mut edges = Edges::default();
        match self.node() {
            Node::Number(_) | Node::ImaginaryUnit | Node::Pi | Node::Euler | Node::Symbol(_) => {}
            Node::Function { args, .. } | Node::Sum(args) | Node::Product(args) => {
                edges.operands = args.iter();
            }
            Node::Div(left, right) | Node::Pow(left, right) => {
                edges.first = Some(left);
                edges.second = Some(right);
            }
            Node::Derivative { inner, .. } => edges.first = Some(inner),
            Node::Poly(polynomial) => {
                edges.operands = polynomial.bases.iter();
                edges.coefficients = polynomial.coefficients.iter();
            }
        }
        edges
    }

    fn stored_terms(&self) -> Option<&[Self]> {
        if let Node::Sum(terms) = self.node() {
            Some(terms)
        } else {
            None
        }
    }

    fn monomials(
        &self,
    ) -> Option<
        impl ExactSizeIterator<Item = (&Self, impl ExactSizeIterator<Item = (&Self, u32)> + Clone)>,
    > {
        if let Node::Poly(polynomial) = self.node() {
            Some(polynomial.monomials())
        } else {
            None
        }
    }

    fn simple_bases(&self) -> Option<bool> {
        if let Node::Poly(polynomial) = self.node() {
            Some(polynomial.simple_bases)
        } else {
            None
        }
    }

    fn positive_integer(&self) -> Option<NonZeroU64> {
        let (numerator, denominator) = self.exact_ratio()?;
        if denominator != 1 {
            return None;
        }
        u64::try_from(numerator).ok().and_then(NonZeroU64::new)
    }
}

impl<'expr> Iterator for Edges<'expr> {
    type Item = &'expr Expr;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(operand) = self.operands.next() {
            return Some(operand);
        }
        if let Some(operand) = self.first.take() {
            return Some(operand);
        }
        if let Some(operand) = self.second.take() {
            return Some(operand);
        }
        self.coefficients.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Every count is bounded by live allocations, so the sum cannot saturate.
        let known = self
            .operands
            .len()
            .saturating_add(usize::from(self.first.is_some()))
            .saturating_add(usize::from(self.second.is_some()))
            .saturating_add(self.coefficients.len());
        (known, Some(known))
    }
}

impl FusedIterator for Edges<'_> {}
