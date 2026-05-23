"""
Tests for NumPy zero-copy integration.
"""
import unittest
try:
    import numpy as np
except ImportError:
    np = None

from symb_anafis import Expr, eval_f64, VmEvaluator, parse

class TestNumpyIntegration(unittest.TestCase):
    def setUp(self):
        if np is None:
            self.skipTest("NumPy not installed")

    def test_eval_batch_numpy(self):
        """Test VmEvaluator.eval_batch with NumPy array."""
        expr = parse("x^2 + 1")
        compiled = VmEvaluator(expr, ["x"])
        
        # Create array
        x = np.array([0.0, 1.0, 2.0, 3.0], dtype=np.float64)
        
        # Evaluate
        result = compiled.eval_batch([x])
        
        # Verify result is numpy array
        self.assertTrue(isinstance(result, np.ndarray))
        
        # Check values
        expected = x**2 + 1.0
        np.testing.assert_array_almost_equal(result, expected)

    def test_eval_f64_numpy(self):
        """Test eval_f64 with NumPy arrays (parallel tree walk)."""
        # Expressions
        exprs = [Expr("x") + Expr("y"), Expr("x") * Expr("y")]
        
        # Data
        n = 1000
        x = np.linspace(0, 1, n)
        y = np.linspace(1, 2, n)
        
        # Prepare input structure: list of lists of arrays
        # Entry 0: data for "x + y" -> [x, y]
        # Entry 1: data for "x * y" -> [x, y]
        data = [
            [x, y],
            [x, y]
        ]
        
        var_names = [["x", "y"], ["x", "y"]]
        
        # Evaluate
        # Note: eval_f64 now returns numpy arrays if input is numpy
        results = eval_f64(exprs, var_names, data)
        
        self.assertTrue(isinstance(results, np.ndarray))
        self.assertEqual(results.shape, (2, 1000))
        
        # Verify values
        expected_sum = x + y
        expected_prod = x * y
        
        np.testing.assert_array_almost_equal(results[0], expected_sum)
        np.testing.assert_array_almost_equal(results[1], expected_prod)

    def test_eval_batch_multivar(self):
        """Test VmEvaluator with multiple columns."""
        expr = parse("x * y")
        compiled = VmEvaluator(expr, ["x", "y"])
        
        x = np.array([1.0, 2.0], dtype=np.float64)
        y = np.array([3.0, 4.0], dtype=np.float64)
        
        result = compiled.eval_batch([x, y])
        
        expected = x * y
        np.testing.assert_array_almost_equal(result, expected)

    def test_eval_batch_mixed_strides(self):
        """Test with sliced/strided arrays to verify generic array support."""
        expr = parse("x")
        compiled = VmEvaluator(expr, ["x"])
        
        # Create strided array (slice)
        full = np.array([0.0, 1.0, 2.0, 3.0, 4.0], dtype=np.float64)
        x = full[::2] # [0, 2, 4]
        
        # Verify that non-contiguous arrays raise ValueError
        # (Our current zero-copy implementation enforces C-contiguous requirement)
        with self.assertRaisesRegex(ValueError, "C-contiguous"):
             compiled.eval_batch([x])

    def test_eval_batch_list_fallback(self):
        """Test VmEvaluator.eval_batch with Python Lists (fallback path)."""
        expr = parse("x^2")
        compiled = VmEvaluator(expr, ["x"])
        
        # Pass Python list
        data = [1.0, 2.0, 3.0, 4.0]
        results = compiled.eval_batch([data])
        
        # Should work and return Python list (hybrid return type)
        self.assertTrue(isinstance(results, list))
        self.assertEqual(results, [1.0, 4.0, 9.0, 16.0])

    def test_evaluate_single_point_numpy(self):
        """Test VmEvaluator.evaluate with NumPy array (Zero-Copy)."""
        # x + y
        expr = parse("x") + parse("y")
        compiled = VmEvaluator(expr, ["x", "y"])
        
        # NumPy array input
        point = np.array([1.0, 2.0])
        result = compiled.evaluate(point)
        
        self.assertAlmostEqual(result, 3.0)

if __name__ == "__main__":
    unittest.main()
