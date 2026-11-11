//! Operand partitions, grouped captures, and lazy integer or rational power blocks.

use core::ops::{Add, Div, Mul, Sub};

use alloc::{format, vec, vec::Vec};

use num_anafis::{Number, RationalType};
use proptest::{arbitrary::any, bool, prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, Expr, Substitution, UserFunction},
};

use super::super::{
    AlgebraicOperands, Group, GroupValues, Pattern, RewriteError, RewriteRule, Selected,
};

proptest! {
    #[test]
    fn universal_scalar_templates_visit_original_children_before_the_parent(coefficient in 2_i32..=9) {
        let ctx = Context::new()?;
        let x = ctx.symb("x")?;
        let y = ctx.symb("y")?;
        let f = ctx.register_function("f", UserFunction::new(1..=1)).expect("unbound name");
        let a = Pattern::capture("a", None);
        let pattern = Pattern::product(vec![Pattern::exact(Expr::number(coefficient)), a.clone()]);
        let rule = RewriteRule::new(
            pattern,
            Pattern::apply(f.clone(), vec![a]).expect("unary function"),
        )
        .expect("bound capture");
        let scale = Div::div(Expr::number(1), Expr::number(coefficient));
        let children = Expr::sum(vec![
            f.call(vec![Expr::product(vec![scale.clone(), x.into()])])
                .expect("unary function"),
            f.call(vec![Expr::product(vec![scale.clone(), y.into()])])
                .expect("unary function"),
        ]);
        let expected = f
            .call(vec![Expr::product(vec![scale, children])])
            .expect("unary function");
        prop_assert_eq!(
            rule.apply_all(&Expr::sum(vec![x.into(), y.into()]), usize::MAX, None),
            Ok(expected)
        );
    }

    #[test]
    fn generic_scalar_templates_infer_coefficients_inside_function_arguments(
        coefficient in 2_i32..=9,
        shape in 0_u8..=4,
        negative in any::<bool>(),
    ) {
        let ctx = Context::new()?;
        let x = ctx.symb("x")?;
        let y = ctx.symb("y")?;
        let host = ctx.register_function("host", UserFunction::new(1..=1)).expect("unbound name");
        let output = ctx.register_function("output", UserFunction::new(1..=1)).expect("unbound name");
        let scalar = if negative { coefficient.saturating_neg() } else { coefficient };
        let value = match shape {
            0 => x.to_expr(),
            1 => Expr::sum(vec![x.into(), y.into()]),
            2 => Expr::product(vec![x.into(), y.into()]),
            3 => x.pow(3),
            _ => Expr::number(7),
        };
        let a = Pattern::capture("a", None);
        let product = Pattern::product(vec![Pattern::exact(Expr::number(scalar)), a.clone()]);
        let rule = RewriteRule::new(
            Pattern::apply(host.clone(), vec![product]).expect("unary host"),
            Pattern::apply(output.clone(), vec![a]).expect("unary output"),
        )
        .expect("bound capture");
        let expected = Expr::product(vec![
            Div::div(Expr::number(1), Expr::number(scalar)),
            value.clone(),
        ]);
        prop_assert_eq!(
            rule.apply_all(&host.call(vec![value]).expect("unary host"), usize::MAX, None),
            Ok(output.call(vec![expected]).expect("unary output"))
        );
    }

    #[test]
    fn implicit_scalar_captures_respect_other_bindings_and_the_work_limit(coefficient in 2_i32..=9) {
        let ctx = Context::new()?;
        let x = ctx.symb("x")?;
        let u = ctx.symb("u")?;
        let host = ctx.register_function("host", UserFunction::new(2..=2)).expect("unbound name");
        let a = Pattern::capture("a", None);
        let quotient = Expr::product(vec![
            Div::div(Expr::number(1), Expr::number(coefficient)),
            x.into(),
        ]);
        let product = Pattern::product(vec![Pattern::exact(Expr::number(coefficient)), a.clone()]);
        let nested = RewriteRule::new(
            Pattern::apply(host.clone(), vec![product.clone(), a.clone()]).expect("binary host"),
            Pattern::exact(u),
        )
        .expect("literal replacement");
        prop_assert_eq!(
            nested.apply_all(&host.call(vec![x.into(), quotient.clone()]).expect("binary host"), usize::MAX, None),
            Ok(u.to_expr())
        );
        let root = RewriteRule::new(product, a).expect("bound capture");
        prop_assert_eq!(
            root.apply_all(&x.to_expr(), 0, None),
            Err(RewriteError::MatchLimitExceeded { limit: 0 })
        );
        prop_assert_eq!(root.apply_all(&x.to_expr(), 1, None), Ok(quotient));
    }

    #[test]
    fn nested_captures_partition_all_operands(sum in any::<bool>(), length in 3_usize..=7, reverse in any::<bool>()) {
        let ctx = Context::new()?;
        let host = ctx.register_function("host", UserFunction::new(1..=1)).expect("unbound name");
        let pair = ctx.register_function("pair", UserFunction::new(2..=2)).expect("unbound name");
        let a = Pattern::capture("a", None);
        let b = Pattern::capture("b", None);
        let group = if sum {
            Pattern::sum(vec![a.clone(), b.clone()])
        } else {
            Pattern::product(vec![a.clone(), b.clone()])
        };
        let rule = RewriteRule::new(
            Pattern::apply(host.clone(), vec![group]).expect("unary host"),
            Pattern::apply(pair.clone(), vec![a, b]).expect("binary pair"),
        )
        .expect("bound captures");
        let mut operands: Vec<_> = (0..length)
            .map(|index| ctx.symb(&format!("v{index}")).map(|symbol| symbol.to_expr()))
            .collect::<Result<_, _>>()?;
        let first = operands.first().expect("nonempty input").clone();
        let remainder = if sum {
            Expr::sum(operands.iter().skip(1).cloned().collect())
        } else {
            Expr::product(operands.iter().skip(1).cloned().collect())
        };
        if reverse {
            operands.reverse();
        }
        let input = host
            .call(vec![if sum { Expr::sum(operands) } else { Expr::product(operands) }])
            .expect("unary host");
        prop_assert_eq!(
            rule.apply_all(&input, usize::MAX, None),
            Ok(pair.call(vec![first, remainder]).expect("binary pair"))
        );
    }

    #[test]
    fn guards_can_select_grouped_root_assignments(sum in any::<bool>(), length in 3_usize..=6) {
        let ctx = Context::new()?;
        let pair = ctx.register_function("pair", UserFunction::new(2..=2)).expect("unbound name");
        let operands: Vec<_> = (0..length)
            .map(|index| ctx.symb(&format!("v{index}")).map(|symbol| symbol.to_expr()))
            .collect::<Result<_, _>>()?;
        let left = if sum {
            Expr::sum(operands.iter().take(2).cloned().collect())
        } else {
            Expr::product(operands.iter().take(2).cloned().collect())
        };
        let right = if sum {
            Expr::sum(operands.iter().skip(2).cloned().collect())
        } else {
            Expr::product(operands.iter().skip(2).cloned().collect())
        };
        let input = if sum { Expr::sum(operands) } else { Expr::product(operands) };
        let a = Pattern::capture("a", None);
        let b = Pattern::capture("b", None);
        let group = if sum {
            Pattern::sum(vec![a.clone(), b.clone()])
        } else {
            Pattern::product(vec![a.clone(), b.clone()])
        };
        let rule = RewriteRule::new(
            group,
            Pattern::apply(pair.clone(), vec![a, b]).expect("binary pair"),
        )
        .expect("bound captures");
        let mut guard = |captures: &HashMap<&str, Expr>| {
            captures.get("a") == Some(&left) && captures.get("b") == Some(&right)
        };
        prop_assert_eq!(
            rule.apply_all(&input, usize::MAX, Some(&mut guard)),
            Ok(pair.call(vec![left.clone(), right.clone()]).expect("binary pair"))
        );
    }

    #[test]
    fn grouped_captures_respect_bindings_from_other_arguments(sum in any::<bool>(), length in 3_usize..=6) {
        let ctx = Context::new()?;
        let host = ctx.register_function("host", UserFunction::new(2..=2)).expect("unbound name");
        let pair = ctx.register_function("pair", UserFunction::new(2..=2)).expect("unbound name");
        let operands: Vec<_> = (0..length)
            .map(|index| ctx.symb(&format!("v{index}")).map(|symbol| symbol.to_expr()))
            .collect::<Result<_, _>>()?;
        let known = if sum {
            Expr::sum(operands.iter().take(2).cloned().collect())
        } else {
            Expr::product(operands.iter().take(2).cloned().collect())
        };
        let remaining = if sum {
            Expr::sum(operands.iter().skip(2).cloned().collect())
        } else {
            Expr::product(operands.iter().skip(2).cloned().collect())
        };
        let all = if sum { Expr::sum(operands) } else { Expr::product(operands) };
        let a = Pattern::capture("a", None);
        let b = Pattern::capture("b", None);
        let group = if sum {
            Pattern::sum(vec![a.clone(), b.clone()])
        } else {
            Pattern::product(vec![a.clone(), b.clone()])
        };
        let rule = RewriteRule::new(
            Pattern::apply(host.clone(), vec![group, a.clone()]).expect("binary host"),
            Pattern::apply(pair.clone(), vec![a, b]).expect("binary pair"),
        )
        .expect("bound captures");
        prop_assert_eq!(
            rule.apply_all(&host.call(vec![all, known.clone()]).expect("binary host"), usize::MAX, None),
            Ok(pair.call(vec![known, remaining]).expect("binary pair"))
        );
    }

    #[test]
    fn power_patterns_can_bind_positive_integer_blocks(block in 2_i64..=9, tail in 0_i64..=1) {
        let ctx = Context::new()?;
        let x = ctx.symb("x")?;
        let u = ctx.symb("u")?;
        let desired = x.pow(block);
        let rule = RewriteRule::new(Pattern::capture("a", None).pow(2), Pattern::exact(u))
            .expect("literal replacement");
        let mut guard = |captures: &HashMap<&str, Expr>| captures.get("a") == Some(&desired);
        let input = x.pow(block.saturating_mul(2).saturating_add(tail));
        let expected = if tail == 0 {
            u.to_expr()
        } else {
            Expr::product(vec![u.into(), x.into()])
        };
        prop_assert_eq!(rule.apply_all(&input, usize::MAX, Some(&mut guard)), Ok(expected));
    }

    #[test]
    fn multiplicative_captures_can_bind_intermediate_power_partitions(left in 2_i64..=6, right in 2_i64..=6) {
        let ctx = Context::new()?;
        let x = ctx.symb("x")?;
        let pair = ctx.register_function("pair", UserFunction::new(2..=2)).expect("unbound name");
        let a = Pattern::capture("a", None);
        let b = Pattern::capture("b", None);
        let rule = RewriteRule::new(
            Pattern::product(vec![a.clone(), b.clone()]),
            Pattern::apply(pair.clone(), vec![a, b]).expect("binary pair"),
        )
        .expect("bound captures");
        let wanted_left = x.pow(left);
        let wanted_right = x.pow(right);
        let mut guard = |captures: &HashMap<&str, Expr>| {
            captures.get("a") == Some(&wanted_left) && captures.get("b") == Some(&wanted_right)
        };
        prop_assert_eq!(
            rule.apply_all(&x.pow(left.saturating_add(right)), usize::MAX, Some(&mut guard)),
            Ok(pair.call(vec![wanted_left.clone(), wanted_right.clone()]).expect("binary pair"))
        );
    }

    #[test]
    fn block_search_exhaustion_returns_no_partial_output(exponent in 4_i64..=30) {
        let ctx = Context::new()?;
        let x = ctx.symb("x")?;
        let rule = RewriteRule::new(
            Pattern::capture("a", None).pow(2),
            Pattern::exact(ctx.symb("u")?),
        )
        .expect("literal replacement");
        let mut reject = |_: &HashMap<&str, Expr>| false;
        prop_assert_eq!(
            rule.apply_all(&x.pow(exponent), 1, Some(&mut reject)),
            Err(RewriteError::MatchLimitExceeded { limit: 1 })
        );
    }

    #[test]
    fn generic_root_rules_consume_explicit_power_groups(copies in 2_i64..=16) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let output = context.register_function("output", UserFunction::new(1..=1)).expect("unbound function");
        let capture = Pattern::capture("a", None);
        let rule = RewriteRule::new(capture.sqrt(), Pattern::apply(output.clone(), vec![capture]).expect("unary template")).expect("bound capture");
        let input = x.pow(Number::from_rational(RationalType::new(copies, 2)));
        let expected = if copies == 2 { input.clone() } else { output.call(vec![x.into()]).expect("unary call").pow(copies) };
        prop_assert_eq!(rule.apply_all(&input, usize::MAX, None), Ok(expected));
    }

    #[test]
    fn fractional_templates_keep_rational_remainders(copies in 3_i64..=12, remainder in 0_i64..=1, inverse in bool::ANY) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let output = context.register_function("output", UserFunction::new(1..=1)).expect("unbound function");
        let capture = Pattern::capture("a", None);
        let numerator = if inverse { -2 } else { 2 };
        let rule = RewriteRule::new(capture.pow(Expr::number(Number::from_rational(RationalType::new(numerator, 3)))), Pattern::apply(output.clone(), vec![capture]).expect("unary template")).expect("bound capture");
        let total = copies.saturating_mul(2).saturating_add(remainder);
        let input = x.pow(Number::from_rational(RationalType::new(if inverse { total.saturating_neg() } else { total }, 3)));
        let replacement = output.call(vec![x.into()]).expect("unary call").pow(copies);
        let residual = x.pow(Number::from_rational(RationalType::new(remainder, 3)));
        let expected = if remainder == 0 { replacement } else if inverse { Div::div(replacement, residual) } else { Mul::mul(replacement, residual) };
        prop_assert_eq!(rule.apply_all(&input, usize::MAX, None), Ok(expected));
    }

    #[test]
    fn integral_templates_consume_groups_inside_fractional_powers(exponent in 3_i64..=17) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let output = context.register_function("output", UserFunction::new(1..=1)).expect("unbound function");
        let capture = Pattern::capture("a", None);
        let rule = RewriteRule::new(capture.pow(2), Pattern::apply(output.clone(), vec![capture]).expect("unary template")).expect("bound capture");
        let input = x.pow(Number::from_rational(RationalType::new(exponent, 2)));
        let copies = exponent.checked_div(4).expect("constant divisor");
        let remainder = exponent.checked_rem(4).expect("constant divisor");
        let mut factors = vec![];
        if copies > 0 { factors.push(output.call(vec![x.into()]).expect("unary call").pow(copies)); }
        match remainder {
            0 => {},
            1 => factors.push(x.sqrt()),
            2 => factors.push(x.into()),
            _ => {
                factors.push(output.call(vec![x.sqrt()]).expect("unary call"));
                factors.push(x.sqrt());
            }
        }
        let expected = Expr::product(factors);
        prop_assert_eq!(rule.apply_all(&input, usize::MAX, None), Ok(expected));
    }

    #[test]
    fn generic_rational_identity_preserves_both_complex_half_planes(copies in 3_i64..=9, remainder in 0_i64..=1, real in -3_i32..=3, imaginary in 1_i32..=3, lower in bool::ANY) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let capture = Pattern::capture("a", None);
        let pattern = capture.pow(Expr::number(Number::from_rational(RationalType::new(2, 3))));
        let rule = RewriteRule::new(pattern.clone(), pattern).expect("bound capture");
        let input = x.pow(Number::from_rational(RationalType::new(copies.saturating_mul(2).saturating_add(remainder), 3)));
        let rewritten = rule.apply_all(&input, usize::MAX, None).expect("unbounded search");
        let value = Expr::number(Number::from_complex(Number::from_float(f64::from(real)), Number::from_float(f64::from(if lower { imaginary.saturating_neg() } else { imaginary }))));
        let point = Substitution::new(x.into(), value);
        let before = point.apply_all(&input);
        let after = point.apply_all(&rewritten);
        let expected = before.as_number().expect("finite approximate powers fold");
        let actual = after.as_number().expect("finite approximate powers fold");
        let tolerance = Mul::mul(Number::from_float(1e-9), Add::add(Number::one(), expected.abs()));
        prop_assert!(Sub::sub(expected, actual).abs() < tolerance);
    }
}

#[test]
fn nested_reciprocal_templates_do_not_introduce_their_zero_exclusion() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation");
    let capture = Pattern::capture("a", None);
    let pattern = Div::div(Pattern::exact(1), Div::div(Pattern::exact(1), capture));
    let rule = RewriteRule::new(pattern, Pattern::exact(u)).expect("literal replacement");
    let input = x.pow(2);
    assert_eq!(rule.apply_all(&input, usize::MAX, None), Ok(input));
}

#[test]
fn the_last_native_power_candidate_advances_instead_of_repeating() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let expression = Div::div(Expr::number(1), x.pow(i64::MAX));
    let operands =
        AlgebraicOperands::new(&expression, false, 0).expect("multiplicative projection");
    let pattern = Div::div(Pattern::exact(1), Pattern::capture("a", None));
    let patterns = [pattern];
    let mut group = Group {
        patterns: &patterns,
        values: GroupValues::Counted(&operands),
        selected: Selected::Counted(vec![]),
        next: 1,
        projection: i64::MAX,
        combination: None,
        sum: false,
        start: 0,
        root: true,
    };
    let draw = group.draw(&patterns[0]);
    assert!(group.candidate(&patterns[0], &draw, &[]).is_none());
}

#[test]
fn normalized_roots_supply_exact_rational_groups_without_peeling_fractional_bases() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation");
    let capture = Pattern::capture("a", None);
    let rule = RewriteRule::new(
        capture.pow(Expr::number(Number::from_rational(RationalType::new(1, 3)))),
        Pattern::exact(u),
    )
    .expect("literal replacement");
    assert_eq!(
        rule.apply_all(&x.sqrt(), usize::MAX, None),
        Ok(Mul::mul(
            u,
            x.pow(Number::from_rational(RationalType::new(1, 6)))
        ))
    );
    let branch =
        RewriteRule::new(capture.pow(2).sqrt(), Pattern::exact(u)).expect("literal replacement");
    let input = x.pow(Number::from_rational(RationalType::new(3, 2)));
    assert_eq!(branch.apply_all(&input, usize::MAX, None), Ok(input));
}

#[test]
fn one_candidate_handles_the_largest_rational_multiplicity() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let output = context
        .register_function("output", UserFunction::new(1..=1))
        .expect("unbound function");
    let capture = Pattern::capture("a", None);
    let rule = RewriteRule::new(
        capture.pow(Expr::number(Number::from_rational(RationalType::new(1, 3)))),
        Pattern::apply(output.clone(), vec![capture]).expect("unary template"),
    )
    .expect("bound capture");
    let input = x.pow(Number::from_rational(RationalType::new(i64::MAX, 3)));
    let expected = output
        .call(vec![x.into()])
        .expect("unary call")
        .pow(i64::MAX);
    assert_eq!(rule.apply_all(&input, 1, None), Ok(expected));
}
