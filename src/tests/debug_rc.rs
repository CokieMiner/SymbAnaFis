#[test]
fn debug_rc_derivative() {
    use crate::{Expr, simplification::simplify};
    use std::collections::HashSet;

    // Simplified RC test
    let rc = Expr::Mul(
        Box::new(Expr::Symbol("V0".to_string())),
        Box::new(Expr::FunctionCall {
            name: "exp".to_string(),
            args: vec![Expr::Div(
                Box::new(Expr::Mul(
                    Box::new(Expr::Number(-1.0)),
                    Box::new(Expr::Symbol("t".to_string())),
                )),
                Box::new(Expr::Mul(
                    Box::new(Expr::Symbol("R".to_string())),
                    Box::new(Expr::Symbol("C".to_string())),
                )),
            )],
        }),
    );

    eprintln!("===== RC CIRCUIT DERIVATIVE TEST =====");
    eprintln!("Original: {}", rc);

    let mut fixed = HashSet::new();
    fixed.insert("R".to_string());
    fixed.insert("C".to_string());
    fixed.insert("V0".to_string());

    let deriv = rc.derive("t", &fixed);
    eprintln!("Raw derivative: {}", deriv);

    let simplified = simplify(deriv);
    eprintln!("Simplified: {}", simplified);
    eprintln!("Simplified Debug: {:#?}", simplified);
    eprintln!("Expected: -V0 * exp(-t / (R * C)) / (R * C)");

    let s = format!("{}", simplified);
    assert!(!s.contains("/ C * R"), "Bug found: {}", s);
}
