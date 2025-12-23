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
/// * `fixed_vars` - Set of variable names that should be treated as constants
/// * `custom_functions` - Set of custom function names the parser should recognize
/// * `context` - Optional symbol context. If provided, symbols are created/looked up there.
///
/// # Returns
/// An `Expr` AST on success, or a `DiffError` on parsing failure.
///
/// # Example
/// ```
/// use symb_anafis::parse;
/// use std::collections::HashSet;
///
/// let fixed_vars = HashSet::new();
/// let custom_fns = HashSet::new();
///
/// let expr = parse("x^2 + sin(x)", &fixed_vars, &custom_fns, None).unwrap();
/// println!("Parsed: {}", expr);
/// ```
///
/// # Note
/// For most use cases, prefer the higher-level `diff()` or `simplify()` functions,
/// or the `Diff`/`Simplify` builders which handle parsing automatically.
pub fn parse(
    input: &str,
    fixed_vars: &HashSet<String>,
    custom_functions: &HashSet<String>,
    context: Option<&crate::core::symbol::SymbolContext>,
) -> Result<Expr, DiffError> {
    // Pipeline: validate -> balance -> lex -> implicit_mul -> parse

    // Step 1: Validate input
    if input.trim().is_empty() {
        return Err(DiffError::EmptyFormula);
    }

    // Step 2: Balance parentheses
    let balanced = lexer::balance_parentheses(input);

    // Step 3: Lexing (two-pass)
    let tokens = lexer::lex(&balanced, fixed_vars, custom_functions)?;

    // Step 4: Insert implicit multiplication
    let tokens_with_mul = implicit_mul::insert_implicit_multiplication(tokens, custom_functions);

    // Step 5: Build AST
    pratt::parse_expression(&tokens_with_mul, context)
}
