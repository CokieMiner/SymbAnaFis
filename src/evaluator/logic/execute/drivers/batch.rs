//! Internal batch helpers for compiled evaluator execution.

use crate::DiffError;
use crate::Expr;
use crate::evaluator::{CompiledEvaluator, ToParamName};
use rayon::prelude::*;
use wide::f64x4;

const CHUNK_SIZE: usize = 256;

/// Evaluates a single expression in chunks for parallel processing.
pub fn eval_single_expr_chunked<V: ToParamName>(
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

    if n_points == 0 {
        return Ok(Vec::new());
    }

    let evaluator = CompiledEvaluator::compile(expr, vars, None).map_err(|e| {
        DiffError::invalid_syntax(format!("Failed to compile expression {expr_idx}: {e}"))
    })?;

    let mut output = vec![0.0; n_points];
    run_chunked_evaluator(&evaluator, columns, &mut output)?;
    Ok(output)
}

pub(super) fn run_chunked_evaluator(
    evaluator: &CompiledEvaluator,
    columns: &[&[f64]],
    output: &mut [f64],
) -> Result<(), DiffError> {
    let n_points = output.len();

    if columns.is_empty() {
        if n_points == 0 {
            return Ok(());
        }
    } else if columns.first().is_some_and(|c| c.len() != n_points) {
        return Err(DiffError::invalid_syntax(
            "Output buffer length must match data column length",
        ));
    }

    if n_points < CHUNK_SIZE {
        evaluator.eval_batch(columns, output, None)?;
    } else {
        output
            .par_chunks_mut(CHUNK_SIZE)
            .enumerate()
            .try_for_each_init(
                || {
                    (
                        vec![f64x4::splat(0.0); evaluator.workspace_size],
                        Vec::with_capacity(columns.len()),
                    )
                },
                |(simd_buffer, col_slices), (chunk_idx, chunk_out)| {
                    let start = chunk_idx * CHUNK_SIZE;
                    let end = start + chunk_out.len();
                    col_slices.clear();
                    for col in columns {
                        col_slices.push(&col[start..end]);
                    }
                    evaluator.eval_batch(col_slices, chunk_out, Some(simd_buffer))
                },
            )?;
    }

    Ok(())
}
