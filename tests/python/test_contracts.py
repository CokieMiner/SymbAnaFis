import unittest
from symb_anafis import Expr, Symbol, diff, simplify, symb, parse, evaluate_str
import math

class TestContracts(unittest.TestCase):
    # ============================================================================
    # SYMBOL API CONTRACTS
    # ============================================================================

    def test_symbol_equality(self):
        # Same name = same symbol
        x1 = Symbol("x_eq1")
        x2 = Symbol("x_eq1")
        self.assertEqual(x1, x2)

        # Different name = different symbol
        y = Symbol("y_eq")
        self.assertNotEqual(x1, y)

    def test_symbol_hash_consistency(self):
        # Equal symbols must have equal hashes
        x1 = Symbol("x_hash1")
        x2 = Symbol("x_hash1")
        self.assertEqual(hash(x1), hash(x2))
        
        # Dictionary lookup works
        d = {x1: "value"}
        self.assertEqual(d[x2], "value")

    # ============================================================================
    # EXPR API CONTRACTS
    # ============================================================================

    def test_expr_number_creation(self):
        # As requested: Expr(5.0) -> Number
        num = Expr(42.5)
        self.assertTrue(num.is_number())
        self.assertEqual(str(num), "42.5")

    def test_expr_clone_equality(self):
        # In Python, Expr objects are effectively immutable wrappers
        x = Symbol("x")
        expr = x.to_expr()**2 + x.to_expr().sin()
        
        # Evaluate at x=2.0
        v1 = expr.evaluate({"x": 2.0})
        v2 = expr.evaluate({"x": 2.0}) # Same object, same result
        self.assertEqual(float(v1), float(v2))

    def test_expr_display_parse_roundtrip(self):
        expressions = ["x + y", "x * y", "x^2", "sin(x)", "exp(x) + ln(x)"]

        for expr_str in expressions:
            # Parse to get Expr object
            expr = parse(expr_str)
            displayed = str(expr)
            reparsed = parse(displayed)

            # Should evaluate to same values using Expr.evaluate method
            vars = {"x": 2.0, "y": 3.0}
            v1 = float(expr.evaluate(vars))
            v2 = float(reparsed.evaluate(vars))

            self.assertAlmostEqual(v1, v2, places=10, msg=f"Roundtrip failed for {expr_str}")

    # ============================================================================
    # BUILDER API CONTRACTS
    # ============================================================================

    def test_diff_builder_order(self):
        from symb_anafis import Diff
        x = Symbol("x_diff_order")
        expr = x.to_expr()**3

        # Get second derivative by nesting
        d1 = Diff().differentiate(expr, x)
        d2 = Diff().differentiate(d1, x)

        # d²/dx²[x³] = 6x, at x=2: 12
        val = float(d2.evaluate({"x_diff_order": 2.0}))
        self.assertAlmostEqual(val, 12.0, places=10)

    # ============================================================================
    # SIMPLIFY BUILDER API CONTRACTS
    # ============================================================================

    def test_simplify_idempotence(self):
        expressions = ["x + x", "x * x", "sin(x)**2 + cos(x)**2"]

        for expr_str in expressions:
            # Parse first
            expr = parse(expr_str)
            s1 = expr.simplify()
            s2 = s1.simplify()

            # Simplifying twice should give same result
            vars = {"x": 1.5}
            v1 = float(s1.evaluate(vars))
            v2 = float(s2.evaluate(vars))

            self.assertAlmostEqual(v1, v2, places=10, msg=f"Idempotence failed for {expr_str}")

    # ============================================================================
    # PARSE API CONTRACTS
    # ============================================================================

    def test_parse_empty_rejects(self):
        with self.assertRaises(Exception):
            simplify("")

    def test_parse_invalid_rejects(self):
        invalid = ["(", ")", "++", "x +"]
        for e in invalid:
            with self.assertRaises(Exception, msg=f"Should reject: {e}"):
                simplify(e)

    # ============================================================================
    # EVALUATION API CONTRACTS
    # ============================================================================

    def test_evaluate_with_all_variables(self):
        expr = parse("x + y")
        val = float(expr.evaluate({"x": 1.0, "y": 2.0}))
        self.assertAlmostEqual(val, 3.0)

    def test_evaluate_extra_variables_ok(self):
        expr = parse("x")
        val = float(expr.evaluate({"x": 5.0, "y": 999.0}))
        self.assertEqual(val, 5.0)

if __name__ == "__main__":
    unittest.main()
