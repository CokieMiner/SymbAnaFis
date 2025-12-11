#[test]
fn test_division_structure() {
    use crate::{Expr, ExprKind};
    use std::sync::Arc;
    // Debug: What structure does the derivative create?
    // Manual construction of: something / (C * R^2)
    let proper = Expr::new(ExprKind::Div(
        Arc::new(Expr::symbol("X")),
        Arc::new(Expr::new(ExprKind::Mul(
            Arc::new(Expr::symbol("C")),
            Arc::new(Expr::new(ExprKind::Pow(
                Arc::new(Expr::symbol("R")),
                Arc::new(Expr::number(2.0)),
            ))),
        ))),
    ));
    eprintln!("Proper structure: {}", proper);
    eprintln!("  Debug: {:?}", proper);

    // What if it's: (something / C) * R^2 ?
    let wrong = Expr::new(ExprKind::Mul(
        Arc::new(Expr::new(ExprKind::Div(
            Arc::new(Expr::symbol("X")),
            Arc::new(Expr::symbol("C")),
        ))),
        Arc::new(Expr::new(ExprKind::Pow(
            Arc::new(Expr::symbol("R")),
            Arc::new(Expr::number(2.0)),
        ))),
    ));
    eprintln!("Wrong structure: {}", wrong);
    eprintln!("  Debug: {:?}", wrong);

    // What about: something / R * C^2 (parsed as (something/R)*C^2)?
    let ambiguous = Expr::new(ExprKind::Mul(
        Arc::new(Expr::new(ExprKind::Div(
            Arc::new(Expr::symbol("X")),
            Arc::new(Expr::symbol("R")),
        ))),
        Arc::new(Expr::new(ExprKind::Pow(
            Arc::new(Expr::symbol("C")),
            Arc::new(Expr::number(2.0)),
        ))),
    ));
    eprintln!("Ambiguous structure: {}", ambiguous);
    eprintln!("  Debug: {:?}", ambiguous);
}
