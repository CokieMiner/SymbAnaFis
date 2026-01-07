//! Lexer implementation - two-pass context-aware tokenization
//!
//! # Robustness Improvements (2024)
//!
//! This lexer has been refactored to improve maintainability and robustness:
//!
//! 1. **Number Parsing**: Simplified to rely on `f64::parse()` for validation
//!    - Removed redundant manual sign handling in scientific notation
//!    - Cleaner character class checking (digits, '.', 'e', 'E', '+', '-')
//!
//! 2. **Derivative Notation**: Extracted into dedicated `scan_derivative_notation()`
//!    - Better isolation of complex parenthesis-balancing logic
//!    - Improved error messages for malformed derivative expressions
//!    - More testable and maintainable code structure
//!
//! 3. **Sequence Resolution**: Documented priority order and tradeoffs
//!    - Implicit multiplication (e.g., "xsin(y)" → "x * sin(y)") is heuristic-based
//!    - Users can disambiguate by using explicit operators or declaring `fixed_vars`
//
use crate::DiffError;
use crate::parser::tokens::{Operator, Token};
use std::cmp::Ordering;
use std::collections::HashSet;

// DESIGN NOTE: BUILTINS vs Operator enum
// ======================================
// This list duplicates function names from the Operator enum in tokens.rs.
// This is INTENTIONAL because they serve different purposes:
// - BUILTINS: String matching during lexical analysis (fast lookup for tokenization)
// - Operator: Type-safe representation after tokenization
//
// Keeping them separate allows the lexer to work with raw strings without needing
// to know about the Operator type's internal structure. If you add a new function,
// add it to BOTH this list AND the Operator enum in tokens.rs.
const BUILTINS: &[&str] = &[
    "sin",
    "sen",
    "cos",
    "tan",
    "cot",
    "sec",
    "csc",
    "asin",
    "acos",
    "atan",
    "acot",
    "asec",
    "acsc",
    "ln",
    "exp",
    "log",
    "log10",
    "log2",
    "exp_polar",
    "sinh",
    "cosh",
    "tanh",
    "coth",
    "sech",
    "csch",
    "asinh",
    "acosh",
    "atanh",
    "acoth",
    "asech",
    "acsch",
    "sqrt",
    "cbrt",
    "sinc",
    "abs",
    "signum",
    "floor",
    "ceil",
    "round",
    "erf",
    "erfc",
    "gamma",
    "digamma",
    "trigamma",
    "tetragamma",
    "polygamma",
    "beta",
    "zeta",
    "besselj",
    "bessely",
    "besseli",
    "besselk",
    "lambertw",
    "ynm",
    "assoc_legendre",
    "hermite",
    "elliptic_e",
    "elliptic_k",
    "zeta_deriv",
    "atan2",
    "spherical_harmonic",
];

use std::sync::OnceLock;

/// Static `HashSet` for O(1) builtin function lookup
/// Initialized once on first access
static BUILTINS_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();

fn get_builtins_set() -> &'static HashSet<&'static str> {
    BUILTINS_SET.get_or_init(|| BUILTINS.iter().copied().collect())
}

/// Check if a string is a builtin function name (O(1) lookup)
#[inline]
fn is_builtin(name: &str) -> bool {
    get_builtins_set().contains(name)
}

/// Check if a character is a valid identifier continuation character
/// Following Rust-like identifier rules: alphanumeric or underscore
#[inline]
fn is_identifier_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Balance parentheses in the input string
pub fn balance_parentheses(input: &str) -> String {
    let open_count = input.chars().filter(|&c| c == '(').count();
    let close_count = input.chars().filter(|&c| c == ')').count();

    match open_count.cmp(&close_count) {
        Ordering::Greater => {
            // More ( than ) → append ) at end
            let missing = open_count - close_count;
            format!("{}{}", input, ")".repeat(missing))
        }
        Ordering::Less => {
            // More ) than ( → prepend ( at start
            let missing = close_count - open_count;
            format!("{}{}", "(".repeat(missing), input)
        }
        Ordering::Equal => {
            // Check for wrong order (e.g., ")(x" → "()(x)")
            let mut depth = 0;
            for c in input.chars() {
                if c == '(' {
                    depth += 1;
                } else if c == ')' {
                    depth -= 1;
                    if depth < 0 {
                        // Closing before opening
                        return format!("({input})");
                    }
                }
            }
            input.to_string()
        }
    }
}

/// Parse a number string to f64
/// Supports: integers (3), decimals (3.14), scientific notation (1e10, 2.5e-3)
///
/// **Locale Support**: Only dot (.) is supported as decimal separator.
/// Comma-based decimals (e.g., "3,14" in some locales) are NOT supported.
/// This is intentional to avoid ambiguity with function argument separators.
///
/// For international/locale-aware parsing, preprocess input to normalize
/// decimal separators before calling this function.
fn parse_number(s: &str) -> Result<f64, DiffError> {
    // Check for multiple decimal separators
    let dot_count = s.chars().filter(|&c| c == '.').count();

    if dot_count > 1 {
        return Err(DiffError::invalid_number(format!(
            "'{s}' (multiple decimal points found)"
        )));
    }

    // Parse with f64 (handles scientific notation automatically)
    s.parse::<f64>().map_err(|_| {
        DiffError::invalid_number(format!(
            "'{s}' (use dot as decimal separator, e.g., '3.14' not '3,14')"
        ))
    })
}

/// Raw token before symbol resolution
#[derive(Debug, Clone)]
enum RawToken {
    Number(f64),
    Sequence(String),   // Multi-char sequence to be resolved
    Operator(char),     // Single-char operator: +, *, ^
    Derivative(String), // Derivative notation starting with ∂
    LeftParen,
    RightParen,
    Comma,
}

/// Helper: Scan derivative notation (∂...) with balanced parentheses
/// Returns the complete derivative string (including the leading ∂)
///
/// This handles complex derivative notation like: ∂_f(x+y)/∂_x
/// by tracking parenthesis depth to ensure balanced expressions.
fn scan_derivative_notation(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> Result<String, DiffError> {
    let mut seq = String::new();
    let mut depth = 0;

    // chars is already positioned at '∂', consume it
    if chars.peek() == Some(&'∂') {
        seq.push(chars.next().unwrap());
    }

    while let Some(&next_c) = chars.peek() {
        // Decision logic based on CURRENT depth
        let accept = if depth > 0 {
            // Inside parens, accept everything except unmatched closing
            true
        } else {
            // Top level: accept specific chars for derivative notation
            next_c.is_alphanumeric()
                || next_c == '_'
                || next_c == '∂'
                || next_c == '^'
                || next_c == '/'
                || next_c == '('
                || next_c == ')'
                || next_c == ','
        };

        if !accept {
            break;
        }

        seq.push(next_c);
        chars.next(); // Consume only if accepted

        // Update depth AFTER consuming
        match next_c {
            '(' => depth += 1,
            ')' => {
                if depth > 0 {
                    depth -= 1;
                } else {
                    // Unmatched closing paren at top level - stop here
                    break;
                }
            }
            _ => {}
        }
    }

    // Basic validation: derivative notation should contain more than just '∂'
    if seq.len() <= 1 {
        return Err(DiffError::invalid_token(format!(
            "Incomplete derivative notation: {seq}"
        )));
    }

    Ok(seq)
}

/// Pass 1: Scan characters and create raw tokens
/// Now tracks byte positions for better error reporting
#[allow(clippy::too_many_lines)]
fn scan_characters(input: &str) -> Result<Vec<RawToken>, DiffError> {
    // Estimate capacity: rough heuristic (input.len() / 2) to minimize reallocations
    let mut tokens = Vec::with_capacity(input.len() / 2);
    let bytes = input.as_bytes();
    let mut pos = 0; // Current byte position

    while pos < bytes.len() {
        let ch = bytes[pos] as char;

        // Skip whitespace (handle UTF-8 properly for whitespace)
        if ch.is_ascii_whitespace() {
            pos += 1;
            continue;
        }

        match ch {
            // Parentheses and Comma
            '(' => {
                tokens.push(RawToken::LeftParen);
                pos += 1;
            }
            ')' => {
                tokens.push(RawToken::RightParen);
                pos += 1;
            }
            ',' => {
                tokens.push(RawToken::Comma);
                pos += 1;
            }

            // Single-char operators
            '+' => {
                tokens.push(RawToken::Operator('+'));
                pos += 1;
            }

            '-' => {
                tokens.push(RawToken::Operator('-'));
                pos += 1;
            }

            '/' => {
                tokens.push(RawToken::Operator('/'));
                pos += 1;
            }

            // Multiplication or power (**)
            '*' => {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'*' {
                    pos += 1;
                    tokens.push(RawToken::Operator('^')); // Treat ** as ^
                } else {
                    tokens.push(RawToken::Operator('*'));
                }
            }

            // Power
            '^' => {
                tokens.push(RawToken::Operator('^'));
                pos += 1;
            }

            // Numbers and scientific notation
            '0'..='9' | '.' => {
                let start_pos = pos;
                let mut num_str = String::new();
                let mut has_exponent = false;

                while pos < bytes.len() {
                    let c = bytes[pos] as char;
                    // Simple character class check - let f64::parse handle validation
                    match c {
                        '0'..='9' | '.' => {
                            num_str.push(c);
                            pos += 1;
                        }
                        'e' | 'E' if !has_exponent => {
                            has_exponent = true;
                            num_str.push(c);
                            pos += 1;
                            // Handle optional sign after exponent
                            if pos < bytes.len() && (bytes[pos] == b'+' || bytes[pos] == b'-') {
                                num_str.push(bytes[pos] as char);
                                pos += 1;
                            }
                        }
                        _ => break,
                    }
                }

                match parse_number(&num_str) {
                    Ok(n) => tokens.push(RawToken::Number(n)),
                    Err(_) => {
                        return Err(DiffError::InvalidNumber {
                            value: num_str,
                            span: Some(crate::Span::new(start_pos, pos)),
                        });
                    }
                }
            }

            // Unicode derivatives (∂)
            _ if input[pos..].starts_with('∂') => {
                let mut remaining_chars = input[pos..].chars().peekable();
                let deriv_str = scan_derivative_notation(&mut remaining_chars)?;
                pos += deriv_str.len(); // Advance pos by the byte length of the derivative string
                tokens.push(RawToken::Derivative(deriv_str));
            }

            // Unicode pi (π) - use remaining string for UTF-8 safety
            _ if input[pos..].starts_with('π') => {
                tokens.push(RawToken::Sequence("pi".to_string()));
                pos += 'π'.len_utf8();
            }

            // Unicode and ASCII identifier sequences
            // Use the remaining string slice for proper UTF-8 handling
            _ if input[pos..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_') =>
            {
                let mut seq = String::new();
                for ch in input[pos..].chars() {
                    if ch.is_alphanumeric() || ch == '_' {
                        seq.push(ch);
                        pos += ch.len_utf8();
                    } else {
                        break;
                    }
                }
                tokens.push(RawToken::Sequence(seq));
            }

            _ => {
                // For invalid tokens, include position information
                return Err(DiffError::InvalidToken {
                    token: ch.to_string(),
                    span: Some(crate::Span::new(pos, pos + 1)),
                });
            }
        }
    }

    Ok(tokens)
}

/// Pass 2: Resolve sequences into tokens using context
pub fn lex<S: std::hash::BuildHasher>(
    input: &str,
    fixed_vars: &HashSet<String, S>,
    custom_functions: &HashSet<String, S>,
) -> Result<Vec<Token>, DiffError> {
    let raw_tokens = scan_characters(input)?;
    // Optimization: Pre-allocate capacity roughly matching raw tokens count
    let mut tokens = Vec::with_capacity(raw_tokens.len());

    for i in 0..raw_tokens.len() {
        match &raw_tokens[i] {
            RawToken::Number(n) => tokens.push(Token::Number(*n)),
            RawToken::LeftParen => tokens.push(Token::LeftParen),
            RawToken::RightParen => tokens.push(Token::RightParen),
            RawToken::Comma => tokens.push(Token::Comma),

            RawToken::Operator(c) => {
                let op = match c {
                    '+' => Operator::Add,
                    '-' => Operator::Sub,
                    '*' => Operator::Mul,
                    '/' => Operator::Div,
                    '^' => Operator::Pow,
                    _ => return Err(DiffError::invalid_token(c.to_string())),
                };
                tokens.push(Token::Operator(op));
            }

            RawToken::Derivative(deriv_str) => {
                match parse_derivative_notation(deriv_str, fixed_vars, custom_functions) {
                    Ok(deriv_token) => tokens.push(deriv_token),
                    Err(_) => return Err(DiffError::invalid_token(deriv_str.clone())),
                }
            }

            RawToken::Sequence(seq) => {
                let next_is_paren =
                    i + 1 < raw_tokens.len() && matches!(raw_tokens[i + 1], RawToken::LeftParen);
                let resolved = resolve_sequence(seq, fixed_vars, custom_functions, next_is_paren);
                tokens.extend(resolved);
            }
        }
    }

    Ok(tokens)
}

/// Resolve a sequence into tokens based on context
///
/// This function implements a priority-based disambiguation strategy for
/// identifier sequences that may contain implicit multiplication.
///
/// **Priority Order:**
/// 1. Exact match in `fixed_vars` (user-declared multi-char variables)
/// 2. Known mathematical constants (pi, e)
/// 3. Built-in functions when followed by `(`
/// 4. Custom functions when followed by `(`
/// 5. Suffix scan for built-ins (e.g., "xsin" → "x", "sin" if followed by `(`)
/// 6. Prefix scan for fixed variables
/// 7. Character-by-character split (final fallback)
///
/// **Design Tradeoff:**
/// The suffix/prefix scanning enables natural notation like `xsin(y)` → `x * sin(y)`,
/// but introduces potential ambiguity. Users can resolve ambiguity by:
/// - Declaring multi-char variables in `fixed_vars`
/// - Using explicit multiplication: `x*sin(y)`
fn resolve_sequence<S: std::hash::BuildHasher>(
    seq: &str,
    fixed_vars: &HashSet<String, S>,
    custom_functions: &HashSet<String, S>,
    next_is_paren: bool,
) -> Vec<Token> {
    // Priority 1: Check if entire sequence is in fixed_vars
    if fixed_vars.contains(seq) {
        return vec![Token::Identifier(seq.to_string())];
    }

    // Priority 1.5: Check for known constants (pi, e)
    if seq == "pi" || seq == "e" {
        return vec![Token::Identifier(seq.to_string())];
    }

    // Priority 2: Check if it's a built-in function followed by (
    // Uses O(1) HashSet lookup via is_builtin()
    if is_builtin(seq)
        && next_is_paren
        && let Some(op) = Operator::parse_str(seq)
    {
        return vec![Token::Operator(op)];
    }

    // Priority 3: Check if it's a custom function followed by (
    if custom_functions.contains(seq) && next_is_paren {
        return vec![Token::Identifier(seq.to_string())];
    }

    // Priority 4: Scan for built-in functions as substrings (if followed by paren)
    // Optimized: Search using length-sorted builtins to avoid O(n^2) scan
    if next_is_paren {
        // Use LazyLock for thread-safe, one-time initialization (cleaner than OnceLock)
        static BUILTINS_SORTED: std::sync::LazyLock<Vec<&'static str>> =
            std::sync::LazyLock::new(|| {
                let mut b = BUILTINS.to_vec();
                b.sort_by_key(|b| std::cmp::Reverse(b.len())); // Descending length
                b
            });

        // Check suffixes first (e.g. "xsin" -> "x", "sin")
        for builtin in BUILTINS_SORTED.iter() {
            if seq.ends_with(builtin) && seq.len() > builtin.len() {
                // Found builtin at the end
                let split_idx = seq.len() - builtin.len();
                let before = &seq[0..split_idx];

                let mut tokens = Vec::with_capacity(2);

                // Recursively resolve the part before
                tokens.extend(resolve_sequence(
                    before,
                    fixed_vars,
                    custom_functions,
                    false,
                ));

                // Add the built-in function
                if let Some(op) = Operator::parse_str(builtin) {
                    tokens.push(Token::Operator(op));
                }

                return tokens;
            }
        }
    }

    // Priority 5 (FALLBACK): Check if it's a valid identifier sequence
    // Unicode-aware: uses is_identifier_continue which accepts alphabetic + '_'
    if seq.chars().all(is_identifier_continue) {
        // Check if any prefix is a fixed variable
        // Use char_indices for Unicode-safe slicing
        let char_boundaries: Vec<usize> = seq.char_indices().map(|(i, _)| i).collect();
        for (char_idx, _byte_idx) in char_boundaries.iter().enumerate() {
            // Include end of string as a valid slice point
            let end_byte = if char_idx + 1 < char_boundaries.len() {
                char_boundaries[char_idx + 1]
            } else {
                seq.len()
            };
            let prefix = &seq[0..end_byte];
            if fixed_vars.contains(prefix) {
                // Found a fixed variable prefix, split here
                let rest = &seq[end_byte..];
                let mut tokens = vec![Token::Identifier(prefix.to_string())];
                if !rest.is_empty() {
                    // Recursively resolve the rest
                    tokens.extend(resolve_sequence(
                        rest,
                        fixed_vars,
                        custom_functions,
                        next_is_paren && end_byte == seq.len(),
                    ));
                }
                return tokens;
            }
        }

        // No fixed variable prefix found, treat as single identifier
        // This preserves multi-character Unicode identifiers like "αβ" or "θ₁"
        return vec![Token::Identifier(seq.to_string())];
    }

    // Priority 6 (FINAL FALLBACK): Split into individual characters (for complex sequences)
    seq.chars()
        .map(|c| Token::Identifier(c.to_string()))
        .collect()
}

/// Parse derivative notation like ∂^`1_f(x)`/∂_x^1
///
/// **Safety**: Validates format before recursively lexing arguments to prevent
/// infinite recursion on malformed input.
fn parse_derivative_notation<S: std::hash::BuildHasher>(
    s: &str,
    fixed_vars: &HashSet<String, S>,
    custom_functions: &HashSet<String, S>,
) -> Result<Token, DiffError> {
    // Format: ∂^order_func(args)/∂_var^order
    if !s.starts_with("∂") || !s.contains("/∂_") {
        return Err(DiffError::invalid_token(format!(
            "Invalid derivative notation '{s}': must start with '∂' and contain '/∂_'"
        )));
    }

    let parts: Vec<&str> = s.split("/∂_").collect();
    if parts.len() != 2 {
        return Err(DiffError::invalid_token(format!(
            "Invalid derivative notation '{}': expected exactly one '/∂_' separator, found {}",
            s,
            parts.len() - 1
        )));
    }

    let left = parts[0]; // ∂^order_func(args) or ∂_func(args)
    let right = parts[1]; // var^order or var

    // Parse left side
    let left_parts: Vec<&str> = left.split('_').collect();
    if left_parts.len() < 2 {
        return Err(DiffError::invalid_token(format!(
            "Invalid derivative notation '{s}': left side must contain underscore separator"
        )));
    }

    // Parse order from left (defaults to 1 if not specified)
    // e.g., "∂^n" -> n, "∂" -> 1
    // Note: '∂' is 3 bytes in UTF-8, '^' is 1 byte = 4 bytes total to skip
    let order_str = if left_parts[0].starts_with("∂^") {
        &left_parts[0][4..] // Skip "∂^" (3 + 1 = 4 bytes)
    } else if left_parts[0] == "∂" {
        "1"
    } else {
        return Err(DiffError::invalid_token(format!(
            "Invalid derivative notation '{s}': expected '∂' or '∂^order'"
        )));
    };

    let order = order_str.parse::<u32>().map_err(|_| {
        DiffError::invalid_token(format!(
            "Invalid derivative notation '{s}': order '{order_str}' is not a valid number"
        ))
    })?;

    // Safety check: prevent unreasonably high derivative orders
    if order > 100 {
        return Err(DiffError::invalid_token(format!(
            "Invalid derivative notation '{s}': order {order} exceeds maximum (100)"
        )));
    }

    // Parse function name and args
    // left_parts[1] is "func(args)" or "func"
    let func_str = left_parts[1];
    let (func_name, args_tokens) = if let Some(open_paren) = func_str.find('(') {
        if func_str.ends_with(')') {
            let name = func_str[..open_paren].to_string();
            let args_body = &func_str[open_paren + 1..func_str.len() - 1];

            // Safety: prevent infinite recursion on malformed nested derivatives
            // Check if we're about to recursively parse another derivative
            if args_body.contains("∂") && args_body.contains("/∂_") {
                // This is likely a nested derivative - limit recursion depth
                // by ensuring the args don't contain another full derivative notation
                let nested_depth = args_body.matches("/∂_").count();
                if nested_depth > 3 {
                    return Err(DiffError::invalid_token(format!(
                        "Invalid derivative notation '{s}': nested derivative depth exceeds limit"
                    )));
                }
            }

            // Recursively lex the arguments
            let tokens = lex(args_body, fixed_vars, custom_functions)?;
            (name, tokens)
        } else {
            return Err(DiffError::invalid_token(format!(
                "Invalid derivative notation '{s}': unmatched parentheses in function arguments"
            )));
        }
    } else {
        (func_str.to_string(), Vec::new())
    };

    // Parse right side: var^order or var
    let right_parts: Vec<&str> = right.split('^').collect();
    let var = right_parts[0].to_string();

    Ok(Token::Derivative {
        order,
        func: func_name,
        args: args_tokens,
        var,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_balance_parentheses() {
        assert_eq!(balance_parentheses("(x + 1"), "(x + 1)");
        assert_eq!(balance_parentheses("x + 1)"), "(x + 1)");
        assert_eq!(balance_parentheses("(x)"), "(x)");
        assert_eq!(balance_parentheses(")(x"), "()(x)");
    }

    #[test]
    fn test_parse_number() {
        assert_eq!(parse_number("3.14").unwrap(), 314.0 / 100.0);
        assert_eq!(parse_number("1e10").unwrap(), 1e10);
        assert_eq!(parse_number("2.5e-3").unwrap(), 0.0025);
        assert!(parse_number("3.14.15").is_err());
    }

    #[test]
    fn test_scan_derivative_complex() {
        // Test explicitly the scanning of complex derivative string
        let input = "∂_f(x+y)/∂_x";
        let tokens = scan_characters(input).unwrap();
        assert_eq!(tokens.len(), 1);
        if let RawToken::Derivative(s) = &tokens[0] {
            assert_eq!(s, "∂_f(x+y)/∂_x");
        } else {
            panic!("Expected Derivative token, got {:?}", tokens[0]);
        }
    }

    #[test]
    fn test_scan_derivative_nested_parens() {
        // Test derivative with deeply nested parentheses
        let input = "∂_f((x+y)*(z-w))/∂_x";
        let tokens = scan_characters(input).unwrap();
        assert_eq!(tokens.len(), 1);
        if let RawToken::Derivative(s) = &tokens[0] {
            assert_eq!(s, "∂_f((x+y)*(z-w))/∂_x");
        } else {
            panic!("Expected Derivative token, got {:?}", tokens[0]);
        }
    }

    #[test]
    fn test_number_scientific_notation() {
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
                    "Failed for {}: expected {}, got {}",
                    input,
                    expected,
                    n
                );
            } else {
                panic!("Expected Number token for {}", input);
            }
        }
    }

    #[test]
    fn test_number_edge_cases() {
        // Test that edge cases are handled correctly
        // A lone dot without digits should fail validation in parse_number
        assert!(scan_characters(".").is_err() || parse_number(".").is_err());

        // Leading sign should NOT be part of number token
        let tokens = scan_characters("-123").unwrap();
        assert_eq!(tokens.len(), 2); // Operator(-) and Number(123)
    }

    #[test]
    fn test_scan_characters() {
        let result = scan_characters("x + 1").unwrap();
        assert_eq!(result.len(), 3);

        let result = scan_characters("sin(x)").unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_lex_basic() {
        let fixed_vars = HashSet::new();
        let custom_funcs = HashSet::new();

        let tokens = lex("x", &fixed_vars, &custom_funcs).unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0], Token::Identifier(_)));
    }

    #[test]
    fn test_lex_with_fixed_vars() {
        let mut fixed_vars = HashSet::new();
        fixed_vars.insert("ax".to_string());
        let custom_funcs = HashSet::new();

        let tokens = lex("ax", &fixed_vars, &custom_funcs).unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Identifier("ax".to_string()));
    }

    #[test]
    fn test_empty_parens() {
        let fixed_vars = HashSet::new();
        let custom_funcs = HashSet::new();

        let tokens = lex("()", &fixed_vars, &custom_funcs).unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0], Token::LeftParen));
        assert!(matches!(tokens[1], Token::RightParen));
    }

    #[test]
    fn test_unicode_identifiers() {
        // Test that multi-char Unicode identifiers work correctly
        let fixed_vars = HashSet::new();
        let custom_funcs = HashSet::new();

        // Greek letters
        let tokens = lex("αβ", &fixed_vars, &custom_funcs).unwrap();
        assert_eq!(tokens.len(), 1);
        if let Token::Identifier(s) = &tokens[0] {
            assert_eq!(s, "αβ"); // Should be treated as single identifier
        } else {
            panic!("Expected Identifier token");
        }

        // Greek letter with subscript (if using Unicode subscripts)
        let tokens = lex("θ₁", &fixed_vars, &custom_funcs).unwrap();
        assert_eq!(tokens.len(), 1);
        if let Token::Identifier(s) = &tokens[0] {
            assert_eq!(s, "θ₁");
        } else {
            panic!("Expected Identifier token");
        }
    }

    #[test]
    fn test_identifier_rules() {
        assert!(is_identifier_continue('x'));
        assert!(is_identifier_continue('1'));
        assert!(is_identifier_continue('_'));
        assert!(is_identifier_continue('β'));
    }

    #[test]
    fn test_derivative_validation() {
        let fixed_vars = HashSet::new();
        let custom_funcs = HashSet::new();

        // Test invalid derivative order
        let result = parse_derivative_notation("∂^999999_f(x)/∂_x", &fixed_vars, &custom_funcs);
        assert!(result.is_err());

        // Test malformed derivative
        let result = parse_derivative_notation(
            "∂_f(x", // Missing closing paren and /∂_x
            &fixed_vars,
            &custom_funcs,
        );
        assert!(result.is_err());
    }
}
