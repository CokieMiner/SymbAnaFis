#[test]
fn debug_rc_derivative() {
    use crate::{Expr, simplification::simplify_expr};
    use std::collections::HashSet;

    // Simplified RC test using n-ary Product
    let rc = Expr::product(vec![
        Expr::symbol("V0"),
        Expr::func(
            "exp",
            Expr::div_expr(
                Expr::product(vec![Expr::number(-1.0), Expr::symbol("t")]),
                Expr::product(vec![Expr::symbol("R"), Expr::symbol("C")]),
            ),
        ),
    ]);

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
