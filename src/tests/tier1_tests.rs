// Phase 3 Tier 1 function tests

mod tier1_trig {

    use crate::diff;

    #[test]
    fn test_tan() {
        let result = diff("tan(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sec"));
    }

    #[test]
    fn test_cot() {
        let result = diff("cot(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("csc"));
    }

    #[test]
    fn test_sec() {
        let result = diff("sec(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sec") && result.contains("tan"));
    }

    #[test]
    fn test_csc() {
        let result = diff("csc(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("csc") && result.contains("cot"));
    }

    #[test]
    fn test_sen_alias() {
        // Portuguese/Spanish alias for sin
        let result = diff("sen(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("cos"));
    }
}

mod tier1_inverse_trig {

    use crate::diff;

    #[test]
    fn test_asin() {
        let result = diff("asin(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sqrt"));
    }

    #[test]
    fn test_acos() {
        let result = diff("acos(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sqrt"));
    }

    #[test]
    fn test_atan() {
        let result = diff("atan(x)".to_string(), "x".to_string(), None, None).unwrap();
        // Result should have division
        assert!(result.contains("/") || result.contains("1"));
    }

    #[test]
    fn test_acot() {
        let result = diff("acot(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("/") || result.contains("1"));
    }

    #[test]
    fn test_asec() {
        let result = diff("asec(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sqrt"));
    }

    #[test]
    fn test_acsc() {
        let result = diff("acsc(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sqrt"));
    }
}

mod tier1_roots {

    use crate::diff;

    #[test]
    fn test_sqrt() {
        let result = diff("sqrt(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sqrt") || result.contains("2"));
    }

    #[test]
    fn test_cbrt() {
        let result = diff("cbrt(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("3") || result.contains("cbrt"));
    }

    #[test]
    fn test_sqrt_composition() {
        let result = diff("sqrt(x^2 + 1)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sqrt") && result.contains("x"));
    }
}

mod tier2_extended_hyperbolics {

    use crate::diff;

    #[test]
    fn test_coth() {
        let result = diff("coth(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("csch"));
    }

    #[test]
    fn test_sech() {
        let result = diff("sech(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sech") && result.contains("tanh"));
    }

    #[test]
    fn test_csch() {
        let result = diff("csch(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("csch") && result.contains("coth"));
    }
}

mod complex_expressions_tier1 {

    use crate::diff;

    #[test]
    fn test_tan_product() {
        let result = diff("tan(x) * x".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_asin_nested() {
        let result = diff("asin(sin(x))".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sqrt_of_sqrt() {
        let result = diff("sqrt(sqrt(x))".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mixed_trig() {
        let result = diff("tan(x) + cot(x)".to_string(), "x".to_string(), None, None);
        assert!(result.is_ok());
    }
}
