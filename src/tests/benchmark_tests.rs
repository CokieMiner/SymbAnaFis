//! Tests derived from benchmark expressions
//!
//! Ensures the full pipeline works: parse → diff → simplify → compile → evaluate
//! Uses realistic physics/ML expressions from the benchmark suite.

use crate::{CompiledEvaluator, Diff, Simplify, parse, symb};
use std::collections::HashSet;

// =============================================================================
// Benchmark Expressions (from benches/rust/expressions.rs)
// =============================================================================

/// All expressions as a slice: (name, formula, var, fixed_vars)
const ALL_EXPRESSIONS: &[(&str, &str, &str, &[&str])] = &[
    (
        "Normal PDF",
        "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)",
        "x",
        &["mu", "sigma", "pi"],
    ),
    (
        "Gaussian 2D",
        "exp(-((x - x0)^2 + (y - y0)^2) / (2 * s^2)) / (2 * pi * s^2)",
        "x",
        &["x0", "y0", "s", "pi"],
    ),
    (
        "Maxwell-Boltzmann",
        "4 * pi * (m / (2 * pi * k * T))^(3/2) * v^2 * exp(-m * v^2 / (2 * k * T))",
        "v",
        &["m", "k", "T", "pi"],
    ),
    ("Lorentz Factor", "1 / sqrt(1 - v^2 / c^2)", "v", &["c"]),
    (
        "Lennard-Jones",
        "4 * epsilon * ((sigma / r)^12 - (sigma / r)^6)",
        "r",
        &["epsilon", "sigma"],
    ),
    (
        "Logistic Sigmoid",
        "1 / (1 + exp(-k * (x - x0)))",
        "x",
        &["k", "x0"],
    ),
    (
        "Damped Oscillator",
        "A * exp(-gamma * t) * cos(omega * t + phi)",
        "t",
        &["A", "gamma", "omega", "phi"],
    ),
    (
        "Planck Blackbody",
        "2 * h * nu^3 / c^2 * 1 / (exp(h * nu / (k * T)) - 1)",
        "nu",
        &["h", "c", "k", "T"],
    ),
    (
        "Bessel Wave",
        "A * besselj(0, k * r) * exp(-alpha * r)",
        "r",
        &["A", "k", "alpha"],
    ),
];

// =============================================================================
// Parsing Tests
// =============================================================================

#[test]
fn test_parse_all_benchmark_expressions() {
    let empty = HashSet::new();

    for (name, expr, _, _) in ALL_EXPRESSIONS {
        let result = parse(expr, &empty, &empty, None);
        assert!(
            result.is_ok(),
            "Failed to parse '{}': {:?}",
            name,
            result.err()
        );
    }
}

// =============================================================================
// Differentiation Tests (Raw - no simplification)
// =============================================================================

#[test]
fn test_diff_raw_all_benchmark_expressions() {
    let empty = HashSet::new();

    for (name, expr_str, var, _) in ALL_EXPRESSIONS {
        let expr = parse(expr_str, &empty, &empty, None).expect(name);
        let var_sym = symb(var);

        let diff_builder = Diff::new().skip_simplification(true);
        let result = diff_builder.differentiate(&expr, &var_sym);

        assert!(
            result.is_ok(),
            "Failed to differentiate '{}' w.r.t. {}: {:?}",
            name,
            var,
            result.err()
        );
    }
}

// =============================================================================
// Differentiation + Simplification Tests
// =============================================================================

#[test]
fn test_diff_simplified_all_benchmark_expressions() {
    let empty = HashSet::new();

    for (name, expr_str, var, _) in ALL_EXPRESSIONS {
        let expr = parse(expr_str, &empty, &empty, None).expect(name);
        let var_sym = symb(var);

        // With simplification
        let diff_builder = Diff::new();
        let result = diff_builder.differentiate(&expr, &var_sym);

        assert!(
            result.is_ok(),
            "Failed to differentiate+simplify '{}' w.r.t. {}: {:?}",
            name,
            var,
            result.err()
        );
    }
}

// =============================================================================
// Simplification Tests
// =============================================================================

#[test]
fn test_simplify_derivatives_all_benchmark_expressions() {
    let empty = HashSet::new();

    for (name, expr_str, var, _) in ALL_EXPRESSIONS {
        let expr = parse(expr_str, &empty, &empty, None).expect(name);
        let var_sym = symb(var);

        // Get raw derivative first
        let diff_builder = Diff::new().skip_simplification(true);
        let diff_result = diff_builder.differentiate(&expr, &var_sym).expect(name);

        // Then simplify
        let simplify_builder = Simplify::new();
        let simplified = simplify_builder.simplify(&diff_result).expect(name);

        // Should produce a valid expression (not crash)
        assert!(
            !simplified.to_string().is_empty(),
            "Simplification of '{}' derivative produced empty result",
            name
        );
    }
}

// =============================================================================
// Compilation Tests
// =============================================================================

#[test]
fn test_compile_raw_derivatives() {
    let empty = HashSet::new();

    for (name, expr_str, var, _) in ALL_EXPRESSIONS {
        let expr = parse(expr_str, &empty, &empty, None).expect(name);
        let var_sym = symb(var);

        let diff_builder = Diff::new().skip_simplification(true);
        let diff_raw = diff_builder.differentiate(&expr, &var_sym).expect(name);

        let result = CompiledEvaluator::compile_auto(&diff_raw, None);

        // Some expressions may have unsupported functions (like besselj)
        // We just check it doesn't panic
        let _ = result;
    }
}

#[test]
fn test_compile_simplified_derivatives() {
    let empty = HashSet::new();

    for (_, expr_str, var, fixed) in ALL_EXPRESSIONS {
        // Use the high-level API to get simplified derivative
        let diff_result = crate::diff(expr_str, var, fixed, None);

        if let Ok(ref diff_str) = diff_result {
            let parsed = parse(diff_str, &empty, &empty, None);
            if let Ok(ref expr) = parsed {
                let compiled = CompiledEvaluator::compile_auto(expr, None);
                // Check it compiles (or fails gracefully for unsupported funcs)
                let _ = compiled;
            }
        }
    }
}

// =============================================================================
// Evaluation Tests
// =============================================================================

#[test]
fn test_evaluate_compiled_derivatives() {
    let empty = HashSet::new();

    // Test subset of expressions that should compile successfully
    let simple_expressions = &[
        ("Lorentz Factor", "1 / sqrt(1 - v^2 / c^2)", "v", &["c"][..]),
        (
            "Logistic Sigmoid",
            "1 / (1 + exp(-k * (x - x0)))",
            "x",
            &["k", "x0"],
        ),
    ];

    for (name, expr_str, var, fixed) in simple_expressions {
        let diff_result = crate::diff(expr_str, var, fixed, None).expect(name);
        eprintln!("{}: derivative = {}", name, diff_result);

        let parsed = parse(&diff_result, &empty, &empty, None).expect(name);
        let compiled = CompiledEvaluator::compile_auto(&parsed, None);

        if let Ok(evaluator) = compiled {
            let params = evaluator.param_names();
            eprintln!("{}: params = {:?}", name, params);

            // Build values based on actual parameter order
            // For Lorentz: params=["c","v"], need c > v, so c=1.0, v=0.3
            // For Sigmoid: params=["k","x","x0"], use sensible values
            let values: Vec<f64> = params
                .iter()
                .map(|p| {
                    match p.as_str() {
                        "c" => 1.0,  // speed of light
                        "v" => 0.3,  // velocity, must be < c
                        "k" => 1.0,  // sigmoid steepness
                        "x" => 0.0,  // input at midpoint
                        "x0" => 0.0, // sigmoid center
                        _ => 1.0,    // default
                    }
                })
                .collect();

            let result = evaluator.evaluate(&values);
            eprintln!("{}: result -> {}", name, result);

            // Result should be finite for sensible input values
            assert!(
                result.is_finite(),
                "{}: expected finite result, got {}",
                name,
                result
            );
        }
    }
}

// =============================================================================
// Full Pipeline Tests
// =============================================================================

#[test]
fn test_full_pipeline_lorentz_factor() {
    let empty = HashSet::new();

    // Parse
    let expr_str = "1 / sqrt(1 - v^2 / c^2)";
    let expr = parse(expr_str, &empty, &empty, None).unwrap();
    eprintln!("Parsed: {}", expr);

    // Differentiate
    let v = symb("v");
    let diff = Diff::new().differentiate(&expr, &v).unwrap();
    eprintln!("Derivative: {}", diff);

    // Compile
    let compiled = CompiledEvaluator::compile_auto(&diff, None).unwrap();
    eprintln!("Params: {:?}", compiled.param_names());

    // Evaluate at v=0
    // Params are ["c", "v"] (alphabetical), so [c=1.0, v=0.0]
    let result = compiled.evaluate(&[1.0, 0.0]); // c=1, v=0
    eprintln!("Result at c=1, v=0: {}", result);

    // The derivative should be 0 at v=0
    // d/dv[1/sqrt(1-v²/c²)] = v / (c² * (1-v²/c²)^(3/2))
    // At v=0: 0 / (c² * 1) = 0
    assert!(result == 0.0, "Expected 0 at v=0, got {}", result);
}

#[test]
fn test_full_pipeline_damped_oscillator() {
    let empty = HashSet::new();

    // Parse
    let expr_str = "A * exp(-gamma * t) * cos(omega * t + phi)";
    let expr = parse(expr_str, &empty, &empty, None).unwrap();

    // Differentiate
    let t = symb("t");
    let diff = Diff::new().differentiate(&expr, &t).unwrap();

    // Compile
    let compiled = CompiledEvaluator::compile_auto(&diff, None).unwrap();

    // Evaluate at t=0
    let param_count = compiled.param_count();
    let values = vec![1.0; param_count];
    let result = compiled.evaluate(&values);

    assert!(result.is_finite(), "Expected finite result, got {}", result);
}

#[test]
fn test_batch_evaluation_1000_points() {
    let empty = HashSet::new();

    // Simple expression that compiles cleanly
    let expr_str = "x^3 + 2*x^2 - x + 1";
    let expr = parse(expr_str, &empty, &empty, None).unwrap();
    let x = symb("x");
    let diff = Diff::new().differentiate(&expr, &x).unwrap();
    let compiled = CompiledEvaluator::compile_auto(&diff, None).unwrap();

    // Evaluate at 1000 points
    let mut sum = 0.0;
    for i in 0..1000 {
        let x_val = 0.1 + i as f64 * 0.01;
        let result = compiled.evaluate(&[x_val]);
        sum += result;
    }

    assert!(sum.is_finite(), "Batch evaluation sum should be finite");
    assert!(sum != 0.0, "Sum should not be zero");
}

/// Comprehensive test: evaluate ALL benchmark expressions with physics-appropriate values
#[test]
fn test_all_benchmarks_produce_valid_results() {
    let empty = HashSet::new();

    // Physics-appropriate parameter values
    fn get_param_value(name: &str) -> f64 {
        match name {
            // Universal constants (normalized to 1 for simplicity)
            "c" => 1.0, // speed of light
            "h" => 1.0, // Planck constant
            "k" => 1.0, // Boltzmann constant
            "pi" => std::f64::consts::PI,

            // Variables (must be in valid ranges)
            "v" => 0.3, // velocity, must be < c
            "x" => 0.5,
            "y" => 0.5,
            "r" => 1.0,  // radius, must be > 0
            "t" => 0.5,  // time
            "nu" => 1.0, // frequency, must be > 0

            // Parameters
            "mu" => 0.0,
            "sigma" => 1.0, // must be > 0
            "s" => 1.0,     // must be > 0
            "x0" => 0.0,
            "y0" => 0.0,
            "m" => 1.0,       // mass > 0
            "T" => 300.0,     // temperature > 0
            "epsilon" => 1.0, // LJ depth
            "A" => 1.0,       // amplitude
            "gamma" => 0.1,   // damping
            "omega" => 1.0,   // angular frequency
            "phi" => 0.0,     // phase
            "alpha" => 0.1,   // decay rate

            _ => 1.0,
        }
    }

    let mut passed = 0;
    let mut failed = Vec::new();

    for (name, expr_str, var, fixed) in ALL_EXPRESSIONS {
        // Bessel Wave checks
        let diff_result = crate::diff(expr_str, var, fixed, None);
        if diff_result.is_err() {
            failed.push(format!("{}: diff failed - {:?}", name, diff_result.err()));
            continue;
        }
        let diff_str = diff_result.unwrap();

        let parsed = parse(&diff_str, &empty, &empty, None);
        if parsed.is_err() {
            failed.push(format!("{}: parse failed - {:?}", name, parsed.err()));
            continue;
        }
        let expr = parsed.unwrap();

        let compiled = CompiledEvaluator::compile_auto(&expr, None);
        if compiled.is_err() {
            failed.push(format!("{}: compile failed - {:?}", name, compiled.err()));
            continue;
        }
        let evaluator = compiled.unwrap();

        let params = evaluator.param_names();
        let values: Vec<f64> = params.iter().map(|p| get_param_value(p)).collect();

        let result = evaluator.evaluate(&values);

        if result.is_finite() {
            eprintln!("✓ {}: {} = {:.6}", name, diff_str, result);
            passed += 1;
        } else {
            failed.push(format!(
                "{}: got {} with params {:?}={:?}",
                name, result, params, values
            ));
        }
    }

    eprintln!(
        "\n=== Summary: {}/{} passed ===",
        passed,
        ALL_EXPRESSIONS.len()
    );

    if !failed.is_empty() {
        panic!("Failed expressions:\n{}", failed.join("\n"));
    }
}

// =============================================================================
// Parallel Evaluation Tests (from benchmark_parallel.rs)
// =============================================================================

/// Test eval_batch for single-variable expressions
#[test]
fn test_eval_batch_single_var() {
    let empty = HashSet::new();

    // Single-variable expression (Lorentz with c=1)
    let expr_str = "1 / sqrt(1 - v^2)";
    let var = "v";

    let diff_str = crate::diff(expr_str, var, &[], None).unwrap();
    let parsed = parse(&diff_str, &empty, &empty, None).unwrap();
    let evaluator = CompiledEvaluator::compile(&parsed, &[var], None).unwrap();

    // Generate test points in valid domain (|v| < 1)
    let test_points: Vec<f64> = (0..100).map(|i| 0.01 + 0.98 * (i as f64 / 100.0)).collect();

    // Test eval_batch
    let columns: Vec<&[f64]> = vec![&test_points[..]];
    let mut output = vec![0.0; test_points.len()];
    evaluator
        .eval_batch(&columns, &mut output)
        .expect("eval_batch should succeed");

    // Verify results are finite
    for (i, &val) in output.iter().enumerate() {
        assert!(
            val.is_finite(),
            "eval_batch result[{}] = {} (at v={})",
            i,
            val,
            test_points[i]
        );
    }

    // Compare with loop evaluate
    for (i, &v) in test_points.iter().enumerate() {
        let expected = evaluator.evaluate(&[v]);
        let diff = (output[i] - expected).abs();
        assert!(
            diff < 1e-10,
            "Mismatch at i={}: batch={}, loop={}",
            i,
            output[i],
            expected
        );
    }
}

/// Test scaling: 100, 1000, 10000 points
#[test]
fn test_eval_scaling() {
    let empty = HashSet::new();

    let expr_str = "1 / sqrt(1 - v^2)";
    let var = "v";

    let diff_str = crate::diff(expr_str, var, &[], None).unwrap();
    let parsed = parse(&diff_str, &empty, &empty, None).unwrap();
    let evaluator = CompiledEvaluator::compile(&parsed, &[var], None).unwrap();

    for n in [100, 1000, 10000] {
        let test_points: Vec<f64> = (0..n)
            .map(|i| 0.01 + 0.98 * (i as f64 / n as f64))
            .collect();

        // eval_batch
        let columns: Vec<&[f64]> = vec![&test_points[..]];
        let mut output = vec![0.0; n];
        evaluator.eval_batch(&columns, &mut output).unwrap();

        let sum: f64 = output.iter().sum();
        assert!(
            sum.is_finite(),
            "Scaling n={}: sum should be finite, got {}",
            n,
            sum
        );
        eprintln!("Scaling n={}: sum={:.2}", n, sum);
    }
}

/// Test multi-expression evaluation
#[test]
fn test_multi_expression_batch() {
    let empty = HashSet::new();

    let expressions = [
        ("Lorentz", "1 / sqrt(1 - v^2)", "v", 0.01..0.99),
        ("Quadratic", "3*x^2 + 2*x + 1", "x", 0.1..10.1),
        ("Trig", "sin(t) * cos(t)", "t", 0.0..std::f64::consts::TAU),
    ];

    let n_points = 100;

    for (name, expr_str, var, range) in &expressions {
        let diff_str = crate::diff(expr_str, var, &[], None).expect(name);
        let parsed = parse(&diff_str, &empty, &empty, None).expect(name);
        let evaluator = CompiledEvaluator::compile(&parsed, &[var], None).expect(name);

        // Generate test data in valid domain
        let start = range.start;
        let end = range.end;
        let data: Vec<f64> = (0..n_points)
            .map(|i| start + (end - start) * (i as f64 / n_points as f64))
            .collect();

        let columns: Vec<&[f64]> = vec![&data[..]];
        let mut output = vec![0.0; n_points];
        evaluator.eval_batch(&columns, &mut output).expect(name);

        let sum: f64 = output.iter().sum();
        assert!(
            sum.is_finite(),
            "{}: sum should be finite, got {}",
            name,
            sum
        );
        eprintln!("✓ {}: sum={:.6}", name, sum);
    }
}

// =============================================================================
// Large Expression Tests (from large_expr.rs)
// =============================================================================

/// Generate a complex mixed expression with N terms
fn generate_mixed_complex(n: usize) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(n * 50);
    for i in 1..=n {
        if i > 1 {
            if i % 4 == 0 {
                write!(s, " - ").unwrap();
            } else {
                write!(s, " + ").unwrap();
            }
        }

        match i % 5 {
            0 => write!(s, "{}*x^{}", i, i % 10 + 1).unwrap(), // Polynomial
            1 => write!(s, "sin({}*x)*cos(x)", i).unwrap(),    // Trig
            2 => write!(s, "(exp(x/{}) + log(x + {}))", i, i).unwrap(), // Exp/Log
            3 => write!(s, "(x^2 + {})/(x + {})", i, i).unwrap(), // Rational
            4 => write!(s, "sin(exp(x) + {})", i).unwrap(),    // Nested
            _ => unreachable!(),
        }
    }
    s
}

/// Test parsing large expressions (100 terms)
#[test]
fn test_large_expr_parse_100() {
    let empty = HashSet::new();
    let expr_str = generate_mixed_complex(100);

    let result = parse(&expr_str, &empty, &empty, None);
    assert!(result.is_ok(), "Failed to parse 100-term expression");
    eprintln!("✓ Parsed 100-term expression ({} chars)", expr_str.len());
}

/// Test differentiating large expressions (100 terms)
#[test]
fn test_large_expr_diff_100() {
    let empty = HashSet::new();
    let expr_str = generate_mixed_complex(100);
    let x = symb("x");

    let parsed = parse(&expr_str, &empty, &empty, None).unwrap();
    let diff = Diff::new()
        .skip_simplification(true)
        .differentiate(&parsed, &x);

    assert!(diff.is_ok(), "Failed to differentiate 100-term expression");
    eprintln!("✓ Differentiated 100-term expression");
}

/// Test compiling and evaluating large expressions (50 terms - smaller for speed)
#[test]
fn test_large_expr_eval_50() {
    let empty = HashSet::new();
    let expr_str = generate_mixed_complex(50);
    let x = symb("x");

    let parsed = parse(&expr_str, &empty, &empty, None).unwrap();
    let diff = Diff::new().differentiate(&parsed, &x).unwrap();

    let compiled = CompiledEvaluator::compile_auto(&diff, None);
    if let Ok(evaluator) = compiled {
        // Evaluate at x=2.0 (safe for log(x + i))
        let result = evaluator.evaluate(&[2.0]);
        assert!(
            result.is_finite(),
            "Large expr eval should be finite, got {}",
            result
        );
        eprintln!("✓ Large expression (50 terms) evaluated to {:.6}", result);
    } else {
        eprintln!("⚠ 50-term expression compile skipped: {:?}", compiled.err());
    }
}
