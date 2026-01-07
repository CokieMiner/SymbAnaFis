//! Implicit multiplication insertion for natural notation
//!
//! Inserts `*` operators between tokens where multiplication is implied, e.g. `2x` → `2 * x`.

use crate::parser::tokens::{Operator, Token};

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
pub fn insert_implicit_multiplication<S: std::hash::BuildHasher>(
    tokens: Vec<Token>,
    custom_functions: &std::collections::HashSet<String, S>,
) -> Vec<Token> {
    if tokens.is_empty() {
        return tokens;
    }

    let mut result = Vec::with_capacity(tokens.len() * 3 / 2);
    let mut it = tokens.into_iter().peekable();

    while let Some(current) = it.next() {
        let needs_mul = it.peek().is_some_and(|next| match (&current, next) {
            // Number * Function operator: 4 sin(x) → 4 * sin(x)
            (Token::Number(_), Token::Operator(op)) if op.is_function() => true,

            // Identifier * Function operator
            (Token::Identifier(_), Token::Operator(op)) if op.is_function() => true,

            // Identifier * (
            (Token::Identifier(name), Token::LeftParen) => {
                // If it's a custom function, do NOT insert multiplication
                !custom_functions.contains(name)
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
            (Token::Number(_) | Token::Identifier(_) | Token::RightParen,
             Token::Identifier(_))
            | (Token::Number(_) | Token::RightParen, Token::LeftParen)
            | (Token::Identifier(_) | Token::RightParen, Token::Number(_)) => true,

            // Function operator * ( is NOT multiplication (it's function call)
            (Token::Operator(op), Token::LeftParen) if op.is_function() => false,

            _ => false,
        });

        result.push(current);
        if needs_mul {
            result.push(Token::Operator(Operator::Mul));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_number_identifier() {
        let tokens = vec![Token::Number(2.0), Token::Identifier("x".to_string())];
        let result = insert_implicit_multiplication(tokens, &HashSet::new());
        assert_eq!(result.len(), 3);
        assert!(matches!(result[1], Token::Operator(Operator::Mul)));
    }

    #[test]
    fn test_identifier_identifier() {
        let tokens = vec![
            Token::Identifier("a".to_string()),
            Token::Identifier("x".to_string()),
        ];
        let result = insert_implicit_multiplication(tokens, &HashSet::new());
        assert_eq!(result.len(), 3);
        assert!(matches!(result[1], Token::Operator(Operator::Mul)));
    }

    #[test]
    fn test_paren_identifier() {
        let tokens = vec![Token::RightParen, Token::Identifier("x".to_string())];
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
            Token::Identifier("x".to_string()),
            Token::Operator(Operator::Sin),
        ];
        let result = insert_implicit_multiplication(tokens, &HashSet::new());
        assert_eq!(result.len(), 3);
        assert!(matches!(result[1], Token::Operator(Operator::Mul)));
    }
}
