use crate::Expr;
use crate::functions::{FunctionContext, FunctionDefinition};

/// Helper to create a simple function definition: f(x) = x * 2.0
fn make_times_two() -> FunctionDefinition {
    FunctionDefinition {
        name: "times_two",
        arity: 1..=1,
        eval: |args| args.first().map(|x| x * 2.0),
        derivative: |_, _| Expr::number(2.0),
    }
}

/// Helper to create a simple function definition: add(a, b) = a + b
fn make_add_custom() -> FunctionDefinition {
    FunctionDefinition {
        name: "add_custom",
        arity: 2..=2,
        eval: |args| {
            if args.len() == 2 {
                Some(args[0] + args[1])
            } else {
                None
            }
        },
        derivative: |_, _| Expr::number(0.0), // Dummy derivative
    }
}

#[test]
fn test_context_registration() {
    let ctx = FunctionContext::new();
    assert!(ctx.is_empty());

    let def = make_times_two();
    ctx.register("times_two", def).unwrap();
    assert_eq!(ctx.len(), 1);

    // Verify lookup via ID
    let sym = crate::core::symbol::symb("times_two");
    let fetched = ctx.get(sym.id()).expect("Should find registered function");
    assert_eq!(fetched.name, "times_two");
}

#[test]
fn test_context_isolation() {
    let ctx1 = FunctionContext::new();
    let ctx2 = FunctionContext::new();

    ctx1.register("f", make_times_two()).unwrap();

    let sym = crate::core::symbol::symb("f");
    assert!(ctx1.get(sym.id()).is_some());
    assert!(ctx2.get(sym.id()).is_none());
}

#[test]
fn test_compiled_evaluation_with_context() {
    let ctx = FunctionContext::new();
    ctx.register("times_two", make_times_two()).unwrap();

    // Expression: times_two(x) + 1
    // We construct it manually or parse it (if parser supports it)
    // Since parser support isn't strictly enforced yet, let's build Expr manually
    let x = crate::symb("x");
    let expr = crate::Expr::func("times_two", x) + crate::Expr::number(1.0);

    // Compile with context
    let evaluator = crate::CompiledEvaluator::compile_with_context(&expr, &["x"], Some(&ctx))
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

    let result = crate::CompiledEvaluator::compile(&expr, &["x"]);
    assert!(result.is_err(), "Should fail to compile unknown function");
}

#[test]
fn test_arity_check() {
    let ctx = FunctionContext::new();
    ctx.register("add2", make_add_custom()).unwrap();

    // Correct arity: add2(x, y)
    let x = crate::symb("x");
    let y = crate::symb("y");
    let expr_ok = crate::Expr::func_multi("add2", vec![x.into(), y.into()]);

    let evaluator =
        crate::CompiledEvaluator::compile_with_context(&expr_ok, &["x", "y"], Some(&ctx))
            .expect("Should compile with correct arity");
    let res = evaluator.evaluate(&[2.0, 3.0]);
    assert_eq!(res, 5.0);

    // Incorrect arity: add2(x)
    let expr_bad = crate::Expr::func_multi("add2", vec![x.into()]);
    let result = crate::CompiledEvaluator::compile_with_context(&expr_bad, &["x"], Some(&ctx));
    assert!(result.is_err(), "Should fail with invalid arity");
}
