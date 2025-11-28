// Phase 3 Tier 2 & 3 function tests

mod tier2_inverse_hyperbolic {

    use crate::diff;

    #[test]
    fn test_asinh() {
        let result = diff("asinh(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sqrt"));
    }

    #[test]
    fn test_acosh() {
        let result = diff("acosh(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sqrt"));
    }

    #[test]
    fn test_atanh() {
        let result = diff("atanh(x)".to_string(), "x".to_string(), None, None).unwrap();
        // 1/(1-x^2)
        assert!(result.contains("1") && result.contains("x"));
    }

    #[test]
    fn test_acoth() {
        let result = diff("acoth(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("1") && result.contains("x"));
    }

    #[test]
    fn test_asech() {
        let result = diff("asech(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sqrt"));
    }

    #[test]
    fn test_acsch() {
        let result = diff("acsch(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("sqrt"));
    }
}

mod tier2_log_variants {

    use crate::diff;

    #[test]
    fn test_log10() {
        let result = diff("log10(x)".to_string(), "x".to_string(), None, None).unwrap();
        // 1/(x*ln(10))
        assert!(result.contains("ln") && result.contains("10"));
    }

    #[test]
    fn test_log2() {
        let result = diff("log2(x)".to_string(), "x".to_string(), None, None).unwrap();
        // 1/(x*ln(2))
        assert!(result.contains("ln") && result.contains("2"));
    }

    #[test]
    fn test_log_default() {
        let result = diff("log(x)".to_string(), "x".to_string(), None, None).unwrap();
        // 1/x
        assert_eq!(result, "1 / x");
    }
}

mod tier3_special_functions {

    use crate::diff;

    #[test]
    fn test_sinc() {
        let result = diff("sinc(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("cos") && result.contains("sin"));
    }

    #[test]
    fn test_erf() {
        let result = diff("erf(x)".to_string(), "x".to_string(), None, None).unwrap();
        // Contains exp(-x^2) and sqrt(pi)
        assert!(result.contains("exp") && result.contains("pi"));
    }

    #[test]
    fn test_erfc() {
        let result = diff("erfc(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("exp") && result.contains("pi"));
    }

    #[test]
    fn test_gamma() {
        let result = diff("gamma(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("gamma") && result.contains("digamma"));
    }

    #[test]
    fn test_lambertw() {
        let result = diff("LambertW(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("LambertW"));
    }
}

mod tier3_unimplemented_placeholders {
    use crate::diff;

    #[test]
    fn test_besselj_parsing() {
        // Should parse but return generic derivative ∂_besselj(x)/∂_x
        let result = diff("besselj(x)".to_string(), "x".to_string(), None, None).unwrap();
        assert!(result.contains("besselj") && result.contains("∂"));
    }

}
