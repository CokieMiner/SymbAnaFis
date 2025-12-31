"""
Wave 3: Numerical & Special Functions Tests
Ported from:
- eval_func_tests.rs
- numerical_accuracy_tests.rs
"""
import unittest
import math
from symb_anafis import (
    diff, simplify, evaluate_str, parse, Expr, Symbol, symb,
    CompiledEvaluator
)

EPSILON = 1e-6

def approx_eq(a, b, eps=EPSILON):
    """Check if two floats are approximately equal."""
    if math.isnan(a) and math.isnan(b):
        return True
    if math.isinf(a) and math.isinf(b):
        return (a > 0) == (b > 0)
    return abs(a - b) < eps or abs(a - b) < eps * max(abs(a), abs(b), 1.0)


class TestTrigEvaluation(unittest.TestCase):
    """Trigonometric function evaluation tests."""

    def test_sin_basic(self):
        result = evaluate_str("sin(0)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)

    def test_sin_pi_half(self):
        result = evaluate_str(f"sin({math.pi/2})", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_cos_basic(self):
        result = evaluate_str("cos(0)", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_cos_pi(self):
        result = evaluate_str(f"cos({math.pi})", [])
        self.assertAlmostEqual(float(result), -1.0, places=10)

    def test_tan_basic(self):
        result = evaluate_str("tan(0)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)

    def test_tan_pi_quarter(self):
        result = evaluate_str(f"tan({math.pi/4})", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)


class TestInverseTrigEvaluation(unittest.TestCase):
    """Inverse trigonometric function evaluation tests."""

    def test_asin_zero(self):
        result = evaluate_str("asin(0)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)

    def test_asin_one(self):
        result = evaluate_str("asin(1)", [])
        self.assertAlmostEqual(float(result), math.pi/2, places=10)

    def test_acos_one(self):
        result = evaluate_str("acos(1)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)

    def test_atan_zero(self):
        result = evaluate_str("atan(0)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)

    def test_atan_one(self):
        result = evaluate_str("atan(1)", [])
        self.assertAlmostEqual(float(result), math.pi/4, places=10)


class TestHyperbolicEvaluation(unittest.TestCase):
    """Hyperbolic function evaluation tests."""

    def test_sinh_zero(self):
        result = evaluate_str("sinh(0)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)

    def test_cosh_zero(self):
        result = evaluate_str("cosh(0)", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_tanh_zero(self):
        result = evaluate_str("tanh(0)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)


class TestExpLogEvaluation(unittest.TestCase):
    """Exponential and logarithmic function evaluation tests."""

    def test_exp_zero(self):
        result = evaluate_str("exp(0)", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_exp_one(self):
        result = evaluate_str("exp(1)", [])
        self.assertAlmostEqual(float(result), math.e, places=10)

    def test_ln_one(self):
        result = evaluate_str("ln(1)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)

    def test_log10_ten(self):
        result = evaluate_str("log10(10)", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_log10_hundred(self):
        result = evaluate_str("log10(100)", [])
        self.assertAlmostEqual(float(result), 2.0, places=10)

    def test_log2_two(self):
        result = evaluate_str("log2(2)", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_log2_eight(self):
        result = evaluate_str("log2(8)", [])
        self.assertAlmostEqual(float(result), 3.0, places=10)


class TestTwoArgLog(unittest.TestCase):
    """Two-argument log(base, value) tests."""

    def test_log_base2_8(self):
        result = evaluate_str("log(2, 8)", [])
        self.assertAlmostEqual(float(result), 3.0, places=10)

    def test_log_base10_1000(self):
        result = evaluate_str("log(10, 1000)", [])
        self.assertAlmostEqual(float(result), 3.0, places=10)

    def test_log_base_x_one(self):
        """log(b, 1) = 0 for any base"""
        result = evaluate_str("log(2, 1)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)


class TestRoots(unittest.TestCase):
    """Root function evaluation tests."""

    def test_sqrt_four(self):
        result = evaluate_str("sqrt(4)", [])
        self.assertAlmostEqual(float(result), 2.0, places=10)

    def test_sqrt_nine(self):
        result = evaluate_str("sqrt(9)", [])
        self.assertAlmostEqual(float(result), 3.0, places=10)

    def test_cbrt_eight(self):
        result = evaluate_str("cbrt(8)", [])
        self.assertAlmostEqual(float(result), 2.0, places=10)

    def test_cbrt_negative(self):
        result = evaluate_str("cbrt(-8)", [])
        self.assertAlmostEqual(float(result), -2.0, places=10)


class TestSpecialFunctions(unittest.TestCase):
    """Special function evaluation tests (gamma, erf, bessel, etc.)."""

    def test_gamma_one(self):
        """Γ(1) = 1"""
        result = evaluate_str("gamma(1)", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_gamma_two(self):
        """Γ(2) = 1"""
        result = evaluate_str("gamma(2)", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_gamma_three(self):
        """Γ(3) = 2! = 2"""
        result = evaluate_str("gamma(3)", [])
        self.assertAlmostEqual(float(result), 2.0, places=10)

    def test_gamma_four(self):
        """Γ(4) = 3! = 6"""
        result = evaluate_str("gamma(4)", [])
        self.assertAlmostEqual(float(result), 6.0, places=10)

    def test_gamma_half(self):
        """Γ(0.5) = √π"""
        result = evaluate_str("gamma(0.5)", [])
        self.assertAlmostEqual(float(result), math.sqrt(math.pi), places=3)

    def test_erf_zero(self):
        """erf(0) = 0"""
        result = evaluate_str("erf(0)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)

    def test_erfc_zero(self):
        """erfc(0) = 1"""
        result = evaluate_str("erfc(0)", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_sinc_zero(self):
        """sinc(0) = 1 (limit)"""
        result = evaluate_str("sinc(0)", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_lambertw_zero(self):
        """W(0) = 0"""
        result = evaluate_str("lambertw(0)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)


class TestBesselFunctions(unittest.TestCase):
    """Bessel function evaluation tests."""

    def test_besselj_0_0(self):
        """J_0(0) = 1"""
        result = evaluate_str("besselj(0, 0)", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)

    def test_besselj_1_0(self):
        """J_1(0) = 0"""
        result = evaluate_str("besselj(1, 0)", [])
        self.assertAlmostEqual(float(result), 0.0, places=10)

    def test_besseli_0_0(self):
        """I_0(0) = 1"""
        result = evaluate_str("besseli(0, 0)", [])
        self.assertAlmostEqual(float(result), 1.0, places=10)


class TestDerivativeAccuracy(unittest.TestCase):
    """Derivative computation accuracy tests."""

    def test_polynomial_derivative(self):
        """d/dx[x^3] = 3x^2"""
        result = diff("x^3", "x")
        self.assertEqual(str(result), "3*x^2")

    def test_sin_derivative(self):
        """d/dx[sin(x)] = cos(x)"""
        result = diff("sin(x)", "x")
        self.assertEqual(str(result), "cos(x)")

    def test_cos_derivative(self):
        """d/dx[cos(x)] = -sin(x)"""
        result = diff("cos(x)", "x")
        self.assertEqual(str(result), "-sin(x)")

    def test_exp_derivative(self):
        """d/dx[exp(x)] = exp(x)"""
        result = diff("exp(x)", "x")
        self.assertEqual(str(result), "exp(x)")

    def test_ln_derivative(self):
        """d/dx[ln(x)] = 1/x"""
        result = diff("ln(x)", "x")
        s = str(result)
        self.assertTrue("1/x" in s or "x^-1" in s or "x^(-1)" in s)

    def test_sinh_derivative(self):
        """d/dx[sinh(x)] = cosh(x)"""
        result = diff("sinh(x)", "x")
        self.assertEqual(str(result), "cosh(x)")

    def test_cosh_derivative(self):
        """d/dx[cosh(x)] = sinh(x)"""
        result = diff("cosh(x)", "x")
        self.assertEqual(str(result), "sinh(x)")

    def test_constant_derivative(self):
        """d/dx[5] = 0"""
        result = diff("5", "x")
        self.assertEqual(str(result), "0")

    def test_linear_derivative(self):
        """d/dx[3*x + 5] = 3"""
        result = diff("3*x + 5", "x")
        self.assertEqual(str(result), "3")


class TestCompiledEvaluator(unittest.TestCase):
    """CompiledEvaluator parity tests."""

    def test_compiled_sin(self):
        expr = parse("sin(x)")
        compiled = CompiledEvaluator(expr, ["x"])
        result = compiled.evaluate([math.pi/2])
        self.assertAlmostEqual(result, 1.0, places=10)

    def test_compiled_polynomial(self):
        expr = parse("x^2 + 2*x + 1")
        compiled = CompiledEvaluator(expr, ["x"])
        result = compiled.evaluate([3.0])
        self.assertAlmostEqual(result, 16.0, places=10)

    def test_compiled_multivar(self):
        expr = parse("x*y + z")
        compiled = CompiledEvaluator(expr, ["x", "y", "z"])
        result = compiled.evaluate([2.0, 3.0, 4.0])
        self.assertAlmostEqual(result, 10.0, places=10)

    def test_compiled_log(self):
        expr = parse("log(2, x)")
        compiled = CompiledEvaluator(expr, ["x"])
        result = compiled.evaluate([8.0])
        self.assertAlmostEqual(result, 3.0, places=10)

    def test_compiled_batch(self):
        try:
            import numpy as np
        except ImportError:
            self.skipTest("NumPy not installed")

        expr = parse("x^2")
        compiled = CompiledEvaluator(expr, ["x"])
        results = compiled.eval_batch([np.array([1.0, 2.0, 3.0, 4.0], dtype=np.float64)])
        self.assertEqual(len(results), 4)
        self.assertAlmostEqual(results[0], 1.0, places=10)
        self.assertAlmostEqual(results[1], 4.0, places=10)
        self.assertAlmostEqual(results[2], 9.0, places=10)
        self.assertAlmostEqual(results[3], 16.0, places=10)


if __name__ == '__main__':
    unittest.main()
