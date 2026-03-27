//! Convenience evaluation internals for quick string-based workflows.

use crate::core::DiffError;
use crate::parser::parse;
use std::collections::{HashMap, HashSet};

#[inline]
fn empty_context() -> (HashSet<String>, HashSet<String>) {
    (HashSet::new(), HashSet::new())
}

pub(in super::super) fn evaluate_str(
    formula: &str,
    vars: &[(&str, f64)],
) -> Result<String, DiffError> {
    let (fixed_vars, custom_fns) = empty_context();
    let expr = parse(formula, &fixed_vars, &custom_fns, None)?;

    let var_map: HashMap<&str, f64> = vars.iter().copied().collect();
    let result = expr.evaluate(&var_map, &HashMap::new());
    Ok(result.to_string())
}
