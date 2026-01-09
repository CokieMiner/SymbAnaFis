//! High-performance parallel numeric batch evaluation
//!
//! This module provides `eval_f64`, using chunked parallel evaluation to
//! avoid allocator contention while maximizing parallelism.
//!
//! # Performance
//! - **Chunked Parallelism**: Splits work into chunks (default 256 points)
//! - **SIMD + Parallel**: Uses `eval_batch` with f64x4 SIMD (processes 4 values at a time)
//! - **Cache Locality**: Processing points in chunks improves L1 cache usage
//!
//! # Feature Gate
//! This module requires the `parallel` feature to be enabled.

use crate::DiffError;
use crate::Expr;
use crate::core::evaluator::{CompiledEvaluator, ToParamName};

use rayon::prelude::*;

/// Chunk size for parallel processing
/// Small enough to provide parallelism for 1000 points (4 chunks)
/// Large enough to amortize allocator/thread overhead
const CHUNK_SIZE: usize = 256;

/// High-performance parallel batch evaluation for pure numeric workloads.
///
/// # Arguments
/// - `exprs`: Slice of expressions to evaluate
/// - `var_names`: For each expression, the variable names in the order they appear in `data`
/// - `data`: Columnar data for each expression. `data[expr_idx][var_idx][point_idx]`
///
/// # Returns
/// A `Vec` of `Vec<f64>` where `result[expr_idx][point_idx]` is the evaluation result.
///
/// # Errors
/// Returns `DiffError` if compilation fails (e.g., unsupported functions).
pub fn eval_f64<V: ToParamName + Sync>(
    exprs: &[&Expr],
    var_names: &[&[V]],
    data: &[&[&[f64]]],
) -> Result<Vec<Vec<f64>>, DiffError> {
    if exprs.len() != var_names.len() || exprs.len() != data.len() {
        return Err(DiffError::invalid_syntax(
            "exprs, var_names, and data must have the same length",
        ));
    }

    // Parallel across expressions
    let results: Result<Vec<Vec<f64>>, DiffError> = (0..exprs.len())
        .into_par_iter()
        .map(|expr_idx| {
            eval_single_expr_chunked(
                exprs[expr_idx],
                var_names[expr_idx],
                data[expr_idx],
                expr_idx,
            )
        })
        .collect();

    results
}

fn eval_single_expr_chunked<V: ToParamName>(
    expr: &Expr,
    vars: &[V],
    columns: &[&[f64]],
    expr_idx: usize,
) -> Result<Vec<f64>, DiffError> {
    let n_points = if columns.is_empty() {
        1
    } else {
        columns[0].len()
    };

    let evaluator = CompiledEvaluator::compile(expr, vars, None).map_err(|e| {
        DiffError::invalid_syntax(format!("Failed to compile expression {expr_idx}: {e}"))
    })?;

    // Pre-allocate output buffer
    let mut output = vec![0.0; n_points];

    // For very small datasets, use sequential eval_batch directly
    if n_points < CHUNK_SIZE {
        evaluator.eval_batch(columns, &mut output, None)?;
    } else {
        // Parallel chunked evaluation with thread-local SIMD buffer reuse
        // Uses map_init to allocate buffer once per thread, not per chunk
        let chunk_indices: Vec<usize> = (0..n_points).step_by(CHUNK_SIZE).collect();

        // Process chunks in parallel, collecting results
        let chunk_results: Result<Vec<(usize, Vec<f64>)>, DiffError> = chunk_indices
            .into_par_iter()
            .map_init(
                || Vec::with_capacity(evaluator.stack_size()),
                |simd_buffer, start| {
                    let end = (start + CHUNK_SIZE).min(n_points);
                    let len = end - start;
                    let col_slices: Vec<&[f64]> =
                        columns.iter().map(|col| &col[start..end]).collect();
                    let mut chunk_out = vec![0.0; len];
                    evaluator.eval_batch(&col_slices, &mut chunk_out, Some(simd_buffer))?;
                    Ok((start, chunk_out))
                },
            )
            .collect();

        // Copy results back to output buffer (maintains order)
        for (start, chunk_out) in chunk_results? {
            output[start..start + chunk_out.len()].copy_from_slice(&chunk_out);
        }
    }

    Ok(output)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::cast_precision_loss)] // Standard test relaxations
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn parse_expr(s: &str) -> Expr {
        crate::parser::parse(s, &HashSet::new(), &HashSet::new(), None).unwrap()
    }

    #[test]
    fn test_eval_f64_simple() {
        let expr = parse_expr("x^2 + y");
        let x_data = [1.0, 2.0, 3.0];
        let y_data = [0.5, 0.5, 0.5];
        let result = eval_f64(&[&expr], &[&["x", "y"]], &[&[&x_data[..], &y_data[..]]]).unwrap();
        assert_eq!(result.len(), 1);
        assert!((result[0][0] - 1.5).abs() < 1e-10);
        assert!((result[0][1] - 4.5).abs() < 1e-10);
        assert!((result[0][2] - 9.5).abs() < 1e-10);
    }

    #[test]
    fn test_eval_f64_trig() {
        let expr = parse_expr("sin(x) * cos(x)");
        let x_data: Vec<f64> = (0..100).map(|i| f64::from(i) * 0.01).collect();
        let result = eval_f64(&[&expr], &[&["x"]], &[&[&x_data[..]]]).unwrap();
        for (i, &res) in result[0].iter().enumerate() {
            let x = i as f64 * 0.01;
            assert!(x.sin().mul_add(-x.cos(), res).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_f64_multiple_exprs() {
        let expr1 = parse_expr("x + 1");
        let expr2 = parse_expr("y * 2");
        let x_data = [1.0, 2.0, 3.0];
        let y_data = [10.0, 20.0, 30.0];
        let result = eval_f64(
            &[&expr1, &expr2],
            &[&["x"], &["y"]],
            &[&[&x_data[..]], &[&y_data[..]]],
        )
        .unwrap();
        assert_eq!(result[0], vec![2.0, 3.0, 4.0]);
        assert_eq!(result[1], vec![20.0, 40.0, 60.0]);
    }

    #[test]
    fn test_eval_f64_large() {
        let expr = parse_expr("sin(x) + cos(x)");
        let x_data: Vec<f64> = (0..100_000).map(|i| f64::from(i) * 0.0001).collect();
        let result = eval_f64(&[&expr], &[&["x"]], &[&[&x_data[..]]]).unwrap();
        assert_eq!(result[0].len(), 100_000);
        // Spot check
        for i in [0, 1000, 50000, 99999] {
            let x = i as f64 * 0.0001;
            assert!((result[0][i] - (x.sin() + x.cos())).abs() < 1e-10);
        }
    }
}
