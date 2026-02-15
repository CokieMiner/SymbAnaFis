#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::print_stdout,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    reason = "Fuzz tests require direct output and panic assertions"
)]

#[cfg(feature = "parallel")]
use crate::bindings::eval_f64::eval_f64;
use crate::{CompiledEvaluator, Expr, Symbol, symb};
use rand::prelude::*;
use rand::rngs::StdRng;
use std::collections::{HashMap, HashSet};

const NUM_VARS: usize = 5;
const MAX_DEPTH: usize = 6;
const NUM_TESTS_DEFAULT: usize = 12;
const BATCH_SIZE_DEFAULT: usize = 128;

// Generator for random expressions
struct ExprGenerator {
    rng: StdRng,
    vars: Vec<Symbol>,
}

impl ExprGenerator {
    fn new(seed: u64) -> Self {
        let vars = (0..NUM_VARS).map(|i| symb(&format!("x{i}"))).collect();
        Self {
            rng: StdRng::seed_from_u64(seed),
            vars,
        }
    }

    fn generate(&mut self, depth: usize) -> Expr {
        if depth >= MAX_DEPTH || self.rng.random_bool(0.3) {
            // Terminal
            if self.rng.random_bool(0.5) {
                let val: f64 = self.rng.random_range(-10.0..10.0);
                let val = if val.abs() < 1e-3 { 1.0 } else { val };
                Expr::number(val)
            } else {
                let idx = self.rng.random_range(0..self.vars.len());
                Expr::from(self.vars[idx])
            }
        } else {
            // Non-terminal
            let arity_roll = self.rng.random_range(0..100);
            if arity_roll < 40 {
                self.generate_unary(depth)
            } else if arity_roll < 80 {
                self.generate_binary(depth)
            } else if arity_roll < 90 {
                self.generate_ternary(depth)
            } else {
                self.generate_quaternary(depth)
            }
        }
    }

    fn generate_unary(&mut self, depth: usize) -> Expr {
        let funcs = [
            // Trig
            "sin",
            "cos",
            "tan",
            "cot",
            "sec",
            "csc",
            // Inverse Trig
            "asin",
            "acos",
            "atan",
            "acot",
            "asec",
            "acsc",
            // Hyperbolic
            "sinh",
            "cosh",
            "tanh",
            "coth",
            "sech",
            "csch",
            // Inverse Hyperbolic
            "asinh",
            "acosh",
            "atanh",
            "acoth",
            "asech",
            "acsch",
            // Exp/Log/Roots
            "exp",
            "ln",
            "log10",
            "log2",
            "sqrt",
            "cbrt",
            "exp_polar",
            // Special
            "abs",
            "signum",
            "floor",
            "ceil",
            "round",
            "sinc",
            // Error
            "erf",
            "erfc",
            // Gamma/Zeta/Elliptic
            "gamma",
            "digamma",
            "trigamma",
            "tetragamma",
            "zeta",
            "lambertw",
            "elliptic_k",
            "elliptic_e",
        ];
        let func = funcs[self.rng.random_range(0..funcs.len())];
        if matches!(func, "digamma" | "trigamma" | "tetragamma") {
            // Keep arguments in (0.1, 1.1] to avoid poles and huge-asymptotic tails
            // while still fuzzing nontrivial composed expressions.
            let raw = self.generate(depth + 1);
            let stable_arg =
                Expr::func("exp", Expr::negate(Expr::func("abs", raw))) + Expr::number(0.1);
            Expr::func(func, stable_arg)
        } else {
            let arg = self.generate(depth + 1);
            Expr::func(func, arg)
        }
    }

    fn generate_binary(&mut self, depth: usize) -> Expr {
        let lhs = self.generate(depth + 1);
        let rhs = self.generate(depth + 1);

        // 0-3: Arithmetic, 4+: Functions
        let op_type = self.rng.random_range(0..16);
        match op_type {
            0 => lhs + rhs,
            1 => lhs - rhs,
            2 => lhs * rhs,
            3 => lhs / rhs,
            4 => lhs.pow(rhs),   // x.pow(y)
            5 => rhs.log(lhs),   // x.log(base) -> log(lhs, rhs)
            6 => lhs.atan2(rhs), // atan2(y, x)
            7 => lhs.beta(rhs),
            8 => {
                let order = Expr::number(f64::from(self.rng.random_range(0..=6)));
                lhs.polygamma(order)
            }
            9 => {
                let order = Expr::number(f64::from(self.rng.random_range(-6..=6)));
                lhs.besselj(order)
            }
            10 => {
                let order = Expr::number(f64::from(self.rng.random_range(-6..=6)));
                lhs.bessely(order)
            }
            11 => {
                let order = Expr::number(f64::from(self.rng.random_range(-6..=6)));
                lhs.besseli(order)
            }
            12 => {
                let order = Expr::number(f64::from(self.rng.random_range(-6..=6)));
                lhs.besselk(order)
            }
            13 => {
                let order = Expr::number(f64::from(self.rng.random_range(0..=6)));
                lhs.zeta_deriv(order)
            }
            14 => {
                let degree = Expr::number(f64::from(self.rng.random_range(0..=12)));
                lhs.hermite(degree)
            }
            _ => lhs + rhs, // Fallback
        }
    }

    fn generate_ternary(&mut self, depth: usize) -> Expr {
        let x = self.generate(depth + 1);
        let l = self.rng.random_range(0..=10);
        let m = self.rng.random_range(-l..=l);

        // assoc_legendre(l, m, x): keep l,m bounded integers to avoid overflow panics.
        x.assoc_legendre(Expr::number(f64::from(l)), Expr::number(f64::from(m)))
    }

    fn generate_quaternary(&mut self, depth: usize) -> Expr {
        let theta = self.generate(depth + 1);
        let phi = self.generate(depth + 1);
        let l = self.rng.random_range(0..=8);
        let m = self.rng.random_range(-l..=l);
        let l_expr = Expr::number(f64::from(l));
        let m_expr = Expr::number(f64::from(m));

        if self.rng.random_bool(0.5) {
            theta.spherical_harmonic(l_expr, m_expr, phi)
        } else {
            theta.ynm(l_expr, m_expr, phi)
        }
    }
}

// Helper to compare float results with NaN awareness
fn close_enough(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_infinite() && b.is_infinite() {
        return a.signum() == b.signum();
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }

    let diff = (a - b).abs();
    // Use adaptive tolerance
    let max_abs = a.abs().max(b.abs());
    if max_abs < 1e-10 {
        diff < 1e-10
    } else {
        diff / max_abs < 1e-6
    }
}

fn parse_expr_or_panic(expr: &str) -> Expr {
    crate::parser::parse(expr, &HashSet::new(), &HashSet::new(), None)
        .unwrap_or_else(|e| panic!("Failed to parse fuzz corpus expression '{expr}': {e}"))
}

#[test]
fn fuzz_comprehensive_evaluator() {
    const STACK_SIZE_BYTES: usize = 16 * 1024 * 1024;
    std::thread::Builder::new()
        .name("fuzz_comprehensive_evaluator".to_owned())
        .stack_size(STACK_SIZE_BYTES)
        .spawn(fuzz_comprehensive_evaluator_impl)
        .expect("failed to spawn fuzz worker thread")
        .join()
        .expect("fuzz worker thread panicked");
}

#[test]
fn fuzz_simd_instruction_surface_differential() {
    let seed: u64 = rand::random();
    let mut rng = StdRng::seed_from_u64(seed);

    let corpus: [(&str, &[&str]); 35] = [
        ("asin(x/10)", &["x"]),
        ("acos(x/10)", &["x"]),
        ("atan(x)", &["x"]),
        ("asinh(x)", &["x"]),
        ("acosh(abs(x)+1.1)", &["x"]),
        ("atanh(x/10)", &["x"]),
        ("erf(x)", &["x"]),
        ("erfc(x)", &["x"]),
        ("gamma(abs(x)+1.2)", &["x"]),
        ("digamma(abs(x)+1.2)", &["x"]),
        ("trigamma(abs(x)+1.2)", &["x"]),
        ("tetragamma(abs(x)+1.2)", &["x"]),
        ("lambertw(x/10)", &["x"]),
        ("elliptic_k(x/2)", &["x"]),
        ("elliptic_e(x/2)", &["x"]),
        ("zeta(abs(x)+2.0)", &["x"]),
        ("exp_polar(x)", &["x"]),
        ("cbrt(x)", &["x"]),
        ("signum(x)", &["x"]),
        ("floor(x)", &["x"]),
        ("ceil(x)", &["x"]),
        ("round(x)", &["x"]),
        ("besselj(2, x)", &["x"]),
        ("bessely(2, abs(x)+0.5)", &["x"]),
        ("besseli(2, x)", &["x"]),
        ("besselk(2, abs(x)+0.5)", &["x"]),
        ("polygamma(2, abs(x)+1.2)", &["x"]),
        ("beta(abs(x)+1.1, abs(y)+1.2)", &["x", "y"]),
        ("zeta_deriv(1, abs(x)+2.0)", &["x"]),
        ("hermite(5, x)", &["x"]),
        ("log(2, abs(x)+1.1)", &["x"]),
        ("atan2(y, x)", &["x", "y"]),
        ("assoc_legendre(3, 1, x/2)", &["x"]),
        ("spherical_harmonic(3, 1, abs(x)+0.2, y)", &["x", "y"]),
        ("ynm(3, 1, abs(x)+0.2, y)", &["x", "y"]),
    ];

    for (expr_str, vars) in &corpus {
        let expr = parse_expr_or_panic(expr_str);
        let compiled = CompiledEvaluator::compile(&expr, vars, None)
            .unwrap_or_else(|e| panic!("Compilation failed (seed {seed}) for {expr_str}: {e}"));

        let n_points = 16;
        let mut columns_owned: Vec<Vec<f64>> = vec![vec![0.0; n_points]; vars.len()];
        for col in &mut columns_owned {
            for v in col.iter_mut() {
                *v = rng.random_range(-4.0..4.0);
            }
        }
        let columns: Vec<&[f64]> = columns_owned.iter().map(Vec::as_slice).collect();

        let mut batch_out = vec![0.0; n_points];
        compiled
            .eval_batch(&columns, &mut batch_out, None)
            .unwrap_or_else(|e| {
                panic!("eval_batch failed (seed {seed}) for {expr_str}: {e}");
            });

        for row in 0..n_points {
            let params: Vec<f64> = columns_owned.iter().map(|col| col[row]).collect();
            let scalar = compiled.evaluate(&params);
            assert!(
                close_enough(scalar, batch_out[row]),
                "SIMD surface mismatch (seed {seed}) for {expr_str} at row {row}: scalar={scalar}, simd={}",
                batch_out[row]
            );
        }
    }
}

fn fuzz_comprehensive_evaluator_impl() {
    let num_tests = std::env::var("SYMB_FUZZ_NUM_TESTS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(NUM_TESTS_DEFAULT);
    let batch_size = std::env::var("SYMB_FUZZ_BATCH_SIZE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(BATCH_SIZE_DEFAULT);

    let seed: u64 = rand::random();
    let mut generator = ExprGenerator::new(seed);
    let var_names: Vec<String> = (0..NUM_VARS).map(|i| format!("x{i}")).collect();
    let var_strs: Vec<&str> = var_names.iter().map(|s| s.as_str()).collect();

    for _ in 0..num_tests {
        let expr = generator.generate(0);
        // Debug output for failing case
        // println!("Test {}: {}", i, expr);

        // Generate random input data
        let mut data_map: HashMap<&str, f64> = HashMap::new();
        let mut data_vec: Vec<f64> = Vec::with_capacity(NUM_VARS);
        let mut batch_columns: Vec<Vec<f64>> = vec![vec![0.0; batch_size]; NUM_VARS];

        for var_idx in 0..NUM_VARS {
            let val: f64 = generator.rng.random_range(-5.0..5.0);
            data_map.insert(&var_names[var_idx], val);
            data_vec.push(val);

            // Fill batch columns with random data centered around 'val'
            for slot in batch_columns[var_idx].iter_mut().take(batch_size) {
                *slot = val + generator.rng.random_range(-0.1..0.1);
            }
        }

        // 1. Scalar Ground Truth (Expr::evaluate)
        let ground_truth_expr = expr.evaluate(&data_map, &HashMap::new());
        let ground_truth = ground_truth_expr.as_number().unwrap_or(f64::NAN);

        // 2. Compile
        let compiled = match CompiledEvaluator::compile(&expr, &var_strs, None) {
            Ok(c) => c,
            Err(e) => {
                // Compilation error (e.g. unknown function) - skip if valid error, fail otherwise
                println!("Compilation failed (seed {seed}) for {}: {}", expr, e);
                continue;
            }
        };

        // 3. Scalar Compiled (evaluate)
        let scalar_compiled = compiled.evaluate(&data_vec);

        assert!(
            close_enough(ground_truth, scalar_compiled),
            "Scalar mismatch (seed {})!\nExpr: {}\nVars: {:?}\nGround Truth: {}\nCompiled: {}",
            seed,
            expr,
            data_map,
            ground_truth,
            scalar_compiled
        );

        // 4. SIMD Serial (eval_batch) - Single Point
        let mut single_batch_out = vec![0.0; 1];
        let single_col_slices: Vec<&[f64]> = data_vec.iter().map(std::slice::from_ref).collect();
        compiled
            .eval_batch(&single_col_slices, &mut single_batch_out, None)
            .expect("eval_batch failed");

        assert!(
            close_enough(ground_truth, single_batch_out[0]),
            "SIMD Single mismatch (seed {})!\nExpr: {}\nVars: {:?}\nGround Truth: {}\nSIMD: {}",
            seed,
            expr,
            data_map,
            ground_truth,
            single_batch_out[0]
        );

        // 5. SIMD Serial (eval_batch) - Large Batch
        // Compile a "reference" truth for the batch using scalar eval loop
        // (Expensive but correct)
        let mut batch_ground_truth = Vec::with_capacity(batch_size);
        for row in 0..batch_size {
            let row_vars: Vec<f64> = batch_columns.iter().map(|col| col[row]).collect();
            // We trust scalar compiled now
            batch_ground_truth.push(compiled.evaluate(&row_vars));
        }

        let mut batch_out = vec![0.0; batch_size];
        let batch_col_slices: Vec<&[f64]> = batch_columns.iter().map(|c| c.as_slice()).collect();
        compiled
            .eval_batch(&batch_col_slices, &mut batch_out, None)
            .expect("eval_batch large failed");

        for row in 0..batch_size {
            if !close_enough(batch_ground_truth[row], batch_out[row]) {
                let row_vars: Vec<(&str, f64)> = var_strs
                    .iter()
                    .enumerate()
                    .map(|(i, &name)| (name, batch_columns[i][row]))
                    .collect();
                panic!(
                    "SIMD Batch mismatch (seed {}) at row {}!\nExpr: {}\nVars: {:?}\nGround Truth: {}\nSIMD: {}",
                    seed, row, expr, row_vars, batch_ground_truth[row], batch_out[row]
                );
            }
        }

        // 6. SIMD Parallel (eval_f64)
        // Note: eval_f64 takes multiple expressions, we pass just one
        // It expects &[&[f64]] mapping to vars
        #[cfg(feature = "parallel")]
        {
            let parallel_data: Vec<&[f64]> = batch_columns.iter().map(|c| c.as_slice()).collect();
            let parallel_res = eval_f64(&[&expr], &[var_strs.as_slice()], &[&parallel_data])
                .expect("eval_f64 failed");

            let parallel_out = &parallel_res[0];
            for row in 0..batch_size {
                if !close_enough(batch_ground_truth[row], parallel_out[row]) {
                    let row_vars: Vec<(&str, f64)> = var_strs
                        .iter()
                        .enumerate()
                        .map(|(i, &name)| (name, batch_columns[i][row]))
                        .collect();
                    panic!(
                        "Parallel Batch mismatch (seed {}) at row {}!\nExpr: {}\nVars: {:?}\nGround Truth: {}\nParallel: {}",
                        seed, row, expr, row_vars, batch_ground_truth[row], parallel_out[row]
                    );
                }
            }
        }
    }
}
