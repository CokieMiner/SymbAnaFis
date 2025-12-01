#[cfg(test)]
mod tests {
    use crate::ast::Expr;
    use crate::parse;
    use crate::simplification::simplify;
    use std::collections::HashSet;

    #[test]
    fn debug_root_issue() {
        // x^(3 * 1 / 3) -> x
        let expr_str = "x^(3 * 1 / 3)";
        println!("Parsing: {}", expr_str);

        let expr = parse(expr_str, &HashSet::new(), &HashSet::new()).unwrap();
        println!("AST: {:?}", expr);

        // We can also check the exponent specifically
        if let Expr::Pow(_, exp) = &expr {
            println!("Exponent AST: {:?}", exp);
            let simplified_exp = simplify(*exp.clone());
            println!("Simplified Exponent: {:?}", simplified_exp);
            println!("Simplified Exponent Display: {}", simplified_exp);
        }

        let result = simplify(expr);
        println!("Full Simplified Display: {}", result);
        assert_eq!(format!("{}", result), "x");

        // Test sqrt(x^2)
        let expr_sqrt = parse("sqrt(x^2)", &HashSet::new(), &HashSet::new()).unwrap();
        println!("AST sqrt: {:?}", expr_sqrt);
        let result_sqrt = simplify(expr_sqrt);
        println!("Result sqrt: {}", result_sqrt);
        assert_eq!(format!("{}", result_sqrt), "x");

        // Test cbrt(x^3)
        let expr_cbrt = parse("cbrt(x^3)", &HashSet::new(), &HashSet::new()).unwrap();
        println!("AST cbrt: {:?}", expr_cbrt);
        let result_cbrt = simplify(expr_cbrt);
        println!("Result cbrt: {}", result_cbrt);
        assert_eq!(format!("{}", result_cbrt), "x");
    }
}
