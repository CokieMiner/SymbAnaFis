//! Exact scalar quotient identity, matching, capacity, and denominator domains.

use core::ops::{Add, Div, Mul};

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::{collection, prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, Expr, ExprView, Pattern, RewriteRule, Substitution, Substitutions},
};

proptest! {
    #[test]
    fn exact_scalar_division_shares_multiplicative_coefficients(
        numerator in 1_i64..=9,
        denominator in 1_u64..=9,
        imaginary in -4_i64..=4,
        negative in proptest::bool::ANY,
        base_kind in 0_usize..7,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let base = match base_kind {
            0 => x.to_expr(),
            1 => Add::add(x, y),
            2 => Mul::mul(x, y),
            3 => x.pow(3),
            4 => x.sin(),
            5 => Add::add(x.pow(2), x.pow(3)),
            _ => Div::div(x, y),
        };
        let scalar = Number::from_complex(
            Number::from_rational(RationalType::from_parts(
                if negative { numerator.saturating_neg() } else { numerator }, denominator,
            )),
            Number::from_int(imaginary),
        );
        let inverse = Div::div(Number::one(), &scalar);
        prop_assert_eq!(Div::div(&base, scalar), Mul::mul(Expr::number(inverse), &base));
    }

    #[test]
    fn denominator_coefficients_and_chained_divisions_share_one_node(
        numerator in 1_i64..=9,
        denominator in 1_u64..=9,
        coefficient in -9_i64..=9,
        imaginary in -3_i64..=3,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let scalar = Number::from_complex(
            Number::from_rational(RationalType::from_parts(numerator, denominator)),
            Number::from_int(imaginary),
        );
        let top = Mul::mul(Expr::number(coefficient), x);
        let quotient = Div::div(Number::from_int(coefficient), &scalar);
        let base = Div::div(x, y);
        let expected = Mul::mul(Expr::number(quotient), &base);
        prop_assert_eq!(&Div::div(&top, Mul::mul(Expr::number(scalar.clone()), y)), &expected);
        prop_assert_eq!(&Div::div(Div::div(&top, y), scalar.clone()), &expected);
        prop_assert_eq!(&Div::div(Div::div(&top, scalar.clone()), y), &expected);
        prop_assert_eq!(&Mul::mul(Div::div(&top, scalar), y.pow(-1)), &expected);
        prop_assert!(expected == base || matches!(expected.view(), ExprView::Product([_, shared_quotient]) if shared_quotient == &base));
    }

    #[test]
    fn weighted_quotients_share_their_base_and_collect_exact_coefficients(
        coefficients in collection::vec(-9_i64..=9, 2..=12),
        denominator in 1_i64..=9,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let base = Div::div(x, y);
        let mut terms = vec![];
        let mut total = 0_i64;
        for coefficient in coefficients {
            total = total.saturating_add(coefficient);
            let scalar = Expr::number(Number::from_rational(RationalType::new(coefficient, denominator)));
            let term = Div::div(Mul::mul(Expr::number(coefficient), x), Mul::mul(Expr::number(denominator), y));
            prop_assert_eq!(&term, &Mul::mul(&scalar, &base));
            prop_assert!(term == base || matches!(term.view(), ExprView::Product([stored, quotient]) if stored == &scalar && quotient == &base));
            terms.push(term);
        }
        let coefficient = Expr::number(Number::from_rational(RationalType::new(total, denominator)));
        prop_assert_eq!(Expr::sum(terms), Mul::mul(coefficient, base));
    }

    #[test]
    fn quotient_sources_match_the_shared_base_under_numeric_weights(
        numerator in -9_i64..=9,
        denominator in 1_i64..=9,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let u = context.symb("u")?;
        let base = Div::div(x, y);
        let input = Div::div(Mul::mul(Expr::number(numerator), x), Mul::mul(Expr::number(denominator), y));
        let coefficient = Expr::number(Number::from_rational(RationalType::new(numerator, denominator)));
        let expected = Mul::mul(coefficient, u);
        prop_assert_eq!(&Substitution::new(base.clone(), u.into()).apply_all(&input), &expected);
        let simultaneous = Substitutions::new([(base.clone(), u.into())]).expect("a single source cannot conflict");
        prop_assert_eq!(&simultaneous.apply_all(&input), &expected);
        let rule = RewriteRule::new(Pattern::exact(base), Pattern::exact(u)).expect("a literal replacement has no captures");
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(expected));
    }

    #[test]
    fn quotient_products_and_nested_denominators_preserve_independent_rational_values(
        first in -5_i64..=5,
        second in -5_i64..=5,
        first_denominator in 1_i64..=5,
        second_denominator in 1_i64..=5,
        x_value in -5_i64..=-1,
        y_value in 1_i64..=5,
        z_value in -5_i64..=-1,
        w_value in 1_i64..=5,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let z = context.symb("z")?;
        let w = context.symb("w")?;
        let left = Div::div(Mul::mul(Expr::number(first), x), Mul::mul(Expr::number(first_denominator), y));
        let right = Div::div(Mul::mul(Expr::number(second), z), Mul::mul(Expr::number(second_denominator), w));
        let coefficient = Number::from_rational(RationalType::new(first.saturating_mul(second), first_denominator.saturating_mul(second_denominator)));
        let expected = Mul::mul(Expr::number(coefficient), Div::div(Mul::mul(x, z), Mul::mul(y, w)));
        let product = Mul::mul(&left, &right);
        prop_assert_eq!(&product, &expected);
        let values = HashMap::from([
            (x, Expr::number(x_value)), (y, Expr::number(y_value)),
            (z, Expr::number(z_value)), (w, Expr::number(w_value)),
        ]);
        let first_value = Div::div(Number::from_int(first.saturating_mul(x_value)), Number::from_int(first_denominator.saturating_mul(y_value)));
        let second_value = Div::div(Number::from_int(second.saturating_mul(z_value)), Number::from_int(second_denominator.saturating_mul(w_value)));
        prop_assert_eq!(product.substitute_symbols(&values), Expr::number(Mul::mul(&first_value, &second_value)));
        if second != 0 {
            let nested = Div::div(left, right);
            prop_assert_eq!(nested.substitute_symbols(&values), Expr::number(Div::div(first_value, second_value)));
        }
    }

    #[test]
    fn exact_scalar_quotients_collect_terms_and_integer_powers(
        denominator in 2_i64..=7,
        exponent in 2_i64..=7,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let inverse = Div::div(Number::one(), Number::from_int(denominator));
        let quotient = Div::div(x, denominator);
        let product = Mul::mul(Expr::number(inverse.clone()), x);
        prop_assert_eq!(Add::add(&quotient, &product), Mul::mul(Expr::number(Mul::mul(Number::from_int(2), inverse)), x));
        prop_assert_eq!(quotient.pow(exponent), Div::div(x.pow(exponent), Expr::number(denominator).pow(exponent)));
    }

    #[test]
    fn quotient_and_coefficient_sources_share_rewrite_and_conflict_semantics(denominator in 2_i64..=19) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let v = context.symb("v")?;
        let quotient = Div::div(x, denominator);
        let inverse = Div::div(Number::one(), Number::from_int(denominator));
        let product = Mul::mul(Expr::number(inverse), x);
        let rule = RewriteRule::new(Pattern::exact(quotient.sin()), Pattern::exact(u))
            .expect("a literal replacement has no unbound captures");
        prop_assert_eq!(rule.apply_all(&product.sin(), None, None), Ok(u.to_expr()));
        prop_assert_eq!(Substitution::new(quotient.clone(), u.into()).apply_all(&product), u.to_expr());
        prop_assert!(Substitutions::new([(quotient.clone(), u.into()), (product.clone(), v.into())]).is_err());
        prop_assert!(Substitutions::new([(quotient, u.into()), (product, u.into())]).is_ok());
        let a = Pattern::capture("a", None);
        let generic = RewriteRule::new(Div::div(&a, denominator).sin(), a.cos())
            .expect("the replacement capture is bound");
        prop_assert_eq!(generic.apply_all(&Div::div(x, denominator).sin(), None, None), Ok(x.cos()));
    }
}

#[test]
fn nonexact_and_zero_denominators_remain_explicit() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    for denominator in [
        Expr::number(0),
        Expr::number(0.0),
        Expr::number(-0.0),
        Expr::number(3.0),
        Expr::number(f64::INFINITY),
        Expr::number(f64::from_bits(0x7ff8_0000_0000_0001)),
        Expr::number(Number::from_complex(
            Number::from_int(2),
            Number::from_float(1.0),
        )),
    ] {
        assert!(
            matches!(Div::div(x, &denominator).view(), ExprView::Div(top, bottom)
            if top == &x.to_expr() && bottom == &denominator)
        );
    }
    assert!(matches!(
        Div::div(Expr::number(f64::from_bits(1)), 2).view(),
        ExprView::Div(..)
    ));
    let approximate = Mul::mul(Expr::number(0.1), x);
    assert!(matches!(Div::div(approximate, 3).view(), ExprView::Div(..)));
}

#[test]
fn unrepresentable_exact_coefficients_remain_quotients() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    for scalar in [
        Number::from_rational(RationalType::from_parts(1, u64::MAX)),
        Number::from_rational(RationalType::from_parts(-1, u64::MAX)),
        Number::from_complex(Number::from_int(i64::MAX), Number::one()),
    ] {
        let denominator = Expr::number(scalar);
        assert!(
            matches!(Div::div(x, &denominator).view(), ExprView::Div(top, bottom)
            if top == &x.to_expr() && bottom == &denominator)
        );
    }
    let numerator = Mul::mul(
        Expr::number(Number::from_rational(RationalType::from_parts(i64::MAX, 2))),
        x,
    );
    let denominator = Expr::number(Number::from_rational(RationalType::new(1, 3)));
    assert!(
        matches!(Div::div(&numerator, &denominator).view(), ExprView::Div(top, bottom)
        if top == &numerator && bottom == &denominator)
    );
    assert_eq!(
        Div::div(x, i64::MIN),
        Mul::mul(
            Expr::number(Number::from_rational(RationalType::from_parts(
                -1,
                1_u64 << 63
            ))),
            x
        )
    );
}

#[test]
fn exact_scaling_preserves_symbolic_zero_exclusions() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let y = context.symb("y").expect("symbol allocation");
    let half = Expr::number(Number::from_rational(RationalType::new(1, 2)));
    let scaled = Div::div(x, Mul::mul(Expr::number(2), y));
    assert_eq!(scaled, Div::div(Mul::mul(&half, x), y));
    assert!(scaled.substitute_symbol(y, &Expr::number(0)).fold(false, |found, node|
        found || matches!(node.view(), ExprView::Div(_, denominator) if denominator.is_zero_num())));
    let cancellation = Div::div(x, Mul::mul(Expr::number(2), x));
    assert_ne!(cancellation, half);
    assert!(cancellation.substitute_symbol(x, &Expr::number(0)).fold(false, |found, node|
        found || matches!(node.view(), ExprView::Div(_, denominator) if denominator.is_zero_num())));
    let nested = Div::div(Expr::number(1), Div::div(x, y));
    assert!(
        matches!(nested.view(), ExprView::Div(_, denominator) if matches!(denominator.view(), ExprView::Div(..)))
    );
    assert_eq!(
        Div::div(Expr::number(1), Div::div(x, 2)),
        Div::div(Expr::number(2), x)
    );
    assert_eq!(
        Div::div(x, Expr::i()),
        Mul::mul(Mul::mul(Expr::number(-1), Expr::i()), x)
    );
    assert_eq!(
        Expr::product(vec![half, x.into(), Expr::number(2)]),
        x.to_expr()
    );
}

#[test]
fn weighted_nested_quotients_normalize_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| {
            let context = Context::new();
            let x = context.symb("x").expect("symbol allocation");
            let y = context.symb("y").expect("symbol allocation");
            let mut left = x.to_expr();
            let mut right = y.to_expr();
            for _ in 0..4096 {
                left = Div::div(Expr::number(2), left);
                right = Div::div(Expr::number(3), right);
            }
            let product = Expr::product(vec![Expr::number(5), left.clone(), right]);
            let powered = left.pow(3);
            assert!(product.contains_symbol(x));
            assert!(product.contains_symbol(y));
            assert!(powered.contains_symbol(x));
            assert!(product.node_count() < 12_400);
            assert!(powered.node_count() < 12_400);
        })
        .expect("the small-stack worker starts")
        .join()
        .expect("weighted quotient normalization and destruction remain iterative");
}
