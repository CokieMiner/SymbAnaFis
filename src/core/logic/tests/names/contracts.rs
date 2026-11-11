//! Registered function ownership, uniqueness, and argument contracts.

use alloc::{format, vec};

use proptest::{prop_assert, prop_assert_eq, prop_assert_ne, proptest};

use crate::{
    core::{Context, Expr, Symbol, UserFunction},
    errors::FunctionError,
    rewrite::Pattern,
};

use super::super::super::Function;

proptest! {
    #[test]
    fn duplicate_names_fail_and_removal_allows_a_fresh_arity(
        suffix in "[a-z]{1,24}", first_arity in 0_usize..=5, next_arity in 6_usize..=10,
    ) {
        let context = Context::new().expect("context");
        let name = format!("custom_{suffix}");
        let first = context.register_function(&name, UserFunction::new(first_arity..=first_arity))
            .expect("the custom name is unbound");
        prop_assert_eq!(
            context.register_function(&name, UserFunction::new(next_arity..=next_arity)),
            Err(FunctionError::DuplicateName(name.clone()))
        );
        prop_assert_eq!(context.get_function(&name), Some(first.clone()));
        prop_assert!(context.remove_function(&name));
        let next = context.register_function(&name, UserFunction::new(next_arity..=next_arity))
            .expect("removal releases the name binding");
        prop_assert_ne!(&first, &next);
        prop_assert_eq!(first.arity(), first_arity..=first_arity);
        prop_assert_eq!(next.arity(), next_arity..=next_arity);
        prop_assert!(first.call(vec![Expr::number(1); first_arity]).is_ok());
        prop_assert!(next.call(vec![Expr::number(1); next_arity]).is_ok());
    }

    #[test]
    fn calls_and_patterns_reject_unregistered_arities(
        minimum in 0_usize..=5, width in 0_usize..=5, actual in 0_usize..=12,
    ) {
        let maximum = minimum.checked_add(width).expect("bounded arities fit usize");
        let context = Context::new().expect("context");
        let function = context.register_function("arity_property", UserFunction::new(minimum..=maximum))
            .expect("the custom name is unbound");
        let call = function.call(vec![Expr::number(1); actual]);
        let pattern = Pattern::apply(function, vec![Pattern::capture("a", None); actual]);
        let accepts = (minimum..=maximum).contains(&actual);
        prop_assert_eq!(call.is_ok(), accepts);
        prop_assert_eq!(pattern.is_ok(), accepts);
        if !accepts {
            prop_assert_eq!(call, Err(FunctionError::InvalidArity { minimum, maximum, actual }));
        }
    }

    #[test]
    fn empty_arity_ranges_do_not_publish_function_metadata(
        minimum in 1_usize..=100, difference in 1_usize..=100,
    ) {
        let maximum = minimum.saturating_sub(difference);
        let context = Context::new().expect("context");
        prop_assert_eq!(
            context.register_function("invalid_range", UserFunction::new(minimum..=maximum)),
            Err(FunctionError::InvalidArityRange { minimum, maximum })
        );
        prop_assert!(context.get_function("invalid_range").is_none());
        prop_assert!(context.is_empty());
    }

    #[test]
    fn builtin_names_and_aliases_are_reserved(
        name in proptest::sample::select(vec!["sin", "cos", "hermite", "bessel_j", "besselj", "lambert_w", "lambertw"]),
        actual in 0_usize..=5,
    ) {
        let context = Context::new().expect("context");
        prop_assert_eq!(
            context.register_function(name, UserFunction::default()),
            Err(FunctionError::DuplicateName(name.into()))
        );
        prop_assert!(context.get_function(name).is_none());
        let builtin = Function::builtin(name).expect("the sampled builtin name is known");
        prop_assert_eq!(
            builtin.call(vec![Symbol::intern("builtin_arity_argument").expect("symbol").to_expr(); actual]).is_ok(),
            builtin.arity().contains(&actual)
        );
    }
}
