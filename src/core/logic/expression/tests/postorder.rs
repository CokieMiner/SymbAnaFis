//! Postorder numbering: distinct nodes, earlier links, and edge-order agreement.

use alloc::{sync::Arc, vec, vec::Vec};

use proptest::{collection, prop_assume, proptest};

use crate::{collections::FixedSet, core::Context};

use super::super::{Expr, Node, Postorder, StoredForm};

/// Assert the invariants that the unchecked reads of `Postorder` rely on.
/// `children` gives the children the numbering traverses below a node.
fn assert_numbering(order: &Postorder<'_>, input: &Expr, children: fn(&Expr) -> Vec<Expr>) {
    let descendants = order.descendants();
    let distinct: FixedSet<_> = descendants
        .iter()
        .map(|visit| visit.expression().pointer())
        .collect();
    assert_eq!(distinct.len(), descendants.len(), "nodes are distinct");
    assert_eq!(order.root.expression(), input, "the root is the input");
    for (position, visit) in descendants
        .iter()
        .enumerate()
        .chain([(descendants.len(), &order.root)])
    {
        let links = order
            .numbering
            .links
            .get(visit.links.start..visit.links.end)
            .expect("a recorded span lies inside the link list");
        assert!(
            links.iter().all(|&link| link < position),
            "every link points to an earlier descendant"
        );
        let expected = children(visit.expression());
        let linked: Vec<&Expr> = links
            .iter()
            .map(|&link| {
                descendants
                    .get(link)
                    .expect("a link indexes the descendants")
                    .expression()
            })
            .collect();
        assert_eq!(linked.len(), expected.len(), "one link per child");
        assert!(
            linked
                .iter()
                .zip(&expected)
                .all(|(link, child)| *link == child),
            "links follow child order"
        );
    }
}

/// Assert that a forest numbers shared nodes once, links earlier positions,
/// and records each root by the position of its own expression.
fn assert_forest(roots: &[Expr]) {
    let forest = Postorder::forest(roots);
    let visits = &forest.numbering.visits;
    let distinct: FixedSet<_> = visits
        .iter()
        .map(|visit| visit.expression().pointer())
        .collect();
    assert_eq!(distinct.len(), visits.len(), "nodes are distinct");
    for (position, visit) in visits.iter().enumerate() {
        let links = forest
            .numbering
            .links
            .get(visit.links.start..visit.links.end)
            .expect("a recorded span lies inside the link list");
        assert!(
            links.iter().all(|&link| link < position),
            "every link points to an earlier visit"
        );
    }
    assert_eq!(forest.roots.len(), roots.len(), "one position per root");
    for (root, &position) in roots.iter().zip(&forest.roots) {
        let visit = visits.get(position).expect("a root position names a visit");
        assert_eq!(visit.expression(), root, "a root position names its root");
    }
}

proptest! {
    #[test]
    fn stored_and_logical_numberings_link_earlier_children_in_edge_order(
        exponents in collection::vec(2_i64..=9, 2..=6),
        shared_power in 2_i64..=4,
    ) {
        let mut distinct = exponents;
        distinct.sort_unstable();
        distinct.dedup();
        prop_assume!(distinct.len() >= 2);
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let shared = Expr::sum(vec![x.sin(), y.into()]);
        let polynomial = Expr::sum(distinct.iter().map(|power| x.pow(*power)).collect());
        let mixed = Expr::sum(vec![
            shared.pow(shared_power),
            Expr::product(vec![shared.clone(), polynomial.clone()]),
            polynomial,
            Expr::number(1),
        ]);
        let roots = [mixed.clone(), shared.clone(), mixed.clone()];
        let opaque = Expr::sum(vec![x.sin().pow(2), x.sin().pow(3)]);
        let input = Node::normalize_quotient(Expr::sum(vec![mixed.sin(), opaque]), shared);
        assert_forest(&roots);
        assert_numbering(&Postorder::stored(&input), &input, |expression| {
            expression.stored_children().cloned().collect()
        });
        assert_numbering(&Postorder::logical(&input), &input, |expression| {
            if Node::projects(expression) {
                expression.children().cloned().collect()
            } else {
                expression.stored_children().cloned().collect()
            }
        });
        // Compact storage in symbols is a leaf, and a sum holding it as its
        // group numbers only its other operands; storage with opaque bases is
        // projected.
        assert_numbering(&Postorder::compact(&input), &input, |expression| {
            let scalar = |storage: &Expr| {
                let Node::Poly(compact) = storage.node() else {
                    return false;
                };
                compact
                    .bases
                    .iter()
                    .all(|base| matches!(base.node(), Node::Symbol(_) | Node::Pi | Node::Euler))
            };
            let grouped = expression
                .stored_terms()
                .is_some_and(|terms| terms.iter().any(scalar));
            if scalar(expression) {
                Vec::new()
            } else if grouped {
                expression
                    .stored_children()
                    .filter(|term| !matches!(term.node(), Node::Poly(_)))
                    .cloned()
                    .collect()
            } else if Node::projects(expression) {
                expression.children().cloned().collect()
            } else {
                expression.stored_children().cloned().collect()
            }
        });
        assert_numbering(&Postorder::factors(&input), &input, |expression| {
            if matches!(expression.node(), Node::Product(_) | Node::Div(..)) {
                expression.stored_children().cloned().collect()
            } else {
                Vec::new()
            }
        });
    }
}

#[test]
fn a_term_met_with_one_handle_before_its_projection_is_numbered_once() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let y = context.symb("y").expect("symbol");
    let term = || Expr::product(vec![x.pow(2), y.into()]);
    let polynomial = Expr::sum(vec![term(), x.pow(3), Expr::number(1)]);
    // The denominator is expanded first, so the product inside `sin` is met
    // with one handle before the numerator's projection adds a second.
    let input = Node::normalize_quotient(polynomial, term().sin());
    let parts = if let Node::Div(numerator, denominator) = input.node()
        && let Node::Function { args, .. } = denominator.node()
    {
        Some((numerator, args.first()))
    } else {
        None
    };
    let (numerator, argument) = parts.expect("a quotient by a function stays a quotient");
    assert!(
        Node::projects(numerator),
        "the numerator holds a compact group"
    );
    let product = argument.expect("sin has one argument");
    assert_eq!(
        Arc::strong_count(&product.0),
        1,
        "the product has one handle"
    );
    assert_numbering(&Postorder::logical(&input), &input, |expression| {
        if Node::projects(expression) {
            expression.children().cloned().collect()
        } else {
            expression.stored_children().cloned().collect()
        }
    });
}
