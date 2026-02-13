//! Head-to-head comparison: **SymbAnaFis** vs **Symbolica** on a large expression.
//!
//! Phases measured:
//! 1. **Parsing** — both libraries
//! 2. **Compilation** (parsed) — compile the original expression
//! 3. **Evaluation** (parsed) — evaluate the original expression
//! 4. **Differentiation** — SymbAnaFis (raw, no simplification) vs Symbolica
//! 5. **Simplification + Compilation** — SymbAnaFis simplified, SymbAnaFis raw, Symbolica
//! 6. **Evaluation** (×100) — all three differentiated forms

#![allow(clippy::print_stderr, reason = "benchmark output")]
#![allow(clippy::use_debug, reason = "timing display")]
#![allow(clippy::uninlined_format_args, reason = "benchmark output")]
#![allow(clippy::non_ascii_literal, reason = "benchmark output")]
#![allow(clippy::cast_precision_loss, reason = "benchmark output")]
#![allow(clippy::too_many_lines, reason = "benchmark output")]
#![allow(clippy::missing_docs_in_private_items, reason = "benchmark output")]
#![allow(clippy::doc_markdown, reason = "benchmark output")]
#![allow(clippy::unwrap_used, reason = "benchmark output")]

use dotenvy::dotenv;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::time::{Duration, Instant};

use symb_anafis::{CompiledEvaluator, Diff, Simplify, parse, symb};
use symbolica::{
    LicenseManager,
    atom::{Atom, AtomCore, Indeterminate},
    evaluate::{FunctionMap, OptimizationSettings},
    parser::ParseSettings,
    wrap_input,
};

const EVAL_ITERATIONS: u32 = 100;
const VAR_NAME: &str = "x6";

fn main() {
    dotenv().ok();

    if let Ok(key) = env::var("SYMBOLICA_LICENSE") {
        match LicenseManager::set_license_key(&key) {
            Ok(()) => eprintln!("Symbolica license key set."),
            Err(e) => eprintln!("Warning: Symbolica license key failed: {e}"),
        }
    }

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

    let initial_values = vec![
        0.1, 0.2, 0.3, 0.0, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8,
        1.9, 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9,
    ];

    // Verify correct mapping
    let _param_map: HashMap<&str, f64> = params_str
        .iter()
        .zip(initial_values.iter())
        .map(|(&k, &v)| (k, v))
        .collect();

    // ═══════════════════════════════════════════════════════════════════
    //  1. PARSING
    // ═══════════════════════════════════════════════════════════════════
    eprintln!("═══ Phase 1: Parsing ═══");

    let (saf_expr, saf_parse) = {
        let empty = HashSet::new();
        let t = Instant::now();
        let e = parse(&expr_str, &empty, &empty, None).expect("SymbAnaFis parse failed");
        (e, t.elapsed())
    };
    eprintln!("  SymbAnaFis:  {saf_parse:?}");

    let (sym_expr, sym_parse) = {
        let t = Instant::now();
        let e = Atom::parse(wrap_input!(&expr_str), ParseSettings::default())
            .expect("Symbolica parse failed");
        (e, t.elapsed())
    };
    eprintln!("  Symbolica:   {sym_parse:?}");

    // ═══════════════════════════════════════════════════════════════════
    //  2. COMPILATION (parsed expression, before differentiation)
    // ═══════════════════════════════════════════════════════════════════
    eprintln!("\n═══ Phase 2: Compilation (parsed expression) ═══");

    // ── SymbAnaFis: compile the parsed expression (raw) ──
    let (saf_parsed_evaluator, saf_parsed_compile, saf_parsed_params) = {
        let t = Instant::now();
        let evaluator = CompiledEvaluator::compile(&saf_expr, &params_str, None)
            .expect("SymbAnaFis compile (parsed) failed");
        (evaluator, t.elapsed(), initial_values.clone())
    };
    eprintln!("  SymbAnaFis (compile):             {saf_parsed_compile:?}");

    // ── SymbAnaFis: simplify + compile the parsed expression ──
    let (
        saf_parsed_simp_evaluator,
        saf_parsed_simp_time,
        saf_parsed_simp_compile_time,
        saf_parsed_simp_params,
    ) = {
        let t_simp = Instant::now();
        let simplified = Simplify::new()
            .simplify(&saf_expr)
            .expect("SymbAnaFis simplify (parsed) failed");
        let simplify_time = t_simp.elapsed();

        let t_comp = Instant::now();
        let evaluator = CompiledEvaluator::compile(&simplified, &params_str, None)
            .expect("SymbAnaFis compile (parsed simplified) failed");
        let compile_time = t_comp.elapsed();

        (
            evaluator,
            simplify_time,
            compile_time,
            initial_values.clone(),
        )
    };
    eprintln!(
        "  SymbAnaFis (simplify + compile):  {:?} + {:?} = {:?}",
        saf_parsed_simp_time,
        saf_parsed_simp_compile_time,
        saf_parsed_simp_time + saf_parsed_simp_compile_time
    );

    // ── Symbolica: compile the parsed expression ──
    let (mut sym_parsed_evaluator, sym_parsed_compile, sym_parsed_params) = {
        let vars: Vec<Atom> = params_str
            .iter()
            .map(|s| Atom::parse(wrap_input!(s), ParseSettings::default()).unwrap())
            .collect();

        let t = Instant::now();
        let func_map = FunctionMap::new();
        let settings = OptimizationSettings::default();
        let evaluator = sym_expr
            .evaluator(&func_map, &vars, settings)
            .expect("Symbolica compile (parsed) failed")
            .map_coeff(&|coeff| coeff.to_real().map_or(1.0, std::convert::Into::into));
        (evaluator, t.elapsed(), initial_values.clone())
    };
    eprintln!("  Symbolica (compile):             {sym_parsed_compile:?}");

    // ═══════════════════════════════════════════════════════════════════
    //  3. EVALUATION (parsed expression, ×{EVAL_ITERATIONS})
    // ═══════════════════════════════════════════════════════════════════
    eprintln!("\n═══ Phase 3: Evaluation of parsed expression (×{EVAL_ITERATIONS}) ═══");

    let (saf_parsed_eval, saf_parsed_result) =
        bench_eval_saf(&saf_parsed_evaluator, &saf_parsed_params);
    eprintln!("  SymbAnaFis (raw):        {saf_parsed_eval:?}  → {saf_parsed_result}");

    let (saf_parsed_simp_eval, saf_parsed_simp_result) =
        bench_eval_saf(&saf_parsed_simp_evaluator, &saf_parsed_simp_params);
    eprintln!("  SymbAnaFis (simplified): {saf_parsed_simp_eval:?}  → {saf_parsed_simp_result}");

    let (sym_parsed_eval, sym_parsed_result) =
        bench_eval_sym(&mut sym_parsed_evaluator, &sym_parsed_params);
    eprintln!("  Symbolica:               {sym_parsed_eval:?}  → {sym_parsed_result}");

    // ═══════════════════════════════════════════════════════════════════
    //  4. DIFFERENTIATION
    // ═══════════════════════════════════════════════════════════════════
    eprintln!("\n═══ Phase 4: Differentiation (d/d{VAR_NAME}) ═══");

    let (saf_diff_raw, saf_diff_time) = {
        let x = symb(VAR_NAME);
        let t = Instant::now();
        let d = Diff::new()
            .skip_simplification(true)
            .differentiate(&saf_expr, &x)
            .expect("SymbAnaFis diff failed");
        (d, t.elapsed())
    };
    eprintln!("  SymbAnaFis (raw):  {saf_diff_time:?}");

    let (sym_diff, sym_diff_time) = {
        let var_atom = Atom::parse(wrap_input!(VAR_NAME), ParseSettings::default())
            .expect("Symbolica parse var failed");
        let var_indet: Indeterminate = var_atom
            .try_into()
            .expect("Symbolica var → Indeterminate failed");
        let t = Instant::now();
        let d = sym_expr.derivative(var_indet);
        (d, t.elapsed())
    };
    eprintln!("  Symbolica:         {sym_diff_time:?}");

    // ═══════════════════════════════════════════════════════════════════
    //  5. SIMPLIFICATION + COMPILATION
    // ═══════════════════════════════════════════════════════════════════
    eprintln!("\n═══ Phase 5: Simplification + Compilation ═══");

    // ── SymbAnaFis: simplify then compile ──
    let (
        saf_evaluator_simplified,
        saf_simplify_time,
        saf_compile_simplified_time,
        saf_params_simplified,
    ) = {
        let t_simp = Instant::now();
        let simplified = Simplify::new()
            .simplify(&saf_diff_raw)
            .expect("SymbAnaFis simplify failed");
        let simplify_time = t_simp.elapsed();

        let t_comp = Instant::now();
        let evaluator = CompiledEvaluator::compile(&simplified, &params_str, None)
            .expect("SymbAnaFis compile (simplified) failed");
        let compile_time = t_comp.elapsed();

        (
            evaluator,
            simplify_time,
            compile_time,
            initial_values.clone(),
        )
    };
    eprintln!(
        "  SymbAnaFis (simplify + compile): {:?} + {:?} = {:?}",
        saf_simplify_time,
        saf_compile_simplified_time,
        saf_simplify_time + saf_compile_simplified_time
    );

    // ── SymbAnaFis: compile raw (no simplification) ──
    let (saf_evaluator_raw, saf_compile_raw_time, saf_params_raw) = {
        let t = Instant::now();
        let evaluator = CompiledEvaluator::compile(&saf_diff_raw, &params_str, None)
            .expect("SymbAnaFis compile (raw) failed");
        (evaluator, t.elapsed(), initial_values.clone())
    };
    eprintln!(
        "  SymbAnaFis (compile raw):        {:?}",
        saf_compile_raw_time
    );

    // ── Symbolica: compile ──
    let (mut sym_evaluator, sym_compile_time, sym_params) = {
        let vars: Vec<Atom> = params_str
            .iter()
            .map(|s| Atom::parse(wrap_input!(s), ParseSettings::default()).unwrap())
            .collect();

        let t = Instant::now();
        let func_map = FunctionMap::new();
        let settings = OptimizationSettings::default();
        let evaluator = sym_diff
            .evaluator(&func_map, &vars, settings)
            .expect("Symbolica compile failed")
            .map_coeff(&|coeff| coeff.to_real().map_or(1.0, std::convert::Into::into));
        (evaluator, t.elapsed(), initial_values.clone())
    };
    eprintln!("  Symbolica (compile):             {sym_compile_time:?}");

    // ═══════════════════════════════════════════════════════════════════
    //  6. EVALUATION of differentiated expressions (×{EVAL_ITERATIONS})
    // ═══════════════════════════════════════════════════════════════════
    eprintln!("\n═══ Phase 6: Evaluation of derivatives (×{EVAL_ITERATIONS}) ═══");

    let (saf_eval_simplified, saf_result_simplified) =
        bench_eval_saf(&saf_evaluator_simplified, &saf_params_simplified);
    eprintln!("  SymbAnaFis (simplified): {saf_eval_simplified:?}  → {saf_result_simplified}");

    let (saf_eval_raw, saf_result_raw) = bench_eval_saf(&saf_evaluator_raw, &saf_params_raw);
    eprintln!("  SymbAnaFis (raw):        {saf_eval_raw:?}  → {saf_result_raw}");

    let (sym_eval, sym_result) = bench_eval_sym(&mut sym_evaluator, &sym_params);
    eprintln!("  Symbolica:               {sym_eval:?}  → {sym_result}");

    // ═══════════════════════════════════════════════════════════════════
    //  SUMMARY
    // ═══════════════════════════════════════════════════════════════════
    eprintln!("\n{}", "═".repeat(60));
    eprintln!("SUMMARY");
    eprintln!("{}", "═".repeat(60));
    eprintln!("{:<35} {:>12} {:>12}", "", "SymbAnaFis", "Symbolica");
    eprintln!("{}", "─".repeat(60));
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Parse",
        fmt_dur(saf_parse),
        fmt_dur(sym_parse)
    );
    eprintln!("{}", "─".repeat(60));
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Compile (parsed)",
        fmt_dur(saf_parsed_compile),
        fmt_dur(sym_parsed_compile)
    );
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Simplify (parsed)",
        fmt_dur(saf_parsed_simp_time),
        "—"
    );
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Compile (parsed simplified)",
        fmt_dur(saf_parsed_simp_compile_time),
        "—"
    );
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Eval (parsed)",
        fmt_dur(saf_parsed_eval),
        fmt_dur(sym_parsed_eval)
    );
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Eval (parsed simplified)",
        fmt_dur(saf_parsed_simp_eval),
        "—"
    );
    eprintln!("{}", "─".repeat(60));
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Diff",
        fmt_dur(saf_diff_time),
        fmt_dur(sym_diff_time)
    );
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Simplify",
        fmt_dur(saf_simplify_time),
        "—"
    );
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Compile (simplified)",
        fmt_dur(saf_compile_simplified_time),
        fmt_dur(sym_compile_time)
    );
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Compile (raw)",
        fmt_dur(saf_compile_raw_time),
        "—"
    );
    eprintln!("{}", "─".repeat(60));
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Eval avg (simplified)",
        fmt_dur(saf_eval_simplified),
        fmt_dur(sym_eval)
    );
    eprintln!(
        "{:<35} {:>12} {:>12}",
        "Eval avg (raw)",
        fmt_dur(saf_eval_raw),
        "—"
    );
    eprintln!("{}", "═".repeat(60));
}

/// Benchmark evaluation for a `SymbAnaFis` compiled evaluator.
fn bench_eval_saf(evaluator: &CompiledEvaluator, params: &[f64]) -> (Duration, f64) {
    let mut last = 0.0;
    let t = Instant::now();
    for _ in 0..EVAL_ITERATIONS {
        last = evaluator.evaluate(params);
    }
    (t.elapsed() / EVAL_ITERATIONS, last)
}

/// Benchmark evaluation for a Symbolica evaluator.
fn bench_eval_sym(
    evaluator: &mut symbolica::evaluate::ExpressionEvaluator<f64>,
    params: &[f64],
) -> (Duration, f64) {
    let mut last = 0.0;
    let t = Instant::now();
    for _ in 0..EVAL_ITERATIONS {
        last = evaluator.evaluate_single(params);
    }
    (t.elapsed() / EVAL_ITERATIONS, last)
}

/// Format a `Duration` as a human-readable string.
fn fmt_dur(d: Duration) -> String {
    let us = d.as_micros();
    if us < 1_000 {
        format!("{us}µs")
    } else if us < 1_000_000 {
        format!("{:.2}ms", us as f64 / 1_000.0)
    } else {
        format!("{:.3}s", d.as_secs_f64())
    }
}
