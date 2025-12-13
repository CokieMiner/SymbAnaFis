#!/usr/bin/env python3
"""
Unified Benchmark Runner

Runs all Python benchmarks and generates comparison tables.
"""

import sys
import json
from datetime import datetime
from typing import Any

# Try to import all libraries
HAS_SYMPY = False
HAS_SYMB_ANAFIS = False

try:
    import sympy as sp
    HAS_SYMPY = True
except ImportError:
    print("WARNING: SymPy not installed. Skipping SymPy benchmarks.")

try:
    import symb_anafis
    HAS_SYMB_ANAFIS = True
except ImportError:
    print("WARNING: symb_anafis not installed. Run: maturin develop --release")


def format_time(us: float) -> str:
    """Format time in appropriate units."""
    if us < 1:
        return f"{us * 1000:.2f} ns"
    elif us < 1000:
        return f"{us:.2f} µs"
    else:
        return f"{us / 1000:.2f} ms"


def calculate_speedup(sympy_us: float, our_us: float) -> str:
    """Calculate speedup ratio."""
    if our_us <= 0:
        return "N/A"
    ratio = sympy_us / our_us
    if ratio >= 1:
        return f"**{ratio:.1f}x** faster"
    else:
        return f"-{1/ratio:.1f}x slower"


def run_benchmarks():
    """Run all available benchmarks."""
    results = {}
    
    if HAS_SYMPY:
        print("\n" + "=" * 70)
        print("Running SymPy Benchmarks...")
        print("=" * 70)
        from bench_sympy import main as run_sympy
        results["sympy"] = run_sympy()
    
    if HAS_SYMB_ANAFIS:
        print("\n" + "=" * 70)
        print("Running SymbAnaFis Benchmarks...")
        print("=" * 70)
        from bench_symb_anafis import main as run_symb_anafis
        results["symb_anafis"] = run_symb_anafis()
    
    return results


def generate_comparison_table(results: dict[str, Any]) -> str:
    """Generate markdown comparison tables."""
    if not (HAS_SYMPY and HAS_SYMB_ANAFIS):
        return "Cannot generate comparison: missing library"
    
    sympy_results = results.get("sympy", {})
    our_results = results.get("symb_anafis", {})
    
    lines = []
    lines.append("# Python Benchmark Comparison")
    lines.append(f"\n**Date:** {datetime.now().strftime('%Y-%m-%d %H:%M')}")
    if HAS_SYMPY:
        lines.append(f"**SymPy Version:** {sp.__version__}")
    lines.append("")
    
    # Parsing comparison
    if "parsing" in sympy_results and "parsing" in our_results:
        lines.append("## Parsing")
        lines.append("")
        lines.append("| Expression | SymPy (µs) | SymbAnaFis (µs) | Speedup |")
        lines.append("|------------|------------|-----------------|---------|")
        
        sympy_parsing = {r[0]: r[1] for r in sympy_results["parsing"]}
        our_parsing = {r[0]: r[1] for r in our_results["parsing"]}
        
        for name in ["polynomial", "trig_simple", "complex_expr", "nested_trig"]:
            if name in sympy_parsing and name in our_parsing:
                speedup = calculate_speedup(sympy_parsing[name], our_parsing[name])
                lines.append(f"| {name} | {sympy_parsing[name]:.2f} | {our_parsing[name]:.2f} | {speedup} |")
        lines.append("")
    
    # Differentiation comparison
    if "differentiation" in sympy_results and "differentiation" in our_results:
        lines.append("## Differentiation (Full Pipeline)")
        lines.append("")
        lines.append("| Expression | SymPy (µs) | SymbAnaFis (µs) | Speedup |")
        lines.append("|------------|------------|-----------------|---------|")
        
        sympy_diff = {r[0]: r[1] for r in sympy_results["differentiation"]}
        our_diff = {r[0]: r[1] for r in our_results["differentiation"]}
        
        for name in ["full/polynomial", "full/trig_simple", "full/chain_sin", 
                     "full/exp_squared", "full/complex_expr", "full/quotient",
                     "full/nested_trig", "full/power_xx"]:
            if name in sympy_diff and name in our_diff:
                display_name = name.replace("full/", "")
                speedup = calculate_speedup(sympy_diff[name], our_diff[name])
                lines.append(f"| {display_name} | {sympy_diff[name]:.2f} | {our_diff[name]:.2f} | {speedup} |")
        lines.append("")
    
    # Simplification comparison
    if "simplification" in sympy_results and "simplification" in our_results:
        lines.append("## Simplification")
        lines.append("")
        lines.append("| Expression | SymPy (µs) | SymbAnaFis (µs) | Speedup |")
        lines.append("|------------|------------|-----------------|---------|")
        
        sympy_simp = {r[0]: r[1] for r in sympy_results["simplification"]}
        our_simp = {r[0]: r[1] for r in our_results["simplification"]}
        
        for name in ["pythagorean", "perfect_square", "fraction_cancel", "exp_combine",
                     "like_terms", "hyperbolic", "frac_add", "power_combine"]:
            if name in sympy_simp and name in our_simp:
                speedup = calculate_speedup(sympy_simp[name], our_simp[name])
                lines.append(f"| {name} | {sympy_simp[name]:.2f} | {our_simp[name]:.2f} | {speedup} |")
        lines.append("")
    
    return "\n".join(lines)


def main():
    print("=" * 70)
    print("SymbAnaFis Benchmark Suite")
    print("Comparing: SymPy vs SymbAnaFis (Python bindings)")
    print("=" * 70)
    
    if not HAS_SYMPY and not HAS_SYMB_ANAFIS:
        print("ERROR: No libraries available for benchmarking!")
        sys.exit(1)
    
    results = run_benchmarks()
    
    print("\n" + "=" * 70)
    print("Generating Comparison Tables...")
    print("=" * 70)
    
    comparison = generate_comparison_table(results)
    print(comparison)
    
    print("\n" + "=" * 70)
    print("All Benchmarks Complete!")
    print("=" * 70)


if __name__ == "__main__":
    main()
