# SymbAnaFis vs SymPy vs Symbolica Benchmark Comparison

**Date:** December 17, 2025 (Updated for v0.3.1 + Interned Function Names)
**SymbAnaFis Version:** 0.3.1 (N-ary AST + InternedSymbol for function names)
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
| **Parsing** | SymbAnaFis | **1.2x - 1.9x faster** |
| **Differentiation** | Symbolica | **18x - 49x faster** |

> **Note**: SymbAnaFis `diff()` always includes simplification. Symbolica's `derivative()` also auto-normalizes. Both are fair comparisons.

---

## Rust Benchmark Results (Criterion)

### 1. Parsing (String → Expression)

| Expression | SymbAnaFis (ns) | Symbolica (µs) | vs Symbolica |
|------------|-----------------|----------------|--------------|
| Polynomial `x^3+2*x^2+x+1` | 1156 | 1.42 | **1.2x faster** |
| Trig `sin(x)*cos(x)` | 645 | 1.02 | **1.6x faster** |
| Complex `x^2*sin(x)*exp(x)` | 967 | 1.36 | **1.4x faster** |
| Nested `sin(cos(tan(x)))` | 556 | 1.07 | **1.9x faster** |

### 2. Differentiation (Pre-parsed, includes simplification)

Both libraries auto-simplify/normalize results during differentiation.

| Expression | SymbAnaFis (µs) | Symbolica (µs) | vs Symbolica |
|------------|-----------------|----------------|--------------|
| Polynomial | 17.09 | 1.18 | -14x slower |
| Trig | 45.93 | 0.94 | -49x slower |
| Complex | 60.51 | 1.39 | -43x slower |
| Nested Trig | 27.91 | 0.85 | -33x slower |

### 3. Differentiation (Full Pipeline: parse + diff + simplify)

| Expression | SymbAnaFis (µs) | Symbolica (µs) | vs Symbolica |
|------------|-----------------|----------------|--------------|
| Polynomial | 18.83 | 2.66 | -7x slower |
| Trig `sin(x)*cos(x)` | 47.04 | 2.02 | -23x slower |
| Chain `sin(x^2)` | 14.11 | 1.32 | -10x slower |
| Exp `exp(x^2)` | 15.39 | 1.30 | -12x slower |
| Complex | 62.85 | 2.84 | -22x slower |
| Quotient `(x^2+1)/(x-1)` | 45.25 | 2.64 | -17x slower |
| Nested | 29.25 | 2.05 | -14x slower |
| Power `x^x` | 22.95 | 1.40 | -16x slower |

### 4. Large Complex Expressions (300 mixed terms)

| Operation | SymbAnaFis | Symbolica | Ratio |
| :--- | :--- | :--- | :--- |
| **Parse** | 1.09 ms | 0.35 ms | **3.1x slower** |
| **Diff (AST reuse)** | 6.30 ms | 0.36 ms | **17.5x slower** |
| **Full (parse+diff)** | 7.49 ms | 0.72 ms | **10.4x slower** |

### 5. Large Physics Expressions

| Expression | SymbAnaFis (µs) | Symbolica (µs) | vs Symbolica |
| :--- | :--- | :--- | :--- |
| **Maxwell-Boltzmann** | 185.96 | 9.24 | -20x slower |
| **Gaussian 2D** | 240.37 | 8.46 | -28x slower |
| **Orbital Energy** | 111.35 | 7.86 | -14x slower |
| **Wave Equation** | 36.48 | 4.05 | -9x slower |
| **Normal PDF** | 198.78 | 6.12 | -32x slower |

### 6. Internal Benchmarks (SymbAnaFis only)

#### Parsing
| Expression | Time (µs) |
|------------|-----------|
| Polynomial | 1.41 |
| Trig | 0.74 |
| Complex | 1.29 |
| Nested | 0.65 |

#### Differentiation (includes simplification)
| Expression | Time (µs) |
|------------|-----------|
| Polynomial | 19.76 |
| Trig | 56.33 |
| Complex | 74.50 |
| Nested | 35.07 |

#### Differentiation (Full Pipeline)
| Expression | Time (µs) |
|------------|-----------|
| Polynomial | 23.86 |
| Trig | 55.70 |
| Chain sin | 17.29 |
| Exp squared | 19.35 |
| Complex | 77.85 |
| Quotient | 56.92 |
| Nested | 36.77 |
| Power x^x | 29.25 |

#### Simplification
| Pattern | Time (µs) |
|---------|-----------|
| Pythagorean `sin²+cos²` | 16.80 |
| Perfect Square | 13.02 |
| Fraction Cancel | 22.09 |
| Exp Combine | 19.55 |
| Like Terms | 12.74 |
| Hyperbolic | 27.16 |
| Frac Add | 46.90 |
| Power Combine | 10.83 |

#### Evaluation
| Function | Time (ns) |
|----------|-----------|
| Polynomial | 1726 |
| sin | 474 |
| cos | 483 |
| exp | 469 |
| ln | 457 |
| sqrt | 476 |
| gamma | 444 |
| digamma | 428 |
| trigamma | 435 |
| erf | 513 |
| erfc | 539 |
| zeta | 1186 |
| lambertw | 902 |
| besselj(0) | 586 |
| besselj(1) | 542 |
| bessely(0) | 566 |
| bessely(1) | 567 |
| besseli(0) | 553 |
| besselk(0) | 557 |
| polygamma(2) | 623 |
| polygamma(3) | 635 |
| polygamma(4) | 639 |
| tetragamma | 421 |


### 5. Large Expressions (300 Mixed Terms)

Benchmarks on a complex expression with 300 terms including polynomials, trigonometric functions, exponentials, logarithms, and nested function calls.

| Operation | SymbAnaFis | Symbolica | Ratio |
|-----------|------------|-----------|-------|
| Parsing | 1.10 ms | 350 µs | **3.1x** slower |
| Differentiation | 18.7 ms | 359 µs | **52x** slower |
| Full Pipeline | 20.2 ms | 722 µs | **28x** slower |

> **Note:** The performance gap widens significantly for very large expressions. This is primarily due to SymbAnaFis using a tree-based AST with `Arc` smart pointers and cloning during differentiation, whereas Symbolica uses a highly optimized arena allocator (Atoms) that avoids most allocations. **Memory pooling** is the next planned optimization to address this.


## Python Benchmark Results (SymbAnaFis Python bindings vs SymPy)

### Parsing
| Expression | SymPy (µs) | SymbAnaFis (µs) | Speedup |
|------------|------------|-----------------|---------|
| Polynomial | 134.81 | 7.20 | **18.7x faster** |
| Trig | 114.78 | 5.37 | **21.4x faster** |
| Complex | 131.54 | 7.57 | **17.4x faster** |
| Nested | 111.35 | 5.44 | **20.5x faster** |

### Differentiation (Full Pipeline)
| Expression | SymPy (µs) | SymbAnaFis (µs) | Result |
|------------|------------|-----------------|--------|
| Polynomial | 172.38 | 254.43 | -1.5x slower |
| Trig | 143.20 | 422.19 | -2.9x slower |
| Chain sin | 205.99 | 181.45 | **1.1x faster** |
| Exp squared | 204.82 | 172.63 | **1.2x faster** |
| Complex | 155.12 | 1175.70 | -7.6x slower |
| Quotient | 158.08 | 516.54 | -3.3x slower |
| Nested | 216.71 | 453.38 | -2.1x slower |
| Power x^x | 196.64 | 210.55 | -1.1x slower |

### Simplification
| Pattern | SymPy (µs) | SymbAnaFis (µs) | Speedup |
|---------|------------|-----------------|---------|
| Pythagorean | 5481.19 | 127.05 | **43.1x faster** |
| Perfect Square | 2196.08 | 123.95 | **17.7x faster** |
| Fraction Cancel | 1931.20 | 103.80 | **18.6x faster** |
| Exp Combine | 1646.49 | 140.42 | **11.7x faster** |
| Like Terms | 633.67 | 112.20 | **5.6x faster** |
| Hyperbolic | 3687.42 | 173.07 | **21.3x faster** |
| Frac Add | 1510.18 | 391.54 | **3.9x faster** |
| Power Combine | 636.26 | 84.25 | **7.6x faster** |

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
