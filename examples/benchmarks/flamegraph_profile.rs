//! SymbAnaFis-only profiling target for flamegraph.
//!
//! Usage:
//!   `cargo flamegraph --example flamegraph_profile -o out.svg -- <mode>`
//!
//! Modes:
//!   parse     – Profile parsing only (3 iterations)
//!   diff      – Profile differentiation only (3 iterations, `skip_simplification`)
//!   simplify  – Profile simplification only (1 iteration)
//!   compile   – Profile compilation only (3 iterations)
//!   all       – Run every phase once (original behaviour)

#![allow(clippy::print_stderr, reason = "profile output")]
#![allow(clippy::use_debug, reason = "timing display")]
#![allow(clippy::uninlined_format_args, reason = "profile output")]
#![allow(clippy::non_ascii_literal, reason = "profile output")]
#![allow(clippy::cast_precision_loss, reason = "profile output")]
#![allow(clippy::too_many_lines, reason = "profile output")]
#![allow(clippy::missing_docs_in_private_items, reason = "profile output")]
#![allow(clippy::unwrap_used, reason = "profile output")]

use std::collections::HashSet;
use std::fs;
use std::hint::black_box;
use std::time::Instant;

use symb_anafis::{CompiledEvaluator, Diff, Simplify, parse, symb};

const VAR_NAME: &str = "x6";

fn load_expr() -> String {
    let file_path = "examples/symblica_exp/big_expr.txt";
    if !std::path::Path::new(file_path).exists() {
        eprintln!("Error: {file_path} not found. Run from the project root.");
        #[allow(clippy::exit, reason = "Required to exit with error code from example")]
        std::process::exit(1);
    }
    let s = fs::read_to_string(file_path).expect("Failed to read expression file");
    eprintln!("Expression: {} characters", s.len());
    s
}

fn params() -> Vec<&'static str> {
    vec![
        "alpha", "amuq", "ammu", "xcp1", "e1245", "xcp4", "e3e2", "e1234", "e2345", "e1235",
        "e1345", "amel2", "e2e1", "e5e2", "e4e2", "e3e1", "e4e1", "e5e1", "ammu2", "amuq2", "e5e3",
        "e4e3", "x5", "x6", "x1", "x3", "x4", "xcp3", "xcp2",
    ]
}

fn values() -> Vec<f64> {
    vec![
        0.1, 0.2, 0.3, 0.0, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8,
        1.9, 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9,
    ]
}

// ─────────────────────────────────────────────────────────
// Phase runners
// ─────────────────────────────────────────────────────────

fn bench_parse(expr_str: &str, iters: u32) {
    let empty = HashSet::new();
    eprintln!("═══ Profiling: Parse ({iters} iterations) ═══");
    let start = Instant::now();
    for i in 0..iters {
        let t = Instant::now();
        let expr = parse(expr_str, &empty, &empty, None).expect("parse failed");
        eprintln!("  iter {}: {:?}", i + 1, t.elapsed());
        black_box(expr);
    }
    let elapsed = start.elapsed();
    eprintln!("  Total: {elapsed:?}  ({:?}/iter)", elapsed / iters);
}

fn bench_diff(expr_str: &str, iters: u32) {
    let empty = HashSet::new();
    // Parse once outside the profiling loop
    eprintln!("  Pre-parsing expression...");
    let expr = parse(expr_str, &empty, &empty, None).expect("parse failed");
    let x = symb(VAR_NAME);

    eprintln!("═══ Profiling: Diff ({iters} iterations, skip_simplification) ═══");
    let start = Instant::now();
    for i in 0..iters {
        let t = Instant::now();
        let diff_expr = Diff::new()
            .skip_simplification(true)
            .differentiate(&expr, &x)
            .expect("diff failed");
        eprintln!("  iter {}: {:?}", i + 1, t.elapsed());
        black_box(diff_expr);
    }
    let elapsed = start.elapsed();
    eprintln!("  Total: {elapsed:?}  ({:?}/iter)", elapsed / iters);
}

fn bench_simplify(expr_str: &str, iters: u32) {
    let empty = HashSet::new();
    // Parse + diff once outside the profiling loop
    eprintln!("  Pre-parsing + differentiating...");
    let expr = parse(expr_str, &empty, &empty, None).expect("parse failed");
    let x = symb(VAR_NAME);
    let diff_expr = Diff::new()
        .skip_simplification(true)
        .differentiate(&expr, &x)
        .expect("diff failed");

    eprintln!("═══ Profiling: Simplify ({iters} iterations) ═══");
    let start = Instant::now();
    for i in 0..iters {
        let t = Instant::now();
        let simplified = Simplify::new()
            .simplify(&diff_expr)
            .expect("simplify failed");
        eprintln!("  iter {}: {:?}", i + 1, t.elapsed());
        black_box(simplified);
    }
    let elapsed = start.elapsed();
    eprintln!("  Total: {elapsed:?}  ({:?}/iter)", elapsed / iters);
}

fn bench_compile(expr_str: &str, iters: u32) {
    let empty = HashSet::new();
    let params_str = params();
    // Parse once
    eprintln!("  Pre-parsing...");
    let expr = parse(expr_str, &empty, &empty, None).expect("parse failed");

    eprintln!("═══ Profiling: Compile ({iters} iterations) ═══");
    let start = Instant::now();
    for i in 0..iters {
        let t = Instant::now();
        let evaluator =
            CompiledEvaluator::compile(&expr, &params_str, None).expect("compile failed");
        eprintln!("  iter {}: {:?}", i + 1, t.elapsed());
        black_box(evaluator);
    }
    let elapsed = start.elapsed();
    eprintln!("  Total: {elapsed:?}  ({:?}/iter)", elapsed / iters);
}

fn run_all(expr_str: &str) {
    let empty = HashSet::new();
    let params_str = params();
    let vals = values();

    // 1. Parse
    eprintln!("═══ Phase 1: Parsing ═══");
    let t1 = Instant::now();
    let expr = parse(expr_str, &empty, &empty, None).expect("parse failed");
    eprintln!("  Parse: {:?}", t1.elapsed());

    // 2. Compile (parsed)
    eprintln!("\n═══ Phase 2: Compile (parsed) ═══");
    let t2 = Instant::now();
    let evaluator = CompiledEvaluator::compile(&expr, &params_str, None).expect("compile failed");
    eprintln!("  Compile: {:?}", t2.elapsed());

    // 3. Evaluate (parsed)
    eprintln!("\n═══ Phase 3: Evaluate (parsed) ═══");
    let t3 = Instant::now();
    let result = evaluator.evaluate(&vals);
    eprintln!("  Eval: {:?}  → {result}", t3.elapsed());

    // 4. Differentiate
    eprintln!("\n═══ Phase 4: Differentiation (d/d{VAR_NAME}) ═══");
    let x = symb(VAR_NAME);
    let t4 = Instant::now();
    let diff_expr = Diff::new()
        .skip_simplification(true)
        .differentiate(&expr, &x)
        .expect("diff failed");
    eprintln!("  Diff: {:?}", t4.elapsed());

    // 5. Simplify
    eprintln!("\n═══ Phase 5: Simplify derivative ═══");
    let t5 = Instant::now();
    let simplified = Simplify::new()
        .simplify(&diff_expr)
        .expect("simplify failed");
    eprintln!("  Simplify: {:?}", t5.elapsed());

    // 6. Compile (simplified)
    eprintln!("\n═══ Phase 6: Compile (simplified) ═══");
    let t6 = Instant::now();
    let eval_simplified = CompiledEvaluator::compile(&simplified, &params_str, None)
        .expect("compile simplified failed");
    eprintln!("  Compile: {:?}", t6.elapsed());

    // 7. Compile (raw derivative)
    eprintln!("\n═══ Phase 7: Compile (raw derivative) ═══");
    let t7 = Instant::now();
    let eval_raw =
        CompiledEvaluator::compile(&diff_expr, &params_str, None).expect("compile raw failed");
    eprintln!("  Compile: {:?}", t7.elapsed());

    // 8. Evaluate derivatives
    eprintln!("\n═══ Phase 8: Evaluate derivatives ═══");
    let t8 = Instant::now();
    let r1 = eval_simplified.evaluate(&vals);
    eprintln!("  Eval (simplified): {:?}  → {r1}", t8.elapsed());

    let t9 = Instant::now();
    let r2 = eval_raw.evaluate(&vals);
    eprintln!("  Eval (raw):        {:?}  → {r2}", t9.elapsed());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map_or("all", |s| s.as_str());

    let expr_str = load_expr();

    match mode {
        "parse" => bench_parse(&expr_str, 3),
        "diff" => bench_diff(&expr_str, 3),
        "simplify" => bench_simplify(&expr_str, 1),
        "compile" => bench_compile(&expr_str, 3),
        "all" => run_all(&expr_str),
        _ => {
            eprintln!("Unknown mode: {mode}");
            eprintln!("Available: parse, diff, simplify, compile, all");
            #[allow(clippy::exit, reason = "Required to exit with error code from example")]
            std::process::exit(1);
        }
    }

    eprintln!("\nDone.");
}
