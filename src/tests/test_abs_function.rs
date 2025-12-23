use crate::parser::parse;
use crate::simplify;
use std::collections::HashSet;

#[test]
fn test_abs_function_simplification() {
    let expr = parse("abs(sigma)", &HashSet::new(), &HashSet::new(), None).unwrap();
    println!("Parsed expr: {:?}", expr);

    let expr_str = expr.to_string();
    let simplified = simplify(&expr_str, None, None).unwrap();
    println!("Simplified expr: {}", simplified);

    // Check that abs(sigma) stays as a function call
    assert!(
        simplified.contains("abs(sigma)"),
        "Expected 'abs(sigma)', got '{}'",
        simplified
    );
}

#[test]
fn test_abs_in_product() {
    let expr = parse(
        "sqrt(2) * (mu - x) * abs(sigma) / (2 * sigma^4)",
        &HashSet::new(),
        &HashSet::new(),
        None,
    )
    .unwrap();
    println!("Parsed expr: {:?}", expr);

    let expr_str = expr.to_string();
    let simplified = simplify(&expr_str, None, None).unwrap();
    println!("Simplified expr: {}", simplified);

    // Check that it simplifies correctly - accept equivalent orderings
    // (-x + mu) is same as (mu - x)
    assert!(
        simplified.contains("sqrt(2)")
            && (simplified.contains("mu - x") || simplified.contains("-x + mu")),
        "Expected simplification to contain 'sqrt(2)' and 'mu - x' or '-x + mu', got '{}'",
        simplified
    );
}
