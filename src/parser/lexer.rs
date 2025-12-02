// Lexer implementation - two-pass context-aware tokenization
use crate::DiffError;
use crate::parser::tokens::{Operator, Token};
use std::collections::HashSet;

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
    "sign",
    "erf",
    "erfc",
    "gamma",
    "digamma",
    "trigamma",
    "tetragamma",
    "polygamma",
    "beta",
    "besselj",
    "bessely",
    "besseli",
    "besselk",
    "LambertW",
    "Ynm",
    "assoc_legendre",
    "hermite",
    "elliptic_e",
    "elliptic_k",
];

/// Balance parentheses in the input string
pub fn balance_parentheses(input: &str) -> String {
    let open_count = input.chars().filter(|&c| c == '(').count();
    let close_count = input.chars().filter(|&c| c == ')').count();

    use std::cmp::Ordering;
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
                        return format!("({})", input);
                    }
                }
            }
            input.to_string()
        }
    }
}

/// Parse a number with locale support
/// Supports: 3.14, 3,14, ,5, 5,, 1e10, 2.5e-3
fn parse_number(s: &str) -> Result<f64, DiffError> {
    // Check for multiple decimal separators
    let dot_count = s.chars().filter(|&c| c == '.').count();

    if dot_count > 1 {
        return Err(DiffError::InvalidNumber(s.to_string()));
    }

    // Parse with f64 (handles scientific notation automatically)
    s.parse::<f64>()
        .map_err(|_| DiffError::InvalidNumber(s.to_string()))
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

/// Pass 1: Scan characters and create raw tokens
fn scan_characters(input: &str) -> Result<Vec<RawToken>, DiffError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            // Skip whitespace
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }

            // Parentheses and Comma
            '(' => {
                tokens.push(RawToken::LeftParen);
                chars.next();
            }
            ')' => {
                tokens.push(RawToken::RightParen);
                chars.next();
            }
            ',' => {
                tokens.push(RawToken::Comma);
                chars.next();
            }

            // Single-char operators
            '+' => {
                tokens.push(RawToken::Operator('+'));
                chars.next();
            }

            '-' => {
                tokens.push(RawToken::Operator('-'));
                chars.next();
            }

            '/' => {
                tokens.push(RawToken::Operator('/'));
                chars.next();
            }

            // Multiplication or power (**)
            '*' => {
                chars.next();
                if chars.peek() == Some(&'*') {
                    chars.next();
                    tokens.push(RawToken::Operator('^')); // Treat ** as ^
                } else {
                    tokens.push(RawToken::Operator('*'));
                }
            }

            // Power
            '^' => {
                tokens.push(RawToken::Operator('^'));
                chars.next();
            }

            // Numbers
            '0'..='9' | '.' => {
                let mut num_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' {
                        num_str.push(c);
                        chars.next();

                        // Handle scientific notation sign
                        if (c == 'e' || c == 'E')
                            && (chars.peek() == Some(&'+') || chars.peek() == Some(&'-'))
                        {
                            num_str.push(chars.next().unwrap());
                        }
                    } else {
                        break;
                    }
                }
                let num = parse_number(&num_str)?;
                tokens.push(RawToken::Number(num));
            }

            // Alphabetic sequences (Unicode-aware) and derivative symbol
            c if c.is_alphabetic() || c == '_' || c == '∂' => {
                let mut seq = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric()
                        || c == '_'
                        || c == '∂'
                        || (seq.starts_with('∂')
                            && (c == '^' || c == '/' || c == '(' || c == ')' || c == ','))
                    {
                        seq.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }

                // Check if this is derivative notation
                if seq.starts_with('∂') {
                    tokens.push(RawToken::Derivative(seq));
                } else {
                    tokens.push(RawToken::Sequence(seq));
                }
            }

            _ => {
                return Err(DiffError::InvalidToken(ch.to_string()));
            }
        }
    }

    Ok(tokens)
}

/// Pass 2: Resolve sequences into tokens using context
pub fn lex(
    input: &str,
    fixed_vars: &HashSet<String>,
    custom_functions: &HashSet<String>,
) -> Result<Vec<Token>, DiffError> {
    let raw_tokens = scan_characters(input)?;
    let mut tokens = Vec::new();

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
                    _ => return Err(DiffError::InvalidToken(c.to_string())),
                };
                tokens.push(Token::Operator(op));
            }

            RawToken::Derivative(deriv_str) => match parse_derivative_notation(deriv_str) {
                Ok(deriv_token) => tokens.push(deriv_token),
                Err(_) => return Err(DiffError::InvalidToken(deriv_str.to_string())),
            },

            RawToken::Sequence(seq) => {
                let next_is_paren =
                    i + 1 < raw_tokens.len() && matches!(raw_tokens[i + 1], RawToken::LeftParen);
                let resolved = resolve_sequence(seq, fixed_vars, custom_functions, next_is_paren);
                tokens.extend(resolved);
            }
        }
    }

    // Special case: empty parens () → Number(1.0)
    let mut i = 0;
    while i < tokens.len() {
        if i + 1 < tokens.len()
            && matches!(tokens[i], Token::LeftParen)
            && matches!(tokens[i + 1], Token::RightParen)
        {
            // Replace () with 1.0
            tokens.splice(i..=i + 1, vec![Token::Number(1.0)]);
        }
        i += 1;
    }

    Ok(tokens)
}

/// Resolve a sequence into tokens based on context
fn resolve_sequence(
    seq: &str,
    fixed_vars: &HashSet<String>,
    custom_functions: &HashSet<String>,
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
    if BUILTINS.contains(&seq)
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
    if next_is_paren {
        for i in 0..seq.len() {
            for builtin in BUILTINS {
                if seq[i..].starts_with(builtin) && i + builtin.len() == seq.len() {
                    // Found builtin at position i, going to end of sequence
                    let before = &seq[0..i];
                    let mut tokens = Vec::new();

                    // Recursively resolve the part before
                    if !before.is_empty() {
                        tokens.extend(resolve_sequence(
                            before,
                            fixed_vars,
                            custom_functions,
                            false,
                        ));
                    }

                    // Add the built-in function
                    if let Some(op) = Operator::parse_str(builtin) {
                        tokens.push(Token::Operator(op));
                    }

                    return tokens;
                }
            }
        }
    }

    // Priority 5 (FALLBACK): Check if it's a pure alphabetic sequence (variable)
    if seq.chars().all(|c| c.is_alphabetic() || c == '_') {
        // Check if any prefix is a fixed variable
        for i in 1..=seq.len() {
            let prefix = &seq[0..i];
            if fixed_vars.contains(prefix) {
                // Found a fixed variable prefix, split here
                let rest = &seq[i..];
                let mut tokens = vec![Token::Identifier(prefix.to_string())];
                if !rest.is_empty() {
                    // Recursively resolve the rest
                    tokens.extend(resolve_sequence(
                        rest,
                        fixed_vars,
                        custom_functions,
                        next_is_paren && i == seq.len(), // Only pass next_is_paren if this is the last part
                    ));
                }
                return tokens;
            }
        }

        // No fixed variable prefix found, treat as single identifier
        return vec![Token::Identifier(seq.to_string())];
    }

    // Priority 6 (FINAL FALLBACK): Split into individual characters (for complex sequences)
    seq.chars()
        .map(|c| Token::Identifier(c.to_string()))
        .collect()
}

/// Parse derivative notation like ∂^1_f(x)/∂_x^1
fn parse_derivative_notation(s: &str) -> Result<Token, DiffError> {
    // Format: ∂^order_func(args)/∂_var^order
    if !s.starts_with("∂^") || !s.contains("/∂_") {
        return Err(DiffError::InvalidToken(s.to_string()));
    }

    let parts: Vec<&str> = s.split("/∂_").collect();
    if parts.len() != 2 {
        return Err(DiffError::InvalidToken(s.to_string()));
    }

    let left = parts[0]; // ∂^order_func(args)
    let right = parts[1]; // var^order

    // Parse order from right side
    let right_parts: Vec<&str> = right.split('^').collect();
    if right_parts.len() != 2 {
        return Err(DiffError::InvalidToken(s.to_string()));
    }

    let order: u32 = right_parts[1]
        .parse()
        .map_err(|_| DiffError::InvalidToken(s.to_string()))?;
    let var = right_parts[0].to_string();

    // Parse function and args from left side
    if !left.starts_with("∂^") {
        return Err(DiffError::InvalidToken(s.to_string()));
    }

    // Convert to chars for safe indexing
    let left_chars: Vec<char> = left.chars().collect();

    // Find the position of '_'
    let underscore_pos = left_chars.iter().position(|&c| c == '_');
    if underscore_pos.is_none() {
        return Err(DiffError::InvalidToken(s.to_string()));
    }

    let underscore_pos = underscore_pos.unwrap();

    // Extract order string: from after "∂^" (position 2) to before '_'
    let order_chars = &left_chars[2..underscore_pos];
    let order_str: String = order_chars.iter().collect();
    let expected_order: u32 = order_str
        .parse()
        .map_err(|_| DiffError::InvalidToken(s.to_string()))?;
    if expected_order != order {
        return Err(DiffError::InvalidToken(s.to_string()));
    }

    // Extract function and args: from after '_' to end
    let func_chars = &left_chars[underscore_pos + 1..];
    let func_and_args: String = func_chars.iter().collect();

    Ok(Token::Derivative {
        order,
        func: func_and_args,
        args: "".to_string(), // For now, we'll store the full func(args) string
        var,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0], Token::Number(1.0)));
    }
}
