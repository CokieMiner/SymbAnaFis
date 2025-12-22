// Comprehensive edge case tests - based on edge_cases.md

mod input_validation {

    use crate::diff;

    #[test]
    fn test_empty_input() {
        assert!(diff("", "x", None, None).is_err());
    }

    #[test]
    fn test_only_whitespace() {
        assert!(diff("   ", "x", None, None).is_err());
    }

    #[test]
    fn test_single_symbol() {
        assert_eq!(diff("x", "x", None, None).unwrap(), "1");
    }

    #[test]
    fn test_single_different_symbol() {
        assert_eq!(diff("y", "x", None, None).unwrap(), "0");
    }

    #[test]
    fn test_only_number() {
        assert_eq!(diff("42", "x", None, None).unwrap(), "0");
    }
}

mod number_parsing {

    use crate::diff;

    #[test]
    fn test_leading_dot() {
        let result = diff(".5", "x", None, None).unwrap();
        assert_eq!(result, "0"); // derivative of 0.5
    }

    #[test]
    fn test_trailing_dot() {
        let result = diff("5.", "x", None, None).unwrap();
        assert_eq!(result, "0"); // derivative of 5.0
    }

    #[test]
    fn test_dot_decimal() {
        let result = diff("3.14*x", "x", None, None).unwrap();
        assert!(result.contains("3.14"));
    }

    #[test]
    fn test_scientific_notation_basic() {
        let result = diff("1e10", "x", None, None).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_scientific_notation_negative_exp() {
        let result = diff("2.5e-3", "x", None, None).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_scientific_notation_with_var() {
        let result = diff("1e10*x", "x", None, None).unwrap();
        assert!(result.contains("1") || result.contains("e"));
    }

    #[test]
    fn test_multiple_decimals_error() {
        assert!(diff("3.14.15", "x", None, None).is_err());
    }
}

mod parentheses_edge_cases {

    use crate::ExprKind;
    use crate::diff;
    use std::collections::HashSet;

    #[test]
    fn test_unmatched_open_paren() {
        // (x + 1 should auto-close to (x + 1)
        let result = diff("(x+1", "x", None, None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_unmatched_close_paren() {
        // x + 1) should auto-open to (x + 1)
        let result = diff("x+1)", "x", None, None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_multiple_unmatched_open() {
        let result = diff("((x+1", "x", None, None).unwrap();
        assert_eq!(result, "1");
    }

    // Test for complex arguments
    #[test]
    fn test_complex_derivative_args() {
        // Test parsing of ∂^1_f(x+y)/∂_x^1
        // This requires the lexer to recursively lex "x+y" and proper AST construction
        let fixed_vars = HashSet::new();
        let custom_funcs = HashSet::new();
        let expr = crate::parser::parse("∂_f(x+y)/∂_x", &fixed_vars, &custom_funcs, None).unwrap();
        // Check structure: Derivative { inner: FunctionCall(f, args=[Add(x,y)]), var: x, order: 1 }
        if let ExprKind::Derivative { inner, var, order } = expr.kind {
            assert_eq!(var, "x");
            assert_eq!(order, 1);
            if let ExprKind::FunctionCall { name, args } = &inner.kind {
                assert_eq!(name.as_str(), "f");
                assert_eq!(args.len(), 1);
                // Verify arg is x+y (now Sum)
                match &args[0].kind {
                    ExprKind::Sum(terms) => {
                        assert!(terms.len() == 2);
                        let has_x = terms
                            .iter()
                            .any(|t| matches!(&t.kind, ExprKind::Symbol(s) if s.as_str() == "x"));
                        let has_y = terms
                            .iter()
                            .any(|t| matches!(&t.kind, ExprKind::Symbol(s) if s.as_str() == "y"));
                        assert!(has_x && has_y, "Expected x and y in sum");
                    }
                    _ => panic!("Expected Sum expression for argument"),
                }
            } else {
                panic!("Expected FunctionCall inside Derivative");
            }
        } else {
            panic!("Expected Derivative expression");
        }
    }

    #[test]
    fn test_multi_arg_derivative() {
        // Test parsing of ∂_f(x, y)/∂_x (function with 2 arguments)
        let fixed_vars = HashSet::new();
        let custom_funcs = HashSet::new();
        let expr = crate::parser::parse("∂_f(x,y)/∂_x", &fixed_vars, &custom_funcs, None).unwrap();

        if let ExprKind::Derivative { inner, .. } = expr.kind {
            if let ExprKind::FunctionCall { name, args } = &inner.kind {
                assert_eq!(name.as_str(), "f");
                assert_eq!(args.len(), 2);
                // Verify args are x and y
                assert!(matches!(args[0].kind, ExprKind::Symbol(ref s) if s.as_str() == "x"));
                assert!(matches!(args[1].kind, ExprKind::Symbol(ref s) if s.as_str() == "y"));
            } else {
                panic!("Expected FunctionCall");
            }
        } else {
            panic!("Expected Derivative");
        }
    }

    #[test]
    fn test_empty_parens() {
        // () should be an error
        assert!(diff("()", "x", None, None).is_err());
    }

    #[test]
    fn test_empty_parens_with_mul() {
        // x*() should be an error
        assert!(diff("x*()", "x", None, None).is_err());
    }

    #[test]
    fn test_nested_empty_parens() {
        assert!(diff("(())", "x", None, None).is_err());
    }

    #[test]
    fn test_wrong_order_parens() {
        // )(x is invalid
        assert!(diff(")(x", "x", None, None).is_err());
    }
}

mod symbol_grouping {

    use crate::diff;

    #[test]
    fn test_cosine_without_fixed_vars() {
        // "cosine" should NOT match "cos", becomes individual chars
        let result = diff("cosine", "x", None, None);
        // Should parse but derivative might be complex
        assert!(result.is_ok());
    }

    #[test]
    fn test_sinus_as_fixed_var() {
        let result = diff("sinus", "x", Some(&["sinus"]), None).unwrap();
        assert_eq!(result, "0"); // sinus is constant
    }

    #[test]
    fn test_expense_no_exp_match() {
        // "expense" should NOT match "exp"
        let result = diff("expense", "x", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multi_char_fixed_var() {
        let result = diff("ax*x", "x", Some(&["ax"]), None).unwrap();
        assert!(result.contains("ax"));
    }
}

mod differentiation_edge_cases {

    use crate::DiffError;
    use crate::diff;

    #[test]
    fn test_var_not_in_formula() {
        let result = diff("y", "x", None, None).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_all_constants() {
        let result = diff("a+b", "x", Some(&["a", "b"]), None).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_var_in_both_fixed_and_diff() {
        let result = diff("x", "x", Some(&["x"]), None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DiffError::VariableInBothFixedAndDiff { .. }
        ));
    }

    #[test]
    fn test_function_of_constant() {
        let result = diff("sin(a)", "x", Some(&["a"]), None).unwrap();
        assert_eq!(result, "0");
    }

    #[test]
    fn test_nested_functions() {
        let result = diff("sin(cos(x))", "x", None, None).unwrap();
        assert!(result.contains("cos") && result.contains("sin"));
    }
}

mod api_validation {

    use crate::DiffError;
    use crate::diff;

    #[test]
    fn test_name_collision() {
        let result = diff("x", "x", Some(&["f"]), Some(&["f"]));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DiffError::NameCollision { .. }
        ));
    }

    #[test]
    fn test_empty_fixed_vars() {
        let result = diff("x", "x", Some(&[]), None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_empty_custom_functions() {
        let result = diff("x", "x", None, Some(&[])).unwrap();
        assert_eq!(result, "1");
    }
}

mod implicit_multiplication {

    use crate::diff;

    #[test]
    fn test_number_variable() {
        let result = diff("2x", "x", None, None).unwrap();
        assert_eq!(result, "2");
    }

    #[test]
    fn test_variable_variable() {
        let result = diff("ax", "x", Some(&["a"]), None).unwrap();
        assert_eq!(result, "a");
    }

    #[test]
    fn test_paren_multiplication() {
        let result = diff("(x)(x)", "x", None, None).unwrap();
        assert!(result.contains("x")); // Should have product rule result
    }

    #[test]
    fn test_number_paren() {
        let result = diff("2(x+1)", "x", None, None).unwrap();
        assert_eq!(result, "2");
    }
}

mod complex_expressions {

    use crate::diff;

    #[test]
    fn test_polynomial() {
        let result = diff("x^3 + 2*x^2 + 3*x + 4", "x", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_product_chain() {
        let result = diff("x*(x*(x+1))", "x", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_div_with_functions() {
        let result = diff("sin(x)/x", "x", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_composition_chain() {
        let result = diff("sin(cos(exp(x)))", "x", None, None);
        assert!(result.is_ok());
    }
}

mod phase2_features {

    use crate::diff;

    #[test]
    fn test_subtraction_basic() {
        let result = diff("x - 5", "x", None, None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_subtraction_vars() {
        let result = diff("x - a", "x", Some(&["a"]), None).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_division_basic() {
        let result = diff("x / 2", "x", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_quotient_rule() {
        let result = diff("x / (x + 1)", "x", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sinh_basic() {
        let result = diff("sinh(x)", "x", None, None).unwrap();
        assert!(result.contains("cosh"));
    }

    #[test]
    fn test_cosh_basic() {
        let result = diff("cosh(x)", "x", None, None).unwrap();
        assert!(result.contains("sinh"));
    }

    #[test]
    fn test_tanh_basic() {
        let result = diff("tanh(x)", "x", None, None).unwrap();
        assert!(result.contains("tanh") || result.contains("sech"));
    }

    #[test]
    fn test_x_to_x_logarithmic() {
        let result = diff("x^x", "x", None, None).unwrap();
        assert!(result.contains("ln"));
    }

    #[test]
    fn test_x_to_variable_exponent() {
        let result = diff("x^(2*x)", "x", None, None).unwrap();
        assert!(result.contains("ln")); // Should use logarithmic diff
    }

    #[test]
    fn test_base_to_x() {
        let result = diff("a^x", "x", Some(&["a"]), None).unwrap();
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
        let result = diff(&formula, "x", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deeply_nested() {
        let mut formula = "x".to_string();
        for _ in 0..10 {
            formula = format!("sin({})", formula);
        }
        let result = diff(&formula, "x", None, None);
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
        let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
        let result = diff(&formula, "x", Some(&var_refs), None);
        assert!(result.is_ok());
    }
}
