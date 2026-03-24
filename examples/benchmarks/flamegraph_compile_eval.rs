//! Focused flamegraph target: compile + evaluate only.
//!
//! Usage: cargo flamegraph --example `flamegraph_compile_eval` -o `flamegraph_compile_eval.svg`

#![allow(clippy::print_stderr, reason = "profile output")]
#![allow(clippy::use_debug, reason = "timing display")]
#![allow(clippy::non_ascii_literal, reason = "profile output")]
#![allow(clippy::cast_precision_loss, reason = "profile output")]
#![allow(clippy::missing_docs_in_private_items, reason = "profile output")]
#![allow(clippy::unwrap_used, reason = "profile output")]

use std::collections::HashSet;
use std::fs;
use std::hint::black_box;
use std::time::Instant;

use symb_anafis::{CompiledEvaluator, Diff, parse, symb};

#[allow(clippy::too_many_lines, reason = "profiling harness kept in one place")]
fn main() {
    let file_path = "examples/symblica_exp/big_expr.txt";
    if !std::path::Path::new(file_path).exists() {
        eprintln!("Error: {file_path} not found. Run from the project root.");
        return;
    }

    let expr_str = fs::read_to_string(file_path).expect("Failed to read expression file");

    let params_str = vec![
        "alpha", "amuq", "ammu", "xcp1", "e1245", "xcp4", "e3e2", "e1234", "e2345", "e1235",
        "e1345", "amel2", "e2e1", "e5e2", "e4e2", "e3e1", "e4e1", "e5e1", "ammu2", "amuq2", "e5e3",
        "e4e3", "x5", "x6", "x1", "x3", "x4", "xcp3", "xcp2",
    ];

    let values: Vec<f64> = vec![
        0.1, 0.2, 0.3, 0.0, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8,
        1.9, 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9,
    ];

    // Parse once (not profiled)
    let empty = HashSet::new();
    let expr = parse(&expr_str, &empty, &empty, None).expect("parse failed");

    // Differentiate once (not profiled)
    let x = symb("x6");
    let diff_expr = Diff::new()
        .skip_simplification(true)
        .differentiate(&expr, &x)
        .expect("diff failed");

    eprintln!("Expr chars: {}", expr_str.len());
    eprintln!("Starting compile+eval profiling loop...\n");

    // ═══ Compile loop ═══
    let compile_iters = 10;
    let t0 = Instant::now();
    let mut evaluator = None;
    for _ in 0..compile_iters {
        let ev = CompiledEvaluator::compile(black_box(&expr), &params_str, None)
            .expect("compile failed");
        evaluator = Some(ev);
    }
    let compile_time = t0.elapsed();
    eprintln!(
        "Compile (parsed) x{compile_iters}: {:?} ({:?}/iter)",
        compile_time,
        compile_time / compile_iters
    );

    // Compile raw derivative
    let t1 = Instant::now();
    let mut eval_raw = None;
    for _ in 0..compile_iters {
        let ev = CompiledEvaluator::compile(black_box(&diff_expr), &params_str, None)
            .expect("compile failed");
        eval_raw = Some(ev);
    }
    let compile_raw_time = t1.elapsed();
    eprintln!(
        "Compile (raw deriv) x{compile_iters}: {:?} ({:?}/iter)",
        compile_raw_time,
        compile_raw_time / compile_iters
    );

    // ═══ Evaluate loop ═══
    let eval_iters = 0;
    #[cfg(feature = "parallel")]
    let batch_iters = 10; // Profile batch eval
    let evaluator = evaluator.unwrap();
    let eval_raw = eval_raw.unwrap();

    // Scalar eval
    if eval_iters > 0 {
        let t2 = Instant::now();
        for _ in 0..eval_iters {
            black_box(evaluator.evaluate(black_box(&values)));
        }
        let eval_time = t2.elapsed();
        eprintln!(
            "\nEval (parsed) x{eval_iters}: {:?} ({:?}/iter)",
            eval_time,
            eval_time / eval_iters
        );

        let t3 = Instant::now();
        for _ in 0..eval_iters {
            black_box(eval_raw.evaluate(black_box(&values)));
        }
        let eval_raw_time = t3.elapsed();
        eprintln!(
            "Eval (raw deriv) x{eval_iters}: {:?} ({:?}/iter)",
            eval_raw_time,
            eval_raw_time / eval_iters
        );
    }

    // Batch eval
    #[cfg(feature = "parallel")]
    if batch_iters > 0 {
        let n = 1000;
        let columns: Vec<Vec<f64>> = (0..params_str.len())
            .map(|i| {
                (0..n)
                    .map(|j| (j as f64).mul_add(0.001, values[i]))
                    .collect()
            })
            .collect();
        let col_refs: Vec<&[f64]> = columns.iter().map(Vec::as_slice).collect();
        let mut output = vec![0.0; n];

        let t4 = Instant::now();
        for _ in 0..batch_iters {
            evaluator
                .eval_batch(black_box(&col_refs), black_box(&mut output), None)
                .unwrap();
        }
        let batch_time = t4.elapsed();
        eprintln!(
            "\nBatch eval (parsed, {n} pts) x{batch_iters}: {:?} ({:?}/iter)",
            batch_time,
            batch_time / batch_iters
        );

        let t5 = Instant::now();
        for _ in 0..batch_iters {
            eval_raw
                .eval_batch(black_box(&col_refs), black_box(&mut output), None)
                .unwrap();
        }
        let batch_raw_time = t5.elapsed();
        eprintln!(
            "Batch eval (raw deriv, {n} pts) x{batch_iters}: {:?} ({:?}/iter)",
            batch_raw_time,
            batch_raw_time / batch_iters
        );
    }

    eprintln!("\nDone.");
}
