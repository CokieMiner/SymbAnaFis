#[cfg(test)]
mod tests {
    use crate::simplify as simplify_string;
    use crate::{Expr, ExprKind, parser::parse};
    use std::collections::HashSet;

    // NOTE: The e^a → exp(a) rule has been intentionally removed to avoid
    // conflicts with user-defined variables named 'e' (like eccentricity).
    // Users should write exp(a) directly if they want the exponential function.

    #[test]
    fn test_exp_power_simplification() {
        // exp(b)^a → exp(a*b) - this rule is still valid
        let result = simplify_string("exp(x)^2", None, None).unwrap();

        if let Ok(ast) = parse(&result, &HashSet::new(), &HashSet::new(), None) {
            match ast.kind {
                ExprKind::FunctionCall { name, args } => {
                    assert_eq!(name, "exp");
                    assert_eq!(args.len(), 1);
                    // Should be exp(2*x) or exp(Product([2, x]))
                    if let ExprKind::Product(factors) = &args[0].kind {
                        let has_2 = factors.iter().any(|f| **f == Expr::number(2.0));
                        let has_x = factors.iter().any(|f| **f == Expr::symbol("x"));
                        assert!(has_2 && has_x, "Expected 2*x");
                    } else {
                        panic!("Expected Product in exp argument, got {:?}", args[0]);
                    }
                }
                _ => panic!("Expected FunctionCall, got {:?}", ast),
            }
        } else {
            panic!("Failed to parse simplified expression: {}", result);
        }
    }
}
