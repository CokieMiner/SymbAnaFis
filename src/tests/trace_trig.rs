#[cfg(test)]
mod tests {
    use crate::{Expr, ExprKind, simplification::simplify_expr};
    use std::collections::HashSet;
    use std::f64::consts::PI;
    use std::sync::Arc;

    #[test]
    fn test_trig_reflection_shifts() {
        // sin(pi - x) = sin(x)
        let expr = Expr::new(ExprKind::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::new(ExprKind::Sub(
                Arc::new(Expr::number(PI)),
                Arc::new(Expr::symbol("x")),
            ))],
        });
        let simplified = simplify_expr(expr, HashSet::new());
        println!("sin(pi - x) -> {}", simplified);
        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name, "sin");
            assert_eq!(args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sin(x), got {:?}", simplified);
        }

        // cos(pi + x) = -cos(x)
        let expr = Expr::new(ExprKind::FunctionCall {
            name: "cos".to_string(),
            args: vec![Expr::new(ExprKind::Add(
                Arc::new(Expr::number(PI)),
                Arc::new(Expr::symbol("x")),
            ))],
        });
        let simplified = simplify_expr(expr, HashSet::new());
        println!("cos(pi + x) -> {}", simplified);
        if let ExprKind::Mul(a, b) = &simplified.kind {
            assert_eq!(**a, Expr::number(-1.0));
            if let ExprKind::FunctionCall { name, args } = &b.kind {
                assert_eq!(name, "cos");
                assert_eq!(args[0], Expr::symbol("x"));
            } else {
                panic!("Expected cos(x), got {:?}", simplified);
            }
        } else {
            panic!("Expected -cos(x), got {:?}", simplified);
        }

        // sin(3pi/2 - x) = -cos(x)
        let expr = Expr::new(ExprKind::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::new(ExprKind::Sub(
                Arc::new(Expr::number(3.0 * PI / 2.0)),
                Arc::new(Expr::symbol("x")),
            ))],
        });
        let simplified = simplify_expr(expr, HashSet::new());
        println!("sin(3pi/2 - x) -> {}", simplified);
        if let ExprKind::Mul(a, b) = &simplified.kind {
            assert_eq!(**a, Expr::number(-1.0));
            if let ExprKind::FunctionCall { name, args } = &b.kind {
                assert_eq!(name, "cos");
                assert_eq!(args[0], Expr::symbol("x"));
            } else {
                panic!("Expected cos(x), got {:?}", simplified);
            }
        } else {
            panic!("Expected -cos(x), got {:?}", simplified);
        }
    }
}
