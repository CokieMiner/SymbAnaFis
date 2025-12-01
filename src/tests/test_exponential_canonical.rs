#[cfg(test)]
mod tests {
    use crate::simplify as simplify_string;
    use crate::{Expr, parser::parse};
    use std::collections::HashSet;

    // NOTE: The e^a → exp(a) rule has been intentionally removed to avoid
    // conflicts with user-defined variables named 'e' (like eccentricity).
    // Users should write exp(a) directly if they want the exponential function.

    /*
    #[test]
    fn test_e_to_power_becomes_exp() {
        // e^x -> exp(x)
        let result = simplify_string("e^x".to_string(), None, None).unwrap();
        // This test is disabled because we removed the rule
    }
    */

    #[test]
    fn test_exp_power_simplification() {
        // exp(b)^a → exp(a*b) - this rule is still valid
        let result = simplify_string("exp(x)^2".to_string(), None, None).unwrap();

        if let Ok(ast) = parse(&result, &HashSet::new(), &HashSet::new()) {
            match ast {
                Expr::FunctionCall { name, args } => {
                    assert_eq!(name, "exp");
                    assert_eq!(args.len(), 1);
                    // Should be exp(2*x) or exp(x*2)
                    if let Expr::Mul(a, b) = &args[0] {
                        let has_2 = **a == Expr::Number(2.0) || **b == Expr::Number(2.0);
                        let has_x = **a == Expr::Symbol("x".to_string())
                            || **b == Expr::Symbol("x".to_string());
                        assert!(has_2 && has_x, "Expected 2*x");
                    } else {
                        panic!("Expected Mul in exp argument, got {:?}", args[0]);
                    }
                }
                _ => panic!("Expected FunctionCall, got {:?}", ast),
            }
        } else {
            panic!("Failed to parse simplified expression: {}", result);
        }
    }

    /*
    #[test]
    fn test_combined_exponential_rules() {
        // e^x * e^y should become exp(x) * exp(y) -> exp(x+y)
        // This test relied on e^x -> exp(x) conversion which is removed
    }
    */
}
