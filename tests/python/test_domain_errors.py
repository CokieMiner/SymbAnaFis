"""
Tests for domain error handling in special functions.

These tests verify that domain errors are properly raised as ValueError
when numeric constants are passed to functions at their poles or outside
their domains.

Note: Domain checks only apply when arguments are numeric constants.
Symbolic expressions (like Expr("x")) always pass through to allow
symbolic manipulation.
"""
import pytest
from symb_anafis import Expr, parse


class TestGammaPoles:
    """Test gamma function family raises at non-positive integer poles."""

    def test_gamma_at_zero(self):
        """gamma(0) is a pole"""
        with pytest.raises(ValueError, match="pole at non-positive integer"):
            Expr(0).gamma()

    def test_gamma_at_neg_one(self):
        """gamma(-1) is a pole"""
        with pytest.raises(ValueError, match="pole at non-positive integer"):
            Expr(-1).gamma()

    def test_gamma_at_neg_two(self):
        """gamma(-2) is a pole"""
        with pytest.raises(ValueError, match="pole at non-positive integer"):
            Expr(-2).gamma()

    def test_gamma_symbolic_passes(self):
        """gamma(x) should NOT raise - x is symbolic"""
        result = Expr("x").gamma()
        assert result is not None

    def test_gamma_positive_passes(self):
        """gamma(2) should work fine"""
        result = Expr(2).gamma()
        assert result is not None

    def test_digamma_at_zero(self):
        """digamma(0) is a pole"""
        with pytest.raises(ValueError, match="pole at non-positive integer"):
            Expr(0).digamma()

    def test_trigamma_at_neg_one(self):
        """trigamma(-1) is a pole"""
        with pytest.raises(ValueError, match="pole at non-positive integer"):
            Expr(-1).trigamma()

    def test_tetragamma_at_zero(self):
        """tetragamma(0) is a pole"""
        with pytest.raises(ValueError, match="pole at non-positive integer"):
            Expr(0).tetragamma()


class TestZetaPole:
    """Test zeta function raises at s=1 pole."""

    def test_zeta_at_one(self):
        """zeta(1) is a pole"""
        with pytest.raises(ValueError, match="pole at s=1"):
            Expr(1).zeta()

    def test_zeta_at_two_passes(self):
        """zeta(2) should work (pi^2/6)"""
        result = Expr(2).zeta()
        assert result is not None

    def test_zeta_symbolic_passes(self):
        """zeta(s) should NOT raise - s is symbolic"""
        result = Expr("s").zeta()
        assert result is not None


class TestLambertWDomain:
    """Test Lambert W function raises for x < -1/e."""

    def test_lambertw_below_domain(self):
        """lambertw(-1) is outside domain (x < -1/e ≈ -0.368)"""
        with pytest.raises(ValueError, match="must be >= -1/e"):
            Expr(-1).lambertw()

    def test_lambertw_at_zero_passes(self):
        """lambertw(0) = 0"""
        result = Expr(0).lambertw()
        assert result is not None

    def test_lambertw_positive_passes(self):
        """lambertw(1) should work"""
        result = Expr(1).lambertw()
        assert result is not None


class TestBesselDomain:
    """Test Bessel Y and K functions raise for x <= 0."""

    def test_bessely_at_zero(self):
        """bessely(0, 0) is undefined"""
        with pytest.raises(ValueError, match="x must be > 0"):
            Expr(0).bessely(0)

    def test_bessely_negative(self):
        """bessely(0, -1) is undefined"""
        with pytest.raises(ValueError, match="x must be > 0"):
            Expr(-1).bessely(0)

    def test_besselk_at_zero(self):
        """besselk(0, 0) is undefined"""
        with pytest.raises(ValueError, match="x must be > 0"):
            Expr(0).besselk(0)

    def test_bessely_positive_passes(self):
        """bessely(0, 1) should work"""
        result = Expr(1).bessely(0)
        assert result is not None


class TestEllipticDomain:
    """Test elliptic integrals raise for |k| >= 1."""

    def test_elliptic_k_at_one(self):
        """elliptic_k(1) is a singular point"""
        with pytest.raises(ValueError, match="must be < 1"):
            Expr(1).elliptic_k()

    def test_elliptic_k_above_one(self):
        """elliptic_k(2) is outside domain"""
        with pytest.raises(ValueError, match="must be < 1"):
            Expr(2).elliptic_k()

    def test_elliptic_e_above_one(self):
        """elliptic_e(2) is outside domain"""
        with pytest.raises(ValueError, match="must be <= 1"):
            Expr(2).elliptic_e()

    def test_elliptic_k_inside_passes(self):
        """elliptic_k(0.5) should work"""
        result = Expr(0.5).elliptic_k()
        assert result is not None


class TestPolygammaDomain:
    """Test polygamma function domain restrictions."""

    def test_polygamma_negative_order(self):
        """polygamma(-1, x) is undefined"""
        with pytest.raises(ValueError, match="non-negative integer"):
            Expr(2).polygamma(-1)

    def test_polygamma_at_pole(self):
        """polygamma(1, 0) is a pole"""
        with pytest.raises(ValueError, match="pole at non-positive integer"):
            Expr(0).polygamma(1)


class TestBetaDomain:
    """Test beta function domain restrictions."""

    def test_beta_first_arg_pole(self):
        """beta(0, 1) is undefined"""
        with pytest.raises(ValueError, match="pole at non-positive integer"):
            Expr(0).beta(1)

    def test_beta_second_arg_pole(self):
        """beta(1, -1) is undefined"""
        with pytest.raises(ValueError, match="pole at non-positive integer"):
            Expr(1).beta(-1)

    def test_beta_positive_passes(self):
        """beta(2, 3) should work"""
        result = Expr(2).beta(3)
        assert result is not None


class TestLogDomain:
    """Test logarithm domain restrictions."""

    def test_log_base_zero(self):
        """log(0, x) is undefined"""
        with pytest.raises(ValueError, match="must be positive and not 1"):
            Expr(2).log(0)

    def test_log_base_one(self):
        """log(1, x) is undefined (would give 0/0)"""
        with pytest.raises(ValueError, match="must be positive and not 1"):
            Expr(2).log(1)

    def test_log_value_zero(self):
        """log(base, 0) is undefined"""
        with pytest.raises(ValueError, match="must be positive"):
            Expr(0).log(2)

    def test_log_valid_passes(self):
        """log(2, 8) = 3"""
        result = Expr(8).log(2)
        assert result is not None


class TestHermiteDomain:
    """Test Hermite polynomial domain restrictions."""

    def test_hermite_negative_order(self):
        """hermite(-1, x) is undefined"""
        with pytest.raises(ValueError, match="non-negative integer"):
            Expr(1).hermite(-1)

    def test_hermite_valid_passes(self):
        """hermite(2, 1) should work"""
        result = Expr(1).hermite(2)
        assert result is not None


class TestLegendreDomain:
    """Test Associated Legendre polynomial domain restrictions."""

    def test_assoc_legendre_negative_l(self):
        """assoc_legendre(-1, 0, x) is undefined"""
        with pytest.raises(ValueError, match="non-negative integer"):
            Expr(0.5).assoc_legendre(-1, 0)

    def test_assoc_legendre_m_too_large(self):
        """assoc_legendre(1, 3, x) invalid: |m| > l"""
        with pytest.raises(ValueError, match=r"\|m\| must be <= l"):
            Expr(0.5).assoc_legendre(1, 3)

    def test_assoc_legendre_x_outside_domain(self):
        """assoc_legendre(1, 0, 2) invalid: |x| > 1"""
        with pytest.raises(ValueError, match=r"\|x\| must be <= 1"):
            Expr(2).assoc_legendre(1, 0)

    def test_assoc_legendre_valid_passes(self):
        """assoc_legendre(2, 1, 0.5) should work"""
        result = Expr(0.5).assoc_legendre(2, 1)
        assert result is not None


class TestSphericalHarmonicDomain:
    """Test spherical harmonic domain restrictions."""

    def test_spherical_harmonic_negative_l(self):
        """spherical_harmonic(-1, 0, theta, phi) is undefined"""
        with pytest.raises(ValueError, match="non-negative integer"):
            Expr(0.5).spherical_harmonic(-1, 0, 0)

    def test_spherical_harmonic_m_too_large(self):
        """spherical_harmonic(1, 3, theta, phi) invalid: |m| > l"""
        with pytest.raises(ValueError, match=r"\|m\| must be <= l"):
            Expr(0.5).spherical_harmonic(1, 3, 0)


class TestZetaDerivDomain:
    """Test zeta derivative domain restrictions."""

    def test_zeta_deriv_at_pole(self):
        """zeta_deriv(1, 1) is undefined"""
        with pytest.raises(ValueError, match="pole at s=1"):
            Expr(1).zeta_deriv(1)

    def test_zeta_deriv_negative_order(self):
        """zeta_deriv(-1, 2) invalid order"""
        with pytest.raises(ValueError, match="non-negative integer"):
            Expr(2).zeta_deriv(-1)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
