//! Builtin function identities and their alias spellings.

use alloc::{collections::BTreeSet, sync::Arc};

use crate::collections::HashSet;

use super::super::super::{BuiltinFunction, Function, FunctionDefinition, UserFunction};

#[test]
fn builtin_handles_share_static_identity_without_acquiring_custom_rules() {
    let builtin = BuiltinFunction::Sin.function();
    let textual = Function::builtin("sin").expect("sine is registered");
    let local = FunctionDefinition::local(Arc::from("sin"), UserFunction::new(1..=1));
    let other_local = FunctionDefinition::local(Arc::from("sin"), UserFunction::new(1..=1));
    let cosine = Function::builtin("cos").expect("cosine is registered");
    assert_eq!(
        builtin, textual,
        "typed and textual builtin identities agree"
    );
    assert_eq!(textual.as_str(), "sin", "static spelling remains available");
    assert!(
        textual.definition().is_none(),
        "builtins have no custom definition"
    );
    assert!(
        local.definition().is_some(),
        "custom handles own their definition"
    );
    assert!(
        BuiltinFunction::of(&builtin).is_some_and(|identity| identity.definition().arity() == 1),
        "the static definition preserves unary arity"
    );
    assert!(
        BuiltinFunction::of(&local).is_none(),
        "a matching custom spelling cannot inherit builtin folds"
    );
    let identities = [builtin, textual, local, other_local, cosine];
    let hashed: HashSet<_> = identities.iter().cloned().collect();
    let ordered: BTreeSet<_> = identities.iter().cloned().collect();
    assert_eq!(
        hashed.len(),
        4,
        "only the two builtin sine handles are equal"
    );
    assert_eq!(
        ordered.len(),
        hashed.len(),
        "ordering agrees with identity equality"
    );
    for left in &identities {
        for right in &identities {
            assert_eq!(
                left.cmp(right).is_eq(),
                left == right,
                "same-spelling namespaces preserve the Eq/Ord contract"
            );
        }
    }
}

#[test]
fn numeric_spelling_aliases_resolve_to_the_same_builtin_identity() {
    for (canonical, alias) in [
        ("besselj", "bessel_j"),
        ("bessely", "bessel_y"),
        ("besseli", "bessel_i"),
        ("besselk", "bessel_k"),
        ("lambertw", "lambert_w"),
        ("heaviside", "step"),
        ("mod", "modulo"),
    ] {
        assert_eq!(
            Function::builtin(canonical),
            Function::builtin(alias),
            "{alias} should preserve the canonical builtin identity"
        );
    }
    assert!(
        ["ynm", "lambert_w0", "lambert_wm1"]
            .iter()
            .all(|name| BuiltinFunction::from_name(name).is_none()),
        "unsupported spellings have no builtin identities"
    );
    let local = FunctionDefinition::local(Arc::from("bessel_j"), UserFunction::new(2..=2));
    assert_ne!(
        local,
        Function::builtin("bessel_j").expect("the Bessel alias is registered"),
        "a context-local alias spelling stays isolated from the builtin"
    );
    assert!(
        BuiltinFunction::of(&local).is_none(),
        "local aliases do not inherit builtin folding rules"
    );
}

#[test]
fn same_named_custom_identities_order_by_registration() {
    let first = FunctionDefinition::local(Arc::from("f"), UserFunction::new(1..=1));
    let second = FunctionDefinition::local(Arc::from("f"), UserFunction::new(1..=1));
    assert_ne!(first, second, "each registration is a distinct identity");
    assert_eq!(
        first.cmp(&second),
        core::cmp::Ordering::Less,
        "registration order breaks the tie, not the heap address"
    );
    assert_eq!(
        second.cmp(&first),
        core::cmp::Ordering::Greater,
        "the order is total"
    );
}

#[test]
fn component_step_selection_and_remainder_builtins_fold_exact_literals() {
    use num_anafis::{Number, RationalType};

    use super::super::super::{Expr, ExprView};

    let three_plus_four_i = Expr::number(Number::from_complex(
        Number::from_int(3),
        Number::from_int(4),
    ));
    let three_minus_four_i = Expr::number(Number::from_complex(
        Number::from_int(3),
        Number::from_int(-4),
    ));
    assert_eq!(three_plus_four_i.re(), Expr::number(3));
    assert_eq!(three_plus_four_i.im(), Expr::number(4));
    assert_eq!(three_plus_four_i.conj(), three_minus_four_i);
    assert_eq!(Expr::i().re(), Expr::number(0));
    assert_eq!(Expr::i().im(), Expr::number(1));
    assert_eq!(Expr::i().conj(), -Expr::i());

    let half = Expr::number(Number::from_rational(RationalType::new(1, 2)));
    let two_thirds = Expr::number(Number::from_rational(RationalType::new(2, 3)));
    let three_halves = Expr::number(Number::from_rational(RationalType::new(3, 2)));
    let five_halves = Expr::number(Number::from_rational(RationalType::new(5, 2)));
    assert_eq!(Expr::number(-2).heaviside(), Expr::number(0));
    assert_eq!(Expr::number(0).heaviside(), half);
    assert_eq!(Expr::number(3).heaviside(), Expr::number(1));
    assert!(matches!(
        Expr::i().heaviside().view(),
        ExprView::Function { .. }
    ));

    assert_eq!(Expr::number(3).hypot(4), Expr::number(5));
    assert_eq!(
        Expr::number(Number::from_rational(RationalType::new(3, 5)))
            .hypot(Number::from_rational(RationalType::new(4, 5))),
        Expr::number(1)
    );
    assert!(matches!(
        Expr::number(1).hypot(1).view(),
        ExprView::Function { .. }
    ));

    assert_eq!(half.max(two_thirds.clone()), two_thirds);
    assert_eq!(Expr::number(-3).min(5), Expr::number(-3));
    assert_eq!(three_halves.clamp(0, 1), Expr::number(1));
    assert_eq!(Expr::number(-7).modulo(3), Expr::number(2));
    assert_eq!(five_halves.modulo(1), half);
    assert_eq!(Expr::number(-7).rem(3), Expr::number(-1));
    assert!(matches!(
        Expr::number(7).modulo(0).view(),
        ExprView::Function { .. }
    ));
}

#[test]
fn component_step_selection_and_remainder_builtins_prove_domains() {
    use core::ops::Add;

    use super::super::super::{Context, Expr, Property, Truth};

    let context = Context::new().expect("context");
    let z = context.symb("z").expect("z");
    let r = context.symb("r").expect("r");
    let p = context.symb("p").expect("p");
    r.set_properties(&[Property::Real]).expect("real");
    p.set_properties(&[Property::Positive]).expect("positive");

    assert_eq!(z.re().properties().get(Property::Real), Truth::True);
    assert_eq!(z.im().properties().get(Property::Real), Truth::True);
    assert_eq!(r.im().properties().get(Property::Zero), Truth::True);
    assert_eq!(
        Add::add(r, Expr::i())
            .im()
            .properties()
            .get(Property::Nonzero),
        Truth::True
    );
    assert_eq!(
        Add::add(r, Expr::i())
            .conj()
            .properties()
            .get(Property::Real),
        Truth::False
    );

    assert_eq!(
        r.heaviside().properties().get(Property::Rational),
        Truth::True
    );
    assert_eq!(
        r.heaviside().properties().get(Property::Nonnegative),
        Truth::True
    );
    assert_eq!(
        r.abs().heaviside().properties().get(Property::Positive),
        Truth::True
    );
    assert_eq!(
        p.heaviside().properties().get(Property::Integer),
        Truth::True
    );

    assert_eq!(p.hypot(r).properties().get(Property::Positive), Truth::True);
    assert_eq!(p.max(r).properties().get(Property::Positive), Truth::True);
    assert_eq!(
        r.clamp(p, Add::add(p, 1))
            .properties()
            .get(Property::Positive),
        Truth::True
    );
    assert_eq!(
        r.modulo(p).properties().get(Property::Nonnegative),
        Truth::True
    );
    assert_eq!(r.modulo(0).properties().get(Property::Finite), Truth::False);
    assert_eq!(
        p.rem(p).properties().get(Property::Nonnegative),
        Truth::True
    );
}
