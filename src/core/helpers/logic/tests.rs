//! Tests for foundational `core/` modules: poly, error, traits, and visitor.

#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod error_tests {
    use super::super::error::*;

    #[test]
    fn test_span_creation() {
        let span = Span::new(5, 10);
        assert_eq!(span.start(), 5);
        assert_eq!(span.end(), 10);
        assert!(span.is_valid());
    }

    #[test]
    fn test_span_swap() {
        let span = Span::new(10, 5);
        assert_eq!(span.start(), 5);
        assert_eq!(span.end(), 10);
        assert!(span.is_valid());
    }

    #[test]
    fn test_span_at() {
        let span = Span::at(7);
        assert_eq!(span.start(), 7);
        assert_eq!(span.end(), 8);
        assert!(span.is_valid());
    }

    #[test]
    fn test_span_empty() {
        let span = Span::empty();
        assert_eq!(span.start(), 0);
        assert_eq!(span.end(), 0);
        assert!(!span.is_valid());
    }

    #[test]
    fn test_span_display() {
        let span1 = Span::new(4, 8);
        assert_eq!(span1.display(), " at positions 5-8");

        let span2 = Span::at(9);
        assert_eq!(span2.display(), " at position 10");

        let span3 = Span::empty();
        assert_eq!(span3.display(), "");
    }

    #[test]
    fn test_diff_error_display() {
        let err1 = DiffError::EmptyFormula;
        assert_eq!(format!("{err1}"), "Formula cannot be empty");

        let err2 = DiffError::invalid_syntax("test message");
        assert_eq!(format!("{err2}"), "Invalid syntax: test message");

        let err3 = DiffError::invalid_syntax_at("spanned message", Span::new(1, 3));
        assert_eq!(
            format!("{err3}"),
            "Invalid syntax: spanned message at positions 2-3"
        );

        let err4 = DiffError::MaxDepthExceeded;
        assert_eq!(
            format!("{err4}"),
            "Expression nesting depth exceeds maximum limit"
        );
    }

    #[test]
    fn test_diff_error_constructors() {
        let err5 = DiffError::invalid_syntax("msg");
        match err5 {
            DiffError::InvalidSyntax { msg, span: None } => assert_eq!(msg, "msg"),
            _ => panic!("Wrong error type"),
        }

        let err6 = DiffError::invalid_syntax_at("msg", Span::at(5));
        match err6 {
            DiffError::InvalidSyntax {
                msg,
                span: Some(span),
            } => {
                assert_eq!(msg, "msg");
                assert_eq!(span.start(), 5);
            }
            _ => panic!("Wrong error type"),
        }
    }
}

#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod traits_tests {
    use super::super::traits::*;

    #[test]
    fn test_is_zero() {
        assert!(is_zero(0.0));
        assert!(is_zero(1e-15));
        assert!(is_zero(-1e-15));
        assert!(!is_zero(0.1));
        assert!(!is_zero(-0.1));
    }

    #[test]
    fn test_is_one() {
        assert!(is_one(1.0));
        assert!(is_one(1.0 + 1e-15));
        assert!(is_one(1.0 - 1e-15));
        assert!(!is_one(1.1));
        assert!(!is_one(0.9));
    }

    #[test]
    fn test_is_neg_one() {
        assert!(is_neg_one(-1.0));
        assert!(is_neg_one(-1.0 + 1e-15));
        assert!(!is_neg_one(1.0));
    }
}
