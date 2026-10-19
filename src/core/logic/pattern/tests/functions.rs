//! Literal folding, typed builtin arguments, and captured function construction.

use alloc::vec;

use proptest::{prop_assert_eq, proptest};

use crate::core::{Context, Expr, Function, UserFunction};

use super::super::{Pattern, RewriteRule};

#[test]
fn required_functions_are_conservative_about_virtual_powers_and_exact_templates() {
    let context = Context::new();
    let host = context
        .register_function("host", UserFunction::new(1..=1))
        .expect("unbound name");
    let x = context.symb("x").expect("available symbol");
    let a = Pattern::capture("a", None);
    let source = Pattern::sum(vec![
        a.sqrt(),
        a.sin(),
        a.cos(),
        Pattern::apply(host.clone(), vec![a.clone()]).expect("unary host"),
        Pattern::exact(x.exp()),
    ]);
    let rule = RewriteRule::new(source, a).expect("bound capture");
    let required = rule.required_functions().collect::<alloc::vec::Vec<_>>();
    assert_eq!(required.len(), 3);
    assert!(required.contains(&host));
    assert!(required.contains(&Function::builtin("sin").expect("known builtin")));
    assert!(required.contains(&Function::builtin("cos").expect("known builtin")));
}

proptest! {
    #[test]
    fn literal_function_arguments_fold_inside_captured_calls(
        value in 0_i32..=16, operation in 0_u8..=5,
    ) {
        let context = Context::new();
        let host = context.register_function("host", UserFunction::new(2..=2)).expect("unbound name");
        let x = context.symb("x")?;
        let a = Pattern::capture("a", None);
        let squared = value.saturating_mul(value);
        let (literal, expected) = match operation {
            0 => (Pattern::exact(squared).sqrt(), Expr::number(value)),
            1 => (Pattern::exact(0).sin(), Expr::number(0)),
            2 => (Pattern::exact(0).cos(), Expr::number(1)),
            3 => (Pattern::exact(0).exp(), Expr::number(1)),
            4 => (Pattern::exact(value).floor(), Expr::number(value)),
            _ => (Pattern::exact(0).lambertw(0), Expr::number(0)),
        };
        let pattern = Pattern::apply(host.clone(), vec![a.clone(), literal]).expect("binary host");
        let rule = RewriteRule::new(pattern, a).expect("bound capture");
        let input = host.call(vec![x.into(), expected]).expect("binary host");
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(x.to_expr()));
    }

    #[test]
    fn builtin_templates_bind_the_documented_argument_positions(
        family in 0_u8..=5, first in -3_i32..=3, second in -3_i32..=3,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let a = Pattern::capture("a", None);
        let (pattern, name, arguments) = match family {
            0 => (a.log(first), "log", vec![Expr::number(first), x.into()]),
            1 => (a.besselj(first), "besselj", vec![Expr::number(first), x.into()]),
            2 => (a.beta(first), "beta", vec![x.into(), Expr::number(first)]),
            3 => (a.atan2(first), "atan2", vec![x.into(), Expr::number(first)]),
            4 => (a.assoc_legendre(first, second), "assoc_legendre", vec![Expr::number(first), Expr::number(second), x.into()]),
            _ => (a.spherical_harmonic(first, second, 1), "spherical_harmonic", vec![Expr::number(first), Expr::number(second), x.into(), Expr::number(1)]),
        };
        let input = Function::builtin(name).expect("known builtin").call(arguments).expect("accepted arity");
        let rule = RewriteRule::new(pattern, Pattern::exact(u)).expect("literal replacement");
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(u.to_expr()));
    }

    #[test]
    fn literal_and_fluent_calls_have_the_same_numeric_policy(value in -8_i32..=8) {
        let context = Context::new();
        let host = context.register_function("host", UserFunction::new(2..=2)).expect("unbound name");
        let x = context.symb("x")?;
        let a = Pattern::capture("a", None);
        let argument = Expr::number(f64::from(value));
        let sine = Function::builtin("sin").expect("known builtin");
        let literal = Pattern::apply(sine, vec![Pattern::exact(&argument)]).expect("unary builtin");
        let nested = Pattern::apply(host.clone(), vec![a.clone(), literal]).expect("binary host");
        let rule = RewriteRule::new(nested, a).expect("bound capture");
        prop_assert_eq!(rule.apply_all(&host.call(vec![x.into(), argument.sin()]).expect("binary host"), None, None), Ok(x.to_expr()));
    }
}
