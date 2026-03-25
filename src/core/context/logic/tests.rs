#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod api_user_tests {
    use super::super::super::{Context, UserFunction};
    use crate::Expr;

    #[test]
    fn test_context_creation() {
        let ctx = Context::new();
        assert!(ctx.function_names().is_empty());
    }

    #[test]
    fn test_context_with_symbol() {
        let ctx = Context::new().with_symbol("x").with_symbol("y");
        assert!(ctx.contains_symbol("x"));
        assert!(ctx.contains_symbol("y"));
        assert!(!ctx.contains_symbol("z"));
    }

    #[test]
    fn test_context_symbol_isolation() {
        let ctx1 = Context::new().with_symbol("__iso_test_var1__");
        let ctx2 = Context::new().with_symbol("__iso_test_var1__");
        let x1 = ctx1.symb("__iso_test_var1__");
        let x2 = ctx2.symb("__iso_test_var1__");
        assert_ne!(x1.id(), x2.id());
    }

    #[test]
    fn test_context_user_function() {
        let ctx = Context::new().with_function(
            "f",
            UserFunction::new(1..=1).body(|args| 2.0 * (*args[0]).clone()),
        );
        assert!(ctx.has_function("f"));
        assert!(!ctx.has_function("g"));
        let f = ctx.get_user_fn("f").expect("Should pass");
        assert!(f.has_body());
        assert!(f.accepts_arity(1));
        assert!(!f.accepts_arity(2));
    }

    #[test]
    fn test_context_function_name_only() {
        let ctx = Context::new().with_function_name("g");
        assert!(ctx.has_function("g"));
        let g = ctx.get_user_fn("g").expect("Should pass");
        assert!(!g.has_body());
    }

    #[test]
    fn test_user_function_partial() {
        let f = UserFunction::new(2..=2)
            .partial(0, |_args| Expr::number(1.0))
            .expect("valid arg")
            .partial(1, |_args| Expr::number(2.0))
            .expect("valid arg");
        assert!(f.has_partial(0));
        assert!(f.has_partial(1));
        assert!(!f.has_partial(2));
    }

    #[test]
    fn test_context_clear() {
        let mut ctx = Context::new().with_symbol("x").with_function_name("f");
        assert!(ctx.contains_symbol("x"));
        assert!(ctx.has_function("f"));
        ctx.clear_all();
        assert!(!ctx.contains_symbol("x"));
        assert!(!ctx.has_function("f"));
    }
}
