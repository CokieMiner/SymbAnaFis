//! SymbAnaFis-only profiling target for flamegraph.
//!
//! Runs each phase once (no Symbolica) so the flamegraph is clean.

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
use std::time::Instant;

use symb_anafis::{CompiledEvaluator, Diff, Simplify, parse, symb};

const VAR_NAME: &str = "x6";

fn main() {
    let file_path = "examples/symblica_exp/big_expr.txt";
    if !std::path::Path::new(file_path).exists() {
        eprintln!("Error: {file_path} not found. Run from the project root.");
        return;
    }

    let expr_str = fs::read_to_string(file_path).expect("Failed to read expression file");
    eprintln!("Expression: {} characters\n", expr_str.len());

    let params_str = vec![
        "alpha", "amuq", "ammu", "xcp1", "e1245", "xcp4", "e3e2", "e1234", "e2345", "e1235",
        "e1345", "amel2", "e2e1", "e5e2", "e4e2", "e3e1", "e4e1", "e5e1", "ammu2", "amuq2", "e5e3",
        "e4e3", "x5", "x6", "x1", "x3", "x4", "xcp3", "xcp2",
    ];

    let values: Vec<f64> = vec![
        0.1, 0.2, 0.3, 0.0, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8,
        1.9, 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9,
    ];

    // ═══ 1. Parse ═══
    eprintln!("═══ Phase 1: Parsing ═══");
    let t1 = Instant::now();
    let empty = HashSet::new();
    let expr = parse(&expr_str, &empty, &empty, None).expect("parse failed");
    eprintln!("  Parse: {:?}", t1.elapsed());

    // ═══ 2. Compile (parsed) ═══
    eprintln!("\n═══ Phase 2: Compile (parsed) ═══");
    let t2 = Instant::now();
    let evaluator = CompiledEvaluator::compile(&expr, &params_str, None).expect("compile failed");
    eprintln!("  Compile: {:?}", t2.elapsed());

    // ═══ 3. Evaluate (parsed) ═══
    eprintln!("\n═══ Phase 3: Evaluate (parsed) ═══");
    let t3 = Instant::now();
    let result = evaluator.evaluate(&values);
    eprintln!("  Eval: {:?}  → {result}", t3.elapsed());

    // ═══ 4. Differentiate ═══
    eprintln!("\n═══ Phase 4: Differentiation (d/d{VAR_NAME}) ═══");
    let x = symb(VAR_NAME);
    let t4 = Instant::now();
    let diff_expr = Diff::new()
        .skip_simplification(true)
        .differentiate(&expr, &x)
        .expect("diff failed");
    eprintln!("  Diff: {:?}", t4.elapsed());

    // ═══ 5. Simplify ═══
    eprintln!("\n═══ Phase 5: Simplify derivative ═══");
    let t5 = Instant::now();
    let simplified = Simplify::new()
        .simplify(&diff_expr)
        .expect("simplify failed");
    eprintln!("  Simplify: {:?}", t5.elapsed());

    // ═══ 6. Compile (simplified) ═══
    eprintln!("\n═══ Phase 6: Compile (simplified) ═══");
    let t6 = Instant::now();
    let eval_simplified = CompiledEvaluator::compile(&simplified, &params_str, None)
        .expect("compile simplified failed");
    eprintln!("  Compile: {:?}", t6.elapsed());

    // ═══ 7. Compile (raw derivative) ═══
    eprintln!("\n═══ Phase 7: Compile (raw derivative) ═══");
    let t7 = Instant::now();
    let eval_raw =
        CompiledEvaluator::compile(&diff_expr, &params_str, None).expect("compile raw failed");
    eprintln!("  Compile: {:?}", t7.elapsed());

    // ═══ 8. Evaluate derivatives ═══
    eprintln!("\n═══ Phase 8: Evaluate derivatives ═══");
    let t8 = Instant::now();
    let r1 = eval_simplified.evaluate(&values);
    eprintln!("  Eval (simplified): {:?}  → {r1}", t8.elapsed());

    let t9 = Instant::now();
    let r2 = eval_raw.evaluate(&values);
    eprintln!("  Eval (raw):        {:?}  → {r2}", t9.elapsed());

    eprintln!("\nDone.");
}
