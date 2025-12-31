import unittest
from symb_anafis import Expr, Symbol, diff, simplify
import math

class TestEdgeCases(unittest.TestCase):
    # ============================================================================
    # INPUT VALIDATION
    # ============================================================================

    def test_empty_input(self):
        with self.assertRaises(Exception):
            diff("", "x")

    def test_only_whitespace(self):
        with self.assertRaises(Exception):
            diff("   ", "x")

    def test_single_symbol(self):
        self.assertEqual(str(diff("x", "x")), "1")

    def test_single_different_symbol(self):
        self.assertEqual(str(diff("y", "x")), "0")

    def test_only_number(self):
        self.assertEqual(str(diff("42", "x")), "0")

    # ============================================================================
    # NUMBER PARSING
    # ============================================================================

    def test_leading_dot(self):
        self.assertEqual(str(diff(".5", "x")), "0")

    def test_trailing_dot(self):
        self.assertEqual(str(diff("5.", "x")), "0")

    def test_dot_decimal(self):
        res = str(diff("3.14*x", "x"))
        self.assertTrue("3.14" in res)

    def test_scientific_notation(self):
        self.assertEqual(str(diff("1e10", "x")), "0")
        self.assertEqual(str(diff("2.5e-3", "x")), "0")
        
    def test_multiple_decimals_error(self):
        with self.assertRaises(Exception):
            diff("3.14.15", "x")

    # ============================================================================
    # PARENTHESES EDGE CASES
    # ============================================================================

    def test_unmatched_open_paren(self):
        # (x + 1 should auto-close
        self.assertEqual(str(diff("(x+1", "x")), "1")

    def test_unmatched_close_paren(self):
        # x + 1) should auto-open
        self.assertEqual(str(diff("x+1)", "x")), "1")

    def test_empty_parens(self):
        with self.assertRaises(Exception):
            diff("()", "x")

    # ============================================================================
    # DIFFERENTIATION EDGE CASES
    # ============================================================================

    def test_var_not_in_formula(self):
        self.assertEqual(str(diff("y", "x")), "0")

    def test_var_in_both_fixed_and_diff(self):
        from symb_anafis import Diff
        x = Symbol("x")
        with self.assertRaises(ValueError):
            # Using builder to set fixed_vars
            Diff().fixed_vars([x]).differentiate(x, x)

    # ============================================================================
    # IMPLICIT MULTIPLICATION
    # ============================================================================

    def test_implicit_multiplication(self):
        self.assertEqual(str(diff("2x", "x")), "2")
        # With fixed var
        from symb_anafis import Diff
        a = Symbol("a")
        res = Diff().fixed_vars([a]).diff_str("ax", "x")
        self.assertEqual(res, "a")

    # ============================================================================
    # STRESS TESTS (Wave 1 Subset)
    # ============================================================================

    def test_very_long_sum(self):
        formula = " + ".join([f"x^{i}" for i in range(20)])
        res = diff(formula, "x")
        self.assertIsNotNone(res)

    def test_deeply_nested_sin(self):
        formula = "x"
        for _ in range(10):
            formula = f"sin({formula})"
        res = diff(formula, "x")
        self.assertIsNotNone(res)

if __name__ == "__main__":
    unittest.main()
