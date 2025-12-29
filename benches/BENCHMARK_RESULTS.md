# Benchmark Results

**SymbAnaFis Version:** 0.4.1  
**Date:** 2025-12-29

## System Specifications

- **CPU:** AMD Ryzen AI 7 350 w/ Radeon 860M (8 cores, 16 threads)
- **CPU Max:** 5.09 GHz
- **RAM:** 32 GB (30 GiB total)
- **OS:** Linux 6.17.12 (Fedora 43)
- **Rust:** rustc 1.90.0 (2025-09-14)
- **Backend:** Plotters

## Test Expressions

| Name | Expression | Nodes | Domain |
|------|------------|-------|--------|
| Normal PDF | `exp(-(x-μ)²/(2σ²))/√(2πσ²)` | ~30 | Statistics |
| Gaussian 2D | `exp(-((x-x₀)²+(y-y₀)²)/(2s²))/(2πs²)` | ~40 | ML/Physics |
| Maxwell-Boltzmann | `4π(m/(2πkT))^(3/2) v² exp(-mv²/(2kT))` | ~50 | Physics |
| Lorentz Factor | `1/√(1-v²/c²)` | ~15 | Relativity |
| Lennard-Jones | `4ε((σ/r)¹² - (σ/r)⁶)` | ~25 | Chemistry |
| Logistic Sigmoid | `1/(1+exp(-k(x-x₀)))` | ~15 | ML |
| Damped Oscillator | `A·exp(-γt)·cos(ωt+φ)` | ~25 | Physics |
| Planck Blackbody | `2hν³/c² · 1/(exp(hν/(kT))-1)` | ~35 | Physics |
| Bessel Wave | `besselj(0,k*r)*cos(ω*t)` | ~10 | Physics |

---

## 1. Parsing (String → AST)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 2.70 µs | 4.39 µs | **1.63x** |
| Gaussian 2D | 3.73 µs | 6.22 µs | **1.67x** |
| Maxwell-Boltzmann | 4.17 µs | 6.03 µs | **1.45x** |
| Lorentz Factor | 1.41 µs | 2.27 µs | **1.61x** |
| Lennard-Jones | 2.18 µs | 3.59 µs | **1.65x** |
| Logistic Sigmoid | 1.72 µs | 2.12 µs | **1.23x** |
| Damped Oscillator | 2.05 µs | 2.48 µs | **1.21x** |
| Planck Blackbody | 2.73 µs | 4.00 µs | **1.47x** |
| Bessel Wave | 1.83 µs | 2.23 µs | **1.22x** |

> **Result:** SymbAnaFis parses **1.2x - 1.7x** faster than Symbolica.

---

## 2. Differentiation (Raw - No Simplification)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 1.03 µs | 1.57 µs | **1.52x** |
| Gaussian 2D | 0.87 µs | 2.12 µs | **2.44x** |
| Maxwell-Boltzmann | 1.60 µs | 3.15 µs | **1.97x** |
| Lorentz Factor | 1.13 µs | 1.79 µs | **1.58x** |
| Lennard-Jones | 1.21 µs | 1.81 µs | **1.50x** |
| Logistic Sigmoid | 0.54 µs | 1.10 µs | **2.04x** |
| Damped Oscillator | 1.05 µs | 1.60 µs | **1.52x** |
| Planck Blackbody | 1.32 µs | 2.96 µs | **2.24x** |
| Bessel Wave | 1.45 µs | 1.62 µs | **1.12x** |

> **Result:** SymbAnaFis raw differentiation is **1.1x - 2.4x** faster.

---

## 3. Differentiation (Fair Comparison)

> **Methodology:** Both libraries tested with equivalent "light" simplification (term collection only, no deep restructuring).

| Expression | SA (diff_only) | Symbolica (diff) | SA Speedup |
|------------|----------------|------------------|------------|
| Normal PDF | 1.04 µs | 1.58 µs | **1.52x** |
| Gaussian 2D | 0.88 µs | 2.14 µs | **2.43x** |
| Maxwell-Boltzmann | 1.61 µs | 3.16 µs | **1.96x** |
| Lorentz Factor | 1.01 µs | 1.79 µs | **1.77x** |
| Lennard-Jones | 1.22 µs | 1.82 µs | **1.49x** |
| Logistic Sigmoid | 0.55 µs | 1.10 µs | **2.00x** |
| Damped Oscillator | 1.07 µs | 1.60 µs | **1.50x** |
| Planck Blackbody | 1.33 µs | 2.95 µs | **2.22x** |
| Bessel Wave | 1.45 µs | 1.63 µs | **1.12x** |

### SymbAnaFis Full Simplification Cost

| Expression | SA diff_only | SA diff+simplify | Simplify Overhead |
|------------|--------------|------------------|-------------------|
| Normal PDF | 1.04 µs | 79 µs | **76x** |
| Gaussian 2D | 0.88 µs | 71 µs | **81x** |
| Maxwell-Boltzmann | 1.61 µs | 175 µs | **109x** |
| Lorentz Factor | 1.01 µs | 135 µs | **134x** |
| Lennard-Jones | 1.22 µs | 15.4 µs | **13x** |
| Logistic Sigmoid | 0.55 µs | 62 µs | **113x** |
| Damped Oscillator | 1.07 µs | 80 µs | **75x** |
| Planck Blackbody | 1.33 µs | 180 µs | **135x** |
| Bessel Wave | 1.45 µs | 69 µs | **48x** |

> **Note:** SymbAnaFis full simplification performs deep AST restructuring (trig identities, algebraic transformations). Symbolica only performs light term collection.

---

## 4. Simplification Only (SymbAnaFis)

| Expression | Time |
|------------|------|
| Normal PDF | 75 µs |
| Gaussian 2D | 69 µs |
| Maxwell-Boltzmann | 172 µs |
| Lorentz Factor | 132 µs |
| Lennard-Jones | 14 µs |
| Logistic Sigmoid | 63 µs |
| Damped Oscillator | 79 µs |
| Planck Blackbody | 178 µs |
| Bessel Wave | 67 µs |

---

## 5. Compilation (AST → Bytecode/Evaluator)

> **Note:** Times shown are for compiling the **simplified** expression (post-differentiation).

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 0.69 µs | 8.89 µs | **12.9x** |
| Gaussian 2D | 0.88 µs | 16.4 µs | **18.6x** |
| Maxwell-Boltzmann | 0.94 µs | 8.55 µs | **9.1x** |
| Lorentz Factor | 0.55 µs | 4.84 µs | **8.8x** |
| Lennard-Jones | 0.56 µs | 13.0 µs | **23.2x** |
| Logistic Sigmoid | 0.71 µs | 5.01 µs | **7.1x** |
| Damped Oscillator | 0.84 µs | 7.69 µs | **9.2x** |
| Planck Blackbody | 1.46 µs | 5.09 µs | **3.5x** |
| Bessel Wave | 0.82 µs | *(skipped)* | — |

> **Result:** SymbAnaFis compilation is **3.5x - 23x** faster than Symbolica's evaluator creation.

---

## 6. Evaluation (Compiled, 1000 points)

| Expression | SymbAnaFis (Simpl) | Symbolica (SY) | SA vs SY |
|------------|--------------------|----------------|----------|
| Normal PDF | 53.7 µs | 33.3 µs | 0.62x |
| Gaussian 2D | 57.7 µs | 34.4 µs | 0.60x |
| Maxwell-Boltzmann | 68.8 µs | 42.8 µs | 0.62x |
| Lorentz Factor | 41.8 µs | 32.4 µs | 0.78x |
| Lennard-Jones | 47.8 µs | 35.0 µs | 0.73x |
| Logistic Sigmoid | 75.9 µs | 29.9 µs | 0.39x |
| Damped Oscillator | 45.4 µs | 34.0 µs | 0.75x |
| Planck Blackbody | 84.5 µs | 33.1 µs | 0.39x |
| Bessel Wave | 86.2 µs | *(skipped)* | — |

> **Result:** Symbolica's evaluator is **1.3x - 2.6x** faster than SymbAnaFis for small expressions.

---

## 7. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SY vs SA) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 139 µs | 53.9 µs | **2.6x** |
| Gaussian 2D | 143 µs | 71.4 µs | **2.0x** |
| Maxwell-Boltzmann | 267 µs | 115 µs | **2.3x** |
| Lorentz Factor | 184 µs | 58.0 µs | **3.2x** |
| Lennard-Jones | 65 µs | 61.5 µs | **1.06x** |
| Logistic Sigmoid | 128 µs | 47.8 µs | **2.7x** |
| Damped Oscillator | 133 µs | 80.3 µs | **1.66x** |
| Planck Blackbody | 293 µs | 97.0 µs | **3.0x** |
| Bessel Wave | 162 µs | *(skipped)* | — |

> **Result:** Symbolica is **1.06x - 3.2x** faster in the full pipeline, mainly due to:
> 1. Lighter simplification (only term collection vs full restructuring)
> 2. Faster evaluation engine

---

## 8. Large Expressions (100-300 terms)

> **Note:** Large expressions with mixed terms (polynomials, trig, exp, sqrt, fractions).

### 100 Terms

| Operation | SymbAnaFis | Symbolica | Speedup |
|-----------|------------|-----------|---------| 
| Parse | 74.5 µs | 105 µs | **SA 1.4x** |
| Diff (no simplify) | 48.8 µs | 114 µs | **SA 2.3x** |
| Diff+Simplify | 3.82 ms | — | — |
| Compile (simplified) | 15.6 µs | 1,042 µs | **SA 67x** |
| Eval 1000pts (simplified) | 1,599 µs | 1,868 µs | **SA 1.17x** |

### 300 Terms

| Operation | SymbAnaFis | Symbolica | Speedup |
|-----------|------------|-----------|---------| 
| Parse | 231 µs | 336 µs | **SA 1.5x** |
| Diff (no simplify) | 147 µs | 376 µs | **SA 2.6x** |
| Diff+Simplify | 11.8 ms | — | — |
| Compile (simplified) | 47.5 µs | 13,823 µs | **SA 291x** |
| Eval 1000pts (simplified) | 5,209 µs | 5,292 µs | **SA 1.02x** |

---

## 9. Tree-Walk vs Compiled Evaluation

> **Note:** Compares generalized `evaluate()` (HashMap-based tree-walk) vs compiled bytecode evaluation.

| Expression | Tree-Walk (1000 pts) | Compiled (1000 pts) | Speedup |
|------------|----------------------|---------------------|---------|
| Normal PDF | 505 µs | 50.7 µs | **10.0x** |
| Gaussian 2D | 1,000 µs | 54.4 µs | **18.4x** |
| Maxwell-Boltzmann | 599 µs | 56.3 µs | **10.6x** |
| Lorentz Factor | 387 µs | 39.2 µs | **9.9x** |
| Lennard-Jones | 322 µs | 41.8 µs | **7.7x** |
| Logistic Sigmoid | 512 µs | 72.7 µs | **7.0x** |
| Damped Oscillator | 465 µs | 36.3 µs | **12.8x** |
| Planck Blackbody | 910 µs | 67.4 µs | **13.5x** |
| Bessel Wave | 584 µs | 75.5 µs | **7.7x** |

> **Result:** Compiled evaluation is **7x - 18x faster** than tree-walk evaluation. Use `CompiledEvaluator` for repeated evaluation of the same expression.

---

## 10. Batch Evaluation Performance (SIMD-optimized)

> **Note:** `eval_batch` now uses f64x4 SIMD to process 4 values simultaneously.

| Points | loop_evaluate | eval_batch (SIMD) | Speedup |
|--------|---------------|-------------------|---------|
| 100 | 3.50 µs | 1.21 µs | **2.9x** |
| 1,000 | 35.0 µs | 12.2 µs | **2.9x** |
| 10,000 | 350 µs | 122 µs | **2.9x** |
| 100,000 | 3.50 ms | 1.22 ms | **2.9x** |

> **Result:** SIMD-optimized `eval_batch` is consistently **~2.9x faster** than loop evaluation by processing 4 f64 values per instruction using f64x4 vectors.

---

## 11. Multi-Expression Batch Evaluation

> **Note:** Evaluates 3 different expressions (Lorentz, Quadratic, Trig) × 1000 points each.

| Method | Time | vs Sequential |
|--------|------|---------------|
| **eval_batch_per_expr (SIMD)** | **22.6 µs** | **58% faster** |
| eval_f64_per_expr (SIMD+parallel) | 34.5 µs | 35% faster |
| sequential_loops | 53.3 µs | baseline |

> **Result:** SIMD-optimized `eval_batch` is **~2.4x faster** than sequential evaluation loops when processing multiple expressions.

---

## 12. eval_f64 vs evaluate_parallel APIs

> **Note:** Compares the two high-level parallel evaluation APIs.

### `eval_f64` vs `evaluate_parallel` (High Load - 10,000 points)

| API | Time | Notes |
|-----|------|-------|
| `eval_f64` (SIMD+parallel) | **37.4 µs** | **6.1x Faster**. Uses f64x4 SIMD + chunked parallelism. |
| `evaluate_parallel` | 230 µs | Slower due to per-point evaluation overhead. |

**Result:** `eval_f64` scales significantly better. For 10,000 points, it is **~6.1x faster** than the general API.
- `eval_f64` uses `&[f64]` (8 bytes/item) → Cache friendly.
- `evaluate_parallel` uses `Vec<Value>` (24 bytes/item) → Memory bound.
- Zero-allocation optimization on `evaluate_parallel` showed no gain, confirming the bottleneck is data layout, not allocator contention.

---

## Summary

| Operation | Winner | Speedup |
|-----------|--------|---------|
| **Parsing** | SymbAnaFis | **1.2x - 1.7x** faster |
| **Differentiation** | SymbAnaFis | **1.1x - 2.4x** faster |
| **Compilation** | SymbAnaFis | **3.5x - 291x** faster |
| **Tree-Walk → Compiled** | Compiled | **7x - 18x** faster |
| **eval_batch vs loop** | eval_batch (SIMD) | **~2.9x** faster |
| **Evaluation** (small expr) | Symbolica | **1.3x - 2.6x** faster |
| **Evaluation** (large expr, simplified) | SymbAnaFis | **1.02x - 1.17x** faster |
| **Full Pipeline** (small) | Symbolica | **1.06x - 3.2x** faster |

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
   - **Batch numerical work:** Use `eval_f64` for maximum performance (6x faster than generic parallel API)
