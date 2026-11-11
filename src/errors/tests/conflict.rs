//! Bounded rendering of conflicting substitution destinations.

use alloc::{format, string::ToString, vec::Vec};

use crate::{Expr, errors::SubstitutionError};

/// A sum of `count` distinct symbols, whose rendering exceeds the byte budget.
fn wide_expression(count: usize) -> Expr {
    let terms: Vec<Expr> = (0..count)
        .map(|index| Expr::symbol(&format!("x{index}")).expect("symbol allocation"))
        .collect();
    Expr::sum(terms)
}

#[test]
fn small_conflicts_render_every_expression_in_full() {
    let error = SubstitutionError::Conflict {
        from: Expr::symbol("a").expect("symbol allocation"),
        first: Expr::symbol("b").expect("symbol allocation"),
        second: Expr::symbol("c").expect("symbol allocation"),
    };
    assert_eq!(
        error.to_string(),
        "conflicting destinations for a: b and c",
        "expressions under the budget are rendered in full"
    );
}

#[test]
fn wide_conflicts_stay_within_the_per_expression_budget() {
    let wide = wide_expression(64);
    let error = SubstitutionError::Conflict {
        from: wide.clone(),
        first: wide.clone(),
        second: wide,
    };
    let message = error.to_string();
    // 29 bytes of prefix, three expressions of at most 96 bytes plus an
    // ellipsis each, and 7 bytes of separators.
    assert!(
        message.len() <= 333,
        "each expression is capped at 96 bytes plus an ellipsis, got {}",
        message.len()
    );
    assert!(message.ends_with("..."), "truncation is marked: {message}");
}

#[test]
fn truncation_lands_on_a_character_boundary() {
    let name = format!("z{}", "\u{e9}".repeat(48));
    let expression = Expr::symbol(&name).expect("symbol allocation");
    let error = SubstitutionError::Conflict {
        from: expression.clone(),
        first: expression.clone(),
        second: expression,
    };
    let truncated = format!("z{}...", "\u{e9}".repeat(47));
    assert_eq!(
        error.to_string(),
        format!("conflicting destinations for {truncated}: {truncated} and {truncated}"),
        "a split multi-byte character is dropped instead of being replaced"
    );
}
