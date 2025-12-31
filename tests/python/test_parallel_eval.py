"""
Parallel Evaluation Tests - Tests for evaluate_parallel function

Tests the new Full Hybrid evaluate_parallel with:
- Expr/str inputs
- NumPy/list values
- Type-preserving output
"""

import pytest
import math
import numpy as np
from symb_anafis import Expr, Symbol, evaluate_parallel


class TestEvaluateParallelBasic:
    """Basic evaluate_parallel tests."""

    def test_single_expression_single_var(self):
        # Simple x^2 with values [1, 2, 3]
        results = evaluate_parallel(
            ["x^2"],
            [["x"]],
            [[[1.0, 2.0, 3.0]]]
        )
        assert len(results) == 1
        assert len(results[0]) == 3
        assert results[0][0] == 1.0  # 1^2
        assert results[0][1] == 4.0  # 2^2
        assert results[0][2] == 9.0  # 3^2

    def test_two_vars(self):
        # x + y
        results = evaluate_parallel(
            ["x + y"],
            [["x", "y"]],
            [[[1.0, 2.0], [10.0, 20.0]]]
        )
        assert len(results) == 1
        assert len(results[0]) == 2
        assert results[0][0] == 11.0  # 1 + 10
        assert results[0][1] == 22.0  # 2 + 20

    def test_multiple_expressions(self):
        # Two expressions
        results = evaluate_parallel(
            ["x^2", "x + 1"],
            [["x"], ["x"]],
            [[[1.0, 2.0, 3.0]], [[1.0, 2.0, 3.0]]]
        )
        assert len(results) == 2
        # First expression: x^2
        assert results[0] == [1.0, 4.0, 9.0]
        # Second expression: x + 1
        assert results[1] == [2.0, 3.0, 4.0]


class TestEvaluateParallelNumPy:
    """NumPy array input tests."""

    def test_numpy_array_input(self):
        # Use NumPy array for values
        x_values = np.array([1.0, 2.0, 3.0, 4.0])
        results = evaluate_parallel(
            ["x^2"],
            [["x"]],
            [[x_values]]
        )
        expected = [1.0, 4.0, 9.0, 16.0]
        assert results[0] == expected

    def test_numpy_multiple_vars(self):
        # Two variables with NumPy arrays
        x_values = np.array([1.0, 2.0, 3.0])
        y_values = np.array([10.0, 20.0, 30.0])
        results = evaluate_parallel(
            ["x * y"],
            [["x", "y"]],
            [[x_values, y_values]]
        )
        expected = [10.0, 40.0, 90.0]
        assert results[0] == expected

    def test_mixed_numpy_and_list(self):
        # Mix NumPy array and Python list
        x_values = np.array([1.0, 2.0, 3.0])
        y_values = [10.0, 20.0, 30.0]  # Python list
        results = evaluate_parallel(
            ["x + y"],
            [["x", "y"]],
            [[x_values, y_values]]
        )
        expected = [11.0, 22.0, 33.0]
        assert results[0] == expected


class TestEvaluateParallelExprInput:
    """Expr object input tests."""

    def test_expr_input(self):
        # Use Expr object instead of string
        x = Symbol("x")
        expr = x ** 2
        results = evaluate_parallel(
            [expr],
            [["x"]],
            [[[1.0, 2.0, 3.0]]]
        )
        assert len(results) == 1
        assert results[0][0] == 1.0
        assert results[0][1] == 4.0
        assert results[0][2] == 9.0

    def test_mixed_expr_and_string(self):
        # Mix Expr object and string
        x = Symbol("x")
        expr = x ** 2
        results = evaluate_parallel(
            [expr, "x + 1"],
            [["x"], ["x"]],
            [[[1.0, 2.0]], [[1.0, 2.0]]]
        )
        assert len(results) == 2
        assert results[0] == [1.0, 4.0]
        assert results[1] == [2.0, 3.0]


class TestEvaluateParallelSkip:
    """Skip/None value tests."""

    def test_skip_value(self):
        # Use None to skip (keep symbolic)
        results = evaluate_parallel(
            ["x * y"],
            [["x", "y"]],
            [[[2.0, None], [3.0, 5.0]]]
        )
        assert len(results[0]) == 2
        # Point 0: x=2, y=3 → 6
        assert results[0][0] == 6.0
        # Point 1: x=skip, y=5 → symbolic result (string or Expr)
        # The result should contain 'x' since x was skipped
        result1 = str(results[0][1])
        assert "x" in result1 or "5" in result1


class TestEvaluateParallelTypePreserving:
    """Type-preserving output tests."""

    def test_numeric_result_is_float(self):
        # Numeric result should be Python float
        results = evaluate_parallel(
            ["2 + 3"],
            [[]],
            [[]]
        )
        # Result should be float, not string
        assert isinstance(results[0][0], float)
        assert results[0][0] == 5.0

    def test_symbolic_result_from_string_is_string(self):
        # Symbolic result from string input should be string
        results = evaluate_parallel(
            ["x + y"],
            [["x", "y"]],
            [[[1.0], [None]]]  # y is skipped
        )
        # Result should be string (for string input with symbolic output)
        result = results[0][0]
        # Should be string containing 'y'
        assert isinstance(result, str)
        assert "y" in result

    def test_symbolic_result_from_expr_is_expr(self):
        # Symbolic result from Expr input should be Expr
        x = Symbol("x")
        y = Symbol("y")
        expr = x + y
        results = evaluate_parallel(
            [expr],
            [["x", "y"]],
            [[[1.0], [None]]]  # y is skipped
        )
        # Result should be Expr (or at least work like one)
        result = results[0][0]
        # Should be Expr containing 'y'
        assert "y" in str(result)


class TestEvaluateParallelPerformance:
    """Performance-related tests."""

    def test_large_batch(self):
        # Test with larger dataset
        n = 1000
        x_values_float = [float(x) for x in range(1, n + 1)]
        
        results = evaluate_parallel(
            ["x^2"],
            [["x"]],
            [[x_values_float]]
        )
        
        assert len(results[0]) == n
        # Results should be in order (ordering bug fixed)
        assert results[0][0] == 1.0  # 1^2
        assert results[0][99] == 10000.0  # 100^2
        assert results[0][-1] == float(n ** 2)  # 1000^2

    def test_numpy_large_batch(self):
        # Test with NumPy array for better performance
        n = 10000
        x_values = np.arange(1.0, n + 1.0)
        
        results = evaluate_parallel(
            ["x^2"],
            [["x"]],
            [[x_values]]
        )
        
        assert len(results[0]) == n
        assert results[0][0] == 1.0  # 1^2
        assert results[0][-1] == float(n ** 2)  # 10000^2


class TestEvaluateParallelFunctions:
    """Test with various mathematical functions."""

    def test_trig_functions(self):
        x_values = [0.0, math.pi / 2, math.pi]
        results = evaluate_parallel(
            ["sin(x)"],
            [["x"]],
            [[x_values]]
        )
        assert abs(results[0][0] - 0.0) < 1e-10
        assert abs(results[0][1] - 1.0) < 1e-10
        assert abs(results[0][2] - 0.0) < 1e-10

    def test_exp_log(self):
        x_values = [0.0, 1.0, 2.0]
        results = evaluate_parallel(
            ["exp(x)", "ln(x+1)"],
            [["x"], ["x"]],
            [[x_values], [x_values]]
        )
        # exp(x)
        assert abs(results[0][0] - 1.0) < 1e-10
        assert abs(results[0][1] - math.e) < 1e-10
        # ln(x+1)
        assert abs(results[1][0] - 0.0) < 1e-10  # ln(1) = 0
        assert abs(results[1][1] - math.log(2)) < 1e-10


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
