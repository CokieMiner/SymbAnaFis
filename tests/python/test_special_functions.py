"""
Special Functions Tests - Ported from Rust eval_func_tests.rs

Tests Bessel, Gamma, Beta, Lambert W, Error functions, Elliptic integrals,
Hermite polynomials, and other special mathematical functions.
"""

import pytest
import math
from symb_anafis import evaluate_str, CompiledEvaluator, parse, Expr

EPSILON = 1e-6
LOOSE_EPSILON = 1e-3


def approx_eq(a: float, b: float, eps: float = EPSILON) -> bool:
    """Check if two floats are approximately equal."""
    if math.isnan(a) and math.isnan(b):
        return True
    if math.isinf(a) and math.isinf(b):
        return (a > 0) == (b > 0)
    return abs(a - b) < eps or abs(a - b) < eps * max(abs(a), abs(b))


def eval_expr(expr_str: str, **vars) -> float:
    """Parse and evaluate expression, optionally with variables."""
    if vars:
        var_list = [(k, v) for k, v in vars.items()]
        result = evaluate_str(expr_str, var_list)
    else:
        result = evaluate_str(expr_str, [])
    return float(result)


class TestTrigFunctions:
    """Basic trigonometric function tests."""

    def test_sin(self):
        assert approx_eq(eval_expr("sin(0)"), 0.0)
        assert approx_eq(eval_expr("sin(1.5707963267948966)"), 1.0)  # π/2

    def test_cos(self):
        assert approx_eq(eval_expr("cos(0)"), 1.0)
        assert approx_eq(eval_expr("cos(3.141592653589793)"), -1.0)  # π

    def test_tan(self):
        assert approx_eq(eval_expr("tan(0)"), 0.0)
        assert approx_eq(eval_expr("tan(0.7853981633974483)"), 1.0)  # π/4

    def test_reciprocal_trig(self):
        # cot(π/4) = 1
        assert approx_eq(eval_expr("cot(0.7853981633974483)"), 1.0)
        # sec(0) = 1
        assert approx_eq(eval_expr("sec(0)"), 1.0)
        # csc(π/2) = 1
        assert approx_eq(eval_expr("csc(1.5707963267948966)"), 1.0)
        # cot(0) = Inf
        assert math.isinf(eval_expr("cot(0)"))
        # csc(0) = Inf
        assert math.isinf(eval_expr("csc(0)"))


class TestInverseTrig:
    """Inverse trigonometric function tests."""

    def test_asin_acos_atan(self):
        pi = math.pi
        assert approx_eq(eval_expr("asin(0)"), 0.0)
        assert approx_eq(eval_expr("asin(1)"), pi / 2)
        assert approx_eq(eval_expr("acos(1)"), 0.0)
        assert approx_eq(eval_expr("acos(0)"), pi / 2)
        assert approx_eq(eval_expr("atan(0)"), 0.0)
        assert approx_eq(eval_expr("atan(1)"), pi / 4)

    def test_acot(self):
        assert approx_eq(eval_expr("acot(1)"), math.pi / 4)

    def test_asec_acsc_domain(self):
        # asec(1) = 0
        assert approx_eq(eval_expr("asec(1)"), 0.0)
        # asec(0.5) = NaN (|x| < 1)
        assert math.isnan(eval_expr("asec(0.5)"))
        # acsc(0.5) = NaN
        assert math.isnan(eval_expr("acsc(0.5)"))


class TestHyperbolic:
    """Hyperbolic function tests."""

    def test_basic_hyperbolic(self):
        assert approx_eq(eval_expr("sinh(0)"), 0.0)
        assert approx_eq(eval_expr("cosh(0)"), 1.0)
        assert approx_eq(eval_expr("tanh(0)"), 0.0)
        assert approx_eq(eval_expr("sech(0)"), 1.0)

    def test_hyperbolic_known_values(self):
        assert approx_eq(eval_expr("sinh(1)"), 1.1752011936438014)
        assert approx_eq(eval_expr("cosh(1)"), 1.5430806348152437)

    def test_hyperbolic_singularities(self):
        # coth(0) = Inf
        assert math.isinf(eval_expr("coth(0)"))
        # csch(0) = Inf
        assert math.isinf(eval_expr("csch(0)"))


class TestInverseHyperbolic:
    """Inverse hyperbolic function tests."""

    def test_inverse_hyperbolic_basic(self):
        assert approx_eq(eval_expr("asinh(0)"), 0.0)
        assert approx_eq(eval_expr("acosh(1)"), 0.0)
        assert approx_eq(eval_expr("atanh(0)"), 0.0)

    def test_acoth(self):
        assert approx_eq(eval_expr("acoth(2)"), 0.5493061443340549)
        # acoth(0.5) = NaN (|x| <= 1)
        assert math.isnan(eval_expr("acoth(0.5)"))


class TestExpLog:
    """Exponential and logarithmic function tests."""

    def test_exp(self):
        assert approx_eq(eval_expr("exp(0)"), 1.0)
        assert approx_eq(eval_expr("exp(1)"), math.e)

    def test_ln(self):
        assert approx_eq(eval_expr("ln(1)"), 0.0)
        # ln(-1) = NaN
        assert math.isnan(eval_expr("ln(-1)"))

    def test_log10_log2(self):
        assert approx_eq(eval_expr("log10(10)"), 1.0)
        assert approx_eq(eval_expr("log10(100)"), 2.0)
        assert approx_eq(eval_expr("log2(2)"), 1.0)
        assert approx_eq(eval_expr("log2(8)"), 3.0)

    def test_log_two_arg(self):
        # log(base, x)
        assert approx_eq(eval_expr("log(2, 8)"), 3.0)
        assert approx_eq(eval_expr("log(10, 1000)"), 3.0)
        assert approx_eq(eval_expr("log(3, 27)"), 3.0)
        assert approx_eq(eval_expr("log(5, 125)"), 3.0)

    def test_log_domain_errors(self):
        # log(-2, 8) - invalid base returns NaN or stays symbolic
        try:
            result = eval_expr("log(-2, 8)")
            # If it evaluates, should be NaN
            assert math.isnan(result)
        except ValueError:
            # Expression returned as string (didn't evaluate) - also acceptable
            pass

        # log(2, -8) = NaN
        try:
            result = eval_expr("log(2, -8)")
            assert math.isnan(result)
        except ValueError:
            pass


class TestRoots:
    """Root function tests."""

    def test_sqrt(self):
        assert approx_eq(eval_expr("sqrt(4)"), 2.0)
        assert approx_eq(eval_expr("sqrt(9)"), 3.0)
        # sqrt(-1) = NaN
        assert math.isnan(eval_expr("sqrt(-1)"))

    def test_cbrt(self):
        assert approx_eq(eval_expr("cbrt(8)"), 2.0)
        assert approx_eq(eval_expr("cbrt(27)"), 3.0)
        assert approx_eq(eval_expr("cbrt(-8)"), -2.0)


class TestSpecialFunctions:
    """Special mathematical functions."""

    def test_sinc(self):
        # sinc(0) = 1 (limit)
        assert approx_eq(eval_expr("sinc(0)"), 1.0)
        # sinc(π) ≈ 0
        assert abs(eval_expr("sinc(3.141592653589793)")) < 1e-10

    def test_abs(self):
        assert approx_eq(eval_expr("abs(-5)"), 5.0)
        assert approx_eq(eval_expr("abs(3.5)"), 3.5)


class TestErrorFunctions:
    """Error function tests."""

    def test_erf(self):
        assert approx_eq(eval_expr("erf(0)"), 0.0)
        erf3 = eval_expr("erf(3)")
        assert 0.9999 < erf3 < 1.0

    def test_erfc(self):
        assert approx_eq(eval_expr("erfc(0)"), 1.0)
        # erf(x) + erfc(x) = 1
        erf1 = eval_expr("erf(1)")
        erfc1 = eval_expr("erfc(1)")
        assert abs(erf1 + erfc1 - 1.0) < 0.001


class TestGammaFunctions:
    """Gamma and related function tests."""

    def test_gamma_integers(self):
        # Γ(n) = (n-1)! for positive integers
        assert approx_eq(eval_expr("gamma(1)"), 1.0)
        assert approx_eq(eval_expr("gamma(2)"), 1.0)
        assert approx_eq(eval_expr("gamma(3)"), 2.0)
        assert approx_eq(eval_expr("gamma(4)"), 6.0)
        assert approx_eq(eval_expr("gamma(5)"), 24.0)

    def test_gamma_half(self):
        # Γ(0.5) = √π
        gamma_half = eval_expr("gamma(0.5)")
        assert abs(gamma_half - math.sqrt(math.pi)) < 0.001

    def test_digamma(self):
        # ψ(1) = -γ ≈ -0.5772
        psi1 = eval_expr("digamma(1)")
        assert abs(psi1 - (-0.5772156649015329)) < 0.01

    def test_trigamma(self):
        # ψ₁(1) = π²/6
        trigamma1 = eval_expr("trigamma(1)")
        expected = math.pi**2 / 6.0
        assert abs(trigamma1 - expected) < 0.01

    def test_beta(self):
        # B(1,1) = 1
        assert approx_eq(eval_expr("beta(1, 1)"), 1.0, LOOSE_EPSILON)
        # B(a,b) = B(b,a) (symmetry)
        beta23 = eval_expr("beta(2, 3)")
        beta32 = eval_expr("beta(3, 2)")
        assert approx_eq(beta23, beta32)

    def test_polygamma(self):
        # polygamma(0, x) = digamma(x)
        poly0_1 = eval_expr("polygamma(0, 1)")
        digamma1 = eval_expr("digamma(1)")
        assert approx_eq(poly0_1, digamma1)


class TestLambertW:
    """Lambert W function tests."""

    def test_lambertw_zero(self):
        assert approx_eq(eval_expr("lambertw(0)"), 0.0)

    def test_lambertw_e(self):
        # W(e) = 1
        w_e = eval_expr("lambertw(2.718281828459045)")
        assert approx_eq(w_e, 1.0, LOOSE_EPSILON)


class TestBesselFunctions:
    """Bessel function tests."""

    def test_bessel_j(self):
        # J_0(0) = 1
        j0_0 = eval_expr("besselj(0, 0)")
        assert approx_eq(j0_0, 1.0, LOOSE_EPSILON)
        # J_1(0) = 0
        j1_0 = eval_expr("besselj(1, 0)")
        assert approx_eq(j1_0, 0.0, LOOSE_EPSILON)
        # J_0(2.4048) ≈ 0 (first zero)
        assert abs(eval_expr("besselj(0, 2.4048)")) < 0.01

    def test_bessel_y(self):
        # Y_0(1) ≈ 0.0883
        y0_1 = eval_expr("bessely(0, 1)")
        assert abs(y0_1 - 0.0883) < 0.01

    def test_bessel_i(self):
        # I_0(0) = 1
        i0_0 = eval_expr("besseli(0, 0)")
        assert approx_eq(i0_0, 1.0)
        # I_1(0) = 0
        i1_0 = eval_expr("besseli(1, 0)")
        assert approx_eq(i1_0, 0.0)


class TestHermitePolynomials:
    """Hermite polynomial tests."""

    def test_hermite_h0(self):
        # H_0(x) = 1
        assert approx_eq(eval_expr("hermite(0, 2)"), 1.0)

    def test_hermite_h1(self):
        # H_1(x) = 2x
        assert approx_eq(eval_expr("hermite(1, 3)"), 6.0)

    def test_hermite_h2(self):
        # H_2(x) = 4x² - 2
        assert approx_eq(eval_expr("hermite(2, 2)"), 14.0)  # 4*4-2


class TestEllipticIntegrals:
    """Elliptic integral tests."""

    def test_elliptic_k_zero(self):
        # K(0) = π/2
        k0 = eval_expr("elliptic_k(0)")
        assert approx_eq(k0, math.pi / 2, LOOSE_EPSILON)

    def test_elliptic_e_zero(self):
        # E(0) = π/2
        e0 = eval_expr("elliptic_e(0)")
        assert approx_eq(e0, math.pi / 2, LOOSE_EPSILON)


class TestVariableSubstitution:
    """Variable substitution in evaluate tests."""

    def test_sin_with_variable(self):
        result = eval_expr("sin(x)", x=math.pi / 2)
        assert approx_eq(result, 1.0)

    def test_compound_expression(self):
        result = eval_expr("exp(x) + y", x=0.0, y=2.0)
        assert approx_eq(result, 3.0)  # e^0 + 2 = 1 + 2 = 3

    def test_log_with_variables(self):
        result = eval_expr("log(2, x)", x=8.0)
        assert approx_eq(result, 3.0)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
