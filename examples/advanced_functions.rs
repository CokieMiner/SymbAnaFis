use symb_anafis::diff;

fn main() {
    println!("SymbAnaFis Advanced Functions Example");
    println!("-------------------------------------");

    // Example 1: Tangent function
    let expr1 = "tan(x)";
    let result1 = diff(expr1.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr1, result1);
    // Expected: sec(x)^2

    // Example 2: Hyperbolic sine
    let expr2 = "sinh(x)";
    let result2 = diff(expr2.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr2, result2);
    // Expected: cosh(x)

    // Example 3: Square root
    let expr3 = "sqrt(x)";
    let result3 = diff(expr3.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr3, result3);
    // Expected: 1 / (2 * sqrt(x))

    // Example 4: Log base 10
    let expr4 = "log10(x)";
    let result4 = diff(expr4.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr4, result4);
    // Expected: 1 / (x * ln(10))

    // Example 5: Error function
    let expr5 = "erf(x)";
    let result5 = diff(expr5.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr5, result5);
    // Expected: (2 / sqrt(pi)) * exp(-x^2)

    // Example 6: Gamma function
    let expr6 = "gamma(x)";
    let result6 = diff(expr6.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr6, result6);
    // Expected: gamma(x) * digamma(x)

    // Example 7: Lambert W function
    let expr7 = "LambertW(x)";
    let result7 = diff(expr7.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr7, result7);
    // Expected: LambertW(x) / (x * (1 + LambertW(x)))

    // Example 8: Digamma function (now has proper derivative)
    let expr8 = "digamma(x)";
    let result8 = diff(expr8.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr8, result8);
    // Expected: trigamma(x)

    // Example 9: Beta function (multi-argument, proper derivative)
    let expr9 = "beta(x, y)";
    let result9 = diff(expr9.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr9, result9);
    // Expected: beta(x, y) * (digamma(x) - digamma(x + y))

    // Example 10: Bessel J function (multi-argument, proper derivative)
    let expr10 = "besselj(0, x)";
    let result10 = diff(expr10.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr10, result10);
    // Expected: (besselj(-1, x) - besselj(1, x)) * 1/2

    // Example 11: Trigamma function (now has proper derivative)
    let expr11 = "trigamma(x)";
    let result11 = diff(expr11.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr11, result11);
    // Expected: tetragamma(x)

    // Example 12: Tetragamma function (now has proper derivative)
    let expr12 = "tetragamma(x)";
    let result12 = diff(expr12.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr12, result12);
    // Expected: polygamma(3, x)

    // Example 13: Polygamma function (general form, proper derivatives)
    let expr13 = "polygamma(0, x)";
    let result13 = diff(expr13.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr13, result13);
    // Expected: polygamma(1, x)

    let expr14 = "polygamma(1, x)";
    let result14 = diff(expr14.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr14, result14);
    // Expected: polygamma(2, x)
}