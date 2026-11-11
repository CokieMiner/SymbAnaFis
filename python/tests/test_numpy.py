"""Comprehensive NumPy interoperability: ufuncs, zero-copy complex/real buffers, N-D grids, and broadcasting."""

import array
from fractions import Fraction
import unittest

try:
    import numpy as np
except ImportError:
    np = None  # type: ignore[assignment]

from symb_anafis import (
    Context, CovarianceMatrix, EvaluationError, Evaluator, Expr, Number, Pattern,
    RewriteRule, SimplificationSequence, Substitution, Substitutions, Uncertainty,
)


@unittest.skipIf(np is None, "NumPy is not installed")
class NumPySymbolicInterop(unittest.TestCase):
    def test_symmetric_array_arithmetic_and_ufuncs_on_expr(self):
        ctx = Context()
        x, y = ctx.symbols("x", "y")
        arr = np.array([1.0, 2.0, 3.0])

        lhs = x + arr
        rhs = arr + x
        self.assertIsInstance(lhs, np.ndarray)
        self.assertEqual(lhs.shape, (3,))
        self.assertEqual(list(lhs), [x + 1.0, x + 2.0, x + 3.0])
        self.assertEqual(list(rhs), [1.0 + x, 2.0 + x, 3.0 + x])

        prod = x * np.array([[1, 2], [3, 4]])
        self.assertEqual(prod.shape, (2, 2))
        self.assertEqual(prod[1, 0], 3 * x)

        # Standard NumPy ufuncs map cleanly to SymbAnaFis symbolic methods.
        self.assertEqual(np.sin(x), x.sin())
        self.assertEqual(np.cos(x), x.cos())
        self.assertEqual(np.tan(x), x.tan())
        self.assertEqual(np.arcsin(x), x.asin())
        self.assertEqual(np.arccos(x), x.acos())
        self.assertEqual(np.arctan(x), x.atan())
        self.assertEqual(np.arctan2(y, x), y.atan2(x))
        self.assertEqual(np.arctan2(1.0, x), Expr(1.0).atan2(x))
        self.assertEqual(np.sinh(x), x.sinh())
        self.assertEqual(np.cosh(x), x.cosh())
        self.assertEqual(np.tanh(x), x.tanh())
        self.assertEqual(np.arcsinh(x), x.asinh())
        self.assertEqual(np.arccosh(x), x.acosh())
        self.assertEqual(np.arctanh(x), x.atanh())
        self.assertEqual(np.exp(x), x.exp())
        self.assertEqual(np.expm1(x), x.exp_m1())
        self.assertEqual(np.log(x), x.ln())
        self.assertEqual(np.log1p(x), x.ln_1p())
        self.assertEqual(np.log2(x), x.log2())
        self.assertEqual(np.log10(x), x.log10())
        self.assertEqual(np.sqrt(x), x.sqrt())
        self.assertEqual(np.cbrt(x), x.cbrt())
        self.assertEqual(np.abs(x), x.abs())
        self.assertEqual(np.sign(x), x.signum())
        self.assertEqual(np.conj(x), x.conj())
        self.assertEqual(np.conjugate(x), x.conj())
        self.assertEqual(np.hypot(x, y), x.hypot(y))
        self.assertEqual(np.maximum(x, y), x.max(y))
        self.assertEqual(np.minimum(x, y), x.min(y))
        self.assertEqual(np.mod(x, y), x.modulo(y))
        self.assertEqual(np.remainder(x, y), x.modulo(y))
        self.assertEqual(np.fmod(x, y), x.rem(y))
        self.assertEqual(np.floor(x), x.floor())
        self.assertEqual(np.ceil(x), x.ceil())
        self.assertEqual(np.trunc(x), x.trunc())
        self.assertEqual(np.rint(x), x.round())
        self.assertEqual(round(x), x.round())
        self.assertEqual(np.square(x), x**2)
        self.assertEqual(np.power(x, 3), x**3)
        self.assertEqual(np.add(x, y), x + y)

        # Mixed scalar + array ufunc call broadcasts cleanly.
        mixed = np.arctan2(np.array([1.0, 2.0]), x)
        self.assertEqual(list(mixed), [Expr(1.0).atan2(x), Expr(2.0).atan2(x)])

    def test_ufuncs_on_patterns_and_numpy_scalars_on_number(self):
        x = Context().symb("x")
        a = Pattern.capture("a")
        rule = RewriteRule(np.log(np.exp(a)), a)
        self.assertEqual(rule.apply(x.exp().ln()), x)

        # Number equality and arithmetic with NumPy scalars stay exact and return Python bool.
        self.assertEqual(Number(Fraction(1, 3)) + np.int64(1), Number(Fraction(4, 3)))
        self.assertEqual(np.int64(1) + Number(Fraction(1, 3)), Number(Fraction(4, 3)))
        self.assertEqual(Number(2) * np.float32(1.5), Number(3.0))
        eq_result = Number(2) == np.complex128(2 + 0j)
        self.assertIs(type(eq_result), bool)
        self.assertTrue(eq_result)
        self.assertFalse(Number(1) == np.bool_(True))
        with self.assertRaises(TypeError):
            _ = Number(2) + np.bool_(True)
        with self.assertRaises(TypeError):
            _ = x + np.bool_(True)


@unittest.skipIf(np is None, "NumPy is not installed")
class NumPyBatchEvaluation(unittest.TestCase):
    def test_zero_copy_complex_and_real_buffers_and_aliasing(self):
        x, y = Context().symbols("x", "y")
        expr = x**2 + 2 * y + 1

        for backend, dtype in (
            ("f64", np.float64),
            ("f32", np.float32),
            ("complex128", np.complex128),
            ("complex64", np.complex64),
        ):
            for native in (False, True):
                prog = Evaluator(expr, [x, y], backend=backend, native=native)
                if np.issubdtype(dtype, np.complexfloating):
                    xs = np.array([1 + 2j, -0.5 + 1j, 3 - 1j, 0j], dtype=dtype)
                    ys = np.array([2 - 1j, 1 + 0.5j, -1 + 2j, 4j], dtype=dtype)
                else:
                    xs = np.array([1.0, -2.5, 3.0, 0.25], dtype=dtype)
                    ys = np.array([4.0, 1.5, -1.0, 2.0], dtype=dtype)
                expected = xs**2 + 2 * ys + 1

                # Scalar evaluation with 1D NumPy array
                s_val = prog.evaluate(np.array([xs[0], ys[0]], dtype=dtype))
                np.testing.assert_allclose(s_val, expected[0], rtol=1e-5, atol=1e-5)

                # Batch sequential & parallel
                res = prog.evaluate_batch([xs, ys])
                res_par = prog.evaluate_batch([xs, ys], parallel=True)
                self.assertIsInstance(res, np.ndarray)
                self.assertEqual(res.dtype, dtype)
                np.testing.assert_allclose(res, expected, rtol=1e-5, atol=1e-5)
                np.testing.assert_allclose(res_par, expected, rtol=1e-5, atol=1e-5)

                # Writable out= buffer (distinct & aliased in-place)
                out = np.empty_like(xs)
                ret = prog.evaluate_batch([xs, ys], out=out)
                self.assertIs(ret, out)
                np.testing.assert_allclose(out, expected, rtol=1e-5, atol=1e-5)

                aliased = xs.copy()
                ret_alias = prog.evaluate_batch([aliased, ys], out=aliased, parallel=True)
                self.assertIs(ret_alias, aliased)
                np.testing.assert_allclose(aliased, expected, rtol=1e-5, atol=1e-5)

                # Read-only out= rejected
                ro = np.empty_like(xs)
                ro.flags.writeable = False
                with self.assertRaises(ValueError):
                    prog.evaluate_batch([xs, ys], out=ro)

    def test_nd_meshgrid_sparse_broadcasting_and_scalar_broadcasting(self):
        x, y = Context().symbols("x", "y")
        prog = Evaluator(x**2 + 3 * y, [x, y], backend="f64", native=True)

        # Dense 2D meshgrid
        X, Y = np.meshgrid(np.linspace(-1.0, 1.0, 5), np.linspace(0.0, 2.0, 4))
        Z = prog.evaluate_batch([X, Y])
        self.assertEqual(Z.shape, (4, 5))
        np.testing.assert_allclose(Z, X**2 + 3 * Y, rtol=1e-12)

        # 2D out= buffer (distinct and aliased in-place)
        out2d = np.empty_like(X)
        ret = prog.evaluate_batch([X, Y], out=out2d)
        self.assertIs(ret, out2d)
        np.testing.assert_allclose(out2d, X**2 + 3 * Y, rtol=1e-12)

        X_alias = X.copy()
        prog.evaluate_batch([X_alias, Y], out=X_alias)
        np.testing.assert_allclose(X_alias, X**2 + 3 * Y, rtol=1e-12)

        # Wrong 2D out shape rejected
        with self.assertRaises(ValueError):
            prog.evaluate_batch([X, Y], out=np.empty((5, 4), dtype=np.float64))

        # Sparse 2D meshgrid broadcasting: (1, 5) + (4, 1) -> (4, 5)
        Xs, Ys = np.meshgrid(np.linspace(-1.0, 1.0, 5), np.linspace(0.0, 2.0, 4), sparse=True)
        Zs = prog.evaluate_batch([Xs, Ys])
        self.assertEqual(Zs.shape, (4, 5))
        np.testing.assert_allclose(Zs, X**2 + 3 * Y, rtol=1e-12)

        # Scalar broadcasting against 1D and 2D arrays
        xs = np.array([1.0, 2.0, 3.0])
        res_1d = prog.evaluate_batch([xs, 10.0])
        np.testing.assert_allclose(res_1d, xs**2 + 30.0)

        res_2d = prog.evaluate_batch([X, 2.0])
        self.assertEqual(res_2d.shape, X.shape)
        np.testing.assert_allclose(res_2d, X**2 + 6.0)

        # All-1D mismatched lengths still raise EvaluationError (no accidental 1-element broadcast)
        with self.assertRaises(EvaluationError):
            prog.evaluate_batch([np.array([1.0, 2.0]), np.array([3.0])])

    def test_strided_views_dtype_conversion_and_strict_rejection(self):
        x, y = Context().symbols("x", "y")
        prog = Evaluator(x + 2 * y, [x, y], backend="f64")

        # Strided slices & transposed 2D matrix
        base = np.arange(10, dtype=np.float64)
        np.testing.assert_allclose(
            prog.evaluate_batch([base[::2], base[1::2]]),
            base[::2] + 2 * base[1::2],
        )
        points = np.array([[1.0, 10.0], [2.0, 20.0], [3.0, 30.0]], dtype=np.float64)
        np.testing.assert_allclose(prog.evaluate_batch(points.T), [21.0, 42.0, 63.0])

        # Convertible numeric dtypes (int64, float32 -> f64)
        np.testing.assert_allclose(
            prog.evaluate_batch([np.arange(4, dtype=np.int64), np.arange(4, dtype=np.float32)]),
            [0.0, 3.0, 6.0, 9.0],
        )

        # Complex arrays/scalars MUST be rejected on real backends (never silently drop imag!)
        with self.assertRaises(TypeError):
            prog.evaluate_batch([np.array([1 + 1j, 2j]), np.array([1.0, 2.0])])
        with self.assertRaises(TypeError):
            prog.evaluate(np.array([1 + 1j, 2.0]))
        with self.assertRaises(TypeError):
            prog.evaluate([np.complex128(1 + 2j), 1.0])
        with self.assertRaises(TypeError):
            prog.evaluate_batch([np.array([1.0, 2.0]), np.complex64(1 + 2j)])

        # 2D / 0D arrays rejected for scalar evaluate()
        with self.assertRaises(ValueError):
            prog.evaluate(np.array([[1.0, 2.0]]))
        with self.assertRaises(ValueError):
            Evaluator(x + 1, [x], backend="f64").evaluate(np.array(3.0))

        # Stdlib array.array / memoryview buffer protocol works too
        buf_x = array.array("d", [1.0, 2.0, 3.0])
        buf_y = array.array("d", [4.0, 5.0, 6.0])
        np.testing.assert_allclose(prog.evaluate_batch([buf_x, buf_y]), [9.0, 12.0, 15.0])

        # Informative reprs on all public wrapper classes
        self.assertIn("Evaluator(", repr(prog))
        self.assertIn("CovarianceMatrix(", repr(CovarianceMatrix.diagonal([1, 2])))
        u = Uncertainty.propagate(x + y, [x, y], CovarianceMatrix.diagonal([1, 4]))
        self.assertIn("Uncertainty(", repr(u))
        self.assertIn(r"\pm", u._repr_latex_())
        self.assertIn("Pattern(", repr(Pattern.capture("a")))
        self.assertIn("RewriteRule(", repr(RewriteRule(Pattern.capture("a").sin(), 0)))
        self.assertIn("Substitutions(", repr(Substitutions({x: y})))
        self.assertIn("Substitution(", repr(Substitution(x, y)))
        self.assertIn("SimplificationSequence(", repr(SimplificationSequence.compact()))
