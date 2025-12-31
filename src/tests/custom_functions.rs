use crate::Expr;
use crate::core::unified_context::Context;

#[test]
fn test_context_registration() {
    use crate::core::unified_context::UserFunction;

    // times_two(x) = 2*x  (body is symbolic Expr)
    let user_fn = UserFunction::new(1..=1)
        .body(|args| 2.0 * (*args[0]).clone())
        .partial(0, |_args| Expr::number(2.0))
        .expect("valid arg");

    let ctx = Context::new().with_function("times_two", user_fn);

    // Verify the function is registered
    assert!(ctx.has_function("times_two"));
}

#[test]
fn test_context_isolation() {
    use crate::core::unified_context::UserFunction;

    // f(x) = 2*x
    let user_fn = UserFunction::new(1..=1)
        .body(|args| 2.0 * (*args[0]).clone())
        .partial(0, |_args| Expr::number(2.0))
        .expect("valid arg");

    let ctx1 = Context::new().with_function("f", user_fn);
    let ctx2 = Context::new();

    assert!(ctx1.has_function("f"));
    assert!(!ctx2.has_function("f"));
}

#[test]
fn test_compiled_evaluation_with_context() {
    use crate::core::unified_context::UserFunction;

    // times_two(x) = 2*x
    let user_fn = UserFunction::new(1..=1)
        .body(|args| 2.0 * (*args[0]).clone())
        .partial(0, |_args| Expr::number(2.0))
        .expect("valid arg");

    let ctx = Context::new().with_function("times_two", user_fn);

    // Expression: times_two(x) + 1
    let x = crate::symb("x");
    let expr = crate::Expr::func("times_two", x) + crate::Expr::number(1.0);

    // Compile with context
    let evaluator = crate::CompiledEvaluator::compile(&expr, &["x"], Some(&ctx))
        .expect("Compilation should succeed");

    // Evaluate at x = 10.0 -> times_two(10) + 1 = 20 + 1 = 21
    let result = evaluator.evaluate(&[10.0]);
    assert!(
        (result - 21.0).abs() < 1e-10,
        "Expected 21.0, got {}",
        result
    );
}

#[test]
fn test_compiled_evaluation_missing_context() {
    // Same expression but compile WITHOUT context
    let x = crate::symb("x");
    let expr = crate::Expr::func("times_two", x) + crate::Expr::number(1.0);

    let result = crate::CompiledEvaluator::compile(&expr, &["x"], None);
    assert!(result.is_err(), "Should fail to compile unknown function");
}

#[test]
fn test_arity_check() {
    use crate::core::unified_context::UserFunction;

    // add2(x, y) = x + y
    // Note: the body must handle the case when called with wrong arity during expansion
    let user_fn = UserFunction::new(2..=2)
        .body(|args| {
            if args.len() >= 2 {
                (*args[0]).clone() + (*args[1]).clone()
            } else {
                Expr::number(f64::NAN) // Fallback for invalid arity
            }
        })
        .partial(0, |_args| Expr::number(1.0))
        .expect("valid arg")
        .partial(1, |_args| Expr::number(1.0))
        .expect("valid arg");

    let ctx = Context::new().with_function("add2", user_fn);

    // Correct arity: add2(x, y)
    let x = crate::symb("x");
    let y = crate::symb("y");
    let expr_ok = crate::Expr::func_multi("add2", vec![x.into(), y.into()]);

    let evaluator = crate::CompiledEvaluator::compile(&expr_ok, &["x", "y"], Some(&ctx))
        .expect("Should compile with correct arity");
    let res = evaluator.evaluate(&[2.0, 3.0]);
    assert!((res - 5.0).abs() < 1e-10, "Expected 5.0, got {}", res);

    // Incorrect arity: add2(x)
    // The function won't be expanded because arity doesn't match,
    // but the compiler should still fail because the external function call remains
    let expr_bad = crate::Expr::func_multi("add2", vec![x.into()]);
    let result = crate::CompiledEvaluator::compile(&expr_bad, &["x"], Some(&ctx));
    assert!(result.is_err(), "Should fail with invalid arity");
}
