#[test]
fn debug_rc_derivative() {
    use crate::{Expr, ExprKind, simplification::simplify_expr};
    use std::collections::HashSet;
    use std::sync::Arc;
    // Simplified RC test
    let rc = Expr::new(ExprKind::Mul(
        Arc::new(Expr::symbol("V0")),
        Arc::new(Expr::new(ExprKind::FunctionCall {
            name: "exp".to_string(),
            args: vec![Expr::new(ExprKind::Div(
                Arc::new(Expr::new(ExprKind::Mul(
                    Arc::new(Expr::number(-1.0)),
                    Arc::new(Expr::symbol("t")),
                ))),
                Arc::new(Expr::new(ExprKind::Mul(
                    Arc::new(Expr::symbol("R")),
                    Arc::new(Expr::symbol("C")),
                ))),
            ))],
        })),
    ));

    eprintln!("===== RC CIRCUIT DERIVATIVE TEST =====");
    eprintln!("Original: {}", rc);

    let mut fixed = HashSet::new();
    fixed.insert("R".to_string());
    fixed.insert("C".to_string());
    fixed.insert("V0".to_string());

    let deriv = rc.derive(
        "t",
        &fixed,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
    );
    eprintln!("Raw derivative: {}", deriv);

    let simplified = simplify_expr(deriv, fixed);
    eprintln!("Simplified: {}", simplified);
    eprintln!("Simplified Debug: {:#?}", simplified);
    eprintln!("Expected: -V0 * exp(-t / (R * C)) / (R * C)");

    let s = format!("{}", simplified);
    assert!(!s.contains("/ C * R"), "Bug found: {}", s);
}
