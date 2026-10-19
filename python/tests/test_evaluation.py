"""Native exact/float execution, batch shapes, and independent workspaces."""

import math
import sys
import unittest
from fractions import Fraction

from symb_anafis import Context, EvaluationError, Evaluator, Expr, Number


class NumericalPrograms(unittest.TestCase):
    def test_exact_rational_and_complex_execution(self):
        x, y = Context().symbols("x", "y")
        expression = (x+y)/3
        result = expression.compile([y, x]).evaluate([Fraction(1, 3), 2])
        self.assertEqual(result.real, Fraction(7, 9))
        self.assertTrue(result.is_exact)
        value = Number.complex(Fraction(1, 3), Fraction(2, 5))
        result = (x+Expr.i()).compile([x]).evaluate([value])
        self.assertEqual(result.real, Fraction(1, 3))
        self.assertEqual(result.imag, Fraction(7, 5))
        self.assertTrue(result.is_exact)
        self.assertEqual(expression.evaluate({x: 2, y: 1}), 1)

    def test_float_programs_direct_execution_and_diagnostics(self):
        x = Context().symb("x")
        expression = x.sin()+x.cos()+x**3
        program = expression.compile([x], backend="f64")
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
        program = (x**2+1).compile([x])
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
            (x+y).compile([x])
        with self.assertRaises(EvaluationError):
            x.compile([x, x])
        with self.assertRaises(EvaluationError):
            x.evaluate([(x, 1), (x, 2)])
        with self.assertRaises(TypeError):
            x.compile([x+1])
        with self.assertRaises(TypeError):
            x.compile([x], backend="f64").evaluate([1j])
        with self.assertRaises(TypeError):
            x.evaluate({x: True})
        with self.assertRaises(ValueError):
            x.compile([x], backend="unknown")
        function = context.register_function("unknown", 1)
        with self.assertRaises(EvaluationError):
            function(x).compile([x])
