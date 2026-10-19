//! Compile scientific derivatives and inspect their scalar register programs.
//!
//! Formulas use their regular domains. Probability densities require positive
//! scales; physical models require positive dimensional parameters and their
//! stated kinematic domains. Disassembly does not evaluate numerical inputs.
//! Simplification has a bounded search; exhaustion retains the raw program.
#![expect(
    clippy::print_stdout,
    reason = "the executable prints formulas and borrowed bytecode listings"
)]

use core::error::Error;

use symb_anafis::{Context, Evaluator, Expr, SimplificationSequence};

const CASES: &[(&str, &str, &str)] = &[
    (
        "Normal PDF",
        "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)",
        "x",
    ),
    (
        "Gaussian 2D",
        "exp(-((x - x0)^2 + (y - y0)^2) / (2 * s^2)) / (2 * pi * s^2)",
        "x",
    ),
    (
        "Maxwell-Boltzmann speed density",
        "4 * pi * (m / (2 * pi * k * T))^(3/2) * v^2 * exp(-m * v^2 / (2 * k * T))",
        "v",
    ),
    ("Lorentz factor", "1 / sqrt(1 - v^2 / c^2)", "v"),
    (
        "Lennard-Jones potential",
        "4 * epsilon * ((sigma / r)^12 - (sigma / r)^6)",
        "r",
    ),
    ("Logistic sigmoid", "1 / (1 + exp(-k * (x - x0)))", "x"),
    (
        "Damped oscillator",
        "A * exp(-gamma * t) * cos(omega * t + phi)",
        "t",
    ),
    (
        "Planck spectral radiance",
        "2 * h * nu^3 / c^2 / (exp(h * nu / (k * T)) - 1)",
        "nu",
    ),
    (
        "Attenuated Bessel wave",
        "A * besselj(0, k * r) * exp(-alpha * r)",
        "r",
    ),
];

fn main() -> Result<(), Box<dyn Error>> {
    let sequence = SimplificationSequence::compilation().match_limit(Some(10_000));
    for &(name, source, variable) in CASES {
        let context = Context::new();
        let expression = context.parse(source)?;
        let coordinate = context.symb(variable)?;
        let derivative = Expr::derivative(expression.clone(), coordinate, 1);
        let parameters: Vec<_> = context
            .symbol_names()
            .iter()
            .map(|parameter| context.symb(parameter))
            .collect::<Result<_, _>>()?;
        let raw = Evaluator::<f64>::compile(&derivative, &parameters)?;

        println!("\n{name}");
        println!("f = {expression}");
        println!("d/d{} = {derivative}", coordinate.to_expr());
        println!("Raw derivative:\n{}", raw.disassemble());
        match sequence.simplify(&derivative) {
            Ok(result) => {
                let simplified = result.into_expression();
                let optimized = Evaluator::<f64>::compile(&simplified, &parameters)?;
                println!("Explicit compilation simplification: {simplified}");
                println!("Simplified derivative:\n{}", optimized.disassemble());
            }
            Err(error) => {
                println!("Simplification stopped: {error}. The raw program remains available.");
            }
        }
    }
    Ok(())
}
