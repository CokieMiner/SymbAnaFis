//! Deterministic structural ordering with canonical-pointer shortcuts.

#![expect(
    unsafe_code,
    reason = "the pending stack reads only slots below its logical length, which push initialized, and equally shaped polynomials share their power positions"
)]

use core::{
    cmp::Ordering,
    mem::MaybeUninit,
    ops::Range,
    slice::{Iter, from_ref},
};

use alloc::vec::Vec;

use super::{Expr, Node, Polynomial};

type NodePair<'node> = (&'node Expr, &'node Expr);

/// Comparisons deferred until every earlier comparison is equal. A cursor
/// yields its next unequal pair only when it is reached, so a comparison
/// decided by an early pair never scans the rest.
enum Pending<'node> {
    /// The remaining positions of two operand lists of equal length.
    Operands(Iter<'node, Expr>, Iter<'node, Expr>),
    /// Power positions of two polynomials with equal term lengths and
    /// exponents, whose bases are compared position by position.
    Bases(&'node Polynomial, &'node Polynomial, Range<usize>),
}

/// A work stack for iterative structural comparisons.
///
/// Most comparisons are decided within a few levels and never need heap
/// storage. The inline tail keeps those comparisons allocation-free while
/// spilling deeper nested comparisons to a vector.
///
/// Slots below `len` are initialized. Popped slots stay initialized in memory
/// but are outside the logical length and are overwritten by the next push.
struct PendingStack<'node> {
    inline: [MaybeUninit<Pending<'node>>; 8],
    len: usize,
    spill: Vec<Pending<'node>>,
}

impl<'node> PendingStack<'node> {
    const fn new() -> Self {
        Self {
            inline: [const { MaybeUninit::uninit() }; 8],
            len: 0,
            spill: Vec::new(),
        }
    }

    fn push(&mut self, pending: Pending<'node>) {
        if let Some(slot) = self.inline.get_mut(self.len) {
            slot.write(pending);
            self.len = self.len.saturating_add(1);
            return;
        }
        self.spill.push(pending);
    }

    /// The next unequal pair in comparison order. A cursor with pairs left
    /// after that pair stays on the stack.
    fn next_pair(&mut self) -> Option<NodePair<'node>> {
        loop {
            let pending = if let Some(pending) = self.spill.pop() {
                pending
            } else {
                let len = self.len.checked_sub(1)?;
                self.len = len;
                // SAFETY: `len` was below the previous logical length, and every
                // slot below the logical length was initialized by `push`.
                // Cursors borrow and have no drop glue, so reading one does not
                // duplicate ownership.
                unsafe { self.inline.get_unchecked(len).assume_init_read() }
            };
            match pending {
                Pending::Operands(mut left, mut right) => {
                    while let (Some(first), Some(second)) = (left.next(), right.next()) {
                        if first != second {
                            if !left.as_slice().is_empty() {
                                self.push(Pending::Operands(left, right));
                            }
                            return Some((first, second));
                        }
                    }
                }
                Pending::Bases(left, right, mut positions) => {
                    for position in positions.by_ref() {
                        // SAFETY: `positions` lies within the powers of both
                        // polynomials, which have equal term lengths, and every
                        // base position indexes its own base table.
                        let pair = unsafe {
                            (
                                left.bases
                                    .get_unchecked(left.powers.get_unchecked(position).0),
                                right
                                    .bases
                                    .get_unchecked(right.powers.get_unchecked(position).0),
                            )
                        };
                        if pair.0 != pair.1 {
                            if !positions.is_empty() {
                                self.push(Pending::Bases(left, right, positions));
                            }
                            return Some(pair);
                        }
                    }
                }
            }
        }
    }
}

/// Compare structures iteratively, without cached hashes or recursion.
impl Node {
    pub const POLYNOMIAL_RANK: u8 = 7;
    pub const PRODUCT_RANK: u8 = 8;
    pub const POWER_RANK: u8 = 10;

    pub fn compare(left: &Expr, right: &Expr) -> Ordering {
        if left == right {
            return Ordering::Equal;
        }
        let root_order = left.node().rank().cmp(&right.node().rank());
        if root_order != Ordering::Equal {
            return root_order;
        }
        match (left.node(), right.node()) {
            (Self::Number(a), Self::Number(b)) => return a.compare(b),
            (Self::Symbol(a), Self::Symbol(b)) => {
                return a.name().cmp(&b.name()).then_with(|| a.id().cmp(&b.id()));
            }
            (Self::ImaginaryUnit, Self::ImaginaryUnit)
            | (Self::Pi, Self::Pi)
            | (Self::Euler, Self::Euler) => return Ordering::Equal,
            _ => {}
        }
        let mut pending = PendingStack::new();
        let mut next = Some((left, right));
        while let Some((first, second)) = next.take().or_else(|| pending.next_pair()) {
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
            Self::Product(_) => Self::PRODUCT_RANK,
            Self::Div(..) => 9,
            Self::Pow(..) => Self::POWER_RANK,
            Self::Derivative { .. } => 11,
        }
    }
}

fn compare_node<'node>(
    left: &'node Node,
    right: &'node Node,
    next: &mut Option<NodePair<'node>>,
    pending: &mut PendingStack<'node>,
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
        (Node::Poly(a), Node::Poly(b)) => compare_polynomial(a, b, pending),
        (Node::Div(a, b), Node::Div(c, d)) | (Node::Pow(a, b), Node::Pow(c, d)) => {
            if a == c {
                *next = Some((b, d));
            } else {
                *next = Some((a, c));
                if b != d {
                    pending.push(Pending::Operands(from_ref(b).iter(), from_ref(d).iter()));
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
    pending: &mut PendingStack<'node>,
) -> Ordering {
    let shape = left
        .coefficients
        .len()
        .cmp(&right.coefficients.len())
        .then_with(|| left.lengths.cmp(&right.lengths))
        .then_with(|| {
            left.powers
                .iter()
                .map(|(_, exponent)| exponent)
                .cmp(right.powers.iter().map(|(_, exponent)| exponent))
        });
    if shape == Ordering::Equal {
        // Bases decide position by position, term after term, before any
        // coefficient.
        pending.push(Pending::Operands(
            left.coefficients.iter(),
            right.coefficients.iter(),
        ));
        pending.push(Pending::Bases(left, right, 0..left.powers.len()));
    }
    shape
}

fn compare_children<'node>(
    left: &'node [Expr],
    right: &'node [Expr],
    next: &mut Option<NodePair<'node>>,
    pending: &mut PendingStack<'node>,
) -> Ordering {
    let length = left.len().cmp(&right.len());
    if length == Ordering::Equal {
        let mut first = left.iter();
        let mut second = right.iter();
        while let (Some(a), Some(b)) = (first.next(), second.next()) {
            if a == b {
                continue;
            }
            if left.len() > 1
                && let (Node::Number(x), Node::Number(y)) = (a.node(), b.node())
            {
                let coefficient_order = x.compare(y);
                if coefficient_order != Ordering::Equal {
                    return coefficient_order;
                }
            }
            *next = Some((a, b));
            if !first.as_slice().is_empty() {
                pending.push(Pending::Operands(first, second));
            }
            break;
        }
    }
    length
}
