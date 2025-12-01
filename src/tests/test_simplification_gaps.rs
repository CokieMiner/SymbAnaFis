#[cfg(test)]
mod tests {
    use crate::simplify;

    #[test]
    fn test_cancellation_in_addition() {
        // (1 + x - x) should simplify to 1
        let expr = "(1 + x - x)";
        let result = simplify(expr.to_string(), None, None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_combine_implicit_coefficients() {
        // 5*x - x should simplify to 4*x
        let expr = "5*x - x";
        let result = simplify(expr.to_string(), None, None).unwrap();
        assert_eq!(result, "4 * x");
    }

    #[test]
    fn test_inverse_functions() {
        // exp(ln(x^2)) should simplify to x^2
        let expr = "exp(ln(x^2))";
        let result = simplify(expr.to_string(), None, None).unwrap();
        assert_eq!(result, "x^2");
    }

    #[test]
    fn test_sqrt_simplification() {
        // sqrt(x^4) should simplify to x^2
        let expr = "sqrt(x^4)";
        let result = simplify(expr.to_string(), None, None).unwrap();
        assert_eq!(result, "x^2");
    }

    #[test]
    fn test_combined_case() {
        // exp(ln(x^2)) + sqrt(x^4) -> x^2 + x^2 -> 2*x^2
        let expr = "exp(ln(x^2)) + sqrt(x^4)";
        let result = simplify(expr.to_string(), None, None).unwrap();
        assert_eq!(result, "2 * x^2");
    }

    #[test]
    fn test_pow_e_ln_combined_case() {
        // e^(ln(x^2)) + sqrt(x^4) -> x^2 + x^2 -> 2*x^2
        let expr = "e^(ln(x^2)) + sqrt(x^4)";
        let result = simplify(expr.to_string(), None, None).unwrap();
        assert_eq!(result, "2 * x^2");
    }

    #[test]
    fn test_pow_e_ln_single() {
        // e^(ln(x^2)) -> x^2
        let expr = "e^(ln(x^2))";
        let result = simplify(expr.to_string(), None, None).unwrap();
        assert_eq!(result, "x^2");
    }

    #[test]
    fn test_pow_e_complex_exponent_ln() {
        // e^(sin(x) * ln(y)) -> y^sin(x)
        let expr = "e^(sin(x) * ln(y))";
        let result = simplify(expr.to_string(), None, None).unwrap();
        assert_eq!(result, "y^(sin(x))");
    }
}
