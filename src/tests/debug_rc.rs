#[test]
fn debug_rc_derivative() {
    use crate::{Expr, core::Context, simplification::simplify_expr};
    use std::collections::{HashMap, HashSet};

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

    let known_symbols = HashSet::new();

    // Use with_symbols for known symbols (or just use Context::new() if none needed)
    let context = Context::new()
        .with_symbol("R")
        .with_symbol("C")
        .with_symbol("V0");
    let deriv = rc.derive("t", Some(&context));
    eprintln!("Raw derivative: {}", deriv);

    let simplified = simplify_expr(
        deriv,
        known_symbols,
        HashMap::new(),
        None,
        None,
        None,
        false,
    );
    eprintln!("Simplified: {}", simplified);
    eprintln!("Simplified Debug: {:#?}", simplified);
    eprintln!("Expected: -V0 * exp(-t / (R * C)) / (R * C)");

    let s = format!("{}", simplified);
    assert!(!s.contains("/ C * R"), "Bug found: {}", s);
}
