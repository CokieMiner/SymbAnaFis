use crate::simplify;

#[test]
fn test_combine_like_terms_simple() {
    println!("Starting test_combine_like_terms_simple");
    // 3x + x = 4x
    let expr = "3*x + x";
    println!("Simplifying: {}", expr);
    let simplified = simplify(expr.to_string(), None, None).unwrap();
    println!("Result: {}", simplified);
    assert_eq!(simplified, "4 * x");

    // x + 3x = 4x
    let expr = "x + 3*x";
    println!("Simplifying: {}", expr);
    let simplified = simplify(expr.to_string(), None, None).unwrap();
    println!("Result: {}", simplified);
    assert_eq!(simplified, "4 * x");
}

#[test]
fn test_combine_like_terms_subtraction() {
    // 3x - x = 2x
    let expr = "3*x - x";
    let simplified = simplify(expr.to_string(), None, None).unwrap();
    assert_eq!(simplified, "2 * x");

    // x - 3x = -2x
    let expr = "x - 3*x";
    let simplified = simplify(expr.to_string(), None, None).unwrap();
    assert_eq!(simplified, "-2 * x");
}
