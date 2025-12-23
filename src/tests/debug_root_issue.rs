#[cfg(test)]
mod tests {
    use crate::core::expr::ExprKind;
    use crate::parser::parse;
    use crate::simplification::simplify_expr;
    use std::collections::HashSet;

    #[test]
    fn debug_root_issue() {
        // x^(3 * 1 / 3) -> x
        let expr_str = "x^(3 * 1 / 3)";
        println!("Parsing: {}", expr_str);

        let expr = parse(expr_str, &HashSet::new(), &HashSet::new(), None).unwrap();
        println!("AST: {:?}", expr);

        // We can also check the exponent specifically
        if let ExprKind::Pow(_, exp) = &expr.kind {
            println!("Exponent AST: {:?}", exp);
            let simplified_exp = simplify_expr(exp.as_ref().clone(), HashSet::new());
            println!("Simplified Exponent: {:?}", simplified_exp);
            println!("Simplified Exponent Display: {}", simplified_exp);
        }

        let result = simplify_expr(expr, HashSet::new());
        println!("Full Simplified Display: {}", result);
        assert_eq!(format!("{}", result), "x");

        // Test sqrt(x^2) = |x| for all real x
        let expr_sqrt = parse("sqrt(x^2)", &HashSet::new(), &HashSet::new(), None).unwrap();
        println!("AST sqrt: {:?}", expr_sqrt);
        let result_sqrt = simplify_expr(expr_sqrt, HashSet::new());
        println!("Result sqrt: {}", result_sqrt);
        assert_eq!(format!("{}", result_sqrt), "abs(x)");

        // Test cbrt(x^3)
        let expr_cbrt = parse("cbrt(x^3)", &HashSet::new(), &HashSet::new(), None).unwrap();
        println!("AST cbrt: {:?}", expr_cbrt);
        let result_cbrt = simplify_expr(expr_cbrt, HashSet::new());
        println!("Result cbrt: {}", result_cbrt);
        assert_eq!(format!("{}", result_cbrt), "x");
    }
}
