#!/usr/bin/env python3
"""
Evaluation benchmark comparing SymbAnaFis, SymPy, and Symbolica.
Outputs timing AND results for precision verification.
"""

import timeit
import sympy
from sympy import symbols, sympify, sin, cos, tan, exp, log, sqrt
from sympy import gamma, digamma, polygamma, zeta, besselj, bessely, erf, erfc
from sympy import LambertW, diff
import math

x = symbols('x')
TEST_VALUE = 2.5

def run_bench(name, stmt, setup, number=1000):
    t = timeit.Timer(stmt, setup)
    
    try:
        time_taken = t.timeit(number=number)
        per_iter = (time_taken / number) * 1e6 # microseconds
        print(f"{name:<40} {per_iter:>10.2f} us/iter")
    except Exception as e:
        print(f"{name:<40} FAILED: {e}")

def bench_and_eval(name, sympy_expr, sympy_value, number=1000):
    """Benchmark and evaluate a SymPy expression."""
    setup = f"""
from sympy import symbols, sin, cos, tan, exp, log, sqrt
from sympy import gamma, digamma, polygamma, zeta, besselj, bessely, erf, erfc
from sympy import LambertW, diff
x = symbols('x')
expr = {sympy_expr}
"""
    stmt = f"expr.evalf(subs={{x: {sympy_value}}})"
    
    try:
        t = timeit.Timer(stmt, setup)
        time_taken = t.timeit(number=number)
        per_iter = (time_taken / number) * 1e6
        
        expr = eval(sympy_expr)
        result = float(expr.evalf(subs={x: sympy_value}))
        
        print(f"{name:<40} {per_iter:>10.2f} µs  result = {result:.10f}")
        return result
    except Exception as e:
        print(f"{name:<40} FAILED: {e}")
        return None

print("=" * 80)
print("SYMPY EVALUATION BENCHMARKS")
print(f"Test value: x = {TEST_VALUE}")
print("=" * 80)

# ==============================================================================
# 1. Parsing Benchmarks (String -> Expression)
# ==============================================================================
print(f"\n{'='*30} PARSING {'='*30}")
run_bench("parse_poly_x^3+2x^2+x", 
          "sympify('x**3 + 2*x**2 + x')", 
          "from sympy import sympify", number=1000)

run_bench("parse_trig_sin(x)*cos(x)", 
          "sympify('sin(x) * cos(x)')", 
          "from sympy import sympify", number=1000)

run_bench("parse_complex_x^2*sin(x)*exp(x)", 
          "sympify('x**2 * sin(x) * exp(x)')", 
          "from sympy import sympify", number=1000)

run_bench("parse_nested_sin(cos(tan(x)))", 
          "sympify('sin(cos(tan(x)))')", 
          "from sympy import sympify", number=1000)


# ==============================================================================
# 2. Differentiation (Expression -> Expression)
# ==============================================================================
print(f"\n{'='*30} DIFFERENTIATION (AST) {'='*30}")

setup_ast = """
from sympy import symbols, sin, cos, tan, exp, diff
x, y = symbols('x y')
poly = x**3 + 2*x**2 + x
trig = sin(x) * cos(x)
complex_expr = x**2 * sin(x) * exp(x)
nested = sin(cos(tan(x)))
"""

run_bench("diff_ast_poly", "diff(poly, x)", setup_ast, number=1000)
run_bench("diff_ast_trig", "diff(trig, x)", setup_ast, number=1000)
run_bench("diff_ast_complex", "diff(complex_expr, x)", setup_ast, number=1000)
run_bench("diff_ast_nested", "diff(nested, x)", setup_ast, number=1000)


# ==============================================================================
# 3. Differentiation (String -> Expression)
# ==============================================================================
print(f"\n{'='*30} DIFFERENTIATION (FULL) {'='*30}")

setup_full = "from sympy import sympify, diff, Symbol; x = Symbol('x')"

run_bench("poly_x^3+2x^2+x", 
          "diff(sympify('x**3 + 2*x**2 + x'), x)", 
          setup_full, number=1000)

run_bench("trig_sin(x)*cos(x)", 
          "diff(sympify('sin(x) * cos(x)'), x)", 
          setup_full, number=1000)

run_bench("chain_sin(x^2)", 
          "diff(sympify('sin(x**2)'), x)", 
          setup_full, number=1000)

run_bench("exp_e^(x^2)", 
          "diff(sympify('exp(x**2)'), x)", 
          setup_full, number=1000)

run_bench("complex_x^2*sin(x)*exp(x)", 
          "diff(sympify('x**2 * sin(x) * exp(x)'), x)", 
          setup_full, number=1000)

run_bench("quotient_(x^2+1)/(x-1)", 
          "diff(sympify('(x**2 + 1) / (x - 1)'), x)", 
          setup_full, number=1000)

run_bench("nested_sin(cos(tan(x)))", 
          "diff(sympify('sin(cos(tan(x)))'), x)", 
          setup_full, number=1000)

run_bench("power_x^x", 
          "diff(sympify('x**x'), x)", 
          setup_full, number=1000)


# ==============================================================================
# 4. Simplification (Full)
# ==============================================================================
print(f"\n{'='*30} SIMPLIFICATION (FULL) {'='*30}")

setup_simp_full = "from sympy import sympify, simplify"

run_bench("pythagorean_sin^2+cos^2", 
          "simplify(sympify('sin(x)**2 + cos(x)**2'))", 
          setup_simp_full, number=100)

run_bench("perfect_square_x^2+2x+1", 
          "simplify(sympify('x**2 + 2*x + 1'))", 
          setup_simp_full, number=100)

run_bench("fraction_(x+1)^2/(x+1)", 
          "simplify(sympify('(x + 1)**2 / (x + 1)'))", 
          setup_simp_full, number=100)

run_bench("exp_combine_e^x*e^y", 
          "simplify(sympify('exp(x) * exp(y)'))", 
          setup_simp_full, number=100)

run_bench("like_terms_2x+3x+x", 
          "simplify(sympify('2*x + 3*x + x'))", 
          setup_simp_full, number=100)


# ==============================================================================
# 5. EVALUATION - Common Functions (baseline)
# ==============================================================================
print(f"\n{'='*30} EVALUATION: COMMON FUNCTIONS {'='*30}")

bench_and_eval("polynomial x^3+2x^2+x+1", "x**3 + 2*x**2 + x + 1", TEST_VALUE)
bench_and_eval("sin(x)", "sin(x)", TEST_VALUE)
bench_and_eval("cos(x)", "cos(x)", TEST_VALUE)
bench_and_eval("tan(x)", "tan(x)", TEST_VALUE)
bench_and_eval("exp(x)", "exp(x)", TEST_VALUE)
bench_and_eval("ln(x)", "log(x)", TEST_VALUE)
bench_and_eval("sqrt(x)", "sqrt(x)", TEST_VALUE)
bench_and_eval("sin(x)*cos(x)", "sin(x)*cos(x)", TEST_VALUE)
bench_and_eval("exp(x)*sin(x)", "exp(x)*sin(x)", TEST_VALUE)
bench_and_eval("nested sin(cos(tan(x)))", "sin(cos(tan(x)))", TEST_VALUE)


# ==============================================================================
# 6. EVALUATION - Gamma family
# ==============================================================================
print(f"\n{'='*30} EVALUATION: GAMMA FAMILY {'='*30}")

bench_and_eval("gamma(x)", "gamma(x)", TEST_VALUE)
bench_and_eval("digamma(x)", "polygamma(0, x)", TEST_VALUE)
bench_and_eval("trigamma(x)", "polygamma(1, x)", TEST_VALUE)
bench_and_eval("polygamma(2, x)", "polygamma(2, x)", TEST_VALUE)
bench_and_eval("polygamma(3, x)", "polygamma(3, x)", TEST_VALUE)
bench_and_eval("polygamma(4, x)", "polygamma(4, x)", TEST_VALUE)


# ==============================================================================
# 7. EVALUATION - Bessel
# ==============================================================================
print(f"\n{'='*30} EVALUATION: BESSEL FUNCTIONS {'='*30}")

bench_and_eval("besselj(0, x)", "besselj(0, x)", TEST_VALUE)
bench_and_eval("besselj(1, x)", "besselj(1, x)", TEST_VALUE)
bench_and_eval("besselj(2, x)", "besselj(2, x)", TEST_VALUE)
bench_and_eval("bessely(0, x)", "bessely(0, x)", TEST_VALUE)
bench_and_eval("bessely(1, x)", "bessely(1, x)", TEST_VALUE)


# ==============================================================================
# 8. EVALUATION - Zeta and derivatives
# ==============================================================================
print(f"\n{'='*30} EVALUATION: ZETA FUNCTION {'='*30}")

bench_and_eval("zeta(x)", "zeta(x)", TEST_VALUE)

# SymPy zeta derivatives via diff
print("\n# Higher zeta derivatives (computed via symbolic diff):")
try:
    zeta1 = diff(zeta(x), x)
    zeta2 = diff(zeta1, x)
    zeta3 = diff(zeta2, x)
    
    setup_zeta = """
from sympy import symbols, zeta, diff
x = symbols('x')
zeta1 = diff(zeta(x), x)
zeta2 = diff(zeta1, x)
zeta3 = diff(zeta2, x)
"""
    
    t = timeit.Timer("zeta1.evalf(subs={x: 2.5})", setup_zeta)
    time1 = (t.timeit(100) / 100) * 1e6
    r1 = float(zeta1.evalf(subs={x: TEST_VALUE}))
    print(f"{'zeta_deriv(1, x)':<40} {time1:>10.2f} µs  result = {r1:.10f}")
    
    t = timeit.Timer("zeta2.evalf(subs={x: 2.5})", setup_zeta)
    time2 = (t.timeit(100) / 100) * 1e6
    r2 = float(zeta2.evalf(subs={x: TEST_VALUE}))
    print(f"{'zeta_deriv(2, x)':<40} {time2:>10.2f} µs  result = {r2:.10f}")
    
    t = timeit.Timer("zeta3.evalf(subs={x: 2.5})", setup_zeta)
    time3 = (t.timeit(100) / 100) * 1e6
    r3 = float(zeta3.evalf(subs={x: TEST_VALUE}))
    print(f"{'zeta_deriv(3, x)':<40} {time3:>10.2f} µs  result = {r3:.10f}")
except Exception as e:
    print(f"Zeta derivatives failed: {e}")


# ==============================================================================
# 9. EVALUATION - Error function
# ==============================================================================
print(f"\n{'='*30} EVALUATION: ERROR FUNCTIONS {'='*30}")

bench_and_eval("erf(x)", "erf(x)", TEST_VALUE)
bench_and_eval("erfc(x)", "erfc(x)", TEST_VALUE)


# ==============================================================================
# 10. EVALUATION - Lambert W
# ==============================================================================
print(f"\n{'='*30} EVALUATION: LAMBERT W {'='*30}")

bench_and_eval("lambertw(x)", "LambertW(x)", TEST_VALUE)


# ==============================================================================
# 11. EVALUATION - Complex combinations
# ==============================================================================
print(f"\n{'='*30} EVALUATION: COMPLEX COMBINATIONS {'='*30}")

bench_and_eval("gamma(x)*besselj(0,x)+erf(x)", 
               "gamma(x)*besselj(0,x)+erf(x)", TEST_VALUE)
bench_and_eval("polygamma(2,x)*zeta(x)", 
               "polygamma(2,x)*zeta(x)", TEST_VALUE)
# Note: SymPy cannot numerically evaluate zeta derivatives - SymbAnaFis has custom implementation!
print("zeta_deriv(3,x)*gamma(x)                 N/A (SymPy cannot evaluate zeta derivatives)")


# ==============================================================================
# 12. Combined (Diff + Simplify)
# ==============================================================================
print(f"\n{'='*30} COMBINED (DIFF + SIMPLIFY) {'='*30}")

setup_combined = "from sympy import sympify, diff, simplify, Symbol; x = Symbol('x')"

run_bench("d/dx[sin(x)^2]_simplified", 
          "simplify(diff(sympify('sin(x)**2'), x))", 
          setup_combined, number=100)

run_bench("d/dx[(x^2+1)/(x-1)]_simplified", 
          "simplify(diff(sympify('(x**2 + 1) / (x - 1)'), x))", 
          setup_combined, number=100)


print("\n" + "=" * 80)
print("Done! Compare these results with SymbAnaFis and Symbolica outputs.")
print("=" * 80)
