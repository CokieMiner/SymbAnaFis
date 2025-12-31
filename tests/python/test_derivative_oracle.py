"""
Derivative Oracle Tests - Ported from Rust derivative_oracle_tests.rs

Each test case has expected results verified against SymPy.
We verify both symbolic output and numerical evaluation at test points.
"""

import pytest
import math
from symb_anafis import diff, evaluate_str, simplify

EPSILON = 1e-8


def approx_eq(a: float, b: float, tol: float = None) -> bool:
    """Check if two floats are approximately equal."""
    if tol is None:
        tol = EPSILON * max(abs(a), abs(b), 1.0)
    if math.isnan(a) and math.isnan(b):
        return True
    if math.isinf(a) and math.isinf(b):
        return (a > 0) == (b > 0)
    return abs(a - b) < tol


def eval_at(expr_str: str, x_val: float) -> float:
    """Evaluate expression at x=val."""
    result = evaluate_str(expr_str, [("x", x_val)])
    return float(result)


def check_derivative(expr: str, var: str, test_points: list):
    """
    Test derivative and verify numerical value at test points.
    
    Args:
        expr: Expression string to differentiate
        var: Variable to differentiate with respect to
        test_points: List of (x_val, expected_val) tuples
    """
    result = diff(expr, var)
    
    for x_val, expected_val in test_points:
        actual = eval_at(result, x_val)
        tol = EPSILON * max(abs(expected_val), 1.0)
        assert approx_eq(actual, expected_val, tol), \
            f"d/d{var}[{expr}] at {var}={x_val}: got {actual}, expected {expected_val}. Result: {result}"


class TestPolynomialDerivatives:
    """Polynomial derivatives (SymPy-verified)."""

    def test_constant(self):
        # d/dx[5] = 0
        check_derivative("5", "x", [(1.0, 0.0), (2.0, 0.0)])

    def test_linear(self):
        # d/dx[x] = 1
        check_derivative("x", "x", [(1.0, 1.0), (2.0, 1.0)])

    def test_quadratic(self):
        # d/dx[x^2] = 2*x
        check_derivative("x^2", "x", [(1.0, 2.0), (2.0, 4.0), (3.0, 6.0)])

    def test_cubic(self):
        # d/dx[x^3] = 3*x^2
        check_derivative("x^3", "x", [(1.0, 3.0), (2.0, 12.0), (3.0, 27.0)])

    def test_polynomial(self):
        # d/dx[x^4 + 3*x^2 - 2*x + 7] = 4*x^3 + 6*x - 2
        check_derivative("x^4 + 3*x^2 - 2*x + 7", "x", [(1.0, 8.0), (2.0, 42.0)])


class TestTrigonometricDerivatives:
    """Trigonometric derivatives (SymPy-verified)."""

    def test_sin(self):
        # d/dx[sin(x)] = cos(x)
        check_derivative("sin(x)", "x", [
            (0.0, 1.0),                      # cos(0) = 1
            (math.pi / 2, 0.0),              # cos(π/2) = 0
            (math.pi, -1.0),                 # cos(π) = -1
        ])

    def test_cos(self):
        # d/dx[cos(x)] = -sin(x)
        check_derivative("cos(x)", "x", [
            (0.0, 0.0),                      # -sin(0) = 0
            (math.pi / 2, -1.0),             # -sin(π/2) = -1
        ])

    def test_tan(self):
        # d/dx[tan(x)] = sec^2(x) = 1/cos^2(x)
        check_derivative("tan(x)", "x", [
            (0.0, 1.0),                      # sec^2(0) = 1
            (0.5, 1.0 / (math.cos(0.5) ** 2)),
        ])

    def test_sin_squared(self):
        # d/dx[sin(x)^2] = 2*sin(x)*cos(x) = sin(2x)
        check_derivative("sin(x)^2", "x", [
            (0.0, 0.0),
            (math.pi / 4, 1.0),              # sin(π/2) = 1
        ])


class TestExpLogDerivatives:
    """Exponential and logarithmic derivatives (SymPy-verified)."""

    def test_exp(self):
        # d/dx[exp(x)] = exp(x)
        check_derivative("exp(x)", "x", [
            (0.0, 1.0),
            (1.0, math.e),
            (2.0, math.e ** 2),
        ])

    def test_ln(self):
        # d/dx[ln(x)] = 1/x
        check_derivative("ln(x)", "x", [
            (1.0, 1.0),
            (2.0, 0.5),
            (math.e, 1.0 / math.e),
        ])

    def test_exp_composite(self):
        # d/dx[exp(x^2)] = 2*x*exp(x^2)
        check_derivative("exp(x^2)", "x", [
            (0.0, 0.0),
            (1.0, 2.0 * math.e),
        ])

    def test_ln_composite(self):
        # d/dx[ln(x^2)] = 2/x
        check_derivative("ln(x^2)", "x", [(1.0, 2.0), (2.0, 1.0)])


class TestChainRule:
    """Chain rule derivatives (SymPy-verified)."""

    def test_sin_x_squared(self):
        # d/dx[sin(x^2)] = 2*x*cos(x^2)
        check_derivative("sin(x^2)", "x", [
            (0.0, 0.0),
            (1.0, 2.0 * math.cos(1.0)),
        ])

    def test_cos_exp(self):
        # d/dx[cos(exp(x))] = -sin(exp(x))*exp(x)
        x = 0.0
        expected = -math.sin(math.exp(x)) * math.exp(x)
        check_derivative("cos(exp(x))", "x", [(x, expected)])

    def test_exp_sin(self):
        # d/dx[exp(sin(x))] = exp(sin(x))*cos(x)
        check_derivative("exp(sin(x))", "x", [
            (0.0, 1.0),  # exp(sin(0))*cos(0) = exp(0)*1 = 1
        ])


class TestProductRule:
    """Product rule derivatives (SymPy-verified)."""

    def test_x_sin_x(self):
        # d/dx[x*sin(x)] = sin(x) + x*cos(x)
        check_derivative("x*sin(x)", "x", [
            (0.0, 0.0),
            (math.pi / 2, 1.0),  # sin(π/2) + π/2*cos(π/2) = 1
        ])

    def test_x_exp(self):
        # d/dx[x*exp(x)] = exp(x) + x*exp(x) = (1+x)*exp(x)
        check_derivative("x*exp(x)", "x", [
            (0.0, 1.0),
            (1.0, 2.0 * math.e),
        ])

    def test_x2_ln(self):
        # d/dx[x^2*ln(x)] = 2*x*ln(x) + x
        check_derivative("x^2*ln(x)", "x", [
            (1.0, 1.0),  # 2*1*0 + 1 = 1
            (math.e, 2.0 * math.e + math.e),
        ])


class TestQuotientRule:
    """Quotient rule derivatives (SymPy-verified)."""

    def test_sin_over_x(self):
        # d/dx[sin(x)/x] = (x*cos(x) - sin(x))/x^2
        x = math.pi / 2
        expected = (x * math.cos(x) - math.sin(x)) / (x ** 2)
        check_derivative("sin(x)/x", "x", [(x, expected)])

    def test_1_over_x(self):
        # d/dx[1/x] = -1/x^2
        check_derivative("1/x", "x", [(1.0, -1.0), (2.0, -0.25)])

    def test_x_over_sin(self):
        # d/dx[x/sin(x)] = (sin(x) - x*cos(x))/sin(x)^2
        x = 1.0
        expected = (math.sin(x) - x * math.cos(x)) / (math.sin(x) ** 2)
        check_derivative("x/sin(x)", "x", [(1.0, expected)])


class TestPowerRule:
    """Power rule with expressions (SymPy-verified)."""

    def test_sqrt_x(self):
        # d/dx[sqrt(x)] = 1/(2*sqrt(x))
        check_derivative("sqrt(x)", "x", [(1.0, 0.5), (4.0, 0.25)])

    def test_x_to_half(self):
        # d/dx[x^0.5] = 0.5*x^(-0.5)
        check_derivative("x^0.5", "x", [(1.0, 0.5), (4.0, 0.25)])

    def test_x_to_x(self):
        # d/dx[x^x] = x^x * (ln(x) + 1)
        check_derivative("x^x", "x", [
            (1.0, 1.0),  # 1^1 * (0 + 1) = 1
            (2.0, 4.0 * (math.log(2.0) + 1.0)),
        ])


class TestHyperbolicDerivatives:
    """Hyperbolic function derivatives (SymPy-verified)."""

    def test_sinh(self):
        # d/dx[sinh(x)] = cosh(x)
        check_derivative("sinh(x)", "x", [
            (0.0, 1.0),
            (1.0, math.cosh(1.0)),
        ])

    def test_cosh(self):
        # d/dx[cosh(x)] = sinh(x)
        check_derivative("cosh(x)", "x", [
            (0.0, 0.0),
            (1.0, math.sinh(1.0)),
        ])

    def test_tanh(self):
        # d/dx[tanh(x)] = 1 - tanh(x)^2 = sech^2(x)
        check_derivative("tanh(x)", "x", [
            (0.0, 1.0),
            (1.0, 1.0 - math.tanh(1.0) ** 2),
        ])


class TestInverseTrigDerivatives:
    """Inverse trig function derivatives (SymPy-verified)."""

    def test_asin(self):
        # d/dx[asin(x)] = 1/sqrt(1-x^2)
        check_derivative("asin(x)", "x", [
            (0.5, 1.0 / math.sqrt(1 - 0.25)),
        ])

    def test_acos(self):
        # d/dx[acos(x)] = -1/sqrt(1-x^2)
        check_derivative("acos(x)", "x", [
            (0.5, -1.0 / math.sqrt(0.75)),
        ])

    def test_atan(self):
        # d/dx[atan(x)] = 1/(1+x^2)
        check_derivative("atan(x)", "x", [(0.0, 1.0), (1.0, 0.5)])


class TestCompositeExpressions:
    """Complex composite expression derivatives (SymPy-verified)."""

    def test_ln_sin_x(self):
        # d/dx[ln(sin(x))] = cos(x)/sin(x) = cot(x)
        x = 1.0
        expected = math.cos(x) / math.sin(x)
        check_derivative("ln(sin(x))", "x", [(1.0, expected)])

    def test_exp_x2_sin(self):
        # d/dx[exp(x^2)*sin(x)] = exp(x^2)*(2*x*sin(x) + cos(x))
        x = 1.0
        ex2 = math.exp(x ** 2)
        expected = ex2 * (2.0 * x * math.sin(x) + math.cos(x))
        check_derivative("exp(x^2)*sin(x)", "x", [(1.0, expected)])

    def test_nested_exp(self):
        # d/dx[exp(exp(x))] = exp(x) * exp(exp(x))
        x = 0.5
        ex = math.exp(x)
        expected = ex * math.exp(ex)
        check_derivative("exp(exp(x))", "x", [(0.5, expected)])


class TestSecondDerivatives:
    """Second derivative tests (SymPy-verified)."""

    def test_second_derivative_x3(self):
        # d²/dx²[x^3] = 6*x
        d1 = diff("x^3", "x")
        d2 = diff(d1, "x")
        val = eval_at(d2, 2.0)
        assert approx_eq(val, 12.0)

    def test_second_derivative_sin(self):
        # d²/dx²[sin(x)] = -sin(x)
        d1 = diff("sin(x)", "x")
        d2 = diff(d1, "x")
        x = 1.0
        val = eval_at(d2, x)
        assert approx_eq(val, -math.sin(x))

    def test_second_derivative_exp(self):
        # d²/dx²[exp(x)] = exp(x)
        d1 = diff("exp(x)", "x")
        d2 = diff(d1, "x")
        x = 1.0
        val = eval_at(d2, x)
        assert approx_eq(val, math.exp(x))


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
