//! Head-to-head comparison: **`SymbAnaFis`** vs **Symbolica** on a large expression.
//!
//! `SymbAnaFis` applies `SimplificationSequence::compilation()` before compiling;
//! simplification time is included in all total compilation and pipeline timings.
//! Evaluation compares **VM vs VM** and **Native JIT vs Native JIT** head-to-head.
//!
//! Phases measured:
//! 1. **Parsing** — both libraries
//! 2. **Compilation (parsed)** — `SymbAnaFis` (simplify + VM + native JIT) vs Symbolica (VM + SymJIT)
//! 3. **Evaluation (parsed)** — VM vs VM and Native JIT vs Native JIT + numerical verification
//! 4. **Differentiation** — `SymbAnaFis` vs Symbolica (`d/dx6`)
//! 5. **Compilation (derivative)** — VM & Native JIT compile + end-to-end diff pipelines
//! 6. **Evaluation (derivative)** — VM vs VM and Native JIT vs Native JIT + numerical verification
//! 7. **Summary** — consolidated headline timings and speedups

#![expect(clippy::print_stderr, reason = "benchmark table output")]
#![expect(
    clippy::non_ascii_literal,
    reason = "Unicode table formatting and units"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "benchmark timing sums and node counts"
)]

use core::{hint::black_box, time::Duration};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Instant,
};

use dotenvy::dotenv;
use symb_anafis::{Context, Expr, Program, SimplificationSequence, Symbol, Workspace};
use symbolica::{
    atom::{Atom, AtomCore, Indeterminate},
    evaluate::{ExpressionEvaluator, JITCompilationSettings, JITCompiledEvaluator},
    parser::ParseSettings,
    prelude::LicenseManager,
};

// ============================================================================
// Configuration
// ============================================================================

const EVAL_WARMUP: u32 = 10;
const EVAL_SAMPLES: usize = 50;
const EVAL_BATCH: u32 = 20;
const VAR_NAME: &str = "x6";
const FILE_PATH: &str = "data/big_expr.txt";

const PARAM_NAMES: [&str; 29] = [
    "alpha", "amuq", "ammu", "xcp1", "e1245", "xcp4", "e3e2", "e1234", "e2345", "e1235", "e1345",
    "amel2", "e2e1", "e5e2", "e4e2", "e3e1", "e4e1", "e5e1", "ammu2", "amuq2", "e5e3", "e4e3",
    "x5", "x6", "x1", "x3", "x4", "xcp3", "xcp2",
];

const INITIAL_VALUES: [f64; 29] = [
    0.1, 0.2, 0.3, 0.0, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9,
    2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9,
];

// ============================================================================
// Entry Point
// ============================================================================

fn main() {
    init_symbolica();

    let Some(expr_str) = load_expression_file(FILE_PATH) else {
        return;
    };

    let context = Context::new().expect("context");
    let saf_params: Vec<Symbol<'_>> = PARAM_NAMES
        .iter()
        .map(|name| {
            context
                .symb(name)
                .expect("SymbAnaFis symbol binding failed")
        })
        .collect();

    let sym_vars: Vec<Atom> = PARAM_NAMES
        .iter()
        .map(|name| {
            Atom::parse(*name, "global", ParseSettings::default())
                .expect("Symbolica symbol parse failed")
        })
        .collect();

    let sym_var_indet: Indeterminate = Atom::parse(VAR_NAME, "global", ParseSettings::default())
        .expect("Symbolica parse var failed")
        .try_into()
        .expect("Symbolica var -> Indeterminate failed");

    table::banner("Benchmark: SymbAnaFis vs Symbolica");
    eprintln!(
        "Input: {FILE_PATH} ({} chars) | Params: {} | Diff var: {VAR_NAME} | Eval: {EVAL_WARMUP} warmup + {EVAL_SAMPLES}×{EVAL_BATCH}",
        fmt_count(expr_str.len()),
        PARAM_NAMES.len(),
    );

    // Phase 1: Parsing
    let (saf_expr, saf_parse) = {
        let t = Instant::now();
        let expr = context.parse(&expr_str).expect("SymbAnaFis parse failed");
        (expr, t.elapsed())
    };
    let (sym_expr, sym_parse) = {
        let t = Instant::now();
        let expr = Atom::parse(&expr_str, "global", ParseSettings::default())
            .expect("Symbolica parse failed");
        (expr, t.elapsed())
    };
    drop(expr_str);

    table::header("Phase 1: Parsing", "Metric");
    table::dur_row("Parse time", saf_parse, sym_parse);
    table::count_row("DAG nodes", saf_expr.node_count(), None);

    // Phases 2 & 3: Parsed Expression (Simplify + Compile + Eval)
    let parsed_summary = run_stage(
        "Phase 2: Compilation (parsed expression)",
        &format!("Phase 3: Evaluation (parsed, {EVAL_WARMUP} warmup, {EVAL_SAMPLES}×{EVAL_BATCH})"),
        &saf_expr,
        &saf_params,
        &sym_expr,
        &sym_vars,
        None,
    );

    // Phase 4: Differentiation
    let (saf_diff, saf_diff_time) = {
        let var = context.symb(VAR_NAME).expect("symbol lookup failed");
        let t = Instant::now();
        let deriv = Expr::derivative(saf_expr.clone(), var, 1);
        let elapsed = t.elapsed();
        drop(saf_expr);
        (deriv, elapsed)
    };
    let (sym_diff, sym_diff_time) = {
        let t = Instant::now();
        let deriv = sym_expr.derivative(sym_var_indet);
        let elapsed = t.elapsed();
        drop(sym_expr);
        (deriv, elapsed)
    };

    table::header(
        &format!("Phase 4: Differentiation (d/d{VAR_NAME})"),
        "Metric",
    );
    table::dur_row("Diff time", saf_diff_time, sym_diff_time);
    table::count_row("Derivative DAG nodes", saf_diff.node_count(), None);

    // Phases 5 & 6: Derivative Expression (Simplify + Compile + Eval)
    let deriv_summary = run_stage(
        "Phase 5: Compilation (derivative expression)",
        &format!(
            "Phase 6: Evaluation (derivative, {EVAL_WARMUP} warmup, {EVAL_SAMPLES}×{EVAL_BATCH})"
        ),
        &saf_diff,
        &saf_params,
        &sym_diff,
        &sym_vars,
        Some((saf_diff_time, sym_diff_time)),
    );

    // Phase 7: Summary
    print_summary(
        (saf_parse, sym_parse),
        parsed_summary,
        (saf_diff_time, sym_diff_time),
        deriv_summary,
    );
}

// ============================================================================
// Engine Wrappers
// ============================================================================

/// Summary statistics and final scalar value from a warmed, multi-sample evaluation run.
#[derive(Clone, Copy)]
struct EvalStats {
    median: Duration,
    mean: Duration,
    std_dev: Duration,
    min: Duration,
    p95: Duration,
    value: f64,
}

impl EvalStats {
    fn from_samples(samples: &mut [f64], value: f64) -> Self {
        samples.sort_by(f64::total_cmp);
        let n = samples.len();
        let n_f64 = f64::from(u32::try_from(n).expect("sample count fits u32"));

        let min = samples[0];
        let median = percentile_sorted(samples, 0.50);
        let p95 = percentile_sorted(samples, 0.95);
        let mean = samples.iter().sum::<f64>() / n_f64;
        let variance = if n > 1 {
            samples
                .iter()
                .map(|&x| {
                    let diff = x - mean;
                    diff * diff
                })
                .sum::<f64>()
                / (n_f64 - 1.0)
        } else {
            0.0
        };

        Self {
            median: Duration::from_secs_f64(median),
            mean: Duration::from_secs_f64(mean),
            std_dev: Duration::from_secs_f64(variance.sqrt()),
            min: Duration::from_secs_f64(min),
            p95: Duration::from_secs_f64(p95),
            value,
        }
    }
}

/// Simplified and compiled `SymbAnaFis` VM + native JIT programs with a reusable workspace.
struct SafProgram {
    vm: Program<f64>,
    native: Program<f64>,
    workspace: Workspace<f64>,
    simplify_time: Duration,
    vm_compile_time: Duration,
    native_lower_time: Duration,
    raw_dag_nodes: usize,
    simp_dag_nodes: usize,
}

impl SafProgram {
    fn compile(expr: &Expr, params: &[Symbol<'_>]) -> Self {
        let raw_dag_nodes = expr.node_count();

        let t_simp = Instant::now();
        let simplified = SimplificationSequence::compilation()
            .simplify(expr)
            .expect("SymbAnaFis simplify failed")
            .into_expression();
        let simplify_time = t_simp.elapsed();
        let simp_dag_nodes = simplified.node_count();

        let t_vm = Instant::now();
        let vm = Program::<f64>::compile(&simplified, params).expect("SymbAnaFis compile failed");
        let vm_compile_time = t_vm.elapsed();

        let t_native = Instant::now();
        let native = vm
            .with_native()
            .expect("SymbAnaFis native JIT compile failed");
        let native_lower_time = t_native.elapsed();

        let workspace = Workspace::new();
        Self {
            vm,
            native,
            workspace,
            simplify_time,
            vm_compile_time,
            native_lower_time,
            raw_dag_nodes,
            simp_dag_nodes,
        }
    }

    const fn total_vm_compile(&self) -> Duration {
        self.simplify_time.saturating_add(self.vm_compile_time)
    }

    const fn total_native_compile(&self) -> Duration {
        self.total_vm_compile()
            .saturating_add(self.native_lower_time)
    }

    fn eval_vm(&mut self, params: &[f64]) -> EvalStats {
        measure_eval(|| {
            self.vm
                .eval_with(&mut self.workspace, black_box(params))
                .expect("VM evaluation failed")
        })
    }

    fn eval_native(&mut self, params: &[f64]) -> EvalStats {
        measure_eval(|| {
            self.native
                .eval_with(&mut self.workspace, black_box(params))
                .expect("native evaluation failed")
        })
    }
}

/// Compiled Symbolica VM evaluator + SymJIT native evaluator.
struct SymProgram {
    vm: ExpressionEvaluator<f64>,
    native: Option<JITCompiledEvaluator<f64>>,
    vm_compile_time: Duration,
    native_lower_time: Option<Duration>,
}

impl SymProgram {
    fn compile(atom: &Atom, vars: &[Atom]) -> Self {
        let t_vm = Instant::now();
        let vm = atom
            .evaluator(vars)
            .build()
            .expect("Symbolica VM compile failed")
            .map_coeff(&|coeff| coeff.re.to_f64());
        let vm_compile_time = t_vm.elapsed();

        let t_native = Instant::now();
        let (native, native_lower_time) = match vm.jit_compile(JITCompilationSettings::default()) {
            Ok(compiled) => (Some(compiled), Some(t_native.elapsed())),
            Err(err) => {
                eprintln!("Warning: Symbolica native JIT failed: {err}");
                (None, None)
            }
        };

        Self {
            vm,
            native,
            vm_compile_time,
            native_lower_time,
        }
    }

    fn total_native_compile(&self) -> Option<Duration> {
        self.native_lower_time
            .map(|lower| self.vm_compile_time.saturating_add(lower))
    }

    fn instruction_count(&self) -> usize {
        self.vm.export_instructions().instructions.len()
    }

    fn eval_vm(&mut self, params: &[f64]) -> EvalStats {
        measure_eval(|| self.vm.evaluate_single(black_box(params)))
    }

    fn eval_native(&mut self, params: &[f64]) -> Option<EvalStats> {
        let native = self.native.as_mut()?;
        let mut out = [0.0];
        Some(measure_eval(|| {
            native.evaluate(black_box(params), &mut out);
            out[0]
        }))
    }
}

// ============================================================================
// Stage Execution (Compilation + Evaluation)
// ============================================================================

/// Headline timings retained from one compiled + evaluated expression stage.
#[derive(Clone, Copy)]
struct StageSummary {
    saf_vm_compile: Duration,
    sym_vm_compile: Duration,
    saf_native_compile: Duration,
    sym_native_compile: Option<Duration>,
    saf_vm_eval: Duration,
    sym_vm_eval: Duration,
    saf_native_eval: Duration,
    sym_native_eval: Option<Duration>,
}

fn run_stage(
    compile_title: &str,
    eval_title: &str,
    saf_expr: &Expr,
    saf_params: &[Symbol<'_>],
    sym_expr: &Atom,
    sym_vars: &[Atom],
    diff_times: Option<(Duration, Duration)>,
) -> StageSummary {
    let mut saf = SafProgram::compile(saf_expr, saf_params);
    let mut sym = SymProgram::compile(sym_expr, sym_vars);

    // Print compilation table
    table::header(compile_title, "Metric");
    table::section("Timings");
    table::row(
        "  Simplify (compilation)",
        &fmt_dur(saf.simplify_time),
        "—",
        "—",
    );
    table::dur_row(
        "  VM compile / build",
        saf.vm_compile_time,
        sym.vm_compile_time,
    );
    table::opt_dur_row(
        "  Native JIT lower",
        saf.native_lower_time,
        sym.native_lower_time,
    );
    table::dur_row(
        "  Total VM (+ simp)",
        saf.total_vm_compile(),
        sym.vm_compile_time,
    );
    table::opt_dur_row(
        "  Total Native (+ simp)",
        saf.total_native_compile(),
        sym.total_native_compile(),
    );

    table::section("Artifacts");
    table::row(
        "  DAG nodes (raw → simp)",
        &format!(
            "{} → {}",
            fmt_count(saf.raw_dag_nodes),
            fmt_count(saf.simp_dag_nodes)
        ),
        "—",
        "—",
    );
    table::count_row(
        "  VM instructions",
        saf.vm.instruction_count(),
        Some(sym.instruction_count()),
    );
    table::count_row("  Registers", saf.vm.workspace_size(), None);
    table::count_row(
        "  Native code (bytes)",
        saf.native.native_bytes().map_or(0, <[u8]>::len),
        None,
    );

    if let Some((saf_diff, sym_diff)) = diff_times {
        table::section("End-to-End Pipeline (Diff + Compile)");
        table::dur_row(
            "  Diff + VM",
            saf_diff + saf.total_vm_compile(),
            sym_diff + sym.vm_compile_time,
        );
        table::opt_dur_row(
            "  Diff + Native JIT",
            saf_diff + saf.total_native_compile(),
            sym.total_native_compile().map(|t| sym_diff + t),
        );
    }

    // Evaluate both VM and Native JIT head-to-head
    let saf_vm = saf.eval_vm(&INITIAL_VALUES);
    let sym_vm = sym.eval_vm(&INITIAL_VALUES);
    let saf_nat = saf.eval_native(&INITIAL_VALUES);
    let sym_nat = sym.eval_native(&INITIAL_VALUES);

    table::header(eval_title, "Metric");
    table::eval_group("VM vs VM", saf_vm, Some(sym_vm));
    table::eval_group("Native JIT vs Native JIT", saf_nat, sym_nat);

    table::section("Verification");
    table::row(
        "  Value (VM)",
        &format!("{:.6}", saf_vm.value),
        &format!("{:.6}", sym_vm.value),
        &fmt_rel_diff(rel_diff(saf_vm.value, sym_vm.value)),
    );
    let (sym_nat_val, nat_err) = sym_nat.map_or_else(
        || {
            (
                "—".to_owned(),
                fmt_rel_diff(rel_diff(saf_nat.value, sym_vm.value)),
            )
        },
        |sn| {
            (
                format!("{:.6}", sn.value),
                fmt_rel_diff(rel_diff(saf_nat.value, sn.value)),
            )
        },
    );
    table::row(
        "  Value (Native JIT)",
        &format!("{:.6}", saf_nat.value),
        &sym_nat_val,
        &nat_err,
    );

    StageSummary {
        saf_vm_compile: saf.total_vm_compile(),
        sym_vm_compile: sym.vm_compile_time,
        saf_native_compile: saf.total_native_compile(),
        sym_native_compile: sym.total_native_compile(),
        saf_vm_eval: saf_vm.median,
        sym_vm_eval: sym_vm.median,
        saf_native_eval: saf_nat.median,
        sym_native_eval: sym_nat.map(|s| s.median),
    }
}

fn print_summary(
    parse_times: (Duration, Duration),
    parsed: StageSummary,
    diff_times: (Duration, Duration),
    deriv: StageSummary,
) {
    table::header("Summary", "Stage");
    table::dur_row("Parse", parse_times.0, parse_times.1);
    table::dur_row(
        "Parsed compile (VM)",
        parsed.saf_vm_compile,
        parsed.sym_vm_compile,
    );
    table::opt_dur_row(
        "Parsed compile (Native)",
        parsed.saf_native_compile,
        parsed.sym_native_compile,
    );
    table::dur_row(
        "Parsed eval — VM (med)",
        parsed.saf_vm_eval,
        parsed.sym_vm_eval,
    );
    table::opt_dur_row(
        "Parsed eval — Native (med)",
        parsed.saf_native_eval,
        parsed.sym_native_eval,
    );
    table::dur_row(
        &format!("Differentiate (d/d{VAR_NAME})"),
        diff_times.0,
        diff_times.1,
    );
    table::dur_row(
        "Deriv compile (VM)",
        deriv.saf_vm_compile,
        deriv.sym_vm_compile,
    );
    table::opt_dur_row(
        "Deriv compile (Native)",
        deriv.saf_native_compile,
        deriv.sym_native_compile,
    );
    table::dur_row(
        "Deriv pipeline (Diff+VM)",
        diff_times.0 + deriv.saf_vm_compile,
        diff_times.1 + deriv.sym_vm_compile,
    );
    table::opt_dur_row(
        "Deriv pipeline (Diff+Nat)",
        diff_times.0 + deriv.saf_native_compile,
        deriv.sym_native_compile.map(|t| diff_times.1 + t),
    );
    table::dur_row(
        "Deriv eval — VM (med)",
        deriv.saf_vm_eval,
        deriv.sym_vm_eval,
    );
    table::opt_dur_row(
        "Deriv eval — Native (med)",
        deriv.saf_native_eval,
        deriv.sym_native_eval,
    );
}

// ============================================================================
// Setup, Timing & Formatting Helpers
// ============================================================================

fn init_symbolica() {
    dotenv().ok();
    if let Ok(key) = env::var("SYMBOLICA_LICENSE") {
        match LicenseManager::set_license_key(&key) {
            Ok(()) => eprintln!("Symbolica license key set."),
            Err(err) => eprintln!("Warning: Symbolica license key failed: {err}"),
        }
    }
}

fn load_expression_file(path: &str) -> Option<String> {
    let beside_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let resolved = if Path::new(path).exists() {
        PathBuf::from(path)
    } else if beside_manifest.exists() {
        beside_manifest
    } else {
        eprintln!(
            "Error: {path} not found in the working directory or beside the manifest ({}).",
            beside_manifest.display()
        );
        return None;
    };
    Some(fs::read_to_string(&resolved).expect("Failed to read expression file"))
}

fn measure_eval(mut eval_once: impl FnMut() -> f64) -> EvalStats {
    let mut last = 0.0;
    for _ in 0..EVAL_WARMUP {
        last = eval_once();
        black_box(last);
    }

    let mut samples = Vec::with_capacity(EVAL_SAMPLES);
    for _ in 0..EVAL_SAMPLES {
        let t = Instant::now();
        for _ in 0..EVAL_BATCH {
            last = eval_once();
            black_box(last);
        }
        samples.push(t.elapsed().as_secs_f64() / f64::from(EVAL_BATCH));
    }

    EvalStats::from_samples(&mut samples, last)
}

fn percentile_sorted(sorted: &[f64], q: f64) -> f64 {
    let last_idx = sorted.len() - 1;
    let pos = q * f64::from(u32::try_from(last_idx).expect("sample count fits u32"));
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "pos is bounded in [0, sorted.len() - 1]"
    )]
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(last_idx);
    let frac = pos - f64::from(u32::try_from(lo).expect("index fits u32"));
    frac.mul_add(sorted[hi] - sorted[lo], sorted[lo])
}

fn rel_diff(value: f64, reference: f64) -> f64 {
    (value - reference).abs() / reference.abs().max(1.0)
}

const fn dur_scale(reference: Duration) -> (f64, &'static str) {
    let ns = reference.as_nanos();
    if ns < 1_000 {
        (1e9, "ns")
    } else if ns < 999_995 {
        (1e6, "µs")
    } else if ns < 999_995_000 {
        (1e3, "ms")
    } else {
        (1.0, "s")
    }
}

fn fmt_dur(d: Duration) -> String {
    let ns = d.as_nanos();
    if ns < 1_000 {
        format!("{ns}ns")
    } else if ns < 999_995 {
        format!("{:.2}µs", d.as_secs_f64() * 1e6)
    } else if ns < 999_995_000 {
        format!("{:.2}ms", d.as_secs_f64() * 1e3)
    } else {
        format!("{:.3}s", d.as_secs_f64())
    }
}

fn fmt_dur_pm(mean: Duration, std_dev: Duration) -> String {
    let (scale, unit) = dur_scale(mean);
    let m = mean.as_secs_f64() * scale;
    let s = std_dev.as_secs_f64() * scale;
    if unit == "ns" {
        format!("{m:.0} ± {s:.0}ns")
    } else if m >= 100.0 {
        format!("{m:.1} ± {s:.1}{unit}")
    } else {
        format!("{m:.2} ± {s:.2}{unit}")
    }
}

fn fmt_dur_range(low: Duration, high: Duration) -> String {
    let (scale, unit) = dur_scale(high);
    let lo = low.as_secs_f64() * scale;
    let hi = high.as_secs_f64() * scale;
    if unit == "ns" {
        format!("{lo:.0}..{hi:.0}ns")
    } else if hi >= 100.0 {
        format!("{lo:.1}..{hi:.1}{unit}")
    } else {
        format!("{lo:.2}..{hi:.2}{unit}")
    }
}

fn fmt_count(n: usize) -> String {
    let digits = n.to_string();
    let bytes = digits.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + len / 3);
    for (idx, &byte) in bytes.iter().enumerate() {
        if idx > 0 && (len - idx).is_multiple_of(3) {
            out.push(',');
        }
        out.push(char::from(byte));
    }
    out
}

fn fmt_speedup(saf: Duration, sym: Duration) -> String {
    let saf_secs = saf.as_secs_f64();
    if saf_secs == 0.0 {
        return "∞".to_owned();
    }
    let ratio = sym.as_secs_f64() / saf_secs;
    if ratio >= 99.95 {
        format!("{ratio:.0}×")
    } else if ratio >= 9.995 {
        format!("{ratio:.1}×")
    } else {
        format!("{ratio:.2}×")
    }
}

fn fmt_rel_diff(err: f64) -> String {
    if err == 0.0 {
        "0 (exact)".to_owned()
    } else {
        format!("{err:.2e}")
    }
}

// ============================================================================
// Table Rendering
// ============================================================================

mod table {
    use core::time::Duration;

    use super::{EvalStats, fmt_count, fmt_dur, fmt_dur_pm, fmt_dur_range, fmt_speedup};

    const LABEL_W: usize = 28;
    const COL_W: usize = 20;
    const SPEEDUP_W: usize = 12;
    const WIDTH: usize = LABEL_W + 2 + COL_W + 2 + COL_W + 2 + SPEEDUP_W;

    pub fn banner(title: &str) {
        let prefix = format!("═══ {title} ");
        let pad = WIDTH.saturating_sub(prefix.chars().count());
        eprintln!("\n{prefix}{}", "═".repeat(pad));
    }

    pub fn header(title: &str, first_col: &str) {
        banner(title);
        eprintln!(
            "{first_col:<LABEL_W$}  {:>COL_W$}  {:>COL_W$}  {:>SPEEDUP_W$}",
            "SymbAnaFis", "Symbolica", "vs Sym"
        );
        eprintln!("{}", "─".repeat(WIDTH));
    }

    pub fn section(title: &str) {
        eprintln!("  {title}");
    }

    pub fn row(label: &str, saf: &str, sym: &str, speedup: &str) {
        eprintln!("{label:<LABEL_W$}  {saf:>COL_W$}  {sym:>COL_W$}  {speedup:>SPEEDUP_W$}");
    }

    pub fn dur_row(label: &str, saf: Duration, sym: Duration) {
        row(label, &fmt_dur(saf), &fmt_dur(sym), &fmt_speedup(saf, sym));
    }

    pub fn opt_dur_row(label: &str, saf: Duration, sym: Option<Duration>) {
        let (sym_str, speedup_str) = sym.map_or_else(
            || ("—".to_owned(), "—".to_owned()),
            |s| (fmt_dur(s), fmt_speedup(saf, s)),
        );
        row(label, &fmt_dur(saf), &sym_str, &speedup_str);
    }

    pub fn count_row(label: &str, saf: usize, sym: Option<usize>) {
        let sym_str = sym.map_or_else(|| "—".to_owned(), fmt_count);
        row(label, &fmt_count(saf), &sym_str, "—");
    }

    pub fn eval_group(title: &str, saf: EvalStats, sym: Option<EvalStats>) {
        section(title);
        opt_dur_row("  Median (p50)", saf.median, sym.map(|s| s.median));
        let (sym_pm, speedup_pm) = sym.map_or_else(
            || ("—".to_owned(), "—".to_owned()),
            |s| (fmt_dur_pm(s.mean, s.std_dev), fmt_speedup(saf.mean, s.mean)),
        );
        row(
            "  Mean ± σ",
            &fmt_dur_pm(saf.mean, saf.std_dev),
            &sym_pm,
            &speedup_pm,
        );
        let sym_range = sym.map_or_else(|| "—".to_owned(), |s| fmt_dur_range(s.min, s.p95));
        row(
            "  Min..p95",
            &fmt_dur_range(saf.min, saf.p95),
            &sym_range,
            "—",
        );
    }
}
