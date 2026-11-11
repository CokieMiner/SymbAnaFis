"""Native exact/float execution, batch shapes, and independent workspaces."""

import math
import sys
import unittest
from fractions import Fraction

try:
    import numpy as np
except ImportError:
    np = None  # type: ignore[assignment]

from symb_anafis import Context, EvaluationError, Evaluator, Expr, Number


class NumericalPrograms(unittest.TestCase):
    def test_exact_rational_and_complex_execution(self):
        x, y = Context().symbols("x", "y")
        expression = (x+y)/3
        result = Evaluator(expression, [y, x]).evaluate([Fraction(1, 3), 2])
        self.assertEqual(result.real, Fraction(7, 9))
        self.assertTrue(result.is_exact)
        value = Number.complex(Fraction(1, 3), Fraction(2, 5))
        result = Evaluator(x+Expr.i(), [x]).evaluate([value])
        self.assertEqual(result.real, Fraction(1, 3))
        self.assertEqual(result.imag, Fraction(7, 5))
        self.assertTrue(result.is_exact)
        self.assertEqual(expression.evaluate({x: 2, y: 1}), 1)

    def test_float_programs_direct_execution_and_diagnostics(self):
        x = Context().symb("x")
        expression = x.sin()+x.cos()+x**3
        program = Evaluator(expression, [x], backend="f64")
        for value in (-1.25, 0.0, 0.5, 2.0):
            expected = math.sin(value)+math.cos(value)+value**3
            self.assertAlmostEqual(program.evaluate([value]), expected, places=13)
            self.assertAlmostEqual(expression.evaluate({x: value}, backend="f64"), expected, places=13)
        self.assertEqual(program.parameters, (x,))
        self.assertGreater(program.instruction_count, 0)
        self.assertGreaterEqual(program.workspace_size, 1)
        self.assertIsInstance(program.disassemble(), str)
        self.assertGreater(len(program.disassemble()), 0)

    def test_batch_columns_zero_parameter_rows_and_shape_errors(self):
        x, y = Context().symbols("x", "y")
        for backend in ("number", "f64"):
            program = Evaluator(x+2*y, [x, y], backend=backend)
            self.assertEqual(program.evaluate_batch([[1, 2, 3], [4, 5, 6]]), [9, 12, 15])
            with self.assertRaises(EvaluationError):
                program.evaluate_batch([[1, 2], [3]])
            with self.assertRaises(EvaluationError):
                program.evaluate_batch([[1, 2]])
            with self.assertRaises(EvaluationError):
                program.evaluate([1])
            constant = Evaluator(7, [], backend=backend)
            self.assertEqual(constant.evaluate([]), 7)
            self.assertEqual(constant.evaluate_batch([], rows=3), [7, 7, 7])
            self.assertEqual(constant.evaluate_batch([]), [])
            with self.assertRaises(MemoryError):
                constant.evaluate_batch([], rows=sys.maxsize)

    def test_workspaces_are_independent_and_symbols_outlive_the_context(self):
        context = Context()
        x = context.symb("x")
        program = Evaluator(x**2+1, [x])
        other = program.copy()
        context.reset()
        del context
        self.assertEqual(program.evaluate([2]), 5)
        self.assertEqual(other.evaluate([3]), 10)
        self.assertEqual(program.evaluate([4]), 17)

    def test_invalid_coordinates_bindings_and_bodies_do_not_become_numbers(self):
        context = Context()
        x, y = context.symbols("x", "y")
        with self.assertRaises(EvaluationError):
            Evaluator(x+y, [x])
        with self.assertRaises(EvaluationError):
            Evaluator(x, [x, x])
        with self.assertRaises(EvaluationError):
            x.evaluate([(x, 1), (x, 2)])
        with self.assertRaises(TypeError):
            Evaluator(x, [x+1])
        with self.assertRaises(TypeError):
            Evaluator(x, [x], backend="f64").evaluate([1j])
        with self.assertRaises(TypeError):
            x.evaluate({x: True})
        with self.assertRaises(ValueError):
            Evaluator(x, [x], backend="unknown")
        function = context.register_function("unknown", 1)
        with self.assertRaises(EvaluationError):
            Evaluator(function(x), [x])

    def test_buffer_protocol_numpy_interop_and_out_buffer(self):
        if np is None:
            return
        x, y = Context().symbols("x", "y")
        program = Evaluator(x**2 + y, [x, y], backend="f64")
        x_arr = np.array([1.0, 2.0, 3.0], dtype=np.float64)
        y_arr = np.array([4.0, 5.0, 6.0], dtype=np.float64)

        result = program.evaluate_batch([x_arr, y_arr])
        self.assertIsInstance(result, np.ndarray)
        np.testing.assert_allclose(result, [5.0, 9.0, 15.0])

        out_buf = np.empty(3, dtype=np.float64)
        out_res = program.evaluate_batch([x_arr, y_arr], out=out_buf)
        self.assertIs(out_res, out_buf)
        np.testing.assert_allclose(out_buf, [5.0, 9.0, 15.0])

        # Read-only output buffers must be rejected cleanly.
        readonly_out = np.empty(3, dtype=np.float64)
        readonly_out.flags.writeable = False
        with self.assertRaises(ValueError):
            program.evaluate_batch([x_arr, y_arr], out=readonly_out)

        # In-place aliasing (out=x_arr) must stage safely without corrupting reads.
        aliased = np.array([1.0, 2.0, 3.0], dtype=np.float64)
        res_alias = program.evaluate_batch([aliased, y_arr], out=aliased)
        self.assertIs(res_alias, aliased)
        np.testing.assert_allclose(aliased, [5.0, 9.0, 15.0])

        # Undersized or 2D output buffers are rejected.
        with self.assertRaises(ValueError):
            program.evaluate_batch([x_arr, y_arr], out=np.empty(2, dtype=np.float64))
        with self.assertRaises(ValueError):
            program.evaluate_batch([x_arr, y_arr], out=np.empty((1, 3), dtype=np.float64))

        # NumPy scalar inputs in Python lists are coerced; NumPy bools are rejected.
        self.assertAlmostEqual(program.evaluate([np.float64(2.0), np.int64(3)]), 7.0)
        with self.assertRaises(TypeError):
            program.evaluate([np.bool_(True), 1.0])
        with self.assertRaises(TypeError):
            program.evaluate_batch([np.array([True, False]), np.array([1.0, 2.0])])

    def test_f32_and_complex_backends_and_parallel_batches(self):
        x, y = Context().symbols("x", "y")
        expr_real = x**2 + 2 * y
        f32_prog = Evaluator(expr_real, [x, y], backend="f32")
        self.assertAlmostEqual(f32_prog.evaluate([3.0, 4.0]), 17.0, places=5)
        self.assertAlmostEqual(expr_real.evaluate({x: 3.0, y: 4.0}, backend="f32"), 17.0, places=5)
        self.assertEqual(f32_prog.evaluate_batch([[1.0, 2.0], [3.0, 4.0]]), [7.0, 12.0])
        self.assertEqual(
            f32_prog.evaluate_batch([[1.0, 2.0], [3.0, 4.0]], parallel=True), [7.0, 12.0]
        )

        # Number backend rejects out= explicitly.
        with self.assertRaises(TypeError):
            Evaluator(x + 1, [x], backend="number").evaluate_batch([[1, 2]], out=[0, 0])

        expr_complex = x**2 + y
        for backend in ("complex128", "complex64"):
            prog = Evaluator(expr_complex, [x, y], backend=backend)
            self.assertAlmostEqual(prog.evaluate([1j, 1]), 0j, places=5)
            self.assertAlmostEqual(expr_complex.evaluate({x: 1j, y: 1}, backend=backend), 0j, places=5)
            batch = prog.evaluate_batch([[1j, 2 + 1j], [1, -1]])
            batch_par = prog.evaluate_batch([[1j, 2 + 1j], [1, -1]], parallel=True)
            expected = [0j, 2 + 4j]
            for got, got_par, exp in zip(batch, batch_par, expected):
                self.assertAlmostEqual(got, exp, places=5)
                self.assertAlmostEqual(got_par, exp, places=5)

            if np is not None:
                dtype = np.complex128 if backend == "complex128" else np.complex64
                xs = np.array([1j, 2 + 1j], dtype=dtype)
                ys = np.array([1, -1], dtype=dtype)
                arr = prog.evaluate_batch([xs, ys])
                self.assertIsInstance(arr, np.ndarray)
                self.assertEqual(arr.dtype, dtype)
                np.testing.assert_allclose(arr, expected, rtol=1e-5, atol=1e-5)

                out_ro = np.empty(2, dtype=dtype)
                out_ro.flags.writeable = False
                with self.assertRaises(ValueError):
                    prog.evaluate_batch([xs, ys], out=out_ro)

                aliased = xs.copy()
                res = prog.evaluate_batch([aliased, ys], out=aliased)
                self.assertIs(res, aliased)
                np.testing.assert_allclose(aliased, expected, rtol=1e-5, atol=1e-5)

    def test_native_jit_and_wasm_export(self):
        x, y = Context().symbols("x", "y")
        expr = x.sin() + y**2 + 1
        with self.assertRaises(EvaluationError):
            Evaluator(expr, [x, y], backend="number").with_native()
        with self.assertRaises(EvaluationError):
            Evaluator(expr, [x, y], backend="number").to_wasm_bytes()

        for backend in ("f64", "f32"):
            vm = Evaluator(expr, [x, y], backend=backend)
            self.assertIsNone(vm.native_bytes)
            jit = vm.with_native()
            jit_direct = Evaluator(expr, [x, y], backend=backend, native=True)
            self.assertIsInstance(jit.native_bytes, bytes)
            self.assertGreater(len(jit.native_bytes), 0)
            self.assertIsInstance(jit_direct.native_bytes, bytes)

            xs = [0.1 * i for i in range(35)]
            ys = [-0.5 + 0.05 * i for i in range(35)]
            places = 12 if backend == "f64" else 5
            self.assertAlmostEqual(jit.evaluate([0.5, 2.0]), vm.evaluate([0.5, 2.0]), places=places)
            vm_batch = vm.evaluate_batch([xs, ys])
            jit_batch = jit.evaluate_batch([xs, ys])
            jit_par = jit.evaluate_batch([xs, ys], parallel=True)
            for a, b, c in zip(vm_batch, jit_batch, jit_par):
                self.assertAlmostEqual(a, b, places=places)
                self.assertAlmostEqual(a, c, places=places)

            wasm = vm.to_wasm_bytes()
            self.assertIsInstance(wasm, bytes)
            self.assertTrue(wasm.startswith(b"\0asm\x01\0\0\0"))

        for backend in ("complex128", "complex64"):
            vm = Evaluator(x**2 + y, [x, y], backend=backend)
            jit = vm.with_native()
            self.assertIsInstance(jit.native_bytes, bytes)
            self.assertGreater(len(jit.native_bytes), 0)
            places = 12 if backend == "complex128" else 5
            self.assertAlmostEqual(
                jit.evaluate([1 + 2j, -3j]),
                vm.evaluate([1 + 2j, -3j]),
                places=places,
            )
            with self.assertRaises(EvaluationError):
                vm.to_wasm_bytes()

    def test_multi_output_evaluator_backends_cse_jit_wasm_and_aliasing(self):
        x, y = Context().symbols("x", "y")
        denom = x**2 + y**2 + 2
        e1 = (x.sin() + y) / denom
        e2 = (x.cos() - y) / denom

        # Shared CSE across outputs reduces total instructions and pairs sin/cos.
        p1 = Evaluator(e1, [x, y], backend="f64")
        p2 = Evaluator(e2, [x, y], backend="f64")
        joint = Evaluator([e1, e2], [x, y], backend="f64")
        self.assertEqual(joint.output_count, 2)
        self.assertLess(joint.instruction_count, p1.instruction_count + p2.instruction_count)
        listing = joint.disassemble()
        self.assertIn("sin_cos", listing)
        self.assertIn(", r", listing)
        self.assertIn("outputs=2", repr(joint))

        # Exact Number backend + inferred parameters.
        exact = Evaluator([x + y, x * y])
        self.assertEqual(exact.output_count, 2)
        self.assertEqual(exact.parameters, (x, y))
        self.assertEqual(exact.evaluate([3, 4]), (7, 12))
        self.assertEqual(exact.evaluate_batch([[1, 2], [3, 4]]), ([4, 6], [3, 8]))
        self.assertEqual(exact.evaluate_batch([[1, 2], [3, 4]], parallel=True), ([4, 6], [3, 8]))

        # Empty outputs and single-item list.
        empty = Evaluator([], [x, y], backend="f64", native=True)
        self.assertEqual(empty.output_count, 0)
        self.assertEqual(empty.evaluate([1.0, 2.0]), ())
        self.assertEqual(empty.evaluate_batch([[1.0, 2.0], [3.0, 4.0]]), ())
        single_list = Evaluator([x + 1], [x], backend="f64")
        self.assertEqual(single_list.output_count, 1)
        self.assertEqual(single_list.evaluate([3.0]), (4.0,))

        # Native JIT + WASM + complex backends.
        for backend in ("f64", "f32"):
            vm = Evaluator([e1, e2], [x, y], backend=backend)
            jit = vm.with_native()
            self.assertIsInstance(jit.native_bytes, bytes)
            self.assertTrue(vm.to_wasm_bytes().startswith(b"\0asm\x01\0\0\0"))
            places = 12 if backend == "f64" else 5
            v0, v1 = vm.evaluate([0.5, 1.5])
            j0, j1 = jit.evaluate([0.5, 1.5])
            self.assertAlmostEqual(v0, j0, places=places)
            self.assertAlmostEqual(v1, j1, places=places)
            self.assertAlmostEqual(v0, p1.evaluate([0.5, 1.5]), places=places)
            self.assertAlmostEqual(v1, p2.evaluate([0.5, 1.5]), places=places)

        c_jit = Evaluator([x + y, x * y], [x, y], backend="complex128", native=True)
        c0, c1 = c_jit.evaluate([1 + 2j, 3 - 1j])
        self.assertAlmostEqual(c0, 4 + 1j, places=12)
        self.assertAlmostEqual(c1, 5 + 5j, places=12)

        if np is None:
            return

        jit = joint.with_native()
        xs = np.linspace(-2.0, 2.0, 65, dtype=np.float64)
        ys = np.linspace(0.5, 3.0, 65, dtype=np.float64)
        r0, r1 = jit.evaluate_batch([xs, ys], parallel=True)
        self.assertIsInstance(r0, np.ndarray)
        self.assertIsInstance(r1, np.ndarray)
        np.testing.assert_allclose(r0, p1.evaluate_batch([xs, ys]), rtol=1e-12, atol=1e-12)
        np.testing.assert_allclose(r1, p2.evaluate_batch([xs, ys]), rtol=1e-12, atol=1e-12)

        # Zero-copy out=(out0, out1).
        out0 = np.empty_like(xs)
        out1 = np.empty_like(xs)
        ret0, ret1 = jit.evaluate_batch([xs, ys], out=(out0, out1), parallel=True)
        self.assertIs(ret0, out0)
        self.assertIs(ret1, out1)
        np.testing.assert_allclose(out0, r0, rtol=1e-12, atol=1e-12)
        np.testing.assert_allclose(out1, r1, rtol=1e-12, atol=1e-12)

        # N-D meshgrid + scalar broadcast preserves shape on every output.
        gx, gy = np.meshgrid(np.linspace(-1.0, 1.0, 4), np.linspace(0.5, 1.5, 3), sparse=True)
        g0, g1 = jit.evaluate_batch([gx, gy])
        self.assertEqual(g0.shape, (3, 4))
        self.assertEqual(g1.shape, (3, 4))

        # Input-output in-place aliasing AND output-output duplicate buffer aliasing.
        ax, ay = xs.copy(), ys.copy()
        jit.evaluate_batch([ax, ay], out=(ax, ay), parallel=True)
        np.testing.assert_allclose(ax, r0, rtol=1e-12, atol=1e-12)
        np.testing.assert_allclose(ay, r1, rtol=1e-12, atol=1e-12)

        dup = np.empty_like(xs)
        jit.evaluate_batch([xs, ys], out=(dup, dup))
        np.testing.assert_allclose(dup, r1, rtol=1e-12, atol=1e-12)

        # Validation errors for wrong out container/length/read-only.
        with self.assertRaises(TypeError):
            jit.evaluate_batch([xs, ys], out=out0)
        with self.assertRaises(ValueError):
            jit.evaluate_batch([xs, ys], out=(out0,))
        ro = np.empty_like(xs)
        ro.flags.writeable = False
        with self.assertRaises(ValueError):
            jit.evaluate_batch([xs, ys], out=(out0, ro))
