use crate::{Diff, DiffError, Expr, symb};

#[test]
fn test_builder_configuration() {
    // Test defaults - use a variable symbol for differentiation
    let x = symb("x");
    let diff = Diff::new();
    let res = diff.differentiate(&x.pow(2.0), &x).unwrap();
    assert_eq!(format!("{}", res), "2*x");

    // Test domain_safe
    let _diff_safe = Diff::new().domain_safe(true);

    // Test known_symbols parameter in diff_str
    let res = Diff::new().diff_str("alpha*x", "x", &["alpha"]).unwrap();
    assert_eq!(res, "alpha");
}

#[test]
fn test_custom_derivatives() {
    use crate::core::unified_context::UserFunction;

    // Custom rule: d/dx[my_func(u)] = 3*u^2 * u'
    // Define my_func(u) with ∂my_func/∂u = 3*u^2
    let my_func = UserFunction::new(1..=1)
        .partial(0, |args: &[std::sync::Arc<Expr>]| {
            // ∂my_func/∂u = 3*u^2
            Expr::number(3.0) * Expr::from(&args[0]).pow(2.0)
        })
        .expect("valid arg");

    let diff = Diff::new().user_fn("my_func", my_func);

    // Test: my_func(x) -> 3*x^2 * 1 = 3x^2
    let x = symb("x");
    let expr = Expr::func("my_func", x.to_expr());
    let res = diff.differentiate(&expr, &x).unwrap();
    assert_eq!(format!("{}", res), "3*x^2");

    // Test chain rule: my_func(x^2) -> 3*(x^2)^2 * (2x) = 6x^5
    let expr2 = Expr::func("my_func", x.clone().pow(2.0));
    let res2 = diff.differentiate(&expr2, &x).unwrap();

    assert_eq!(format!("{}", res2), "6*x^5");
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
        .differentiate(&deeply_nested, &x)
        .expect("Should pass within default limits");

    // Should fail with strict limits
    let diff_strict = Diff::new().max_depth(5);
    let res = diff_strict.differentiate(&deeply_nested, &x);

    assert!(matches!(res, Err(DiffError::MaxDepthExceeded)));
}

#[test]
fn test_node_limits() {
    // Create broad tree with different bases to avoid like-term combination
    // Use sin(x^n) for each n to create unique terms
    let x = symb("x");
    let mut terms: Vec<Expr> = Vec::new();
    for i in 1..=20 {
        terms.push(Expr::func("sin", x.clone().pow(i as f64)));
    }
    let broad = Expr::sum(terms);

    let diff_strict = Diff::new().max_nodes(50);
    let res = diff_strict.differentiate(&broad, &x);

    assert!(matches!(res, Err(DiffError::MaxNodesExceeded)));
}

#[test]
fn test_symbol_method_chaining() {
    let x = symb("x");

    // sin(x)^2 + cos(x)^2
    // Symbol.pow(2.0) works and returns Expr, but sin() returns Expr, so use pow_of
    let expr = x.clone().sin().pow(2.0) + x.clone().cos().pow(2.0);

    // Accept either ordering (canonical ordering now deferred to simplify)
    let display = format!("{}", expr);
    assert!(display == "cos(x)^2 + sin(x)^2" || display == "sin(x)^2 + cos(x)^2");

    let diff = Diff::new();
    let res = diff.differentiate(&expr, &x).unwrap();

    // limit of differentiation: 2sin(x)cos(x) - 2cos(x)sin(x) = 0
    assert_eq!(format!("{}", res), "0");
}

#[test]
fn test_advanced_functions() {
    let x = symb("x");

    // gamma(x) calls Symbol::gamma -> Expr
    let expr = x.clone().gamma();
    let diff = Diff::new();
    let res = diff.differentiate(&expr, &x).unwrap();

    // Ensure it runs without error
    assert!(!format!("{}", res).is_empty());
}

#[test]
fn test_api_reusability() {
    // Builder should be reusable (cloneable)
    let diff = Diff::new();

    // Use known_symbols parameter in diff_str
    let _res1 = diff.diff_str("alpha*x", "x", &["alpha"]).unwrap();
    let _res2 = diff.diff_str("alpha*x^2", "x", &["alpha"]).unwrap();

    // Clone
    let diff2 = diff.clone().domain_safe(true);
    let _res3 = diff2.diff_str("sqrt(x^2)", "x", &[]).unwrap();
}

#[test]
fn test_error_handling() {
    use crate::core::unified_context::UserFunction;

    // Pass closure directly
    let user_fn = UserFunction::new(1..=1)
        .partial(0, |_: &[std::sync::Arc<Expr>]| Expr::number(0.0))
        .expect("valid arg");
    let diff = Diff::new().user_fn("my_func", user_fn);

    // Should fail because "my_func" is both custom func and in known_symbols
    let res = diff.diff_str("my_func(x)", "x", &["my_func"]);
    assert!(matches!(res, Err(DiffError::NameCollision { .. })));
}
