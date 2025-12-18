#[test]
fn test_division_structure() {
    use crate::Expr;
    // Debug: What structure does the derivative create?
    // Manual construction of: something / (C * R^2)
    let proper = Expr::div_expr(
        Expr::symbol("X"),
        Expr::product(vec![
            Expr::symbol("C"),
            Expr::pow(Expr::symbol("R"), Expr::number(2.0)),
        ]),
    );
    eprintln!("Proper structure: {}", proper);
    eprintln!("  Debug: {:?}", proper);

    // What if it's: (something / C) * R^2 ?
    let wrong = Expr::product(vec![
        Expr::div_expr(Expr::symbol("X"), Expr::symbol("C")),
        Expr::pow(Expr::symbol("R"), Expr::number(2.0)),
    ]);
    eprintln!("Wrong structure: {}", wrong);
    eprintln!("  Debug: {:?}", wrong);

    // What about: something / R * C^2 (parsed as (something/R)*C^2)?
    let ambiguous = Expr::product(vec![
        Expr::div_expr(Expr::symbol("X"), Expr::symbol("R")),
        Expr::pow(Expr::symbol("C"), Expr::number(2.0)),
    ]);
    eprintln!("Ambiguous structure: {}", ambiguous);
    eprintln!("  Debug: {:?}", ambiguous);
}
