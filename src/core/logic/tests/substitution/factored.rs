//! Manifest symbolic factors in expanded additive groups.

use core::ops::{Add, Div};

use alloc::vec;

use num_anafis::Number;
use proptest::{arbitrary::any, prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, Expr, Pattern, RewriteRule, Substitution, Substitutions, UserFunction},
};

proptest! {
    #[test]
    fn symbolic_factors_replace_expanded_groups_and_preserve_remaining_terms(
        scale in 1_i32..=9,
        denominator in 1_i32..=9,
        negative in any::<bool>(),
        extra in 0_i32..=9,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let z = context.symb("z")?;
        let u = context.symb("u")?;
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("unbound name");
        let factor = f.call(vec![z.into()]).expect("unary function");
        let numerator = if negative { scale.saturating_neg() } else { scale };
        let coefficient = Expr::number(Div::div(Number::from(numerator), Number::from(denominator)));
        let signed_extra = if negative { extra.saturating_neg() } else { extra };
        let residual = Expr::product(vec![Expr::number(signed_extra), factor.clone(), x.into()]);
        let source = Add::add(x, y);
        let mut terms = vec![
            Expr::product(vec![coefficient.clone(), factor.clone(), x.into()]),
            Expr::product(vec![coefficient.clone(), factor.clone(), y.into()]),
            z.pow(2),
        ];
        if extra != 0 { terms.push(residual.clone()); }
        let input = Expr::sum(terms);
        let mut expected_terms = vec![Expr::product(vec![coefficient, factor, u.into()]), z.pow(2)];
        if extra != 0 { expected_terms.push(residual); }
        let expected = Expr::sum(expected_terms);
        let substitution = Substitution::new(source.clone(), u.into());
        let result = substitution.apply_all(&input);
        prop_assert_eq!(&result, &expected);
        let rule = RewriteRule::new(Pattern::exact(source), Pattern::exact(u)).expect("literal rule");
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(expected));
    }

    #[test]
    fn every_disjoint_factor_group_is_consumed_even_after_an_explicit_match(
        exponent in 2_i32..=6,
        factors in 1_i32..=8,
        include_explicit in any::<bool>(),
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let z = context.symb("z")?;
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("unbound name");
        let source = Add::add(x.pow(exponent), x.pow(exponent.saturating_add(1)));
        let mut input = vec![z.pow(2)];
        let mut expected = vec![z.pow(2)];
        if include_explicit { input.push(source.clone()); expected.push(u.into()); }
        for index in 0..factors {
            let factor = f.call(vec![Expr::number(index)]).expect("unary function");
            input.extend([
                Expr::product(vec![factor.clone(), x.pow(exponent)]),
                Expr::product(vec![factor.clone(), x.pow(exponent.saturating_add(1))]),
            ]);
            expected.push(Expr::product(vec![factor, u.into()]));
        }
        let result = Substitution::new(source, u.into()).apply_all(&Expr::sum(input));
        prop_assert!(!result.contains_symbol(x));
        prop_assert_eq!(result, Expr::sum(expected));
    }

    #[test]
    fn constant_terms_in_sources_share_a_symbolic_multiplier(value in 1_i32..=12) {
        let context = Context::new();
        let x = context.symb("x")?;
        let z = context.symb("z")?;
        let u = context.symb("u")?;
        let factor = z.sin();
        let source = Add::add(x, value);
        let input = Expr::sum(vec![
            Expr::product(vec![factor.clone(), x.into()]),
            Expr::product(vec![Expr::number(value), factor.clone()]),
            z.pow(2),
        ]);
        let result = Substitution::new(source, u.into()).apply_all(&input);
        prop_assert_eq!(result, Expr::sum(vec![Expr::product(vec![factor, u.into()]), z.pow(2)]));
    }

    #[test]
    fn surviving_factors_are_transformed_without_visiting_destinations(value in -5_i32..=5) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let z = context.symb("z")?;
        let w = context.symb("w")?;
        let u = context.symb("u")?;
        let factor = z.sin();
        let destination = Add::add(z, u);
        let source = Add::add(x, y);
        let input = Expr::sum(vec![
            Expr::product(vec![factor.clone(), x.into()]),
            Expr::product(vec![factor, y.into()]),
            Expr::product(vec![Expr::number(value), u.into()]),
        ]);
        let substitutions = Substitutions::new([
            (source, destination.clone()), (z.into(), w.into()), (u.into(), Expr::number(99)),
        ]).expect("distinct sources");
        let result = substitutions.apply_all(&input);
        prop_assert_eq!(result, Expr::sum(vec![
            Expr::product(vec![w.sin(), destination]),
            Expr::number(value.saturating_mul(99)),
        ]));
    }

    #[test]
    fn common_factors_can_contain_the_source_and_reuse_its_replacement(
        first in -4_i32..=4, second in -4_i32..=4,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let u = context.symb("u")?;
        let source = Add::add(x, y);
        let input = Expr::sum(vec![
            Expr::product(vec![source.clone(), x.into()]),
            Expr::product(vec![source.clone(), y.into()]),
        ]);
        let result = Substitution::new(source, u.into()).apply_all(&input);
        prop_assert_eq!(&result, &u.pow(2));
        let values = HashMap::from([
            (x, Expr::number(first)), (y, Expr::number(second)),
            (u, Expr::number(first.saturating_add(second))),
        ]);
        prop_assert_eq!(result.substitute_symbols(&values), input.substitute_symbols(&values));
    }

    #[test]
    fn complex_constants_and_scales_share_manifest_symbolic_factors(scale in 1_i32..=9) {
        let context = Context::new();
        let x = context.symb("x")?;
        let z = context.symb("z")?;
        let u = context.symb("u")?;
        let factor = z.sin();
        let coefficient = Expr::product(vec![Expr::number(scale), Expr::i()]);
        let source = Add::add(x, Expr::i());
        let input = Expr::sum(vec![
            Expr::product(vec![coefficient.clone(), factor.clone(), x.into()]),
            Expr::product(vec![coefficient.clone(), factor.clone(), Expr::i()]),
        ]);
        let result = Substitution::new(source, u.into()).apply_all(&input);
        prop_assert_eq!(result, Expr::product(vec![coefficient, factor, u.into()]));
    }

    #[test]
    fn common_symbolic_denominators_retain_their_original_zero_domains(
        nested in any::<bool>(),
        exponent in 1_i32..=5,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let z = context.symb("z")?;
        let w = context.symb("w")?;
        let u = context.symb("u")?;
        let denominator = if nested { Div::div(z.to_expr(), w) } else { z.pow(exponent) };
        let source = Add::add(x, y);
        let input = Expr::sum(vec![
            Div::div(x.to_expr(), &denominator),
            Div::div(y.to_expr(), &denominator),
        ]);
        let result = Substitution::new(source.clone(), u.into()).apply_all(&input);
        prop_assert_eq!(result, Div::div(u.to_expr(), &denominator));
        let simultaneous = Substitutions::new([
            (source, u.into()), (denominator, Expr::number(0)),
        ]).expect("distinct sources");
        prop_assert_eq!(simultaneous.apply_all(&input), Div::div(u.to_expr(), Expr::number(0)));
    }

    #[test]
    fn identity_groups_preserve_the_original_canonical_representation(
        exponent in 2_i32..=8, negative in any::<bool>(),
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let z = context.symb("z")?;
        let source = Add::add(x, y);
        let sum = Expr::sum(vec![
            Expr::product(vec![z.sin(), x.into()]),
            Expr::product(vec![z.sin(), y.into()]),
        ]);
        let signed = if negative { exponent.saturating_neg() } else { exponent };
        let power = x.pow(signed).pow(3);
        for (from, input) in [(source, sum), (x.pow(2), power)] {
            prop_assert_eq!(&Substitution::new(from.clone(), from.clone()).apply_all(&input), &input);
            let rule = RewriteRule::new(Pattern::exact(&from), Pattern::exact(&from)).expect("literal rule");
            prop_assert_eq!(rule.apply_all(&input, None, None), Ok(input.clone()));
            let identities = Substitutions::new([(from.clone(), from), (z.into(), z.into())]).expect("identity sources");
            prop_assert_eq!(identities.apply_all(&input), input);
        }
    }
}

#[test]
fn nonfinite_factors_are_not_pulled_out_of_addition() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let y = context.symb("y").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation");
    let substitution = Substitution::new(Add::add(x, y), u.into());
    for value in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let input = Expr::sum(vec![
            Expr::product(vec![Expr::number(value), x.into()]),
            Expr::product(vec![Expr::number(value), y.into()]),
        ]);
        assert_eq!(
            substitution.apply_all(&input),
            input,
            "nonfinite scalar factors retain their operations"
        );
    }
}
