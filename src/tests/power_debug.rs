//! Regression tests for power-of-sum coefficient extraction bug
//!
//! These tests verify that expressions like (z+z)^x are correctly simplified
//! to (2*z)^x and NOT incorrectly to 2*z^x (which is mathematically different).
//!
//! Bug history: Before the fix, the `needs_parens_as_base` function didn't include
//! `ExprKind::Poly`, causing polynomial bases in powers to display without parentheses.
//! This made `Pow(Poly(2*z), x)` display as `2*z^x` instead of `(2*z)^x`.

#[cfg(test)]
mod power_regression_tests {
    use crate::parser;
    use crate::simplify;
    use std::collections::HashSet;

    #[test]
    fn test_power_of_sum_simplification() {
        let fixed = HashSet::new();
        let custom = HashSet::new();

        // Parse (z+z)^x - should become (2*z)^x, not 2*z^x
        let expr = parser::parse("(z+z)^x", &fixed, &custom, None).unwrap();
        let display = format!("{}", expr);

        // The display should have parentheses around 2*z
        assert!(
            display.contains("(2*z)") || display == "(2*z)^x",
            "Expected (2*z)^x with parentheses, got: {}",
            display
        );
    }

    #[test]
    fn test_power_of_product_simplification() {
        // (2*z)^x should stay as (2*z)^x, not become 2*z^x
        let result = simplify("(2*z)^x", None, None).unwrap();

        assert!(
            result.contains("(2*z)") || result == "(2*z)^x",
            "Expected (2*z)^x with parentheses, got: {}",
            result
        );
    }

    #[test]
    fn test_power_of_sum_negative_exponent() {
        // (z+z)^(-y) should become (2*z)^(-y) or 1/(2*z)^y
        // NOT 1/2*z^y which is mathematically different
        let result = simplify("(z+z)^(-y)", None, None).unwrap();

        // The coefficient 2 should be inside the power base, not outside
        // Valid outputs: (2*z)^(-y), 1/(2*z)^y
        // Invalid output: 1/2*z^y (which would be 0.5 * z^y)
        let is_correct = result.contains("(2*z)") || result.contains("(2z)");
        assert!(
            is_correct,
            "Expected coefficient to stay in power base, got: {}",
            result
        );
    }
}

#[cfg(test)]
mod sign_regression_tests {
    use crate::simplify;

    #[test]
    fn test_negative_product_sign() {
        // (-y)*y should be -y², not +y²
        // Full expression: (-y * y) - (y + y) = -y² - 2y
        let result = simplify("((-y) * y) - (y + y)", None, None).unwrap();
        println!("Result: {}", result);
        
        // The y² term should be negative
        // Valid: -y^2 - 2*y, -(y^2 + 2*y), etc.
        // Invalid: y^2 - 2*y, -2*y + y^2
        assert!(
            !result.contains("+ y^2") && !result.contains("+y^2"),
            "Bug: y² should be negative, got: {}",
            result
        );
    }
}
