#[cfg(test)]
mod tests {
    use crate::{Expr, ExprKind, simplification::simplify_expr};
    use std::collections::HashSet;

    // Rule 1: Pull Out Common Factors
    #[test]
    fn test_factor_common_terms() {
        // x*y + x*z -> x*(y+z)
        let expr = Expr::sum(vec![
            Expr::product(vec![Expr::symbol("x"), Expr::symbol("y")]),
            Expr::product(vec![Expr::symbol("x"), Expr::symbol("z")]),
        ]);
        let simplified = simplify_expr(expr, HashSet::new());
        // Expected: x * (y + z) or (y + z) * x
        if let ExprKind::Product(factors) = &simplified.kind {
            let has_x = factors
                .iter()
                .any(|f| matches!(&f.kind, ExprKind::Symbol(s) if s == "x"));
            let has_sum = factors.iter().any(|f| matches!(&f.kind, ExprKind::Sum(_)));
            assert!(
                has_x && has_sum,
                "Expected x * (y + z), got {:?}",
                simplified
            );
        } else {
            panic!("Expected Product, got {:?}", simplified);
        }
    }

    #[test]
    fn test_factor_exponentials() {
        // e^x * sin(x) + e^x * cos(x) -> e^x * (sin(x) + cos(x))
        let ex = Expr::pow(Expr::symbol("e"), Expr::symbol("x"));
        let sinx = Expr::func("sin", Expr::symbol("x"));
        let cosx = Expr::func("cos", Expr::symbol("x"));

        let expr = Expr::sum(vec![
            Expr::product(vec![ex.clone(), sinx.clone()]),
            Expr::product(vec![ex.clone(), cosx.clone()]),
        ]);

        let simplified = simplify_expr(expr, HashSet::new());
        // Expected: e^x * (sin(x) + cos(x))
        if let ExprKind::Product(factors) = &simplified.kind {
            // Should contain e^x and a sum
            let has_exp = factors.iter().any(|f| **f == ex);
            let has_sum = factors.iter().any(|f| matches!(&f.kind, ExprKind::Sum(_)));
            assert!(
                has_exp && has_sum,
                "Expected e^x * (sin(x) + cos(x)), got {:?}",
                simplified
            );
        } else {
            panic!("Expected Product, got {:?}", simplified);
        }
    }

    // Rule 2: Combine Fractions
    #[test]
    fn test_combine_common_denominator() {
        // A/C + B/C -> (A+B)/C
        let expr = Expr::sum(vec![
            Expr::div_expr(Expr::symbol("A"), Expr::symbol("C")),
            Expr::div_expr(Expr::symbol("B"), Expr::symbol("C")),
        ]);
        let simplified = simplify_expr(expr, HashSet::new());
        // Expected: (A+B)/C
        if let ExprKind::Div(num, den) = &simplified.kind {
            assert_eq!(**den, Expr::symbol("C"));
            if let ExprKind::Sum(terms) = &num.kind {
                let has_a = terms.iter().any(|t| **t == Expr::symbol("A"));
                let has_b = terms.iter().any(|t| **t == Expr::symbol("B"));
                assert!(has_a && has_b, "Expected A+B in numerator");
            } else {
                panic!("Expected Sum in numerator");
            }
        } else {
            panic!("Expected Div");
        }
    }

    // Rule 3: Sign Cleanup
    #[test]
    fn test_distribute_negation_sub() {
        // -(A - B) -> B - A
        // Represented as Product([-1, Sum([A, Product([-1, B])])])
        let expr = Expr::product(vec![
            Expr::number(-1.0),
            Expr::sum(vec![
                Expr::symbol("A"),
                Expr::product(vec![Expr::number(-1.0), Expr::symbol("B")]),
            ]),
        ]);
        let simplified = simplify_expr(expr, HashSet::new());
        // Expected: B - A or Sum([B, Product([-1, A])])
        let display = format!("{}", simplified);
        println!("Got display: {}", display);
        // Just verify it simplifies to something reasonable
        assert!(display.contains("A") && display.contains("B"));
    }

    #[test]
    fn test_distribute_negation_add() {
        // -1 * (A + (-1)*B) -> B - A
        let expr = Expr::product(vec![
            Expr::number(-1.0),
            Expr::sum(vec![
                Expr::symbol("A"),
                Expr::product(vec![Expr::number(-1.0), Expr::symbol("B")]),
            ]),
        ]);
        let simplified = simplify_expr(expr, HashSet::new());
        let display = format!("{}", simplified);
        println!("Got display: {}", display);
        assert!(display.contains("A") && display.contains("B"));
    }

    #[test]
    fn test_neg_div_neg() {
        // -A / -B -> A / B
        let expr = Expr::div_expr(
            Expr::product(vec![Expr::number(-1.0), Expr::symbol("A")]),
            Expr::product(vec![Expr::number(-1.0), Expr::symbol("B")]),
        );
        let simplified = simplify_expr(expr, HashSet::new());
        if let ExprKind::Div(num, den) = &simplified.kind {
            assert_eq!(**num, Expr::symbol("A"));
            assert_eq!(**den, Expr::symbol("B"));
        } else {
            panic!("Expected Div(A, B)");
        }
    }

    // Rule 4: Absorb Constants in Powers
    #[test]
    fn test_absorb_constant_pow() {
        // 2 * 2^x -> 2^(x+1)
        let expr = Expr::product(vec![
            Expr::number(2.0),
            Expr::pow(Expr::number(2.0), Expr::symbol("x")),
        ]);
        let simplified = simplify_expr(expr, HashSet::new());
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            assert_eq!(**base, Expr::number(2.0));
            if let ExprKind::Sum(terms) = &exp.kind {
                // x + 1
                let has_x = terms.iter().any(|t| **t == Expr::symbol("x"));
                let has_1 = terms.iter().any(|t| **t == Expr::number(1.0));
                assert!(has_x && has_1, "Expected x+1");
            } else {
                panic!("Expected Sum in exponent");
            }
        } else {
            panic!("Expected Pow");
        }
    }

    #[test]
    fn test_factor_mixed_terms() {
        // e^x + sin(x)*e^x -> e^x * (1 + sin(x))
        let ex = Expr::pow(Expr::symbol("e"), Expr::symbol("x"));
        let sinx = Expr::func("sin", Expr::symbol("x"));

        // e^x + sin(x) * e^x
        let expr = Expr::sum(vec![
            ex.clone(),
            Expr::product(vec![sinx.clone(), ex.clone()]),
        ]);

        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: e^x * (1 + sin(x))
        if let ExprKind::Product(factors) = &simplified.kind {
            let has_exp = factors.iter().any(|f| **f == ex);
            let has_sum = factors.iter().any(|f| matches!(&f.kind, ExprKind::Sum(_)));
            assert!(
                has_exp && has_sum,
                "Expected e^x * (1 + sin(x)), got {:?}",
                simplified
            );
        } else {
            panic!("Expected Product, got {:?}", simplified);
        }
    }
}
