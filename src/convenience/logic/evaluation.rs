//! Convenience evaluation internals for quick string-based workflows.

use crate::{DiffError, parser};
use std::collections::HashSet;

#[inline]
fn empty_context() -> (HashSet<String>, HashSet<String>) {
    (HashSet::new(), HashSet::new())
}

pub(in crate::convenience) fn evaluate_str(
    formula: &str,
    vars: &[(&str, f64)],
) -> Result<String, DiffError> {
    let (fixed_vars, custom_fns) = empty_context();
    let expr = parser::parse(formula, &fixed_vars, &custom_fns, None)?;

    let var_map: std::collections::HashMap<&str, f64> = vars.iter().copied().collect();
    let result = expr.evaluate(&var_map, &std::collections::HashMap::new());
    Ok(result.to_string())
}
