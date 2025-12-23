# Benchmark Results

**SymbAnaFis Version:** 0.4.0  
**Date:** 2025-12-22  
**System:** Linux (Plotters Backend)

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
| Normal PDF | 2.61 µs | 4.45 µs | **1.70x** |
| Gaussian 2D | 3.58 µs | 6.28 µs | **1.75x** |
| Maxwell-Boltzmann | 4.10 µs | 6.02 µs | **1.47x** |
| Lorentz Factor | 1.38 µs | 2.33 µs | **1.69x** |
| Lennard-Jones | 1.97 µs | 3.57 µs | **1.81x** |
| Logistic Sigmoid | 1.79 µs | 2.13 µs | **1.19x** |
| Damped Oscillator | 2.01 µs | 2.47 µs | **1.23x** |
| Planck Blackbody | 2.76 µs | 3.99 µs | **1.45x** |
| Bessel Wave | 1.80 µs | 2.23 µs | **1.24x** |

> **Result:** SymbAnaFis parses **1.2x - 1.8x** faster than Symbolica.

---

## 2. Differentiation (Raw - No Simplification)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 1.23 µs | 1.57 µs | **1.28x** |
| Gaussian 2D | 1.20 µs | 2.13 µs | **1.78x** |
| Maxwell-Boltzmann | 1.61 µs | 3.14 µs | **1.95x** |
| Lorentz Factor | 1.11 µs | 1.77 µs | **1.59x** |
| Lennard-Jones | 1.15 µs | 1.83 µs | **1.59x** |
| Logistic Sigmoid | 0.60 µs | 1.09 µs | **1.82x** |
| Damped Oscillator | 1.01 µs | 1.60 µs | **1.58x** |
| Planck Blackbody | 1.58 µs | 2.98 µs | **1.89x** |
| Bessel Wave | 1.55 µs | 1.61 µs | **1.04x** |

> **Result:** SymbAnaFis raw differentiation is **1.1x - 2.0x** faster.

---

## 3. Differentiation (Fair Comparison)

> **Methodology:** Both libraries tested with equivalent "light" simplification (term collection only, no deep restructuring).

| Expression | SA (diff_only) | Symbolica (diff) | SA Speedup |
|------------|----------------|------------------|------------|
| Normal PDF | 1.23 µs | 1.58 µs | **1.28x** |
| Gaussian 2D | 1.19 µs | 2.12 µs | **1.78x** |
| Maxwell-Boltzmann | 1.61 µs | 3.15 µs | **1.96x** |
| Lorentz Factor | 1.12 µs | 1.77 µs | **1.58x** |
| Lennard-Jones | 1.17 µs | 1.83 µs | **1.56x** |
| Logistic Sigmoid | 0.61 µs | 1.09 µs | **1.79x** |
| Damped Oscillator | 1.03 µs | 1.59 µs | **1.54x** |
| Planck Blackbody | 1.59 µs | 2.96 µs | **1.86x** |
| Bessel Wave | 1.42 µs | 1.61 µs | **1.13x** |

### SymbAnaFis Full Simplification Cost

| Expression | SA diff_only | SA diff+simplify | Simplify Overhead |
|------------|--------------|------------------|-------------------|
| Normal PDF | 1.23 µs | 163 µs | **133x** |
| Gaussian 2D | 1.19 µs | 115 µs | **97x** |
| Maxwell-Boltzmann | 1.61 µs | 202 µs | **125x** |
| Lorentz Factor | 1.12 µs | 187 µs | **167x** |
| Lennard-Jones | 1.17 µs | 112 µs | **96x** |
| Logistic Sigmoid | 0.61 µs | 56 µs | **92x** |
| Damped Oscillator | 1.03 µs | 82 µs | **80x** |
| Planck Blackbody | 1.59 µs | 257 µs | **162x** |
| Bessel Wave | 1.42 µs | 82 µs | **58x** |

> **Note:** SymbAnaFis full simplification performs deep AST restructuring (trig identities, algebraic transformations). Symbolica only performs light term collection.

---

## 4. Simplification Only (SymbAnaFis)

| Expression | Time |
|------------|------|
| Normal PDF | 163 µs |
| Gaussian 2D | 114 µs |
| Maxwell-Boltzmann | 200 µs |
| Lorentz Factor | 186 µs |
| Lennard-Jones | 111 µs |
| Logistic Sigmoid | 56 µs |
| Damped Oscillator | 81 µs |
| Planck Blackbody | 254 µs |
| Bessel Wave | 81 µs |

---

## 5. Compilation (AST → Bytecode/Evaluator)

> **Note:** Times shown are for compiling the **simplified** expression (post-differentiation).

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 0.54 µs | 8.78 µs | **16.3x** |
| Gaussian 2D | 0.78 µs | 16.3 µs | **20.9x** |
| Maxwell-Boltzmann | 0.84 µs | 8.39 µs | **10.0x** |
| Lorentz Factor | 0.47 µs | 4.77 µs | **10.1x** |
| Lennard-Jones | 0.49 µs | 12.8 µs | **26.1x** |
| Logistic Sigmoid | 0.48 µs | 4.89 µs | **10.2x** |
| Damped Oscillator | 0.77 µs | 7.44 µs | **9.7x** |
| Planck Blackbody | 1.34 µs | 4.99 µs | **3.7x** |

> **Result:** SymbAnaFis compilation is **3.6x - 25x** faster than Symbolica's evaluator creation.

---

## 6. Evaluation (Compiled, 1000 points)

| Expression | SymbAnaFis (Simpl) | Symbolica (SY) | SA vs SY |
|------------|--------------------|----------------|--------------------| 
| Normal PDF | 75 µs | 33 µs | 0.44x |
| Gaussian 2D | 77 µs | 34 µs | 0.44x |
| Maxwell-Boltzmann | 89 µs | 42 µs | 0.47x |
| Lorentz Factor | 40 µs | 32 µs | 0.80x |
| Lennard-Jones | 53 µs | 35 µs | **0.66x** |
| Logistic Sigmoid | 71 µs | 30 µs | 0.42x |
| Damped Oscillator | 49 µs | 33 µs | 0.67x |
| Planck Blackbody | 143 µs | 32 µs | 0.22x |
| Bessel Wave | 73 µs | *(skipped)* | — |

> **Result:** With SIMD (f64x4), SymbAnaFis closed the gap to **0.4x - 0.8x** of Symbolica's speed. The **Lorentz Factor** benchmark now runs at **80%** of Symbolica's performance!

---

## 7. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SY vs SA) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 256 µs | 53 µs | **4.8x** |
| Gaussian 2D | 207 µs | 70 µs | **3.0x** |
| Maxwell-Boltzmann | 314 µs | 113 µs | **2.8x** |
| Lorentz Factor | 239 µs | 57 µs | **4.2x** |
| Lennard-Jones | 177 µs | 61 µs | **2.9x** |
| Logistic Sigmoid | 128 µs | 48 µs | **2.7x** |
| Damped Oscillator | 149 µs | 80 µs | **1.9x** |
| Planck Blackbody | 431 µs | 95 µs | **4.5x** |
| Bessel Wave | 164 µs | *(skipped)* | — |

> **Result:** Symbolica is **1.9x - 4.8x** faster in the full pipeline, mainly due to:
> 1. Lighter simplification (only term collection vs full restructuring)
> 2. Faster evaluation engine

---

## 8. Large Expressions (100-300 terms)

> **Note:** Large expressions with mixed terms (polynomials, trig, exp, log, fractions).

### 100 Terms

| Operation | SymbAnaFis | Symbolica | Speedup |
|-----------|------------|-----------|---------|
| Parse | 75 µs | 107 µs | **SA 1.4x** |
| Diff (no simplify) | 45 µs | 110 µs | **SA 2.5x** |
| Compile (simplified) | 11.6 µs | 1,018 µs | **SA 88x** |
| Eval 1000pts (simplified) | 1,983 µs | 1,471 µs | **SY 1.3x** |

### 300 Terms

| Operation | SymbAnaFis | Symbolica | Speedup |
|-----------|------------|-----------|---------|
| Parse | 227 µs | 340 µs | **SA 1.5x** |
| Diff (no simplify) | 133 µs | 335 µs | **SA 2.5x** |
| Compile (simplified) | 35.8 µs | 11,712 µs | **SA 327x** |
| Eval 1000pts (simplified) | 6,046 µs | 4,080 µs | **SY 1.5x** |

> **Key Insight:** After SymbAnaFis's full simplification, compilation is dramatically faster (88x-327x). Symbolica's evaluator is still faster for large expressions (1.3x-1.5x), but the gap narrowed significantly after inlining optimizations (~12% improvement).

---

## 9. Tree-Walk vs Compiled Evaluation

> **Note:** Compares generalized `evaluate()` (HashMap-based tree-walk) vs compiled bytecode evaluation.

| Expression | Tree-Walk (1000 pts) | Compiled (1000 pts) | Speedup |
|------------|----------------------|---------------------|---------|
| Normal PDF | 504 µs | 75 µs | **6.7x** |
| Gaussian 2D | 997 µs | 77 µs | **12.9x** |
| Maxwell-Boltzmann | 604 µs | 89 µs | **6.8x** |
| Lorentz Factor | 388 µs | 40 µs | **9.7x** |
| Lennard-Jones | 319 µs | 53 µs | **6.0x** |
| Logistic Sigmoid | 386 µs | 71 µs | **5.4x** |
| Damped Oscillator | 464 µs | 49 µs | **9.5x** |
| Planck Blackbody | 942 µs | 143 µs | **6.6x** |
| Bessel Wave | 580 µs | 73 µs | **7.9x** |

> **Result:** Compiled evaluation is **5x - 13x faster** than tree-walk evaluation. Use `CompiledEvaluator` for repeated evaluation of the same expression.

---

## 10. Batch Evaluation Performance (SIMD-optimized)

> **Note:** `eval_batch` now uses f64x4 SIMD to process 4 values simultaneously.

| Points | loop_evaluate | eval_batch (SIMD) | Speedup |
|--------|---------------|-------------------|---------|
| 100 | 3.71 µs | 1.23 µs | **3.0x** |
| 1,000 | 36.8 µs | 12.4 µs | **3.0x** |
| 10,000 | 366 µs | 124 µs | **3.0x** |
| 100,000 | 3.67 ms | 1.23 ms | **3.0x** |

> **Result:** SIMD-optimized `eval_batch` is now **~3x faster** than loop evaluation by processing 4 f64 values per instruction using f64x4 vectors.

---

## 11. Multi-Expression Batch Evaluation

> **Note:** Evaluates 3 different expressions (Lorentz, Quadratic, Trig) × 1000 points each.

| Method | Time | vs Sequential |
|--------|------|---------------|
| **eval_batch_per_expr (SIMD)** | **22.8 µs** | **65% faster** |
| eval_f64_per_expr (SIMD+parallel) | 34.0 µs | 47% faster |
| sequential_loops | 64.4 µs | baseline |

> **Result:** SIMD-optimized `eval_batch` is **~2.8x faster** than sequential evaluation loops when processing multiple expressions.

---

## 12. eval_f64 vs evaluate_parallel APIs

> **Note:** Compares the two high-level parallel evaluation APIs.

### `eval_f64` vs `evaluate_parallel` (High Load - 10,000 points)

| API | Time | Notes |
|-----|------|-------|
| `eval_f64` (SIMD+parallel) | **36 µs** | **6.4x Faster**. Uses f64x4 SIMD + chunked parallelism. |
| `evaluate_parallel` | 230 µs | Slower due to per-point evaluation overhead. |

**Result:** `eval_f64` scales significantly better. For 10,000 points, it is **~4.0x faster** than the general API.
- `eval_f64` uses `&[f64]` (8 bytes/item) -> Cache friendly.
- `evaluate_parallel` uses `Vec<Value>` (24 bytes/item) -> Memory bound.
- Zero-allocation optimization on `evaluate_parallel` showed no gain, confirming the bottleneck is data layout, not allocator contention.

---

## Summary

| Operation | Winner | Speedup |
|-----------|--------|---------|
| **Parsing** | SymbAnaFis | **1.2x - 1.8x** faster |
| **Differentiation** | SymbAnaFis | **1.1x - 2.5x** faster |
| **Compilation** | SymbAnaFis | **3.7x - 311x** faster |
| **Tree-Walk → Compiled** | Compiled | **5x - 13x** faster |
| **eval_batch vs loop** | eval_batch | **~16%** faster |
| **Evaluation** (small expr) | Symbolica | **1.4x - 4.5x** faster |
| **Evaluation** (large expr, simplified) | Symbolica | **1.3x - 1.5x** faster |
| **Full Pipeline** (small) | Symbolica | **1.9x - 4.8x** faster |

### Key Insights

1. **Compile for repeated evaluation:** Compiled bytecode is 5-13x faster than tree-walk evaluation.

2. **Simplification pays off:** For large expressions, SymbAnaFis's full simplification dramatically reduces expression size, leading to much faster compilation and evaluation.

3. **Different strategies:**
   - **Symbolica:** Light term collection (`3x + 2x → 5x`), faster simplification, optimized evaluator
   - **SymbAnaFis:** Deep AST restructuring (trig identities, algebraic normalization), massive compilation speedup

4. **Batch evaluation helps:** Using `eval_batch` provides ~16% speedup over calling `evaluate()` in a loop.

5. **When to use which:**
   - **Small expressions, one-shot evaluation:** Symbolica's faster evaluation wins
   - **Large expressions, repeated evaluation:** SymbAnaFis's simplification + fast compile wins
```
