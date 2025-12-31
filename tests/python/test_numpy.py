"""
Tests for NumPy zero-copy integration.
"""
import unittest
try:
    import numpy as np
except ImportError:
    np = None

from symb_anafis import Expr, eval_f64, CompiledEvaluator, parse

class TestNumpyIntegration(unittest.TestCase):
    def setUp(self):
        if np is None:
            self.skipTest("NumPy not installed")

    def test_eval_batch_numpy(self):
        """Test CompiledEvaluator.eval_batch with NumPy array."""
        expr = parse("x^2 + 1")
        compiled = CompiledEvaluator(expr, ["x"])
        
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
        # Note: eval_f64 returns list of lists of floats (not numpy arrays yet)
        results = eval_f64(exprs, var_names, data)
        
        self.assertEqual(len(results), 2)
        self.assertEqual(len(results[0]), n)
        self.assertEqual(len(results[1]), n)
        
        # Verify values
        expected_sum = x + y
        expected_prod = x * y
        
        np.testing.assert_array_almost_equal(results[0], expected_sum)
        np.testing.assert_array_almost_equal(results[1], expected_prod)

    def test_eval_batch_multivar(self):
        """Test CompiledEvaluator with multiple columns."""
        expr = parse("x * y")
        compiled = CompiledEvaluator(expr, ["x", "y"])
        
        x = np.array([1.0, 2.0], dtype=np.float64)
        y = np.array([3.0, 4.0], dtype=np.float64)
        
        result = compiled.eval_batch([x, y])
        
        expected = x * y
        np.testing.assert_array_almost_equal(result, expected)

    def test_eval_batch_mixed_strides(self):
        """Test with sliced/strided arrays to verify generic array support."""
        expr = parse("x")
        compiled = CompiledEvaluator(expr, ["x"])
        
        # Create strided array (slice)
        full = np.array([0.0, 1.0, 2.0, 3.0, 4.0], dtype=np.float64)
        x = full[::2] # [0, 2, 4]
        
        # Note: PyReadonlyArray1 should handle strides via as_slice() IF contiguous?
        # WARNING: as_slice() returns None or fails if not contiguous!
        # Rust numpy documentation says as_slice() needs contiguous.
        # If passed non-contiguous, unwrapping might panic!
        # Let's see if our implementation handles it or panics.
        # Ideally, we should check if we support it.
        # The user said "as_slice().unwrap()". This implies we assume contiguous or user ensures it.
        # Let's test non-contiguous. If it fails, that's expected per current aggressive implementation.
        # To make it robust, we should probably force contiguous in python or check in rust.
        
        # For now, let's verify if 'as_slice().unwrap()' works with strided.
        # If it panics, we verify strictness.
        
        # Verify that non-contiguous arrays raise ValueError
        # (Our current zero-copy implementation enforces C-contiguous requirement)
        with self.assertRaisesRegex(ValueError, "C-contiguous"):
             compiled.eval_batch([x])

    def test_eval_batch_list_fallback(self):
        """Test CompiledEvaluator.eval_batch with Python Lists (fallback path)."""
        expr = parse("x^2")
        compiled = CompiledEvaluator(expr, ["x"])
        
        # Pass Python list
        data = [1.0, 2.0, 3.0, 4.0]
        results = compiled.eval_batch([data])
        
        # Should work and return numpy array
        self.assertTrue(isinstance(results, np.ndarray))
        np.testing.assert_array_almost_equal(results, np.array(data)**2)

    def test_evaluate_single_point_numpy(self):
        """Test CompiledEvaluator.evaluate with NumPy array (Zero-Copy)."""
        # x + y
        expr = parse("x") + parse("y")
        compiled = CompiledEvaluator(expr, ["x", "y"])
        
        # NumPy array input
        point = np.array([1.0, 2.0])
        result = compiled.evaluate(point)
        
        self.assertAlmostEqual(result, 3.0)

if __name__ == "__main__":
    unittest.main()
