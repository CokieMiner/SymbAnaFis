#!/usr/bin/env python3
"""
API Showcase: Complete SymbAnaFis Feature Demonstration (Python)

This example demonstrates ALL the capabilities of SymbAnaFis from Python:
- String-based and Object-based APIs
- Differentiation (single variable, multi-variable)
- Simplification
- Numerical Evaluation
- Gradient, Hessian, and Jacobian
- LaTeX and Unicode output
- Custom derivatives
- Safety features and configuration
- Uncertainty propagation
- Parallel evaluation

Run with: python examples/api_showcase.py
"""

from symb_anafis import (
    Expr, Diff, Simplify,
    diff, simplify, parse,
    gradient, hessian, jacobian, evaluate,
    uncertainty_propagation, relative_uncertainty,
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
    print("║          Symbolic Differentiation Library                         ║")
    print("╚══════════════════════════════════════════════════════════════════╝\n")

    part1_string_api()
    part2_object_api()
    part3_numerical_evaluation()
    part4_multi_variable_calculus()
    part5_all_functions()
    part6_custom_derivatives()
    part7_safety_features()
    part8_expression_output()
    part9_uncertainty_propagation()
    if HAS_PARALLEL:
        part10_parallel_evaluation()
    else:
        print("━" * 66)
        print("⚡ PART 10: PARALLEL EVALUATION")
        print("   (Requires 'parallel' feature - not available)")
        print("━" * 66 + "\n")

    print("\n✅ Showcase Complete!")


# =============================================================================
# PART 1: STRING-BASED API
# =============================================================================
def part1_string_api():
    print("━" * 66)
    print("📦 PART 1: STRING-BASED API")
    print("   Best for parsing user input, configuration files, web APIs")
    print("━" * 66 + "\n")

    # 1.1 Simple diff() function
    print("  1.1 Simple Differentiation: diff()")
    formula = "x^3 + 2*x^2 - 5*x + 1"
    print(f"      Formula: {formula}")
    result = diff(formula, "x")
    print(f"      d/dx:    {result}\n")

    # 1.2 Simple simplify() function
    print("  1.2 Simplification: simplify()")
    ugly = "x + x + x + 0*y + 1*z"
    print(f"      Before: {ugly}")
    clean = simplify(ugly)
    print(f"      After:  {clean}\n")

    # 1.3 Diff builder with options
    print("  1.3 Diff Builder with Options")
    d = Diff().fixed_var("a").fixed_var("b").domain_safe(True)
    result = d.diff_str("a*x^2 + b*x + c", "x")
    print(f"      d/dx [ax² + bx + c] with a,b as constants: {result}\n")

    # 1.4 Simplify builder with options
    print("  1.4 Simplify Builder with Options")
    s = Simplify().fixed_var("k").domain_safe(True)
    result = s.simplify_str("k*x + k*y")
    print(f"      Simplified k*x + k*y: {result}\n")


# =============================================================================
# PART 2: OBJECT-BASED API
# =============================================================================
def part2_object_api():
    print("━" * 66)
    print("🐍 PART 2: OBJECT-BASED API")
    print("   Build expressions programmatically in Python")
    print("━" * 66 + "\n")

    # 2.1 Creating symbols
    print("  2.1 Creating Symbols: Expr()")
    x = Expr("x")
    y = Expr("y")
    print("      Created x and y symbols\n")

    # 2.2 Building expressions with operators
    print("  2.2 Building Expressions with Operators (+, -, *, /, **)")
    expr1 = x + y
    expr2 = x * y
    expr3 = x ** 2 + y ** 2
    print(f"      x + y   = {expr1}")
    print(f"      x * y   = {expr2}")
    print(f"      x² + y² = {expr3}\n")

    # 2.3 Building expressions with functions
    print("  2.3 Building Expressions with Functions")
    expr_sin = x.sin()
    expr_cos = x.cos()
    expr_exp = x.exp()
    print(f"      sin(x) = {expr_sin}")
    print(f"      cos(x) = {expr_cos}")
    print(f"      exp(x) = {expr_exp}\n")

    # 2.4 Powers
    print("  2.4 Powers: x.pow(n)")
    squared = x.pow(2)
    cubed = x.pow(3)
    print(f"      x²  = {squared}")
    print(f"      x³  = {cubed}\n")

    # 2.5 Differentiation with Expr
    print("  2.5 Differentiating Expression Objects")
    f = x.pow(3) + (Expr("2") * x).sin()
    print(f"      f(x) = {f}")
    df = Diff().differentiate(f, "x")
    print(f"      f'(x) = {df}\n")

    # 2.6 Expr utility methods
    print("  2.6 Expression Utility Methods")
    complex_expr = x.pow(2) + y.sin()
    print(f"      Expression: {complex_expr}")
    print(f"      Node count: {complex_expr.node_count()}")
    print(f"      Max depth:  {complex_expr.max_depth()}")
    print()


# =============================================================================
# PART 3: NUMERICAL EVALUATION
# =============================================================================
def part3_numerical_evaluation():
    print("━" * 66)
    print("🔢 PART 3: NUMERICAL EVALUATION")
    print("   Evaluate expressions with specific variable values")
    print("━" * 66 + "\n")

    # 3.1 Evaluate Expr with dict
    print("  3.1 Evaluate Expression Objects")
    x = Expr("x")
    y = Expr("y")
    expr = x.pow(2) + y.pow(2)

    result = expr.evaluate({"x": 3.0, "y": 4.0})
    print("      x² + y² at (x=3, y=4)")
    print(f"      Result: {result} (expected: 25)\n")

    # 3.2 Evaluate using evaluate()
    print("  3.2 Evaluate from String: evaluate()")
    result = evaluate("sin(pi/6)^2 + cos(pi/6)^2", [])
    print(f"      sin²(π/6) + cos²(π/6) = {result} (expected: 1)\n")

    # 3.3 Partial evaluation (mixed symbolic/numeric)
    print("  3.3 Partial Evaluation")
    result = evaluate("a*x + b", [("a", 2.0), ("b", 5.0)])
    print(f"      a*x + b with a=2, b=5 → {result}\n")


# =============================================================================
# PART 4: MULTI-VARIABLE CALCULUS
# =============================================================================
def part4_multi_variable_calculus():
    print("━" * 66)
    print("📐 PART 4: MULTI-VARIABLE CALCULUS")
    print("   Gradient, Hessian, and Jacobian computations")
    print("━" * 66 + "\n")

    # 4.1 Gradient
    print("  4.1 Gradient: ∇f = [∂f/∂x, ∂f/∂y, ...]")
    grad = gradient("x^2*y + y^3", ["x", "y"])
    print("      f(x,y) = x²y + y³")
    print(f"      ∂f/∂x = {grad[0]}")
    print(f"      ∂f/∂y = {grad[1]}\n")

    # 4.2 Hessian
    print("  4.2 Hessian Matrix: H[i][j] = ∂²f/∂xᵢ∂xⱼ")
    hess = hessian("x^2*y + y^3", ["x", "y"])
    print("      f = x²y + y³")
    print(f"      H = | {hess[0][0]} {hess[0][1]} |")
    print(f"          | {hess[1][0]} {hess[1][1]} |\n")

    # 4.3 Jacobian
    print("  4.3 Jacobian Matrix: J[i][j] = ∂fᵢ/∂xⱼ")
    jac = jacobian(["x^2 + y", "x*y"], ["x", "y"])
    print("      f₁ = x² + y, f₂ = xy")
    print(f"      J = | {jac[0][0]} {jac[0][1]} |")
    print(f"          | {jac[1][0]} {jac[1][1]} |\n")


# =============================================================================
# PART 5: ALL SUPPORTED FUNCTIONS
# =============================================================================
def part5_all_functions():
    print("━" * 66)
    print("📚 PART 5: ALL SUPPORTED MATHEMATICAL FUNCTIONS")
    print("   60+ functions with symbolic differentiation and numeric eval")
    print("━" * 66 + "\n")

    print("  TRIGONOMETRIC:")
    print("    sin(x), cos(x), tan(x), cot(x), sec(x), csc(x)\n")

    print("  INVERSE TRIG:")
    print("    asin(x), acos(x), atan(x), acot(x), asec(x), acsc(x)\n")

    print("  HYPERBOLIC:")
    print("    sinh(x), cosh(x), tanh(x), coth(x), sech(x), csch(x)\n")

    print("  INVERSE HYPERBOLIC:")
    print("    asinh(x), acosh(x), atanh(x), acoth(x), asech(x), acsch(x)\n")

    print("  EXPONENTIAL & LOGARITHMIC:")
    print("    exp(x), ln(x), log(x), log10(x), log2(x)\n")

    print("  POWERS & ROOTS:")
    print("    sqrt(x), cbrt(x), x**n, x**(1/n)\n")

    print("  SPECIAL FUNCTIONS:")
    print("    gamma(x), digamma(x), trigamma(x), polygamma(n,x)")
    print("    erf(x), erfc(x), zeta(x), beta(a,b)\n")

    print("  BESSEL FUNCTIONS:")
    print("    besselj(n,x), bessely(n,x), besseli(n,x), besselk(n,x)\n")

    print("  OTHER:")
    print("    abs(x), sign(x), sinc(x), lambertw(x)\n")

    # Demonstrate some derivatives
    print("  Example Derivatives:")
    examples = [
        ("gamma(x)", "x"),
        ("erf(x)", "x"),
        ("lambertw(x)", "x"),
    ]
    for expr, var in examples:
        result = diff(expr, var)
        print(f"    d/d{var} [{expr}] = {result}")
    print()


# =============================================================================
# PART 6: CUSTOM DERIVATIVES
# =============================================================================
def part6_custom_derivatives():
    print("━" * 66)
    print("✨ PART 6: CUSTOM DERIVATIVES")
    print("   Define derivative rules for user-defined functions")
    print("━" * 66 + "\n")

    # 6.1 Custom Derivative Rule
    print("  6.1 Custom Derivative Rule: user_fn()")
    print("      Define: my_func(u) with derivative: d/dx[my_func(u)] = 3u² · u'")

    def my_func_partial(args):
        # For f(u), return ∂f/∂u = 3u²
        # args[0] is the first argument expression
        return 3 * args[0] ** 2

    x = Expr("x")
    custom_diff = Diff().user_fn("my_func", 1, my_func_partial)

    # Test the custom derivative
    result = custom_diff.diff_str("my_func(x^2)", "x")
    print(f"      d/dx[my_func(x²)] = {result}")
    print("      Expected: 3*(x^2)^2 * 2*x = 3*x^4 * 2*x = 6*x^5")
    print()


# =============================================================================
# PART 7: SAFETY FEATURES
# =============================================================================
def part7_safety_features():
    print("━" * 66)
    print("🛡️  PART 7: SAFETY FEATURES & CONFIGURATION")
    print("   Prevent resource exhaustion and handle edge cases")
    print("━" * 66 + "\n")

    # 7.1 Max Depth Limit
    print("  7.1 Maximum Expression Depth")
    print("      Diff().max_depth(25) - prevents deeply nested expressions")
    print()

    # 7.2 Max Node Count Limit
    print("  7.2 Maximum Node Count")
    print("      Diff().max_nodes(500) - prevents exponential growth")
    print()

    # 7.3 Domain Safety
    print("  7.3 Domain Safety Mode")
    print("      Diff().domain_safe(True)")
    print("      Prevents simplifications that could introduce undefined values")
    print("      Example: √(x²) = |x| (not x, which fails for x < 0)\n")

    # 7.4 Fixed Variables
    print("  7.4 Fixed Variables (Constants)")
    d = Diff().fixed_var("a").fixed_var("b")
    result = d.diff_str("a*x^2 + b*x + c", "x")
    print("      With a, b as constants:")
    print(f"      d/dx [ax² + bx + c] = {result}")
    print()


# =============================================================================
# PART 8: EXPRESSION OUTPUT FORMATS
# =============================================================================
def part8_expression_output():
    print("━" * 66)
    print("🖨️  PART 8: EXPRESSION OUTPUT FORMATS")
    print("   LaTeX and Unicode output for beautiful display")
    print("━" * 66 + "\n")

    x = Expr("x")
    y = Expr("y")
    alpha = Expr("alpha")
    sigma = Expr("sigma")

    # 8.1 LaTeX Output
    print("  8.1 LaTeX Output: to_latex()")
    expr1 = x.pow(2) / y
    expr2 = x.sin() * alpha.pow(2) + sigma
    print(f"      x²/y        → {expr1.to_latex()}")
    print(f"      sin(x)·α²+σ → {expr2.to_latex()}\n")

    # 8.2 Unicode Output
    print("  8.2 Unicode Output: to_unicode()")
    pi_expr = Expr("pi")
    omega = Expr("omega")
    expr3 = pi_expr + omega.pow(2)
    print(f"      π + ω²  → {expr3.to_unicode()}\n")

    # 8.3 Regular Display
    print("  8.3 Standard Display: str()")
    expr4 = x.sin() + y.cos()
    print(f"      sin(x) + cos(y) → {expr4}\n")


# =============================================================================
# PART 9: UNCERTAINTY PROPAGATION
# =============================================================================
def part9_uncertainty_propagation():
    print("━" * 66)
    print("📊 PART 9: UNCERTAINTY PROPAGATION")
    print("   Calculate error propagation using partial derivatives")
    print("━" * 66 + "\n")

    # 9.1 Basic uncertainty (symbolic variances)
    print("  9.1 Basic Uncertainty: σ_f = √(Σ(∂f/∂xᵢ)²σᵢ²)")
    result = uncertainty_propagation("x + y", ["x", "y"])
    print("      f = x + y")
    print(f"      σ_f = {result}\n")

    # 9.2 Product formula
    print("  9.2 Product Formula Uncertainty")
    result = uncertainty_propagation("x * y", ["x", "y"])
    print("      f = x * y")
    print(f"      σ_f = {result}\n")

    # 9.3 Numeric covariance
    print("  9.3 Numeric Uncertainty Values")
    # σ_x = 0.1 (so σ_x² = 0.01), σ_y = 0.2 (so σ_y² = 0.04)
    result = uncertainty_propagation("x * y", ["x", "y"], [0.01, 0.04])
    print("      f = x * y with σ_x = 0.1, σ_y = 0.2")
    print(f"      σ_f = {result}\n")

    # 9.4 Relative uncertainty
    print("  9.4 Relative Uncertainty: σ_f / |f|")
    result = relative_uncertainty("x^2", ["x"])
    print("      f = x²")
    print(f"      σ_f/|f| = {result}\n")


# =============================================================================
# PART 10: PARALLEL EVALUATION
# =============================================================================
def part10_parallel_evaluation():
    print("━" * 66)
    print("⚡ PART 10: PARALLEL EVALUATION")
    print("   Evaluate multiple expressions at multiple points in parallel")
    print("━" * 66 + "\n")

    # 10.1 Basic parallel eval
    print("  10.1 Basic Parallel Evaluation")
    print("      Evaluate x² at x = 1, 2, 3, 4, 5")
    
    results = evaluate_parallel(
        ["x^2"],
        [["x"]],
        [[[1.0, 2.0, 3.0, 4.0, 5.0]]]
    )
    print(f"      Results: {results[0]}\n")

    # 10.2 Multiple expressions
    print("  10.2 Multiple Expressions in Parallel")
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

    # 10.3 Two variables
    print("  10.3 Two Variables")
    print("      Evaluate x + y at (x=1,y=10), (x=2,y=20), (x=3,y=30)")
    results = evaluate_parallel(
        ["x + y"],
        [["x", "y"]],
        [[[1.0, 2.0, 3.0], [10.0, 20.0, 30.0]]]
    )
    print(f"      Results: {results[0]}\n")

    # 10.4 SKIP for partial evaluation
    print("  10.4 SKIP for Partial Symbolic Evaluation")
    print("      Evaluate x*y with x=2,SKIP,4 and y=3,5,6")
    results = evaluate_parallel(
        ["x * y"],
        [["x", "y"]],
        [[[2.0, None, 4.0], [3.0, 5.0, 6.0]]]  # None = SKIP
    )
    print(f"      Point 0: x=2, y=3 → {results[0][0]}")
    print(f"      Point 1: x=SKIP, y=5 → {results[0][1]} (symbolic!)")
    print(f"      Point 2: x=4, y=6 → {results[0][2]}\n")


if __name__ == "__main__":
    main()

