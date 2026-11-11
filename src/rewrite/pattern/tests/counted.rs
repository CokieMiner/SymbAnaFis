//! The counted-operand head index is a superset of every value a slot projects
//! to, so a rule that skips a slot on it skips only values it would reject.
//!
//! Helpers: `demands` lists every demand a counted group can ask for,
//! `slot_values` projects one slot under each of them, and `factor_heads`
//! collects the heads of a value's factors.

use alloc::{vec, vec::Vec};

use crate::core::{AlgebraicOperands, Expr, ExprView, OperandDemand};

use super::super::{Heads, PatternNode};

/// Every demand a counted group can ask a slot for.
fn demands() -> Vec<OperandDemand> {
    let mut demands = vec![
        OperandDemand::Single,
        OperandDemand::Whole,
        OperandDemand::Power(1),
        OperandDemand::Power(2),
        OperandDemand::Copies(1),
        OperandDemand::Copies(2),
        OperandDemand::Copies(3),
        OperandDemand::Split(1),
        OperandDemand::Split(2),
        OperandDemand::Root {
            exponent: 1,
            block: 1,
        },
        OperandDemand::Root {
            exponent: 2,
            block: 1,
        },
    ];
    for numerator in 1..=3 {
        for denominator in 1..=3 {
            if let Some(step) = core::num::NonZeroU64::new(denominator) {
                demands.push(OperandDemand::Rational {
                    numerator,
                    denominator: step,
                });
            }
        }
    }
    demands
}

/// The heads of a value's factors, as `Heads::counted` bounds them.
fn factor_heads(value: &Expr) -> Heads {
    match value.view() {
        ExprView::Product(items) => items
            .iter()
            .fold(Heads::OTHER, |set, factor| set.union(Heads::of(factor))),
        ExprView::Sum(_)
        | ExprView::Div(..)
        | ExprView::Pow(..)
        | ExprView::Function { .. }
        | ExprView::Number(_)
        | ExprView::ImaginaryUnit
        | ExprView::Pi
        | ExprView::Euler
        | ExprView::Symbol(_)
        | ExprView::Derivative { .. } => Heads::OTHER.union(Heads::of(value)),
    }
}

/// Assert that the index of `expression` bounds every value each slot projects
/// to under every demand.
fn assert_index_bounds(expression: &Expr) {
    let operands = AlgebraicOperands::new(expression, true, 0).expect("an additive projection");
    let index: Vec<_> = operands.originals().map(Heads::counted).collect();
    assert_eq!(index.len(), operands.len(), "one entry per slot");
    for slot in 0..operands.len() {
        for demand in demands() {
            let Some((value, _)) =
                operands.candidate(slot, demand, None, core::iter::empty(), false)
            else {
                continue;
            };
            let bound = index.get(slot).copied().expect("one entry per slot");
            assert!(
                bound.value.covers(Heads::of(&value)),
                "slot {slot} under {demand:?} projects to {value}, outside its value bound"
            );
            assert!(
                bound.factors.covers(factor_heads(&value)),
                "slot {slot} under {demand:?} projects to {value}, outside its factor bound"
            );
        }
    }
}

#[test]
fn counted_index_bounds_signed_collected_and_powered_terms() {
    let x = Expr::symbol("index_x").expect("a fresh symbol");
    let y = Expr::symbol("index_y").expect("a fresh symbol");
    let z = Expr::symbol("index_z").expect("a fresh symbol");
    let terms = vec![
        x.clone(),
        x.clone() * 3,
        x.clone() * -3,
        x.clone() * y.clone(),
        x.clone() * y.clone() * -7,
        x.clone() * y.clone() * z.clone(),
        x.clone() * y.clone() * z.clone() * 5,
        x.clone() * y.clone() * z * -5,
        x.clone().pow(2),
        x.clone().pow(2) * 4,
        x.clone().pow(2) * y.clone() * -2,
        x.clone().sin(),
        x.clone().sin() * 6,
        x.clone().sin() * y.clone(),
        x.clone().sin() * y.clone() * -1,
        x.clone().sin() / y.clone(),
        Expr::number(4),
        Expr::number(-4),
        Expr::pi(),
        x.clone().pow(2) / y.clone(),
        (x.clone() * y).pow(2),
        x.clone().sqrt(),
        x.sqrt() * 3,
    ];
    assert_index_bounds(&Expr::sum(terms));
}

#[test]
fn counted_index_bounds_a_compact_polynomial_group() {
    let x = Expr::symbol("index_poly_x").expect("a fresh symbol");
    let y = Expr::symbol("index_poly_y").expect("a fresh symbol");
    let body = (x.clone() + y.clone()).pow(3) + (x.clone() - y).pow(2) * 2;
    assert_index_bounds(&(body + x.sin()));
}

#[test]
fn counted_index_bounds_every_demand_of_a_single_term() {
    let x = Expr::symbol("index_single_x").expect("a fresh symbol");
    let y = Expr::symbol("index_single_y").expect("a fresh symbol");
    for term in [
        x.clone(),
        x.clone() * -2,
        x.clone().pow(3),
        x.sin(),
        Expr::number(2),
    ] {
        assert_index_bounds(&Expr::sum(vec![term, y.clone()]));
    }
}

#[test]
fn required_factor_heads_ignore_an_inferred_coefficient() {
    let a = super::super::Pattern::capture("a", None);
    let coefficient = super::super::Pattern::exact(Expr::number(3));
    let product = PatternNode::Product(vec![coefficient, a.sin()]);
    assert_eq!(
        product.required_factors(),
        Heads::CALL,
        "the inferred coefficient is not required"
    );
    let bare = PatternNode::Product(vec![a.sin(), a.cos()]);
    assert_eq!(bare.required_factors(), Heads::CALL);
    let captured = PatternNode::Product(vec![a.clone(), a.cos()]);
    assert_eq!(
        captured.required_factors(),
        Heads(0),
        "a capture factor constrains nothing"
    );
    assert_eq!(
        PatternNode::Sum(vec![a.sin()]).required_factors(),
        Heads(0),
        "only a product constrains its factors"
    );
}
