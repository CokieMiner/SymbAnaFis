#!/usr/bin/env python3
"""
API Showcase: Complete SymbAnaFis Feature Demonstration (Python)

This example demonstrates ALL the capabilities of SymbAnaFis from Python,
following the structure of the API Reference documentation.

Run with: python examples/api_showcase.py
"""

from symb_anafis import (
    Expr, Diff, Simplify,
    diff, simplify, parse,
    gradient, hessian, jacobian, evaluate, evaluate_str,
    uncertainty_propagation, relative_uncertainty,
    Context, CompiledEvaluator, Dual, Symbol, symb,
    gradient_str, hessian_str, jacobian_str
)

# Try to import parallel evaluation (only available with parallel feature)
try:
    from symb_anafis import evaluate_parallel
    HAS_PARALLEL = True
except ImportError:
    HAS_PARALLEL = False


def main():
    print("╔══════════════════════════════════════════════════════════════════╗")
    print("║          SYMB ANAFIS: COMPLETE API SHOWCASE (Python)             ║")
    print("║          Symbolic Differentiation Library                        ║")
    print("╚══════════════════════════════════════════════════════════════════╝\n")

    # Following API_REFERENCE.md structure
    section_quick_start()
    section_symbol_management()
    section_core_functions()
    section_builder_pattern_api()
    section_expression_output()
    section_uncertainty_propagation()
    section_custom_functions()
    section_evaluation()
    section_vector_calculus()
    section_automatic_differentiation()
    if HAS_PARALLEL:
        section_parallel_evaluation()
    else:
        print("━" * 66)
        print("⚡ SECTION 11: PARALLEL EVALUATION")
        print("   (Requires 'parallel' feature - not available)")
        print("━" * 66 + "\n")
    section_compilation_and_performance()
    section_builtin_functions()
    section_expression_syntax()
    section_error_handling()

    print("\n✅ Showcase Complete!")


# =============================================================================
# SECTION 1: QUICK START
# =============================================================================
def section_quick_start():
    print("━" * 66)
    print("1. QUICK START")
    print("━" * 66 + "\n")

    print("  1.1 Differentiation")
    formula = "x^3 + sin(x)"
    print(f"      Input:  diff('{formula}', 'x')")
    result = diff(formula, "x")
    print(f"      Output: {result}\n")

    print("  1.2 Simplification")
    formula = "sin(x)^2 + cos(x)^2"
    print(f"      Input:  simplify('{formula}')")
    result = simplify(formula)
    print(f"      Output: {result}\n")


# =============================================================================
# SECTION 2: SYMBOL MANAGEMENT
# =============================================================================
def section_symbol_management():
    print("━" * 66)
    print("2. SYMBOL MANAGEMENT")
    print("━" * 66 + "\n")

    print("  2.1 Creating Symbols with Expr()")
    x = Expr("x")
    y = Expr("y")
    alpha = Expr("alpha")
    print("      x = Expr('x')")
    print("      y = Expr('y')")
    print("      alpha = Expr('alpha')\n")

    print("  2.2 Context: Isolated Environments")
    ctx1 = Context()
    ctx2 = Context()

    x1 = ctx1.symb("x")
    x2 = ctx2.symb("x")

    print("      ctx1 = Context()")
    print("      ctx2 = Context()")
    print("      x1 = ctx1.symb('x')")
    print("      x2 = ctx2.symb('x')")
    print(f"      ctx1 symbol id: {x1.id()}")
    print(f"      ctx2 symbol id: {x2.id()} (different!)\n")

    print("  2.3 Context API Methods")
    print(f"      ctx1.is_empty(): {ctx1.is_empty()}")
    print(f"      ctx1.symbol_names(): {ctx1.symbol_names()}")
    print(f"      ctx1.contains_symbol('x'): {ctx1.contains_symbol('x')}\n")

    print("  2.4 Global Symbol Registry")
    from symb_anafis import symbol_exists, symbol_count, symbol_names
    print(f"      symbol_exists('x'): {symbol_exists('x')}")
    print(f"      symbol_count(): {symbol_count()}")
    print(f"      symbol_names(): {symbol_names()}\n")


# =============================================================================
# SECTION 3: CORE FUNCTIONS
# =============================================================================
def section_core_functions():
    print("━" * 66)
    print("3. CORE FUNCTIONS")
    print("━" * 66 + "\n")

    print("  3.1 diff(formula, var)")
    print("      Differentiate x^2 + 2*x + 1 with respect to x")
    result = diff("x^2 + 2*x + 1", "x")
    print(f"      Result: {result}\n")

    print("  3.2 simplify(formula)")
    print("      Simplify x^2 + 2*x + 1")
    result = simplify("x^2 + 2*x + 1")
    print(f"      Result: {result}\n")

    print("  3.3 parse(formula)")
    expr = parse("x^2 + sin(x)")
    print(f"      parse('x^2 + sin(x)') -> {expr}")
    print(f"      Type: {type(expr).__name__}\n")

    print("  3.4 Type-Safe Expressions (Object-based API)")
    x = Expr("x")
    expr = x.pow(2) + x.sin()  # x² + sin(x)
    print(f"      x = Expr('x')")
    print(f"      expr = x.pow(2) + x.sin()")
    print(f"      Result: {expr}\n")


# =============================================================================
# SECTION 4: BUILDER PATTERN API
# =============================================================================
def section_builder_pattern_api():
    print("━" * 66)
    print("4. BUILDER PATTERN API")
    print("━" * 66 + "\n")

    print("  4.1 Diff Builder")
    print("      Diff().domain_safe(True).fixed_var(symb('a')).diff_str(...)")
    d = Diff().domain_safe(True).fixed_var(symb("a"))
    result = d.diff_str("a*x^2 + x", "x")
    print(f"      d/dx [ax² + x] with a as constant: {result}\n")

    print("  4.2 Diff Builder: Multiple Fixed Variables")
    print("      Diff().domain_safe(True).fixed_vars([symb('a'), symb('b')]).diff_str(...)")
    d2 = Diff().domain_safe(True).fixed_vars([symb("a"), symb("b")])
    result2 = d2.diff_str("a*x + b*y + x^2", "x")
    print(f"      d/dx [ax + by + x²] with a,b fixed: {result2}\n")

    print("  4.2 Diff Builder Options")
    d = Diff().max_depth(100).max_nodes(1000)
    result = d.diff_str("x^3", "x")
    print(f"      With max_depth=100, max_nodes=1000: {result}\n")

    print("  4.3 Simplify Builder")
    print("      Simplify().domain_safe(True).fixed_var(symb('k')).simplify_str(...)")
    s = Simplify().domain_safe(True).fixed_var(symb("k"))
    result = s.simplify_str("k*x + k*y")
    print(f"      Simplify k*x + k*y: {result}\n")

    print("  4.4 Differentiating Expression Objects")
    x = Expr("x")
    f = x.pow(3) + x.sin()
    df = Diff().differentiate(f, "x")
    print(f"      f(x) = {f}")
    print(f"      f'(x) = {df}\n")

    print("  4.5 Expression Inspection")
    complex_expr = x.pow(2) + Expr("y").sin()
    print(f"      Expression: {complex_expr}")
    print(f"      Node count: {complex_expr.node_count()}")
    print(f"      Max depth:  {complex_expr.max_depth()}")
    print()


# =============================================================================
# SECTION 5: EXPRESSION OUTPUT
# =============================================================================
def section_expression_output():
    print("━" * 66)
    print("5. EXPRESSION OUTPUT")
    print("━" * 66 + "\n")

    x = Expr("x")
    y = Expr("y")
    sigma = Expr("sigma")

    print("  5.1 LaTeX Output: to_latex()")
    expr1 = x.pow(2) / sigma
    expr2 = x.sin() * y
    print(f"      x²/y      → {expr1.to_latex()}")
    print(f"      sin(x)·y → {expr2.to_latex()}\n")

    print("  5.2 Unicode Output: to_unicode()")
    pi_expr = Expr("pi")
    omega = Expr("omega")
    expr3 = pi_expr + omega.pow(2)
    print(f"      π + ω² → {expr3.to_unicode()}\n")

    print("  5.3 Standard Display: str()")
    expr4 = x.sin() + y.cos()
    print(f"      sin(x) + cos(y) → {expr4}\n")


# =============================================================================
# SECTION 6: UNCERTAINTY PROPAGATION
# =============================================================================
def section_uncertainty_propagation():
    print("━" * 66)
    print("6. UNCERTAINTY PROPAGATION")
    print("   σ_f = √(Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ))")
    print("━" * 66 + "\n")

    print("  6.1 Basic Uncertainty (symbolic)")
    result = uncertainty_propagation("x + y", ["x", "y"])
    print("      f = x + y")
    print(f"      σ_f = {result}\n")

    print("  6.2 Product Formula Uncertainty")
    result = uncertainty_propagation("x * y", ["x", "y"])
    print("      f = x * y")
    print(f"      σ_f = {result}\n")

    print("  6.3 Numeric Uncertainty Values")
    # σ_x = 0.1 (so σ_x² = 0.01), σ_y = 0.2 (so σ_y² = 0.04)
    result = uncertainty_propagation("x * y", ["x", "y"], [0.01, 0.04])
    print("      f = x * y with σ_x = 0.1, σ_y = 0.2")
    print(f"      σ_f = {result}\n")

    print("  6.4 Relative Uncertainty")
    result = relative_uncertainty("x^2", ["x"])
    print("      f = x²")
    print(f"      σ_f/|f| = {result}\n")


# =============================================================================
# SECTION 7: CUSTOM FUNCTIONS
# =============================================================================
def section_custom_functions():
    print("━" * 66)
    print("7. CUSTOM FUNCTIONS")
    print("━" * 66 + "\n")

    print("  7.1 Single-Argument Custom Derivatives")
    print("      Define: my_func(u) with derivative ∂f/∂u = 2u")

    def my_func_partial(args):
        # For f(u), return ∂f/∂u = 2u
        return 2 * args[0]

    x = Expr("x")
    # Body is None, partials is a list [∂f/∂u]
    custom_diff = Diff().user_fn("my_func", 1, None, [my_func_partial])

    # Test the custom derivative
    result = custom_diff.diff_str("my_func(x^2)", "x")
    print(f"      d/dx[my_func(x²)] = {result}")
    print(f"      Chain rule: 2u · u' = 2(x²) · 2x = 4x³\n")

    print("  7.2 Custom Function with Body (for evaluation)")
    print("      Define: sq(u) = u² with derivative 2u")

    def sq_body(args):
        return args[0] ** 2

    def sq_partial(args):
        return 2 * args[0]

    # Register with both body (for evaluation) AND partial derivatives list (for diff)
    sq_diff = Diff().user_fn("sq", 1, sq_body, [sq_partial])
    
    # 1. Differentiation
    result = sq_diff.diff_str("sq(x)", "x")
    print(f"      d/dx[sq(x)] = {result}")

    # 2. Numeric Evaluation (Now supported!)
    from symb_anafis import evaluate
    # We need to register it in a context or use evaluate with valid scope if we were evaluating purely
    # But since Diff returns a string, we can't easily eval the result string with 'sq' unless 'sq' is in the Evaluator's context.
    # However, we can demonstrate proper object construction if we extended the API to return Expr.
    
    # Let's show Context providing the function for evaluation:
    ctx = Context().with_function("sq", 1, sq_body, [sq_partial])
    expr = ctx.symb("x")
    # This requires constructing a function call expr manually or via parser if we had it exposed attached to context
    # For now, let's just confirm the API accepts the body callback without error.
    print(f"      (Body callback registered successfully)\n")


# =============================================================================
# SECTION 8: EVALUATION
# =============================================================================
def section_evaluation():
    print("━" * 66)
    print("8. EVALUATION")
    print("━" * 66 + "\n")

    print("  8.1 evaluate_str() from String (full evaluation)")
    result = evaluate_str("x * y + 1", [("x", 3.0), ("y", 2.0)])
    print(f"      x*y + 1 with x=3, y=2 → {result} (expected: 7)\n")

    print("  8.2 evaluate_str() Partial Evaluation")
    result = evaluate_str("x * y + 1", [("x", 3.0)])
    print(f"      x*y + 1 with x=3 → {result}\n")

    print("  8.3 Expr.evaluate()")
    x = Expr("x")
    y = Expr("y")
    expr = x.pow(2) + y.pow(2)
    result = expr.evaluate({"x": 3.0, "y": 4.0})
    print("      x² + y² at (x=3, y=4)")
    print(f"      Result: {result} (expected: 25)\n")


# =============================================================================
# SECTION 9: VECTOR CALCULUS
# =============================================================================
def section_vector_calculus():
    print("━" * 66)
    print("9. VECTOR CALCULUS")
    print("━" * 66 + "\n")

    print("  9.1 Gradient: ∇f = [∂f/∂x, ∂f/∂y, ...]")
    grad = gradient_str("x^2*y + y^3", ["x", "y"])
    print("      f(x,y) = x²y + y³")
    print(f"      ∂f/∂x = {grad[0]}")
    print(f"      ∂f/∂y = {grad[1]}\n")

    print("  9.2 Hessian Matrix: H[i][j] = ∂²f/∂xᵢ∂xⱼ")
    hess = hessian_str("x^2*y + y^3", ["x", "y"])
    print("      f = x²y + y³")
    print(f"      H = | {hess[0][0]} {hess[0][1]} |")
    print(f"          | {hess[1][0]} {hess[1][1]} |\n")

    print("  9.3 Jacobian Matrix: J[i][j] = ∂fᵢ/∂xⱼ")
    jac = jacobian_str(["x^2 + y", "x*y"], ["x", "y"])
    print("      f₁ = x² + y, f₂ = xy")
    print(f"      J = | {jac[0][0]} {jac[0][1]} |")
    print(f"          | {jac[1][0]} {jac[1][1]} |\n")


# =============================================================================
# SECTION 10: AUTOMATIC DIFFERENTIATION
# =============================================================================
def section_automatic_differentiation():
    print("━" * 66)
    print("10. AUTOMATIC DIFFERENTIATION")
    print("    Dual Numbers: a + bε where ε² = 0")
    print("━" * 66 + "\n")

    print("  10.1 Basic Usage")
    print("      f(x) = x² + 3x + 1, find f(2) and f'(2)")

    x = Dual(2.0, 1.0)
    fx = x * x + Dual.constant(3.0) * x + Dual.constant(1.0)

    print(f"      f(2)  = {fx.val}")
    print(f"      f'(2) = {fx.eps}\n")

    print("  10.2 Transcendental Functions")
    print("      f(x) = sin(x) * exp(x) at x = 1")

    x = Dual(1.0, 1.0)
    fx = x.sin() * x.exp()

    print(f"      f(1)  = {fx.val:.6f}")
    print(f"      f'(1) = {fx.eps:.6f}\n")

    print("  10.3 Chain Rule (Automatic)")
    print("      f(x) = sin(x² + 1), f'(x) = cos(x² + 1) * 2x")

    x = Dual(1.5, 1.0)
    inner = x * x + Dual.constant(1.0)
    fx = inner.sin()

    print(f"      f(1.5)  = {fx.val:.6f}")
    print(f"      f'(1.5) = {fx.eps:.6f}\n")


# =============================================================================
# SECTION 11: PARALLEL EVALUATION
# =============================================================================
def section_parallel_evaluation():
    print("━" * 66)
    print("11. PARALLEL EVALUATION")
    print("    Requires 'parallel' feature")
    print("━" * 66 + "\n")

    print("  11.1 Basic Parallel Evaluation")
    print("      Evaluate x² at x = 1, 2, 3, 4, 5")

    results = evaluate_parallel(
        ["x^2"],
        [["x"]],
        [[[1.0, 2.0, 3.0, 4.0, 5.0]]]
    )
    print(f"      Results: {results[0]}\n")

    print("  11.2 Multiple Expressions in Parallel")
    results = evaluate_parallel(
        ["x^2", "x^3"],
        [["x"], ["x"]],
        [
            [[1.0, 2.0, 3.0]],
            [[1.0, 2.0, 3.0]]
        ]
    )
    print(f"      x²: {results[0]}")
    print(f"      x³: {results[1]}\n")

    print("  11.3 Two Variables")
    print("      Evaluate x + y at (x=1,y=10), (x=2,y=20), (x=3,y=30)")
    results = evaluate_parallel(
        ["x + y"],
        [["x", "y"]],
        [[[1.0, 2.0, 3.0], [10.0, 20.0, 30.0]]]
    )
    print(f"      Results: {results[0]}\n")

    print("  11.4 SKIP for Partial Symbolic Evaluation")
    print("      Evaluate x*y with x=2,SKIP,4 and y=3,5,6")
    results = evaluate_parallel(
        ["x * y"],
        [["x", "y"]],
        [[[2.0, None, 4.0], [3.0, 5.0, 6.0]]]  # None = SKIP
    )
    print(f"      Point 0: x=2, y=3 → {results[0][0]}")
    print(f"      Point 1: x=SKIP, y=5 → {results[0][1]} (symbolic!)")
    print(f"      Point 2: x=4, y=6 → {results[0][2]}\n")


# =============================================================================
# SECTION 12: COMPILATION & PERFORMANCE
# =============================================================================
def section_compilation_and_performance():
    print("━" * 66)
    print("12. COMPILATION & PERFORMANCE")
    print("    CompiledEvaluator for high-performance repeated evaluation")
    print("━" * 66 + "\n")

    print("  12.1 Compile Expression")
    x = Expr("x")
    expr = x.sin() * x.pow(2) + 1
    print(f"      Expression: {expr}")

    compiled = CompiledEvaluator(expr, ["x"])

    val = 2.0
    result = compiled.evaluate([val])
    print(f"      Result at x={val}: {result}")
    print(f"      Instructions: {compiled.instruction_count()}\n")

    print("  12.2 Batch Evaluation")
    inputs = [0.0, 1.0, 2.0, 3.0, 4.0]
    batch_results = compiled.eval_batch([inputs])
    print(f"      Inputs: {inputs}")
    print(f"      Results: {batch_results}\n")

    # 12.3 Compilation with Context (Custom Functions)
    print("  12.3 Compilation with Custom Functions")
    # Define body callback
    def my_sq_body(args):
        return args[0] ** 2.0

    # Create Context with function (using new API signature: body first, partials list)
    ctx = Context().with_function("my_sq", 1, my_sq_body, [])

    # Create expression calling custom function
    # We rely on parse here since Expr builder for functions might not be exposed directly
    from symb_anafis import parse
    expr_custom = parse("my_sq(x) + 5", custom_functions=["my_sq"])
    print(f"      Expression: {expr_custom}")

    # Compile with context (newly supported in bindings)
    compiled_ctx = CompiledEvaluator(expr_custom, ["x"], ctx)

    res = compiled_ctx.evaluate([3.0])
    print(f"      my_sq(3) + 5 = {res} (expected: 14.0)\n")


# =============================================================================
# SECTION 13: BUILT-IN FUNCTIONS
# =============================================================================
def section_builtin_functions():
    print("━" * 66)
    print("13. BUILT-IN FUNCTIONS")
    print("    60+ functions with symbolic differentiation and numeric eval")
    print("━" * 66 + "\n")

    print("  TRIGONOMETRIC:")
    print("    sin, cos, tan, cot, sec, csc\n")

    print("  INVERSE TRIG:")
    print("    asin, acos, atan, atan2, acot, asec, acsc\n")

    print("  HYPERBOLIC:")
    print("    sinh, cosh, tanh, coth, sech, csch\n")

    print("  INVERSE HYPERBOLIC:")
    print("    asinh, acosh, atanh, acoth, asech, acsch\n")

    print("  EXPONENTIAL & LOGARITHMIC:")
    print("    exp, ln, log(base, x), log10, log2\n")

    print("  POWERS & ROOTS:")
    print("    sqrt, cbrt, x**n\n")

    print("  SPECIAL FUNCTIONS:")
    print("    gamma, digamma, trigamma, polygamma(n, x)")
    print("    erf, erfc, zeta, beta(a, b)\n")

    print("  BESSEL FUNCTIONS:")
    print("    besselj(n, x), bessely(n, x), besseli(n, x), besselk(n, x)\n")

    print("  ORTHOGONAL POLYNOMIALS:")
    print("    hermite(n, x), assoc_legendre(l, m, x)\n")

    print("  SPHERICAL HARMONICS:")
    print("    spherical_harmonic(l, m, theta, phi), ynm(l, m, theta, phi)\n")

    print("  OTHER:")
    print("    abs, signum, sinc, lambertw, floor, ceil, round\n")

    print("  Example Derivatives:")
    examples = [
        ("gamma(x)", "x"),
        ("erf(x)", "x"),
        ("besselj(0, x)", "x"),
        ("atan2(y, x)", "x"),
    ]
    for expr, var in examples:
        result = diff(expr, var)
        print(f"    d/d{var} [{expr}] = {result}")
    print()


# =============================================================================
# SECTION 14: EXPRESSION SYNTAX
# =============================================================================
def section_expression_syntax():
    print("━" * 66)
    print("14. EXPRESSION SYNTAX")
    print("━" * 66 + "\n")

    print("  Elements:")
    print("    Variables:      x, y, sigma, theta, phi")
    print("    Numbers:        1, 3.14, 1e-5")
    print("    Operators:      +, -, *, /, ^")
    print("    Functions:      sin(x), log(10, x)")
    print("    Constants:      pi, e (auto-recognized)")
    print("    Implicit mult:  2x, (x+1)(x-1)")
    print()

    print("  Operator Precedence:")
    print("    Highest: ^ (power, right-associative)")
    print("    Medium:  *, / (left-associative)")
    print("    Lowest:  +, - (left-associative)")
    print()

    print("  Syntax Examples:")
    examples = [
        "x^2 + 3*x + 1",
        "sin(x)^2 + cos(x)^2",
        "2*(x + y)",
        "x^2 * y + y^3",
        "log(10, x)",
        "besselj(0, x)",
        "atan2(y, x)",
    ]
    for expr in examples:
        result = simplify(expr)
        print(f"    {expr:20s} → {result}")
    print()


# =============================================================================
# SECTION 15: ERROR HANDLING
# =============================================================================
def section_error_handling():
    print("━" * 66)
    print("15. ERROR HANDLING")
    print("━" * 66 + "\n")

    print("  All functions return Result<T, DiffError>:")
    print()

    # Example 1: Empty formula
    print("  1. Empty Formula:")
    try:
        result = diff("", "x")
        print(f"     Result: {result}")
    except Exception as e:
        print(f"     Error: {type(e).__name__}")

    # Example 2: Invalid syntax
    print("  2. Invalid Syntax:")
    try:
        result = diff("((", "x")
        print(f"     Result: {result}")
    except Exception as e:
        print(f"     Error: {type(e).__name__}: {e}")

    # Example 3: Invalid number
    print("  3. Invalid Number:")
    try:
        result = diff("x^abc", "x")
        print(f"     Result: {result}")
    except Exception as e:
        print(f"     Error: {type(e).__name__}: {e}")

    # Example 4: Variable conflict (using known_symbols)
    print("  4. Variable Conflict:")
    try:
        # This would be equivalent to passing x in known_symbols and differentiating w.r.t. x
        result = diff("x^2", "x")  # This should work normally
        print(f"     Result: {result}")
        print("     (Note: Python API doesn't expose known_symbols conflict directly)")
    except Exception as e:
        print(f"     Error: {type(e).__name__}: {e}")

    print()
    print("  Common Error Types:")
    print("    EmptyFormula")
    print("    InvalidSyntax")
    print("    InvalidNumber")
    print("    InvalidToken")
    print("    UnexpectedToken")
    print("    UnexpectedEndOfInput")
    print("    InvalidFunctionCall")
    print("    VariableInBothFixedAndDiff")
    print("    NameCollision")
    print("    UnsupportedOperation")
    print()


if __name__ == "__main__":
    main()