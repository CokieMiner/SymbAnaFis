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
use std::borrow::Cow;
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
    "spherical_harmonic",
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
pub fn balance_parentheses(input: &str) -> Cow<'_, str> {
    let open_count = input.chars().filter(|&c| c == '(').count();
    let close_count = input.chars().filter(|&c| c == ')').count();

    match open_count.cmp(&close_count) {
        Ordering::Greater => {
            // More ( than ) → append ) at end
            let missing = open_count - close_count;
            Cow::Owned(format!("{}{}", input, ")".repeat(missing)))
        }
        Ordering::Less => {
            // More ) than ( → prepend ( at start
            let missing = close_count - open_count;
            Cow::Owned(format!("{}{}", "(".repeat(missing), input))
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
                        return Cow::Owned(format!("({input})"));
                    }
                }
            }
            Cow::Borrowed(input)
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
    // Remove underscores for parsing (standard in many languages/parsers)
    let sanitized = s.replace('_', "");

    // Check for multiple decimal separators
    let dot_count = sanitized.chars().filter(|&c| c == '.').count();

    if dot_count > 1 {
        return Err(DiffError::invalid_number(format!(
            "'{s}' (multiple decimal points found)"
        )));
    }

    // Parse with f64 (handles scientific notation automatically)
    sanitized.parse::<f64>().map_err(|_e| {
        DiffError::invalid_number(format!(
            "'{s}' (use dot as decimal separator, e.g., '3.14' not '3,14')"
        ))
    })
}

/// Raw token before symbol resolution
#[derive(Debug, Clone)]
enum RawToken<'src> {
    Number(f64),
    Sequence(Cow<'src, str>),   // Multi-char sequence to be resolved
    Operator(char),     // Single-char operator: +, *, ^
    Derivative(Cow<'src, str>), // Derivative notation starting with ∂
    LeftParen,
    RightParen,
    Comma,
}

/// Helper: Scan derivative notation (∂...) with balanced parentheses
/// Returns the complete derivative string (including the leading ∂)
///
/// This handles complex derivative notation like: ∂_f(x+y)/∂_x
/// by tracking parenthesis depth to ensure balanced expressions.
#[allow(clippy::string_slice, reason = "Slicing is safe because len is calculated from utf8 lengths")]
fn scan_derivative_notation(input: &str) -> Result<&str, DiffError> {
    let mut chars = input.chars().peekable();
    let mut len = 0;
    let mut depth = 0;

    // chars is already positioned at start of input. Check for ∂
    if chars.peek() == Some(&'\u{2202}') {
        let c = chars.next().expect("Checked peek");
        len += c.len_utf8();
    } else {
        // This function expects to be called when ∂ is detected
        // but we handle the case where it's not starting with it if needed,
        // though the caller (lexer) checks it.
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
                || next_c == '\u{2202}'
                || next_c == '^'
                || next_c == '/'
                || next_c == '('
                || next_c == ')'
                || next_c == ','
        };

        if !accept {
            break;
        }

        chars.next(); // Consume
        len += next_c.len_utf8();

        // Update depth AFTER consuming
        match next_c {
            '(' => depth += 1,
            ')' => {
                if depth > 0 {
                    depth -= 1;
                } else {
                    // Unmatched closing paren at top level - stop here
                    // This logic mirrors the original: break on unmatched closing paren
                    // However, we consumed it.
                    // "Unmatched closing paren at top level - stop here"
                    // Original code:
                    // seq.push(next_c); chars.next();
                    // if depth > 0 { depth -= 1 } else { break }
                    // So it INCLUDED the closing paren in seq, then broke loop.
                    // Wait, if depth == 0, break.
                    // So ")(x" -> seq includes ")".
                    // But then it breaks.
                    // Let's verify original behavior.
                    break;
                }
            }
            _ => {}
        }
    }

    let seq = &input[..len];

    // Basic validation: derivative notation should contain more than just '∂'
    if seq.len() <= '\u{2202}'.len_utf8() {
        return Err(DiffError::invalid_token(format!(
            "Incomplete derivative notation: {seq}"
        )));
    }

    Ok(seq)
}

/// Pass 1: Scan characters and create raw tokens
/// Now tracks byte positions for better error reporting
#[allow(
    clippy::too_many_lines,
    clippy::string_slice,
    reason = "Complex character scanning logic requires comprehensive tokenization and safe slicing is manually verified"
)]
fn scan_characters(input: &str) -> Result<Vec<RawToken<'_>>, DiffError> {
    // Estimate capacity: rough heuristic (input.len() / 2) to minimize reallocations
    // Heuristic: assume average token length is ~2 characters
    #[allow(
        clippy::integer_division,
        reason = "Integer division for capacity estimation in token vector"
    )]
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
                let mut has_exponent = false;

                while pos < bytes.len() {
                    let c = bytes[pos] as char;
                    // Simple character class check - let f64::parse handle validation
                    match c {
                        '0'..='9' | '.' | '_' => {
                            pos += 1;
                        }
                        'e' | 'E' if !has_exponent => {
                            has_exponent = true;
                            pos += 1;
                            // Handle optional sign after exponent
                            if pos < bytes.len() && (bytes[pos] == b'+' || bytes[pos] == b'-') {
                                pos += 1;
                            }
                        }
                        _ => break,
                    }
                }

                let num_str = &input[start_pos..pos];
                match parse_number(num_str) {
                    Ok(n) => tokens.push(RawToken::Number(n)),
                    Err(_) => {
                        return Err(DiffError::InvalidNumber {
                            value: num_str.to_owned(),
                            span: Some(crate::Span::new(start_pos, pos)),
                        });
                    }
                }
            }

            // Unicode derivatives (∂)
            _ if input.get(pos..).is_some_and(|s| s.starts_with('\u{2202}')) => {
                let remaining_str = input.get(pos..).expect("Checked in guard");
                let deriv_str = scan_derivative_notation(remaining_str)?;
                pos += deriv_str.len(); // Advance pos by the byte length of the derivative string
                tokens.push(RawToken::Derivative(Cow::Borrowed(deriv_str)));
            }

            // Unicode pi (π) - use remaining string for UTF-8 safety
            _ if input.get(pos..).is_some_and(|s| s.starts_with('\u{3c0}')) => {
                tokens.push(RawToken::Sequence(Cow::Borrowed("pi")));
                pos += '\u{3c0}'.len_utf8();
            }

            // Unicode and ASCII identifier sequences
            // Use the remaining string slice for proper UTF-8 handling
            _ if input
                .get(pos..)
                .and_then(|s| s.chars().next())
                .is_some_and(|c| c.is_alphabetic() || c == '_') =>
            {
                let start_pos = pos;
                let remaining = input.get(pos..).expect("Checked in guard");
                for current_char in remaining.chars() {
                    if current_char.is_alphanumeric() || current_char == '_' {
                        pos += current_char.len_utf8();
                    } else {
                        break;
                    }
                }
                tokens.push(RawToken::Sequence(Cow::Borrowed(&input[start_pos..pos])));
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
pub fn lex<'src, S: std::hash::BuildHasher>(
    input: &'src str,
    fixed_vars: &HashSet<String, S>,
    custom_functions: &HashSet<String, S>,
) -> Result<Vec<Token<'src>>, DiffError> {
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
                // deriv_str is Cow::Borrowed from input (guaranteed by scan_characters)
                let s = match deriv_str {
                    Cow::Borrowed(s) => s,
                    Cow::Owned(s) => return Err(DiffError::invalid_token(s.clone())),
                };
                match parse_derivative_notation(s, fixed_vars, custom_functions) {
                    Ok(deriv_token) => tokens.push(deriv_token),
                    Err(_) => return Err(DiffError::invalid_token(s.to_string())),
                }
            }

            RawToken::Sequence(seq) => {
                let next_is_paren =
                    i + 1 < raw_tokens.len() && matches!(raw_tokens[i + 1], RawToken::LeftParen);
                
                // seq is Cow::Borrowed from input (guaranteed by scan_characters)
                let s = match seq {
                    Cow::Borrowed(s) => s,
                    Cow::Owned(s) => return Err(DiffError::invalid_token(s.clone())),
                };
                resolve_sequence(s, fixed_vars, custom_functions, next_is_paren, &mut tokens);
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
#[allow(clippy::string_slice, reason = "Slicing by char indices yields valid UTF-8 sequences")]
fn resolve_sequence<'src, S: std::hash::BuildHasher>(
    seq: &'src str,
    fixed_vars: &HashSet<String, S>,
    custom_functions: &HashSet<String, S>,
    next_is_paren: bool,
    output: &mut Vec<Token<'src>>,
) {
    // Priority 1: Check if entire sequence is in fixed_vars
    if fixed_vars.contains(seq) {
        output.push(Token::Identifier(Cow::Borrowed(seq)));
        return;
    }

    // Priority 1.5: Check for known constants (pi, e)
    if crate::core::known_symbols::is_known_constant(seq) {
        output.push(Token::Identifier(Cow::Borrowed(seq)));
        return;
    }

    // Priority 2: Check if it's a built-in function followed by (
    // Uses O(1) HashSet lookup via is_builtin()
    if is_builtin(seq)
        && next_is_paren
        && let Some(op) = Operator::parse_str(seq)
    {
        output.push(Token::Operator(op));
        return;
    }

    // Priority 3: Check if it's a custom function followed by (
    if custom_functions.contains(seq) && next_is_paren {
        output.push(Token::Identifier(Cow::Borrowed(seq)));
        return;
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
                let before = seq.get(0..split_idx).expect("Checked suffix boundaries");

                // Recursively resolve the part before
                resolve_sequence(
                    before,
                    fixed_vars,
                    custom_functions,
                    false,
                    output,
                );

                // Add the built-in function
                if let Some(op) = Operator::parse_str(builtin) {
                    output.push(Token::Operator(op));
                }

                return;
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
            let prefix = seq.get(0..end_byte).expect("Checked char boundaries");
            if fixed_vars.contains(prefix) {
                // Found a fixed variable prefix, split here
                let rest = seq.get(end_byte..).expect("Checked char boundaries");
                output.push(Token::Identifier(Cow::Borrowed(prefix)));
                if !rest.is_empty() {
                    // Recursively resolve the rest
                    resolve_sequence(
                        rest,
                        fixed_vars,
                        custom_functions,
                        next_is_paren && end_byte == seq.len(),
                        output,
                    );
                }
                return;
            }
        }

        // No fixed variable prefix found, treat as single identifier
        // This preserves multi-character Unicode identifiers like "αβ" or "θ₁"
        output.push(Token::Identifier(Cow::Borrowed(seq)));
        return;
    }

    // Priority 6 (FINAL FALLBACK): Split into individual characters (for complex sequences)
    // NOTE: Slicing by char indices ensures we return borrowed slices instead of allocating strings
    let mut char_indices = seq.char_indices().map(|(i, _)| i).peekable();
    while let Some(start) = char_indices.next() {
        let end = char_indices.peek().copied().unwrap_or(seq.len());
        output.push(Token::Identifier(Cow::Borrowed(&seq[start..end])));
    }
}

/// Parse derivative notation like ∂^`1_f(x)`/∂_x^1
///
/// **Safety**: Validates format before recursively lexing arguments to prevent
/// infinite recursion on malformed input.
fn parse_derivative_notation<'src, S: std::hash::BuildHasher>(
    s: &'src str,
    fixed_vars: &HashSet<String, S>,
    custom_functions: &HashSet<String, S>,
) -> Result<Token<'src>, DiffError> {
    // Format: ∂^order_func(args)/∂_var^order
    if !s.starts_with("\u{2202}") || !s.contains("/\u{2202}_") {
        return Err(DiffError::invalid_token(format!(
            "Invalid derivative notation '{s}': must start with '\u{2202}' and contain '/\u{2202}_'"
        )));
    }

    let parts: Vec<&str> = s.split("/\u{2202}_").collect();
    if parts.len() != 2 {
        return Err(DiffError::invalid_token(format!(
            "Invalid derivative notation '{}': expected exactly one '/\u{2202}_' separator, found {}",
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
    let order_str = if left_parts[0].starts_with("\u{2202}^") {
        left_parts[0].get(4..).expect("Checked prefix boundaries")
    } else if left_parts[0] == "\u{2202}" {
        "1"
    } else {
        return Err(DiffError::invalid_token(format!(
            "Invalid derivative notation '{s}': expected '\u{2202}' or '\u{2202}^order'"
        )));
    };

    let order = order_str.parse::<u32>().map_err(|_e| {
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
            let name = func_str
                .get(..open_paren)
                .expect("Checked paren position");
                // .to_owned(); // Changed to borrow
            let args_body = func_str
                .get(open_paren + 1..func_str.len() - 1)
                .expect("Checked paren boundaries");

            // Safety: prevent infinite recursion on malformed nested derivatives
            // Check if we're about to recursively parse another derivative
            if args_body.contains("\u{2202}") && args_body.contains("/\u{2202}_") {
                // This is likely a nested derivative - limit recursion depth
                // by ensuring the args don't contain another full derivative notation
                let nested_depth = args_body.matches("/\u{2202}_").count();
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
        (func_str, Vec::new())
    };

    // Parse right side: var^order or var
    let right_parts: Vec<&str> = right.split('^').collect();
    let var = right_parts[0]; //.to_owned();

    Ok(Token::Derivative {
        order,
        func: Cow::Borrowed(func_name), //Borrow
        args: args_tokens,
        var: Cow::Borrowed(var), // Borrow
    })
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
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
    fn test_scan_derivative_nested_parens() {
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
                    "Failed for {input}: expected {expected}, got {n}"
                );
            } else {
                panic!("Expected Number token for {input}");
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
        let result1 = scan_characters("x + 1").unwrap();
        assert_eq!(result1.len(), 3);

        let result2 = scan_characters("sin(x)").unwrap();
        assert_eq!(result2.len(), 4);
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
        fixed_vars.insert("ax".to_owned());
        let custom_funcs = HashSet::new();

        let tokens = lex("ax", &fixed_vars, &custom_funcs).unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Identifier(Cow::Borrowed("ax")));
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
    fn test_identifier_rules() {
        assert!(is_identifier_continue('x'));
        assert!(is_identifier_continue('1'));
        assert!(is_identifier_continue('_'));
        assert!(is_identifier_continue('\u{3b2}'));
    }

    #[test]
    fn test_derivative_validation() {
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
    fn test_builtins_operator_sync() {
        // Issue #1: Compile-time test to ensure BUILTINS array stays in sync with Operator enum
        // This test verifies that every function name parseable by Operator::parse_str
        // is also present in BUILTINS array (except arithmetic operators)

        use crate::parser::tokens::Operator;

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
            // Verify it parses to an Operator
            let op = Operator::parse_str(func_name);
            assert!(
                op.is_some(),
                "Operator::parse_str should recognize '{func_name}'"
            );

            // Verify it's in BUILTINS (unless it's an alias)
            if func_name != "sen" && func_name != "sign" && func_name != "sgn" {
                assert!(
                    builtin_set.contains(func_name),
                    "BUILTINS array missing function '{func_name}' - add it to the BUILTINS array in lexer.rs"
                );
            }
        }

        // Verify that arithmetic operators are NOT in BUILTINS
        assert!(!builtin_set.contains("+"));
        assert!(!builtin_set.contains("-"));
        assert!(!builtin_set.contains("*"));
        assert!(!builtin_set.contains("/"));
        assert!(!builtin_set.contains("^"));
    }
}
