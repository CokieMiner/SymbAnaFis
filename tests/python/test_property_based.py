"""
Property-Based Tests - Basic fuzzing and invariant tests

Tests mathematical properties that should always hold:
- Commutativity, associativity
- Identity elements
- Derivative consistency
- Simplification stability
"""

import pytest
import math
import random
from symb_anafis import diff, simplify, evaluate_str, parse, Expr, Symbol

EPSILON = 1e-8


def approx_eq(a: float, b: float, eps: float = EPSILON) -> bool:
    """Check if two floats are approximately equal."""
    if math.isnan(a) and math.isnan(b):
        return True
    if math.isinf(a) and math.isinf(b):
        return (a > 0) == (b > 0)
    if abs(a) < eps and abs(b) < eps:
        return True
    return abs(a - b) < eps * max(abs(a), abs(b), 1.0)


class TestArithmeticProperties:
    """Test fundamental arithmetic properties."""

    def test_addition_commutativity(self):
        """a + b = b + a"""
        for _ in range(20):
            a, b = random.uniform(-100, 100), random.uniform(-100, 100)
            expr1 = f"{a} + {b}"
            expr2 = f"{b} + {a}"
            r1 = float(evaluate_str(expr1, []))
            r2 = float(evaluate_str(expr2, []))
            assert approx_eq(r1, r2), f"{expr1} != {expr2}"

    def test_multiplication_commutativity(self):
        """a * b = b * a"""
        for _ in range(20):
            a, b = random.uniform(-100, 100), random.uniform(-100, 100)
            expr1 = f"{a} * {b}"
            expr2 = f"{b} * {a}"
            r1 = float(evaluate_str(expr1, []))
            r2 = float(evaluate_str(expr2, []))
            assert approx_eq(r1, r2), f"{expr1} != {expr2}"

    def test_addition_identity(self):
        """a + 0 = a"""
        for _ in range(20):
            a = random.uniform(-1000, 1000)
            result = float(evaluate_str(f"{a} + 0", []))
            assert approx_eq(result, a), f"{a} + 0 != {a}"

    def test_multiplication_identity(self):
        """a * 1 = a"""
        for _ in range(20):
            a = random.uniform(-1000, 1000)
            result = float(evaluate_str(f"{a} * 1", []))
            assert approx_eq(result, a), f"{a} * 1 != {a}"

    def test_multiplication_by_zero(self):
        """a * 0 = 0"""
        for _ in range(20):
            a = random.uniform(-1000, 1000)
            result = float(evaluate_str(f"{a} * 0", []))
            assert approx_eq(result, 0.0), f"{a} * 0 != 0"

    def test_double_negation(self):
        """-(-a) = a"""
        for _ in range(20):
            a = random.uniform(-1000, 1000)
            result = float(evaluate_str(f"-(-{a})", []))
            assert approx_eq(result, a), f"-(-{a}) != {a}"


class TestDerivativeProperties:
    """Test derivative properties that should always hold."""

    def test_constant_derivative_is_zero(self):
        """d/dx[c] = 0 for any constant c"""
        for _ in range(10):
            c = random.uniform(-1000, 1000)
            result = diff(str(c), "x")
            val = float(evaluate_str(result, [("x", random.uniform(-10, 10))]))
            assert approx_eq(val, 0.0), f"d/dx[{c}] should be 0"

    def test_linear_derivative_is_coefficient(self):
        """d/dx[a*x] = a for any constant a"""
        for _ in range(10):
            a = random.uniform(-100, 100)
            result = diff(f"{a}*x", "x", known_symbols=[])
            x_val = random.uniform(-10, 10)
            val = float(evaluate_str(result, [("x", x_val)]))
            assert approx_eq(val, a), f"d/dx[{a}*x] should be {a}, got {val}"

    def test_sum_rule(self):
        """d/dx[f + g] = d/dx[f] + d/dx[g]"""
        # Test with f = x^2, g = x^3
        df = diff("x^2", "x")
        dg = diff("x^3", "x")
        d_sum = diff("x^2 + x^3", "x")
        
        for _ in range(10):
            x = random.uniform(0.1, 10)  # Positive to avoid issues
            df_val = float(evaluate_str(df, [("x", x)]))
            dg_val = float(evaluate_str(dg, [("x", x)]))
            d_sum_val = float(evaluate_str(d_sum, [("x", x)]))
            assert approx_eq(d_sum_val, df_val + dg_val), \
                f"Sum rule failed at x={x}"

    def test_product_rule(self):
        """d/dx[f*g] = f'*g + f*g'"""
        # f = x, g = sin(x)
        # f' = 1, g' = cos(x)
        # d/dx[x*sin(x)] = sin(x) + x*cos(x)
        d_prod = diff("x*sin(x)", "x")
        
        for _ in range(10):
            x = random.uniform(0.1, 5)
            expected = math.sin(x) + x * math.cos(x)
            actual = float(evaluate_str(d_prod, [("x", x)]))
            assert approx_eq(actual, expected, 1e-6), \
                f"Product rule failed at x={x}: got {actual}, expected {expected}"

    def test_chain_rule(self):
        """d/dx[f(g(x))] = f'(g(x)) * g'(x)"""
        # f = sin, g = x^2
        # d/dx[sin(x^2)] = cos(x^2) * 2x
        d_chain = diff("sin(x^2)", "x")
        
        for _ in range(10):
            x = random.uniform(0.1, 5)
            expected = math.cos(x**2) * 2 * x
            actual = float(evaluate_str(d_chain, [("x", x)]))
            assert approx_eq(actual, expected, 1e-6), \
                f"Chain rule failed at x={x}: got {actual}, expected {expected}"


class TestSimplificationProperties:
    """Test simplification stability and correctness."""

    def test_simplify_idempotent(self):
        """simplify(simplify(e)) = simplify(e)"""
        expressions = [
            "x + x",
            "x * 1",
            "x + 0",
            "x^2 / x",
            "sin(x)^2 + cos(x)^2",
        ]
        for expr in expressions:
            s1 = simplify(expr)
            s2 = simplify(s1)
            assert s1 == s2, f"Simplify not idempotent for {expr}: '{s1}' != '{s2}'"

    def test_simplify_preserves_value(self):
        """Simplification should preserve numerical values."""
        x_val = 2.0
        test_cases = [
            ("x + x", 4.0),   # 2 + 2 = 4
            ("x * x", 4.0),   # 2 * 2 = 4
            ("x + 0", 2.0),   # 2 + 0 = 2
            ("x * 1", 2.0),   # 2 * 1 = 2
        ]
        for expr, expected in test_cases:
            simplified = simplify(expr)
            orig_val = float(evaluate_str(expr, [("x", x_val)]))
            simp_val = float(evaluate_str(simplified, [("x", x_val)]))
            assert approx_eq(orig_val, expected), f"Original {expr} != {expected}"
            assert approx_eq(simp_val, expected), f"Simplified {simplified} != {expected}"


class TestTrigIdentities:
    """Test trigonometric identities."""

    def test_pythagorean_identity(self):
        """sin²(x) + cos²(x) = 1"""
        for _ in range(20):
            x = random.uniform(-10, 10)
            expr = "sin(x)^2 + cos(x)^2"
            result = float(evaluate_str(expr, [("x", x)]))
            assert approx_eq(result, 1.0), \
                f"sin²({x}) + cos²({x}) = {result}, expected 1"

    def test_double_angle_sin(self):
        """sin(2x) = 2*sin(x)*cos(x)"""
        for _ in range(20):
            x = random.uniform(-5, 5)
            lhs = float(evaluate_str("sin(2*x)", [("x", x)]))
            rhs = float(evaluate_str("2*sin(x)*cos(x)", [("x", x)]))
            assert approx_eq(lhs, rhs), \
                f"sin(2*{x}) = {lhs}, 2*sin({x})*cos({x}) = {rhs}"

    def test_tan_identity(self):
        """tan(x) = sin(x)/cos(x)"""
        for _ in range(20):
            x = random.uniform(-1.5, 1.5)  # Avoid cos(x) = 0
            lhs = float(evaluate_str("tan(x)", [("x", x)]))
            rhs = float(evaluate_str("sin(x)/cos(x)", [("x", x)]))
            assert approx_eq(lhs, rhs), \
                f"tan({x}) = {lhs}, sin({x})/cos({x}) = {rhs}"


class TestExpLogIdentities:
    """Test exponential/logarithmic identities."""

    def test_exp_ln_inverse(self):
        """exp(ln(x)) = x for x > 0"""
        for _ in range(20):
            x = random.uniform(0.1, 100)
            result = float(evaluate_str("exp(ln(x))", [("x", x)]))
            assert approx_eq(result, x), f"exp(ln({x})) = {result}, expected {x}"

    def test_ln_exp_inverse(self):
        """ln(exp(x)) = x"""
        for _ in range(20):
            x = random.uniform(-10, 10)
            result = float(evaluate_str("ln(exp(x))", [("x", x)]))
            assert approx_eq(result, x), f"ln(exp({x})) = {result}, expected {x}"

    def test_log_product(self):
        """ln(a*b) = ln(a) + ln(b) for a,b > 0"""
        for _ in range(20):
            a, b = random.uniform(0.1, 100), random.uniform(0.1, 100)
            lhs = float(evaluate_str(f"ln({a}*{b})", []))
            rhs = float(evaluate_str(f"ln({a}) + ln({b})", []))
            assert approx_eq(lhs, rhs), \
                f"ln({a}*{b}) = {lhs}, ln({a}) + ln({b}) = {rhs}"

    def test_log_power(self):
        """ln(x^n) = n*ln(x) for x > 0"""
        for _ in range(20):
            x = random.uniform(0.1, 10)
            n = random.uniform(0.5, 5)
            lhs = float(evaluate_str(f"ln(x^{n})", [("x", x)]))
            rhs = float(evaluate_str(f"{n}*ln(x)", [("x", x)]))
            assert approx_eq(lhs, rhs), \
                f"ln({x}^{n}) = {lhs}, {n}*ln({x}) = {rhs}"


class TestHyperbolicIdentities:
    """Test hyperbolic function identities."""

    def test_hyperbolic_pythagorean(self):
        """cosh²(x) - sinh²(x) = 1"""
        for _ in range(20):
            x = random.uniform(-5, 5)
            result = float(evaluate_str("cosh(x)^2 - sinh(x)^2", [("x", x)]))
            assert approx_eq(result, 1.0), \
                f"cosh²({x}) - sinh²({x}) = {result}, expected 1"

    def test_tanh_identity(self):
        """tanh(x) = sinh(x)/cosh(x)"""
        for _ in range(20):
            x = random.uniform(-5, 5)
            lhs = float(evaluate_str("tanh(x)", [("x", x)]))
            rhs = float(evaluate_str("sinh(x)/cosh(x)", [("x", x)]))
            assert approx_eq(lhs, rhs), \
                f"tanh({x}) = {lhs}, sinh({x})/cosh({x}) = {rhs}"


class TestPowerProperties:
    """Test power law identities."""

    def test_power_addition_rule(self):
        """x^a * x^b = x^(a+b) for x > 0"""
        for _ in range(20):
            x = random.uniform(0.1, 10)
            a = random.uniform(0.5, 3)
            b = random.uniform(0.5, 3)
            lhs = float(evaluate_str(f"x^{a} * x^{b}", [("x", x)]))
            rhs = float(evaluate_str(f"x^({a}+{b})", [("x", x)]))
            assert approx_eq(lhs, rhs), \
                f"x^{a} * x^{b} = {lhs}, x^({a}+{b}) = {rhs} at x={x}"

    def test_power_of_power_rule(self):
        """(x^a)^b = x^(a*b) for x > 0"""
        for _ in range(20):
            x = random.uniform(0.5, 5)
            a = random.uniform(0.5, 2)
            b = random.uniform(0.5, 2)
            lhs = float(evaluate_str(f"(x^{a})^{b}", [("x", x)]))
            rhs = float(evaluate_str(f"x^({a}*{b})", [("x", x)]))
            assert approx_eq(lhs, rhs), \
                f"(x^{a})^{b} = {lhs}, x^({a}*{b}) = {rhs} at x={x}"

    def test_product_power_rule(self):
        """(x*y)^a = x^a * y^a for x,y > 0"""
        for _ in range(20):
            x = random.uniform(0.5, 5)
            y = random.uniform(0.5, 5)
            a = random.uniform(0.5, 3)
            lhs = float(evaluate_str(f"(x*y)^{a}", [("x", x), ("y", y)]))
            rhs = float(evaluate_str(f"x^{a} * y^{a}", [("x", x), ("y", y)]))
            assert approx_eq(lhs, rhs), \
                f"(x*y)^{a} = {lhs}, x^{a}*y^{a} = {rhs}"

    def test_quotient_power_rule(self):
        """(x/y)^a = x^a / y^a for x,y > 0"""
        for _ in range(20):
            x = random.uniform(0.5, 5)
            y = random.uniform(0.5, 5)
            a = random.uniform(0.5, 3)
            lhs = float(evaluate_str(f"(x/y)^{a}", [("x", x), ("y", y)]))
            rhs = float(evaluate_str(f"x^{a} / y^{a}", [("x", x), ("y", y)]))
            assert approx_eq(lhs, rhs), \
                f"(x/y)^{a} = {lhs}, x^{a}/y^{a} = {rhs}"


class TestAssociativeProperties:
    """Test associativity of operations."""

    def test_addition_associativity(self):
        """(a + b) + c = a + (b + c)"""
        for _ in range(20):
            a, b, c = [random.uniform(-100, 100) for _ in range(3)]
            lhs = float(evaluate_str(f"({a} + {b}) + {c}", []))
            rhs = float(evaluate_str(f"{a} + ({b} + {c})", []))
            assert approx_eq(lhs, rhs), \
                f"({a}+{b})+{c} = {lhs}, {a}+({b}+{c}) = {rhs}"

    def test_multiplication_associativity(self):
        """(a * b) * c = a * (b * c)"""
        for _ in range(20):
            a, b, c = [random.uniform(-10, 10) for _ in range(3)]
            lhs = float(evaluate_str(f"({a} * {b}) * {c}", []))
            rhs = float(evaluate_str(f"{a} * ({b} * {c})", []))
            assert approx_eq(lhs, rhs), \
                f"({a}*{b})*{c} = {lhs}, {a}*({b}*{c}) = {rhs}"


class TestDistributiveProperties:
    """Test distributive law."""

    def test_left_distributivity(self):
        """a * (b + c) = a*b + a*c"""
        for _ in range(20):
            a, b, c = [random.uniform(-10, 10) for _ in range(3)]
            lhs = float(evaluate_str(f"{a} * ({b} + {c})", []))
            rhs = float(evaluate_str(f"{a}*{b} + {a}*{c}", []))
            assert approx_eq(lhs, rhs), \
                f"{a}*({b}+{c}) = {lhs}, {a}*{b}+{a}*{c} = {rhs}"

    def test_right_distributivity(self):
        """(a + b) * c = a*c + b*c"""
        for _ in range(20):
            a, b, c = [random.uniform(-10, 10) for _ in range(3)]
            lhs = float(evaluate_str(f"({a} + {b}) * {c}", []))
            rhs = float(evaluate_str(f"{a}*{c} + {b}*{c}", []))
            assert approx_eq(lhs, rhs), \
                f"({a}+{b})*{c} = {lhs}, {a}*{c}+{b}*{c} = {rhs}"


class TestSpecialFunctionProperties:
    """Test special function properties."""

    def test_sinh_definition(self):
        """sinh(x) = (exp(x) - exp(-x))/2"""
        for _ in range(20):
            x = random.uniform(-5, 5)
            lhs = float(evaluate_str("sinh(x)", [("x", x)]))
            rhs = float(evaluate_str("(exp(x) - exp(-x))/2", [("x", x)]))
            assert approx_eq(lhs, rhs), \
                f"sinh({x}) = {lhs}, (exp(x)-exp(-x))/2 = {rhs}"

    def test_cosh_definition(self):
        """cosh(x) = (exp(x) + exp(-x))/2"""
        for _ in range(20):
            x = random.uniform(-5, 5)
            lhs = float(evaluate_str("cosh(x)", [("x", x)]))
            rhs = float(evaluate_str("(exp(x) + exp(-x))/2", [("x", x)]))
            assert approx_eq(lhs, rhs), \
                f"cosh({x}) = {lhs}, (exp(x)+exp(-x))/2 = {rhs}"

    def test_abs_symmetry(self):
        """abs(-x) = abs(x)"""
        for _ in range(20):
            x = random.uniform(-100, 100)
            lhs = float(evaluate_str("abs(-x)", [("x", x)]))
            rhs = float(evaluate_str("abs(x)", [("x", x)]))
            assert approx_eq(lhs, rhs), \
                f"abs(-{x}) = {lhs}, abs({x}) = {rhs}"

    def test_abs_nonnegative(self):
        """abs(x) >= 0 for all x"""
        for _ in range(20):
            x = random.uniform(-100, 100)
            result = float(evaluate_str("abs(x)", [("x", x)]))
            assert result >= 0, f"abs({x}) = {result} < 0"

    def test_sqrt_square_identity(self):
        """sqrt(x)^2 = x for x >= 0"""
        for _ in range(20):
            x = random.uniform(0.01, 100)
            result = float(evaluate_str("sqrt(x)^2", [("x", x)]))
            assert approx_eq(result, x), \
                f"sqrt({x})^2 = {result}, expected {x}"

    def test_floor_ceil_relationship(self):
        """floor(x) <= x <= ceil(x)"""
        for _ in range(20):
            x = random.uniform(-100, 100)
            floor_x = float(evaluate_str("floor(x)", [("x", x)]))
            ceil_x = float(evaluate_str("ceil(x)", [("x", x)]))
            assert floor_x <= x <= ceil_x, \
                f"floor({x})={floor_x}, ceil({x})={ceil_x}"


class TestVmEvaluatorProperties:
    """Test VmEvaluator consistency properties."""

    def test_compiled_matches_string_eval(self):
        """Compiled evaluation should match string evaluation."""
        from symb_anafis import VmEvaluator, parse

        expressions = ["x^2", "sin(x)", "x + 2*x", "exp(x)", "ln(x)"]
        for expr_str in expressions:
            expr = parse(expr_str)
            compiled = VmEvaluator(expr, ["x"])
            for _ in range(10):
                x = random.uniform(0.1, 10)
                compiled_val = compiled.evaluate([x])
                string_val = float(evaluate_str(expr_str, [("x", x)]))
                assert approx_eq(compiled_val, string_val), \
                    f"Compiled vs string mismatch for {expr_str} at x={x}"

    def test_compiled_deterministic(self):
        """Multiple evaluations with same input should give same result."""
        from symb_anafis import VmEvaluator, parse

        expr = parse("sin(x) + cos(x)")
        compiled = VmEvaluator(expr, ["x"])
        for _ in range(10):
            x = random.uniform(-10, 10)
            results = [compiled.evaluate([x]) for _ in range(5)]
            assert all(approx_eq(r, results[0]) for r in results), \
                f"Non-deterministic results for x={x}: {results}"

    def test_compiled_batch_vs_single(self):
        """Batch evaluation should match single evaluations."""
        from symb_anafis import VmEvaluator, parse
        import numpy as np

        expr = parse("x^2 + 2*x + 1")
        compiled = VmEvaluator(expr, ["x"])
        x_values = np.array([random.uniform(-10, 10) for _ in range(20)])

        # eval_batch expects columnar data: one array per variable
        batch_results = compiled.eval_batch([x_values])
        single_results = [compiled.evaluate([x]) for x in x_values]

        for i, (batch_val, single_val) in enumerate(zip(batch_results, single_results)):
            assert approx_eq(batch_val, single_val), \
                f"Batch vs single mismatch at index {i}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
