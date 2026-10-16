//! Deterministic structural ordering with canonical-pointer shortcuts.

use core::cmp::Ordering;

use alloc::vec::Vec;

use super::{Expr, Node, Polynomial};

/// Compare structures iteratively, without cached hashes or recursion.
impl Node {
    pub const POLYNOMIAL_RANK: u8 = 7;

    pub fn compare(left: &Expr, right: &Expr) -> Ordering {
        let mut pending = Vec::new();
        let immediate = compare_expression(left, right, &mut pending);
        if immediate != Ordering::Equal {
            return immediate;
        }
        while let Some((first, second)) = pending.pop() {
            let order = compare_expression(first, second, &mut pending);
            if order != Ordering::Equal {
                return order;
            }
        }
        Ordering::Equal
    }

    pub const fn rank(&self) -> u8 {
        match self {
            Self::Number(_) => 0,
            Self::ImaginaryUnit => 1,
            Self::Pi => 2,
            Self::Euler => 3,
            Self::Symbol(_) => 4,
            Self::Function { .. } => 5,
            Self::Sum(_) => 6,
            Self::Poly(_) => Self::POLYNOMIAL_RANK,
            Self::Product(_) => 8,
            Self::Div(..) => 9,
            Self::Pow(..) => 10,
            Self::Derivative { .. } => 11,
        }
    }
}

fn compare_expression<'node>(
    left: &'node Expr,
    right: &'node Expr,
    pending: &mut Vec<(&'node Expr, &'node Expr)>,
) -> Ordering {
    if left.same_node(right) {
        return Ordering::Equal;
    }
    let kind_order = left.node().rank().cmp(&right.node().rank());
    if kind_order != Ordering::Equal {
        return kind_order;
    }
    compare_node(left.node(), right.node(), pending)
}

fn compare_node<'node>(
    left: &'node Node,
    right: &'node Node,
    pending: &mut Vec<(&'node Expr, &'node Expr)>,
) -> Ordering {
    match (left, right) {
        (Node::Number(a), Node::Number(b)) => a.compare(b),
        (Node::ImaginaryUnit, Node::ImaginaryUnit)
        | (Node::Pi, Node::Pi)
        | (Node::Euler, Node::Euler) => Ordering::Equal,
        (Node::Symbol(a), Node::Symbol(b)) => {
            a.name().cmp(&b.name()).then_with(|| a.id().cmp(&b.id()))
        }
        (
            Node::Function {
                name: a_name,
                args: a,
            },
            Node::Function {
                name: b_name,
                args: b,
            },
        ) => {
            let names = a_name.cmp(b_name);
            if names != Ordering::Equal {
                return names;
            }
            compare_children(a, b, pending)
        }
        (Node::Sum(a), Node::Sum(b)) | (Node::Product(a), Node::Product(b)) => {
            compare_children(a, b, pending)
        }
        (Node::Poly(a), Node::Poly(b)) => compare_polynomial(a, b, pending),
        (Node::Div(a, b), Node::Div(c, d)) | (Node::Pow(a, b), Node::Pow(c, d)) => {
            pending.push((b, d));
            pending.push((a, c));
            Ordering::Equal
        }
        (
            Node::Derivative {
                inner: a,
                var: a_var,
                order: a_order,
            },
            Node::Derivative {
                inner: b,
                var: b_var,
                order: b_order,
            },
        ) => {
            let order = a_var
                .id()
                .cmp(&b_var.id())
                .then_with(|| a_order.cmp(b_order));
            if order == Ordering::Equal {
                pending.push((a, b));
            }
            order
        }
        // Different kinds have already been ordered by the driver.
        (
            Node::Number(_)
            | Node::ImaginaryUnit
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Sum(_)
            | Node::Poly(_)
            | Node::Product(_)
            | Node::Div(..)
            | Node::Pow(..)
            | Node::Derivative { .. },
            _,
        ) => left.rank().cmp(&right.rank()),
    }
}

fn compare_polynomial<'node>(
    left: &'node Polynomial,
    right: &'node Polynomial,
    pending: &mut Vec<(&'node Expr, &'node Expr)>,
) -> Ordering {
    let shape = left
        .terms
        .len()
        .cmp(&right.terms.len())
        .then_with(|| {
            left.terms
                .iter()
                .map(|term| term.powers.len())
                .cmp(right.terms.iter().map(|term| term.powers.len()))
        })
        .then_with(|| {
            left.terms
                .iter()
                .flat_map(|term| term.powers.iter().map(|(_, power)| power))
                .cmp(
                    right
                        .terms
                        .iter()
                        .flat_map(|term| term.powers.iter().map(|(_, power)| power)),
                )
        });
    if shape != Ordering::Equal {
        return shape;
    }
    pending.extend(
        left.terms
            .iter()
            .zip(&right.terms)
            .rev()
            .map(|(first, second)| (&first.coefficient, &second.coefficient)),
    );
    pending.extend(
        left.terms
            .iter()
            .zip(&right.terms)
            .rev()
            .flat_map(|(first, second)| {
                first
                    .powers
                    .iter()
                    .zip(&second.powers)
                    .rev()
                    .map(|((first_base, _), (second_base, _))| (first_base, second_base))
            }),
    );
    compare_children(&left.bases, &right.bases, pending)
}

fn compare_children<'node>(
    left: &'node [Expr],
    right: &'node [Expr],
    pending: &mut Vec<(&'node Expr, &'node Expr)>,
) -> Ordering {
    let length = left.len().cmp(&right.len());
    if length == Ordering::Equal {
        pending.extend(left.iter().zip(right).rev());
    }
    length
}
