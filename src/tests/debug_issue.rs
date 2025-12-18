#[cfg(test)]
mod tests {
    use crate::simplify;

    #[test]
    fn debug_numerator_simplification() {
        // The numerator from d/dx [(x^2 + 1) / (x + 1)]
        // Current: -x^2 + 2x(x + 1) - 1
        // Should be: x^2 + 2x - 1 (after expanding 2x(x+1) = 2x^2 + 2x)

        let expr = "-x^2 + 2*x*(x + 1) - 1";
        let result = simplify(expr, None, None).unwrap();
        println!("Input: {}", expr);
        println!("Result: {}", result);

        // Expanded form
        let expanded = "-x^2 + 2*x^2 + 2*x - 1";
        let result2 = simplify(expanded, None, None).unwrap();
        println!("\nExpanded input: {}", expanded);
        println!("Result: {}", result2);

        // Target form
        let target = "x^2 + 2*x - 1";
        let result3 = simplify(target, None, None).unwrap();
        println!("\nTarget: {}", target);
        println!("Result: {}", result3);
    }

    #[test]
    fn debug_cubic_expansion() {
        use crate::parse;
        use crate::simplification::simplify_expr;
        use std::collections::HashSet;

        // Test (x+1)^3 expansion: x^3 + 3*x^2 + 3*x + 1
        let expr_str = "x^3 + 3*x^2 + 3*x + 1";
        let empty: HashSet<String> = HashSet::new();
        let parsed = parse(expr_str, &empty, &empty, None).unwrap();
        println!("Input: {}", expr_str);
        println!("Parsed AST: {:?}", parsed);

        // Direct simplification with trace
        let result = simplify_expr(parsed, empty);
        println!("Result AST: {:?}", result);
        println!("Result: {}", result);
    }
}
