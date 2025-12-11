#!/usr/bin/env python3
"""
Evaluation benchmark comparing SymbAnaFis, SymPy, and Symbolica.
Outputs timing AND results for precision verification.
"""

import timeit
import sympy
from sympy import symbols, sin, cos, tan, exp, log, sqrt
from sympy import gamma, digamma, polygamma, zeta, besselj, bessely, erf, erfc
import math

x = symbols('x')
TEST_VALUE = 2.5

def bench_and_eval(name, sympy_expr, sympy_value, number=1000):
    """Benchmark and evaluate a SymPy expression."""
    # Time it
    setup = f"from sympy import *; x = symbols('x'); expr = {sympy_expr}"
    stmt = f"expr.evalf(subs={{x: {sympy_value}}})"
    
    try:
        t = timeit.Timer(stmt, setup)
        time_taken = t.timeit(number=number)
        per_iter = (time_taken / number) * 1e6  # microseconds
        
        # Get actual result
        expr = eval(sympy_expr)
        result = float(expr.evalf(subs={x: sympy_value}))
        
        print(f"{name:<35} {per_iter:>10.2f} µs  result = {result:.10f}")
        return result
    except Exception as e:
        print(f"{name:<35} FAILED: {e}")
        return None

print("=" * 80)
print("SYMPY EVALUATION BENCHMARKS")
print(f"Test value: x = {TEST_VALUE}")
print("=" * 80)

# ==============================================================================
# 1. Common Functions (baseline)
# ==============================================================================
print(f"\n{'='*30} COMMON FUNCTIONS {'='*30}")

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
# 2. Special Functions - Gamma family
# ==============================================================================
print(f"\n{'='*30} GAMMA FAMILY {'='*30}")

bench_and_eval("gamma(x)", "gamma(x)", TEST_VALUE)
bench_and_eval("digamma(x) = polygamma(0,x)", "polygamma(0, x)", TEST_VALUE)
bench_and_eval("trigamma(x) = polygamma(1,x)", "polygamma(1, x)", TEST_VALUE)
bench_and_eval("polygamma(2, x)", "polygamma(2, x)", TEST_VALUE)
bench_and_eval("polygamma(3, x)", "polygamma(3, x)", TEST_VALUE)
bench_and_eval("polygamma(4, x)", "polygamma(4, x)", TEST_VALUE)

# ==============================================================================
# 3. Special Functions - Bessel
# ==============================================================================
print(f"\n{'='*30} BESSEL FUNCTIONS {'='*30}")

bench_and_eval("besselj(0, x)", "besselj(0, x)", TEST_VALUE)
bench_and_eval("besselj(1, x)", "besselj(1, x)", TEST_VALUE)
bench_and_eval("besselj(2, x)", "besselj(2, x)", TEST_VALUE)
bench_and_eval("bessely(0, x)", "bessely(0, x)", TEST_VALUE)
bench_and_eval("bessely(1, x)", "bessely(1, x)", TEST_VALUE)

# ==============================================================================
# 4. Special Functions - Zeta and derivatives
# ==============================================================================
print(f"\n{'='*30} ZETA FUNCTION {'='*30}")

bench_and_eval("zeta(x)", "zeta(x)", TEST_VALUE)
# SymPy zeta derivatives via diff
bench_and_eval("zeta'(x) = diff(zeta,x)", "diff(zeta(x), x)", TEST_VALUE)

# For higher derivatives, we need to evaluate the diff result
setup_zeta = """
from sympy import symbols, zeta, diff
x = symbols('x')
zeta1 = diff(zeta(x), x)
zeta2 = diff(zeta1, x)
zeta3 = diff(zeta2, x)
"""
print("\n# Higher zeta derivatives (computed via symbolic diff):")
try:
    exec(setup_zeta)
    zeta1 = sympy.diff(zeta(x), x)
    zeta2 = sympy.diff(zeta1, x)
    zeta3 = sympy.diff(zeta2, x)
    
    t = timeit.Timer("zeta1.evalf(subs={x: 2.5})", setup_zeta + "x = symbols('x')")
    time1 = (t.timeit(100) / 100) * 1e6
    r1 = float(zeta1.evalf(subs={x: TEST_VALUE}))
    print(f"{'zeta_deriv(1, x)':<35} {time1:>10.2f} µs  result = {r1:.10f}")
    
    t = timeit.Timer("zeta2.evalf(subs={x: 2.5})", setup_zeta + "x = symbols('x')")
    time2 = (t.timeit(100) / 100) * 1e6
    r2 = float(zeta2.evalf(subs={x: TEST_VALUE}))
    print(f"{'zeta_deriv(2, x)':<35} {time2:>10.2f} µs  result = {r2:.10f}")
    
    t = timeit.Timer("zeta3.evalf(subs={x: 2.5})", setup_zeta + "x = symbols('x')")
    time3 = (t.timeit(100) / 100) * 1e6
    r3 = float(zeta3.evalf(subs={x: TEST_VALUE}))
    print(f"{'zeta_deriv(3, x)':<35} {time3:>10.2f} µs  result = {r3:.10f}")
except Exception as e:
    print(f"Zeta derivatives failed: {e}")

# ==============================================================================
# 5. Special Functions - Error function
# ==============================================================================
print(f"\n{'='*30} ERROR FUNCTIONS {'='*30}")

bench_and_eval("erf(x)", "erf(x)", TEST_VALUE)
bench_and_eval("erfc(x)", "erfc(x)", TEST_VALUE)

# ==============================================================================
# 6. Lambert W
# ==============================================================================
print(f"\n{'='*30} LAMBERT W {'='*30}")

from sympy import LambertW
bench_and_eval("lambertw(x)", "LambertW(x)", TEST_VALUE)

# ==============================================================================
# 7. Complex combinations
# ==============================================================================
print(f"\n{'='*30} COMPLEX COMBINATIONS {'='*30}")

bench_and_eval("gamma(x)*besselj(0,x)+erf(x)", 
               "gamma(x)*besselj(0,x)+erf(x)", TEST_VALUE)
bench_and_eval("polygamma(2,x)*zeta(x)", 
               "polygamma(2,x)*zeta(x)", TEST_VALUE)

print("\n" + "=" * 80)
print("Done! Compare these results with SymbAnaFis and Symbolica outputs.")
print("=" * 80)
