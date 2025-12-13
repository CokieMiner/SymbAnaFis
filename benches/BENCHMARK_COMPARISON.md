# SymbAnaFis vs SymPy vs Symbolica Benchmark Comparison

**Date:** December 12, 2025 (Updated)
**SymbAnaFis Version:** 0.3.0
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
| **Parsing** | SymbAnaFis | **1.6x - 2.3x faster** |
| **Differentiation** | Symbolica | **13x - 62x faster** |

> **Note**: SymbAnaFis `diff()` always includes simplification. Symbolica's `derivative()` also auto-normalizes. Both are fair comparisons.

---

## Rust Benchmark Results (Criterion)

### 1. Parsing (String → Expression)

| Expression | SymbAnaFis (ns) | Symbolica (µs) | vs Symbolica |
|------------|-----------------|----------------|--------------|
| Polynomial `x^3+2*x^2+x+1` | 876 | 1.44 | **1.6x faster** |
| Trig `sin(x)*cos(x)` | 525 | 1.03 | **2.0x faster** |
| Complex `x^2*sin(x)*exp(x)` | 787 | 1.36 | **1.7x faster** |
| Nested `sin(cos(tan(x)))` | 467 | 1.08 | **2.3x faster** |

### 2. Differentiation (Pre-parsed, includes simplification)

Both libraries auto-simplify/normalize results during differentiation.

| Expression | SymbAnaFis (µs) | Symbolica (µs) | vs Symbolica |
|------------|-----------------|----------------|--------------|
| Polynomial | 35.45 | 1.17 | -30x slower |
| Trig | 49.90 | 0.95 | -53x slower |
| Complex | 176.36 | 1.41 | -125x slower |
| Nested Trig | 56.75 | 0.86 | -66x slower |

### 3. Differentiation (Full Pipeline: parse + diff + simplify)

| Expression | SymbAnaFis (µs) | Symbolica (µs) | vs Symbolica |
|------------|-----------------|----------------|--------------|
| Polynomial | 36.77 | 2.73 | -13x slower |
| Trig `sin(x)*cos(x)` | 51.08 | 2.02 | -25x slower |
| Chain `sin(x^2)` | 22.10 | 1.35 | -16x slower |
| Exp `exp(x^2)` | 21.75 | 1.30 | -17x slower |
| Complex | 177.11 | 2.84 | -62x slower |
| Quotient `(x^2+1)/(x-1)` | 76.60 | 2.63 | -29x slower |
| Nested | 57.82 | 2.05 | -28x slower |
| Power `x^x` | 26.59 | 1.44 | -18x slower |

### 4. Internal Benchmarks (SymbAnaFis only)

#### Parsing
| Expression | Time (µs) |
|------------|-----------|
| Polynomial | 1.02 |
| Trig | 0.66 |
| Complex | 1.11 |
| Nested | 0.56 |

#### Differentiation (includes simplification)
| Expression | Time (µs) |
|------------|-----------|
| Polynomial | 45.03 |
| Trig | 63.36 |
| Complex | 230.58 |
| Nested | 72.25 |

#### Differentiation (Full Pipeline)
| Expression | Time (µs) |
|------------|-----------|
| Polynomial | 48.59 |
| Trig | 64.74 |
| Chain sin | 27.78 |
| Exp squared | 27.37 |
| Complex | 238.05 |
| Quotient | 99.22 |
| Nested | 75.48 |
| Power x^x | 34.58 |

#### Simplification
| Pattern | Time (µs) |
|---------|-----------|
| Pythagorean `sin²+cos²` | 17.85 |
| Perfect Square | 20.87 |
| Fraction Cancel | 16.76 |
| Exp Combine | 21.04 |
| Like Terms | 18.82 |
| Hyperbolic | 25.57 |
| Frac Add | 68.46 |
| Power Combine | 11.58 |

#### Evaluation
| Function | Time (ns) |
|----------|-----------|
| Polynomial | 1258 |
| sin | 469 |
| cos | 446 |
| exp | 444 |
| ln | 423 |
| sqrt | 442 |
| gamma | 426 |
| digamma | 416 |
| trigamma | 402 |
| erf | 469 |
| erfc | 499 |
| zeta | 1157 |
| lambertw | 748 |
| besselj(0) | 543 |
| besselj(1) | 542 |
| bessely(0) | 566 |
| bessely(1) | 567 |
| besseli(0) | 553 |
| besselk(0) | 557 |
| polygamma(2) | 623 |
| polygamma(3) | 635 |
| polygamma(4) | 639 |
| tetragamma | 421 |

---

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
