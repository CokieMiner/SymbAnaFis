#[cfg(test)]
mod tests {
    use crate::core::known_symbols::KS;
    use crate::{Expr, ExprKind, simplification::simplify_expr};
    use std::collections::{HashMap, HashSet};
    use std::f64::consts::PI;

    #[test]
    fn test_trig_reflection_shifts() {
        // sin(pi - x) = sin(x) represented as Sum([pi, Product([-1, x])])
        let expr = Expr::func(
            "sin",
            Expr::sum(vec![
                Expr::number(PI),
                Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
            ]),
        );
        let simplified = simplify_expr(
            expr,
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        println!("sin(pi - x) -> {}", simplified);
        if let ExprKind::FunctionCall { name, args } = &simplified.kind {
            assert_eq!(name.as_str(), "sin");
            assert_eq!(*args[0], Expr::symbol("x"));
        } else {
            panic!("Expected sin(x), got {:?}", simplified);
        }

        // cos(pi + x) = -cos(x)
        let expr = Expr::func("cos", Expr::sum(vec![Expr::number(PI), Expr::symbol("x")]));
        let simplified = simplify_expr(
            expr,
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        println!("cos(pi + x) -> {}", simplified);
        if let ExprKind::Product(factors) = &simplified.kind {
            assert!(factors.len() == 2);
            assert!(factors.iter().any(|f| **f == Expr::number(-1.0)));
            let has_cos = factors.iter().any(|f| {
                if let ExprKind::FunctionCall { name, args } = &f.kind {
                    name.id() == KS.cos && *args[0] == Expr::symbol("x")
                } else {
                    false
                }
            });
            assert!(has_cos, "Expected cos(x), got {:?}", simplified);
        } else {
            panic!("Expected -cos(x), got {:?}", simplified);
        }

        // sin(3pi/2 - x) = -cos(x)
        let expr = Expr::func(
            "sin",
            Expr::sum(vec![
                Expr::number(3.0 * PI / 2.0),
                Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]),
            ]),
        );
        let simplified = simplify_expr(
            expr,
            HashSet::new(),
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        println!("sin(3pi/2 - x) -> {}", simplified);
        if let ExprKind::Product(factors) = &simplified.kind {
            assert!(factors.iter().any(|f| **f == Expr::number(-1.0)));
            let has_cos = factors.iter().any(|f| {
                if let ExprKind::FunctionCall { name, args } = &f.kind {
                    name.id() == KS.cos && *args[0] == Expr::symbol("x")
                } else {
                    false
                }
            });
            assert!(has_cos, "Expected cos(x), got {:?}", simplified);
        } else {
            panic!("Expected -cos(x), got {:?}", simplified);
        }
    }
}
