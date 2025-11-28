// Comprehensive edge case tests - based on edge_cases.md

mod input_validation {

    use crate::diff;

    #[test]
    fn test_empty_input() {
        assert!(diff("".to_string(), "x".to_string(), None, None).is_err());
    }

    #[test]
    fn test_only_whitespace() {
        assert!(diff("   ".to_string(), "x".to_string(), None, None).is_err());
    }

    #[test]
    fn test_single_symbol() {
        assert_eq!(
            diff("x".to_string(), "x".to_string(), None, None).unwrap(),
            "1"
        );
    }

    #[test]
    fn test_single_different_symbol() {
        assert_eq!(
            diff("y".to_string(), "x".to_string(), None, None).unwrap(),
            "0"
        );
    }

    #[test]
    fn test_only_number() {
        assert_eq!(
            diff("42".to_string(), "x".to_string(), None, None).unwrap(),
            "0"
        );
    }
}

mod number_parsing {

    use crate::diff;

    #[test]
    fn test_leading_dot() {
        let result = diff(".5".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "0"); // derivative of 0.5
    }

    #[test]
    fn test_trailing_dot() {
        let result = diff("5.".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "0"); // derivative of 5.0
    }

    #[test]
    fn test_dot_decimal() {
        let result = diff("3.14*x".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("3.14"));
    }

    #[test]
    fn test_scientific_notation_basic() {
        let result = diff("1e10".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_scientific_notation_negative_exp() {
        let result = diff("2.5e-3".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_scientific_notation_with_var() {
        let result = diff("1e10*x".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("1") || result.contains("e"));
    }

    #[test]
    fn test_multiple_decimals_error() {
        assert!(diff("3.14.15".to_string(), "x".to_string(), None, None).is_err());
    }
}

mod parentheses_edge_cases {

    use crate::diff;

    #[test]
    fn test_unmatched_open_paren() {
        // (x + 1 should auto-close to (x + 1)
        let result = diff("(x+1".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_unmatched_close_paren() {
        // x + 1) should auto-open to (x + 1)
        let result = diff("x+1)".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_multiple_unmatched_open() {
        let result = diff("((x+1".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_empty_parens() {
        // () should be treated as 1
        let result = diff("()".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "0"); // d/dx(1) = 0
    }

    #[test]
    fn test_empty_parens_with_mul() {
        // x*() = x*1 = x, derivative is 1
        let result = diff("x*()".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_nested_empty_parens() {
        let result = diff("(())".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "0"); // () = 1, (()) = (1) = 1
    }

    #[test]
    fn test_wrong_order_parens() {
        // )(x should be wrapped
        let result = diff(")(x".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "1");
    }
}

mod symbol_grouping {

    use crate::diff;

    #[test]
    fn test_cosine_without_fixed_vars() {
        // "cosine" should NOT match "cos", becomes individual chars
        let result = diff("cosine".to_string(), "x".to_string(), None, None);
        // Should parse but derivative might be complex
        assert!(result.is_ok());
    }

    #[test]
    fn test_sinus_as_fixed_var() {
        let result = diff(
            "sinus".to_string(),
            "x".to_string(),
            Some(&["sinus".to_string()]),
            None,
        )
        .unwrap();
        assert_eq!(result, "0"); // sinus is constant
    }

    #[test]
    fn test_expense_no_exp_match() {
        // "expense" should NOT match "exp"
        let result = diff("expense".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multi_char_fixed_var() {
        let result = diff(
            "ax*x".to_string(),
            "x".to_string(),
            Some(&["ax".to_string()]),
            None,
        )
        .unwrap();
        assert!(result.contains("ax"));
    }
}

mod differentiation_edge_cases {

    use crate::DiffError;
    use crate::diff;

    #[test]
    fn test_var_not_in_formula() {
        let result = diff("y".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_all_constants() {
        let result = diff(
            "a+b".to_string(),
            "x".to_string(),
            Some(&["a".to_string(), "b".to_string()]),
            None,
        )
        .unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_var_in_both_fixed_and_diff() {
        let result = diff(
            "x".to_string(),
            "x".to_string(),
            Some(&["x".to_string()]),
            None,
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DiffError::VariableInBothFixedAndDiff { .. }
        ));
    }

    #[test]
    fn test_function_of_constant() {
        let result = diff(
            "sin(a)".to_string(),
            "x".to_string(),
            Some(&["a".to_string()]),
            None,
        )
        .unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_nested_functions() {
        let result = diff("sin(cos(x))".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("cos") && result.contains("sin"));
    }
}

mod api_validation {

    use crate::DiffError;
    use crate::diff;

    #[test]
    fn test_name_collision() {
        let result = diff(
            "x".to_string(),
            "x".to_string(),
            Some(&["f".to_string()]),
            Some(&["f".to_string()]),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DiffError::NameCollision { .. }
        ));
    }

    #[test]
    fn test_empty_fixed_vars() {
        let result = diff("x".to_string(), "x".to_string(), Some(&[]), None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_empty_custom_functions() {
        let result = diff("x".to_string(), "x".to_string(), None, Some(&[])).unwrap();
        assert_eq!(result, "1");
    }
}

mod implicit_multiplication {

    use crate::diff;

    #[test]
    fn test_number_variable() {
        let result = diff("2x".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "2");
    }

    #[test]
    fn test_variable_variable() {
        let result = diff(
            "ax".to_string(),
            "x".to_string(),
            Some(&["a".to_string()]),
            None,
        )
        .unwrap();
        assert_eq!(result, "a");
    }

    #[test]
    fn test_paren_multiplication() {
        let result = diff("(x)(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("x")); // Should have product rule result
    }

    #[test]
    fn test_number_paren() {
        let result = diff("2(x+1)".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "2");
    }
}

mod complex_expressions {

    use crate::diff;

    #[test]
    fn test_polynomial() {
        let result = diff(
            "x^3 + 2*x^2 + 3*x + 4".to_string(),
            "x".to_string(),
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_product_chain() {
        let result = diff("x*(x*(x+1))".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_div_with_functions() {
        let result = diff("sin(x)/x".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_composition_chain() {
        let result = diff("sin(cos(exp(x)))".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }
}

mod phase2_features {

    use crate::diff;

    #[test]
    fn test_subtraction_basic() {
        let result = diff("x - 5".to_string(), "x".to_string(), None, None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_subtraction_vars() {
        let result = diff(
            "x - a".to_string(),
            "x".to_string(),
            Some(&["a".to_string()]),
            None,
        )
        .unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_division_basic() {
        let result = diff("x / 2".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_quotient_rule() {
        let result = diff("x / (x + 1)".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sinh_basic() {
        let result = diff("sinh(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("cosh"));
    }

    #[test]
    fn test_cosh_basic() {
        let result = diff("cosh(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sinh"));
    }

    #[test]
    fn test_tanh_basic() {
        let result = diff("tanh(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("tanh") || result.contains("sech"));
    }

    #[test]
    fn test_x_to_x_logarithmic() {
        let result = diff("x^x".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("ln"));
    }

    #[test]
    fn test_x_to_variable_exponent() {
        let result = diff("x^(2*x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("ln")); // Should use logarithmic diff
    }

    #[test]
    fn test_base_to_x() {
        let result = diff(
            "a^x".to_string(),
            "x".to_string(),
            Some(&["a".to_string()]),
            None,
        )
        .unwrap();
        assert!(result.contains("ln")); // a^x uses logarithmic diff
    }
}

mod stress_tests {

    use crate::diff;

    #[test]
    fn test_very_long_sum() {
        let formula = (0..20)
            .map(|i| format!("x^{}", i))
            .collect::<Vec<_>>()
            .join(" + ");
        let result = diff(formula, "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deeply_nested() {
        let mut formula = "x".to_string();
        for _ in 0..10 {
            formula = format!("sin({})", formula);
        }
        let result = diff(formula, "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_many_variables() {
        let vars: Vec<String> = (0..20).map(|i| format!("a{}", i)).collect();
        let formula = vars
            .iter()
            .map(|v| format!("{}*x", v))
            .collect::<Vec<_>>()
            .join(" + ");
        let var_refs: Vec<String> = vars.clone();
        let result = diff(formula, "x".to_string(), Some(&var_refs), None);
        assert!(result.is_ok());
    }
}
