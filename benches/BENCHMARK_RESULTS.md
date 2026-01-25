# Benchmark Results

**SymbAnaFis Version:** Unrealesed Dev Build  
**Date:** 2026-01-25
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
| Normal PDF        |        2.651 µs |       4.421 µs |          **1.67x** |
| Gaussian 2D       |        3.633 µs |       6.152 µs |          **1.69x** |
| Maxwell-Boltzmann |        4.252 µs |       5.958 µs |          **1.40x** |
| Lorentz Factor    |        1.385 µs |       2.289 µs |          **1.65x** |
| Lennard-Jones     |        2.270 µs |       3.558 µs |          **1.57x** |
| Logistic Sigmoid  |        1.687 µs |       2.194 µs |          **1.30x** |
| Damped Oscillator |        1.928 µs |       2.554 µs |          **1.32x** |
| Planck Blackbody  |        2.796 µs |       3.932 µs |          **1.41x** |
| Bessel Wave       |        1.794 µs |       2.269 µs |          **1.26x** |

> **Result:** SymbAnaFis parses **1.2x - 1.7x** faster than Symbolica.

---

## 2. Differentiation (Light)

| Expression        | SA (diff_only) | Symbolica (diff) | SA Speedup |
| ----------------- | -------------: | ---------------: | ---------: |
| Normal PDF        |       0.902 µs |         1.634 µs |  **1.81x** |
| Gaussian 2D       |       0.770 µs |         2.171 µs |  **2.82x** |
| Maxwell-Boltzmann |       1.305 µs |         3.214 µs |  **2.46x** |
| Lorentz Factor    |       0.914 µs |         1.798 µs |  **1.97x** |
| Lennard-Jones     |       1.158 µs |         1.837 µs |  **1.59x** |
| Logistic Sigmoid  |       0.547 µs |         1.127 µs |  **2.06x** |
| Damped Oscillator |       1.084 µs |         1.573 µs |  **1.45x** |
| Planck Blackbody  |       1.217 µs |         3.020 µs |  **2.48x** |
| Bessel Wave       |       1.426 µs |         1.643 µs |  **1.15x** |

### SymbAnaFis Full Simplification Cost

| Expression        | SA diff_only | SA diff+simplify |
| ----------------- | -----------: | ---------------: |
| Normal PDF        |     0.902 µs |        75.126 µs |
| Gaussian 2D       |     0.770 µs |        68.985 µs |
| Maxwell-Boltzmann |     1.305 µs |       172.730 µs |
| Lorentz Factor    |     0.914 µs |       133.330 µs |
| Lennard-Jones     |     1.158 µs |        15.277 µs |
| Logistic Sigmoid  |     0.547 µs |        62.188 µs |
| Damped Oscillator |     1.084 µs |        82.387 µs |
| Planck Blackbody  |     1.217 µs |       187.770 µs |
| Bessel Wave       |     1.426 µs |        68.604 µs |

---

## 4. Simplification Only (SymbAnaFis)

| Expression        | Time (median) |
| ----------------- | ------------: |
| Normal PDF        |     74.249 µs |
| Gaussian 2D       |     67.806 µs |
| Maxwell-Boltzmann |     171.45 µs |
| Lorentz Factor    |     132.42 µs |
| Lennard-Jones     |     14.257 µs |
| Logistic Sigmoid  |     60.818 µs |
| Damped Oscillator |     81.281 µs |
| Planck Blackbody  |     185.27 µs |
| Bessel Wave       |     67.651 µs |

---

## 5. Compilation (simplified) (medians)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------: | -------------: | -----------------: |
| Normal PDF        |        0.627 µs |       8.972 µs |          **14.3x** |
| Gaussian 2D       |        0.678 µs |       16.46 µs |          **24.3x** |
| Maxwell-Boltzmann |        1.081 µs |       8.322 µs |           **7.7x** |
| Lorentz Factor    |        0.472 µs |       4.861 µs |          **10.3x** |
| Lennard-Jones     |        0.522 µs |       13.00 µs |          **24.9x** |
| Logistic Sigmoid  |        0.462 µs |       4.928 µs |          **10.7x** |
| Damped Oscillator |        0.559 µs |       7.413 µs |          **13.3x** |
| Planck Blackbody  |        1.000 µs |       5.025 µs |           **5.0x** |
| Bessel Wave       |        0.791 µs |    *(skipped)* |                  — |

---

## 6. Evaluation (Compiled, 1000 points) (medians)

| Expression        | SymbAnaFis (Simpl) | Symbolica (SY) | SA vs SY |
| ----------------- | -----------------: | -------------: | -------: |
| Normal PDF        |           44.99 µs |       32.82 µs |    0.73x |
| Gaussian 2D       |           43.75 µs |       33.53 µs |    0.77x |
| Maxwell-Boltzmann |           70.10 µs |       42.22 µs |    0.60x |
| Lorentz Factor    |           32.13 µs |       32.29 µs |    1.00x |
| Lennard-Jones     |           30.20 µs |       34.17 µs |    1.13x |
| Logistic Sigmoid  |           30.18 µs |       30.04 µs |    1.00x |
| Damped Oscillator |           43.76 µs |       33.11 µs |    0.76x |
| Planck Blackbody  |           54.28 µs |       32.38 µs |    0.60x |
| Bessel Wave       |           99.84 µs |    *(skipped)* |        — |

---

## 7. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression        | SA (Full Simp) | SA (No Simp) | Symbolica (SY) | SA Full vs SY | SA No-Simp vs SY |
| ----------------- | -------------: | -----------: | -------------: | ------------: | ---------------: |
| Normal PDF        |      120.77 µs |     47.50 µs |       49.04 µs |         0.41x |        **1.03x** |
| Gaussian 2D       |      118.25 µs |     53.04 µs |       65.36 µs |         0.55x |        **1.23x** |
| Maxwell-Boltzmann |      255.14 µs |     82.52 µs |      109.05 µs |         0.43x |        **1.32x** |
| Lorentz Factor    |      163.65 µs |     33.94 µs |       53.70 µs |         0.33x |        **1.58x** |
| Lennard-Jones     |       59.34 µs |     50.40 µs |       56.56 µs |         0.95x |        **1.12x** |
| Logistic Sigmoid  |       95.10 µs |     29.25 µs |       43.23 µs |         0.45x |        **1.48x** |
| Damped Oscillator |      132.60 µs |     52.07 µs |       75.35 µs |         0.57x |        **1.45x** |
| Planck Blackbody  |      253.45 µs |     55.57 µs |       91.31 µs |         0.36x |        **1.64x** |
| Bessel Wave       |      178.00 µs |    113.14 µs |    *(skipped)* |             — |                — |

> **Key Finding:** Without full simplification, SymbAnaFis beats Symbolica on **all 8 expressions** (avg **1.33x faster**).
> The performance gap with full simplification is entirely due to deep algebraic restructuring (60-180µs overhead).

---

## 8. Large Expressions (100 / 300 terms) (medians)

### 100 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |   72.54 µs | 105.53 µs | **SA 1.45x** |
| Diff (no simplify)        |   43.61 µs | 112.73 µs | **SA 2.58x** |
| Diff+Simplify             |   3.759 ms |         — |            — |
| Compile (raw)             |   96.77 µs |  1.022 ms | **SA 10.6x** |
| Compile (simplified)      |   32.84 µs |  1.022 ms | **SA 31.1x** |
| Eval 1000pts (raw)        |   2.507 ms |  1.883 ms |        0.75x |
| Eval 1000pts (simplified) |   1.608 ms |  1.883 ms | **SA 1.17x** |

### 300 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |  223.54 µs | 336.85 µs | **SA 1.51x** |
| Diff (no simplify)        |  132.04 µs | 368.43 µs | **SA 2.79x** |
| Diff+Simplify             |   10.89 ms |         — |            — |
| Compile (raw)             |  723.89 µs | 11.639 ms | **SA 16.1x** |
| Compile (simplified)      |  165.52 µs | 11.639 ms | **SA 70.3x** |
| Eval 1000pts (raw)        |   7.335 ms |  5.310 ms |        0.72x |
| Eval 1000pts (simplified) |   4.607 ms |  5.310 ms | **SA 1.15x** |

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

