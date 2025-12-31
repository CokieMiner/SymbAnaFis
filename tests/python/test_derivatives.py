import sys
import os
import unittest
import math
from typing import List, Tuple

# Ensure we can import the local module
sys.path.append(os.getcwd())

import symb_anafis as symb
from symb_anafis import Diff, diff, evaluate_str, simplify

class TestDerivatives(unittest.TestCase):
    """
    Port of src/tests/derivative_oracle_tests.rs
    """

    def verify_derivative(self, expr, var, expected_symbolic, test_points: List[Tuple[float, float]]):
        """
        Helper to verify derivative both symbolically and numerically.
        """
        # 1. Compute derivative
        result = diff(expr, var)

        # 2. Check numerical accuracy
        for (x_val, expected_val) in test_points:
            # Eval result at x_val
            # result might be symbolic string
            # evaluate_str requires [(name, val)]
            val_res = evaluate_str(result, list({var: x_val}.items()))
            final_val = float(val_res)
            
            tolerance = 1e-6 * max(abs(expected_val), 1.0)
            self.assertAlmostEqual(final_val, expected_val, delta=tolerance, 
                                   msg=f"d/d{var}[{expr}] at {x_val}")

        # 3. Check symbolic form (optional/approximate checking)
        if expected_symbolic:
            # Instead of exact string match, let's normalize both via simplify
            # This is hard because simplification might differ.
            # We can try to evaluate (result - expected) at points and check for 0.
            diff_expr = f"({result}) - ({expected_symbolic})"
            for (x_val, _) in test_points:
                val_diff = evaluate_str(diff_expr, list({var: x_val}.items()))
                f_diff = float(val_diff)
                self.assertAlmostEqual(f_diff, 0.0, delta=1e-6, 
                                       msg=f"Symbolic mismatch for {expr}: got {result}, expected {expected_symbolic}")

    def test_polynomials(self):
        # d/dx[5] = 0
        self.verify_derivative("5", "x", "0", [(1.0, 0.0), (2.0, 0.0)])
        # d/dx[x] = 1
        self.verify_derivative("x", "x", "1", [(1.0, 1.0)])
        # d/dx[x^2] = 2x
        self.verify_derivative("x^2", "x", "2*x", [(1.0, 2.0), (2.0, 4.0)])
        # d/dx[x^3] = 3x^2
        self.verify_derivative("x^3", "x", "3*x^2", [(1.0, 3.0), (2.0, 12.0)])

    def test_trig(self):
        # sin(x) -> cos(x)
        self.verify_derivative("sin(x)", "x", "cos(x)", [
            (0.0, 1.0),
            (math.pi/2, 0.0),
            (math.pi, -1.0)
        ])
        # cos(x) -> -sin(x)
        self.verify_derivative("cos(x)", "x", "-sin(x)", [
            (0.0, 0.0),
            (math.pi/2, -1.0)
        ])

    def test_exponential(self):
        # exp(x) -> exp(x)
        self.verify_derivative("exp(x)", "x", "exp(x)", [
            (0.0, 1.0),
            (1.0, math.e)
        ])
        # ln(x) -> 1/x
        self.verify_derivative("ln(x)", "x", "1/x", [
            (1.0, 1.0),
            (2.0, 0.5)
        ])

    def test_chain_rule(self):
        # sin(x^2) -> 2x cos(x^2)
        self.verify_derivative("sin(x^2)", "x", "2*x*cos(x^2)", [
            (0.0, 0.0),
            (1.0, 2.0 * math.cos(1.0))
        ])
        # exp(sin(x)) -> exp(sin(x)) * cos(x)
        self.verify_derivative("exp(sin(x))", "x", "exp(sin(x))*cos(x)", [
            (0.0, 1.0)
        ])

    def test_product_rule(self):
        # x sin(x) -> sin(x) + x cos(x)
        self.verify_derivative("x*sin(x)", "x", "sin(x) + x*cos(x)", [
            (0.0, 0.0),
            (math.pi/2, 1.0)
        ])

    def test_quotient_rule(self):
        # sin(x)/x -> (x cos x - sin x)/x^2
        # At pi/2: (pi/2 * 0 - 1) / (pi^2/4) = -4/pi^2 approx -0.405
        expected = -4.0 / (math.pi**2)
        self.verify_derivative("sin(x)/x", "x", "(x*cos(x) - sin(x))/x^2", [
            (math.pi/2, expected)
        ])

if __name__ == '__main__':
    unittest.main()
