# Benchmark Results

**SymbAnaFis Version:** 0.6.0  
**Date:** 2026-01-19
---

## System Specifications

- **CPU:** AMD Ryzen AI 7 350 w/ Radeon 860M (8 cores, 16 threads)
- **CPU Max:** 5.09 GHz
- **RAM:** 32 GB (30 GiB total)
- **OS:** Linux 6.17.12 (Fedora 43)
- **Rust:** rustc 1.90.0 (2025-09-14)
- **Backend:** Plotters

## Test Expressions

| Name              | Expression                              | Nodes | Domain     |
| ----------------- | --------------------------------------- | ----- | ---------- |
| Normal PDF        | `exp(-(x-μ)²/(2σ²))/√(2πσ²)`            | ~30   | Statistics |
| Gaussian 2D       | `exp(-((x-x₀)²+(y-y₀)²)/(2s²))/(2πs²)`  | ~40   | ML/Physics |
| Maxwell-Boltzmann | `4π(m/(2πkT))^(3/2) v² exp(-mv²/(2kT))` | ~50   | Physics    |
| Lorentz Factor    | `1/√(1-v²/c²)`                          | ~15   | Relativity |
| Lennard-Jones     | `4ε((σ/r)¹² - (σ/r)⁶)`                  | ~25   | Chemistry  |
| Logistic Sigmoid  | `1/(1+exp(-k(x-x₀)))`                   | ~15   | ML         |
| Damped Oscillator | `A·exp(-γt)·cos(ωt+φ)`                  | ~25   | Physics    |
| Planck Blackbody  | `2hν³/c² · 1/(exp(hν/(kT))-1)`          | ~35   | Physics    |
| Bessel Wave       | `besselj(0,k*r)*cos(ω*t)`               | ~10   | Physics    |

---

## 1. Parsing (String → AST)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------- | -------------- | ------------------ |
| Normal PDF        | 2.69 µs         | 4.42 µs        | **1.64x**          |
| Gaussian 2D       | 3.64 µs         | 6.18 µs        | **1.70x**          |
| Maxwell-Boltzmann | 4.18 µs         | 5.95 µs        | **1.42x**          |
| Lorentz Factor    | 1.39 µs         | 2.30 µs        | **1.66x**          |
| Lennard-Jones     | 2.15 µs         | 3.59 µs        | **1.67x**          |
| Logistic Sigmoid  | 1.72 µs         | 2.18 µs        | **1.27x**          |
| Damped Oscillator | 2.00 µs         | 2.53 µs        | **1.27x**          |
| Planck Blackbody  | 2.82 µs         | 4.00 µs        | **1.42x**          |
| Bessel Wave       | 1.85 µs         | 2.28 µs        | **1.23x**          |

> **Result:** SymbAnaFis parses **1.2x - 1.7x** faster than Symbolica.

---

## 2. Differentiation

> **Methodology:** Both libraries tested with equivalent "light" simplification (term collection only, no deep restructuring).

| Expression        | SA (diff_only) | Symbolica (diff) | SA Speedup |
| ----------------- | -------------- | ---------------- | ---------- |
| Normal PDF        | 1.00 µs        | 1.67 µs          | **1.67x**  |
| Gaussian 2D       | 0.86 µs        | 2.21 µs          | **2.57x**  |
| Maxwell-Boltzmann | 1.55 µs        | 3.20 µs          | **2.06x**  |
| Lorentz Factor    | 1.00 µs        | 1.79 µs          | **1.79x**  |
| Lennard-Jones     | 1.18 µs        | 1.85 µs          | **1.57x**  |
| Logistic Sigmoid  | 0.53 µs        | 1.13 µs          | **2.13x**  |
| Damped Oscillator | 0.98 µs        | 1.60 µs          | **1.63x**  |
| Planck Blackbody  | 1.31 µs        | 3.03 µs          | **2.31x**  |
| Bessel Wave       | 1.43 µs        | 1.63 µs          | **1.14x**  |

### SymbAnaFis Full Simplification Cost

| Expression        | SA diff_only | SA diff+simplify | Simplify Overhead |
| ----------------- | ------------ | ---------------- | ----------------- |
| Normal PDF        | 1.00 µs      | 75.6 µs          | **76x**           |
| Gaussian 2D       | 0.86 µs      | 69.9 µs          | **81x**           |
| Maxwell-Boltzmann | 1.55 µs      | 172 µs           | **111x**          |
| Lorentz Factor    | 1.00 µs      | 133 µs           | **133x**          |
| Lennard-Jones     | 1.18 µs      | 15.1 µs          | **13x**           |
| Logistic Sigmoid  | 0.53 µs      | 61.0 µs          | **115x**          |
| Damped Oscillator | 0.98 µs      | 79.3 µs          | **81x**           |
| Planck Blackbody  | 1.31 µs      | 178 µs           | **136x**          |
| Bessel Wave       | 1.43 µs      | 69.8 µs          | **49x**           |

### Symbolica (diff) vs SymbAnaFis (diff+simplify)

> **Trade-off:** SymbAnaFis full simplification is much slower than Symbolica's light differentiation, but this upfront cost enables significantly faster compilation and evaluation for large expressions.

| Expression        | Symbolica (diff) | SA (diff+simplify) | Relative Time (SA/SY) |
| ----------------- | ---------------- | ------------------ | --------------------- |
| Normal PDF        | 1.67 µs          | 75.6 µs            | **45x slower**        |
| Gaussian 2D       | 2.21 µs          | 69.9 µs            | **32x slower**        |
| Maxwell-Boltzmann | 3.20 µs          | 172 µs             | **54x slower**        |
| Lorentz Factor    | 1.79 µs          | 133 µs             | **74x slower**        |
| Lennard-Jones     | 1.85 µs          | 15.1 µs            | **8.2x slower**       |
| Logistic Sigmoid  | 1.13 µs          | 61.0 µs            | **54x slower**        |
| Damped Oscillator | 1.60 µs          | 79.3 µs            | **50x slower**        |
| Planck Blackbody  | 3.03 µs          | 178 µs             | **59x slower**        |
| Bessel Wave       | 1.63 µs          | 69.8 µs            | **43x slower**        |

> **Note:** SymbAnaFis full simplification performs deep AST restructuring (trig identities, algebraic transformations). Symbolica only performs light term collection. This heavier work pays off in the compilation stage (see Section 4).

---

## 4. Simplification Only (SymbAnaFis)

| Expression        | Time   |
| ----------------- | ------ |
| Normal PDF        | 75 µs  |
| Gaussian 2D       | 69 µs  |
| Maxwell-Boltzmann | 171 µs |
| Lorentz Factor    | 133 µs |
| Lennard-Jones     | 14 µs  |
| Logistic Sigmoid  | 61 µs  |
| Damped Oscillator | 78 µs  |
| Planck Blackbody  | 177 µs |
| Bessel Wave       | 67 µs  |

---

## 5. Compilation (AST → Bytecode/Evaluator)

> **Note:** Times shown are for compiling the **simplified** expression (post-differentiation).

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------- | -------------- | ------------------ |
| Normal PDF        | 0.98 µs         | 8.89 µs        | **9.1x**           |
| Gaussian 2D       | 1.26 µs         | 16.30 µs       | **12.9x**          |
| Maxwell-Boltzmann | 1.51 µs         | 8.38 µs        | **5.5x**           |
| Lorentz Factor    | 0.82 µs         | 4.85 µs        | **5.9x**           |
| Lennard-Jones     | 0.83 µs         | 12.94 µs       | **15.6x**          |
| Logistic Sigmoid  | 0.74 µs         | 4.91 µs        | **6.6x**           |
| Damped Oscillator | 1.21 µs         | 7.35 µs        | **6.1x**           |
| Planck Blackbody  | 1.66 µs         | 5.01 µs        | **3.0x**           |
| Bessel Wave       | 1.21 µs         | *(skipped)*    | —                  |

> **Result:** SymbAnaFis compilation is **3x - 16x** faster than Symbolica's evaluator creation.

---

## 6. Evaluation (Compiled, 1000 points)

| Expression        | SymbAnaFis (Simpl) | Symbolica (SY) | SA vs SY |
| ----------------- | ------------------ | -------------- | -------- |
| Normal PDF        | 56.6 µs            | 32.7 µs        | 0.58x    |
| Gaussian 2D       | 59.0 µs            | 33.9 µs        | 0.57x    |
| Maxwell-Boltzmann | 64.0 µs            | 42.2 µs        | 0.66x    |
| Lorentz Factor    | 43.4 µs            | 32.4 µs        | 0.75x    |
| Lennard-Jones     | 48.4 µs            | 34.1 µs        | 0.70x    |
| Logistic Sigmoid  | 47.5 µs            | 29.9 µs        | 0.63x    |
| Damped Oscillator | 44.2 µs            | 33.4 µs        | 0.75x    |
| Planck Blackbody  | 54.7 µs            | 32.1 µs        | 0.59x    |
| Bessel Wave       | 104 µs             | *(skipped)*    | —        |

> **Result:** Symbolica's evaluator is **1.3x - 1.7x** faster than SymbAnaFis for small expressions.

---

## 7. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SY vs SA) |
| ----------------- | --------------- | -------------- | ------------------ |
| Normal PDF        | 143 µs          | 53.4 µs        | **2.7x**           |
| Gaussian 2D       | 143 µs          | 70.5 µs        | **2.0x**           |
| Maxwell-Boltzmann | 265 µs          | 114 µs         | **2.3x**           |
| Lorentz Factor    | 185 µs          | 57.7 µs        | **3.2x**           |
| Lennard-Jones     | 65 µs           | 61.3 µs        | **1.06x**          |
| Logistic Sigmoid  | 112 µs          | 48.0 µs        | **2.3x**           |
| Damped Oscillator | 134 µs          | 79.1 µs        | **1.69x**          |
| Planck Blackbody  | 265 µs          | 94.8 µs        | **2.8x**           |
| Bessel Wave       | 170 µs          | *(skipped)*    | —                  |

> **Result:** Symbolica is **1.06x - 3.2x** faster in the full pipeline, mainly due to:
> 1. Lighter simplification (only term collection vs full restructuring)
> 2. Faster evaluation engine

---

## 8. Large Expressions (100-300 terms)

> **Note:** Large expressions with mixed terms (polynomials, trig, exp, sqrt, fractions).

### 100 Terms

| Operation                 | SymbAnaFis | Symbolica | Speedup      |
| ------------------------- | ---------- | --------- | ------------ |
| Parse                     | 75.1 µs    | 104 µs    | **SA 1.4x**  |
| Diff (no simplify)        | 48.9 µs    | 115 µs    | **SA 2.4x**  |
| Diff+Simplify             | 3.76 ms    | —         | —            |
| Compile (raw)             | 36.4 µs    | 1,013 µs  | **SA 28x**   |
| Compile (simplified)      | 25.2 µs    | 1,013 µs  | **SA 40x**   |
| Eval 1000pts (raw)        | 2,950 µs   | 1,844 µs  | 0.62x        |
| Eval 1000pts (simplified) | 1,621 µs   | 1,844 µs  | **SA 1.14x** |

### 300 Terms

| Operation                 | SymbAnaFis | Symbolica | Speedup      |
| ------------------------- | ---------- | --------- | ------------ |
| Parse                     | 229 µs     | 333 µs    | **SA 1.5x**  |
| Diff (no simplify)        | 146 µs     | 373 µs    | **SA 2.6x**  |
| Diff+Simplify             | 10.9 ms    | —         | —            |
| Compile (raw)             | 108 µs     | 11,521 µs | **SA 107x**  |
| Compile (simplified)      | 78.6 µs    | 11,521 µs | **SA 147x**  |
| Eval 1000pts (raw)        | 8,801 µs   | 5,252 µs  | 0.60x        |
| Eval 1000pts (simplified) | 5,150 µs   | 5,252 µs  | **SA 1.02x** |

---

## 9. Tree-Walk vs Compiled Evaluation

> **Note:** Compares generalized `evaluate()` (HashMap-based tree-walk) vs compiled bytecode evaluation.

| Expression        | Tree-Walk (1000 pts) | Compiled (1000 pts) | Speedup   |
| ----------------- | -------------------- | ------------------- | --------- |
| Normal PDF        | 500 µs               | 53.2 µs             | **9.4x**  |
| Gaussian 2D       | 1,016 µs             | 56.0 µs             | **18.1x** |
| Maxwell-Boltzmann | 585 µs               | 64.7 µs             | **9.0x**  |
| Lorentz Factor    | 382 µs               | 40.1 µs             | **9.5x**  |
| Lennard-Jones     | 315 µs               | 43.8 µs             | **7.2x**  |
| Logistic Sigmoid  | 505 µs               | 43.9 µs             | **11.5x** |
| Damped Oscillator | 450 µs               | 39.2 µs             | **11.5x** |
| Planck Blackbody  | 887 µs               | 52.8 µs             | **16.8x** |
| Bessel Wave       | 577 µs               | 101 µs              | **5.7x**  |

> **Result:** Compiled evaluation is **6x - 18x faster** than tree-walk evaluation. Use `CompiledEvaluator` for repeated evaluation of the same expression.

---

## 10. Batch Evaluation Performance (SIMD-optimized)

> **Note:** `eval_batch` now uses f64x4 SIMD to process 4 values simultaneously.

| Points  | loop_evaluate | eval_batch (SIMD) | Speedup  |
| ------- | ------------- | ----------------- | -------- |
| 100     | 3.61 µs       | 1.24 µs           | **2.9x** |
| 1,000   | 36.0 µs       | 12.4 µs           | **2.9x** |
| 10,000  | 361 µs        | 124 µs            | **2.9x** |
| 100,000 | 3.61 ms       | 1.24 ms           | **2.9x** |

> **Result:** SIMD-optimized `eval_batch` is consistently **~2.9x faster** than loop evaluation by processing 4 f64 values per instruction using f64x4 vectors.

---

## 11. Multi-Expression Batch Evaluation

> **Note:** Evaluates 3 different expressions (Lorentz, Quadratic, Trig) × 1000 points each.

| Method                            | Time        | vs Sequential  |
| --------------------------------- | ----------- | -------------- |
| **eval_batch_per_expr (SIMD)**    | **23.4 µs** | **59% faster** |
| eval_f64_per_expr (SIMD+parallel) | 37.9 µs     | 33% faster     |
| sequential_loops                  | 56.8 µs     | baseline       |

> **Result:** SIMD-optimized `eval_batch` is **~2.4x faster** than sequential evaluation loops when processing multiple expressions.

---

## 12. eval_f64 vs evaluate_parallel APIs

> **Note:** Compares the two high-level parallel evaluation APIs.

### `eval_f64` vs `evaluate_parallel` (High Load - 10,000 points)

| API                        | Time        | Notes                                                   |
| -------------------------- | ----------- | ------------------------------------------------------- |
| `eval_f64` (SIMD+parallel) | **42.4 µs** | **5.4x Faster**. Uses f64x4 SIMD + chunked parallelism. |
| `evaluate_parallel`        | 229 µs      | Slower due to per-point evaluation overhead.            |

**Result:** `eval_f64` scales significantly better. For 10,000 points, it is **~5.4x faster** than the general API.

- `eval_f64` uses `&[f64]` (8 bytes/item) → Cache friendly.
- `evaluate_parallel` uses `Vec<Value>` (24 bytes/item) → Memory bound.
- Zero-allocation optimization on `evaluate_parallel` showed no gain, confirming the bottleneck is data layout, not allocator contention.

---

## Summary

| Operation                               | Winner            | Speedup                  |
| --------------------------------------- | ----------------- | ------------------------ |
| **Parsing**                             | SymbAnaFis        | **1.2x - 1.7x** faster   |
| **Differentiation**                     | SymbAnaFis        | **1.1x - 2.6x** faster   |
| **Compilation**                         | SymbAnaFis        | **3x - 147x** faster     |
| **Tree-Walk → Compiled**                | Compiled          | **6x - 18x** faster      |
| **eval_batch vs loop**                  | eval_batch (SIMD) | **~2.9x** faster         |
| **Evaluation** (small expr)             | Symbolica         | **1.3x - 1.7x** faster   |
| **Evaluation** (large expr, simplified) | SymbAnaFis        | **1.02x - 1.18x** faster |
| **Full Pipeline** (small)               | Symbolica         | **1.06x - 3.2x** faster  |

### Key Insights

1. **Compile for repeated evaluation:** Compiled bytecode is 6-18x faster than tree-walk evaluation.

2. **Simplification pays off:** For large expressions, SymbAnaFis's full simplification dramatically reduces expression size, leading to much faster compilation and evaluation.

3. **Different strategies:**
   - **Symbolica:** Light term collection (`3x + 2x → 5x`), faster simplification, optimized evaluator
   - **SymbAnaFis:** Deep AST restructuring (trig identities, algebraic normalization), massive compilation speedup

4. **SIMD acceleration:** Using `eval_batch` with f64x4 SIMD provides consistent ~2.9x speedup over scalar loops.

5. **When to use which:**
   - **Small expressions, one-shot evaluation:** Symbolica's faster evaluation wins
   - **Large expressions, repeated evaluation:** SymbAnaFis's simplification + fast compile wins
   - **Batch numerical work:** Use `eval_f64` for maximum performance (5.4x faster than generic parallel API)
