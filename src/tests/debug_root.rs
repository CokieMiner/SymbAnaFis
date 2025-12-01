#[cfg(test)]
mod tests {
    use crate::simplify;

    #[test]
    fn debug_root_simplification() {
        // sqrt(x^2) = |x| for all real x
        let result = simplify("sqrt(x^2)".to_string(), None, None).unwrap();
        println!("Simplified Display: {}", result);
        assert_eq!(result, "abs(x)");
    }
}
