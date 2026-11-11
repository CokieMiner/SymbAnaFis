"""Real covariance formulas, domain exclusions, and independently generated PSD inputs."""

import random
import unittest

from symb_anafis import Context, CovarianceMatrix, Expr, Property, Truth, Uncertainty, UncertaintyError


class CovariancePropagation(unittest.TestCase):
    def test_numeric_diagonal_correlated_and_symbolic_variances(self):
        x, y, sx, sy = Context().symbols("x", "y", "sx", "sy")
        diagonal = CovarianceMatrix.diagonal([4, 9])
        correlated = CovarianceMatrix([[4, 1], [1, 9]])
        self.assertEqual(Uncertainty.propagate(x+2*y, [x, y], diagonal).variance, Expr(40))
        result = Uncertainty.propagate(x+2*y, [x, y], correlated)
        self.assertEqual(result.variance, Expr(44))
        self.assertEqual(result.sensitivities, (Expr(1), Expr(2)))
        self.assertEqual(result.standard_deviation(), Expr(44).sqrt())
        self.assertEqual(result.relative(), Expr(44).sqrt()/(x+2*y).abs())
        symbolic = CovarianceMatrix.diagonal([sx**2, sy**2])
        self.assertEqual(Uncertainty.propagate(x+2*y, [x, y], symbolic).variance, sx**2+4*sy**2)
        self.assertEqual(diagonal.dimension, 2)
        self.assertEqual(diagonal[0, 1], Expr(0))
        self.assertIsNone(diagonal.get(2, 0))
        with self.assertRaises(IndexError):
            _ = diagonal[-1, 0]

    def test_shape_symmetry_dimensions_and_disproved_domains(self):
        x = Context().symb("x")
        for entries in ([[1, 0]], [[1, 1e-30], [0, 1]], [[-1]], [[1j]], [[float("inf")]]):
            with self.assertRaises(UncertaintyError):
                CovarianceMatrix(entries)
        one, two = CovarianceMatrix.diagonal([1]), CovarianceMatrix.diagonal([1, 1])
        with self.assertRaises(UncertaintyError):
            Uncertainty.propagate(x, [], one)
        with self.assertRaises(UncertaintyError):
            Uncertainty.propagate(x, [x, x], two)
        with self.assertRaises(UncertaintyError):
            Uncertainty.propagate(Expr.i(), [x], one)

    def test_zero_covariances_retain_poles_and_numeric_approximation(self):
        x, y = Context().symbols("x", "y")
        diagonal = CovarianceMatrix.diagonal([0, 0])
        dense = CovarianceMatrix([[0, 0], [0, 0]])
        expression = x.ln()+y.ln()
        first = Uncertainty.propagate(expression, [x, y], diagonal).variance
        self.assertEqual(first, Uncertainty.propagate(expression, [x, y], dense).variance)
        at_pole = first.substitute(x, 0)
        self.assertNotEqual(at_pole, Expr(0))
        self.assertIsNot(at_pole.properties[Property.FINITE], Truth.TRUE)
        mixed = CovarianceMatrix([[2, 1], [1.0, 2]])
        result = Uncertainty.propagate(x+y, [x, y], mixed)
        self.assertEqual(result.variance, Expr(6.0))
        constant = Uncertainty.propagate(5, [x], CovarianceMatrix.diagonal([2.0]))
        self.assertEqual(constant.variance, Expr(0.0))

    def test_generated_psd_covariances_follow_independent_sensitivity_formulas(self):
        generator = random.Random(13973)
        x, y = Context().symbols("x", "y")
        for _ in range(48):
            a, b, c = generator.randint(1, 5), generator.randint(-4, 4), generator.randint(1, 5)
            p, q = generator.randint(-5, 5), generator.randint(-5, 5)
            covariance = CovarianceMatrix([[a*a, a*b], [a*b, b*b+c*c]])
            result = Uncertainty.propagate(x**2+x*y, [x, y], covariance)
            first, second = 2*p+q, p
            expected = a*a*first*first+2*a*b*first*second+(b*b+c*c)*second*second
            self.assertEqual(result.variance.evaluate({x: p, y: q}), expected)
