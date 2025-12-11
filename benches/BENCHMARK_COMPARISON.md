# SymbAnaFis vs SymPy vs Symbolica Benchmark Comparison

**Date:** December 10, 2025 (Updated)
**SymbAnaFis Version:** 0.3.0
**SymPy Version:** Latest (Python 3)
**Symbolica Version:** 1.0.1
**System:** Linux
**Criterion Version:** 0.8.1

## Summary

SymbAnaFis is a symbolic mathematics library written in Rust, designed for parsing, differentiation, and simplification of mathematical expressions. This document compares its performance against SymPy and Symbolica.

### Key Findings vs SymPy

| Category | Winner | Speedup Range |
|----------|--------|---------------|
| **Parsing** | SymbAnaFis | **120x - 190x faster** |
| **Differentiation (AST)** | SymbAnaFis | **28x - 154x faster** |
| **Differentiation (Full)** | Mixed | **2.1x - 6.8x faster** (except Complex: -1.5x) |
| **Simplification** | SymbAnaFis | **35x - 297x faster** |
| **Combined Diff + Simplify** | SymbAnaFis | **45x - 90x faster** |
| **Evaluation** | SymbAnaFis | **32x - 3886x faster** |

### Key Findings vs Symbolica

| Category | Winner | Notes |
|----------|--------|-------|
| **Parsing** | SymbAnaFis | **1.5x - 2.3x faster** |
| **Differentiation (AST only)** | SymbAnaFis | **1.7x - 2.9x faster** |
| **Differentiation (Full)** | Symbolica | **17x - 73x faster** |
| **Evaluation** | N/A | Symbolica uses compiled evaluators (different approach) |

---

## Detailed Results

### 1. Parsing (String → Expression)

SymbAnaFis uses a custom recursive descent parser that is orders of magnitude faster than `sympify` and faster than Symbolica's parser.

| Expression | SymbAnaFis (µs) | Symbolica (µs) | SymPy (µs) | vs Symbolica | vs SymPy |
|------------|-----------------|----------------|------------|--------------|----------|
| Polynomial `x^3+...` | 0.84 | 1.41 | 133.00 | **1.7x** | **158x** |
| Trig `sin(x)*cos(x)` | 0.59 | 1.14 | 107.96 | **1.9x** | **183x** |
| Complex `x^2*sin(x)*exp(x)` | 1.04 | 1.47 | 116.66 | **1.4x** | **112x** |
| Nested `sin(cos(tan(x)))` | 0.54 | 1.23 | 101.70 | **2.3x** | **188x** |

### 2. Differentiation (AST Only)

Pure differentiation speed on pre-parsed expressions **without simplification**. This measures raw differentiation engine speed.

| Expression | SymbAnaFis (µs) | Symbolica (µs) | SymPy (µs) | vs Symbolica | vs SymPy |
|------------|-----------------|----------------|------------|--------------|----------|
| Polynomial | 0.43 | 1.25 | 24.98 | **2.9x** | **58x** |
| Trig | 0.39 | 1.04 | 20.02 | **2.7x** | **51x** |
| Complex | 0.72 | 1.49 | 19.21 | **2.1x** | **27x** |
| Nested Trig | 0.54 | 0.93 | 83.69 | **1.7x** | **155x** |

### 3. Differentiation (Full Pipeline)

Includes parsing and automatic simplification. Both SymbAnaFis and Symbolica return simplified results.

| Expression | SymbAnaFis (µs) | Symbolica (µs) | SymPy (µs) | vs Symbolica | vs SymPy |
|------------|-----------------|----------------|------------|--------------|----------|
| Polynomial | 46.4 | 2.74 | 150.19 | **-17x** | **3.2x** |
| Trig `sin(x)cos(x)` | 61.3 | 2.22 | 130.15 | **-28x** | **2.1x** |
| Chain `sin(x^2)` | 28.6 | 1.51 | 189.81 | **-19x** | **6.6x** |
| Exp `exp(x^2)` | 28.2 | 1.47 | 190.42 | **-19x** | **6.8x** |
| Complex | 216.0 | 2.95 | 146.02 | **-73x** | **-1.5x** |
| Quotient `(x^2+1)/(x-1)` | 92.7 | 2.79 | 152.71 | **-33x** | **1.6x** |
| Nested | 71.9 | 2.22 | 207.31 | **-32x** | **2.9x** |
| Power `x^x` | 33.7 | 1.59 | 184.07 | **-21x** | **5.5x** |

### 4. Simplification

SymbAnaFis provides extensive rule-based simplification with pattern matching.

| Expression | SymbAnaFis (µs) | SymPy (µs) | Speedup |
|------------|-----------------|------------|---------|
| Pythagorean `sin^2+cos^2` | 17.8 | 5282 | **297x** |
| Perfect Square | 20.0 | 2146 | **107x** |
| Fraction Cancel | 19.8 | 1334 | **67x** |
| Exp Combine `e^x*e^y` | 22.1 | 1631 | **74x** |
| Like Terms `2x+3x+x` | 17.9 | 631 | **35x** |
| Hyperbolic `(e^x-e^-x)/2` | 26.1 | 3727 | **143x** |
| Frac Add `(x^2+1)/(...)...`| 144.5 | 3624 | **25x** |
| Power Combine | 18.0 | 634 | **35x** |

### 5. Combined Operations

Real-world scenarios often require differentiating and then simplifying the result.

| Operation | SymbAnaFis (µs) | SymPy (µs) | Speedup |
|-----------|-----------------|------------|---------|
| `d/dx[sin(x)^2]` simplified | 34.3 | 3090 | **90x** |
| `d/dx[(x^2+1)/(x-1)]` simplified | 147.2 | 6674 | **45x** |

### 6. Evaluation (Expression → Number)

Numerical evaluation of pre-parsed expressions at x = 2.5. Both libraries produce matching results.

| Function | SymbAnaFis (ns) | SymPy (µs) | Speedup | Result |
|----------|-----------------|------------|---------|--------|
| Polynomial `x^3+2x^2+x+1` | 110 | 33.3 | **303x** | 31.625 |
| `sin(x)` | 50 | 8.3 | **166x** | 0.5984721441 |
| `cos(x)` | 48 | 7.8 | **163x** | -0.8011436155 |
| `gamma(x)` | 59 | 23.7 | **402x** | 1.3293403882 |
| `digamma(x)` | 51 | 48.0 | **941x** | 0.7031566394 |
| `trigamma(x)` | 45 | 145.1 | **3224x** | 0.4903577576 |
| `polygamma(2, x)` | 120 | 147.6 | **1230x** | -0.2362040516 |
| `polygamma(3, x)` | 120 | 152.4 | **1270x** | 0.2239058488 |
| `polygamma(4, x)` | 123 | 154.5 | **1256x** | -0.3137559995 |
| `besselj(0, x)` | 59 | 37.4 | **634x** | -0.0483837758 |
| `besselj(1, x)` | 60 | 37.4 | **623x** | 0.4970941025 |
| `bessely(0, x)` | 63 | 165.0 | **2619x** | 0.4980703584 |
| `bessely(1, x)` | 65 | 252.6 | **3886x** | 0.1459181375 |
| `zeta(x)` | 731 | 23.7 | **32x** | 1.3414972364 |
| `erf(x)` | 67 | 24.4 | **364x** | 0.9995930479 |
| `lambertw(x)` | 99 | 24.9 | **251x** | 0.9585863567 |

> [!NOTE]
> SymbAnaFis uses direct native Rust implementations for special functions, while SymPy uses
> Python's arbitrary-precision arithmetic via `evalf()`. This explains the 100x-4000x speedup.

#### Unique Capabilities

**SymbAnaFis** can numerically evaluate zeta derivatives that **SymPy cannot**:

| Function | SymbAnaFis (µs) | Result | SymPy |
|----------|-----------------|--------|-------|
| `zeta_deriv(1, 2.5)` | 1.97 | -0.3859406642 | N/A |
| `zeta_deriv(2, 2.5)` | 1.98 | 0.5735024089 | N/A |
| `zeta_deriv(3, 2.5)` | 2.01 | -1.1327791776 | N/A |

---

## Analysis: SymbAnaFis vs Symbolica

### Why Symbolica is Faster for Full Differentiation

Both libraries use AST-based representations (`Num`, `Var`, `Fun`, `Mul`, `Add`, `Pow`), but Symbolica employs several low-level optimizations:

| Optimization | Symbolica | SymbAnaFis |
|--------------|-----------|------------|
| **Memory Representation** | Compact `Vec<u8>` with type tags | `Arc<Symbol>` with heap allocation |
| **Workspace** | Thread-local memory pool | Fresh allocations per operation |
| **Normalization** | Lightweight inline normalization | Multi-pass rule-based simplification |
| **Data Layout** | Cache-friendly byte arrays | Pointer-chasing through `Arc` |

### Simplification Philosophy

The key difference is in **simplification strategy**:

- **Symbolica**: Uses a lightweight `normalize()` function that combines like terms and performs basic algebraic simplification during derivative construction.

- **SymbAnaFis**: Uses an extensible **rule-based simplification engine** that applies many pattern-matching rules over multiple passes. This is more powerful for complex identities (trigonometric, hyperbolic) but slower for basic operations.

### Where SymbAnaFis Excels

1. **Parsing**: 1.5-2.3x faster - simpler AST construction without byte packing
2. **AST Differentiation**: 1.7-2.9x faster - direct tree manipulation
3. **Trigonometric Identities**: Extensive patterns (sin²+cos²=1, double angles, etc.)
4. **Hyperbolic Functions**: sinh, cosh, tanh recognition and simplification
5. **Custom Functions**: First-class support for user-defined functions with derivatives
6. **Extensibility**: Rule-based engine can be extended with new patterns

### Where Symbolica Excels

1. **Full Differentiation Pipeline**: 15-20x faster due to memory optimizations
2. **Polynomial Operations**: Native multivariate polynomial factorization
3. **Large Expressions**: Coefficient ring optimizations and streaming
4. **Series Expansion**: Built-in Taylor/Laurent series
5. **Pattern Matching**: Powerful wildcard-based pattern matching

### Where SymbAnaFis Struggles

**Polynomial division and rational functions** are a weak point:

| Expression | vs Symbolica | vs SymPy | Notes |
|------------|--------------|----------|-------|
| Complex `x^2*sin(x)*exp(x)` | **-73x** | **-1.5x** | Division in derivative |
| Quotient `(x^2+1)/(x-1)` | **-33x** | **1.6x** | Explicit division |

The quotient rule produces expressions like `(2x(x-1) - (x²+1))/(x-1)²` which then require:

1. Expanding the numerator polynomial
2. Combining like terms
3. Handling polynomial division/cancellation
4. Normalizing the fraction

Symbolica has native polynomial coefficient rings and optimized rational function handling. SymbAnaFis applies pattern-matching rules iteratively rather than using specialized polynomial algorithms. This is a target for future optimization.

---

## Analysis: Why SymbAnaFis Beats SymPy

1. **Rule-based engine with ExprKind filtering**: O(1) rule lookup instead of O(n) scanning
2. **No Python overhead**: Pure Rust with zero-cost abstractions
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
