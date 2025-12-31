use crate::core::expr::Expr;
use crate::symb;

#[test]
fn test_log_simplification_generic() {
    // log(b, 1) = 0
    let b = symb("b");

    let expr = Expr::number(1.0).log(b);
    // simplification should produce 0
    // simplify expects string? No, simplify function takes formula string.
    // Or I can use builder on expression.

    let simplified = crate::Simplify::new().simplify(&expr).unwrap();
    assert_eq!(simplified.as_number(), Some(0.0));

    // log(b, b) = 1
    let expr = b.log(b); // log(b, b)
    let simplified = crate::Simplify::new().simplify(&expr).unwrap();
    assert_eq!(simplified.as_number(), Some(1.0));
}

#[test]
fn test_log_power_rule_generic() {
    let x = symb("x");
    let b = symb("b");

    // log(b, x^2) -> 2 * log(b, abs(x)) (even power)
    let expr = x.pow(2.0).log(b); // log(b, x^2)
    let simplified = crate::Simplify::new().simplify(&expr).unwrap();
    // Expected: 2 * log(b, abs(x))
    // Structure: Product(2.0, FunctionCall(log, [b, FunctionCall(abs, [x])]))

    // Check string representation
    let s = format!("{}", simplified);
    assert!(s.contains("2") && s.contains("log") && s.contains("abs"));
    // exact string might vary by parentheses, e.g. "2*log(b, abs(x))"
}

#[test]
fn test_log_power_rule_generic_odd() {
    let x = symb("x");
    let b = symb("b");

    // log(b, x^3) -> 3 * log(b, x) (odd power) using domain_safe=false (default)
    let expr = x.pow(3.0).log(b);
    let simplified = crate::Simplify::new().simplify(&expr).unwrap();
    let s = format!("{}", simplified);
    assert!(s.contains("3*log"));
}

#[test]
fn test_log_sqrt_generic() {
    let x = symb("x");
    let b = symb("b");

    // log(b, sqrt(x)) -> 0.5 * log(b, x)
    let expr = x.sqrt().log(b);
    let simplified = crate::Simplify::new().simplify(&expr).unwrap();
    let s = format!("{}", simplified);
    println!("{}", s);
    assert!(s.contains("log(b, x)/2"));
}
