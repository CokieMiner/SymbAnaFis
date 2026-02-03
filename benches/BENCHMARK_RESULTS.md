# Benchmark Results

**SymbAnaFis Version:** Unreleased Dev Build  
**Date:** 2026-02-03
---

## System Specifications

- **CPU:** AMD Ryzen AI 7 350 w/ Radeon 860M (8 cores, 16 threads)
- **CPU Max:** 5.09 GHz
- **RAM:** 32 GB (30 GiB total)
- **OS:** Linux 6.17.12 (Fedora 43)
- **Rust:** rustc 1.93.0
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

## TL;DR - Average Speedups

| Category                  | Avg Speedup (SA vs SY) | Notes                                   |
|---------------------------|------------------------|-----------------------------------------|
| Parsing                   |                  1.48x | SA 1.3x-1.6x faster                     |
| Differentiation           |                  1.44x | SA 1.00x-2.03x faster                   |
| Compilation               |                  7.0x  | SA 2.6x-19.4x faster                    |
| Evaluation                |                  1.29x | SA competitive, faster on 6/8           |
| Full Pipeline (No Simp)   |                  2.0x  | SA beats SY on all expressions          |
| Full Pipeline (With Simp) |                  0.64x | SY faster due to SA deep simplification |
---

## 1. Parsing (String → AST)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------: | -------------: | -----------------: |
| Normal PDF        |        2.782 µs |       4.472 µs |          **1.61x** |
| Gaussian 2D       |        3.827 µs |       6.288 µs |          **1.64x** |
| Maxwell-Boltzmann |        4.513 µs |       5.989 µs |          **1.33x** |
| Lorentz Factor    |        1.565 µs |       2.318 µs |          **1.48x** |
| Lennard-Jones     |        2.261 µs |       3.605 µs |          **1.59x** |
| Logistic Sigmoid  |        1.594 µs |       2.170 µs |          **1.36x** |
| Damped Oscillator |        1.812 µs |       2.549 µs |          **1.41x** |
| Planck Blackbody  |        2.741 µs |       4.051 µs |          **1.48x** |
| Bessel Wave       |        1.621 µs |       2.269 µs |          **1.40x** |

> **Result:** SymbAnaFis parses **1.3x - 1.6x** faster than Symbolica.

---

## 2. Differentiation (Light)

| Expression        | SA (diff_only) | Symbolica (diff) | SA Speedup |
| ----------------- | -------------: | ---------------: | ---------: |
| Normal PDF        |       1.318 µs |         1.650 µs |  **1.25x** |
| Gaussian 2D       |       1.090 µs |         2.216 µs |  **2.03x** |
| Maxwell-Boltzmann |       1.705 µs |         3.248 µs |  **1.90x** |
| Lorentz Factor    |       1.442 µs |         1.818 µs |  **1.26x** |
| Lennard-Jones     |       1.673 µs |         1.847 µs |  **1.10x** |
| Logistic Sigmoid  |       0.824 µs |         1.125 µs |  **1.37x** |
| Damped Oscillator |       1.168 µs |         1.597 µs |  **1.37x** |
| Planck Blackbody  |       1.770 µs |         3.028 µs |  **1.71x** |
| Bessel Wave       |       1.638 µs |         1.631 µs |  **1.00x** |

### SymbAnaFis Full Simplification Cost

| Expression        | SA diff_only | SA diff+simplify |
| ----------------- | -----------: | ---------------: |
| Normal PDF        |     1.318 µs |        68.00 µs |
| Gaussian 2D       |     1.090 µs |        64.68 µs |
| Maxwell-Boltzmann |     1.705 µs |       122.30 µs |
| Lorentz Factor    |     1.442 µs |       109.81 µs |
| Lennard-Jones     |     1.673 µs |        17.12 µs |
| Logistic Sigmoid  |     0.824 µs |        55.80 µs |
| Damped Oscillator |     1.168 µs |        53.61 µs |
| Planck Blackbody  |     1.770 µs |       146.29 µs |
| Bessel Wave       |     1.638 µs |        61.89 µs |

---

## 3. Compilation (Raw vs Simplified)

### Raw Compilation

| Expression        | SymbAnaFis (Raw) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | ---------------: | -------------: | -----------------: |
| Normal PDF        |         1.455 µs |       8.836 µs |           **6.1x** |
| Gaussian 2D       |         1.576 µs |       16.39 µs |          **10.4x** |
| Maxwell-Boltzmann |         2.179 µs |       8.409 µs |           **3.9x** |
| Lorentz Factor    |         1.409 µs |       4.866 µs |           **3.5x** |
| Lennard-Jones     |         0.671 µs |       13.00 µs |          **19.4x** |
| Logistic Sigmoid  |         0.974 µs |       4.864 µs |           **5.0x** |
| Damped Oscillator |         1.420 µs |       7.395 µs |           **5.2x** |
| Planck Blackbody  |         1.947 µs |       5.027 µs |           **2.6x** |
| Bessel Wave       |         1.717 µs |    *(skipped)* |                  — |

### Simplified Compilation

| Expression        | SymbAnaFis (Simplified) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | ----------------------: | -------------: | -----------------: |
| Normal PDF        |                1.210 µs |       8.836 µs |           **7.3x** |
| Gaussian 2D       |                1.327 µs |       16.39 µs |          **12.4x** |
| Maxwell-Boltzmann |                1.317 µs |       8.409 µs |           **6.4x** |
| Lorentz Factor    |                1.146 µs |       4.866 µs |           **4.2x** |
| Lennard-Jones     |                0.604 µs |       13.00 µs |          **21.5x** |
| Logistic Sigmoid  |                1.121 µs |       4.864 µs |           **4.3x** |
| Damped Oscillator |                1.151 µs |       7.395 µs |           **6.4x** |
| Planck Blackbody  |                2.332 µs |       5.027 µs |           **2.2x** |
| Bessel Wave       |                1.466 µs |    *(skipped)* |                  — |

> **Result:** SymbAnaFis compiles **2.6x - 19.4x** faster than Symbolica (avg ~7.0x).

## 4. Evaluation (Compiled, 1000 points)

### Raw Evaluation

| Expression        | SymbAnaFis (Raw) | Symbolica (SY) | SA vs SY |
| ----------------- | ---------------: | -------------: | -------: |
| Normal PDF        |         20.78 µs |       33.30 µs |    1.60x |
| Gaussian 2D       |         23.30 µs |       33.83 µs |    1.45x |
| Maxwell-Boltzmann |         34.47 µs |       42.55 µs |    1.23x |
| Lorentz Factor    |         20.66 µs |       32.35 µs |    1.57x |
| Lennard-Jones     |         29.36 µs |       34.63 µs |    1.18x |
| Logistic Sigmoid  |         17.96 µs |       30.09 µs |    1.67x |
| Damped Oscillator |         36.66 µs |       33.18 µs |    0.91x |
| Planck Blackbody  |         44.18 µs |       32.54 µs |    0.74x |
| Bessel Wave       |         92.81 µs |    *(skipped)* |        — |

### Simplified Evaluation

| Expression        | SymbAnaFis (Simplified) | Symbolica (SY) | SA vs SY |
| ----------------- | ----------------------: | -------------: | -------: |
| Normal PDF        |                24.67 µs |       33.30 µs |    1.35x |
| Gaussian 2D       |                23.93 µs |       33.83 µs |    1.41x |
| Maxwell-Boltzmann |                37.09 µs |       42.55 µs |    1.15x |
| Lorentz Factor    |                19.10 µs |       32.35 µs |    1.69x |
| Lennard-Jones     |                29.15 µs |       34.63 µs |    1.19x |
| Logistic Sigmoid  |                21.62 µs |       30.09 µs |    1.39x |
| Damped Oscillator |                32.55 µs |       33.18 µs |    1.02x |
| Planck Blackbody  |                48.79 µs |       32.54 µs |    0.67x |
| Bessel Wave       |                79.50 µs |    *(skipped)* |        — |

> **Result:** SymbAnaFis evaluation is competitive, with SA faster on 6/8 expressions (avg 1.29x vs SY).

---

## 5. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression        | SA (Full Simp) | SA (No Simp) | Symbolica (SY) | SA Full vs SY | SA No-Simp vs SY |
| ----------------- | -------------: | -----------: | -------------: | ------------: | ---------------: |
| Normal PDF        |      110.89 µs |     26.88 µs |       49.29 µs |         0.44x |        **1.83x** |
| Gaussian 2D       |      105.41 µs |     30.39 µs |       65.46 µs |         0.62x |        **2.15x** |
| Maxwell-Boltzmann |      169.34 µs |     45.03 µs |      109.17 µs |         0.64x |        **2.43x** |
| Lorentz Factor    |      133.12 µs |     25.62 µs |       53.96 µs |         0.41x |        **2.11x** |
| Lennard-Jones     |       49.52 µs |     34.37 µs |       57.10 µs |         1.15x |        **1.66x** |
| Logistic Sigmoid  |       81.22 µs |     22.12 µs |       43.08 µs |         0.53x |        **1.95x** |
| Damped Oscillator |       90.64 µs |     41.52 µs |       75.45 µs |         0.83x |        **1.82x** |
| Planck Blackbody  |      191.48 µs |     51.16 µs |       91.45 µs |         0.48x |        **1.79x** |
| Bessel Wave       |      149.17 µs |     98.78 µs |    *(skipped)* |             — |                — |

> **Key Finding:** Without full simplification, SymbAnaFis beats Symbolica on **all 8 expressions** (avg **2.0x faster**).
> The performance gap with full simplification is entirely due to deep algebraic restructuring (60-180µs overhead).

---

## 6. Large Expressions (100 / 300 terms) (medians)

### 100 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |   84.75 µs | 106.74 µs | **SA 1.26x** |
| Diff (no simplify)        |   65.11 µs | 115.44 µs | **SA 1.77x** |
| Diff+Simplify             |   1.586 ms |         — |            — |
| Compile (raw)             |  189.96 µs |  1.042 ms | **SA 5.49x** |
| Compile (simplified)      |   82.71 µs |  1.042 ms | **SA 12.6x** |
| Eval 1000pts (raw)        |   1.966 ms |  1.861 ms |      0.95x   |
| Eval 1000pts (simplified) |   1.121 ms |  1.861 ms | **SA 1.66x** |

### 300 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |  276.57 µs | 343.46 µs | **SA 1.24x** |
| Diff (no simplify)        |  205.93 µs | 379.91 µs | **SA 1.84x** |
| Diff+Simplify             |   4.991 ms |         — |            — |
| Compile (raw)             |   1.403 ms |  15.18 ms | **SA 10.8x** |
| Compile (simplified)      |  455.28 µs |  15.18 ms | **SA 33.3x** |
| Eval 1000pts (raw)        |   5.275 ms |  5.295 ms |      1.00x   |
| Eval 1000pts (simplified) |   3.037 ms |  5.295 ms | **SA 1.74x** |

---

## 7. Tree-Walk vs Compiled Evaluation (medians)

| Expression        | Tree-Walk (1000 pts) | Compiled (1000 pts) |   Speedup |
| ----------------- | -------------------: | ------------------: | --------: |
| Normal PDF        |            484.91 µs |            26.20 µs | **18.5x** |
| Gaussian 2D       |            502.91 µs |            26.25 µs | **19.2x** |
| Maxwell-Boltzmann |            580.52 µs |            42.58 µs | **13.6x** |
| Lorentz Factor    |            369.64 µs |            21.39 µs | **17.3x** |
| Lennard-Jones     |            177.89 µs |            31.66 µs |  **5.6x** |
| Logistic Sigmoid  |            483.30 µs |            22.90 µs | **21.1x** |
| Damped Oscillator |            443.40 µs |            35.32 µs | **12.6x** |
| Planck Blackbody  |            884.71 µs |            53.10 µs | **16.7x** |
| Bessel Wave       |            564.91 µs |            82.01 µs |  **6.9x** |

---

## 8. Batch Evaluation Performance (SIMD)

| Points  | loop_evaluate | eval_batch (SIMD) |  Speedup |
| ------- | ------------: | ----------------: | -------: |
| 100     |      1.794 µs |          0.913 µs | **2.0x** |
| 1,000   |      18.25 µs |          9.069 µs | **2.0x** |
| 10,000  |     183.76 µs |         90.89  µs | **2.0x** |
| 100,000 |      1.809 ms |          0.919 ms | **2.0x** |

---

## 9. Multi-Expression Batch Evaluation

| Method                            |     Time | vs Sequential |
| --------------------------------- | -------: | ------------: |
| sequential_loops                  | 3.593 ms |      baseline |
| eval_batch_per_expr (SIMD)        | 1.863 ms |   1.9x faster |
| eval_f64_per_expr (SIMD+parallel) | 599.0 µs |   6.0x faster |

---

## 10. eval_f64 vs evaluate_parallel APIs

| API                        |      Time |
| -------------------------- | --------: |
| `eval_f64` (SIMD+parallel) |  38.72 µs |
| `evaluate_parallel`        | 231.63 µs |

---

## Use Cases and Recommendations

### When to Use Full Simplification
- **Complex/large expressions**: Where deep algebraic restructuring can significantly reduce evaluation time (e.g., expressions with redundancies or large terms). The upfront cost (60-180µs) pays off for repeated evaluations.
- **High-performance scenarios**: When you need the absolute fastest evaluation and the expression has significant simplification potential.

### When to Skip Full Simplification
- **Small/simple expressions**: Where the simplification overhead outweighs the benefits, and raw/no-simp evaluation is faster (e.g., short expressions like Lennard-Jones).
- **One-off evaluations**: If the expression is evaluated only a few times, the cost isn't justified.
- **Time-sensitive parsing/diff**: When you need fast differentiation without the full pipeline, as SA is already faster in diff-only mode.
- **Real-time applications**: Where low latency is critical, and you can tolerate slightly slower evaluation for faster setup.

### General Tips
- Use `eval_f64` for single evaluations with SIMD+parallel benefits.
- For batch evaluations, leverage `eval_batch` or `eval_f64_per_expr` for massive speedups.
- SymbAnaFis excels in compilation and evaluation, especially without heavy simplification.

