use alloc::{collections::BTreeSet, sync::Arc};

use crate::collections::HashSet;

use super::super::{
    BuiltinFunction, FunctionDefinition, FunctionIdentity, FunctionName, UserFunction,
};

#[test]
fn builtin_handles_share_static_identity_without_acquiring_custom_rules() {
    let builtin = BuiltinFunction::Sin.function();
    let textual = FunctionName::builtin("sin").expect("sine is registered");
    let local = FunctionDefinition::local(Arc::from("sin"), UserFunction::new(1..=1));
    let other_local = FunctionDefinition::local(Arc::from("sin"), UserFunction::new(1..=1));
    let cosine = FunctionName::builtin("cos").expect("cosine is registered");
    assert_eq!(
        builtin, textual,
        "typed and textual builtin identities agree"
    );
    assert!(
        matches!(textual.0, FunctionIdentity::Builtin(BuiltinFunction::Sin)),
        "textual builtin construction does not own an interned name"
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
        FunctionDefinition::get(&builtin).is_some_and(|definition| definition.arity() == 1),
        "the static definition preserves unary arity"
    );
    assert!(
        FunctionDefinition::get(&local).is_none(),
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
    ] {
        assert_eq!(
            FunctionName::builtin(canonical),
            FunctionName::builtin(alias),
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
        FunctionName::builtin("bessel_j").expect("the Bessel alias is registered"),
        "a context-local alias spelling stays isolated from the builtin"
    );
    assert!(
        FunctionDefinition::get(&local).is_none(),
        "local aliases do not inherit builtin folding rules"
    );
}
