/// API Showcase: Complete SymbAnaFis Feature Demonstration (Rust)
///
/// This example demonstrates ALL the capabilities of SymbAnaFis from Rust,
/// following the structure of the API Reference documentation.
///
/// Run with: cargo run --example api_showcase
use num_traits::Float;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[allow(unused_imports)]
use symb_anafis::{
    CompiledEvaluator, Context, CovEntry, CovarianceMatrix, Diff, Dual, Expr, Simplify, Symbol,
    UserFunction, diff, evaluate_str, gradient, gradient_str, hessian, hessian_str, jacobian,
    jacobian_str, parse, relative_uncertainty, simplify, symb, uncertainty_propagation,
};

fn main() {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║          SYMB ANAFIS: COMPLETE API SHOWCASE (Rust)               ║");
    println!("║          Symbolic Differentiation Library                        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Following API_REFERENCE.md structure
    section_quick_start();
    section_symbol_management();
    section_core_functions();
    section_builder_pattern_api();
    section_expression_output();
    section_uncertainty_propagation();
    section_custom_functions();
    section_evaluation();
    section_vector_calculus();
    section_automatic_differentiation();
    #[cfg(feature = "parallel")]
    section_parallel_evaluation();
    #[cfg(feature = "parallel")]
    section_compilation_and_performance();
    #[cfg(not(feature = "parallel"))]
    {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("11. PARALLEL EVALUATION (requires 'parallel' feature)");
        println!("    Not available in this build");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        section_compilation_and_performance();
    }

    section_builtin_functions();
    section_expression_syntax();
    section_error_handling();

    println!("\n✅ Showcase Complete!");
}

// =============================================================================
// SECTION 1: QUICK START
// =============================================================================
fn section_quick_start() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("1. QUICK START");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 1.1 Differentiation
    println!("  1.1 Differentiation");
    println!("      Input:  diff(\"x^3 + sin(x)\", \"x\")");
    let result = diff("x^3 + sin(x)", "x", &[], None).unwrap();
    println!("      Output: {}\n", result);

    // 1.2 Simplification
    println!("  1.2 Simplification");
    println!("      Input:  simplify(\"sin(x)^2 + cos(x)^2\")");
    let result = simplify("sin(x)^2 + cos(x)^2", &[], None).unwrap();
    println!("      Output: {}\n", result);
}

// =============================================================================
// SECTION 2: SYMBOL MANAGEMENT
// =============================================================================
fn section_symbol_management() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("2. SYMBOL MANAGEMENT");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 2.1 Creating Symbols with symb()
    println!("  2.1 Creating Symbols with symb()");
    let _x: Symbol = symb("x");
    let _y: Symbol = symb("y");
    let _alpha: Symbol = symb("alpha");
    println!("      let x = symb(\"x\");");
    println!("      let y = symb(\"y\");");
    println!("      let alpha = symb(\"alpha\");\n");

    // 2.2 Context: Isolated Environments
    println!("  2.2 Context: Isolated Environments");
    let ctx1 = Context::new();
    let ctx2 = Context::new();

    let x1 = ctx1.symb("x");
    let x2 = ctx2.symb("x");

    println!("      let ctx1 = Context::new();");
    println!("      let ctx2 = Context::new();");
    println!("      let x1 = ctx1.symb(\"x\");");
    println!("      let x2 = ctx2.symb(\"x\");");
    println!("      ctx1.symb(\"x\").id() = {}", x1.id());
    println!("      ctx2.symb(\"x\").id() = {} (different!)\n", x2.id());

    // 2.3 Context API Methods
    println!("  2.3 Context API Methods");
    println!("      ctx1.is_empty(): {}", ctx1.is_empty());
    println!("      ctx1.symbol_names(): {:?}", ctx1.symbol_names());
    println!(
        "      ctx1.contains_symbol(\"x\"): {}\n",
        ctx1.contains_symbol("x")
    );

    // 2.4 Registry Management (Global)
    println!("  2.4 Global Symbol Registry");
    println!(
        "      symbol_exists(\"x\"): {}",
        symb_anafis::symbol_exists("x")
    );
    println!("      symbol_count(): {}", symb_anafis::symbol_count());
    println!("      symbol_names(): {:?}\n", symb_anafis::symbol_names());
}

// =============================================================================
// SECTION 3: CORE FUNCTIONS
// =============================================================================
fn section_core_functions() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("3. CORE FUNCTIONS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 3.1 diff(formula, var)
    println!("  3.1 diff(formula, var)");
    println!("      Differentiate x^2 + 2*x + 1 with respect to x");
    let result = diff("x^2 + 2*x + 1", "x", &[], None).unwrap();
    println!("      Result: {}\n", result);

    // 3.2 simplify(formula)
    println!("  3.2 simplify(formula)");
    println!("      Simplify x^2 + 2*x + 1");
    let result = simplify("x^2 + 2*x + 1", &[], None).unwrap();
    println!("      Result: {}\n", result);

    // 3.3 parse(formula)
    println!("  3.3 parse(formula)");
    println!("      parse(\"x^2 + sin(x)\")");
    let expr = parse("x^2 + sin(x)", &HashSet::new(), &HashSet::new(), None).unwrap();
    println!("      Result: {}", expr);
    println!("      Type: Expr\n");

    // 3.3 Type-Safe Expressions (Object-based API)
    println!("  3.3 Type-Safe Expressions");
    let x = symb("x");
    let expr = x.pow(2.0) + x.sin(); // x² + sin(x)
    println!("      let x = symb(\"x\");");
    println!("      let expr = x.pow(2.0) + x.sin();");
    println!("      Result: {}\n", expr);
}

// =============================================================================
// SECTION 4: BUILDER PATTERN API
// =============================================================================
fn section_builder_pattern_api() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("4. BUILDER PATTERN API");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 4.1 Diff Builder
    println!("  4.1 Diff Builder");
    println!("      Diff::new().domain_safe(true).fixed_var(\"a\").diff_str(...)");
    let result = Diff::new()
        .domain_safe(true)
        .diff_str("a*x^2 + x", "x", &["a"])
        .unwrap();
    println!("      d/dx [ax² + x] with a as constant: {}\n", result);

    // 4.2 Diff Builder Options
    println!("  4.2 Diff Builder Options");
    let result = Diff::new()
        .max_depth(100)
        .max_nodes(1000)
        .diff_str("x^3", "x", &[])
        .unwrap();
    println!("      With max_depth=100, max_nodes=1000: {}\n", result);

    // 4.3 Simplify Builder
    println!("  4.3 Simplify Builder");
    println!("      Simplify::new().domain_safe(true).fixed_var(\"k\").simplify_str(...)");
    let result = Simplify::new()
        .domain_safe(true)
        .fixed_vars(&[&symb("x"), &symb("y")])
        .simplify_str("k*x + k*y", &["k"])
        .unwrap();
    println!("      Simplify k*x + k*y: {}\n", result);

    // 4.4 Differentiating Expression Objects
    println!("  4.4 Differentiating Expression Objects");
    let x = symb("x");
    let f = x.pow(3.0) + x.sin() + 1.0;
    let df = Diff::new().differentiate(&f, &x).unwrap();
    println!("      f(x) = {}", f);
    println!("      f'(x) = {}\n", df);

    // 4.5 Expression Inspection
    println!("  4.5 Expression Inspection");
    let y = symb("y");
    let complex_expr = x.pow(2.0) + y.sin();
    println!("      Expression: {}", complex_expr);
    println!("      Node count: {}", complex_expr.node_count());
    println!("      Max depth:  {}\n", complex_expr.max_depth());
}

// =============================================================================
// SECTION 5: EXPRESSION OUTPUT
// =============================================================================
fn section_expression_output() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("5. EXPRESSION OUTPUT");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let x = symb("x");
    let y = symb("y");
    let _sigma = symb("sigma");

    // 5.1 LaTeX Output
    println!("  5.1 LaTeX Output: to_latex()");
    let expr1 = x.pow(2.0) / y;
    let expr2 = x.sin() * y;
    println!("      x²/y      → {}", expr1.to_latex());
    println!("      sin(x)·y → {}\n", expr2.to_latex());

    // 5.2 Unicode Output
    println!("  5.2 Unicode Output: to_unicode()");
    let pi_expr = symb("pi");
    let omega = symb("omega");
    let expr3 = pi_expr + omega.pow(2.0);
    println!("      π + ω² → {}\n", expr3.to_unicode());

    // 5.3 Standard Display
    println!("  5.3 Standard Display: Display trait");
    let expr4 = x.sin() + y.cos();
    println!("      sin(x) + cos(y) → {}\n", expr4);
}

// =============================================================================
// SECTION 6: UNCERTAINTY PROPAGATION
// =============================================================================
fn section_uncertainty_propagation() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("6. UNCERTAINTY PROPAGATION");
    println!("   σ_f = √(Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ))");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 6.1 Basic Uncertainty (symbolic)
    println!("  6.1 Basic Uncertainty (symbolic)");
    let x = symb("x");
    let y = symb("y");
    let expr = x + y;
    println!("      f = x + y");
    match uncertainty_propagation(&expr, &["x", "y"], None) {
        Ok(sigma) => println!("      σ_f = {}\n", sigma),
        Err(e) => println!("      Error: {:?}\n", e),
    }

    // 6.2 Product Formula Uncertainty
    println!("  6.2 Product Formula Uncertainty");
    let expr2 = x * y;
    println!("      f = x * y");
    match uncertainty_propagation(&expr2, &["x", "y"], None) {
        Ok(sigma) => println!("      σ_f = {}\n", sigma),
        Err(e) => println!("      Error: {:?}\n", e),
    }

    // 6.3 Numeric Covariance
    println!("  6.3 Numeric Uncertainty Values");
    println!("      f = x * y with σ_x = 0.1, σ_y = 0.2");
    let cov = CovarianceMatrix::diagonal(vec![
        CovEntry::Num(0.01), // σ_x² = 0.01 (σ_x = 0.1)
        CovEntry::Num(0.04), // σ_y² = 0.04 (σ_y = 0.2)
    ]);
    match uncertainty_propagation(&expr2, &["x", "y"], Some(&cov)) {
        Ok(sigma) => println!("      σ_f = {}\n", sigma),
        Err(e) => println!("      Error: {:?}\n", e),
    }

    // 6.4 Relative Uncertainty
    println!("  6.4 Relative Uncertainty");
    let expr3 = x.pow(2.0);
    println!("      f = x²");
    match relative_uncertainty(&expr3, &["x"], None) {
        Ok(rel) => println!("      σ_f/|f| = {}\n", rel),
        Err(e) => println!("      Error: {:?}\n", e),
    }
}

// =============================================================================
// SECTION 7: CUSTOM FUNCTIONS
// =============================================================================
fn section_custom_functions() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("7. CUSTOM FUNCTIONS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 7.1 Single-Argument Custom Derivatives
    println!("  7.1 Single-Argument Custom Derivatives");
    println!("      Define: my_func(u) with derivative ∂f/∂u = 2u");

    let _x = symb("x");

    let my_func_partial = UserFunction::new(1..=1)
        .partial(0, |args: &[Arc<Expr>]| {
            // ∂f/∂u = 2u
            Expr::number(2.0) * Expr::from(&args[0])
        })
        .expect("valid arg");

    let custom_diff = Diff::new().user_fn("my_func", my_func_partial);

    // Test the custom derivative
    let result = custom_diff.diff_str("my_func(x^2)", "x", &[]).unwrap();
    println!("      d/dx[my_func(x²)] = {}", result);
    println!("      Chain rule: 2u · u' = 2(x²) · 2x = 4x³\n");

    // 7.2 Custom Function with Body (for evaluation)
    println!("  7.2 Custom Function with Body");
    println!("      Define: sq(u) = u² with derivative 2u");

    let sq_fn = UserFunction::new(1..=1)
        .body(|args| Expr::from(&args[0]).pow(2.0))
        .partial(0, |args: &[Arc<Expr>]| {
            Expr::number(2.0) * Expr::from(&args[0])
        })
        .expect("valid arg");

    let sq_diff = Diff::new().user_fn("sq", sq_fn);
    let result = sq_diff.diff_str("sq(x)", "x", &[]).unwrap();
    println!("      d/dx[sq(x)] = {}\n", result);
}

// =============================================================================
// SECTION 8: EVALUATION
// =============================================================================
fn section_evaluation() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("8. EVALUATION");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 8.1 evaluate_str() (full evaluation)
    println!("  8.1 evaluate_str() (full evaluation)");
    let result = evaluate_str("x * y + 1", &[("x", 3.0), ("y", 2.0)]).unwrap();
    println!("      x*y + 1 with x=3, y=2 → {} (expected: 7)\n", result);

    // 8.2 evaluate_str() (partial evaluation)
    println!("  8.2 evaluate_str() (partial evaluation)");
    let result = evaluate_str("x * y + 1", &[("x", 3.0)]).unwrap();
    println!("      x*y + 1 with x=3 → {}\n", result);

    // 8.3 Expr.evaluate()
    println!("  8.3 Expr.evaluate()");
    let x = symb("x");
    let y = symb("y");
    let expr = x.pow(2.0) + y.pow(2.0);

    let mut vars: HashMap<&str, f64> = HashMap::new();
    vars.insert("x", 3.0);
    vars.insert("y", 4.0);

    let result = expr.evaluate(&vars, &HashMap::new());
    println!("      x² + y² at (x=3, y=4)");
    if let Some(n) = result.as_number() {
        println!("      Result: {} (expected: 25)\n", n);
    }
}

// =============================================================================
// SECTION 9: VECTOR CALCULUS
// =============================================================================
fn section_vector_calculus() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("9. VECTOR CALCULUS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 9.1 Gradient
    println!("  9.1 Gradient: ∇f = [∂f/∂x, ∂f/∂y, ...]");
    let grad = gradient_str("x^2*y + y^3", &["x", "y"]).unwrap();
    println!("      f(x,y) = x²y + y³");
    println!("      ∂f/∂x = {}", grad[0]);
    println!("      ∂f/∂y = {}\n", grad[1]);

    // 9.2 Hessian Matrix
    println!("  9.2 Hessian Matrix: H[i][j] = ∂²f/∂xᵢ∂xⱼ");
    let hess = hessian_str("x^2*y + y^3", &["x", "y"]).unwrap();
    println!("      f = x²y + y³");
    println!("      H = | {} {} |", hess[0][0], hess[0][1]);
    println!("          | {} {} |\n", hess[1][0], hess[1][1]);

    // 9.3 Jacobian Matrix
    println!("  9.3 Jacobian Matrix: J[i][j] = ∂fᵢ/∂xⱼ");
    let jac = jacobian_str(&["x^2 + y", "x*y"], &["x", "y"]).unwrap();
    println!("      f₁ = x² + y, f₂ = xy");
    println!("      J = | {} {} |", jac[0][0], jac[0][1]);
    println!("          | {} {} |\n", jac[1][0], jac[1][1]);
}

// =============================================================================
// SECTION 10: AUTOMATIC DIFFERENTIATION
// =============================================================================
fn section_automatic_differentiation() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("10. AUTOMATIC DIFFERENTIATION");
    println!("    Dual Numbers: a + bε where ε² = 0");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 10.1 Basic Usage
    println!("  10.1 Basic Usage");
    println!("      f(x) = x² + 3x + 1, find f(2) and f'(2)");

    // x = 2, x' = 1
    let x = Dual::new(2.0, 1.0);

    // f(x)
    let fx = x * x + Dual::new(3.0, 0.0) * x + Dual::new(1.0, 0.0);

    println!("      f(2)  = {} (Value)", fx.val);
    println!("      f'(2) = {} (Derivative)\n", fx.eps);

    // 10.2 Transcendental Functions
    println!("  10.2 Transcendental Functions");
    println!("      f(x) = sin(x) * exp(x) at x = 1");

    let x = Dual::new(1.0, 1.0);
    let fx = x.sin() * x.exp();

    println!("      f(1)  = {:.6}", fx.val);
    println!("      f'(1) = {:.6}\n", fx.eps);

    // 10.3 Chain Rule (Automatic)
    println!("  10.3 Chain Rule (Automatic)");
    println!("      f(x) = sin(x² + 1), f'(x) = cos(x² + 1) * 2x");

    let x = Dual::new(1.5, 1.0);
    let inner = x * x + Dual::new(1.0, 0.0); // x² + 1
    let fx = inner.sin();

    println!("      f(1.5)  = {:.6}", fx.val);
    println!("      f'(1.5) = {:.6}\n", fx.eps);
}

// =============================================================================
// SECTION 11: PARALLEL EVALUATION (requires "parallel" feature)
// =============================================================================
#[cfg(feature = "parallel")]
fn section_parallel_evaluation() {
    use symb_anafis::eval_f64;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("11. PARALLEL EVALUATION");
    println!("    Requires 'parallel' feature");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 11.1 High-Performance eval_f64
    println!("  11.1 High-Performance eval_f64");
    println!("      Evaluate x² at x = 1, 2, 3, 4, 5");

    let x = symb("x");
    let expr = x.pow(2.0);
    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];

    let results = eval_f64(&[&expr], &[&[&x]], &[&[&x_data]]).unwrap();
    println!("      Results: {:?}\n", results[0]);
}

// =============================================================================
// SECTION 12: COMPILATION & PERFORMANCE
// =============================================================================
fn section_compilation_and_performance() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("12. COMPILATION & PERFORMANCE");
    println!("    CompiledEvaluator for high-performance repeated evaluation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 12.1 Compile Expression
    println!("  12.1 Compile Expression");
    let x = symb("x");
    let expr = x.sin() * x.pow(2.0) + Expr::number(1.0);
    println!("      Expression: {}", expr);

    let compiled = CompiledEvaluator::compile(&expr, &["x"], None).unwrap();

    let val = 2.0;
    let result = compiled.evaluate(&[val]);
    println!("      Result at x={}: {}", val, result);
    println!("      Instructions: {}\n", compiled.instruction_count());

    // 12.2 Batch Evaluation
    println!("  12.2 Batch Evaluation: eval_batch()");
    let inputs = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let mut batch_result = vec![0.0; inputs.len()];
    compiled
        .eval_batch(&[&inputs], &mut batch_result, None)
        .unwrap();
    println!("      Inputs: {:?}", inputs);
    println!("      Results: {:?}\n", batch_result);

    // 12.3 Compilation with Context (Custom Functions)
    println!("  12.3 Compilation with Custom Functions");
    let ctx = Context::new().with_function(
        "my_sq",
        UserFunction::new(1..=1).body(|args| Expr::from(&args[0]).pow(2.0)),
    );

    let expr_custom = Expr::func("my_sq", x.to_expr()) + Expr::number(5.0);
    println!("      Expression: {}", expr_custom);

    let compiled_ctx = CompiledEvaluator::compile(&expr_custom, &["x"], Some(&ctx))
        .expect("Compilation with context failed");

    let res = compiled_ctx.evaluate(&[3.0]);
    println!("      my_sq(3) + 5 = {}\n", res);
}

// =============================================================================
// SECTION 13: BUILT-IN FUNCTIONS
// =============================================================================
fn section_builtin_functions() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("13. BUILT-IN FUNCTIONS");
    println!("    60+ functions with symbolic differentiation and numeric eval");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("  TRIGONOMETRIC:");
    println!("    sin, cos, tan, cot, sec, csc\n");

    println!("  INVERSE TRIG:");
    println!("    asin, acos, atan, atan2, acot, asec, acsc\n");

    println!("  HYPERBOLIC:");
    println!("    sinh, cosh, tanh, coth, sech, csch\n");

    println!("  INVERSE HYPERBOLIC:");
    println!("    asinh, acosh, atanh, acoth, asech, acsch\n");

    println!("  EXPONENTIAL & LOGARITHMIC:");
    println!("    exp, ln, log(base, x), log10, log2\n");

    println!("  POWERS & ROOTS:");
    println!("    sqrt, cbrt, x**n\n");

    println!("  SPECIAL FUNCTIONS:");
    println!("    gamma, digamma, trigamma, polygamma(n, x)");
    println!("    erf, erfc, zeta, beta(a, b)\n");

    println!("  BESSEL FUNCTIONS:");
    println!("    besselj(n, x), bessely(n, x), besseli(n, x), besselk(n, x)\n");

    println!("  ORTHOGONAL POLYNOMIALS:");
    println!("    hermite(n, x), assoc_legendre(l, m, x)\n");

    println!("  SPHERICAL HARMONICS:");
    println!("    spherical_harmonic(l, m, theta, phi), ynm(l, m, theta, phi)\n");

    println!("  OTHER:");
    println!("    abs, signum, sinc, lambertw, floor, ceil, round\n");

    println!("  Example Derivatives:");
    let examples = vec![
        ("gamma(x)", "x"),
        ("erf(x)", "x"),
        ("besselj(0, x)", "x"),
        ("atan2(y, x)", "x"),
    ];
    for (expr, var) in examples {
        let result = diff(expr, var, &[], None).unwrap();
        println!("    d/d{} [{}] = {}", var, expr, result);
    }
    println!();
}

// =============================================================================
// SECTION 14: EXPRESSION SYNTAX
// =============================================================================
fn section_expression_syntax() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("14. EXPRESSION SYNTAX");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("  Elements:");
    println!("    Variables:      x, y, sigma, theta, phi");
    println!("    Numbers:        1, 3.14, 1e-5");
    println!("    Operators:      +, -, *, /, ^");
    println!("    Functions:      sin(x), log(10, x)");
    println!("    Constants:      pi, e (auto-recognized)");
    println!("    Implicit mult:  2x, (x+1)(x-1)");
    println!();

    println!("  Operator Precedence:");
    println!("    Highest: ^ (power, right-associative)");
    println!("    Medium:  *, / (left-associative)");
    println!("    Lowest:  +, - (left-associative)");
    println!();

    println!("  Syntax Examples:");
    let examples = vec![
        "x^2 + 3*x + 1",
        "sin(x)^2 + cos(x)^2",
        "2*(x + y)",
        "x^2 * y + y^3",
        "log(10, x)",
        "besselj(0, x)",
        "atan2(y, x)",
    ];
    for expr in examples {
        let result = simplify(expr, &[], None).unwrap();
        println!("    {:20} → {}", expr, result);
    }
    println!();
}

// =============================================================================
// SECTION 15: ERROR HANDLING
// =============================================================================
fn section_error_handling() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("15. ERROR HANDLING");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    use symb_anafis::DiffError;

    println!("  All functions return Result<T, DiffError>:");
    println!();

    // Example 1: Empty formula
    println!("  1. Empty Formula:");
    match diff("", "x", &[], None) {
        Ok(result) => println!("     Result: {}", result),
        Err(DiffError::EmptyFormula) => println!("     Error: EmptyFormula"),
        Err(e) => println!("     Error: {:?}", e),
    }

    // Example 2: Invalid syntax
    println!("  2. Invalid Syntax:");
    match diff("((", "x", &[], None) {
        Ok(result) => println!("     Result: {}", result),
        Err(DiffError::InvalidSyntax { msg, .. }) => {
            println!("     Error: InvalidSyntax - {}", msg)
        }
        Err(e) => println!("     Error: {:?}", e),
    }

    // Example 3: Invalid number
    println!("  3. Invalid Number:");
    match diff("x^abc", "x", &[], None) {
        Ok(result) => println!("     Result: {}", result),
        Err(DiffError::InvalidNumber { value, .. }) => {
            println!("     Error: InvalidNumber - '{}'", value)
        }
        Err(e) => println!("     Error: {:?}", e),
    }

    // Example 4: Variable in both fixed and diff
    println!("  4. Variable Conflict:");
    match Diff::new().diff_str("x^2", "x", &["x"]) {
        Ok(result) => println!("     Result: {}", result),
        Err(DiffError::VariableInBothFixedAndDiff { var }) => {
            println!("     Error: VariableInBothFixedAndDiff - '{}'", var)
        }
        Err(e) => println!("     Error: {:?}", e),
    }

    println!();
    println!("  Common DiffError Variants:");
    println!("    EmptyFormula");
    println!("    InvalidSyntax {{ msg, span }}");
    println!("    InvalidNumber {{ value, span }}");
    println!("    InvalidToken {{ token, span }}");
    println!("    UnexpectedToken {{ expected, got, span }}");
    println!("    UnexpectedEndOfInput");
    println!("    InvalidFunctionCall {{ name, expected, got }}");
    println!("    VariableInBothFixedAndDiff {{ var }}");
    println!("    NameCollision {{ name }}");
    println!("    UnsupportedOperation(String)");
    println!();
}
