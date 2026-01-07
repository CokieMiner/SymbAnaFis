//! Parser module - converts strings to AST
mod implicit_mul;
mod lexer;
mod pratt;
mod tokens;

use crate::{DiffError, Expr};
use std::collections::HashSet;

/// Parse a formula string into an expression AST
///
/// This function converts a mathematical expression string into a structured
/// `Expr` AST that can be differentiated, simplified, or evaluated.
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
/// use std::collections::HashSet;
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
pub fn parse<S: std::hash::BuildHasher + Clone>(
    input: &str,
    known_symbols: &HashSet<String, S>,
    custom_functions: &HashSet<String, S>,
    context: Option<&crate::core::unified_context::Context>,
) -> Result<Expr, DiffError> {
    // Merge known_symbols and custom_functions from context if provided
    let mut merged_symbols = known_symbols.clone();
    let mut merged_custom_functions = custom_functions.clone();

    if let Some(ctx) = context {
        merged_symbols.extend(ctx.symbol_names());
        merged_custom_functions.extend(ctx.function_names());
    }

    // Pipeline: validate -> balance -> lex -> implicit_mul -> parse

    // Step 1: Validate input
    if input.trim().is_empty() {
        return Err(DiffError::EmptyFormula);
    }

    // Step 2: Balance parentheses
    let balanced = lexer::balance_parentheses(input);

    // Step 3: Lexing (two-pass)
    let tokens = lexer::lex(&balanced, &merged_symbols, &merged_custom_functions)?;

    // Step 4: Insert implicit multiplication
    let tokens_with_mul =
        implicit_mul::insert_implicit_multiplication(tokens, &merged_custom_functions);

    // Step 5: Build AST
    pratt::parse_expression(&tokens_with_mul, context)
}
