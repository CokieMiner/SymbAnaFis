#![allow(
    clippy::float_cmp,
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]

use super::implicit_mul::*;
use super::lexer::*;
use super::pratt::*;
use super::tokens::{Operator, Token};
use crate::core::{Expr, ExprKind};
use std::borrow::Cow;
use std::collections::HashSet;

// ============================================================================
// Token Tests
// ============================================================================

#[test]
fn test_tokens_is_function() {
    assert!(Operator::Sin.is_function());
    assert!(Operator::Cos.is_function());
    assert!(!Operator::Add.is_function());
    assert!(!Operator::Mul.is_function());
}

#[test]
fn test_tokens_from_str() {
    assert_eq!(Operator::parse_str("+"), Some(Operator::Add));
    assert_eq!(Operator::parse_str("sin"), Some(Operator::Sin));
    assert_eq!(Operator::parse_str("**"), Some(Operator::Pow));
    assert_eq!(Operator::parse_str("invalid"), None);
}

#[test]
fn test_tokens_precedence() {
    assert!(Operator::Sin.precedence() > Operator::Pow.precedence());
    assert!(Operator::Pow.precedence() > Operator::Mul.precedence());
    assert!(Operator::Mul.precedence() > Operator::Add.precedence());
}

// ============================================================================
// Implicit Multiplication Tests
// ============================================================================

#[test]
fn test_imul_number_identifier() {
    let tokens = vec![Token::Number(2.0), Token::Identifier("x".into())];
    let result = insert_implicit_multiplication(tokens, &HashSet::new());
    assert_eq!(result.len(), 3);
    assert!(matches!(result[1], Token::Operator(Operator::Mul)));
}

#[test]
fn test_imul_identifier_identifier() {
    let tokens = vec![Token::Identifier("a".into()), Token::Identifier("x".into())];
    let result = insert_implicit_multiplication(tokens, &HashSet::new());
    assert_eq!(result.len(), 3);
    assert!(matches!(result[1], Token::Operator(Operator::Mul)));
}

#[test]
fn test_imul_paren_identifier() {
    let tokens = vec![Token::RightParen, Token::Identifier("x".into())];
    let result = insert_implicit_multiplication(tokens, &HashSet::new());
    assert_eq!(result.len(), 3);
    assert!(matches!(result[1], Token::Operator(Operator::Mul)));
}

#[test]
fn test_imul_function_no_multiplication() {
    let tokens = vec![Token::Operator(Operator::Sin), Token::LeftParen];
    let result = insert_implicit_multiplication(tokens, &HashSet::new());
    assert_eq!(result.len(), 2); // No multiplication inserted
}

#[test]
fn test_imul_number_function() {
    let tokens = vec![Token::Number(4.0), Token::Operator(Operator::Sin)];
    let result = insert_implicit_multiplication(tokens, &HashSet::new());
    assert_eq!(result.len(), 3);
    assert!(matches!(result[1], Token::Operator(Operator::Mul)));
}

#[test]
fn test_imul_identifier_function() {
    let tokens = vec![
        Token::Identifier("x".into()),
        Token::Operator(Operator::Sin),
    ];
    let result = insert_implicit_multiplication(tokens, &HashSet::new());
    assert_eq!(result.len(), 3);
    assert!(matches!(result[1], Token::Operator(Operator::Mul)));
}

// ============================================================================
// Pratt Parser Tests
// ============================================================================

#[test]
fn test_pratt_parse_number() {
    let tokens = vec![Token::Number(314.0 / 100.0)];
    let ast = parse_expression(&tokens, None).unwrap();
    assert_eq!(ast, Expr::number(314.0 / 100.0));
}

#[test]
fn test_pratt_parse_symbol() {
    let tokens = vec![Token::Identifier("x".into())];
    let ast = parse_expression(&tokens, None).unwrap();
    assert_eq!(ast, Expr::symbol("x"));
}

#[test]
fn test_pratt_parse_addition() {
    let tokens = vec![
        Token::Number(1.0),
        Token::Operator(Operator::Add),
        Token::Number(2.0),
    ];
    let ast = parse_expression(&tokens, None).unwrap();
    // 1 + 2 now combines like terms → 3
    assert!(matches!(ast.kind, ExprKind::Number(n) if (n - 3.0).abs() < 1e-10));
}

#[test]
fn test_pratt_parse_multiplication() {
    let tokens = vec![
        Token::Identifier("x".into()),
        Token::Operator(Operator::Mul),
        Token::Number(2.0),
    ];
    let ast = parse_expression(&tokens, None).unwrap();
    assert!(matches!(ast.kind, ExprKind::Product(_)));
}

#[test]
fn test_pratt_parse_power() {
    let tokens = vec![
        Token::Identifier("x".into()),
        Token::Operator(Operator::Pow),
        Token::Number(2.0),
    ];
    let ast = parse_expression(&tokens, None).unwrap();
    assert!(matches!(ast.kind, ExprKind::Pow(_, _)));
}

#[test]
fn test_pratt_parse_function() {
    let tokens = vec![
        Token::Operator(Operator::Sin),
        Token::LeftParen,
        Token::Identifier("x".into()),
        Token::RightParen,
    ];
    let ast = parse_expression(&tokens, None).unwrap();
    assert!(matches!(ast.kind, ExprKind::FunctionCall { .. }));
}

#[test]
fn test_pratt_precedence() {
    // x + 2 * 3 should be x + 6 (2*3 evaluated, then combined with x)
    let tokens = vec![
        Token::Identifier("x".into()),
        Token::Operator(Operator::Add),
        Token::Number(2.0),
        Token::Operator(Operator::Mul),
        Token::Number(3.0),
    ];
    let ast = parse_expression(&tokens, None).unwrap();

    // With like-term combination: Sum([6, x])
    match &ast.kind {
        ExprKind::Sum(terms) => {
            assert_eq!(terms.len(), 2);
            // One should be a symbol, one should be a number (6)
            let has_symbol = terms.iter().any(|t| matches!(t.kind, ExprKind::Symbol(_)));
            let has_six = terms
                .iter()
                .any(|t| matches!(t.kind, ExprKind::Number(n) if (n - 6.0).abs() < 1e-10));
            assert!(has_symbol, "Expected a symbol in sum");
            assert!(has_six, "Expected number 6 in sum (from 2*3)");
        }
        _ => panic!("Expected Sum at top level, got {:?}", ast.kind),
    }
}

#[test]
fn test_pratt_parentheses() {
    // (x + 1) * 2
    let tokens = vec![
        Token::LeftParen,
        Token::Identifier("x".into()),
        Token::Operator(Operator::Add),
        Token::Number(1.0),
        Token::RightParen,
        Token::Operator(Operator::Mul),
        Token::Number(2.0),
    ];
    let ast = parse_expression(&tokens, None).unwrap();

    // With n-ary Product: Product([Sum([x, 1]), 2])
    match &ast.kind {
        ExprKind::Product(factors) => {
            assert_eq!(factors.len(), 2);
            let has_sum = factors.iter().any(|f| matches!(f.kind, ExprKind::Sum(_)));
            let contains_two = factors
                .iter()
                .any(|f| matches!(f.kind, ExprKind::Number(n) if (n - 2.0).abs() < 1e-10));
            assert!(has_sum, "Expected a sum in product");
            assert!(contains_two, "Expected number 2 in product");
        }
        _ => panic!("Expected Product at top level, got {:?}", ast.kind),
    }
}

#[test]
fn test_pratt_empty_parentheses() {
    // () should be an error, NOT 1.0 or anything else
    let tokens = vec![Token::LeftParen, Token::RightParen];
    let result = parse_expression(&tokens, None);
    assert!(
        result.is_err(),
        "Empty parentheses should fail to parse, but got: {result:?}"
    );
}

// ============================================================================
// Lexer Tests
// ============================================================================

#[test]
fn test_lexer_balance_parentheses() {
    assert_eq!(balance_parentheses("(x + 1"), "(x + 1)");
    assert_eq!(balance_parentheses("x + 1)"), "(x + 1)");
    assert_eq!(balance_parentheses("(x)"), "(x)");
    assert_eq!(balance_parentheses(")(x"), "()(x)");
}

#[test]
fn test_lexer_parse_number() {
    assert_eq!(parse_number("3.14").unwrap(), 314.0 / 100.0);
    assert_eq!(parse_number("1e10").unwrap(), 1e10);
    assert_eq!(parse_number("2.5e-3").unwrap(), 0.0025);
    assert!(parse_number("3.14.15").is_err());
}

#[test]
fn test_lexer_scan_derivative_complex() {
    // Test explicitly the scanning of complex derivative string
    let input = "\u{2202}_f(x+y)/\u{2202}_x";
    let tokens = scan_characters(input).unwrap();
    assert_eq!(tokens.len(), 1);
    if let RawToken::Derivative(s) = &tokens[0] {
        assert_eq!(s, "\u{2202}_f(x+y)/\u{2202}_x");
    } else {
        panic!("Expected Derivative token, got {:?}", tokens[0]);
    }
}

#[test]
fn test_lexer_scan_derivative_nested_parens() {
    // Test derivative with deeply nested parentheses
    let input = "\u{2202}_f((x+y)*(z-w))/\u{2202}_x";
    let tokens = scan_characters(input).unwrap();
    assert_eq!(tokens.len(), 1);
    if let RawToken::Derivative(s) = &tokens[0] {
        assert_eq!(s, "\u{2202}_f((x+y)*(z-w))/\u{2202}_x");
    } else {
        panic!("Expected Derivative token, got {:?}", tokens[0]);
    }
}

#[test]
fn test_lexer_number_scientific_notation() {
    // Test various scientific notation formats
    let test_cases = vec![
        ("1e10", 1e10),
        ("2.5e-3", 2.5e-3),
        ("3.14e+2", 314.0),
        ("1E10", 1e10),
        ("2.5E-3", 2.5e-3),
    ];

    for (input, expected) in test_cases {
        let tokens = scan_characters(input).unwrap();
        assert_eq!(tokens.len(), 1);
        if let RawToken::Number(n) = tokens[0] {
            assert!(
                (n - expected).abs() < 1e-10,
                "Failed for {input}: expected {expected}, got {n}"
            );
        } else {
            panic!("Expected Number token for {input}");
        }
    }
}

#[test]
fn test_lexer_number_edge_cases() {
    // Test that edge cases are handled correctly
    // A lone dot without digits should fail validation in parse_number
    assert!(scan_characters(".").is_err() || parse_number(".").is_err());

    // Leading sign should NOT be part of number token
    let tokens = scan_characters("-123").unwrap();
    assert_eq!(tokens.len(), 2); // Operator(-) and Number(123)
}

#[test]
fn test_lexer_scan_characters() {
    let result1 = scan_characters("x + 1").unwrap();
    assert_eq!(result1.len(), 3);

    let result2 = scan_characters("sin(x)").unwrap();
    assert_eq!(result2.len(), 4);
}

#[test]
fn test_lexer_basic() {
    let fixed_vars = HashSet::new();
    let custom_funcs = HashSet::new();

    let tokens = lex("x", &fixed_vars, &custom_funcs).unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0], Token::Identifier(_)));
}

#[test]
fn test_lexer_with_fixed_vars() {
    let mut fixed_vars = HashSet::new();
    fixed_vars.insert("ax".to_owned());
    let custom_funcs = HashSet::new();

    let tokens = lex("ax", &fixed_vars, &custom_funcs).unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], Token::Identifier(Cow::Borrowed("ax")));
}

#[test]
fn test_lexer_empty_parens() {
    let fixed_vars = HashSet::new();
    let custom_funcs = HashSet::new();

    let tokens = lex("()", &fixed_vars, &custom_funcs).unwrap();
    assert_eq!(tokens.len(), 2);
    assert!(matches!(tokens[0], Token::LeftParen));
    assert!(matches!(tokens[1], Token::RightParen));
}

#[test]
fn test_lexer_unicode_identifiers() {
    // Test that multi-char Unicode identifiers work correctly
    let fixed_vars = HashSet::new();
    let custom_funcs = HashSet::new();

    // Greek letters
    let tokens1 = lex("\u{3b1}\u{3b2}", &fixed_vars, &custom_funcs).unwrap();
    assert_eq!(tokens1.len(), 1);
    if let Token::Identifier(s) = &tokens1[0] {
        assert_eq!(s, "\u{3b1}\u{3b2}"); // Should be treated as single identifier
    } else {
        panic!("Expected Identifier token");
    }

    // Greek letter with subscript (if using Unicode subscripts)
    let tokens2 = lex("\u{3b8}\u{2081}", &fixed_vars, &custom_funcs).unwrap();
    assert_eq!(tokens2.len(), 1);
    if let Token::Identifier(s) = &tokens2[0] {
        assert_eq!(s, "\u{3b8}\u{2081}");
    } else {
        panic!("Expected Identifier token");
    }
}

#[test]
fn test_lexer_identifier_rules() {
    assert!(is_identifier_continue('x'));
    assert!(is_identifier_continue('1'));
    assert!(is_identifier_continue('_'));
    assert!(is_identifier_continue('\u{3b2}'));
}

#[test]
fn test_lexer_derivative_validation() {
    let fixed_vars = HashSet::new();
    let custom_funcs = HashSet::new();

    // Test invalid derivative order
    let result1 = parse_derivative_notation(
        "\u{2202}^999999_f(x)/\u{2202}_x",
        &fixed_vars,
        &custom_funcs,
    );
    assert!(result1.is_err());

    // Test malformed derivative
    let result2 = parse_derivative_notation(
        "\u{2202}_f(x", // Missing closing paren and /∂_x
        &fixed_vars,
        &custom_funcs,
    );
    assert!(result2.is_err());
}

#[test]
fn test_lexer_builtins_operator_sync() {
    let builtin_set = get_builtins_set();

    // Test a representative sample of functions from each category
    let test_functions = vec![
        // Trig
        "sin",
        "cos",
        "tan",
        "cot",
        "sec",
        "csc",
        // Inverse trig
        "asin",
        "acos",
        "atan",
        "acot",
        "asec",
        "acsc",
        // Hyperbolic
        "sinh",
        "cosh",
        "tanh",
        "coth",
        "sech",
        "csch",
        // Inverse hyperbolic
        "asinh",
        "acosh",
        "atanh",
        "acoth",
        "asech",
        "acsch",
        // Logarithmic
        "ln",
        "exp",
        "log",
        "log10",
        "log2",
        // Roots
        "sqrt",
        "cbrt",
        // Special
        "sinc",
        "abs",
        "signum",
        "floor",
        "ceil",
        "round",
        "erf",
        "erfc",
        "gamma",
        "lgamma",
        "digamma",
        "trigamma",
        "tetragamma",
        "polygamma",
        "beta",
        "zeta",
        "zeta_deriv",
        "besselj",
        "bessely",
        "besseli",
        "besselk",
        "lambertw",
        "ynm",
        "spherical_harmonic",
        "assoc_legendre",
        "hermite",
        "elliptic_e",
        "elliptic_k",
        "exp_polar",
        "atan2",
    ];

    for func_name in test_functions {
        let op = Operator::parse_str(func_name);
        assert!(
            op.is_some(),
            "Operator::parse_str should recognize '{func_name}'"
        );

        if func_name != "sen" && func_name != "sign" && func_name != "sgn" {
            assert!(
                builtin_set.contains(func_name),
                "BUILTINS array missing function '{func_name}' - add it to the BUILTINS array in lexer.rs"
            );
        }
    }

    assert!(!builtin_set.contains("+"));
    assert!(!builtin_set.contains("-"));
    assert!(!builtin_set.contains("*"));
    assert!(!builtin_set.contains("/"));
    assert!(!builtin_set.contains("^"));
}
