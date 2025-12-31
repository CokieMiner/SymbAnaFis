"""
Compiled Evaluator Tests - Tests for CompiledEvaluator Python bindings

Tests batch evaluation, parallel evaluation, and NumPy integration.
"""

import pytest
import math
import numpy as np
from symb_anafis import CompiledEvaluator, Expr, Symbol, parse

EPSILON = 1e-10


def approx_eq(a: float, b: float, eps: float = EPSILON) -> bool:
    """Check if two floats are approximately equal."""
    if math.isnan(a) and math.isnan(b):
        return True
    if math.isinf(a) and math.isinf(b):
        return (a > 0) == (b > 0)
    return abs(a - b) < eps * max(abs(a), abs(b), 1.0)


class TestCompiledEvaluatorBasic:
    """Basic CompiledEvaluator tests."""

    def test_simple_expression(self):
        expr = parse("x^2 + 1")
        evaluator = CompiledEvaluator(expr, ["x"])
        
        result = evaluator.evaluate([2.0])
        assert approx_eq(result, 5.0)  # 2^2 + 1

    def test_two_variables(self):
        expr = parse("x + y")
        evaluator = CompiledEvaluator(expr, ["x", "y"])
        
        result = evaluator.evaluate([3.0, 4.0])
        assert approx_eq(result, 7.0)

    def test_trig_functions(self):
        expr = parse("sin(x)")
        evaluator = CompiledEvaluator(expr, ["x"])
        
        result = evaluator.evaluate([math.pi / 2])
        assert approx_eq(result, 1.0)

    def test_exp_log(self):
        expr = parse("exp(x)")
        evaluator = CompiledEvaluator(expr, ["x"])
        
        result = evaluator.evaluate([1.0])
        assert approx_eq(result, math.e)


class TestCompiledEvaluatorBatch:
    """Batch evaluation tests."""

    def test_eval_batch_simple(self):
        expr = parse("x^2")
        evaluator = CompiledEvaluator(expr, ["x"])
        
        # Batch evaluate for x = 1, 2, 3, 4
        x_values = [1.0, 2.0, 3.0, 4.0]
        results = evaluator.eval_batch([x_values])
        
        expected = [1.0, 4.0, 9.0, 16.0]
        assert len(results) == len(expected)
        for i, (r, e) in enumerate(zip(results, expected)):
            assert approx_eq(r, e), f"Index {i}: expected {e}, got {r}"

    def test_eval_batch_two_vars(self):
        expr = parse("x * y")
        evaluator = CompiledEvaluator(expr, ["x", "y"])
        
        x_values = [1.0, 2.0, 3.0]
        y_values = [10.0, 20.0, 30.0]
        results = evaluator.eval_batch([x_values, y_values])
        
        expected = [10.0, 40.0, 90.0]
        for i, (r, e) in enumerate(zip(results, expected)):
            assert approx_eq(r, e), f"Index {i}: expected {e}, got {r}"

    def test_eval_batch_numpy(self):
        expr = parse("x^2 + x")
        evaluator = CompiledEvaluator(expr, ["x"])
        
        x_values = np.array([1.0, 2.0, 3.0, 4.0, 5.0])
        results = evaluator.eval_batch([x_values])
        
        expected = x_values ** 2 + x_values  # [2, 6, 12, 20, 30]
        for i, (r, e) in enumerate(zip(results, expected)):
            assert approx_eq(r, e), f"Index {i}: expected {e}, got {r}"

    def test_eval_batch_large(self):
        expr = parse("sin(x) + cos(x)")
        evaluator = CompiledEvaluator(expr, ["x"])
        
        n = 10000
        x_values = np.linspace(0, 2 * math.pi, n)
        results = evaluator.eval_batch([x_values])
        
        assert len(results) == n
        # Check first and last
        assert approx_eq(results[0], 1.0)  # sin(0) + cos(0) = 0 + 1 = 1


class TestCompiledEvaluatorFunctions:
    """Tests for various function support."""

    def test_bessel_j(self):
        expr = parse("besselj(0, x)")
        evaluator = CompiledEvaluator(expr, ["x"])
        
        result = evaluator.evaluate([0.0])
        assert approx_eq(result, 1.0)  # J_0(0) = 1

    def test_gamma(self):
        expr = parse("gamma(x)")
        evaluator = CompiledEvaluator(expr, ["x"])
        
        result = evaluator.evaluate([5.0])
        assert approx_eq(result, 24.0)  # Γ(5) = 4! = 24

    def test_erf(self):
        expr = parse("erf(x)")
        evaluator = CompiledEvaluator(expr, ["x"])
        
        result = evaluator.evaluate([0.0])
        assert approx_eq(result, 0.0)

    def test_complex_expression(self):
        expr = parse("sin(x)^2 + cos(x)^2")
        evaluator = CompiledEvaluator(expr, ["x"])
        
        # Should always be 1 (Pythagorean identity)
        for x in [0.0, 0.5, 1.0, 2.0, 3.14159]:
            result = evaluator.evaluate([x])
            assert approx_eq(result, 1.0), f"At x={x}, got {result}"


class TestCompiledEvaluatorFromSymbols:
    """Tests building expressions from Symbols."""

    def test_symbol_expression(self):
        x = Symbol("x")
        expr = x ** 2 + x * 3 + 1
        evaluator = CompiledEvaluator(expr, ["x"])
        
        result = evaluator.evaluate([2.0])
        assert approx_eq(result, 11.0)  # 4 + 6 + 1 = 11

    def test_two_symbol_expression(self):
        x = Symbol("x")
        y = Symbol("y")
        expr = x * y + x + y
        evaluator = CompiledEvaluator(expr, ["x", "y"])
        
        result = evaluator.evaluate([2.0, 3.0])
        assert approx_eq(result, 11.0)  # 6 + 2 + 3 = 11

    def test_function_call(self):
        x = Symbol("x")
        expr = x.sin() + x.cos()
        evaluator = CompiledEvaluator(expr, ["x"])
        
        result = evaluator.evaluate([0.0])
        assert approx_eq(result, 1.0)  # sin(0) + cos(0) = 0 + 1


class TestCompiledEvaluatorEdgeCases:
    """Edge case tests."""

    def test_constant_expression(self):
        expr = parse("42")
        evaluator = CompiledEvaluator(expr, [])
        
        result = evaluator.evaluate([])
        assert approx_eq(result, 42.0)

    def test_pi_constant(self):
        expr = parse("sin(pi/2)")
        evaluator = CompiledEvaluator(expr, [])
        
        result = evaluator.evaluate([])
        assert approx_eq(result, 1.0)

    def test_e_constant(self):
        expr = parse("ln(e)")
        evaluator = CompiledEvaluator(expr, [])
        
        result = evaluator.evaluate([])
        assert approx_eq(result, 1.0)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
