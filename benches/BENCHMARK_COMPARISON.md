# SymbAnaFis vs SymPy vs Symbolica Benchmark Comparison

**Date:** December 19, 2025 (Updated: Lazy Canonicalization + GCD Fix)
**SymbAnaFis Version:** 0.3.1 (N-ary AST + Lazy Canonicalization)
**SymPy Version:** 1.14.0
**Symbolica Version:** 1.0.1
**System:** Linux (Fedora 43)
**Criterion Version:** 0.8.1

## Summary

SymbAnaFis is a symbolic mathematics library written in Rust, designed for parsing, differentiation, and simplification of mathematical expressions. This document compares its performance against SymPy and Symbolica.

### Key Findings vs SymPy (Python Bindings)

| Category | Winner | Speedup Range |
|----------|--------|---------------|
| **Parsing** | SymbAnaFis | **17x - 21x faster** |
| **Differentiation (Full)** | Mixed | **1.1x - 1.2x faster** (some slower) |
| **Simplification** | SymbAnaFis | **4x - 43x faster** |

### Key Findings vs Symbolica (Native Rust)

| Category | Winner | Notes |
|----------|--------|-------|
| **Parsing** | SymbAnaFis | **1.3x - 1.9x faster** (except poly) |
| **Differentiation** | Symbolica | **7.5x - 22x faster** (Full Pipeline) |
| **300-term Parsing** | ~Tie | 384µs vs 349µs (1.1x slower) |

> **Note**: SymbAnaFis `diff()` always includes simplification. Symbolica's `derivative()` also auto-normalizes. Both are fair comparisons.

---

## Rust Benchmark Results (Criterion)

### 1. Parsing (String → Expression)

| Expression | SymbAnaFis (ns) | Symbolica (µs) | vs Symbolica |
|------------|-----------------|----------------|--------------|
| Polynomial `x^3+2*x^2+x+1` | 1060 | 1.39 | **1.3x faster** 🎉 |
| Trig `sin(x)*cos(x)` | 652 | 1.01 | **1.5x faster** |
| Complex `x^2*sin(x)*exp(x)` | 977 | 1.35 | **1.4x faster** |
| Nested `sin(cos(tan(x)))` | 561 | 1.05 | **1.9x faster** |

### 2. Differentiation (Pre-parsed, includes simplification)

Both libraries auto-simplify/normalize results during differentiation.

| Expression | SymbAnaFis (µs) | Symbolica (µs) | vs Symbolica |
|------------|-----------------|----------------|--------------|
| Polynomial | 17.02 | 1.18 | -14.4x slower |
| Trig | 28.96 | 0.93 | -31x slower |
| Complex | 59.45 | 1.41 | -42x slower |
| Nested Trig | 39.58 | 0.85 | -47x slower |

### 3. Differentiation (Full Pipeline: parse + diff + simplify)

| Expression | SymbAnaFis (µs) | Symbolica (µs) | vs Symbolica |
|------------|-----------------|----------------|--------------|
| Polynomial | 18.44 | 2.68 | -6.9x slower |
| Trig `sin(x)*cos(x)` | 30.26 | 2.05 | -14.8x slower |
| Chain `sin(x^2)` | 13.98 | 1.31 | -10.7x slower |
| Exp `exp(x^2)` | 15.26 | 1.29 | -11.8x slower |
| Complex | 60.99 | 2.82 | -21.6x slower |
| Quotient `(x^2+1)/(x-1)` | 50.17 | 2.59 | -19.4x slower |
| Nested | 41.06 | 2.05 | -20x slower |
| Power `x^x` | 22.52 | 1.39 | -16.2x slower |

### 4. Large Complex Expressions (300 mixed terms)

| Operation | SymbAnaFis | Symbolica | Ratio |
| :--- | :--- | :--- | :--- |
| **Parse** | 242 µs | 353 µs | **1.5x faster** 🎉 |
| **Diff (AST reuse)** | 6.76 ms | 351 µs | **19x slower** |
| **Full (parse+diff)** | 7.27 ms | 716 µs | **10x slower** |

> **Optimizations (Dec 2025):**
> - Parser N-ary collection: 8.7ms → 385µs (−95%)
> - Lazy canonicalization: 385µs → 242µs (−37% parse)
> - GCD multivariate fix: prevents infinite loop

### 5. Large Physics Expressions

| Expression | SymbAnaFis (µs) | Symbolica (µs) | vs Symbolica |
| :--- | :--- | :--- | :--- |
| **Maxwell-Boltzmann** | 204.27 | 9.32 | -22x slower |
| **Gaussian 2D** | 201.87 | 8.51 | -24x slower |
| **Orbital Energy** | 97.06 | 7.68 | -13x slower |
| **Wave Equation** | 36.02 | 4.05 | -9x slower |
| **Normal PDF** | 180.37 | 6.13 | -29x slower |

### 6. Internal Benchmarks (SymbAnaFis only)

#### Parsing
| Expression | Time (µs) |
|------------|-----------|
| Polynomial | 1.76 |
| Trig | 0.75 |
| Complex | 1.38 |
| Nested | 0.64 |

#### Differentiation (includes simplification)
| Expression | Time (µs) |
|------------|-----------|
| Polynomial | 22.73 |
| Trig | 37.35 |
| Complex | 74.64 |
| Nested | 34.60 |

#### Differentiation (Full Pipeline)
| Expression | Time (µs) |
|------------|-----------|
| Polynomial | 25.34 |
| Trig | 38.45 |
| Chain sin | 17.16 |
| Exp squared | 19.11 |
| Complex | 78.55 |
| Quotient | 62.98 |
| Nested | 36.27 |
| Power x^x | 29.01 |

#### Simplification
| Pattern | Time (µs) |
|---------|-----------|
| Pythagorean `sin²+cos²` | 17.76 |
| Perfect Square | 13.52 |
| Fraction Cancel | 19.17 |
| Exp Combine | 20.24 |
| Like Terms | 4.42 |
| Hyperbolic | 27.84 |
| Frac Add | 45.05 |
| Power Combine | 8.44 |

#### Evaluation
| Function | Time (ns) |
|----------|-----------|
| Polynomial | 2040 |
| sin | 517 |
| cos | 519 |
| exp | 516 |
| ln | 494 |
| sqrt | 503 |
| gamma | 492 |
| digamma | 486 |
| trigamma | 474 |
| erf | 537 |
| erfc | 557 |
| zeta | 1220 |
| lambertw | 923 |
| besselj(0) | 634 |
| besselj(1) | 643 |
| bessely(0) | 646 |
| bessely(1) | 638 |
| besseli(0) | 644 |
| besselk(0) | 637 |
| polygamma(2) | 720 |
| polygamma(3) | 706 |
| polygamma(4) | 718 |
| tetragamma | 493 |


### 5. Large Expressions (300 Mixed Terms)

Benchmarks on a complex expression with 300 terms including polynomials, trigonometric functions, exponentials, logarithms, and nested function calls.

| Operation | SymbAnaFis | Symbolica | Ratio |
|-----------|------------|-----------|-------|
| Parsing | 384 µs | 349 µs | **1.1x** slower |
| Differentiation | 6.96 ms | 352 µs | **19.8x** slower |
| Full Pipeline | 7.31 ms | 722 µs | **10.1x** slower |

> **Note:** The performance gap widens significantly for very large expressions. This is primarily due to SymbAnaFis using a tree-based AST with `Arc` smart pointers and cloning during differentiation, whereas Symbolica uses a highly optimized arena allocator (Atoms) that avoids most allocations. **Memory pooling** is the next planned optimization to address this.


## Python Benchmark Comparison

**Date:** 2025-12-18 11:28
**SymPy Version:** 1.14.0

## Parsing

| Expression | SymPy (µs) | SymbAnaFis (µs) | Speedup |
|------------|------------|-----------------|---------|
| polynomial | 124.81 | 2.20 | **56.8x** faster |
| trig_simple | 104.28 | 1.17 | **88.8x** faster |
| complex_expr | 116.87 | 1.81 | **64.6x** faster |
| nested_trig | 101.79 | 1.10 | **92.4x** faster |

## Differentiation (Full Pipeline)

| Expression | SymPy (µs) | SymbAnaFis (µs) | Speedup |
|------------|------------|-----------------|---------|
| polynomial | 157.15 | 23.22 | **6.8x** faster |
| trig_simple | 130.77 | 55.69 | **2.3x** faster |
| chain_sin | 193.30 | 17.08 | **11.3x** faster |
| exp_squared | 192.39 | 19.07 | **10.1x** faster |
| complex_expr | 148.70 | 73.28 | **2.0x** faster |
| quotient | 150.58 | 54.08 | **2.8x** faster |
| nested_trig | 208.46 | 35.45 | **5.9x** faster |
| power_xx | 186.00 | 27.68 | **6.7x** faster |

## Simplification

| Expression | SymPy (µs) | SymbAnaFis (µs) | Speedup |
|------------|------------|-----------------|---------|
| pythagorean | 5327.40 | 18.43 | **289.1x** faster |
| perfect_square | 2199.58 | 14.34 | **153.4x** faster |
| fraction_cancel | 1887.17 | 23.21 | **81.3x** faster |
| exp_combine | 1571.92 | 21.12 | **74.4x** faster |
| like_terms | 618.39 | 12.77 | **48.4x** faster |
| hyperbolic | 3585.65 | 28.72 | **124.8x** faster |
| frac_add | 1485.94 | 40.31 | **36.9x** faster |
| power_combine | 623.07 | 11.09 | **56.2x** faster |

## Large Expressions (300 terms)

| Operation | SymPy (ms) | SymbAnaFis (ms) | Speedup |
|-----------|------------|-----------------|---------|
| Parsing | 145.71 | 1.46 | **99.9x** faster |
| Full Pipeline (Parse+Diff) | 149.42 | 8.35 | **17.9x** faster |
| Full Pipeline + Simplify | Timeout (> 15m) | 8.19 | -- |

---

## Analysis: SymbAnaFis vs Symbolica

### Why Symbolica is Faster for Differentiation

Both libraries use AST-based representations (`Num`, `Var`, `Fun`, `Mul`, `Add`, `Pow`), but Symbolica employs several low-level optimizations:

| Optimization | Symbolica | SymbAnaFis |
|--------------|-----------|------------|
| **Memory Representation** | Compact `Vec<u8>` with type tags | `Arc<Symbol>` with heap allocation |
| **Workspace** | Thread-local memory pool | Fresh allocations per operation |
| **Normalization** | Lightweight inline normalization | Multi-pass rule-based simplification |
| **Data Layout** | Cache-friendly byte arrays | Pointer-chasing through `Arc` |

### Where SymbAnaFis Excels

1. **Parsing**: 1.6-2.3x faster than Symbolica - simpler AST construction
2. **Trigonometric Identities**: Extensive patterns (sin²+cos²=1, double angles, etc.)
3. **Hyperbolic Functions**: sinh, cosh, tanh recognition and simplification
4. **Simplification vs SymPy**: 4x-43x faster due to rule-based engine
5. **Custom Functions**: First-class support for user-defined functions with derivatives
6. **Extensibility**: Rule-based engine can be extended with new patterns

### Where Symbolica Excels

1. **Differentiation Pipeline**: 13-62x faster due to memory optimizations
2. **Polynomial Operations**: Native multivariate polynomial factorization
3. **Large Expressions**: Coefficient ring optimizations and streaming
4. **Series Expansion**: Built-in Taylor/Laurent series
5. **Pattern Matching**: Powerful wildcard-based pattern matching

---

## Analysis: Why SymbAnaFis Beats SymPy

1. **Rule-based engine with ExprKind filtering**: O(1) rule lookup instead of O(n) scanning
2. **No Python overhead**: Pure Rust with zero-cost abstractions (18-21x parsing speedup)
3. **Pattern matching optimization**: Rules only run on applicable expression types
4. **Efficient AST representation**: Using Rust's `Arc` for shared expression nodes
5. **Compiled native code**: No interpreter overhead

---

## Hardware

Benchmarks were run on a single machine to ensure fair comparison:

**System Specs:**
-   **CPU:** AMD Ryzen AI 7 350 w/ Radeon 860M
-   **RAM:** 32 GB
-   **OS:** Fedora 43 (Linux 6.17.9-300.fc43.x86_64)
-   **Rust Version:** 1.90.0 (1159e78c4 2025-09-14)
-   **Python Version:** 3.14.0

**Methodology:**
-   **Rust**: `cargo bench` (Criterion, 100 samples)
-   **Python**: `timeit` (1000 iterations)
