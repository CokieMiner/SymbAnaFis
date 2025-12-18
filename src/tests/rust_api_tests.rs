use crate::{Diff, DiffError, Expr, symb};

#[test]
fn test_builder_configuration() {
    // Test defaults - use a variable symbol for differentiation
    let x = symb("x");
    let diff = Diff::new();
    let res = diff.differentiate(x.clone().pow(2.0), &x).unwrap();
    assert_eq!(format!("{}", res), "2x");

    // Test domain_safe
    let _diff_safe = Diff::new().domain_safe(true);

    // Test fixed_var with Symbol
    let a = symb("a");
    let diff_fixed = Diff::new().fixed_var(&a);
    let res = diff_fixed.diff_str("a*x", "x").unwrap();
    assert_eq!(res, "a");
}

#[test]
fn test_custom_derivatives() {
    // Custom rule: d/dx[my_func(u)] = 3*u^2 * u'
    // Pass closure directly, not Arc::new(closure)
    let my_deriv = |inner: &Expr, _var: &str, inner_prime: &Expr| -> Expr {
        // 3 * inner^2 * inner_prime
        Expr::number(3.0) * inner.clone().pow_of(2.0) * inner_prime.clone()
    };

    let diff = Diff::new().custom_derivative("my_func", my_deriv);

    // Test: my_func(x) -> 3*x^2 * 1 = 3x^2
    let x = symb("x");
    let expr = Expr::func("my_func", x.into());
    let res = diff.differentiate(expr, &x).unwrap();
    assert_eq!(format!("{}", res), "3x^2");

    // Test chain rule: my_func(x^2) -> 3*(x^2)^2 * (2x) = 6x^5
    let expr2 = Expr::func("my_func", x.clone().pow(2.0));
    let res2 = diff.differentiate(expr2, &x).unwrap();

    assert_eq!(format!("{}", res2), "6x^5");
}

#[test]
fn test_recursion_limits() {
    let x = symb("x");
    let mut deeply_nested: Expr = x.into();
    // Reduce depth to avoid stack overflow in default run, but enough to trigger limit
    for _ in 0..20 {
        deeply_nested = deeply_nested.sin();
    }

    // Should pass with default/high limits
    let diff = Diff::new();
    let _ = diff
        .differentiate(deeply_nested.clone(), &x)
        .expect("Should pass within default limits");

    // Should fail with strict limits
    let diff_strict = Diff::new().max_depth(5);
    let res = diff_strict.differentiate(deeply_nested, &x);

    assert!(matches!(res, Err(DiffError::MaxDepthExceeded)));
}

#[test]
fn test_node_limits() {
    // Create broad tree
    let x = symb("x");
    let mut broad: Expr = x.into();
    for _ in 0..10 {
        broad = broad.clone() + broad.clone();
    }
    // 2^10 terms roughly

    let diff_strict = Diff::new().max_nodes(50);
    let res = diff_strict.differentiate(broad, &x);

    assert!(matches!(res, Err(DiffError::MaxNodesExceeded)));
}

#[test]
fn test_symbol_method_chaining() {
    let x = symb("x");

    // sin(x)^2 + cos(x)^2
    // Symbol.pow(2.0) works and returns Expr, but sin() returns Expr, so use pow_of
    let expr = x.clone().sin().pow_of(2.0) + x.clone().cos().pow_of(2.0);

    assert_eq!(format!("{}", expr), "cos(x)^2 + sin(x)^2");

    let diff = Diff::new();
    let res = diff.differentiate(expr, &x).unwrap();

    // limit of differentiation: 2sin(x)cos(x) - 2cos(x)sin(x) = 0
    assert_eq!(format!("{}", res), "0");
}

#[test]
fn test_advanced_functions() {
    let x = symb("x");

    // gamma(x) calls Symbol::gamma -> Expr
    let expr = x.clone().gamma();
    let diff = Diff::new();
    let res = diff.differentiate(expr, &x).unwrap();

    // Ensure it runs without error
    assert!(!format!("{}", res).is_empty());
}

#[test]
fn test_api_reusability() {
    // Builder should be reusable (cloneable)
    let a = symb("a");
    let diff = Diff::new().fixed_var(&a);

    let _res1 = diff.diff_str("a*x", "x").unwrap();
    let _res2 = diff.diff_str("a*x^2", "x").unwrap();

    // Clone
    let diff2 = diff.clone().domain_safe(true);
    let _res3 = diff2.diff_str("sqrt(x^2)", "x").unwrap();
}

#[test]
fn test_error_handling() {
    // Pass closure directly
    let my_func = symb("my_func");
    let diff = Diff::new()
        .custom_derivative("my_func", |_, _, _| Expr::number(0.0))
        .fixed_var(&my_func);

    // Should fail because "my_func" is both custom func and fixed var
    let res = diff.diff_str("my_func(x)", "x");
    assert!(matches!(res, Err(DiffError::NameCollision { .. })));
}
