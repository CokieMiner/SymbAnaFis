
import pytest
import numpy as np
import symb_anafis as sa

def test_eval_f64_returns_numpy_array():
    """Verify eval_f64 returns a 2D NumPy array."""
    # Create expressions
    expr1 = sa.parse("x^2")
    expr2 = sa.parse("x + y")
    
    # Create input data (numpy arrays)
    x = np.array([1.0, 2.0, 3.0], dtype=np.float64)
    y = np.array([10.0, 20.0, 30.0], dtype=np.float64)
    
    # 3D data structure: [expr][var][points]
    # Expr 1: x^2 (depends on x)
    # Expr 2: x + y (depends on x, y)
    data = [
        [x],         # For expr1
        [x, y]       # For expr2
    ]
    
    # Run evaluation
    results = sa.eval_f64(
        [expr1, expr2],
        [["x"], ["x", "y"]],
        data
    )
    
    # Check return type
    assert isinstance(results, np.ndarray), f"Expected numpy.ndarray, got {type(results)}"
    assert results.ndim == 2, f"Expected 2 dimensions, got {results.ndim}"
    assert results.shape == (2, 3), f"Expected shape (2, 3), got {results.shape}"
    assert results.dtype == np.float64, f"Expected float64, got {results.dtype}"
    
    # Check values
    expected1 = x**2
    expected2 = x + y
    
    np.testing.assert_allclose(results[0], expected1, err_msg="Expr1 results mismatch")
    np.testing.assert_allclose(results[1], expected2, err_msg="Expr2 results mismatch")

def test_eval_f64_empty_input():
    """Verify eval_f64 handles empty input gracefully (returns list by default)."""
    results = sa.eval_f64([], [], [])
    assert isinstance(results, list)
    assert len(results) == 0

def test_eval_f64_empty_numpy_input():
    """Verify eval_f64 returns empty NumPy array if input suggests NumPy."""
    # Even if empty, if the structure implies numpy, we might want numpy,
    # but strictly speaking if we provide an empty array as data...
    
    expr = sa.parse("x")
    # shape (1,) empty array
    x_data = np.array([], dtype=np.float64)
    data = [[x_data]]
    
    results = sa.eval_f64([expr], [["x"]], data)
    assert isinstance(results, np.ndarray)
    assert results.size == 0
    assert results.shape == (1, 0)

def test_eval_f64_with_python_lists():
    """Verify eval_f64 accepts Python lists but returns NumPy array."""
    expr = sa.parse("x * 2")
    
    # Input as pure Python lists
    x = [1.0, 2.0, 3.0]
    data = [[x]]
    
    results = sa.eval_f64([expr], [["x"]], data)
    
    # Output should be standard Python list (hybrid return type)
    assert isinstance(results, list)
    assert isinstance(results[0], list)
    assert results[0] == [2.0, 4.0, 6.0]

def test_compiled_evaluator_hybrid_returns():
    """Verify VmEvaluator.eval_batch respects input types."""
    expr = sa.parse("x^2")
    evaluator = sa.VmEvaluator(expr, ["x"])
    
    # CASE 1: NumPy Input -> NumPy Output
    x_np = np.array([1.0, 2.0, 3.0], dtype=np.float64)
    res_np = evaluator.eval_batch([x_np])
    assert isinstance(res_np, np.ndarray), "NumPy input should return NumPy array"
    np.testing.assert_allclose(res_np, [1.0, 4.0, 9.0])
    
    # CASE 2: List Input -> List Output
    x_list = [1.0, 2.0, 3.0]
    res_list = evaluator.eval_batch([x_list])
    assert isinstance(res_list, list), "List input should return list"
    assert res_list == [1.0, 4.0, 9.0]

def test_compiled_evaluator_eval_batch_consistency():
    """Verify VmEvaluator.eval_batch still returns correct NumPy array."""
    expr = sa.parse("sin(x)")
    evaluator = sa.VmEvaluator(expr, ["x"])
    
    x = np.array([0.0, np.pi/2, np.pi], dtype=np.float64)
    result = evaluator.eval_batch([x])
    
    assert isinstance(result, np.ndarray)
    assert result.shape == (3,)
    np.testing.assert_allclose(result, np.sin(x), atol=1e-10)
