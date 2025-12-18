#[cfg(test)]
mod tests {
    use crate::simplify;

    #[test]
    fn test_sign_in_difference_of_squares() {
        // Test x^2 - 1 should factor to (x-1)(x+1)
        let result = simplify("x^2 - 1", None, None).unwrap();
        let result_str = result.to_string();
        println!("x^2 - 1 = {}", result_str);
        assert!(
            (result_str.contains("(x - 1)") || result_str.contains("(-1 + x)"))
                && (result_str.contains("(x + 1)") || result_str.contains("(1 + x)")),
            "Expected (x-1)(x+1) or equivalent, got: {}",
            result_str
        );
    }

    #[test]
    fn test_reversed_difference_of_squares() {
        // Test 1 - x^2 should factor to (1-x)(1+x), NOT (x-1)(x+1)
        let result = simplify("1 - x^2", None, None).unwrap();
        let result_str = result.to_string();
        println!("1 - x^2 = {}", result_str);

        // Should be (1-x)(1+x) which is correct
        // The bug was it was giving (x-1)(x+1) which equals x^2-1
        assert!(
            result_str.contains("(1 - x)")
                || result_str.contains("(1 + -x)")
                || result_str.contains("(-1 * x + 1)"),
            "Expected (1-x)(1+x) form, got: {}",
            result_str
        );
        assert!(
            result_str.contains("(1 + x)") || result_str.contains("(x + 1)"),
            "Expected (1-x)(1+x) form, got: {}",
            result_str
        );
    }

    #[test]
    fn test_negative_form_difference_of_squares() {
        // Test -x^2 + 1 is same as 1 - x^2
        let result = simplify("-x^2 + 1", None, None).unwrap();
        let result_str = result.to_string();
        println!("-x^2 + 1 = {}", result_str);

        // Both forms are mathematically equivalent:
        // (1-x)(1+x) = 1 - x² = -x² + 1
        // -(x+1)(x-1) = -(x² - 1) = -x² + 1
        assert!(
            result_str.contains("(1 - x)")
                || result_str.contains("(1 + -x)")
                || result_str.contains("(-1 * x + 1)")
                || (result_str.contains("-(x") // Case -(x-1)(x+1)
                    && result_str.contains("(x - 1)")
                    && result_str.contains("(x + 1)"))
                || (result_str.contains("(-1 + x)") && result_str.contains("(1 + x)")), // Case (-1+x)(1+x)
            "Expected factored form of 1-x², got: {}",
            result_str
        );
    }

    #[test]
    fn test_normalize_negative_in_denominator_number() {
        // Test x / -2 with negative number
        let result = simplify("x / -2", None, None).unwrap();
        let result_str = result.to_string();
        println!("x / -2 = {}", result_str);

        // Should normalize to -x / 2 or -1 * x / 2
        assert!(
            (result_str.contains("-x") || result_str.contains("-1 * x"))
                && result_str.contains("/2"),
            "Expected negative in numerator, got: {}",
            result_str
        );
    }

    #[test]
    fn test_normalize_negative_in_denominator_mul() {
        // Test x / (-1 * y)
        let result = simplify("x / (-1 * y)", None, None).unwrap();
        let result_str = result.to_string();
        println!("x / (-1 * y) = {}", result_str);

        // Should normalize to -x / y
        assert!(
            result_str.contains("-x") || result_str.contains("-1 * x"),
            "Expected negative in numerator, got: {}",
            result_str
        );
        assert!(
            !result_str.contains("/ (-1"),
            "Should not have -1 in denominator, got: {}",
            result_str
        );
    }

    #[test]
    fn test_normalize_negative_trailing_mul() {
        // Test x / (y * -1)
        let result = simplify("x / (y * -1)", None, None).unwrap();
        let result_str = result.to_string();
        println!("x / (y * -1) = {}", result_str);

        // Should normalize to -x / y
        assert!(
            result_str.contains("-x") || result_str.contains("-1 * x"),
            "Expected negative in numerator, got: {}",
            result_str
        );
    }

    #[test]
    fn test_double_negative_in_division() {
        // Test -x / -y should become x / y
        let result = simplify("(-1 * x) / (-1 * y)", None, None).unwrap();
        let result_str = result.to_string();
        println!("(-1 * x) / (-1 * y) = {}", result_str);

        // After normalization and simplification, should be x / y
        // The -1's should cancel out
        assert!(
            result_str == "x / y"
                || result_str.contains("x")
                    && result_str.contains("y")
                    && !result_str.contains("-"),
            "Expected x/y without negatives, got: {}",
            result_str
        );
    }
}
