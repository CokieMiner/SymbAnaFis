//! Deterministic structural ordering with canonical-pointer shortcuts.

use core::cmp::Ordering;

use alloc::vec::Vec;

use super::{Expr, Node, Polynomial};

type NodePair<'node> = (&'node Expr, &'node Expr);

/// Compare structures iteratively, without cached hashes or recursion.
impl Node {
    pub const POLYNOMIAL_RANK: u8 = 7;

    pub fn compare(left: &Expr, right: &Expr) -> Ordering {
        let mut pending = Vec::new();
        let mut next = Some((left, right));
        while let Some((first, second)) = next.take().or_else(|| pending.pop()) {
            if first == second {
                continue;
            }
            let kind_order = first.node().rank().cmp(&second.node().rank());
            if kind_order != Ordering::Equal {
                return kind_order;
            }
            let order = compare_node(first.node(), second.node(), &mut next, &mut pending);
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

fn compare_node<'node>(
    left: &'node Node,
    right: &'node Node,
    next: &mut Option<NodePair<'node>>,
    pending: &mut Vec<NodePair<'node>>,
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
            compare_children(a, b, next, pending)
        }
        (Node::Sum(a), Node::Sum(b)) | (Node::Product(a), Node::Product(b)) => {
            compare_children(a, b, next, pending)
        }
        (Node::Poly(a), Node::Poly(b)) => compare_polynomial(a, b, next, pending),
        (Node::Div(a, b), Node::Div(c, d)) | (Node::Pow(a, b), Node::Pow(c, d)) => {
            if a == c {
                *next = Some((b, d));
            } else {
                *next = Some((a, c));
                if b != d {
                    pending.push((b, d));
                }
            }
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
                *next = Some((a, b));
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
    next: &mut Option<NodePair<'node>>,
    pending: &mut Vec<NodePair<'node>>,
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
            .filter_map(|(first, second)| {
                (first.coefficient != second.coefficient)
                    .then_some((&first.coefficient, &second.coefficient))
            }),
    );
    let mut bases = left
        .terms
        .iter()
        .zip(&right.terms)
        .flat_map(|(first, second)| {
            first.powers.iter().zip(&second.powers).filter_map(
                |((first_base, _), (second_base, _))| {
                    (first_base != second_base).then_some((first_base, second_base))
                },
            )
        });
    *next = bases.next();
    pending.extend(bases.rev());
    Ordering::Equal
}

fn compare_children<'node>(
    left: &'node [Expr],
    right: &'node [Expr],
    next: &mut Option<NodePair<'node>>,
    pending: &mut Vec<NodePair<'node>>,
) -> Ordering {
    let length = left.len().cmp(&right.len());
    if length == Ordering::Equal {
        let mut unequal = left
            .iter()
            .zip(right)
            .filter(|(first, second)| first != second);
        *next = unequal.next();
        if left.len() > 1
            && let Some((first, second)) = *next
            && let (Node::Number(a), Node::Number(b)) = (first.node(), second.node())
        {
            let coefficient_order = a.compare(b);
            if coefficient_order != Ordering::Equal {
                return coefficient_order;
            }
        }
        pending.extend(unequal.rev());
    }
    length
}
