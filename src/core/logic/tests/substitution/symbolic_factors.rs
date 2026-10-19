//! Scalar substitutions inside arbitrary symbolic factors and general powers.

use core::ops::{Div, Mul};

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::{arbitrary::any, prop_assert, prop_assert_eq, prop_assume, proptest};

use crate::{
    collections::HashMap,
    core::{Context, Expr, Pattern, RewriteRule, Substitution, Substitutions, UserFunction},
};

proptest! {
    #[test]
    fn scalar_substitutions_eliminate_the_base_inside_symbolic_factors(
        depends_on_base in any::<bool>(),
        exponent in 1_i64..=10,
        value in -3_i64..=3,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let z = context.symb("z")?;
        let u = context.symb("u")?;
        let f = context.register_function("f", UserFunction::new(2..=2)).expect("the name is unbound");
        let argument = if depends_on_base { x } else { y };
        let factor = f.call(vec![argument.into(), z.into()]).expect("f is binary");
        let source = Mul::mul(Expr::number(2), x);
        let input = Mul::mul(factor, x.pow(exponent));
        let result = Substitution::new(source.clone(), u.into()).apply_all(&input);
        prop_assert!(!result.contains_symbol(x), "all original occurrences of x must be replaced");
        prop_assert!(result.contains_symbol(z));
        prop_assert_eq!(result.contains_symbol(y), !depends_on_base);

        let half = Expr::number(Number::from_rational(RationalType::new(1, 2)));
        let expected_argument = if depends_on_base { Mul::mul(&half, u) } else { y.into() };
        let expected_factor = f.call(vec![expected_argument, z.into()]).expect("f is binary");
        let coefficient = Div::div(Number::one(), Number::from_int(2).pow(&Number::from_int(exponent)));
        let expected = Expr::product(vec![Expr::number(coefficient), expected_factor, u.pow(exponent)]);
        prop_assert_eq!(&result, &expected);
        let simultaneous = Substitutions::new([(source.clone(), u.into())]).expect("one source cannot conflict");
        prop_assert_eq!(&simultaneous.apply_all(&input), &result);

        let rule = RewriteRule::new(Pattern::exact(source), Pattern::exact(u)).expect("the rule has no captures");
        let rewritten = rule.apply_all(&input, None, None).expect("unrestricted matching");
        prop_assert!(!rewritten.contains_symbol(x), "bottom-up rewriting must also replace every x");
        let values = HashMap::from([
            (x, Expr::number(value)),
            (u, Expr::number(value.saturating_mul(2))),
            (y, Expr::number(5)),
            (z, Expr::number(7)),
        ]);
        let evaluated = input.substitute_symbols(&values);
        prop_assert_eq!(&result.substitute_symbols(&values), &evaluated);
        prop_assert_eq!(rewritten.substitute_symbols(&values), evaluated);
    }

    #[test]
    fn arbitrary_composite_factors_are_rewritten_through_every_shared_edge(
        repetitions in 1_i64..=5,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let u = context.symb("u")?;
        let replacement = Mul::mul(Expr::number(Number::from_rational(RationalType::new(1, 2))), u);
        let factor = Expr::sum(vec![x.sin(), x.cos(), y.into()]);
        let input = Mul::mul(factor.pow(repetitions), x);
        let expected_factor = Expr::sum(vec![replacement.sin(), replacement.cos(), y.into()]);
        let expected = Mul::mul(expected_factor.pow(repetitions), &replacement);
        let substitution = Substitution::new(Mul::mul(Expr::number(2), x), u.into());
        let result = substitution.apply_all(&input);
        prop_assert_eq!(&result, &expected);
        prop_assert!(!result.contains_symbol(x));
        prop_assert!(result.contains_symbol(y));
    }

    #[test]
    fn negative_and_fractional_power_bases_are_still_traversed(
        numerator in -9_i64..=9,
        denominator in 1_i64..=9,
    ) {
        prop_assume!(numerator <= 0 || numerator.checked_rem(denominator) != Some(0));
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let source = Mul::mul(Expr::number(2), x);
        let replacement = Mul::mul(Expr::number(Number::from_rational(RationalType::new(1, 2))), u);
        let exponent = Expr::number(Number::from_rational(RationalType::new(numerator, denominator)));
        let input = x.pow(&exponent);
        let expected = replacement.pow(exponent);
        let substitution = Substitution::new(source.clone(), u.into());
        prop_assert_eq!(&substitution.apply_all(&input), &expected);
        let rule = RewriteRule::new(Pattern::exact(source), Pattern::exact(u)).expect("the rule has no captures");
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(expected));
    }

    #[test]
    fn reciprocal_and_root_arguments_replace_the_variable_without_branch_inference(
        degree in 2_i64..=9,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let replacement = Mul::mul(Expr::number(Number::from_rational(RationalType::new(1, 2))), u);
        let input = Expr::sum(vec![Div::div(Expr::number(1), x.pow(degree)), x.sqrt(), x.cbrt()]);
        let scale = Expr::number(Div::div(Number::one(), Number::from_int(2).pow(&Number::from_int(degree))));
        let expected = Expr::sum(vec![
            Div::div(Expr::number(1), Mul::mul(scale, u.pow(degree))),
            replacement.sqrt(),
            replacement.cbrt(),
        ]);
        let substitution = Substitution::new(Mul::mul(Expr::number(2), x), u.into());
        let result = substitution.apply_all(&input);
        prop_assert_eq!(&result, &expected);
        prop_assert!(!result.contains_symbol(x));
    }
}
