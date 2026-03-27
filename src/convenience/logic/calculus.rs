//! Convenience calculus internals built on top of [`crate::Diff`].

use crate::core::DiffError;
use crate::core::Expr;
use crate::core::Symbol;
use crate::diff::Diff;
use crate::parser::parse;
use std::collections::HashSet;

// ============================================================================
// General Helpers
// ============================================================================

#[inline]
fn empty_context() -> (HashSet<String>, HashSet<String>) {
    (HashSet::new(), HashSet::new())
}

#[inline]
fn extract_var_names(vars: &[&Symbol]) -> Vec<String> {
    vars.iter().filter_map(|s| s.name()).collect()
}

#[inline]
fn var_names_to_str_refs(var_names: &[String]) -> Vec<&str> {
    var_names.iter().map(String::as_str).collect()
}

// ============================================================================
// Expression-based API
// ============================================================================

fn gradient_internal(expr: &Expr, vars: &[&str]) -> Result<Vec<Expr>, DiffError> {
    let diff = Diff::new();
    vars.iter()
        .map(|var| diff.differentiate_by_name(expr, var))
        .collect()
}

fn hessian_internal(expr: &Expr, vars: &[&str]) -> Result<Vec<Vec<Expr>>, DiffError> {
    let diff = Diff::new();
    let grad = gradient_internal(expr, vars)?;

    grad.iter()
        .map(|partial| {
            vars.iter()
                .map(|var| diff.differentiate_by_name(partial, var))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect()
}

fn jacobian_internal(exprs: &[Expr], vars: &[&str]) -> Result<Vec<Vec<Expr>>, DiffError> {
    exprs
        .iter()
        .map(|expr| gradient_internal(expr, vars))
        .collect()
}

pub(in super::super) fn gradient(expr: &Expr, vars: &[&Symbol]) -> Result<Vec<Expr>, DiffError> {
    let var_names = extract_var_names(vars);
    let var_refs = var_names_to_str_refs(&var_names);
    gradient_internal(expr, &var_refs)
}

pub(in super::super) fn hessian(
    expr: &Expr,
    vars: &[&Symbol],
) -> Result<Vec<Vec<Expr>>, DiffError> {
    let var_names = extract_var_names(vars);
    let var_refs = var_names_to_str_refs(&var_names);
    hessian_internal(expr, &var_refs)
}

pub(in super::super) fn jacobian(
    exprs: &[Expr],
    vars: &[&Symbol],
) -> Result<Vec<Vec<Expr>>, DiffError> {
    let var_names = extract_var_names(vars);
    let var_refs = var_names_to_str_refs(&var_names);
    jacobian_internal(exprs, &var_refs)
}

// ============================================================================
// String-based API
// ============================================================================

#[inline]
fn parse_formula(formula: &str) -> Result<Expr, DiffError> {
    let (fixed_vars, custom_fns) = empty_context();
    parse(formula, &fixed_vars, &custom_fns, None)
}

#[inline]
fn parse_formulas(formulas: &[&str]) -> Result<Vec<Expr>, DiffError> {
    let (fixed_vars, custom_fns) = empty_context();
    formulas
        .iter()
        .map(|f| parse(f, &fixed_vars, &custom_fns, None))
        .collect()
}

pub(in super::super) fn gradient_str(
    formula: &str,
    vars: &[&str],
) -> Result<Vec<String>, DiffError> {
    let expr = parse_formula(formula)?;
    let grad = gradient_internal(&expr, vars)?;
    Ok(grad.iter().map(ToString::to_string).collect())
}

pub(in super::super) fn hessian_str(
    formula: &str,
    vars: &[&str],
) -> Result<Vec<Vec<String>>, DiffError> {
    let expr = parse_formula(formula)?;
    let hess = hessian_internal(&expr, vars)?;
    Ok(hess
        .iter()
        .map(|row| row.iter().map(ToString::to_string).collect())
        .collect())
}

pub(in super::super) fn jacobian_str(
    formulas: &[&str],
    vars: &[&str],
) -> Result<Vec<Vec<String>>, DiffError> {
    let exprs = parse_formulas(formulas)?;
    let jac = jacobian_internal(&exprs, vars)?;
    Ok(jac
        .iter()
        .map(|row| row.iter().map(ToString::to_string).collect())
        .collect())
}
