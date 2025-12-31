import sys
import os
import unittest
import math
from typing import List

# Ensure we can import the local module
sys.path.append(os.getcwd())

import symb_anafis as symb
from symb_anafis import Diff, diff, evaluate_str, simplify

class TestConsistency(unittest.TestCase):
    """
    Port of src/tests/eval_consistency_tests.rs
    
    Verifies that symbolic derivatives match numerical derivatives (finite differences).
    """

    def numerical_derivative(self, expr_str, var, point, h=1e-7):
        """Compute numerical derivative using central difference."""
        # expr(point + h)
        val_plus = float(evaluate_str(expr_str, list({var: point + h}.items())))
        # expr(point - h)
        val_minus = float(evaluate_str(expr_str, list({var: point - h}.items())))
        
        return (val_plus - val_minus) / (2.0 * h)

    def symbolic_derivative_at(self, expr_str, var, point):
        """Compute symbolic derivative and evaluate it."""
        deriv_str = diff(expr_str, var)
        # Simplify before eval to mimic Rust test behavior, though evaluate handles it
        simp_str = simplify(deriv_str) 
        val = evaluate_str(simp_str, list({var: point}.items()))
        return float(val)

    def verify_consistency(self, expr_str, var, test_points: List[float], tolerance=1e-5):
        """Assert symbolic and numerical derivatives are close."""
        for point in test_points:
            num = self.numerical_derivative(expr_str, var, point)
            sym = self.symbolic_derivative_at(expr_str, var, point)
            
            diff_val = abs(num - sym)
            # Relative tolerance based on magnitude
            max_val = max(abs(num), abs(sym), 1.0)
            rel_tol = tolerance * max_val
            
            self.assertLess(diff_val, rel_tol,
                f"Consistency failure for d/d{var}[{expr_str}] at {point}: num={num}, sym={sym}, diff={diff_val}")

    def test_polynomials(self):
        self.verify_consistency("x^2", "x", [0.5, 1.0, 2.0, 5.0])
        self.verify_consistency("x^3 - 2*x + 1", "x", [0.5, 1.0, 2.0])
        self.verify_consistency("x^5 + x^3 - x", "x", [0.5, 1.0, 1.5])

    def test_trig(self):
        points = [0.5, 1.0, 1.5, 2.0]
        self.verify_consistency("sin(x)", "x", points)
        self.verify_consistency("cos(x)", "x", points)
        # Avoid singularities for tan
        self.verify_consistency("tan(x)", "x", [0.5, 1.0]) 

    def test_exp_log(self):
        self.verify_consistency("exp(x)", "x", [0.0, 0.5, 1.0, 2.0])
        self.verify_consistency("ln(x)", "x", [0.5, 1.0, 2.0, 5.0])

    def test_sqrt(self):
        self.verify_consistency("sqrt(x)", "x", [0.5, 1.0, 2.0, 4.0])

    def test_chain_rule(self):
        self.verify_consistency("sin(x^2)", "x", [0.5, 1.0, 1.5])
        self.verify_consistency("exp(sin(x))", "x", [0.5, 1.0, 1.5])
        self.verify_consistency("ln(cos(x))", "x", [0.1, 0.2, 0.5]) # cos(x)>0 

    def test_product_rule(self):
        self.verify_consistency("x*sin(x)", "x", [0.5, 1.0, 2.0])
        self.verify_consistency("x^2*exp(x)", "x", [0.5, 1.0, 1.5])

    def test_quotient_rule(self):
        self.verify_consistency("sin(x)/x", "x", [0.5, 1.0, 2.0])
        self.verify_consistency("x/exp(x)", "x", [0.5, 1.0, 2.0])

    def test_complex_expressions(self):
        # Physics-like
        self.verify_consistency("x^2*exp(-x^2)", "x", [0.5, 1.0, 1.5])
        # Nested
        self.verify_consistency("sin(exp(x))^2", "x", [0.1, 0.5, 1.0])
        # Rational trig
        self.verify_consistency("sin(x)/(1 + cos(x))", "x", [0.5, 1.0, 1.5])

if __name__ == '__main__':
    unittest.main()
