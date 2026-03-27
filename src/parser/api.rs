//! User-facing parser API.

use super::logic::{balance_parentheses, insert_implicit_multiplication, lex, parse_expression};
use crate::core::{Context, DiffError, Expr};
use std::collections::HashSet;
use std::hash::BuildHasher;

/// Parse a formula string into an expression AST
///
/// This function converts a mathematical expression string into a structured
/// `Expr` AST that can be differentiated, simplified, or evaluated.
///
/// # Performance
/// - Uses O(N) additive chain optimization for large sums (e.g., `a+b+c+...`)
/// - Pre-allocates token vectors with capacity heuristics
/// - Static `BUILTINS_SET` via `OnceLock` for O(1) function lookup
///
/// # Arguments
/// * `input` - The formula string to parse (e.g., "x^2 + sin(x)")
/// * `known_symbols` - Set of known symbol names (for parsing multi-char variables)
/// * `custom_functions` - Set of custom function names the parser should recognize
/// * `context` - Optional Context for symbol resolution and additional symbols/functions.
///   If provided, symbols are created/looked up in the context, and its symbols and
///   function names are merged with the explicitly provided sets.
///
/// # Returns
/// An `Expr` AST on success, or a `DiffError` on parsing failure.
///
/// # Example
/// ```
/// use symb_anafis::parse;
/// use HashSet;
///
/// let known_symbols = HashSet::new();
/// let custom_fns = HashSet::new();
///
/// let expr = parse("x^2 + sin(x)", &known_symbols, &custom_fns, None).unwrap();
/// println!("Parsed: {}", expr);
/// ```
///
/// # Note
/// For most use cases, prefer the higher-level `diff()` or `simplify()` functions,
/// or the `Diff`/`Simplify` builders which handle parsing automatically.
///
/// # Errors
/// Returns `DiffError` if:
/// - The input is empty
/// - The input contains invalid syntax
/// - Parentheses are unbalanced
pub fn parse<S: BuildHasher + Clone>(
    input: &str,
    known_symbols: &HashSet<String, S>,
    custom_functions: &HashSet<String, S>,
    context: Option<&Context>,
) -> Result<Expr, DiffError> {
    let symbols_buf = context.map_or_else(
        || None,
        |ctx| {
            let mut buf = known_symbols.clone();
            buf.extend(ctx.symbol_names());
            Some(buf)
        },
    );
    let symbols_ref = symbols_buf.as_ref().unwrap_or(known_symbols);

    let functions_buf = context.map_or_else(
        || None,
        |ctx| {
            let mut buf = custom_functions.clone();
            buf.extend(ctx.function_names());
            Some(buf)
        },
    );
    let functions_ref = functions_buf.as_ref().unwrap_or(custom_functions);

    if input.trim().is_empty() {
        return Err(DiffError::EmptyFormula);
    }

    let balanced = balance_parentheses(input);
    let tokens = lex(&balanced, symbols_ref, functions_ref)?;
    let tokens_with_mul = insert_implicit_multiplication(tokens, functions_ref);

    parse_expression(&tokens_with_mul, context)
}
