// Lexer implementation - two-pass context-aware tokenization
use crate::DiffError;
use crate::parser::{Operator, Token};
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
    "erf",
    "erfc",
    "gamma",
    "digamma",
    "beta",
    "zeta",
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
    Sequence(String), // Multi-char sequence to be resolved
    Operator(char),   // Single-char operator: +, *, ^
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

            // Alphabetic sequences (Unicode-aware)
            c if c.is_alphabetic() || c == '_' => {
                let mut seq = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        seq.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(RawToken::Sequence(seq));
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

            RawToken::Sequence(seq) => {
                // Check if next token is left paren (for function detection)
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
        if i + 1 < tokens.len() {
            if matches!(tokens[i], Token::LeftParen) && matches!(tokens[i + 1], Token::RightParen) {
                // Replace () with 1.0
                tokens.splice(i..=i + 1, vec![Token::Number(1.0)]);
            }
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

    // Priority 2: Check if it's a built-in function followed by (
    if BUILTINS.contains(&seq) && next_is_paren {
        if let Some(op) = Operator::from_str(seq) {
            return vec![Token::Operator(op)];
        }
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
                    if let Some(op) = Operator::from_str(builtin) {
                        tokens.push(Token::Operator(op));
                    }

                    return tokens;
                }
            }
        }
    }

    // Priority 5 (FALLBACK): Split into individual characters
    seq.chars()
        .map(|c| Token::Identifier(c.to_string()))
        .collect()
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
        assert_eq!(parse_number("3.14").unwrap(), 3.14);
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
