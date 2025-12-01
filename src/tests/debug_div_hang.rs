#[cfg(test)]
mod tests {
    use crate::diff;

    #[test]
    fn test_div_hang_repro() {
        // sin(x)/x
        println!("Starting differentiation of sin(x)/x");
        let result = diff("sin(x)/x".to_string(), "x".to_string(), None, None);
        match result {
            Ok(res) => println!("Result: {}", res),
            Err(e) => println!("Error: {:?}", e),
        }
    }
}
