#!/usr/bin/env python3
"""
Benchmark SymPy for comparison with symb_anafis.
Run: python3 benches/sympy_benchmark.py
"""

import time
from sympy import symbols, sin, cos, tan, exp, ln, diff, simplify, trigsimp
from sympy.parsing.sympy_parser import parse_expr

x, y = symbols('x y')

def benchmark(name, func, iterations=1000):
    """Run a benchmark and return average time in microseconds."""
    # Warmup
    for _ in range(10):
        func()
    
    start = time.perf_counter()
    for _ in range(iterations):
        func()
    elapsed = time.perf_counter() - start
    
    avg_us = (elapsed / iterations) * 1_000_000
    print(f"{name:45} {avg_us:10.2f} µs")
    return avg_us

print("=" * 60)
print("SymPy Benchmark (for comparison with symb_anafis)")
print("=" * 60)
print()

print("DIFFERENTIATION (with simplify)")
print("-" * 60)

# Simple polynomial
benchmark("poly_x^3+2x^2+x", lambda: simplify(diff(x**3 + 2*x**2 + x, x)))

# Trigonometric
benchmark("trig_sin(x)*cos(x)", lambda: simplify(diff(sin(x) * cos(x), x)))

# Chain rule
benchmark("chain_sin(x^2)", lambda: simplify(diff(sin(x**2), x)))

# Exponential
benchmark("exp_e^(x^2)", lambda: simplify(diff(exp(x**2), x)))

# Complex expression
benchmark("complex_x^2*sin(x)*exp(x)", lambda: simplify(diff(x**2 * sin(x) * exp(x), x)))

# Quotient rule
benchmark("quotient_(x^2+1)/(x-1)", lambda: simplify(diff((x**2 + 1) / (x - 1), x)))

# Nested functions
benchmark("nested_sin(cos(tan(x)))", lambda: simplify(diff(sin(cos(tan(x))), x)))

# Power rule with variable exponent
benchmark("power_x^x", lambda: simplify(diff(x**x, x)))

print()
print("SIMPLIFICATION")
print("-" * 60)

# Pythagorean identity
benchmark("pythagorean_sin^2+cos^2", lambda: trigsimp(sin(x)**2 + cos(x)**2))

# Perfect square (factor)
from sympy import factor
benchmark("perfect_square_x^2+2x+1", lambda: factor(x**2 + 2*x + 1))

# Fraction cancellation
benchmark("fraction_(x+1)^2/(x+1)", lambda: simplify((x + 1)**2 / (x + 1)))

# Exponential combination
benchmark("exp_combine_e^x*e^y", lambda: simplify(exp(x) * exp(y)))

# Like terms
benchmark("like_terms_2x+3x+x", lambda: simplify(2*x + 3*x + x))

# Complex fraction addition
benchmark("frac_add_(x^2+1)/(x^2-1)+1/(x+1)", 
          lambda: simplify((x**2 + 1)/(x**2 - 1) + 1/(x + 1)))

# Hyperbolic from exponential
from sympy import sinh
benchmark("hyp_sinh_(e^x-e^-x)/2", lambda: simplify((exp(x) - exp(-x)) / 2))

# Power simplification
benchmark("power_x^2*x^3/x", lambda: simplify(x**2 * x**3 / x))

print()
print("DIFF + SIMPLIFY")
print("-" * 60)

benchmark("d/dx[sin(x)^2]_simplified", 
          lambda: simplify(diff(sin(x)**2, x)))

benchmark("d/dx[(x^2+1)/(x-1)]_simplified", 
          lambda: simplify(diff((x**2 + 1) / (x - 1), x)))

print()
print("=" * 60)
print("Note: Run 'cargo bench' for symb_anafis timings")
print("=" * 60)
