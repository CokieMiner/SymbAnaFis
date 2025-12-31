# Benchmark Results

**SymbAnaFis Version:** 0.5.0  
**Date:** 2025-12-31

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
| Normal PDF        | 2.65 µs         | 4.37 µs        | **1.65x**          |
| Gaussian 2D       | 3.68 µs         | 6.21 µs        | **1.69x**          |
| Maxwell-Boltzmann | 4.13 µs         | 5.95 µs        | **1.44x**          |
| Lorentz Factor    | 1.35 µs         | 2.29 µs        | **1.70x**          |
| Lennard-Jones     | 2.23 µs         | 3.51 µs        | **1.57x**          |
| Logistic Sigmoid  | 1.73 µs         | 2.06 µs        | **1.19x**          |
| Damped Oscillator | 2.05 µs         | 2.42 µs        | **1.18x**          |
| Planck Blackbody  | 2.75 µs         | 3.96 µs        | **1.44x**          |
| Bessel Wave       | 1.89 µs         | 2.22 µs        | **1.17x**          |

> **Result:** SymbAnaFis parses **1.2x - 1.7x** faster than Symbolica.

---

## 2. Differentiation (Raw - No Simplification)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------- | -------------- | ------------------ |
| Normal PDF        | 1.00 µs         | 1.58 µs        | **1.58x**          |
| Gaussian 2D       | 0.85 µs         | 2.13 µs        | **2.51x**          |
| Maxwell-Boltzmann | 1.54 µs         | 3.14 µs        | **2.04x**          |
| Lorentz Factor    | 1.01 µs         | 1.78 µs        | **1.76x**          |
| Lennard-Jones     | 1.14 µs         | 1.82 µs        | **1.60x**          |
| Logistic Sigmoid  | 0.53 µs         | 1.11 µs        | **2.09x**          |
| Damped Oscillator | 1.01 µs         | 1.61 µs        | **1.59x**          |
| Planck Blackbody  | 1.31 µs         | 2.95 µs        | **2.25x**          |
| Bessel Wave       | 1.41 µs         | 1.62 µs        | **1.15x**          |

> **Result:** SymbAnaFis raw differentiation is **1.2x - 2.5x** faster.

---

## 3. Differentiation (Fair Comparison)

> **Methodology:** Both libraries tested with equivalent "light" simplification (term collection only, no deep restructuring).

| Expression        | SA (diff_only) | Symbolica (diff) | SA Speedup |
| ----------------- | -------------- | ---------------- | ---------- |
| Normal PDF        | 1.02 µs        | 1.57 µs          | **1.54x**  |
| Gaussian 2D       | 0.91 µs        | 2.12 µs          | **2.33x**  |
| Maxwell-Boltzmann | 1.54 µs        | 3.12 µs          | **2.03x**  |
| Lorentz Factor    | 1.00 µs        | 1.78 µs          | **1.78x**  |
| Lennard-Jones     | 1.23 µs        | 1.80 µs          | **1.46x**  |
| Logistic Sigmoid  | 0.53 µs        | 1.10 µs          | **2.08x**  |
| Damped Oscillator | 1.01 µs        | 1.59 µs          | **1.57x**  |
| Planck Blackbody  | 1.31 µs        | 2.94 µs          | **2.24x**  |
| Bessel Wave       | 1.39 µs        | 1.62 µs          | **1.17x**  |

### SymbAnaFis Full Simplification Cost

| Expression        | SA diff_only | SA diff+simplify | Simplify Overhead |
| ----------------- | ------------ | ---------------- | ----------------- |
| Normal PDF        | 1.02 µs      | 76 µs            | **75x**           |
| Gaussian 2D       | 0.91 µs      | 70 µs            | **77x**           |
| Maxwell-Boltzmann | 1.54 µs      | 172 µs           | **112x**          |
| Lorentz Factor    | 1.00 µs      | 134 µs           | **134x**          |
| Lennard-Jones     | 1.23 µs      | 15.1 µs          | **12x**           |
| Logistic Sigmoid  | 0.53 µs      | 61 µs            | **115x**          |
| Damped Oscillator | 1.01 µs      | 79 µs            | **78x**           |
| Planck Blackbody  | 1.31 µs      | 179 µs           | **137x**          |
| Bessel Wave       | 1.39 µs      | 68 µs            | **49x**           |

> **Note:** SymbAnaFis full simplification performs deep AST restructuring (trig identities, algebraic transformations). Symbolica only performs light term collection.

---

## 4. Simplification Only (SymbAnaFis)

| Expression        | Time   |
| ----------------- | ------ |
| Normal PDF        | 75 µs  |
| Gaussian 2D       | 68 µs  |
| Maxwell-Boltzmann | 171 µs |
| Lorentz Factor    | 132 µs |
| Lennard-Jones     | 14 µs  |
| Logistic Sigmoid  | 60 µs  |
| Damped Oscillator | 78 µs  |
| Planck Blackbody  | 176 µs |
| Bessel Wave       | 67 µs  |

---

## 5. Compilation (AST → Bytecode/Evaluator)

> **Note:** Times shown are for compiling the **simplified** expression (post-differentiation).

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------- | -------------- | ------------------ |
| Normal PDF        | 0.79 µs         | 8.65 µs        | **10.9x**          |
| Gaussian 2D       | 1.10 µs         | 16.1 µs        | **14.6x**          |
| Maxwell-Boltzmann | 1.10 µs         | 8.36 µs        | **7.6x**           |
| Lorentz Factor    | 0.63 µs         | 4.66 µs        | **7.4x**           |
| Lennard-Jones     | 0.64 µs         | 12.6 µs        | **19.7x**          |
| Logistic Sigmoid  | 0.79 µs         | 4.86 µs        | **6.2x**           |
| Damped Oscillator | 1.01 µs         | 7.33 µs        | **7.3x**           |
| Planck Blackbody  | 1.64 µs         | 4.88 µs        | **3.0x**           |
| Bessel Wave       | 0.98 µs         | *(skipped)*    | —                  |

> **Result:** SymbAnaFis compilation is **3x - 20x** faster than Symbolica's evaluator creation.

---

## 6. Evaluation (Compiled, 1000 points)

| Expression        | SymbAnaFis (Simpl) | Symbolica (SY) | SA vs SY |
| ----------------- | ------------------ | -------------- | -------- |
| Normal PDF        | 53.6 µs            | 32.7 µs        | 0.61x    |
| Gaussian 2D       | 57.4 µs            | 33.8 µs        | 0.59x    |
| Maxwell-Boltzmann | 70.3 µs            | 41.9 µs        | 0.60x    |
| Lorentz Factor    | 41.6 µs            | 32.3 µs        | 0.78x    |
| Lennard-Jones     | 48.4 µs            | 34.4 µs        | 0.71x    |
| Logistic Sigmoid  | 75.5 µs            | 30.0 µs        | 0.40x    |
| Damped Oscillator | 45.4 µs            | 33.0 µs        | 0.73x    |
| Planck Blackbody  | 87.7 µs            | 32.0 µs        | 0.37x    |
| Bessel Wave       | 90.9 µs            | *(skipped)*    | —        |

> **Result:** Symbolica's evaluator is **1.3x - 2.7x** faster than SymbAnaFis for small expressions.

---

## 7. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SY vs SA) |
| ----------------- | --------------- | -------------- | ------------------ |
| Normal PDF        | 140 µs          | 52.6 µs        | **2.7x**           |
| Gaussian 2D       | 142 µs          | 71.0 µs        | **2.0x**           |
| Maxwell-Boltzmann | 262 µs          | 113 µs         | **2.3x**           |
| Lorentz Factor    | 183 µs          | 57.5 µs        | **3.2x**           |
| Lennard-Jones     | 65 µs           | 60.8 µs        | **1.07x**          |
| Logistic Sigmoid  | 128 µs          | 47.6 µs        | **2.7x**           |
| Damped Oscillator | 134 µs          | 79.1 µs        | **1.69x**          |
| Planck Blackbody  | 288 µs          | 95.4 µs        | **3.0x**           |
| Bessel Wave       | 160 µs          | *(skipped)*    | —                  |

> **Result:** Symbolica is **1.07x - 3.2x** faster in the full pipeline, mainly due to:
> 1. Lighter simplification (only term collection vs full restructuring)
> 2. Faster evaluation engine

---

## 8. Large Expressions (100-300 terms)

> **Note:** Large expressions with mixed terms (polynomials, trig, exp, sqrt, fractions).

### 100 Terms

| Operation                 | SymbAnaFis | Symbolica | Speedup      |
| ------------------------- | ---------- | --------- | ------------ |
| Parse                     | 74.8 µs    | 103 µs    | **SA 1.4x**  |
| Diff (no simplify)        | 47.8 µs    | 113 µs    | **SA 2.4x**  |
| Diff+Simplify             | 3.78 ms    | —         | —            |
| Compile (simplified)      | 15.7 µs    | 1,018 µs  | **SA 65x**   |
| Eval 1000pts (simplified) | 1,602 µs   | 1,844 µs  | **SA 1.15x** |

### 300 Terms

| Operation                 | SymbAnaFis | Symbolica | Speedup      |
| ------------------------- | ---------- | --------- | ------------ |
| Parse                     | 229 µs     | 329 µs    | **SA 1.4x**  |
| Diff (no simplify)        | 144 µs     | 369 µs    | **SA 2.6x**  |
| Diff+Simplify             | 11.0 ms    | —         | —            |
| Compile (simplified)      | 47.1 µs    | 12,217 µs | **SA 259x**  |
| Eval 1000pts (simplified) | 5,175 µs   | 5,286 µs  | **SA 1.02x** |

---

## 9. Tree-Walk vs Compiled Evaluation

> **Note:** Compares generalized `evaluate()` (HashMap-based tree-walk) vs compiled bytecode evaluation.

| Expression        | Tree-Walk (1000 pts) | Compiled (1000 pts) | Speedup   |
| ----------------- | -------------------- | ------------------- | --------- |
| Normal PDF        | 501 µs               | 50.5 µs             | **9.9x**  |
| Gaussian 2D       | 996 µs               | 54.5 µs             | **18.3x** |
| Maxwell-Boltzmann | 596 µs               | 56.4 µs             | **10.6x** |
| Lorentz Factor    | 385 µs               | 39.2 µs             | **9.8x**  |
| Lennard-Jones     | 319 µs               | 41.7 µs             | **7.7x**  |
| Logistic Sigmoid  | 501 µs               | 72.5 µs             | **6.9x**  |
| Damped Oscillator | 446 µs               | 35.8 µs             | **12.5x** |
| Planck Blackbody  | 892 µs               | 69.9 µs             | **12.8x** |
| Bessel Wave       | 571 µs               | 78.3 µs             | **7.3x**  |

> **Result:** Compiled evaluation is **7x - 18x faster** than tree-walk evaluation. Use `CompiledEvaluator` for repeated evaluation of the same expression.

---

## 10. Batch Evaluation Performance (SIMD-optimized)

> **Note:** `eval_batch` now uses f64x4 SIMD to process 4 values simultaneously.

| Points  | loop_evaluate | eval_batch (SIMD) | Speedup  |
| ------- | ------------- | ----------------- | -------- |
| 100     | 3.51 µs       | 1.21 µs           | **2.9x** |
| 1,000   | 35.1 µs       | 12.2 µs           | **2.9x** |
| 10,000  | 350 µs        | 122 µs            | **2.9x** |
| 100,000 | 3.52 ms       | 1.23 ms           | **2.9x** |

> **Result:** SIMD-optimized `eval_batch` is consistently **~2.9x faster** than loop evaluation by processing 4 f64 values per instruction using f64x4 vectors.

---

## 11. Multi-Expression Batch Evaluation

> **Note:** Evaluates 3 different expressions (Lorentz, Quadratic, Trig) × 1000 points each.

| Method                            | Time        | vs Sequential  |
| --------------------------------- | ----------- | -------------- |
| **eval_batch_per_expr (SIMD)**    | **22.6 µs** | **58% faster** |
| eval_f64_per_expr (SIMD+parallel) | 37.0 µs     | 31% faster     |
| sequential_loops                  | 53.1 µs     | baseline       |

> **Result:** SIMD-optimized `eval_batch` is **~2.4x faster** than sequential evaluation loops when processing multiple expressions.

---

## 12. eval_f64 vs evaluate_parallel APIs

> **Note:** Compares the two high-level parallel evaluation APIs.

### `eval_f64` vs `evaluate_parallel` (High Load - 10,000 points)

| API                        | Time        | Notes                                                   |
| -------------------------- | ----------- | ------------------------------------------------------- |
| `eval_f64` (SIMD+parallel) | **62.1 µs** | **3.7x Faster**. Uses f64x4 SIMD + chunked parallelism. |
| `evaluate_parallel`        | 229 µs      | Slower due to per-point evaluation overhead.            |

**Result:** `eval_f64` scales significantly better. For 10,000 points, it is **~3.7x faster** than the general API.
- `eval_f64` uses `&[f64]` (8 bytes/item) → Cache friendly.
- `evaluate_parallel` uses `Vec<Value>` (24 bytes/item) → Memory bound.
- Zero-allocation optimization on `evaluate_parallel` showed no gain, confirming the bottleneck is data layout, not allocator contention.

---

## Summary

| Operation                               | Winner            | Speedup                  |
| --------------------------------------- | ----------------- | ------------------------ |
| **Parsing**                             | SymbAnaFis        | **1.2x - 1.7x** faster   |
| **Differentiation**                     | SymbAnaFis        | **1.2x - 2.5x** faster   |
| **Compilation**                         | SymbAnaFis        | **3x - 259x** faster     |
| **Tree-Walk → Compiled**                | Compiled          | **7x - 18x** faster      |
| **eval_batch vs loop**                  | eval_batch (SIMD) | **~2.9x** faster         |
| **Evaluation** (small expr)             | Symbolica         | **1.3x - 2.7x** faster   |
| **Evaluation** (large expr, simplified) | SymbAnaFis        | **1.02x - 1.15x** faster |
| **Full Pipeline** (small)               | Symbolica         | **1.07x - 3.2x** faster  |

### Key Insights

1. **Compile for repeated evaluation:** Compiled bytecode is 7-18x faster than tree-walk evaluation.

2. **Simplification pays off:** For large expressions, SymbAnaFis's full simplification dramatically reduces expression size, leading to much faster compilation and evaluation.

3. **Different strategies:**
   - **Symbolica:** Light term collection (`3x + 2x → 5x`), faster simplification, optimized evaluator
   - **SymbAnaFis:** Deep AST restructuring (trig identities, algebraic normalization), massive compilation speedup

4. **SIMD acceleration:** Using `eval_batch` with f64x4 SIMD provides consistent ~2.9x speedup over scalar loops.

5. **When to use which:**
   - **Small expressions, one-shot evaluation:** Symbolica's faster evaluation wins
   - **Large expressions, repeated evaluation:** SymbAnaFis's simplification + fast compile wins
   - **Batch numerical work:** Use `eval_f64` for maximum performance (3.7x faster than generic parallel API)
