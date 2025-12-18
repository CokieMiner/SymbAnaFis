#[cfg(test)]
mod tests {
    use crate::ExprKind;
    use crate::parser::parse;
    use crate::simplify;
    use std::collections::HashSet;

    #[test]
    fn test_difference_of_squares_debug() {
        // Parse (1 + x) * (1 - x)
        let vars: HashSet<String> = ["x".to_string()].into_iter().collect();
        let funcs: HashSet<String> = HashSet::new();
        let expr = parse("(1 + x) * (1 - x)", &vars, &funcs, None).unwrap();
        println!("Parsed: {:?}", expr);

        // Check structure - now uses Product and Sum with negated terms
        if let ExprKind::Product(factors) = &expr.kind {
            println!("factors: {:?}", factors);
            for (i, factor) in factors.iter().enumerate() {
                println!("factor[{}] = {:?}", i, factor);
            }
        }
    }

    #[test]
    fn test_thermodynamics_heat_flux() {
        // Current output: T0 * 1 / sqrt(alpha) * exp(x^2 / (-4 * alpha * t)) * 1 / sqrt(t) / sqrt(pi)
        // Expected: T0/(√(π·α·t)) * exp(-x²/(4αt))
        // Which is: T0 / sqrt(pi * alpha * t) * exp(-x^2 / (4 * alpha * t))

        let result = "T0 * 1 / sqrt(alpha) * exp(x^2 / (-4 * alpha * t)) * 1 / sqrt(t) / sqrt(pi)";
        let simplified = simplify(result, Some(&["T0", "alpha", "t", "x"]), None).unwrap();

        println!("Thermodynamics heat flux:");
        println!("Input:  {}", result);
        println!("Output: {}", simplified);
        println!("Expected: T0 * exp(-x^2 / (4 * alpha * t)) / sqrt(pi * alpha * t)");
        println!();

        // Test intermediate steps
        let test1 = "1 / sqrt(alpha) * 1 / sqrt(t) / sqrt(pi)";
        let simp1 = simplify(test1, Some(&["alpha", "t"]), None).unwrap();
        println!("Test1: {} => {}", test1, simp1);
        println!("Expected: 1 / sqrt(pi * alpha * t)");

        let test2 = "exp(x^2 / (-4 * alpha * t))";
        let simp2 = simplify(test2, Some(&["alpha", "t", "x"]), None).unwrap();
        println!("Test2: {} => {}", test2, simp2);
        println!("Expected: exp(-x^2 / (4 * alpha * t))");
    }

    #[test]
    fn test_quantum_probability() {
        // Current: -exp(x^2 / (-2 * sigma^2)) * x / (sqrt(pi) * sigma^3)
        // Expected: -x/(σ³√π) * exp(-x²/(2σ²))
        // Which is: -x * exp(-x^2 / (2 * sigma^2)) / (sigma^3 * sqrt(pi))

        let result = "-exp(x^2 / (-2 * sigma^2)) * x / (sqrt(pi) * sigma^3)";
        let simplified = simplify(result, Some(&["sigma", "x"]), None).unwrap();

        println!("\nQuantum probability:");
        println!("Input:  {}", result);
        println!("Output: {}", simplified);
        println!("Expected: -x * exp(-x^2 / (2 * sigma^2)) / (sigma^3 * sqrt(pi))");
    }

    #[test]
    fn test_statistics_normal_pdf() {
        // Current: sqrt(2) * (x - mu) * exp((x - mu)^2 / (-2 * sigma^2)) * 1 / sqrt(pi) / (-2 * sigma^3)
        // Expected: -(x-μ)/(σ²) * f(x) or -(x - mu) * exp(-(x - mu)^2 / (2 * sigma^2)) / (sigma^2 * sqrt(2 * pi * sigma^2))

        let result =
            "sqrt(2) * (x - mu) * exp((x - mu)^2 / (-2 * sigma^2)) * 1 / sqrt(pi) / (-2 * sigma^3)";
        let simplified = simplify(result, Some(&["sigma", "x", "mu"]), None).unwrap();

        println!("\nStatistics normal PDF:");
        println!("Input:  {}", result);
        println!("Output: {}", simplified);
        println!(
            "Expected: -(x - mu) * exp(-(x - mu)^2 / (2 * sigma^2)) / (sigma^3 * sqrt(2 * pi))"
        );
    }

    #[test]
    fn test_relativity_time_dilation() {
        // Current: -t * v / (sqrt((c + v) * (1 - v / c) / c) * c^2)
        // Expected: -t·v/(c²√(1-v²/c²))
        // Which is: -t * v / (c^2 * sqrt(1 - v^2 / c^2))

        let result = "-t * v / (sqrt((c + v) * (1 - v / c) / c) * c^2)";
        let simplified = simplify(result, Some(&["t", "v", "c"]), None).unwrap();

        println!("\nRelativity time dilation:");
        println!("Input:  {}", result);
        println!("Output: {}", simplified);
        println!("Expected: -t * v / (c^2 * sqrt(1 - v^2 / c^2))");

        // Test the inner part
        let test1 = "sqrt((c + v) * (1 - v / c) / c)";
        let simp1 = simplify(test1, Some(&["c", "v"]), None).unwrap();
        println!("Test inner: {} => {}", test1, simp1);
        println!("Expected: sqrt(1 - v^2 / c^2)");

        // Expand (c + v) * (1 - v/c)
        let test2 = "(c + v) * (1 - v / c)";
        let simp2 = simplify(test2, Some(&["c", "v"]), None).unwrap();
        println!("Test expand: {} => {}", test2, simp2);
        println!("Expected: c^2 - v^2");
    }

    #[test]
    fn test_astrophysics_orbital() {
        // Current: (1 + ecc) * (1 - ecc) * a * ecc * sin(theta) / (ecc * cos(theta) + 1)^2
        // Expected: a·e(1-e²)·sinθ / (1 + e·cosθ)²
        // Which is: a * ecc * (1 - ecc^2) * sin(theta) / (1 + ecc * cos(theta))^2

        let result = "(1 + ecc) * (1 - ecc) * a * ecc * sin(theta) / (ecc * cos(theta) + 1)^2";
        let simplified = simplify(result, Some(&["a", "ecc", "theta"]), None).unwrap();

        println!("\nAstrophysics orbital:");
        println!("Input:  {}", result);
        println!("Output: {}", simplified);
        println!("Expected: a * ecc * (1 - ecc^2) * sin(theta) / (1 + ecc * cos(theta))^2");

        // Test (1 + ecc) * (1 - ecc)
        let test1 = "(1 + ecc) * (1 - ecc)";
        let simp1 = simplify(test1, Some(&["ecc"]), None).unwrap();
        println!("Test: {} => {}", test1, simp1);
        println!("Expected: 1 - ecc^2");
    }

    #[test]
    fn test_emf_sign() {
        // Current output: B0 * exp(-t / tau) / tau
        // Expected: -B0 * exp(-t / tau) / tau (negative)

        let result = "B0 * exp(-t / tau) / tau";
        let simplified = simplify(result, Some(&["B0", "tau", "t"]), None).unwrap();

        println!("\nEMF (should already be negative in code):");
        println!("Input:  {}", result);
        println!("Output: {}", simplified);
        println!("Note: The code should add -1 * before simplifying");
    }

    #[test]
    fn test_division_of_roots() {
        // Test: 1 / sqrt(a) / sqrt(b) should become 1 / sqrt(a * b)
        let test = "1 / sqrt(a) / sqrt(b)";
        let result = simplify(test, Some(&["a", "b"]), None).unwrap();
        println!("\nDivision of roots:");
        println!("Input:  {}", test);
        println!("Output: {}", result);
        println!("Expected: 1 / sqrt(a * b)");
    }

    #[test]
    fn test_difference_of_squares() {
        // Test: (1 + x) * (1 - x) should become 1 - x^2
        let test = "(1 + x) * (1 - x)";
        let result = simplify(test, Some(&["x"]), None).unwrap();
        println!("\nDifference of squares:");
        println!("Input:  {}", test);
        println!("Output: {}", result);
        println!("Expected: 1 - x^2");
    }

    #[test]
    fn test_negative_in_exponent() {
        // Test: exp(x / (-a)) should become exp(-x / a)
        let test = "exp(x / (-a))";
        let result = simplify(test, Some(&["x", "a"]), None).unwrap();
        println!("\nNegative in exponent:");
        println!("Input:  {}", test);
        println!("Output: {}", result);
        println!("Expected: exp(-x / a)");
    }
}
