import sys
import os
import unittest

# Ensure we can import the local module
sys.path.append(os.getcwd())

import symb_anafis as symb
from symb_anafis import Expr, Symbol, simplify

class TestRegressions(unittest.TestCase):
    """
    Port of regression tests from Rust.
    """

    def test_rc_derivative_actual_bug(self):
        """
        Port of src/tests/actual_division_bug.rs
        
        Original expression: -C * R * V0 * exp(-t / (C * R)) / (C * R^2)
        Expected simplification: -V0 * exp(-t / (C * R)) / R
        """
        # Symbols
        t = Symbol("t")
        C = Symbol("C")
        R = Symbol("R")
        V0 = Symbol("V0")
        
        # Construct expression using operator overloading
        # exp(-t / (C * R))
        arg = (-t) / (C * R) 
        exp_term = arg.exp() # Call .exp() directly on PyExpr
        
        numerator = (-1.0 * C * R * V0) * exp_term
        denominator = C * (R ** 2.0)
        
        expr = numerator / denominator
        
        # Simplify using Expr method
        simplified = expr.simplify()
        s_res = str(simplified)
        
        print(f"Regression Test Simplified: {s_res}")
        
        # Check for expected simplification
        # Should end with /R or * R^-1
        # And C should be cancelled out (so C shouldn't appear except in the exp arg)
        
        # Check C is cancelled outside exp
        # We can try to split by exp and check pre-factor
        self.assertTrue("/R" in s_res or "*R^-1" in s_res or "/ R" in s_res)
        
        # V0 should be present
        self.assertTrue("V0" in s_res)

    def test_simple_constant_division(self):
        """
        Simpler case: (-1 * C * R) / (C * R^2) -> -1/R
        """
        C = Symbol("C")
        R = Symbol("R")
        
        num = -1.0 * C * R
        den = C * (R ** 2.0)
        expr = num / den
        
        simplified = expr.simplify()
        s_res = str(simplified)
        
        # -1/R or -1.0/R
        self.assertTrue("-1/R" in s_res or "-1.0/R" in s_res or "-R^-1" in s_res)

if __name__ == '__main__':
    unittest.main()
