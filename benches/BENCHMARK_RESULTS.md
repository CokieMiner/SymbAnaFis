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
| Normal PDF | 2.63 µs | 4.49 µs | **1.71x** |
| Gaussian 2D | 3.57 µs | 6.29 µs | **1.76x** |
| Maxwell-Boltzmann | 4.09 µs | 6.10 µs | **1.49x** |
| Lorentz Factor | 1.33 µs | 2.32 µs | **1.74x** |
| Lennard-Jones | 1.97 µs | 3.57 µs | **1.81x** |
| Logistic Sigmoid | 1.67 µs | 2.20 µs | **1.32x** |
| Damped Oscillator | 2.06 µs | 2.52 µs | **1.22x** |
| Planck Blackbody | 2.78 µs | 4.06 µs | **1.46x** |
| Bessel Wave | 1.83 µs | 2.27 µs | **1.24x** |

> **Result:** SymbAnaFis parses **1.2x - 1.8x** faster than Symbolica.

---

## 2. Differentiation (Raw - No Simplification)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 1.26 µs | 1.59 µs | **1.26x** |
| Gaussian 2D | 1.21 µs | 2.15 µs | **1.78x** |
| Maxwell-Boltzmann | 1.62 µs | 3.18 µs | **1.96x** |
| Lorentz Factor | 1.11 µs | 1.78 µs | **1.60x** |
| Lennard-Jones | 1.14 µs | 1.86 µs | **1.63x** |
| Logistic Sigmoid | 0.60 µs | 1.11 µs | **1.85x** |
| Damped Oscillator | 1.01 µs | 1.59 µs | **1.57x** |
| Planck Blackbody | 1.57 µs | 2.98 µs | **1.90x** |
| Bessel Wave | 1.43 µs | 1.63 µs | **1.14x** |

> **Result:** SymbAnaFis raw differentiation is **1.1x - 2.0x** faster.

---

## 3. Differentiation (Fair Comparison)

> **Methodology:** Both libraries tested with equivalent "light" simplification (term collection only, no deep restructuring).

| Expression | SA (diff_only) | Symbolica (diff) | SA Speedup |
|------------|----------------|------------------|------------|
| Normal PDF | 1.23 µs | 1.59 µs | **1.29x** |
| Gaussian 2D | 1.22 µs | 2.14 µs | **1.75x** |
| Maxwell-Boltzmann | 1.61 µs | 3.15 µs | **1.96x** |
| Lorentz Factor | 1.10 µs | 1.79 µs | **1.63x** |
| Lennard-Jones | 1.17 µs | 1.85 µs | **1.58x** |
| Logistic Sigmoid | 0.61 µs | 1.11 µs | **1.82x** |
| Damped Oscillator | 1.01 µs | 1.60 µs | **1.58x** |
| Planck Blackbody | 1.58 µs | 2.97 µs | **1.88x** |
| Bessel Wave | 1.42 µs | 1.62 µs | **1.14x** |

### SymbAnaFis Full Simplification Cost

| Expression | SA diff_only | SA diff+simplify | Simplify Overhead |
|------------|--------------|------------------|-------------------|
| Normal PDF | 1.23 µs | 164 µs | **133x** |
| Gaussian 2D | 1.22 µs | 115 µs | **94x** |
| Maxwell-Boltzmann | 1.61 µs | 203 µs | **126x** |
| Lorentz Factor | 1.10 µs | 185 µs | **168x** |
| Lennard-Jones | 1.17 µs | 112 µs | **96x** |
| Logistic Sigmoid | 0.61 µs | 56 µs | **92x** |
| Damped Oscillator | 1.01 µs | 81 µs | **80x** |
| Planck Blackbody | 1.58 µs | 254 µs | **161x** |
| Bessel Wave | 1.42 µs | 82 µs | **58x** |

> **Note:** SymbAnaFis full simplification performs deep AST restructuring (trig identities, algebraic transformations). Symbolica only performs light term collection.

---

## 4. Simplification Only (SymbAnaFis)

| Expression | Time |
|------------|------|
| Normal PDF | 163 µs |
| Gaussian 2D | 113 µs |
| Maxwell-Boltzmann | 200 µs |
| Lorentz Factor | 184 µs |
| Lennard-Jones | 111 µs |
| Logistic Sigmoid | 55 µs |
| Damped Oscillator | 80 µs |
| Planck Blackbody | 252 µs |
| Bessel Wave | 80 µs |

---

## 5. Compilation (AST → Bytecode/Evaluator)

> **Note:** Times shown are for compiling the **simplified** expression (post-differentiation).

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 0.56 µs | 8.75 µs | **15.6x** |
| Gaussian 2D | 0.74 µs | 16.0 µs | **21.6x** |
| Maxwell-Boltzmann | 0.84 µs | 8.31 µs | **9.9x** |
| Lorentz Factor | 0.48 µs | 4.72 µs | **9.8x** |
| Lennard-Jones | 0.50 µs | 12.7 µs | **25.4x** |
| Logistic Sigmoid | 0.50 µs | 4.89 µs | **9.8x** |
| Damped Oscillator | 0.78 µs | 7.42 µs | **9.5x** |
| Planck Blackbody | 1.39 µs | 4.99 µs | **3.6x** |

> **Result:** SymbAnaFis compilation is **3.6x - 25x** faster than Symbolica's evaluator creation.

---

## 6. Evaluation (Compiled, 1000 points)

| Expression | SymbAnaFis (Simpl) | Symbolica (SY) | Speedup (SY vs SA) |
|------------|--------------------|----------------|--------------------| 
| Normal PDF | 81 µs | 33 µs | **2.5x** |
| Gaussian 2D | 80 µs | 35 µs | **2.3x** |
| Maxwell-Boltzmann | 91 µs | 47 µs | **1.9x** |
| Lorentz Factor | 44 µs | 32 µs | **1.4x** |
| Lennard-Jones | 54 µs | 34 µs | **1.6x** |
| Logistic Sigmoid | 65 µs | 30 µs | **2.2x** |
| Damped Oscillator | 55 µs | 33 µs | **1.7x** |
| Planck Blackbody | 144 µs | 32 µs | **4.5x** |
| Bessel Wave | 69 µs | *(skipped)* | — |

> **Result:** Symbolica's evaluator is **1.4x - 4.5x** faster at runtime execution.

---

## 7. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SY vs SA) |
|------------|-----------------|----------------|--------------------| 
| Normal PDF | 256 µs | 53 µs | **4.8x** |
| Gaussian 2D | 207 µs | 70 µs | **3.0x** |
| Maxwell-Boltzmann | 312 µs | 113 µs | **2.8x** |
| Lorentz Factor | 240 µs | 58 µs | **4.1x** |
| Lennard-Jones | 176 µs | 61 µs | **2.9x** |
| Logistic Sigmoid | 128 µs | 48 µs | **2.7x** |
| Damped Oscillator | 148 µs | 80 µs | **1.9x** |
| Planck Blackbody | 431 µs | 96 µs | **4.5x** |
| Bessel Wave | 163 µs | *(skipped)* | — |

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
| Diff (no simplify) | 45 µs | 111 µs | **SA 2.5x** |
| Compile (simplified) | 1.0 µs | 1,009 µs | **SA 1000x** |
| Eval 1000pts (simplified) | 257 µs | 1,487 µs | **SA 5.8x** |

### 300 Terms

| Operation | SymbAnaFis | Symbolica | Speedup |
|-----------|------------|-----------|---------|
| Parse | 230 µs | 347 µs | **SA 1.5x** |
| Diff (no simplify) | 135 µs | 333 µs | **SA 2.5x** |
| Compile (simplified) | 1.0 µs | 12,924 µs | **SA 12,500x** |
| Eval 1000pts (simplified) | 261 µs | 4,150 µs | **SA 16x** |

> **Key Insight:** After SymbAnaFis's full simplification, the expression becomes much simpler, leading to dramatically faster compilation (1000x-12,500x) and evaluation (6x-16x) compared to Symbolica's unsimplified derivative.

---

## 9. Tree-Walk vs Compiled Evaluation

> **Note:** Compares generalized `evaluate()` (HashMap-based tree-walk) vs compiled bytecode evaluation.

| Expression | Tree-Walk (1000 pts) | Compiled (1000 pts) | Speedup |
|------------|----------------------|---------------------|---------|
| Normal PDF | 509 µs | 79 µs | **6.4x** |
| Gaussian 2D | 1,008 µs | 80 µs | **12.6x** |
| Maxwell-Boltzmann | 608 µs | 92 µs | **6.6x** |
| Lorentz Factor | 394 µs | 42 µs | **9.4x** |
| Lennard-Jones | 318 µs | 62 µs | **5.1x** |
| Logistic Sigmoid | 388 µs | 73 µs | **5.3x** |
| Damped Oscillator | 467 µs | 51 µs | **9.2x** |
| Planck Blackbody | 942 µs | 152 µs | **6.2x** |
| Bessel Wave | 582 µs | 72 µs | **8.1x** |

> **Result:** Compiled evaluation is **5x - 13x faster** than tree-walk evaluation. Use `CompiledEvaluator` for repeated evaluation of the same expression.

---

## 10. Batch Evaluation Performance (eval_batch vs loop)

> **Note:** Compares `eval_batch` (loop inside VM) vs calling `evaluate()` in a loop.

| Points | loop_evaluate | eval_batch | Speedup |
|--------|---------------|------------|---------|
| 100 | 3.77 µs | 3.27 µs | **13%** |
| 1,000 | 37.6 µs | 32.7 µs | **13%** |
| 10,000 | 375 µs | 326 µs | **13%** |
| 100,000 | 3.74 ms | 3.26 ms | **13%** |

> **Result:** `eval_batch` provides a consistent **~13% speedup** by moving the evaluation loop inside the VM, reducing function call overhead.

---

## 11. Multi-Expression Batch Evaluation

> **Note:** Evaluates 3 different expressions (Lorentz, Quadratic, Trig) × 1000 points each.

| Method | Time | vs Sequential |
|--------|------|---------------|
| **eval_batch_per_expr** | **51.5 µs** | **21% faster** |
| eval_f64_per_expr | 52.3 µs | 20% faster |
| sequential_loops | 65.5 µs | baseline |

> **Result:** `eval_batch` is **~21% faster** than sequential evaluation loops when processing multiple expressions.

---

## 12. eval_f64 vs evaluate_parallel APIs

> **Note:** Compares the two high-level parallel evaluation APIs.

### `eval_f64` vs `evaluate_parallel` (High Load - 10,000 points)

| API | Time | Notes |
|-----|------|-------|
| `eval_f64` | **63 µs** | **3.7x Faster**. Data fits in L2/L3 cache (packed f64). |
| `evaluate_parallel` | 233 µs | Slower due to `Value` enum overhead (3x memory usage) and cache misses. |

**Result:** `eval_f64` scales significantly better. For 10,000 points, it is **~3.7x faster** than the general API.
- `eval_f64` uses `&[f64]` (8 bytes/item) -> Cache friendly.
- `evaluate_parallel` uses `Vec<Value>` (24 bytes/item) -> Memory bound.
- Zero-allocation optimization on `evaluate_parallel` showed no gain, confirming the bottleneck is data layout, not allocator contention.

---

## Summary

| Operation | Winner | Speedup |
|-----------|--------|---------|
| **Parsing** | SymbAnaFis | **1.2x - 1.8x** faster |
| **Differentiation** | SymbAnaFis | **1.1x - 2.5x** faster |
| **Compilation** | SymbAnaFis | **3.6x - 12,500x** faster |
| **Tree-Walk → Compiled** | Compiled | **5x - 13x** faster |
| **eval_batch vs loop** | eval_batch | **~13%** faster |
| **Evaluation** (small expr) | Symbolica | **1.4x - 4.5x** faster |
| **Evaluation** (large expr, simplified) | SymbAnaFis | **6x - 16x** faster |
| **Full Pipeline** (small) | Symbolica | **1.9x - 4.8x** faster |

### Key Insights

1. **Compile for repeated evaluation:** Compiled bytecode is 5-13x faster than tree-walk evaluation.

2. **Simplification pays off:** For large expressions, SymbAnaFis's full simplification dramatically reduces expression size, leading to much faster compilation and evaluation.

3. **Different strategies:**
   - **Symbolica:** Light term collection (`3x + 2x → 5x`), faster simplification, optimized evaluator
   - **SymbAnaFis:** Deep AST restructuring (trig identities, algebraic normalization), massive compilation speedup

4. **Batch evaluation helps:** Using `eval_batch` provides ~13% speedup over calling `evaluate()` in a loop.

5. **When to use which:**
   - **Small expressions, one-shot evaluation:** Symbolica's faster evaluation wins
   - **Large expressions, repeated evaluation:** SymbAnaFis's simplification + fast compile wins
```
