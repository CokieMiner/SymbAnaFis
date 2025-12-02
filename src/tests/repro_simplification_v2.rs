#[cfg(test)]
mod tests {
    use crate::simplify;

    #[test]
    fn test_quantum_mechanics_sigma_combination() {
        // sigma * sqrt(pi) * sigma^2 -> sigma^3 * sqrt(pi)
        // The issue is that they might be separated by sqrt(pi)
        let expr = "sigma * sqrt(pi) * sigma^2";
        let simplified = simplify(
            expr.to_string(),
            Some(&["pi".to_string(), "sigma".to_string()]),
            None,
        )
        .unwrap();
        println!("Quantum Mechanics: {}", simplified);
        // Expect sigma^3 * sqrt(pi) (order might vary, but sigma terms should be combined)
        assert!(simplified.contains("sigma^3"));
        assert!(!simplified.contains("sigma *"));
    }

    #[test]
    fn test_heat_conduction_power_expansion() {
        // (x / (2 * sqrt(alpha) * sqrt(t)))^2 -> x^2 / (4 * alpha * t)
        let expr = "(x / (2 * sqrt(alpha) * sqrt(t)))^2";
        let simplified = simplify(
            expr.to_string(),
            Some(&["alpha".to_string(), "t".to_string()]),
            None,
        )
        .unwrap();
        println!("Heat Conduction: {}", simplified);
        assert_eq!(simplified, "x^2 / (4 * alpha * t)");
    }
    #[test]
    fn test_sqrt_combination() {
        // sqrt(2) * sqrt(pi) -> sqrt(2 * pi)
        let expr = "sqrt(2) * sqrt(pi)";
        let simplified = simplify(expr.to_string(), Some(&["pi".to_string()]), None).unwrap();
        println!("Sqrt Combination: {}", simplified);
        assert_eq!(simplified, "sqrt(2 * pi)");
    }
    #[test]
    fn test_nested_sqrt_cancellation() {
        // sqrt(sigma * sqrt(pi)) / (sigma^3 * sqrt(pi)^0.5 * (pi * sigma)^0.5)
        // This is a complex nested sqrt expression that should simplify
        let expr = "sqrt(sigma * sqrt(pi)) / (sigma^3 * sqrt(pi)^0.5 * (pi * sigma)^0.5)";
        let simplified = simplify(
            expr.to_string(),
            Some(&["sigma".to_string(), "pi".to_string()]),
            None,
        )
        .unwrap();
        println!("Nested Sqrt: {}", simplified);

        // The expression should simplify to something involving sigma^3 in the denominator
        // and sqrt(pi) terms. The exact form may vary but should contain these elements.
        assert!(
            simplified.contains("sigma^3") || simplified.contains("sigma ^ 3"),
            "Expected sigma^3 in result, got: {}",
            simplified
        );
        // Should have sqrt or pi terms
        assert!(
            simplified.contains("sqrt") || simplified.contains("pi"),
            "Expected sqrt or pi terms in result, got: {}",
            simplified
        );
        // Should be a fraction (division)
        assert!(
            simplified.contains("/"),
            "Expected a fraction, got: {}",
            simplified
        );
    }

    #[test]
    fn test_normal_pdf_derivative() {
        // Normal PDF: f(x) = exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)
        // This caused stack overflow in applications example
        let expr = "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)";
        let simplified = simplify(
            expr.to_string(),
            Some(&["mu".to_string(), "sigma".to_string(), "pi".to_string()]),
            None,
        )
        .unwrap();
        println!("Normal PDF Simplified: {}", simplified);
    }
}
