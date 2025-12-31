"""
Comprehensive API Tests - Ported from Rust comprehensive_api_tests.rs

Tests gradient, hessian, jacobian APIs and integrated workflows.
"""

import pytest
import math
from symb_anafis import (
    Expr, Symbol, Diff, Simplify, CompiledEvaluator,
    diff, simplify, parse, symb,
    gradient, gradient_str, hessian, hessian_str, jacobian, jacobian_str,
    evaluate_str
)

EPSILON = 1e-9
LOOSE_EPSILON = 1e-4


def approx_eq(a: float, b: float, eps: float = EPSILON) -> bool:
    """Check if two floats are approximately equal."""
    if math.isnan(a) and math.isnan(b):
        return True
    if math.isinf(a) and math.isinf(b):
        return (a > 0) == (b > 0)
    return abs(a - b) < eps or abs(a - b) < eps * max(abs(a), abs(b))


class TestDiffFunction:
    """Tests for the diff() function."""

    def test_diff_basic(self):
        result = diff("x^2", "x")
        assert "2" in result and "x" in result

    def test_diff_with_fixed_vars(self):
        result = diff("a*x^2", "x", known_symbols=["a"])
        assert "a" in result

    def test_diff_with_custom_functions(self):
        result = diff("f(x)", "x", custom_functions=["f"])
        assert "f'" in result or "∂" in result or "diff" in result


class TestSimplifyFunction:
    """Tests for the simplify() function."""

    def test_simplify_basic(self):
        result = simplify("x + x")
        assert "2" in result and "x" in result

    def test_simplify_with_fixed_vars(self):
        result = simplify("a + a", known_symbols=["a"])
        assert "2" in result and "a" in result


class TestDiffBuilder:
    """Tests for the Diff builder class."""

    def test_diff_builder_basic(self):
        d = Diff()
        result = d.diff_str("x^3", "x")
        assert "3" in result and "x" in result

    def test_diff_builder_domain_safe(self):
        d = Diff().domain_safe(True)
        result = d.diff_str("sqrt(x)", "x")
        assert "sqrt" in result or "1/" in result

    def test_diff_builder_known_symbols(self):
        # Use fixed_vars method instead of known_symbols parameter
        builder = Diff()
        builder.fixed_vars([symb("alpha"), symb("beta")])
        result = builder.diff_str("alpha*x + beta", "x")
        assert "alpha" in result

    def test_diff_builder_differentiate_expr(self):
        x = Symbol("x")
        expr = x.to_expr() ** 2.0
        d = Diff()
        result = d.differentiate(expr, x)
        s = str(result)
        assert "2" in s and "x" in s


class TestSimplifyBuilder:
    """Tests for the Simplify builder class."""

    def test_simplify_builder_basic(self):
        s = Simplify()
        result = s.simplify_str("x*1 + 0")
        assert result == "x"

    def test_simplify_builder_domain_safe(self):
        s = Simplify().domain_safe(True)
        result = s.simplify_str("(x^2)^(1/2)")
        assert "x" in result

    def test_simplify_builder_simplify_expr(self):
        x = Symbol("x")
        expr = x.to_expr() + 0.0
        s = Simplify()
        result = s.simplify(expr)
        assert str(result) == "x"


class TestSymbolCreation:
    """Tests for symbol creation and arithmetic."""

    def test_sym_creation(self):
        x = Symbol("x")
        expr = x.to_expr()
        assert str(expr) == "x"

    def test_symbol_arithmetic(self):
        x = Symbol("x")
        y = Symbol("y")

        # Addition
        sum_expr = x + y
        s = str(sum_expr)
        assert "x" in s and "y" in s

        # Multiplication
        prod = x * y
        s = str(prod)
        assert "x" in s and "y" in s

        # Power
        pow_expr = x.to_expr() ** 2.0
        s = str(pow_expr)
        assert "x" in s and "2" in s

    def test_symbol_functions(self):
        x = Symbol("x")
        # All should work without raising
        _ = x.sin()
        _ = x.cos()
        _ = x.exp()
        _ = x.ln()
        _ = x.sqrt()


class TestExprMethods:
    """Tests for Expr methods."""

    def test_expr_evaluate_numeric(self):
        # Use evaluate_str for evaluation
        result = evaluate_str("2 + 3", [])
        assert float(result) == 5.0

    def test_expr_substitute(self):
        x = Symbol("x")
        expr = x.to_expr() + 1.0
        substituted = expr.substitute("x", Expr(5.0))

        # Use CompiledEvaluator to get numeric result
        compiled = CompiledEvaluator(substituted, [])
        result = compiled.evaluate([])
        assert result == 6.0

    def test_expr_node_count(self):
        x = Expr("x")
        assert x.node_count() == 1

        expr = x + Expr(1.0)
        assert expr.node_count() >= 2  # At least Add + children

    def test_expr_max_depth(self):
        x = Expr("x")
        assert x.max_depth() >= 1

        expr = x + Expr(1.0)
        assert expr.max_depth() >= 2  # Add(x, 1)


class TestGradientHessianJacobian:
    """Tests for gradient, hessian, jacobian APIs."""

    def test_gradient_str(self):
        grad = gradient_str("x^2 + y^2", ["x", "y"])
        assert len(grad) == 2
        # ∂/∂x = 2x
        s = grad[0]
        assert "2" in s and "x" in s

    def test_hessian_str(self):
        hess = hessian_str("x^3", ["x"])
        assert len(hess) == 1  # 1x1 matrix
        assert len(hess[0]) == 1
        # ∂²/∂x² of x³ = 6x
        s = hess[0][0]
        assert "6" in s and "x" in s

    def test_jacobian_str(self):
        jac = jacobian_str(["x*y", "x+y"], ["x", "y"])
        assert len(jac) == 2  # 2 rows
        assert len(jac[0]) == 2  # 2 columns

    def test_evaluate_str(self):
        # evaluate_str takes a list of (name, value) tuples
        result = evaluate_str("x^2 + 1", [("x", 3.0)])
        assert result == "10"


class TestIntegratedWorkflows:
    """Full workflow integration tests."""

    def test_diff_then_evaluate_with_substitution(self):
        """Build expression, differentiate, then evaluate with values."""
        # Differentiate x^2 * y + sin(x) with respect to x
        derivative = diff("x^2 * y + sin(x)", "x", known_symbols=["y"])
        # d/dx(x² * y + sin(x)) = 2xy + cos(x)

        # Evaluate at x=π, y=3 using evaluate_str
        result = evaluate_str(derivative, [("x", math.pi), ("y", 3.0)])
        val = float(result)
        # At x=π, y=3: 2*π*3 + cos(π) = 6π - 1 ≈ 17.85
        expected = 2.0 * math.pi * 3.0 + math.cos(math.pi)
        assert approx_eq(val, expected, LOOSE_EPSILON)

    def test_full_hessian_workflow(self):
        """Build expression, compute hessian, evaluate elements."""
        # expr = x² * y + y³
        # Hessian of f(x,y) = x²y + y³:
        # H = [[2y, 2x], [2x, 6y]]
        hess = hessian_str("x^2 * y + y^3", ["x", "y"])
        assert len(hess) == 2
        assert len(hess[0]) == 2

        vars_dict = [("x", 2.0), ("y", 3.0)]

        # H[0][0] = 2y = 6
        h00 = float(evaluate_str(hess[0][0], vars_dict))
        assert approx_eq(h00, 6.0), f"H[0][0] = 2y = 6, got {h00}"

        # H[0][1] = 2x = 4
        h01 = float(evaluate_str(hess[0][1], vars_dict))
        assert approx_eq(h01, 4.0), f"H[0][1] = 2x = 4, got {h01}"

        # H[1][1] = 6y = 18
        h11 = float(evaluate_str(hess[1][1], vars_dict))
        assert approx_eq(h11, 18.0), f"H[1][1] = 6y = 18, got {h11}"

        # Verify symmetry: H[0][1] = H[1][0]
        h10 = float(evaluate_str(hess[1][0], vars_dict))
        assert approx_eq(h10, 4.0), f"H[1][0] = H[0][1] = 4, got {h10}"

    def test_gradient_evaluate_magnitude(self):
        """Gradient → evaluate → compute magnitude."""
        # f(x, y) = x² + y² (paraboloid)
        # Gradient: [2x, 2y]
        grad = gradient_str("x^2 + y^2", ["x", "y"])
        assert len(grad) == 2

        # Evaluate at (3, 4)
        vars_list = [("x", 3.0), ("y", 4.0)]
        gx = float(evaluate_str(grad[0], vars_list))
        gy = float(evaluate_str(grad[1], vars_list))

        # Gradient at (3,4) = [6, 8], magnitude = sqrt(36+64) = 10
        magnitude = math.sqrt(gx**2 + gy**2)
        assert approx_eq(magnitude, 10.0), f"Gradient magnitude should be 10, got {magnitude}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
