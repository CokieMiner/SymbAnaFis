# Benchmark Results

**SymbAnaFis Version:** Unrealesed Dev Build  
**Date:** 2026-01-28
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

## 1. Parsing (String → AST)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------: | -------------: | -----------------: |
| Normal PDF        |        2.662 µs |       4.373 µs |          **1.64x** |
| Gaussian 2D       |        3.600 µs |       6.133 µs |          **1.70x** |
| Maxwell-Boltzmann |        4.138 µs |       5.932 µs |          **1.43x** |
| Lorentz Factor    |        1.368 µs |       2.226 µs |          **1.63x** |
| Lennard-Jones     |        2.165 µs |       3.528 µs |          **1.63x** |
| Logistic Sigmoid  |        1.676 µs |       2.164 µs |          **1.29x** |
| Damped Oscillator |        1.983 µs |       2.514 µs |          **1.27x** |
| Planck Blackbody  |        2.699 µs |       3.956 µs |          **1.47x** |
| Bessel Wave       |        1.766 µs |       2.233 µs |          **1.26x** |

> **Result:** SymbAnaFis parses **1.2x - 1.7x** faster than Symbolica.

---

## 2. Differentiation (Light)

| Expression        | SA (diff_only) | Symbolica (diff) | SA Speedup |
| ----------------- | -------------: | ---------------: | ---------: |
| Normal PDF        |       0.913 µs |         1.655 µs |  **1.81x** |
| Gaussian 2D       |       0.777 µs |         2.206 µs |  **2.84x** |
| Maxwell-Boltzmann |       1.290 µs |         3.242 µs |  **2.51x** |
| Lorentz Factor    |       0.928 µs |         1.811 µs |  **1.95x** |
| Lennard-Jones     |       1.178 µs |         1.850 µs |  **1.57x** |
| Logistic Sigmoid  |       0.551 µs |         1.117 µs |  **2.03x** |
| Damped Oscillator |       0.977 µs |         1.609 µs |  **1.65x** |
| Planck Blackbody  |       1.215 µs |         3.037 µs |  **2.50x** |
| Bessel Wave       |       1.389 µs |         1.654 µs |  **1.19x** |

### SymbAnaFis Full Simplification Cost

| Expression        | SA diff_only | SA diff+simplify |
| ----------------- | -----------: | ---------------: |
| Normal PDF        |     0.913 µs |        76.532 µs |
| Gaussian 2D       |     0.777 µs |        71.157 µs |
| Maxwell-Boltzmann |     1.290 µs |       174.510 µs |
| Lorentz Factor    |     0.928 µs |       135.300 µs |
| Lennard-Jones     |     1.178 µs |        15.313 µs |
| Logistic Sigmoid  |     0.551 µs |        62.473 µs |
| Damped Oscillator |     0.977 µs |        83.143 µs |
| Planck Blackbody  |     1.215 µs |       187.430 µs |
| Bessel Wave       |     1.389 µs |        69.738 µs |

---

## 4. Simplification Only (SymbAnaFis)

| Expression        | Time (median) |
| ----------------- | ------------: |
| Normal PDF        |     75.139 µs |
| Gaussian 2D       |     69.201 µs |
| Maxwell-Boltzmann |     173.21 µs |
| Lorentz Factor    |     134.24 µs |
| Lennard-Jones     |     14.197 µs |
| Logistic Sigmoid  |     61.632 µs |
| Damped Oscillator |     81.379 µs |
| Planck Blackbody  |     186.33 µs |
| Bessel Wave       |     67.902 µs |

---

## 5. Compilation (simplified) (medians)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------: | -------------: | -----------------: |
| Normal PDF        |        0.656 µs |       8.757 µs |          **13.4x** |
| Gaussian 2D       |        0.676 µs |       16.29 µs |          **24.1x** |
| Maxwell-Boltzmann |        1.096 µs |       8.262 µs |           **7.5x** |
| Lorentz Factor    |        0.467 µs |       4.813 µs |          **10.3x** |
| Lennard-Jones     |        0.523 µs |       12.82 µs |          **24.5x** |
| Logistic Sigmoid  |        0.451 µs |       4.848 µs |          **10.7x** |
| Damped Oscillator |        0.553 µs |       7.319 µs |          **13.2x** |
| Planck Blackbody  |        0.983 µs |       4.968 µs |           **5.1x** |
| Bessel Wave       |        0.794 µs |    *(skipped)* |                  — |

---

## 6. Evaluation (Compiled, 1000 points) (medians)

| Expression        | SymbAnaFis (Simpl) | Symbolica (SY) | SA vs SY |
| ----------------- | -----------------: | -------------: | -------: |
| Normal PDF        |           43.19 µs |       32.80 µs |    0.76x |
| Gaussian 2D       |           39.83 µs |       33.65 µs |    0.84x |
| Maxwell-Boltzmann |           62.73 µs |       42.11 µs |    0.67x |
| Lorentz Factor    |           30.07 µs |       32.33 µs |    1.08x |
| Lennard-Jones     |           28.00 µs |       34.27 µs |    1.22x |
| Logistic Sigmoid  |           28.44 µs |       29.92 µs |    1.05x |
| Damped Oscillator |           41.78 µs |       32.97 µs |    0.79x |
| Planck Blackbody  |           50.80 µs |       32.10 µs |    0.63x |
| Bessel Wave       |           98.43 µs |    *(skipped)* |        — |

---

## 7. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression        | SA (Full Simp) | SA (No Simp) | Symbolica (SY) | SA Full vs SY | SA No-Simp vs SY |
| ----------------- | -------------: | -----------: | -------------: | ------------: | ---------------: |
| Normal PDF        |      121.82 µs |     42.21 µs |       48.87 µs |         0.40x |        **1.16x** |
| Gaussian 2D       |      116.27 µs |     45.18 µs |       64.63 µs |         0.56x |        **1.43x** |
| Maxwell-Boltzmann |      249.95 µs |     69.94 µs |      108.87 µs |         0.44x |        **1.56x** |
| Lorentz Factor    |      164.60 µs |     29.92 µs |       53.59 µs |         0.33x |        **1.79x** |
| Lennard-Jones     |       58.36 µs |     46.13 µs |       56.31 µs |         0.96x |        **1.22x** |
| Logistic Sigmoid  |       94.10 µs |     26.09 µs |       43.06 µs |         0.46x |        **1.65x** |
| Damped Oscillator |      130.70 µs |     46.86 µs |       75.20 µs |         0.57x |        **1.60x** |
| Planck Blackbody  |      252.56 µs |     48.84 µs |       91.29 µs |         0.36x |        **1.87x** |
| Bessel Wave       |      180.17 µs |    111.41 µs |    *(skipped)* |             — |                — |

> **Key Finding:** Without full simplification, SymbAnaFis beats Symbolica on **all 8 expressions** (avg **1.33x faster**).
> The performance gap with full simplification is entirely due to deep algebraic restructuring (60-180µs overhead).

---

## 8. Large Expressions (100 / 300 terms) (medians)

### 100 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |   72.14 µs | 105.09 µs | **SA 1.46x** |
| Diff (no simplify)        |   44.01 µs | 114.57 µs | **SA 2.60x** |
| Diff+Simplify             |   3.787 ms |         — |            — |
| Compile (raw)             |   96.14 µs |  1.045 ms | **SA 10.9x** |
| Compile (simplified)      |   32.80 µs |  1.045 ms | **SA 31.8x** |
| Eval 1000pts (raw)        |   2.458 ms |  1.879 ms |        0.76x |
| Eval 1000pts (simplified) |   1.511 ms |  1.879 ms | **SA 1.24x** |

### 300 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |  222.44 µs | 339.39 µs | **SA 1.53x** |
| Diff (no simplify)        |  132.71 µs | 375.83 µs | **SA 2.83x** |
| Diff+Simplify             |   11.31 ms |         — |            — |
| Compile (raw)             |  720.90 µs | 13.23 ms | **SA 18.3x** |
| Compile (simplified)      |  165.08 µs | 13.23 ms | **SA 80.1x** |
| Eval 1000pts (raw)        |   7.085 ms |  5.350 ms |        0.75x |
| Eval 1000pts (simplified) |   4.596 ms |  5.350 ms | **SA 1.16x** |

---

## 9. Tree-Walk vs Compiled Evaluation (medians)

| Expression        | Tree-Walk (1000 pts) | Compiled (1000 pts) |   Speedup |
| ----------------- | -------------------: | ------------------: | --------: |
| Normal PDF        |            486.40 µs |            42.05 µs | **11.6x** |
| Gaussian 2D       |            490.43 µs |            41.68 µs | **11.8x** |
| Maxwell-Boltzmann |            603.35 µs |            68.49 µs |  **8.8x** |
| Lorentz Factor    |            379.67 µs |            29.71 µs | **12.8x** |
| Lennard-Jones     |            316.92 µs |            27.93 µs | **11.3x** |
| Logistic Sigmoid  |            500.00 µs |            25.88 µs | **19.3x** |
| Damped Oscillator |            445.01 µs |            39.41 µs | **11.3x** |
| Planck Blackbody  |            899.38 µs |            51.97 µs | **17.3x** |
| Bessel Wave       |            566.79 µs |            99.29 µs |  **5.7x** |

---

## 10. Batch Evaluation Performance (SIMD)

| Points  | loop_evaluate | eval_batch (SIMD) |  Speedup |
| ------- | ------------: | ----------------: | -------: |
| 100     |      2.679 µs |          1.133 µs | **2.4x** |
| 1,000   |      26.60 µs |          11.36 µs | **2.3x** |
| 10,000  |     269.04 µs |         113.59 µs | **2.4x** |
| 100,000 |      2.667 ms |          1.137 ms | **2.3x** |

---

## 11. Multi-Expression Batch Evaluation

| Method                            |     Time | vs Sequential |
| --------------------------------- | -------: | ------------: |
| eval_batch_per_expr (SIMD)        | 21.43 µs |   1.9x faster |
| eval_f64_per_expr (SIMD+parallel) | 39.57 µs |   1.1x faster |
| sequential_loops                  | 41.77 µs |      baseline |

---

## 12. eval_f64 vs evaluate_parallel APIs

| API                        |      Time |
| -------------------------- | --------: |
| `eval_f64` (SIMD+parallel) |  46.42 µs |
| `evaluate_parallel`        | 229.19 µs |

