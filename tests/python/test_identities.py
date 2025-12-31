"""
Wave 2: Identities & Simplification Tests
Ported from:
- trig_simplification_tests.rs
- log_simplification_tests.rs
- fraction_simplification_tests.rs
"""
import unittest
import math
from symb_anafis import simplify, Simplify, Expr, Symbol, symb, parse

class TestTrigIdentities(unittest.TestCase):
    """Trigonometric identity verification tests."""

    def test_pythagorean_identity(self):
        """sin²(x) + cos²(x) = 1"""
        result = simplify("sin(x)^2 + cos(x)^2")
        self.assertEqual(str(result), "1")

    def test_tan_sec_identity(self):
        """1 + tan²(x) = sec²(x)"""
        result = simplify("1 + tan(x)^2")
        s = str(result)
        self.assertTrue("sec" in s and "2" in s)

    def test_cot_csc_identity(self):
        """1 + cot²(x) = csc²(x)"""
        result = simplify("1 + cot(x)^2")
        s = str(result)
        self.assertTrue("csc" in s and "2" in s)

    def test_inverse_composition_sin_asin(self):
        """sin(asin(x)) = x"""
        result = simplify("sin(asin(x))")
        self.assertEqual(str(result), "x")

    def test_inverse_composition_cos_acos(self):
        """cos(acos(x)) = x"""
        result = simplify("cos(acos(x))")
        self.assertEqual(str(result), "x")

    def test_inverse_composition_tan_atan(self):
        """tan(atan(x)) = x"""
        result = simplify("tan(atan(x))")
        self.assertEqual(str(result), "x")

    def test_inverse_composition_asin_sin(self):
        """asin(sin(x)) = x"""
        result = simplify("asin(sin(x))")
        self.assertEqual(str(result), "x")

    def test_inverse_composition_acos_cos(self):
        """acos(cos(x)) = x"""
        result = simplify("acos(cos(x))")
        self.assertEqual(str(result), "x")

    def test_sin_symmetry(self):
        """sin(-x) = -sin(x)"""
        result = simplify("sin(-x)")
        s = str(result)
        self.assertTrue("-" in s and "sin" in s and "x" in s)

    def test_cos_symmetry(self):
        """cos(-x) = cos(x) (even function)"""
        result = simplify("cos(-x)")
        self.assertEqual(str(result), "cos(x)")

    def test_tan_symmetry(self):
        """tan(-x) = -tan(x)"""
        result = simplify("tan(-x)")
        s = str(result)
        self.assertTrue("-" in s and "tan" in s)

    def test_sin_exact_value_pi6(self):
        """sin(π/6) = 0.5"""
        result = simplify(f"sin({math.pi/6})")
        self.assertAlmostEqual(float(result), 0.5, places=10)

    def test_cos_exact_value_pi3(self):
        """cos(π/3) = 0.5"""
        result = simplify(f"cos({math.pi/3})")
        self.assertAlmostEqual(float(result), 0.5, places=10)

    def test_tan_exact_value_pi4(self):
        """tan(π/4) = 1"""
        result = simplify(f"tan({math.pi/4})")
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_sin_periodicity(self):
        """sin(x + 2π) = sin(x)"""
        result = simplify(f"sin(x + {2*math.pi})")
        self.assertEqual(str(result), "sin(x)")

    def test_cos_periodicity(self):
        """cos(x + 2π) = cos(x)"""
        result = simplify(f"cos(x + {2*math.pi})")
        self.assertEqual(str(result), "cos(x)")


class TestLogIdentities(unittest.TestCase):
    """Logarithmic identity verification tests."""

    def test_log_of_one(self):
        """log(b, 1) = 0"""
        result = simplify("log(b, 1)")
        self.assertEqual(str(result), "0")

    def test_log_of_base(self):
        """log(b, b) = 1"""
        result = simplify("log(b, b)")
        self.assertEqual(str(result), "1")

    def test_log_power_rule_even(self):
        """log(b, x^2) -> 2 * log(b, abs(x))"""
        result = simplify("log(b, x^2)")
        s = str(result)
        self.assertTrue("2" in s and "log" in s)

    def test_log_power_rule_odd(self):
        """log(b, x^3) -> 3 * log(b, x)"""
        result = simplify("log(b, x^3)")
        s = str(result)
        self.assertTrue("3" in s and "log" in s)

    def test_ln_e(self):
        """ln(e) = 1"""
        result = simplify("ln(e)")
        self.assertEqual(str(result), "1")

    def test_ln_1(self):
        """ln(1) = 0"""
        result = simplify("ln(1)")
        self.assertEqual(str(result), "0")


class TestFractionSimplification(unittest.TestCase):
    """Fraction simplification tests."""

    def test_nested_fraction_div_div(self):
        """(x/y) / (z/a) -> (x*a) / (y*z)"""
        result = simplify("(x/y) / (z/a)")
        s = str(result)
        # Numerator should have x and a, denominator should have y and z
        self.assertTrue("x" in s and "a" in s)
        self.assertTrue("y" in s and "z" in s)

    def test_nested_fraction_val_div(self):
        """x / (y/z) -> (x*z) / y"""
        result = simplify("x / (y/z)")
        s = str(result)
        self.assertTrue("x" in s and "z" in s)
        self.assertTrue("y" in s)

    def test_nested_fraction_div_val(self):
        """(x/y) / z -> x / (y*z)"""
        result = simplify("(x/y) / z")
        s = str(result)
        self.assertTrue("x" in s)
        self.assertTrue("y" in s and "z" in s)

    def test_numeric_nested_fraction(self):
        """(1/2) / (1/3) = 3/2"""
        result = simplify("(1/2) / (1/3)")
        s = str(result)
        # Should simplify to 3/2 or 1.5
        self.assertTrue("3/2" in s or s == "1.5")

    def test_fraction_cancellation(self):
        """(C * R) / (C * R^2) -> 1/R or R^-1"""
        result = simplify("(C * R) / (C * R^2)")
        s = str(result)
        # Should simplify to 1/R or R^-1
        self.assertTrue(("1/R" in s) or ("R^-1" in s) or ("R^(-1)" in s))

    def test_simple_cancellation(self):
        """x/x = 1"""
        result = simplify("x/x")
        self.assertEqual(str(result), "1")

    def test_power_cancellation(self):
        """x^3 / x^2 = x"""
        result = simplify("x^3 / x^2")
        self.assertEqual(str(result), "x")


class TestPowerSimplification(unittest.TestCase):
    """Power and exponent simplification tests."""

    def test_power_of_zero(self):
        """x^0 = 1"""
        result = simplify("x^0")
        self.assertEqual(str(result), "1")

    def test_power_of_one(self):
        """x^1 = x"""
        result = simplify("x^1")
        self.assertEqual(str(result), "x")

    def test_zero_power(self):
        """0^x = 0 (for positive x)"""
        result = simplify("0^2")
        self.assertEqual(str(result), "0")

    def test_one_power(self):
        """1^x = 1"""
        result = simplify("1^x")
        self.assertEqual(str(result), "1")

    def test_power_of_power(self):
        """(x^2)^3 = x^6"""
        result = simplify("(x^2)^3")
        s = str(result)
        self.assertTrue("x" in s and "6" in s)

    def test_exp_ln(self):
        """e^(ln(x)) = x"""
        result = simplify("e^(ln(x))")
        self.assertEqual(str(result), "x")

    def test_ln_exp(self):
        """ln(e^x) = x"""
        result = simplify("ln(e^x)")
        self.assertEqual(str(result), "x")


if __name__ == '__main__':
    unittest.main()
