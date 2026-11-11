//! Head-to-head comparison: **`SymbAnaFis`** vs **Symbolica** on a large expression.
//!
//! Phases measured:
//! 1. **Parsing** — both libraries
//! 2. **Compilation** (parsed) — raw & simplified for `SymbAnaFis`, compiled for Symbolica
//! 3. **Evaluation** (parsed) — raw & simplified for `SymbAnaFis`, compiled for Symbolica
//! 4. **Differentiation** — `SymbAnaFis` (raw) vs Symbolica
//! 5. **Compilation** (derivative) — raw & simplified for `SymbAnaFis`, compiled for Symbolica
//! 6. **Evaluation** (derivative) — all three forms

#![expect(clippy::print_stderr, reason = "benchmark output")]
#![expect(clippy::cast_precision_loss, reason = "benchmark output")]
#![expect(clippy::too_many_lines, reason = "benchmark output")]
#![expect(clippy::non_ascii_literal, reason = "benchmark table display")]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "benchmark timing and counting"
)]
#![expect(clippy::as_conversions, reason = "benchmark duration float conversion")]

use core::{hint::black_box, time::Duration};
use dotenvy::dotenv;
use std::{env, fs, time::Instant};

use symb_anafis::{Context, Expr, Program, SimplificationSequence, Symbol, Workspace};
use symbolica::{
    atom::{Atom, AtomCore, Indeterminate},
    parser::ParseSettings,
    prelude::LicenseManager,
};

const EVAL_ITERATIONS: u32 = 100;
const VAR_NAME: &str = "x6";
const FILE_PATH: &str = "examples/benchmarks/data/big_expr.txt";

const PARAM_NAMES: [&str; 29] = [
    "alpha", "amuq", "ammu", "xcp1", "e1245", "xcp4", "e3e2", "e1234", "e2345", "e1235",
    "e1345", "amel2", "e2e1", "e5e2", "e4e2", "e3e1", "e4e1", "e5e1", "ammu2", "amuq2",
    "e5e3", "e4e3", "x5", "x6", "x1", "x3", "x4", "xcp3", "xcp2",
];

const INITIAL_VALUES: [f64; 29] = [
    0.1, 0.2, 0.3, 0.0, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8,
    1.9, 2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9,
];

/// Compiled `SymbAnaFis` bytecode program and reusable execution workspace.
struct SafProgram {
    program: Program<f64>,
    workspace: Workspace<f64>,
    compile_time: Duration,
}

impl SafProgram {
    fn compile(expr: &Expr, params: &[Symbol<'_>]) -> Self {
        let t = Instant::now();
        let program = Program::<f64>::compile(expr, params).expect("SymbAnaFis compile failed");
        let compile_time = t.elapsed();
        let workspace = program.workspace();
        Self {
            program,
            workspace,
            compile_time,
        }
    }

    fn evaluate(&mut self, params: &[f64]) -> (Duration, f64) {
        let mut last = 0.0;
        let t = Instant::now();
        for _ in 0..EVAL_ITERATIONS {
            last = self
                .program
                .evaluate(&mut self.workspace, black_box(params))
                .expect("evaluation failed");
            black_box(last);
        }
        (t.elapsed() / EVAL_ITERATIONS, last)
    }
}

/// Compiled Symbolica evaluator and compilation duration.
struct SymEvaluator {
    evaluator: symbolica::evaluate::ExpressionEvaluator<f64>,
    compile_time: Duration,
}

impl SymEvaluator {
    fn compile(atom: &Atom, vars: &[Atom]) -> Self {
        let t = Instant::now();
        let evaluator = atom
            .evaluator(vars)
            .build()
            .expect("Symbolica compile failed")
            .map_coeff(&|coeff| coeff.re.to_f64());
        let compile_time = t.elapsed();
        Self {
            evaluator,
            compile_time,
        }
    }

    fn evaluate(&mut self, params: &[f64]) -> (Duration, f64) {
        let mut last = 0.0;
        let t = Instant::now();
        for _ in 0..EVAL_ITERATIONS {
            last = self.evaluator.evaluate_single(black_box(params));
            black_box(last);
        }
        (t.elapsed() / EVAL_ITERATIONS, last)
    }

    fn instruction_count(&self) -> usize {
        self.evaluator.export_instructions().instructions.len()
    }
}

fn simplify(expr: &Expr) -> (Expr, Duration) {
    let t = Instant::now();
    let sequence = SimplificationSequence::compilation();
    let simplified = sequence
        .simplify(expr)
        .expect("SymbAnaFis simplify failed")
        .into_expression();
    (simplified, t.elapsed())
}

fn init_symbolica() {
    dotenv().ok();
    if let Ok(key) = env::var("SYMBOLICA_LICENSE") {
        match LicenseManager::set_license_key(&key) {
            Ok(()) => eprintln!("Symbolica license key set."),
            Err(e) => eprintln!("Warning: Symbolica license key failed: {e}"),
        }
    }
}

fn load_expression_file(path: &str) -> Option<String> {
    if !std::path::Path::new(path).exists() {
        eprintln!("Error: {path} not found. Run from the project root.");
        return None;
    }
    let content = fs::read_to_string(path).expect("Failed to read expression file");
    eprintln!("Expression: {} characters\n", content.len());
    Some(content)
}

fn print_phase_header(title: &str) {
    eprintln!("\n═══ {title} ═══");
    eprintln!("{:<36} {:>12} {:>12}", "Metric", "SymbAnaFis", "Symbolica");
    eprintln!("{}", "─".repeat(62));
}

fn print_dur_row(label: &str, saf: Duration, sym: Option<Duration>) {
    let sym_str = sym.map_or_else(|| "—".to_string(), fmt_dur);
    eprintln!("{label:<36} {:>12} {:>12}", fmt_dur(saf), sym_str);
}

fn print_str_row(label: &str, saf: &str, sym: &str) {
    eprintln!("{label:<36} {saf:>12} {sym:>12}");
}

fn print_count_row(label: &str, saf: usize, sym: Option<usize>) {
    let sym_str = sym.map_or_else(|| "—".to_string(), |v| v.to_string());
    eprintln!("{label:<36} {saf:>12} {sym_str:>12}");
}

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

fn dag_size(expr: &Expr) -> usize {
    expr.fold(0, |acc, _| acc + 1)
}

fn main() {
    init_symbolica();

    let Some(expr_str) = load_expression_file(FILE_PATH) else {
        return;
    };

    let context = Context::new().expect("context");
    let saf_params: Vec<Symbol<'_>> = PARAM_NAMES
        .iter()
        .map(|s| context.symb(s).expect("SymbAnaFis symbol binding failed"))
        .collect();

    let sym_vars: Vec<Atom> = PARAM_NAMES
        .iter()
        .map(|s| {
            Atom::parse(*s, "global", ParseSettings::default())
                .expect("Symbolica symbol parse failed")
        })
        .collect();

    let sym_var_indet: Indeterminate = Atom::parse(VAR_NAME, "global", ParseSettings::default())
        .expect("Symbolica parse var failed")
        .try_into()
        .expect("Symbolica var → Indeterminate failed");

    // ═══════════════════════════════════════════════════════════════════
    //  1. PARSING
    // ═══════════════════════════════════════════════════════════════════
    print_phase_header("Phase 1: Parsing");

    let (saf_expr, saf_parse) = {
        let t = Instant::now();
        let e = context.parse(&expr_str).expect("SymbAnaFis parse failed");
        (e, t.elapsed())
    };

    let (sym_expr, sym_parse) = {
        let t = Instant::now();
        let e = Atom::parse(&expr_str, "global", ParseSettings::default())
            .expect("Symbolica parse failed");
        (e, t.elapsed())
    };

    print_dur_row("Parse time", saf_parse, Some(sym_parse));
    print_count_row("DAG / tree size", dag_size(&saf_expr), None);

    // ═══════════════════════════════════════════════════════════════════
    //  2. COMPILATION (parsed expression)
    // ═══════════════════════════════════════════════════════════════════
    print_phase_header("Phase 2: Compilation (parsed expression)");

    let mut saf_parsed_raw = SafProgram::compile(&saf_expr, &saf_params);
    let (saf_parsed_simp_expr, saf_parsed_simp_time) = simplify(&saf_expr);
    let mut saf_parsed_simp = SafProgram::compile(&saf_parsed_simp_expr, &saf_params);
    let mut sym_parsed = SymEvaluator::compile(&sym_expr, &sym_vars);

    print_dur_row(
        "Compile (raw)",
        saf_parsed_raw.compile_time,
        Some(sym_parsed.compile_time),
    );
    print_dur_row("Simplify", saf_parsed_simp_time, None);
    print_dur_row(
        "Compile (simplified)",
        saf_parsed_simp.compile_time,
        None,
    );
    print_count_row(
        "Instructions (raw)",
        saf_parsed_raw.program.instruction_count(),
        Some(sym_parsed.instruction_count()),
    );
    print_count_row(
        "Instructions (simplified)",
        saf_parsed_simp.program.instruction_count(),
        None,
    );
    print_count_row(
        "Registers (raw)",
        saf_parsed_raw.program.workspace_size(),
        None,
    );
    print_count_row(
        "Registers (simplified)",
        saf_parsed_simp.program.workspace_size(),
        None,
    );

    // ═══════════════════════════════════════════════════════════════════
    //  3. EVALUATION (parsed expression, ×{EVAL_ITERATIONS})
    // ═══════════════════════════════════════════════════════════════════
    print_phase_header(&format!(
        "Phase 3: Evaluation of parsed expression (×{EVAL_ITERATIONS})"
    ));

    let saf_parsed_raw_eval = saf_parsed_raw.evaluate(&INITIAL_VALUES);
    let saf_parsed_simp_eval = saf_parsed_simp.evaluate(&INITIAL_VALUES);
    let sym_parsed_eval = sym_parsed.evaluate(&INITIAL_VALUES);

    print_dur_row(
        "Eval avg (raw)",
        saf_parsed_raw_eval.0,
        Some(sym_parsed_eval.0),
    );
    print_dur_row("Eval avg (simplified)", saf_parsed_simp_eval.0, None);
    print_str_row(
        "Result value (raw)",
        &format!("{:.6}", saf_parsed_raw_eval.1),
        &format!("{:.6}", sym_parsed_eval.1),
    );
    print_str_row(
        "Result value (simplified)",
        &format!("{:.6}", saf_parsed_simp_eval.1),
        "—",
    );

    // ═══════════════════════════════════════════════════════════════════
    //  4. DIFFERENTIATION (d/d{VAR_NAME})
    // ═══════════════════════════════════════════════════════════════════
    print_phase_header(&format!("Phase 4: Differentiation (d/d{VAR_NAME})"));

    let (saf_diff_raw, saf_diff_time) = {
        let x = context.symb(VAR_NAME).expect("symbol lookup failed");
        let t = Instant::now();
        let d = Expr::derivative(saf_expr.clone(), x, 1);
        let elapsed = t.elapsed();
        drop(saf_expr);
        (d, elapsed)
    };

    let (sym_diff, sym_diff_time) = {
        let t = Instant::now();
        let d = sym_expr.derivative(sym_var_indet);
        (d, t.elapsed())
    };

    print_dur_row("Diff time", saf_diff_time, Some(sym_diff_time));

    // ═══════════════════════════════════════════════════════════════════
    //  5. COMPILATION (derivative expression)
    // ═══════════════════════════════════════════════════════════════════
    print_phase_header("Phase 5: Compilation (derivative expression)");

    let mut saf_diff_raw_compiled = SafProgram::compile(&saf_diff_raw, &saf_params);
    let (saf_diff_simp_expr, saf_diff_simp_time) = simplify(&saf_diff_raw);
    let mut saf_diff_simp = SafProgram::compile(&saf_diff_simp_expr, &saf_params);
    let mut sym_diff_compiled = SymEvaluator::compile(&sym_diff, &sym_vars);

    print_dur_row(
        "Compile (raw)",
        saf_diff_raw_compiled.compile_time,
        Some(sym_diff_compiled.compile_time),
    );
    print_dur_row("Simplify", saf_diff_simp_time, None);
    print_dur_row(
        "Compile (simplified)",
        saf_diff_simp.compile_time,
        None,
    );
    print_count_row(
        "Instructions (raw)",
        saf_diff_raw_compiled.program.instruction_count(),
        Some(sym_diff_compiled.instruction_count()),
    );
    print_count_row(
        "Instructions (simplified)",
        saf_diff_simp.program.instruction_count(),
        None,
    );
    print_count_row(
        "Registers (raw)",
        saf_diff_raw_compiled.program.workspace_size(),
        None,
    );
    print_count_row(
        "Registers (simplified)",
        saf_diff_simp.program.workspace_size(),
        None,
    );
    print_dur_row(
        "Pipeline (diff + compile)",
        saf_diff_time + saf_diff_raw_compiled.compile_time,
        Some(sym_diff_time + sym_diff_compiled.compile_time),
    );
    print_dur_row(
        "Pipeline (diff + simp + comp)",
        saf_diff_time + saf_diff_simp_time + saf_diff_simp.compile_time,
        Some(sym_diff_time + sym_diff_compiled.compile_time),
    );

    // ═══════════════════════════════════════════════════════════════════
    //  6. EVALUATION of derivatives (×{EVAL_ITERATIONS})
    // ═══════════════════════════════════════════════════════════════════
    print_phase_header(&format!(
        "Phase 6: Evaluation of derivatives (×{EVAL_ITERATIONS})"
    ));

    let saf_diff_raw_eval = saf_diff_raw_compiled.evaluate(&INITIAL_VALUES);
    let saf_diff_simp_eval = saf_diff_simp.evaluate(&INITIAL_VALUES);
    let sym_diff_eval = sym_diff_compiled.evaluate(&INITIAL_VALUES);

    print_dur_row(
        "Eval avg (raw)",
        saf_diff_raw_eval.0,
        Some(sym_diff_eval.0),
    );
    print_dur_row("Eval avg (simplified)", saf_diff_simp_eval.0, None);
    print_str_row(
        "Result value (raw)",
        &format!("{:.6}", saf_diff_raw_eval.1),
        &format!("{:.6}", sym_diff_eval.1),
    );
    print_str_row(
        "Result value (simplified)",
        &format!("{:.6}", saf_diff_simp_eval.1),
        "—",
    );
}
