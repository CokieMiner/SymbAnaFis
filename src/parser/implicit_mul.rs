//! Implicit multiplication insertion for natural notation
//!
//! Inserts `*` operators between tokens where multiplication is implied, e.g. `2x` → `2 * x`.

use crate::parser::tokens::{Operator, Token};

/// Check if implicit multiplication should be inserted between two tokens
fn should_insert_mul<'src, S: std::hash::BuildHasher>(
    current: &Token<'src>,
    next: &Token<'src>,
    custom_functions: &std::collections::HashSet<String, S>,
) -> bool {
    match (current, next) {
        // Number * Function operator: 4 sin(x) → 4 * sin(x)
        (Token::Number(_), Token::Operator(op)) if op.is_function() => true,

        // Identifier * Function operator
        (Token::Identifier(_), Token::Operator(op)) if op.is_function() => true,

        // Identifier * (
        (Token::Identifier(name), Token::LeftParen) => {
            // If it's a custom function, do NOT insert multiplication
            !custom_functions.contains(name.as_ref())
        }

        // ) * Function operator: (a) sin(x) → (a) * sin(x)
        (Token::RightParen, Token::Operator(op)) if op.is_function() => true,

        // Coalesced arms for standard multiplication cases:
        // Number * Identifier: 2x
        // Number * (: 2(x)
        // Identifier * Identifier: xy
        // Identifier * Number: x2
        // ) * Identifier: )x
        // ) * Number: )2
        // ) * (: )(
        (Token::Number(_) | Token::Identifier(_) | Token::RightParen, Token::Identifier(_))
        | (Token::Number(_) | Token::RightParen, Token::LeftParen)
        | (Token::Identifier(_) | Token::RightParen, Token::Number(_)) => true,

        // Function operator * ( is NOT multiplication (it's function call)
        (Token::Operator(op), Token::LeftParen) if op.is_function() => false,

        _ => false,
    }
}

/// Insert implicit multiplication operators between appropriate tokens
///
/// Rules:
/// - Number * Identifier: `2 x` → `2 * x`
/// - Identifier * Identifier: `a x` → `a * x`
/// - Identifier * Function: `x sin` → `x * sin`
/// - ) * Identifier/Number/(: `(a) x` → `(a) * x`
/// - Identifier/Number * (: `x (y)` → `x * (y)` (unless function call)
///
/// Exception: Function followed by ( is NOT multiplication
pub fn insert_implicit_multiplication<'src, S: std::hash::BuildHasher>(
    tokens: Vec<Token<'src>>,
    custom_functions: &std::collections::HashSet<String, S>,
) -> Vec<Token<'src>> {
    if tokens.is_empty() {
        return tokens;
    }

    // Optimization: Check if any insertion is needed before allocating new vector
    let needs_insertion = tokens
        .windows(2)
        .any(|w| should_insert_mul(&w[0], &w[1], custom_functions));

    if !needs_insertion {
        return tokens;
    }

    #[allow(
        clippy::integer_division,
        reason = "Integer division for capacity estimation in token vector"
    )]
    let mut result = Vec::with_capacity(tokens.len() * 3 / 2);
    let mut it = tokens.into_iter().peekable();

    while let Some(current) = it.next() {
        let needs_mul = it
            .peek()
            .is_some_and(|next| should_insert_mul(&current, next, custom_functions));

        result.push(current);
        if needs_mul {
            result.push(Token::Operator(Operator::Mul));
        }
    }

    result
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_number_identifier() {
        let tokens = vec![Token::Number(2.0), Token::Identifier("x".into())];
        let result = insert_implicit_multiplication(tokens, &HashSet::new());
        assert_eq!(result.len(), 3);
        assert!(matches!(result[1], Token::Operator(Operator::Mul)));
    }

    #[test]
    fn test_identifier_identifier() {
        let tokens = vec![
            Token::Identifier("a".into()),
            Token::Identifier("x".into()),
        ];
        let result = insert_implicit_multiplication(tokens, &HashSet::new());
        assert_eq!(result.len(), 3);
        assert!(matches!(result[1], Token::Operator(Operator::Mul)));
    }

    #[test]
    fn test_paren_identifier() {
        let tokens = vec![Token::RightParen, Token::Identifier("x".into())];
        let result = insert_implicit_multiplication(tokens, &HashSet::new());
        assert_eq!(result.len(), 3);
        assert!(matches!(result[1], Token::Operator(Operator::Mul)));
    }

    #[test]
    fn test_function_no_multiplication() {
        let tokens = vec![Token::Operator(Operator::Sin), Token::LeftParen];
        let result = insert_implicit_multiplication(tokens, &HashSet::new());
        assert_eq!(result.len(), 2); // No multiplication inserted
    }

    #[test]
    fn test_number_function() {
        let tokens = vec![Token::Number(4.0), Token::Operator(Operator::Sin)];
        let result = insert_implicit_multiplication(tokens, &HashSet::new());
        assert_eq!(result.len(), 3);
        assert!(matches!(result[1], Token::Operator(Operator::Mul)));
    }

    #[test]
    fn test_identifier_function() {
        let tokens = vec![
            Token::Identifier("x".into()),
            Token::Operator(Operator::Sin),
        ];
        let result = insert_implicit_multiplication(tokens, &HashSet::new());
        assert_eq!(result.len(), 3);
        assert!(matches!(result[1], Token::Operator(Operator::Mul)));
    }
}
