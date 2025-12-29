// Phase 3 Tier 2 & 3 function tests

mod tier2_inverse_hyperbolic {

    use crate::diff;

    #[test]
    fn test_asinh() {
        let result = diff("asinh(x)", "x", &[], None).unwrap();
        assert!(result.contains("sqrt"));
    }

    #[test]
    fn test_acosh() {
        let result = diff("acosh(x)", "x", &[], None).unwrap();
        assert!(result.contains("sqrt"));
    }

    #[test]
    fn test_atanh() {
        let result = diff("atanh(x)", "x", &[], None).unwrap();
        // 1/(1-x^2)
        assert!(result.contains("1") && result.contains("x"));
    }

    #[test]
    fn test_acoth() {
        let result = diff("acoth(x)", "x", &[], None).unwrap();
        assert!(result.contains("1") && result.contains("x"));
    }

    #[test]
    fn test_asech() {
        let result = diff("asech(x)", "x", &[], None).unwrap();
        assert!(result.contains("sqrt"));
    }

    #[test]
    fn test_acsch() {
        let result = diff("acsch(x)", "x", &[], None).unwrap();
        assert!(result.contains("sqrt"));
    }
}

mod tier2_log_variants {

    use crate::diff;

    #[test]
    fn test_log10() {
        let result = diff("log10(x)", "x", &[], None).unwrap();
        // 1/(x*ln(10))
        assert!(result.contains("ln") && result.contains("10"));
    }

    #[test]
    fn test_log2() {
        let result = diff("log2(x)", "x", &[], None).unwrap();
        // 1/(x*ln(2))
        assert!(result.contains("ln") && result.contains("2"));
    }

    #[test]
    fn test_log_with_base() {
        // log(e, x) = ln(x), derivative is 1/x
        let result = diff("log(e, x)", "x", &[], None).unwrap();
        // Should simplify to 1/x
        assert!(result.contains("1/x"));
    }
}

mod tier3_special_functions {

    use crate::diff;

    #[test]
    fn test_sinc() {
        let result = diff("sinc(x)", "x", &[], None).unwrap();
        assert!(result.contains("cos") && result.contains("sin"));
    }

    #[test]
    fn test_erf() {
        let result = diff("erf(x)", "x", &[], None).unwrap();
        // Contains exp(-x^2) and sqrt(pi)
        assert!(result.contains("exp") && result.contains("pi"));
    }

    #[test]
    fn test_erfc() {
        let result = diff("erfc(x)", "x", &[], None).unwrap();
        assert!(result.contains("exp") && result.contains("pi"));
    }

    #[test]
    fn test_gamma() {
        let result = diff("gamma(x)", "x", &[], None).unwrap();
        assert!(result.contains("gamma") && result.contains("digamma"));
    }

    #[test]
    fn test_lambertw() {
        let result = diff("lambertw(x)", "x", &[], None).unwrap();
        assert!(result.contains("lambertw"));
    }
}

mod tier3_unimplemented_placeholders {
    use crate::diff;

    #[test]
    fn test_besselj_parsing() {
        // besselj now has implemented derivatives
        let result = diff("besselj(0, x)", "x", &[], None).unwrap();
        // Should produce a result - just verify it parses and differentiates
        assert!(!result.is_empty());
    }
}
