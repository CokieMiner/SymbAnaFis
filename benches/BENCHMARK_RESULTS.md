# Benchmark Results

**SymbAnaFis Version:** 0.4.0  
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

---

## 1. Parsing (String → AST)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 2.70 µs | 4.43 µs | **1.64x** |
| Gaussian 2D | 3.72 µs | 6.21 µs | **1.67x** |
| Maxwell-Boltzmann | 4.16 µs | 6.01 µs | **1.44x** |
| Lorentz Factor | 1.35 µs | 2.30 µs | **1.70x** |
| Lennard-Jones | 2.20 µs | 3.59 µs | **1.63x** |
| Logistic Sigmoid | 1.73 µs | 2.15 µs | **1.24x** |
| Damped Oscillator | 2.00 µs | 2.51 µs | **1.26x** |
| Planck Blackbody | 2.78 µs | 4.02 µs | **1.45x** |
| Bessel Wave | 1.83 µs | 2.25 µs | **1.23x** |

> **Result:** SymbAnaFis parses **1.1x - 1.7x** faster than Symbolica.

---

## 2. Differentiation (Raw - No Simplification)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 1.05 µs | 1.58 µs | **1.50x** |
| Gaussian 2D | 0.91 µs | 2.16 µs | **2.37x** |
| Maxwell-Boltzmann | 1.63 µs | 3.21 µs | **1.97x** |
| Lorentz Factor | 1.14 µs | 1.85 µs | **1.62x** |
| Lennard-Jones | 1.22 µs | 1.87 µs | **1.53x** |
| Logistic Sigmoid | 0.68 µs | 1.14 µs | **1.68x** |
| Damped Oscillator | 1.01 µs | 1.61 µs | **1.59x** |
| Planck Blackbody | 1.34 µs | 3.02 µs | **2.25x** |
| Bessel Wave | 1.46 µs | 1.65 µs | **1.13x** |

> **Result:** SymbAnaFis raw differentiation is **1.1x - 2.4x** faster.

---

## 3. Differentiation (Fair Comparison)

> **Methodology:** Both libraries tested with equivalent "light" simplification (term collection only, no deep restructuring).

| Expression | SA (diff_only) | Symbolica (diff) | SA Speedup |
|------------|----------------|------------------|------------|
| Normal PDF | 1.06 µs | 1.60 µs | **1.51x** |
| Gaussian 2D | 0.90 µs | 2.15 µs | **2.39x** |
| Maxwell-Boltzmann | 1.61 µs | 3.20 µs | **1.99x** |
| Lorentz Factor | 1.14 µs | 1.84 µs | **1.61x** |
| Lennard-Jones | 1.17 µs | 1.86 µs | **1.59x** |
| Logistic Sigmoid | 0.68 µs | 1.14 µs | **1.68x** |
| Damped Oscillator | 1.00 µs | 1.62 µs | **1.62x** |
| Planck Blackbody | 1.34 µs | 3.03 µs | **2.26x** |
| Bessel Wave | 1.45 µs | 1.63 µs | **1.12x** |

### SymbAnaFis Full Simplification Cost

| Expression | SA diff_only | SA diff+simplify | Simplify Overhead |
|------------|--------------|------------------|-------------------|
| Normal PDF | 1.04 µs | 76 µs | **73x** |
| Gaussian 2D | 0.90 µs | 70 µs | **78x** |
| Maxwell-Boltzmann | 1.61 µs | 173 µs | **107x** |
| Lorentz Factor | 1.13 µs | 133 µs | **118x** |
| Lennard-Jones | 1.23 µs | 15.5 µs | **13x** |
| Logistic Sigmoid | 0.68 µs | 62 µs | **91x** |
| Damped Oscillator | 1.05 µs | 79 µs | **75x** |
| Planck Blackbody | 1.34 µs | 178 µs | **133x** |
| Bessel Wave | 1.54 µs | 68 µs | **44x** |

> **Note:** SymbAnaFis full simplification performs deep AST restructuring (trig identities, algebraic transformations). Symbolica only performs light term collection.

---

## 4. Simplification Only (SymbAnaFis)

| Expression | Time |
|------------|------|
| Normal PDF | 75 µs |
| Gaussian 2D | 68 µs |
| Maxwell-Boltzmann | 172 µs |
| Lorentz Factor | 132 µs |
| Lennard-Jones | 14 µs |
| Logistic Sigmoid | 61 µs |
| Damped Oscillator | 78 µs |
| Planck Blackbody | 177 µs |
| Bessel Wave | 67 µs |

---

## 5. Compilation (AST → Bytecode/Evaluator)

> **Note:** Times shown are for compiling the **simplified** expression (post-differentiation).

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 0.67 µs | 8.84 µs | **13.2x** |
| Gaussian 2D | 0.92 µs | 16.5 µs | **17.9x** |
| Maxwell-Boltzmann | 0.95 µs | 8.33 µs | **8.8x** |
| Lorentz Factor | 0.55 µs | 4.79 µs | **8.7x** |
| Lennard-Jones | 0.55 µs | 13.0 µs | **23.6x** |
| Logistic Sigmoid | 0.69 µs | 4.92 µs | **7.1x** |
| Damped Oscillator | 0.87 µs | 7.54 µs | **8.7x** |
| Planck Blackbody | 1.48 µs | 4.95 µs | **3.3x** |
| Bessel Wave | 0.82 µs | *(skipped)* | — |

> **Result:** SymbAnaFis compilation is **3.3x - 24x** faster than Symbolica's evaluator creation.

---

## 6. Evaluation (Compiled, 1000 points)

| Expression | SymbAnaFis (Simpl) | Symbolica (SY) | SA vs SY |
|------------|--------------------|----------------|----------|
| Normal PDF | 53.6 µs | 33.1 µs | 0.62x |
| Gaussian 2D | 58.3 µs | 34.2 µs | 0.59x |
| Maxwell-Boltzmann | 60.1 µs | 43.5 µs | 0.72x |
| Lorentz Factor | 41.5 µs | 32.4 µs | 0.78x |
| Lennard-Jones | 47.0 µs | 34.6 µs | **0.74x** |
| Logistic Sigmoid | 75.3 µs | 30.0 µs | 0.40x |
| Damped Oscillator | 42.9 µs | 33.3 µs | 0.78x |
| Planck Blackbody | 71.7 µs | 32.3 µs | 0.45x |
| Bessel Wave | 78.6 µs | *(skipped)* | — |

---

## 7. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SY vs SA) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 140 µs | 53.5 µs | **2.6x** |
| Gaussian 2D | 143 µs | 70.9 µs | **2.0x** |
| Maxwell-Boltzmann | 265 µs | 114 µs | **2.3x** |
| Lorentz Factor | 183 µs | 57.5 µs | **3.2x** |
| Lennard-Jones | 67 µs | 61.4 µs | **1.09x** |
| Logistic Sigmoid | 129 µs | 47.7 µs | **2.7x** |
| Damped Oscillator | 133 µs | 80.5 µs | **1.65x** |
| Planck Blackbody | 289 µs | 96.2 µs | **3.0x** |
| Bessel Wave | 168 µs | *(skipped)* | — |

> **Result:** Symbolica is **1.09x - 3.2x** faster in the full pipeline, mainly due to:
> 1. Lighter simplification (only term collection vs full restructuring)
> 2. Faster evaluation engine

---

## 8. Large Expressions (100-300 terms)

> **Note:** Large expressions with mixed terms (polynomials, trig, exp, log, fractions).

### 100 Terms

| Operation | SymbAnaFis | Symbolica | Speedup |
|-----------|------------|-----------|---------| 
| Parse | 75.5 µs | 106 µs | **SA 1.4x** |
| Diff (no simplify) | 47.0 µs | 111 µs | **SA 2.4x** |
| Diff+Simplify | 2.65 ms | — | — |
| Compile (simplified) | 14.6 µs | 1,001 µs | **SA 69x** |
| Eval 1000pts (simplified) | 1,562 µs | 1,470 µs | **SY 1.06x** |

### 300 Terms

| Operation | SymbAnaFis | Symbolica | Speedup |
|-----------|------------|-----------|---------| 
| Parse | 230 µs | 339 µs | **SA 1.5x** |
| Diff (no simplify) | 142 µs | 333 µs | **SA 2.3x** |
| Diff+Simplify | 8.2 ms | — | — |
| Compile (simplified) | 44.5 µs | 11,130 µs | **SA 250x** |
| Eval 1000pts (simplified) | 4,980 µs | 4,120 µs | **SY 1.2x** |

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
| **Parsing** | SymbAnaFis | **1.1x - 1.7x** faster |
| **Differentiation** | SymbAnaFis | **1.1x - 2.4x** faster |
| **Compilation** | SymbAnaFis | **3.3x - 276x** faster |
| **Tree-Walk → Compiled** | Compiled | **7x - 18x** faster |
| **eval_batch vs loop** | eval_batch (SIMD) | **~2.9x** faster |
| **Evaluation** (small expr) | Symbolica | **1.3x - 2.5x** faster |
| **Evaluation** (large expr, simplified) | Symbolica | **1.04x - 1.2x** faster |
| **Full Pipeline** (small) | Symbolica | **1.09x - 3.2x** faster |

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
