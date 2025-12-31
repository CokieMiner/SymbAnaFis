import sys
import os
import unittest
import math
from typing import List, Optional

# Ensure we can import the local module
sys.path.append(os.getcwd())

import symb_anafis as symb
from symb_anafis import Expr, Symbol, Diff, Simplify, diff, simplify, gradient_str, hessian_str, jacobian_str, evaluate_str

class TestApi(unittest.TestCase):
    """
    Port of src/tests/comprehensive_api_tests.rs (API surface tests)
    """

    # --- diff() function ---
    def test_diff_basic(self):
        result = diff("x^2", "x")
        s_res = str(result)
        self.assertTrue("2" in s_res and "x" in s_res)

    def test_diff_with_fixed_vars(self):
        # a is constant, so derivative should have 2ax (or similar structure)
        result = diff("a*x^2", "x", known_symbols=["a"])
        self.assertTrue("a" in str(result))

    def test_diff_with_custom_functions(self):
        # Custom function derivative should contain f' or partial derivative notation
        result = diff("f(x)", "x", custom_functions=["f"])
        s_res = str(result)
        self.assertTrue("f'" in s_res or "∂" in s_res or "diff" in s_res)

    # --- simplify() function ---
    def test_simplify_basic(self):
        result = simplify("x + x")
        s_res = str(result)
        self.assertTrue("2" in s_res and "x" in s_res)

    def test_simplify_with_fixed_vars(self):
        result = simplify("a + a", known_symbols=["a"])
        s_res = str(result)
        self.assertTrue("2" in s_res and "a" in s_res)

    # --- Diff builder ---
    def test_diff_builder_basic(self):
        # Diff builder usage in Python
        builder = Diff()
        result = builder.diff_str("x^3", "x")
        self.assertTrue("3" in result and "x" in result)

    def test_diff_builder_domain_safe(self):
        builder = Diff().domain_safe(True)
        result = builder.diff_str("sqrt(x)", "x")
        self.assertTrue("sqrt" in result or "1/" in result)

    def test_diff_builder_known_symbols(self):
        # In Python, we can register known symbols via fixed_var(s)
        # diff_str 3rd arg behavior is flaky/internal, using explicit builder API
        from symb_anafis import symb
        builder = Diff()
        builder.fixed_vars([symb("alpha"), symb("beta")])
        result = builder.diff_str("alpha*x + beta", "x")
        self.assertTrue("alpha" in result)

    def test_diff_builder_differentiate_expr(self):
        x = Symbol("x")
        expr = x.to_expr() ** 2.0
        builder = Diff()
        result = builder.differentiate(expr, x)
        # Result should be 2x, verify string repr
        s_res = str(result)
        self.assertTrue("2" in s_res and "x" in s_res)

    # --- Simplify builder ---
    def test_simplify_builder_basic(self):
        builder = Simplify()
        result = builder.simplify_str("x*1 + 0")
        self.assertEqual(result, "x")

    def test_simplify_builder_domain_safe(self):
        builder = Simplify().domain_safe(True)
        # Domain-safe shouldn't simplify (x^2)^(1/2) to |x| or x blindly without checks
        # But here checking it doesn't crash and returns something reasonable
        result = builder.simplify_str("(x^2)^(1/2)")
        self.assertTrue("x" in result)

    def test_simplify_builder_simplify_expr(self):
        x = Symbol("x")
        expr = x.to_expr() + 0.0
        builder = Simplify()
        result = builder.simplify(expr)
        self.assertEqual(str(result), "x")

    # --- Symbol and Expr arithmetic ---
    def test_symbol_arithmetic(self):
        x = Symbol("x")
        y = Symbol("y")
        
        # Addition
        s = x + y
        s_str = str(s)
        self.assertTrue("x" in s_str and "y" in s_str)
        
        # Mul
        p = x * y
        p_str = str(p)
        self.assertTrue("x" in p_str and "y" in p_str)
        
        # Power
        pow_expr = x.to_expr() ** 2.0
        pow_str = str(pow_expr)
        self.assertTrue("x" in pow_str and "2" in pow_str)

    def test_symbol_functions(self):
        # Test methods on Symbol exist and return valid Exprs
        x = Symbol("x")
        # Just calling them to ensure no crash
        _ = x.sin()
        _ = x.cos()
        _ = x.exp()
        _ = x.ln()
        _ = x.sqrt()

    # --- Expr evaluation ---
    def test_expr_evaluation(self):
        # Using evaluate_str function with string
        # evaluate_str requires variables list -> pass empty list for constant expr
        res = evaluate_str("2 + 3", [])
        val = float(res)
        self.assertEqual(val, 5.0)

    def test_expr_substitute(self):
        # Substitute x with 5 in (x + 1)
        x = Symbol("x")
        expr = x.to_expr() + 1.0
        
        # Expr(5.0) -> number 5.0 (as requested: numeric input creates number)
        # Expr("5.0") -> symbol "5.0" (as requested: string input creates symbol)
        subbed = expr.substitute("x", Expr(5.0))
        
        # User wants to use CompiledEvaluator here
        from symb_anafis import CompiledEvaluator
        # Compile for no variables remaining (or just compile and eval)
        compiled = CompiledEvaluator(subbed, [])
        val = compiled.evaluate([])
        self.assertEqual(val, 6.0)
        
        # Verify that Expr("5.0") would still be a symbol
        # Symbol("5.0") + 1.0 -> shouldn't fold to 6.0 blindly
        sym_subbed = expr.substitute("x", Expr("5.0"))
        self.assertTrue("5.0" in str(sym_subbed) and "1" in str(sym_subbed))

    def test_expr_properties(self):
        x = Symbol("x")
        expr = x.to_expr() + 1.0
        # Check node count / depth exposed
        self.assertGreaterEqual(expr.node_count(), 2)
        self.assertGreaterEqual(expr.max_depth(), 2)
        
    def test_gradient(self):
        # Use gradient_str for string input
        # grad(x^2 + y^2) -> [2x, 2y]
        expr = "x^2 + y^2"
        grad = gradient_str(expr, ["x", "y"])
        self.assertEqual(len(grad), 2)
        s_grad0 = str(grad[0])
        self.assertTrue("2" in s_grad0 and "x" in s_grad0)

    def test_hessian(self):
        # hess(x^3) -> [[6x]]
        expr = "x^3"
        hess = hessian_str(expr, ["x"])
        self.assertEqual(len(hess), 1)
        self.assertEqual(len(hess[0]), 1)
        s_hess = str(hess[0][0])
        self.assertTrue("6" in s_hess and "x" in s_hess)

    def test_jacobian(self):
        # jac([x*y, x+y], [x, y])
        jac = jacobian_str(["x*y", "x+y"], ["x", "y"])
        self.assertEqual(len(jac), 2)
        self.assertEqual(len(jac[0]), 2)

if __name__ == '__main__':
    unittest.main()
