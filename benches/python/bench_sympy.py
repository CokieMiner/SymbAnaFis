#!/usr/bin/env python3
"""
SymPy Benchmark Suite

Benchmarks for SymPy parsing, differentiation, simplification, and evaluation.
Uses timeit for accurate timing measurements.
"""

import timeit
from typing import Callable
import sympy as sp
from sympy import symbols, sympify, sin, cos, tan, exp, ln, sqrt, pi, gamma, digamma

# =============================================================================
# Configuration
# =============================================================================

WARMUP_RUNS = 10
BENCH_RUNS = 1000

# =============================================================================
# Test Expressions
# =============================================================================

POLYNOMIAL = "x**3 + 2*x**2 + x + 1"
TRIG_SIMPLE = "sin(x) * cos(x)"
COMPLEX_EXPR = "x**2 * sin(x) * exp(x)"
NESTED_TRIG = "sin(cos(tan(x)))"
CHAIN_SIN = "sin(x**2)"
EXP_SQUARED = "exp(x**2)"
QUOTIENT = "(x**2 + 1) / (x - 1)"
POWER_XX = "x**x"

# Simplification expressions
PYTHAGOREAN = "sin(x)**2 + cos(x)**2"
PERFECT_SQUARE = "x**2 + 2*x + 1"
FRACTION_CANCEL = "(x**2 - 1) / (x - 1)"
EXP_COMBINE = "exp(x) * exp(y)"
LIKE_TERMS = "2*x + 3*x + x"
HYPERBOLIC = "(exp(x) - exp(-x)) / 2"
FRAC_ADD = "(x**2 + 1) / (x + 1) + (x - 1) / (x + 1)"
POWER_COMBINE = "x**2 * x**3"


def benchmark(name: str, func: Callable, runs: int = BENCH_RUNS) -> tuple[float, float]:
    """Run a benchmark and return (mean_us, std_us)."""
    # Warmup
    for _ in range(WARMUP_RUNS):
        func()
    
    # Measure
    times = timeit.repeat(func, number=1, repeat=runs)
    mean_us = (sum(times) / len(times)) * 1_000_000
    std_us = ((sum((t * 1_000_000 - mean_us) ** 2 for t in times) / len(times)) ** 0.5)
    return mean_us, std_us


def run_parsing_benchmarks():
    """Benchmark SymPy parsing (sympify)."""
    print("\n=== Parsing Benchmarks ===")
    
    results = []
    
    # Polynomial
    mean, std = benchmark("polynomial", lambda: sympify(POLYNOMIAL))
    results.append(("polynomial", mean, std))
    print(f"  polynomial: {mean:.2f} ± {std:.2f} µs")
    
    # Trig simple
    mean, std = benchmark("trig_simple", lambda: sympify(TRIG_SIMPLE))
    results.append(("trig_simple", mean, std))
    print(f"  trig_simple: {mean:.2f} ± {std:.2f} µs")
    
    # Complex
    mean, std = benchmark("complex_expr", lambda: sympify(COMPLEX_EXPR))
    results.append(("complex_expr", mean, std))
    print(f"  complex_expr: {mean:.2f} ± {std:.2f} µs")
    
    # Nested trig
    mean, std = benchmark("nested_trig", lambda: sympify(NESTED_TRIG))
    results.append(("nested_trig", mean, std))
    print(f"  nested_trig: {mean:.2f} ± {std:.2f} µs")
    
    return results


def run_differentiation_benchmarks():
    """Benchmark SymPy differentiation."""
    print("\n=== Differentiation Benchmarks ===")
    
    x = symbols('x')
    results = []
    
    # Pre-parse expressions
    poly_expr = sympify(POLYNOMIAL)
    trig_expr = sympify(TRIG_SIMPLE)
    complex_expr = sympify(COMPLEX_EXPR)
    nested_expr = sympify(NESTED_TRIG)
    chain_expr = sympify(CHAIN_SIN)
    exp_expr = sympify(EXP_SQUARED)
    quotient_expr = sympify(QUOTIENT)
    power_expr = sympify(POWER_XX)
    
    # AST only (diff without simplify)
    print("  [AST Only]")
    
    mean, std = benchmark("polynomial", lambda: sp.diff(poly_expr, x, evaluate=False))
    results.append(("ast/polynomial", mean, std))
    print(f"    polynomial: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("trig_simple", lambda: sp.diff(trig_expr, x, evaluate=False))
    results.append(("ast/trig_simple", mean, std))
    print(f"    trig_simple: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("complex_expr", lambda: sp.diff(complex_expr, x, evaluate=False))
    results.append(("ast/complex_expr", mean, std))
    print(f"    complex_expr: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("nested_trig", lambda: sp.diff(nested_expr, x, evaluate=False))
    results.append(("ast/nested_trig", mean, std))
    print(f"    nested_trig: {mean:.2f} ± {std:.2f} µs")
    
    # Full pipeline (parse + diff)
    print("  [Full Pipeline]")
    
    mean, std = benchmark("polynomial", lambda: sp.diff(sympify(POLYNOMIAL), x))
    results.append(("full/polynomial", mean, std))
    print(f"    polynomial: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("trig_simple", lambda: sp.diff(sympify(TRIG_SIMPLE), x))
    results.append(("full/trig_simple", mean, std))
    print(f"    trig_simple: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("chain_sin", lambda: sp.diff(sympify(CHAIN_SIN), x))
    results.append(("full/chain_sin", mean, std))
    print(f"    chain_sin: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("exp_squared", lambda: sp.diff(sympify(EXP_SQUARED), x))
    results.append(("full/exp_squared", mean, std))
    print(f"    exp_squared: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("complex_expr", lambda: sp.diff(sympify(COMPLEX_EXPR), x))
    results.append(("full/complex_expr", mean, std))
    print(f"    complex_expr: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("quotient", lambda: sp.diff(sympify(QUOTIENT), x))
    results.append(("full/quotient", mean, std))
    print(f"    quotient: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("nested_trig", lambda: sp.diff(sympify(NESTED_TRIG), x))
    results.append(("full/nested_trig", mean, std))
    print(f"    nested_trig: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("power_xx", lambda: sp.diff(sympify(POWER_XX), x))
    results.append(("full/power_xx", mean, std))
    print(f"    power_xx: {mean:.2f} ± {std:.2f} µs")
    
    return results


def run_simplification_benchmarks():
    """Benchmark SymPy simplification."""
    print("\n=== Simplification Benchmarks ===")
    
    results = []
    
    mean, std = benchmark("pythagorean", lambda: sp.simplify(sympify(PYTHAGOREAN)))
    results.append(("pythagorean", mean, std))
    print(f"  pythagorean: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("perfect_square", lambda: sp.simplify(sympify(PERFECT_SQUARE)))
    results.append(("perfect_square", mean, std))
    print(f"  perfect_square: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("fraction_cancel", lambda: sp.simplify(sympify(FRACTION_CANCEL)))
    results.append(("fraction_cancel", mean, std))
    print(f"  fraction_cancel: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("exp_combine", lambda: sp.simplify(sympify(EXP_COMBINE)))
    results.append(("exp_combine", mean, std))
    print(f"  exp_combine: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("like_terms", lambda: sp.simplify(sympify(LIKE_TERMS)))
    results.append(("like_terms", mean, std))
    print(f"  like_terms: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("hyperbolic", lambda: sp.simplify(sympify(HYPERBOLIC)))
    results.append(("hyperbolic", mean, std))
    print(f"  hyperbolic: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("frac_add", lambda: sp.simplify(sympify(FRAC_ADD)))
    results.append(("frac_add", mean, std))
    print(f"  frac_add: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("power_combine", lambda: sp.simplify(sympify(POWER_COMBINE)))
    results.append(("power_combine", mean, std))
    print(f"  power_combine: {mean:.2f} ± {std:.2f} µs")
    
    return results


def run_evaluation_benchmarks():
    """Benchmark SymPy numerical evaluation."""
    print("\n=== Evaluation Benchmarks ===")
    
    x = symbols('x')
    x_val = 2.5
    results = []
    
    # Pre-parse expressions
    poly = sympify("x**3 + 2*x**2 + x + 1")
    sin_x = sympify("sin(x)")
    cos_x = sympify("cos(x)")
    gamma_x = sympify("gamma(x)")
    digamma_x = sympify("digamma(x)")
    trigamma_x = sympify("trigamma(x)")
    erf_x = sympify("erf(x)")
    zeta_x = sympify("zeta(x)")
    besselj_0 = sympify("besselj(0, x)")
    besselj_1 = sympify("besselj(1, x)")
    bessely_0 = sympify("bessely(0, x)")
    bessely_1 = sympify("bessely(1, x)")
    lambertw_x = sympify("LambertW(x)")
    
    mean, std = benchmark("polynomial", lambda: poly.subs(x, x_val).evalf())
    results.append(("polynomial", mean, std))
    print(f"  polynomial: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("sin", lambda: sin_x.subs(x, x_val).evalf())
    results.append(("sin", mean, std))
    print(f"  sin: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("cos", lambda: cos_x.subs(x, x_val).evalf())
    results.append(("cos", mean, std))
    print(f"  cos: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("gamma", lambda: gamma_x.subs(x, x_val).evalf())
    results.append(("gamma", mean, std))
    print(f"  gamma: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("digamma", lambda: digamma_x.subs(x, x_val).evalf())
    results.append(("digamma", mean, std))
    print(f"  digamma: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("trigamma", lambda: trigamma_x.subs(x, x_val).evalf())
    results.append(("trigamma", mean, std))
    print(f"  trigamma: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("erf", lambda: erf_x.subs(x, x_val).evalf())
    results.append(("erf", mean, std))
    print(f"  erf: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("zeta", lambda: zeta_x.subs(x, x_val).evalf())
    results.append(("zeta", mean, std))
    print(f"  zeta: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("besselj_0", lambda: besselj_0.subs(x, x_val).evalf())
    results.append(("besselj_0", mean, std))
    print(f"  besselj_0: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("besselj_1", lambda: besselj_1.subs(x, x_val).evalf())
    results.append(("besselj_1", mean, std))
    print(f"  besselj_1: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("bessely_0", lambda: bessely_0.subs(x, x_val).evalf())
    results.append(("bessely_0", mean, std))
    print(f"  bessely_0: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("bessely_1", lambda: bessely_1.subs(x, x_val).evalf())
    results.append(("bessely_1", mean, std))
    print(f"  bessely_1: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("lambertw", lambda: lambertw_x.subs(x, x_val).evalf())
    results.append(("lambertw", mean, std))
    print(f"  lambertw: {mean:.2f} ± {std:.2f} µs")
    
    return results


def main():
    print("=" * 60)
    print("SymPy Benchmark Suite")
    print(f"SymPy version: {sp.__version__}")
    print(f"Runs: {BENCH_RUNS}, Warmup: {WARMUP_RUNS}")
    print("=" * 60)
    
    all_results = {}
    
    all_results["parsing"] = run_parsing_benchmarks()
    all_results["differentiation"] = run_differentiation_benchmarks()
    all_results["simplification"] = run_simplification_benchmarks()
    all_results["evaluation"] = run_evaluation_benchmarks()
    
    print("\n" + "=" * 60)
    print("Benchmark Complete!")
    print("=" * 60)
    
    return all_results


if __name__ == "__main__":
    main()
