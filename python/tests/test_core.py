"""Numeric representation, symbol ownership, and independent calculus oracles."""

import gc
import inspect
import random
import struct
import unittest
from fractions import Fraction

import symb_anafis as symbolic
from symb_anafis import Context, Expr, Function, Number, ParseError, Property, Symbol, Truth


class CoreContracts(unittest.TestCase):
    def test_public_exports_are_namespaced_types(self):
        for name in symbolic.__all__:
            self.assertTrue(inspect.isclass(getattr(symbolic, name)), name)

    def test_exact_and_approximate_literal_identity(self):
        self.assertNotEqual(Expr(2), Expr(2.0))
        self.assertNotEqual(Expr(0.0), Expr(-0.0))
        exact = Number(Fraction(-7, 13))
        self.assertEqual(exact.real, Fraction(-7, 13))
        self.assertTrue(exact.is_exact)
        self.assertFalse(Number(2.0).is_exact)
        self.assertEqual(Number(Fraction(1, 2**63 + 1)).real, Fraction(1, 2**63 + 1))
        for unsupported in (True, False, "3"):
            with self.assertRaises(TypeError):
                Number(unsupported)
        for unsupported in (2**63, Fraction(1, 2**64 + 1)):
            with self.assertRaises(OverflowError):
                Number(unsupported)

    def test_complex_components_and_nan_payloads_remain_distinct(self):
        value = Number.complex(Fraction(2, 3), Fraction(-3, 5))
        self.assertEqual(value.real, Fraction(2, 3))
        self.assertEqual(value.imag, Fraction(-3, 5))
        self.assertTrue(value.is_exact)
        self.assertFalse(Number(2 + 3j).is_exact)
        with self.assertRaises(TypeError):
            float(value)
        first = struct.unpack("!d", bytes.fromhex("7ff8000000000001"))[0]
        second = struct.unpack("!d", bytes.fromhex("7ff8000000000002"))[0]
        self.assertEqual(Expr(first), Expr(first))
        self.assertNotEqual(Expr(first), Expr(second))

    def test_expression_hash_and_immutable_structure(self):
        context = Context()
        x = context.symb("x")
        first, second = x + 1, context.parse("x + 1")
        self.assertEqual(first, second)
        self.assertEqual(hash(first), hash(second))
        self.assertEqual({first: 7}[second], 7)
        self.assertNotEqual(Expr(1), 1)
        with self.assertRaises(TypeError):
            bool(first)
        with self.assertRaises(AttributeError):
            first._native = second._native

    def test_scoped_symbols_survive_unbinding_reset_and_context_release(self):
        context, other = Context(), Context()
        x, other_x = context.symb("x"), other.symb("x")
        self.assertNotEqual(x, other_x)
        self.assertEqual(len({x, other_x}), 2)
        self.assertTrue(context.remove_symbol("x"))
        replacement = context.symb("x")
        self.assertNotEqual(x, replacement)
        context.reset()
        self.assertEqual(context.symbol_names, ())
        del context
        gc.collect()
        self.assertEqual((x**2).derivative(x), 2 * x)
        self.assertEqual((x**2).derivative(other_x), Expr(0))
        self.assertEqual(Symbol("python_default_x"), Context.default().symb("python_default_x"))

    def test_parser_and_property_errors_are_explicit(self):
        context = Context()
        x = context.symb("x")
        self.assertEqual(context.parse("2x + sin(x)"), 2*x + x.sin())
        with self.assertRaises(ParseError):
            context.parse("sin(")
        with self.assertRaises(ParseError) as failure:
            context.parse("α + @")
        self.assertEqual(failure.exception.span, (5, 6))
        self.assertEqual(failure.exception.source, "α + @")
        self.assertEqual(type(failure.exception).__module__, "symb_anafis")
        self.assertIs(x.properties[Property.FINITE], Truth.TRUE)
        self.assertIs(x.properties[Property.REAL], Truth.UNKNOWN)
        self.assertIs(x.abs().properties[Property.NONNEGATIVE], Truth.TRUE)
        with self.assertRaises(TypeError):
            bool(Truth.UNKNOWN)
        with self.assertRaises(ValueError):
            Function.builtin("not_registered")

    def test_all_builtin_methods_use_known_function_identities(self):
        x = Context().symb("x")
        unary = "sin cos tan cot sec csc asin acos atan acot asec acsc sinh cosh tanh coth sech csch asinh acosh atanh acoth asech acsch exp exp_m1 exp_neg ln ln_1p log10 log2 sqrt cbrt floor ceil round trunc fract abs signum sinc erf erfc gamma lgamma digamma trigamma tetragamma zeta elliptic_k elliptic_e".split()
        for method in unary:
            self.assertIsInstance(getattr(x, method)(), Expr, method)
        for method in "polygamma besselj bessely besseli besselk zeta_deriv hermite lambertw".split():
            self.assertIsInstance(getattr(x, method)(2), Expr, method)
        self.assertIsInstance(x.log(2), Expr)
        self.assertIsInstance(x.beta(2), Expr)
        self.assertIsInstance(x.atan2(2), Expr)
        self.assertIsInstance(x.assoc_legendre(2, 1), Expr)
        self.assertIsInstance(x.spherical_harmonic(2, 1, x), Expr)

    def test_multivariable_shapes_and_mixed_operator_order(self):
        context = Context()
        x, y = context.symbols("x", "y")
        expression = x**2 + x*y
        self.assertEqual(expression.gradient([y, x, x]), [x, 2*x+y, 2*x+y])
        self.assertEqual(expression.hessian([x, y]), [[Expr(2), Expr(1)], [Expr(1), Expr(0)]])
        self.assertEqual(Expr.jacobian([expression, x*y], [x, y]), [[2*x+y, x], [y, x]])
        self.assertEqual(Expr.jacobian([expression], []), [[]])
        self.assertEqual(Expr.jacobian([], [x]), [])
        self.assertEqual(expression.gradient([]), [])
        function = context.register_function("unknown", 2)
        hessian = function(x, y).hessian([x, y])
        self.assertNotEqual(hessian[0][1], hessian[1][0])
        self.assertEqual(hessian[0][1].view.variable, y)
        self.assertEqual(hessian[0][1].view.order, 1)
        with self.assertRaises(TypeError):
            expression.gradient([x + 1])

    def test_generated_polynomial_derivatives_and_evaluators_follow_integer_oracles(self):
        generator = random.Random(83129)
        context = Context()
        x, y = context.symbols("x", "y")
        for _ in range(64):
            a, b, c, d, e = [generator.randint(-6, 6) for _ in range(5)]
            first, second = generator.randint(-4, 4), generator.randint(-4, 4)
            expression = a*x**2 + b*x*y + c*y**2 + d*x + e*y
            expected = a*first**2 + b*first*second + c*second**2 + d*first + e*second
            self.assertEqual(expression.evaluate({x: first, y: second}), expected)
            self.assertEqual(expression.compile([y, x]).evaluate([second, first]), expected)
            gradient = expression.gradient([x, y])
            self.assertEqual(gradient[0].evaluate({x: first, y: second}), 2*a*first+b*second+d)
            self.assertEqual(gradient[1].evaluate({x: first, y: second}), b*first+2*c*second+e)
            self.assertEqual(expression.hessian([x, y]), [[Expr(2*a), Expr(b)], [Expr(b), Expr(2*c)]])


class CustomFunctions(unittest.TestCase):
    def test_symbolic_builders_are_executed_once_and_survive_the_context(self):
        calls = []
        context = Context()
        x = context.symb("x")

        def body(argument):
            calls.append(argument)
            return argument**3

        function = context.register_function("cube", 1, body=body)
        self.assertEqual(len(calls), 1)
        expression = function(x)
        self.assertTrue(context.remove_function("cube"))
        del context
        gc.collect()
        self.assertEqual(expression.derivative(x), 3*x**2)
        self.assertEqual(expression.compile([x]).evaluate([4]), 64)
        self.assertEqual(len(calls), 1)

    def test_explicit_sparse_partials_and_argument_swap_are_simultaneous(self):
        context = Context()
        x, y = context.symbols("x", "y")
        function = context.register_function("subtract", 2, parameters=[x, y], body=x-y,
                                             partials={0: 1, 1: -1})
        self.assertEqual(function(y, x).evaluate({x: 2, y: 5}), 3)
        self.assertEqual(function(y, x).derivative(x), Expr(-1))
        deferred = context.register_function("no_partials", 1, body=lambda value: value**2, partials={})
        self.assertEqual(deferred(x).derivative(x).kind, "derivative")
        with self.assertRaises(ValueError):
            context.register_function("subtract", 2)
        with self.assertRaises(ValueError):
            context.register_function("sin", 1)
        with self.assertRaises(ValueError):
            context.register_function("duplicate", 2, parameters=[x, x])
        with self.assertRaises(ValueError):
            function(x)

    def test_nullary_and_many_argument_templates_keep_the_registered_order(self):
        context = Context()
        constant = context.register_function("constant", 0, body=lambda: Fraction(3, 7))
        self.assertEqual(constant().compile([]).evaluate([]), Fraction(3, 7))
        variables = context.symbols(*(f"x{index}" for index in range(17)))
        function = context.register_function("weighted", len(variables), body=lambda *arguments:
            Expr.sum((index+1)*argument for index, argument in enumerate(arguments)))
        expression = function(*reversed(variables))
        values = list(range(2, 19))
        expected = sum((index+1)*value for index, value in enumerate(reversed(values)))
        self.assertEqual(expression.evaluate(zip(variables, values)), expected)
        self.assertEqual(expression.compile(variables).evaluate(values), expected)
        self.assertEqual(expression.gradient(variables), [Expr(value) for value in range(17, 0, -1)])

    def test_builder_exceptions_do_not_publish_function_bindings(self):
        context = Context()
        marker = RuntimeError("builder failed")

        def body(_):
            raise marker

        with self.assertRaises(RuntimeError) as failure:
            context.register_function("failed", 1, body=body)
        self.assertIs(failure.exception, marker)
        self.assertIsNone(context.get_function("failed"))

    def test_unresolved_template_derivatives_remain_symbolic_without_changing_the_variable(self):
        context = Context()
        x, y = context.symbols("x", "y")
        function = context.register_function("partial_absolute", 2, body=lambda first, second: first.abs()+second**2)
        unresolved = function(x, y).derivative(x)
        self.assertEqual(unresolved.kind, "derivative")
        self.assertEqual(unresolved.view.variable, x)
        self.assertEqual(function(x, y).derivative(y), 2*y)
        self.assertEqual(function(x, y).evaluate({x: -3, y: 2}), 7)
        unknown = context.register_function("unknown", 1)
        with self.assertRaisesRegex(ValueError, "unresolved differential"):
            context.register_function("invalid_body", 1, body=lambda value: unknown(value).derivative(value))
        self.assertIsNone(context.get_function("invalid_body"))
        with self.assertRaisesRegex(ValueError, "unresolved differential"):
            context.register_function("invalid_partial", 1, partials={0: lambda value: unknown(value).derivative(value)})
        self.assertIsNone(context.get_function("invalid_partial"))
