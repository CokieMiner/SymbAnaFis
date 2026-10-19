//! Structural ordering against a bounded recursive reference comparator.

use core::cmp::Ordering;

use alloc::{format, vec};

use proptest::{collection, prop_assert, prop_assert_eq, prop_oneof, proptest, strategy::Strategy};

use super::super::{Expr, Node, Symbol};

#[expect(
    clippy::arithmetic_side_effects,
    reason = "symbolic division constructs domain-preserving DAG nodes"
)]
fn expression_strategy() -> impl Strategy<Value = Expr> {
    (0_usize..4)
        .prop_map(|index| Expr::symbol(format!("ordering_symbol_{index}")))
        .prop_recursive(3, 24, 4, |inner| {
            prop_oneof![
                inner.clone().prop_map(|value| value.sin()),
                inner.clone().prop_map(|value| value.pow(3)),
                (inner.clone(), 1_i64..=5).prop_map(|(value, coefficient)| {
                    Expr::product(vec![Expr::number(coefficient), value])
                }),
                inner
                    .clone()
                    .prop_map(|value| Expr::sum(vec![value.pow(2), value.pow(3)])),
                inner.clone().prop_map(|value| {
                    Expr::derivative(value, Symbol::intern("ordering_variable"), 2)
                }),
                collection::vec(inner.clone(), 2..=4).prop_map(Expr::sum),
                collection::vec(inner.clone(), 2..=4).prop_map(Expr::product),
                (inner.clone(), inner).prop_map(|(numerator, denominator)| numerator / denominator),
            ]
        })
}

proptest! {
    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "shared symbolic numerators exercise lexicographic denominator comparison"
    )]
    fn iterative_order_matches_recursive_order_and_preserves_transitivity(
        first in expression_strategy(),
        second in expression_strategy(),
        third in expression_strategy(),
        depth in 0_usize..8,
    ) {
        let expressions = [first, second, third];
        for left in &expressions {
            for right in &expressions {
                let expected = reference(left, right);
                prop_assert_eq!(Node::compare(left, right), expected);
                prop_assert_eq!(Node::compare(right, left), expected.reverse());
                prop_assert_eq!(expected == Ordering::Equal, left == right);

                let (mut first_chain, mut second_chain) = (left.clone(), right.clone());
                for _ in 0..depth {
                    first_chain = first_chain.sin();
                    second_chain = second_chain.sin();
                }
                prop_assert_eq!(Node::compare(&first_chain, &second_chain), expected);

                let shared = Expr::symbol("ordering_shared_divisor");
                let first_quotient = &shared / left;
                let second_quotient = &shared / right;
                prop_assert_eq!(Node::compare(&first_quotient, &second_quotient), reference(&first_quotient, &second_quotient));
            }
        }
        let mut ordered = expressions;
        ordered.sort_by(Node::compare);
        let [left, middle, right] = &ordered;
        prop_assert_eq!(Node::compare(left, middle), reference(left, middle));
        prop_assert_eq!(Node::compare(middle, right), reference(middle, right));
        prop_assert_eq!(Node::compare(left, right), reference(left, right));
        prop_assert!(Node::compare(left, middle).is_le());
        prop_assert!(Node::compare(middle, right).is_le());
        prop_assert!(Node::compare(left, right).is_le());
    }
}

fn reference(left: &Expr, right: &Expr) -> Ordering {
    if left == right {
        return Ordering::Equal;
    }
    let kind = left.node().rank().cmp(&right.node().rank());
    if kind != Ordering::Equal {
        return kind;
    }
    match (left.node(), right.node()) {
        (Node::Number(a), Node::Number(b)) => a.compare(b),
        (Node::Symbol(a), Node::Symbol(b)) => {
            a.name().cmp(&b.name()).then_with(|| a.id().cmp(&b.id()))
        }
        (
            Node::Function {
                name: a,
                args: first,
            },
            Node::Function {
                name: b,
                args: second,
            },
        ) => a.cmp(b).then_with(|| children(first, second)),
        (Node::Sum(a), Node::Sum(b)) | (Node::Product(a), Node::Product(b)) => children(a, b),
        (Node::Div(a, b), Node::Div(c, d)) | (Node::Pow(a, b), Node::Pow(c, d)) => {
            reference(a, c).then_with(|| reference(b, d))
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
        ) => a_var
            .id()
            .cmp(&b_var.id())
            .then_with(|| a_order.cmp(b_order))
            .then_with(|| reference(a, b)),
        (Node::Poly(a), Node::Poly(b)) => a
            .terms
            .len()
            .cmp(&b.terms.len())
            .then_with(|| {
                a.terms
                    .iter()
                    .map(|term| term.powers.len())
                    .cmp(b.terms.iter().map(|term| term.powers.len()))
            })
            .then_with(|| {
                a.terms
                    .iter()
                    .flat_map(|term| term.powers.iter().map(|(_, power)| power))
                    .cmp(
                        b.terms
                            .iter()
                            .flat_map(|term| term.powers.iter().map(|(_, power)| power)),
                    )
            })
            .then_with(|| {
                a.terms
                    .iter()
                    .flat_map(|term| term.powers.iter())
                    .zip(b.terms.iter().flat_map(|term| term.powers.iter()))
                    .map(|((first, _), (second, _))| reference(first, second))
                    .find(|order| *order != Ordering::Equal)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| {
                a.terms
                    .iter()
                    .zip(&b.terms)
                    .map(|(first, second)| reference(&first.coefficient, &second.coefficient))
                    .find(|order| *order != Ordering::Equal)
                    .unwrap_or(Ordering::Equal)
            }),
        _ => kind,
    }
}

fn children(left: &[Expr], right: &[Expr]) -> Ordering {
    left.len().cmp(&right.len()).then_with(|| {
        left.iter()
            .zip(right)
            .map(|(first, second)| reference(first, second))
            .find(|order| *order != Ordering::Equal)
            .unwrap_or(Ordering::Equal)
    })
}
