// Parser module - converts strings to AST
mod implicit_mul;
mod lexer;
mod pratt;
mod tokens;

use crate::{DiffError, Expr};
use std::collections::HashSet;

/// Parse a formula string into an AST
///
/// # Arguments
/// * `input` - The formula string to parse
/// * `fixed_vars` - Set of variable names that are constants
/// * `custom_functions` - Set of custom function names
pub fn parse(
    input: &str,
    fixed_vars: &HashSet<String>,
    custom_functions: &HashSet<String>,
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
    pratt::parse_expression(&tokens_with_mul)
}
