#![allow(
    missing_docs,
    clippy::print_stdout,
    clippy::unwrap_used,
    reason = "Example code allows missing docs, print stdout, and unwrap for simplicity"
)]
use std::collections::HashSet;
use std::hint::black_box;
use symb_anafis::{CompiledEvaluator, Diff, parse, symb};

pub const NORMAL_PDF: &str = "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)";
pub const NORMAL_PDF_VAR: &str = "x";
pub const NORMAL_PDF_FIXED: &[&str] = &["mu", "sigma", "pi"];

pub const GAUSSIAN_2D: &str = "exp(-((x - x0)^2 + (y - y0)^2) / (2 * s^2)) / (2 * pi * s^2)";
pub const GAUSSIAN_2D_VAR: &str = "x";
pub const GAUSSIAN_2D_FIXED: &[&str] = &["y", "x0", "y0", "s", "pi"];

pub const MAXWELL_BOLTZMANN: &str =
    "4 * pi * (m / (2 * pi * k * T))^(3/2) * v^2 * exp(-m * v^2 / (2 * k * T))";
pub const MAXWELL_BOLTZMANN_VAR: &str = "v";
pub const MAXWELL_BOLTZMANN_FIXED: &[&str] = &["m", "k", "T", "pi"];

pub const LORENTZ_FACTOR: &str = "1 / sqrt(1 - v^2 / c^2)";
pub const LORENTZ_FACTOR_VAR: &str = "v";
pub const LORENTZ_FACTOR_FIXED: &[&str] = &["c"];

pub const LENNARD_JONES: &str = "4 * epsilon * ((sigma / r)^12 - (sigma / r)^6)";
pub const LENNARD_JONES_VAR: &str = "r";
pub const LENNARD_JONES_FIXED: &[&str] = &["epsilon", "sigma"];

pub const LOGISTIC_SIGMOID: &str = "1 / (1 + exp(-k * (x - x0)))";
pub const LOGISTIC_SIGMOID_VAR: &str = "x";
pub const LOGISTIC_SIGMOID_FIXED: &[&str] = &["k", "x0"];

pub const DAMPED_OSCILLATOR: &str = "A * exp(-gamma * t) * cos(omega * t + phi)";
pub const DAMPED_OSCILLATOR_VAR: &str = "t";
pub const DAMPED_OSCILLATOR_FIXED: &[&str] = &["A", "gamma", "omega", "phi"];

pub const PLANCK_BLACKBODY: &str = "2 * h * nu^3 / c^2 * 1 / (exp(h * nu / (k * T)) - 1)";
pub const PLANCK_BLACKBODY_VAR: &str = "nu";
pub const PLANCK_BLACKBODY_FIXED: &[&str] = &["h", "c", "k", "T"];

pub const BESSEL_WAVE: &str = "A * besselj(0, k * r) * exp(-alpha * r)";
pub const BESSEL_WAVE_VAR: &str = "r";
pub const BESSEL_WAVE_FIXED: &[&str] = &["A", "k", "alpha"];

pub const ALL_EXPRESSIONS: &[(&str, &str, &str, &[&str])] = &[
    ("Normal PDF", NORMAL_PDF, NORMAL_PDF_VAR, NORMAL_PDF_FIXED),
    (
        "Gaussian 2D",
        GAUSSIAN_2D,
        GAUSSIAN_2D_VAR,
        GAUSSIAN_2D_FIXED,
    ),
    (
        "Maxwell-Boltzmann",
        MAXWELL_BOLTZMANN,
        MAXWELL_BOLTZMANN_VAR,
        MAXWELL_BOLTZMANN_FIXED,
    ),
    (
        "Lorentz Factor",
        LORENTZ_FACTOR,
        LORENTZ_FACTOR_VAR,
        LORENTZ_FACTOR_FIXED,
    ),
    (
        "Lennard-Jones",
        LENNARD_JONES,
        LENNARD_JONES_VAR,
        LENNARD_JONES_FIXED,
    ),
    (
        "Logistic Sigmoid",
        LOGISTIC_SIGMOID,
        LOGISTIC_SIGMOID_VAR,
        LOGISTIC_SIGMOID_FIXED,
    ),
    (
        "Damped Oscillator",
        DAMPED_OSCILLATOR,
        DAMPED_OSCILLATOR_VAR,
        DAMPED_OSCILLATOR_FIXED,
    ),
    (
        "Planck Blackbody",
        PLANCK_BLACKBODY,
        PLANCK_BLACKBODY_VAR,
        PLANCK_BLACKBODY_FIXED,
    ),
    (
        "Bessel Wave",
        BESSEL_WAVE,
        BESSEL_WAVE_VAR,
        BESSEL_WAVE_FIXED,
    ),
];

fn main() {
    let empty_set = HashSet::new();
    let iterations = 2000;

    // Test points buffer
    let test_points: Vec<f64> = (0..1000).map(|i| f64::from(i).mul_add(0.01, 0.1)).collect();

    // Pre-allocate values buffer for evaluation (max params is small, 10 is enough)
    let mut values = [1.0_f64; 16];

    for (_, expr_str, var, fixed) in ALL_EXPRESSIONS {
        let fixed_set: HashSet<String> = fixed.iter().copied().map(String::from).collect();
        let var_sym = symb(var);

        let mut params = vec![*var];
        params.extend(fixed.iter().copied());
        params.sort_unstable();

        let var_idx = params.iter().position(|p| p == var).unwrap();

        for _ in 0..iterations {
            let expr = parse(black_box(expr_str), &empty_set, &fixed_set, None).unwrap();
            let diff_expr = Diff::new()
                .skip_simplification(true)
                .differentiate(&expr, &var_sym)
                .unwrap();
            let compiled = CompiledEvaluator::compile(&diff_expr, &params, None).unwrap();

            let param_count = compiled.param_count();
            let mut sum = 0.0;

            // Use the pre-allocated stack buffer
            let current_values = &mut values[..param_count];
            // Reset to 1.0
            for v in current_values.iter_mut() {
                *v = 1.0;
            }

            for &x in &test_points {
                current_values[var_idx] = x;
                sum += compiled.evaluate(current_values);
            }
            black_box(sum);
        }
    }
}
