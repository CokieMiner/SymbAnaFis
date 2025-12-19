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
    elif us < 1_000_000:
        return f"{us / 1000:.2f} ms"
    else:
        return f"{us / 1_000_000:.2f} s"


def generate_mixed_complex(n: int) -> str:
    """
    Generates a complex mixed expression with N terms.
    Includes: polynomials, trig, exponentials, fractions, and nested functions.
    Matches the logic in benches/rust/large_expr.rs.
    """
    terms = []
    
    for i in range(1, n + 1):
        if i > 1:
            pass # Operators will be joined later
            
        term = ""
        # Mix term types based on index
        mode = i % 5
        if mode == 0:
            # Polynomial term: i*x^i
            term = f"{i}*x^{i % 10 + 1}"
        elif mode == 1:
            # Trig term: sin(i*x) * cos(x)
            term = f"sin({i}*x)*cos(x)"
        elif mode == 2:
            # Exponential/Log: exp(x/i) + ln(x + i)
            # Python's ln is usually log, but SymbAnaFis parses ln
            term = f"(exp(x/{i}) + ln(x + {i}))"
        elif mode == 3:
            # Rational: (x^2 + i) / (x + i)
            term = f"(x^2 + {i})/(x + {i})"
        elif mode == 4:
            # Nested: sin(exp(x) + i)
            term = f"sin(exp(x) + {i})"
            
        terms.append(term)

    # Join with mixed operators
    result = []
    for i, term in enumerate(terms):
        if i == 0:
            result.append(term)
        else:
            # Match Rust logic:
            # if i % 3 == 0: " + "
            # elif i % 3 == 1: " - "
            # else: " + "
            # Note: Rust loop was 1-based (i), here enumerate is 0-based index.
            # But the Rust logic used `i` which was the 1-based term count.
            # Let's align with the term count `count = i + 1`
            count = i + 1
            if count % 3 == 0:
                result.append(" + ")
            elif count % 3 == 1:
                result.append(" - ")
            else:
                result.append(" + ")
            result.append(term)
            
    return "".join(result)


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
    
    # Large Expression comparison
    if "large_expr" in sympy_results and "large_expr" in our_results:
        lines.append("## Large Expressions (300 terms)")
        lines.append("")
        lines.append("| Operation | SymPy (ms) | SymbAnaFis (ms) | Speedup |")
        lines.append("|-----------|------------|-----------------|---------|")
        
        sympy_large = {r[0]: r[1] for r in sympy_results["large_expr"]}
        our_large = {r[0]: r[1] for r in our_results["large_expr"]}
        
        # Convert µs to ms for display
        for name, display_name in [("parse_300", "Parsing"), ("full_300", "Full Pipeline (Parse+Diff)"), ("full_300_simp", "Full Pipeline + Simplify")]:
            if name in sympy_large and name in our_large:
                s_val = sympy_large[name]
                o_val = our_large[name]
                
                # Handle timeout (None)
                if s_val is None:
                    sympy_str = "Timeout (> 5m)"
                    speedup = "N/A"
                else:
                    sympy_str = f"{s_val/1000:.2f}"
                    speedup = calculate_speedup(s_val, o_val) if o_val is not None else "N/A"
                
                our_str = f"{o_val/1000:.2f}" if o_val is not None else "Failed"
                
                lines.append(f"| {display_name} | {sympy_str} | {our_str} | {speedup} |")
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
