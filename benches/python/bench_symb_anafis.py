#!/usr/bin/env python3
"""
SymbAnaFis Python Bindings Benchmark Suite

Benchmarks for symb_anafis Python bindings: parsing, differentiation, simplification.
Uses timeit for accurate timing measurements.
"""

import timeit
from typing import Callable

try:
    import symb_anafis
except ImportError:
    print("ERROR: symb_anafis not installed. Run: maturin develop --release")
    exit(1)

# =============================================================================
# Configuration
# =============================================================================

WARMUP_RUNS = 10
BENCH_RUNS = 1000

# =============================================================================
# Test Expressions (using ^ for power, as symb_anafis expects)
# =============================================================================

POLYNOMIAL = "x^3 + 2*x^2 + x + 1"
TRIG_SIMPLE = "sin(x) * cos(x)"
COMPLEX_EXPR = "x^2 * sin(x) * exp(x)"
NESTED_TRIG = "sin(cos(tan(x)))"
CHAIN_SIN = "sin(x^2)"
EXP_SQUARED = "exp(x^2)"
QUOTIENT = "(x^2 + 1) / (x - 1)"
POWER_XX = "x^x"

# Simplification expressions
PYTHAGOREAN = "sin(x)^2 + cos(x)^2"
PERFECT_SQUARE = "x^2 + 2*x + 1"
FRACTION_CANCEL = "(x^2 - 1) / (x - 1)"
EXP_COMBINE = "exp(x) * exp(y)"
LIKE_TERMS = "2*x + 3*x + x"
HYPERBOLIC = "(exp(x) - exp(-x)) / 2"
FRAC_ADD = "(x^2 + 1) / (x + 1) + (x - 1) / (x + 1)"
POWER_COMBINE = "x^2 * x^3"


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
    """Benchmark symb_anafis parsing."""
    print("\n=== Parsing Benchmarks ===")
    
    results = []
    
    mean, std = benchmark("polynomial", lambda: symb_anafis.parse(POLYNOMIAL))
    results.append(("polynomial", mean, std))
    print(f"  polynomial: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("trig_simple", lambda: symb_anafis.parse(TRIG_SIMPLE))
    results.append(("trig_simple", mean, std))
    print(f"  trig_simple: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("complex_expr", lambda: symb_anafis.parse(COMPLEX_EXPR))
    results.append(("complex_expr", mean, std))
    print(f"  complex_expr: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("nested_trig", lambda: symb_anafis.parse(NESTED_TRIG))
    results.append(("nested_trig", mean, std))
    print(f"  nested_trig: {mean:.2f} ± {std:.2f} µs")
    
    return results


def run_differentiation_benchmarks():
    """Benchmark symb_anafis differentiation."""
    print("\n=== Differentiation Benchmarks ===")
    
    results = []
    
    # Full pipeline (parse + diff + simplify)
    print("  [Full Pipeline]")
    
    mean, std = benchmark("polynomial", lambda: symb_anafis.diff(POLYNOMIAL, "x"))
    results.append(("full/polynomial", mean, std))
    print(f"    polynomial: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("trig_simple", lambda: symb_anafis.diff(TRIG_SIMPLE, "x"))
    results.append(("full/trig_simple", mean, std))
    print(f"    trig_simple: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("chain_sin", lambda: symb_anafis.diff(CHAIN_SIN, "x"))
    results.append(("full/chain_sin", mean, std))
    print(f"    chain_sin: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("exp_squared", lambda: symb_anafis.diff(EXP_SQUARED, "x"))
    results.append(("full/exp_squared", mean, std))
    print(f"    exp_squared: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("complex_expr", lambda: symb_anafis.diff(COMPLEX_EXPR, "x"))
    results.append(("full/complex_expr", mean, std))
    print(f"    complex_expr: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("quotient", lambda: symb_anafis.diff(QUOTIENT, "x"))
    results.append(("full/quotient", mean, std))
    print(f"    quotient: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("nested_trig", lambda: symb_anafis.diff(NESTED_TRIG, "x"))
    results.append(("full/nested_trig", mean, std))
    print(f"    nested_trig: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("power_xx", lambda: symb_anafis.diff(POWER_XX, "x"))
    results.append(("full/power_xx", mean, std))
    print(f"    power_xx: {mean:.2f} ± {std:.2f} µs")
    
    # Physics example: RC circuit
    mean, std = benchmark("physics_rc", lambda: symb_anafis.diff(
        "V0 * exp(-t / (R * C))", "t", fixed_vars=["V0", "R", "C"]
    ))
    results.append(("full/physics_rc", mean, std))
    print(f"    physics_rc: {mean:.2f} ± {std:.2f} µs")
    
    # Statistics example: Normal distribution
    mean, std = benchmark("stats_normal", lambda: symb_anafis.diff(
        "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)",
        "x",
        fixed_vars=["mu", "sigma"]
    ))
    results.append(("full/stats_normal", mean, std))
    print(f"    stats_normal: {mean:.2f} ± {std:.2f} µs")
    
    return results


def run_simplification_benchmarks():
    """Benchmark symb_anafis simplification."""
    print("\n=== Simplification Benchmarks ===")
    
    results = []
    
    mean, std = benchmark("pythagorean", lambda: symb_anafis.simplify(PYTHAGOREAN))
    results.append(("pythagorean", mean, std))
    print(f"  pythagorean: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("perfect_square", lambda: symb_anafis.simplify(PERFECT_SQUARE))
    results.append(("perfect_square", mean, std))
    print(f"  perfect_square: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("fraction_cancel", lambda: symb_anafis.simplify(FRACTION_CANCEL))
    results.append(("fraction_cancel", mean, std))
    print(f"  fraction_cancel: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("exp_combine", lambda: symb_anafis.simplify(EXP_COMBINE))
    results.append(("exp_combine", mean, std))
    print(f"  exp_combine: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("like_terms", lambda: symb_anafis.simplify(LIKE_TERMS))
    results.append(("like_terms", mean, std))
    print(f"  like_terms: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("hyperbolic", lambda: symb_anafis.simplify(HYPERBOLIC))
    results.append(("hyperbolic", mean, std))
    print(f"  hyperbolic: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("frac_add", lambda: symb_anafis.simplify(FRAC_ADD))
    results.append(("frac_add", mean, std))
    print(f"  frac_add: {mean:.2f} ± {std:.2f} µs")
    
    mean, std = benchmark("power_combine", lambda: symb_anafis.simplify(POWER_COMBINE))
    results.append(("power_combine", mean, std))
    print(f"  power_combine: {mean:.2f} ± {std:.2f} µs")
    
    return results


def run_combined_benchmarks():
    """Benchmark combined differentiation + simplification."""
    print("\n=== Combined Operations ===")
    
    results = []
    
    # Differentiate sin^2(x)
    mean, std = benchmark("diff_sin_squared", lambda: symb_anafis.diff("sin(x)^2", "x"))
    results.append(("diff_sin_squared", mean, std))
    print(f"  diff_sin_squared: {mean:.2f} ± {std:.2f} µs")
    
    # Differentiate quotient
    mean, std = benchmark("diff_quotient", lambda: symb_anafis.diff(QUOTIENT, "x"))
    results.append(("diff_quotient", mean, std))
    print(f"  diff_quotient: {mean:.2f} ± {std:.2f} µs")
    
    return results


def run_large_expr_benchmarks():
    """Benchmark large expression handling."""
    print("\n=== Large Expression Benchmarks (300 terms) ===")
    from bench_runner import generate_mixed_complex
    
    large_expr_str = generate_mixed_complex(300)
    results = []
    
    # Parsing
    mean, std = benchmark("parse_300", lambda: symb_anafis.parse(large_expr_str), runs=100)
    results.append(("parse_300", mean, std))
    print(f"  Parsing: {mean:.2f} ± {std:.2f} µs")
    
    # Differentiation (Full Pipeline)
    mean, std = benchmark("full_300", lambda: symb_anafis.diff(large_expr_str, "x"), runs=50)
    results.append(("full_300", mean, std))
    print(f"  Full Diff: {mean/1000:.2f} ± {std/1000:.2f} ms")

    # Repeat for comparison with SymPy's Diff+Simplify (since we always simplify)
    # We can just reuse the same lambda or copy the result, but running it again is cleaner for the runner logic
    mean, std = benchmark("full_300_simp", lambda: symb_anafis.diff(large_expr_str, "x"), runs=50)
    results.append(("full_300_simp", mean, std))
    # print(f"  (Rep) Full Diff: {mean/1000:.2f} ± {std/1000:.2f} ms") # Optional print
    
    return results


def main():
    print("=" * 60)
    print("SymbAnaFis Python Bindings Benchmark Suite")
    print(f"Runs: {BENCH_RUNS}, Warmup: {WARMUP_RUNS}")
    print("=" * 60)
    
    all_results = {}
    
    all_results["parsing"] = run_parsing_benchmarks()
    all_results["differentiation"] = run_differentiation_benchmarks()
    all_results["simplification"] = run_simplification_benchmarks()
    all_results["combined"] = run_combined_benchmarks()
    all_results["large_expr"] = run_large_expr_benchmarks()
    
    print("\n" + "=" * 60)
    print("Benchmark Complete!")
    print("=" * 60)
    
    return all_results


if __name__ == "__main__":
    main()
