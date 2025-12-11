#[cfg(test)]
mod tests {
    use crate::{Expr, ExprKind, simplification::simplify_expr};
    use std::collections::HashSet;
    use std::sync::Arc;

    // Rule 1: Pull Out Common Factors
    #[test]
    fn test_factor_common_terms() {
        // x*y + x*z -> x*(y+z)
        let expr = Expr::new(ExprKind::Add(
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::symbol("y")),
            ))),
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::symbol("x")),
                Arc::new(Expr::symbol("z")),
            ))),
        ));
        let simplified = simplify_expr(expr, HashSet::new());
        // Expected: x * (y + z) or (y + z) * x (both are valid after canonicalization)
        // Note: The order of factors and terms might depend on canonicalization
        if let ExprKind::Mul(a, b) = &simplified.kind {
            // Check if it's x * (y+z) or (y+z) * x
            let (x_part, sum_part) = if matches!(a.kind, ExprKind::Symbol(ref s) if s == "x") {
                (a, b)
            } else if matches!(b.kind, ExprKind::Symbol(ref s) if s == "x") {
                (b, a)
            } else {
                panic!("Expected one factor to be x, got {:?}", (a, b));
            };

            assert_eq!(**x_part, Expr::symbol("x"));
            if let ExprKind::Add(c, d) = &sum_part.kind {
                let is_yz = **c == Expr::symbol("y") && **d == Expr::symbol("z");
                let is_zy = **c == Expr::symbol("z") && **d == Expr::symbol("y");
                assert!(
                    is_yz || is_zy,
                    "Expected (y+z) or (z+y), got {:?}",
                    sum_part
                );
            } else {
                panic!("Expected Add inside Mul, got {:?}", sum_part);
            }
        } else {
            panic!("Expected Mul, got {:?}", simplified);
        }
    }

    #[test]
    fn test_factor_exponentials() {
        // e^x * sin(x) + e^x * cos(x) -> e^x * (sin(x) + cos(x))
        // Note: exp(x) gets converted to e^x during simplification
        let ex = Expr::new(ExprKind::Pow(
            Arc::new(Expr::symbol("e")),
            Arc::new(Expr::symbol("x")),
        ));
        let sinx = Expr::new(ExprKind::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::symbol("x")],
        });
        let cosx = Expr::new(ExprKind::FunctionCall {
            name: "cos".to_string(),
            args: vec![Expr::symbol("x")],
        });

        let expr = Expr::new(ExprKind::Add(
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(ex.clone()),
                Arc::new(sinx.clone()),
            ))),
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(ex.clone()),
                Arc::new(cosx.clone()),
            ))),
        ));

        let simplified = simplify_expr(expr, HashSet::new());
        // Expected: e^x * (sin(x) + cos(x)) or (sin(x) + cos(x)) * e^x
        if let ExprKind::Mul(a, b) = &simplified.kind {
            let (exp_part, sum_part) = if **a == ex {
                (a, b)
            } else if **b == ex {
                (b, a)
            } else {
                panic!("Expected e^x as a factor, got Mul({:?}, {:?})", a, b);
            };

            assert_eq!(**exp_part, ex);
            if let ExprKind::Add(c, d) = &sum_part.kind {
                // Check for sin(x) + cos(x)
                let has_sin = c.as_ref() == &sinx || d.as_ref() == &sinx;
                let has_cos = c.as_ref() == &cosx || d.as_ref() == &cosx;
                assert!(has_sin && has_cos, "Expected sin(x) + cos(x)");
            } else {
                panic!("Expected Add inside Mul");
            }
        } else {
            panic!("Expected Mul, got {:?}", simplified);
        }
    }

    // Rule 2: Combine Fractions
    #[test]
    fn test_combine_common_denominator() {
        // A/C + B/C -> (A+B)/C
        let expr = Expr::new(ExprKind::Add(
            Arc::new(Expr::new(ExprKind::Div(
                Arc::new(Expr::symbol("A")),
                Arc::new(Expr::symbol("C")),
            ))),
            Arc::new(Expr::new(ExprKind::Div(
                Arc::new(Expr::symbol("B")),
                Arc::new(Expr::symbol("C")),
            ))),
        ));
        let simplified = simplify_expr(expr, HashSet::new());
        // Expected: (A+B)/C
        if let ExprKind::Div(num, den) = &simplified.kind {
            assert_eq!(**den, Expr::symbol("C"));
            if let ExprKind::Add(a, b) = &num.kind {
                let is_ab = a.as_ref() == &Expr::symbol("A") && b.as_ref() == &Expr::symbol("B");
                let is_ba = a.as_ref() == &Expr::symbol("B") && b.as_ref() == &Expr::symbol("A");
                assert!(is_ab || is_ba, "Expected A+B");
            } else {
                panic!("Expected Add in numerator");
            }
        } else {
            panic!("Expected Div");
        }
    }

    // Rule 3: Sign Cleanup
    #[test]
    fn test_distribute_negation_sub() {
        // -(A - B) -> B - A
        // Represented as -1 * (A - B)
        let expr = Expr::new(ExprKind::Mul(
            Arc::new(Expr::number(-1.0)),
            Arc::new(Expr::new(ExprKind::Sub(
                Arc::new(Expr::symbol("A")),
                Arc::new(Expr::symbol("B")),
            ))),
        ));
        let simplified = simplify_expr(expr, HashSet::new());
        // Expected: B - A or B + (-A), which should display as B - A
        let display = format!("{}", simplified);
        assert!(
            display == "B - A" || display == "B + (-1) * A",
            "Got display: {}",
            display
        );
    }

    #[test]
    fn test_distribute_negation_add() {
        // -1 * (A + (-1)*B) -> B - A
        let expr = Expr::new(ExprKind::Mul(
            Arc::new(Expr::number(-1.0)),
            Arc::new(Expr::new(ExprKind::Add(
                Arc::new(Expr::symbol("A")),
                Arc::new(Expr::new(ExprKind::Mul(
                    Arc::new(Expr::number(-1.0)),
                    Arc::new(Expr::symbol("B")),
                ))),
            ))),
        ));
        let simplified = simplify_expr(expr, HashSet::new());
        let display = format!("{}", simplified);
        assert!(
            display == "B - A" || display == "B + (-1) * A",
            "Got display: {}",
            display
        );
    }

    #[test]
    fn test_neg_div_neg() {
        // -A / -B -> A / B
        let expr = Expr::new(ExprKind::Div(
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::number(-1.0)),
                Arc::new(Expr::symbol("A")),
            ))),
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(Expr::number(-1.0)),
                Arc::new(Expr::symbol("B")),
            ))),
        ));
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
        let expr = Expr::new(ExprKind::Mul(
            Arc::new(Expr::number(2.0)),
            Arc::new(Expr::new(ExprKind::Pow(
                Arc::new(Expr::number(2.0)),
                Arc::new(Expr::symbol("x")),
            ))),
        ));
        let simplified = simplify_expr(expr, HashSet::new());
        if let ExprKind::Pow(base, exp) = &simplified.kind {
            assert_eq!(**base, Expr::number(2.0));
            if let ExprKind::Add(a, b) = &exp.kind {
                // x + 1
                let has_x = a.as_ref() == &Expr::symbol("x") || b.as_ref() == &Expr::symbol("x");
                let has_1 = a.as_ref() == &Expr::number(1.0) || b.as_ref() == &Expr::number(1.0);
                assert!(has_x && has_1, "Expected x+1");
            } else {
                panic!("Expected Add in exponent");
            }
        } else {
            panic!("Expected Pow");
        }
    }
    #[test]
    fn test_factor_mixed_terms() {
        // e^x + sin(x)*e^x -> e^x * (1 + sin(x))
        // Note: exp(x) gets converted to e^x during simplification
        let ex = Expr::new(ExprKind::Pow(
            Arc::new(Expr::symbol("e")),
            Arc::new(Expr::symbol("x")),
        ));
        let sinx = Expr::new(ExprKind::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::symbol("x")],
        });

        // e^x + sin(x) * e^x
        let expr = Expr::new(ExprKind::Add(
            Arc::new(ex.clone()),
            Arc::new(Expr::new(ExprKind::Mul(
                Arc::new(sinx.clone()),
                Arc::new(ex.clone()),
            ))),
        ));

        let simplified = simplify_expr(expr, HashSet::new());

        // Expected: e^x * (1 + sin(x)) or (1 + sin(x)) * e^x
        if let ExprKind::Mul(a, b) = &simplified.kind {
            // One part should be e^x
            let (factor, other) = if **a == ex {
                (a, b)
            } else if **b == ex {
                (b, a)
            } else {
                panic!("Expected e^x as a factor");
            };

            assert_eq!(**factor, ex);

            // The other part should be 1 + sin(x)
            if let ExprKind::Add(u, v) = &other.kind {
                let has_1 = u.as_ref() == &Expr::number(1.0) || v.as_ref() == &Expr::number(1.0);
                let has_sin = u.as_ref() == &sinx || v.as_ref() == &sinx;
                assert!(has_1 && has_sin, "Expected 1 + sin(x)");
            } else {
                panic!("Expected Add(1, sin(x)) in other factor");
            }
        } else {
            panic!("Expected Mul, got {:?}", simplified);
        }
    }
}
