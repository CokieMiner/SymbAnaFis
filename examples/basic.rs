use symb_anafis::diff;

fn main() {
    println!("SymbAnaFis Basic Example");
    println!("------------------------");

    // Example 1: Simple polynomial
    let expr1 = "x^2 + 3*x + 5";
    let result1 = diff(expr1.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr1, result1);
    // Expected: 2 * x + 3

    // Example 2: Trigonometric function
    let expr2 = "sin(x) * cos(x)";
    let result2 = diff(expr2.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr2, result2);
    // Expected: cos(x)^2 - sin(x)^2 (or similar simplified form)

    // Example 3: Exponential and Logarithm
    let expr3 = "exp(x) * ln(x)";
    let result3 = diff(expr3.to_string(), "x".to_string(), None, None).unwrap();
    println!("d/dx [{}] = {}", expr3, result3);
    // Expected: exp(x) * ln(x) + exp(x) / x
}
