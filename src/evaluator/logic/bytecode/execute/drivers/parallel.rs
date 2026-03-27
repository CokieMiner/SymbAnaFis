//! Parallel batch evaluation using Rayon
//!
//! This module provides parallel evaluation of multiple expressions
//! with flexible input types (Expr or string) and type-preserving output.
//!
//! Enable with the `parallel` feature:
//! ```toml
//! symb_anafis = { version = "0.3", features = ["parallel"] }
//! ```
//!
//! # Example
//! ```rust
//! # #[cfg(feature = "parallel")]
//! # {
//! use symb_anafis::{eval_parallel, symb};
//! use symb_anafis::parallel::SKIP;
//!
//! let x = symb("x");
//! let expr = x.pow(2.0);
//!
//! let results = eval_parallel!(
//!     exprs: ["x^2 + y", expr],
//!     vars: [["x", "y"], ["x"]],
//!     values: [
//!         [[1.0, 2.0], [3.0, 4.0]],
//!         [[1.0, 2.0, 3.0]]
//!     ]
//! );
//! # }
//! ```

use super::CompiledEvaluator;
use super::batch::run_chunked_evaluator;
use crate::core::{DiffError, Expr, Symbol, symb};
use crate::parser::parse;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::Arc;

// ============================================================================
// Input Types
// ============================================================================

/// Expression input - can be an Expr or a string to parse
#[derive(Debug, Clone)]
pub enum ExprInput {
    /// Already parsed expression
    Parsed(Expr),
    /// String formula to parse
    String(String),
}

impl From<Expr> for ExprInput {
    fn from(e: Expr) -> Self {
        Self::Parsed(e)
    }
}

impl From<&Expr> for ExprInput {
    fn from(e: &Expr) -> Self {
        Self::Parsed(e.clone())
    }
}

impl From<&str> for ExprInput {
    fn from(s: &str) -> Self {
        Self::String(s.to_owned())
    }
}

impl From<String> for ExprInput {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

/// Variable input - stores symbol ID for O(1) comparison
///
/// Accepts `&str`, `String`, or `&Symbol` via `From` implementations.
/// Internally stores the interned symbol ID for efficient lookup.
#[derive(Debug, Clone)]
pub struct VarInput {
    /// The interned symbol ID.
    id: u64,
    /// The symbol name.
    name: Arc<str>,
}

impl VarInput {
    /// Get the symbol ID for O(1) comparison
    #[inline]
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Get the variable name as &str
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl From<&str> for VarInput {
    fn from(s: &str) -> Self {
        // Intern the symbol to get a stable ID
        let sym = symb(s);
        Self {
            id: sym.id(),
            name: Arc::from(s),
        }
    }
}

impl From<String> for VarInput {
    fn from(s: String) -> Self {
        let sym = symb(&s);
        Self {
            id: sym.id(),
            name: Arc::from(s.as_str()),
        }
    }
}

impl From<&Symbol> for VarInput {
    fn from(s: &Symbol) -> Self {
        Self {
            id: s.id(),
            name: Arc::from(s.name().unwrap_or_default().as_str()),
        }
    }
}

impl From<Symbol> for VarInput {
    fn from(s: Symbol) -> Self {
        Self {
            id: s.id(),
            name: Arc::from(s.name().unwrap_or_default().as_str()),
        }
    }
}

/// Value to substitute - number, expression, or skip
#[derive(Debug, Clone)]
pub enum Value {
    /// Substitute a numeric value
    Num(f64),
    /// Substitute an expression (symbolic substitution)
    Expr(Expr),
    /// Skip - keep the variable symbolic at this point
    Skip,
}

/// Convenience constant for skipping a variable
pub const SKIP: Value = Value::Skip;

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Self::Num(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Self::Num(f64::from(n))
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        // i64->f64: Python integers map naturally to floats
        #[allow(
            clippy::cast_precision_loss,
            reason = "i64→f64: Python integers map naturally to floats"
        )]
        Self::Num(n as f64)
    }
}

impl From<Expr> for Value {
    fn from(e: Expr) -> Self {
        Self::Expr(e)
    }
}

impl From<&Expr> for Value {
    fn from(e: &Expr) -> Self {
        Self::Expr(e.clone())
    }
}

// ============================================================================
// Output Types
// ============================================================================

/// Result of parallel evaluation - preserves input type
#[derive(Debug, Clone)]
pub enum EvalResult {
    /// Result as Expr (when input was Expr)
    Expr(Expr),
    /// Result as String (when input was string)
    String(String),
}

impl EvalResult {
    // Note: Use .to_string() from Display trait (auto-implemented via ToString)

    /// Get result as Expr (parses if needed)
    ///
    /// # Errors
    /// Returns `DiffError` if the string result cannot be parsed.
    pub fn to_expr(&self) -> Result<Expr, DiffError> {
        match self {
            Self::Expr(e) => Ok(e.clone()),
            Self::String(s) => parse(s, &HashSet::new(), &HashSet::new(), None),
        }
    }

    /// Check if this is a string result
    #[must_use]
    pub const fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Check if this is an Expr result
    #[must_use]
    pub const fn is_expr(&self) -> bool {
        matches!(self, Self::Expr(_))
    }

    /// Unwrap as string, panics if Expr
    ///
    /// # Panics
    /// Panics if `self` is `EvalResult::Expr`.
    #[must_use]
    pub fn unwrap_string(self) -> String {
        match self {
            Self::String(s) => s,
            Self::Expr(_) => {
                // Clippy: Panic is justified here as this is an explicit 'unwrap' API that should only be called when sure of the variant
                #[allow(
                    clippy::panic,
                    reason = "Explicit unwrap API, should only be called when sure of variant"
                )]
                // Explicit unwrap API, should only be called when sure of variant
                {
                    panic!("Expected String, got Expr");
                }
            }
        }
    }

    /// Unwrap as Expr, panics if String
    ///
    /// # Panics
    /// Panics if `self` is `EvalResult::String`.
    #[must_use]
    pub fn unwrap_expr(self) -> Expr {
        match self {
            Self::Expr(e) => e,
            Self::String(_) => {
                // Clippy: Panic is justified here as this is an explicit 'unwrap' API that should only be called when sure of the variant
                #[allow(
                    clippy::panic,
                    reason = "Explicit unwrap API, should only be called when sure of variant"
                )]
                // Explicit unwrap API, should only be called when sure of variant
                {
                    panic!("Expected Expr, got String");
                }
            }
        }
    }
}

impl Display for EvalResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::String(s) => write!(f, "{s}"),
            Self::Expr(e) => write!(f, "{e}"),
        }
    }
}

// ============================================================================
// Core Function
// ============================================================================

/// Unified parallel evaluation for multiple expressions with flexible inputs.
///
/// Prefer using the `eval_parallel!` macro for cleaner syntax.
///
/// # Arguments
/// * `exprs` - Vec of expression inputs (Expr or string)
/// * `var_names` - 2D Vec of variable names for each expression
/// * `values` - 3D Vec of substitution values
///
/// # Returns
/// For each expression, a Vec of `EvalResult` at each point.
/// Output type matches input type (string→String, Expr→Expr).
/// Didn't want to have wrapper in my lib but this one as to stay >:|
///
/// # Errors
/// Returns `DiffError` if:
/// - Dimensions of inputs are mismatched
/// - String expressions fail to parse
pub fn evaluate_parallel(
    exprs: Vec<ExprInput>,
    var_names: Vec<Vec<VarInput>>,
    values: Vec<Vec<Vec<Value>>>,
) -> Result<Vec<Vec<EvalResult>>, DiffError> {
    // Delegate to the hint-based version with no hints (will compute internally)
    evaluate_parallel_with_hint(exprs, var_names, values, None)
}

/// Parallel evaluation with optional pre-computed numeric hints.
///
/// When `is_fully_numeric` is provided, it tells the Rust side whether each expression's
/// values are all numeric, allowing us to skip the O(N) check. This is an optimization
/// for Python bindings that already scan all values during conversion.
///
/// # Arguments
/// * `exprs` - Vec of expression inputs (Expr or string)
/// * `var_names` - 2D Vec of variable names for each expression
/// * `values` - 3D Vec of substitution values
/// * `is_fully_numeric` - Optional Vec of bools, one per expression. If Some(true), skips
///   the numeric check and goes directly to the SIMD fast path.
///
/// # Returns
/// For each expression, a Vec of `EvalResult` at each point.
///
/// # Errors
/// Returns `DiffError::UnsupportedOperation` if dimensions of items mismatch,
/// or absolute evaluation triggers invalid data format loops.
///
/// # Panics
/// Panics if numerical chunk broadcasters encounter empty vectors after previous validation buffers.
// Parallel evaluation handles complex dispatch logic, length is justified
#[allow(
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    reason = "Complex dispatch logic, Vec needed for multi-value per-expression"
)]
#[inline]
pub fn evaluate_parallel_with_hint(
    exprs: Vec<ExprInput>,
    var_names: Vec<Vec<VarInput>>,
    values: Vec<Vec<Vec<Value>>>,
    is_fully_numeric: Option<Vec<bool>>,
) -> Result<Vec<Vec<EvalResult>>, DiffError> {
    let n_exprs = exprs.len();
    if var_names.len() != n_exprs || values.len() != n_exprs {
        return Err(DiffError::UnsupportedOperation(
            "Mismatched dimensions in evaluate_parallel".to_owned(),
        ));
    }

    // Parse all expressions and track which were strings
    let parsed: Vec<(Expr, bool)> = exprs
        .into_iter()
        .map(|input| match input {
            ExprInput::Parsed(e) => Ok((e, false)), // false = was Expr
            ExprInput::String(s) => {
                let expr = parse(&s, &HashSet::new(), &HashSet::new(), None)?;
                Ok((expr, true)) // true = was String
            }
        })
        .collect::<Result<Vec<_>, DiffError>>()?;

    // Process each expression in parallel
    let results: Vec<Vec<EvalResult>> = (0..n_exprs)
        .into_par_iter()
        .map(|expr_idx| {
            let (expr, was_string) = &parsed[expr_idx];
            let vars: Vec<&str> = var_names[expr_idx].iter().map(VarInput::as_str).collect();
            let expr_values = &values[expr_idx];

            // Validate dimensions
            if vars.len() != expr_values.len() {
                return Err(DiffError::EvalColumnMismatch {
                    expected: vars.len(),
                    got: expr_values.len(),
                });
            }

            let n_vars = vars.len();
            if n_vars == 0 {
                let empty_vars: HashMap<&str, f64> = HashMap::new();
                let result = expr.evaluate(&empty_vars, &HashMap::new());
                return Ok(vec![if *was_string {
                    EvalResult::String(result.to_string())
                } else {
                    EvalResult::Expr(result)
                }]);
            }
            // Find max points across all variables
            let n_points = expr_values.iter().map(Vec::len).max().unwrap_or(0);
            if n_points == 0 {
                return Ok(vec![]);
            }

            // SAFETY: Validate that no columns are empty when n_points > 0
            // This prevents panics in the fast paths that use .last().expect()
            // Broadcasting from the last value is only valid if the column has at least one value
            if expr_values.iter().any(Vec::is_empty) {
                return Err(DiffError::EvalColumnLengthMismatch);
            }

            // =========================================================
            // OPTIMIZATION: Attempt to compile for fast evaluation
            // =========================================================

            // We ignore all_values_numeric check here - we'll check per-point
            // We also rely on compile() to check that all variables are bound
            let evaluator = CompiledEvaluator::compile(expr, &vars, None).ok();

            if let Some(evaluator) = evaluator {
                // OPTIMIZATION: Use pre-computed hint if available, otherwise compute lazily
                let globally_numeric = is_fully_numeric.as_ref().map_or_else(
                    || {
                        expr_values
                            .iter()
                            .all(|col| col.iter().all(|v| matches!(v, Value::Num(_))))
                    },
                    |hints| hints.get(expr_idx).copied().unwrap_or(false),
                );

                if globally_numeric {
                    // ULTRA-FAST PATH: Unpack to f64 and delegate to high-perf chunked evaluator
                    // 1. Unpack all columns to contiguous f64 vectors upfront.
                    // This avoids repeated allocation and indirection inside the parallel loop.
                    let numeric_cols: Vec<Vec<f64>> = expr_values
                        .iter()
                        .take(n_vars)
                        .map(|var_vals| {
                            let vlen = var_vals.len();
                            if vlen >= n_points {
                                // Common case: column already has enough points, direct unpack
                                var_vals[..n_points]
                                    .iter()
                                    .map(|v| if let Value::Num(n) = v { *n } else { f64::NAN })
                                    .collect()
                            } else {
                                // Broadcasting: fill from column, then repeat last value
                                let mut col = Vec::with_capacity(n_points);
                                for v in var_vals {
                                    col.push(if let Value::Num(n) = v { *n } else { f64::NAN });
                                }
                                // SAFETY: validated that no columns are empty when n_points > 0
                                let last = *col
                                    .last()
                                    .expect("Column cannot be empty (validated earlier)");
                                col.resize(n_points, last);
                                col
                            }
                        })
                        .collect();

                    let col_refs: Vec<&[f64]> = numeric_cols.iter().map(AsRef::as_ref).collect();

                    // 2. Pre-allocate output and run the dedicated f64 evaluator
                    let mut output = vec![0.0; n_points];
                    run_chunked_evaluator(&evaluator, &col_refs, &mut output)
                        .expect("Chunked evaluation failed in parallel numeric path");

                    // 3. Convert results back to the required EvalResult type
                    let results = output
                        .into_iter()
                        .map(|n| {
                            if *was_string {
                                EvalResult::String(format_float(n))
                            } else {
                                EvalResult::Expr(Expr::number(n))
                            }
                        })
                        .collect();

                    return Ok(results);
                }

                // FAST / HYBRID PATH: Use compiled evaluator where possible
                // Use par_chunks to amortize Rayon overhead over many evaluations
                #[allow(clippy::items_after_statements, reason = "Scoped to hybrid path block")]
                const HYBRID_CHUNK: usize = 256;
                let point_indices: Vec<usize> = (0..n_points).collect();
                let chunks: Vec<Vec<EvalResult>> = point_indices
                    .par_chunks(HYBRID_CHUNK)
                    .map_init(
                        || {
                            (
                                Vec::with_capacity(n_vars),
                                vec![0.0; evaluator.workspace_size],
                            )
                        },
                        |buffers, chunk| {
                            let (params, workspace) = buffers;
                            let mut results = Vec::with_capacity(chunk.len());

                            for &point_idx in chunk {
                                // Check if this specific point has all numeric inputs
                                let mut all_numeric = true;
                                for var_vals in expr_values.iter().take(n_vars) {
                                    if point_idx >= var_vals.len() {
                                        if let Some(val) = var_vals.last() {
                                            if !matches!(val, Value::Num(_)) {
                                                all_numeric = false;
                                                break;
                                            }
                                        } else {
                                            all_numeric = false;
                                            break;
                                        }
                                    } else if !matches!(&var_vals[point_idx], Value::Num(_)) {
                                        all_numeric = false;
                                        break;
                                    }
                                }

                                let result = if all_numeric {
                                    // FAST PATH: Run compiled code
                                    params.clear();
                                    for var_vals in expr_values.iter().take(n_vars) {
                                        let val = if point_idx < var_vals.len() {
                                            &var_vals[point_idx]
                                        } else {
                                            var_vals.last().expect(
                                                "Column cannot be empty (validated earlier)",
                                            )
                                        };

                                        if let Value::Num(n) = val {
                                            params.push(*n);
                                        } else {
                                            return Err(DiffError::UnsupportedOperation(
                                                "Value must be Num after all_numeric check"
                                                    .to_owned(),
                                            ));
                                        }
                                    }

                                    let r = evaluator.evaluate_heap(params, workspace);

                                    Ok(if *was_string {
                                        EvalResult::String(format_float(r))
                                    } else {
                                        EvalResult::Expr(Expr::number(r))
                                    })
                                } else {
                                    // SLOW PATH: Fallback for this point
                                    evaluate_slow_point(
                                        expr,
                                        &vars,
                                        expr_values,
                                        point_idx,
                                        *was_string,
                                    )
                                };
                                results.push(result?);
                            }
                            Ok(results)
                        },
                    )
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(chunks.into_iter().flatten().collect())
            } else {
                // SLOW PATH: Compilation failed (unsupported function, etc.)
                // Evaluate entirely using substitution
                let pts: Vec<EvalResult> = (0..n_points)
                    .into_par_iter()
                    .map(|point_idx| {
                        evaluate_slow_point(expr, &vars, expr_values, point_idx, *was_string)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(pts)
            }
        })
        .collect::<Result<Vec<Vec<_>>, _>>()?;

    Ok(results)
}

/// Helper for slow path evaluation of a single point (extracted to avoid code duplication)
fn evaluate_slow_point(
    expr: &Expr,
    vars: &[&str],
    var_values: &[Vec<Value>],
    point_idx: usize,
    was_string: bool,
) -> Result<EvalResult, DiffError> {
    let n_vars = vars.len();
    let mut var_map: HashMap<&str, f64> = HashMap::new();
    let mut expr_subs: Vec<(&str, &Expr)> = Vec::with_capacity(n_vars);

    for var_idx in 0..n_vars {
        let val = if point_idx < var_values[var_idx].len() {
            &var_values[var_idx][point_idx]
        } else {
            // Broadcast from last value
            // SAFETY: Column cannot be empty (validated upfront in evaluate_parallel_with_hint)
            var_values[var_idx]
                .last()
                .ok_or(DiffError::EvalColumnLengthMismatch)?
        };

        match val {
            Value::Num(n) => {
                var_map.insert(vars[var_idx], *n);
            }
            Value::Expr(e) => {
                expr_subs.push((vars[var_idx], e));
            }
            Value::Skip => {
                // Keep symbolic
            }
        }
    }

    // Apply expression substitutions
    let mut result = expr.clone();
    for (var, sub_expr) in expr_subs {
        result = result.substitute(var, sub_expr);
    }

    // Evaluate numerics
    let evaluated = result.evaluate(&var_map, &HashMap::new());

    // Convert to appropriate result type
    Ok(if was_string {
        EvalResult::String(evaluated.to_string())
    } else {
        EvalResult::Expr(evaluated)
    })
}

/// Format a float for string output (helper for compiled evaluator results)
fn format_float(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        // Format as integer if it's a whole number
        #[allow(
            clippy::cast_possible_truncation,
            reason = "Checked fract()==0.0, so cast is safe"
        )]
        let n_int = n as i64;
        format!("{n_int}")
    } else {
        // Use default float formatting
        format!("{n}")
    }
}

// ============================================================================
// Macro for Clean Syntax
// ============================================================================

/// Helper macro to parse nested value arrays
#[macro_export]
#[doc(hidden)]
macro_rules! __parse_values_inner {
    // Single value
    (@val $v:expr) => {
        $crate::Value::from($v)
    };

    // Array of values -> Vec<Value>
    (@arr [$($v:expr),* $(,)?]) => {
        vec![$($crate::__parse_values_inner!(@val $v)),*]
    };

    // Array of arrays -> Vec<Vec<Value>>
    (@arr2 [$([$($v:expr),* $(,)?]),* $(,)?]) => {
        vec![$($crate::__parse_values_inner!(@arr [$($v),*])),*]
    };

    // Array of array of arrays -> Vec<Vec<Vec<Value>>>
    (@arr3 [$([$([$($v:expr),* $(,)?]),* $(,)?]),* $(,)?]) => {
        vec![$($crate::__parse_values_inner!(@arr2 [$([$($v),*]),*])),*]
    };
}

/// Parallel evaluation macro with clean syntax.
///
/// # Example
/// ```rust
/// # #[cfg(feature = "parallel")]
/// # {
/// use symb_anafis::{eval_parallel, symb};
/// use symb_anafis::parallel::SKIP;
///
/// let x = symb("x");
/// let expr = x.pow(2.0);
///
/// let results = eval_parallel!(
///     exprs: ["x + y", expr],
///     vars: [["x", "y"], ["x"]],
///     values: [
///         [[1.0, 2.0], [3.0, 4.0]],
///         [[1.0, 2.0, SKIP]]
///     ]
/// ).expect("Should pass");
///
/// // results[0] is Vec<EvalResult::String>
/// // results[1] is Vec<EvalResult::Expr>
/// # }
/// ```
#[macro_export]
macro_rules! eval_parallel {
    (
        exprs: [$($e:expr),* $(,)?],
        vars: [$([$($v:expr),* $(,)?]),* $(,)?],
        values: [$([$([$($val:expr),* $(,)?]),* $(,)?]),* $(,)?]
    ) => {{
        $crate::evaluate_parallel(
            vec![$($crate::ExprInput::from($e)),*],
            vec![$(vec![$($crate::VarInput::from($v)),*]),*],
            vec![$(vec![$(vec![$($crate::Value::from($val)),*]),*]),*],
        )
    }};
}
