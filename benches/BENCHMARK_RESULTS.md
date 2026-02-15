# Benchmark Results

**SymbAnaFis Version:** 0.8.1  
**Date:** 2026-02-15
---

## System Specifications

- **CPU:** AMD Ryzen AI 7 350 w/ Radeon 860M (8 cores, 16 threads)
- **CPU Max:** 5.04 GHz
- **RAM:** 30 GB (30 GiB total)
- **OS:** Linux 6.18.8-arch2-1 (Endeavour OS)
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

| Category                  | Avg Speedup (SA vs SY) | Notes                                          |
|---------------------------|------------------------|------------------------------------------------|
| Parsing                   |                  1.58x | SA 1.39x-1.75x faster                          |
| Differentiation           |                  1.35x | SA 0.95x-1.98x faster                          |
| Compilation (Raw)         |                  3.52x | SA 0.98x-14.17x faster                         |
| Evaluation (Raw)          |                  1.04x | SA competitive (faster on 5/8 comparable)      |
| Full Pipeline (No Simp)   |                  1.85x | SA beats SY on all 8 comparable expressions    |
| Full Pipeline (With Simp) |                  0.65x | SY faster due to SA deep simplification cost   |

---

## 1. Parsing (String → AST)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------: | -------------: | -----------------: |
| Normal PDF        |        2.660 µs |       4.287 µs |          **1.61x** |
| Gaussian 2D       |        3.378 µs |       5.922 µs |          **1.75x** |
| Maxwell-Boltzmann |        4.074 µs |       5.747 µs |          **1.41x** |
| Lorentz Factor    |        1.373 µs |       2.142 µs |          **1.56x** |
| Lennard-Jones     |        2.078 µs |       3.357 µs |          **1.62x** |
| Logistic Sigmoid  |        1.435 µs |       2.065 µs |          **1.44x** |
| Damped Oscillator |        1.666 µs |       2.395 µs |          **1.44x** |
| Planck Blackbody  |        2.505 µs |       3.895 µs |          **1.56x** |
| Bessel Wave       |        1.488 µs |       2.157 µs |          **1.45x** |

---

## 2. Differentiation (Light)

| Expression        | SA (diff_only) | Symbolica (diff) | SA Speedup |
| ----------------- | -------------: | ---------------: | ---------: |
| Normal PDF        |       1.306 µs |         1.590 µs |  **1.22x** |
| Gaussian 2D       |       1.095 µs |         2.166 µs |  **1.98x** |
| Maxwell-Boltzmann |       1.736 µs |         2.989 µs |  **1.72x** |
| Lorentz Factor    |       1.360 µs |         1.765 µs |  **1.30x** |
| Lennard-Jones     |       1.607 µs |         1.712 µs |  **1.07x** |
| Logistic Sigmoid  |     883.76 ns  |         1.106 µs |  **1.25x** |
| Damped Oscillator |       1.257 µs |         1.479 µs |  **1.18x** |
| Planck Blackbody  |       1.772 µs |         2.827 µs |  **1.60x** |
| Bessel Wave       |       1.583 µs |         1.507 µs |  **0.95x** |

### SymbAnaFis Full Simplification Cost

| Expression        | SA diff_only | SA diff+simplify |
| ----------------- | -----------: | ---------------: |
| Normal PDF        |     1.306 µs |        67.81 µs  |
| Gaussian 2D       |     1.095 µs |        53.16 µs  |
| Maxwell-Boltzmann |     1.736 µs |       122.50 µs  |
| Lorentz Factor    |     1.360 µs |        90.61 µs  |
| Lennard-Jones     |     1.607 µs |        16.49 µs  |
| Logistic Sigmoid  |   883.76 ns  |        54.82 µs  |
| Damped Oscillator |     1.257 µs |        52.35 µs  |
| Planck Blackbody  |     1.772 µs |       142.79 µs  |
| Bessel Wave       |     1.583 µs |        61.53 µs  |

---

## 3. Compilation (Raw vs Simplified)

### Raw Compilation

| Expression        | SymbAnaFis (Raw) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | ---------------: | -------------: | -----------------: |
| Normal PDF        |         2.214 µs |       4.462 µs |          **2.01x** |
| Gaussian 2D       |         2.457 µs |       4.865 µs |          **1.98x** |
| Maxwell-Boltzmann |         3.193 µs |       7.841 µs |          **2.45x** |
| Lorentz Factor    |         2.242 µs |       2.190 µs |            0.98x   |
| Lennard-Jones     |       924.55 ns  |      13.103 µs |         **14.17x** |
| Logistic Sigmoid  |         1.363 µs |       2.580 µs |          **1.89x** |
| Damped Oscillator |         1.752 µs |       2.440 µs |          **1.39x** |
| Planck Blackbody  |         3.511 µs |       4.845 µs |          **1.38x** |
| Bessel Wave       |         2.276 µs |    *(skipped)* |                —   |

### Simplified Compilation

| Expression        | SymbAnaFis (Simplified) | Symbolica (SY) |  Speedup (SA vs SY) |
|-------------------|------------------------:|---------------:|--------------------:|
| Normal PDF        |                1.839 µs |       4.462 µs |           **2.43x** |
| Gaussian 2D       |                2.080 µs |       4.865 µs |           **2.34x** |
| Maxwell-Boltzmann |                2.387 µs |       7.841 µs |           **3.28x** |
| Lorentz Factor    |                1.453 µs |       2.190 µs |           **1.51x** |
| Lennard-Jones     |              920.15 ns  |      13.103 µs |          **14.24x** |
| Logistic Sigmoid  |                1.635 µs |       2.580 µs |           **1.58x** |
| Damped Oscillator |                1.542 µs |       2.440 µs |           **1.58x** |
| Planck Blackbody  |                3.563 µs |       4.845 µs |           **1.36x** |
| Bessel Wave       |                2.093 µs |    *(skipped)* |                 —   |

---

## 4. Evaluation (Compiled, 1000 points)

### Raw Evaluation

| Expression        | SymbAnaFis (Raw) | Symbolica (SY) | SA vs SY |
| ----------------- | ---------------: | -------------: | -------: |
| Normal PDF        |         19.898 µs |       27.433 µs |    1.38x |
| Gaussian 2D       |         22.963 µs |       28.538 µs |    1.24x |
| Maxwell-Boltzmann |         35.925 µs |       34.205 µs |    0.95x |
| Lorentz Factor    |         20.417 µs |       27.390 µs |    1.34x |
| Lennard-Jones     |         27.689 µs |       29.121 µs |    1.05x |
| Logistic Sigmoid  |         17.293 µs |       25.062 µs |    1.45x |
| Damped Oscillator |         36.383 µs |       28.115 µs |    0.77x |
| Planck Blackbody  |         43.715 µs |       26.629 µs |    0.61x |
| Bessel Wave       |         63.068 µs |    *(skipped)* |        — |

### Simplified Evaluation

| Expression        | SymbAnaFis (Simplified) | Symbolica (SY) | SA vs SY |
| ----------------- | ----------------------: | -------------: | -------: |
| Normal PDF        |                24.347 µs |       27.433 µs |    1.13x |
| Gaussian 2D       |                23.832 µs |       28.538 µs |    1.20x |
| Maxwell-Boltzmann |                35.121 µs |       34.205 µs |    0.97x |
| Lorentz Factor    |                16.926 µs |       27.390 µs |    1.62x |
| Lennard-Jones     |                27.327 µs |       29.121 µs |    1.07x |
| Logistic Sigmoid  |                20.873 µs |       25.062 µs |    1.20x |
| Damped Oscillator |                33.473 µs |       28.115 µs |    0.84x |
| Planck Blackbody  |                47.392 µs |       26.629 µs |    0.56x |
| Bessel Wave       |                58.426 µs |    *(skipped)* |        — |

---

## 5. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression        | SA (Full Simp)  |  SA (No Simp) |  Symbolica (SY) | SA Full vs SY | SA No-Simp vs SY |
| ----------------- |  -------------: |  -----------: |  -------------: | ------------: | ---------------: |
| Normal PDF        |      108.39 µs  |     26.875 µs |       47.964 µs |       0.44x   |        **1.78x** |
| Gaussian 2D       |       90.503 µs |     30.609 µs |       53.923 µs |       0.60x   |        **1.76x** |
| Maxwell-Boltzmann |      167.25 µs  |     45.717 µs |       108.77 µs |       0.65x   |        **2.38x** |
| Lorentz Factor    |      112.49 µs  |     25.559 µs |       53.463 µs |       0.48x   |        **2.09x** |
| Lennard-Jones     |       47.998 µs |     32.999 µs |       58.063 µs |     **1.21x** |        **1.76x** |
| Logistic Sigmoid  |       80.134 µs |     21.023 µs |       42.845 µs |       0.53x   |        **2.04x** |
| Damped Oscillator |       90.898 µs |     41.507 µs |       72.942 µs |       0.80x   |        **1.76x** |
| Planck Blackbody  |      187.49 µs  |     52.246 µs |       100.70 µs |       0.54x   |        **1.93x** |
| Bessel Wave       |      124.99 µs  |     68.955 µs |    *(skipped)*  |             — |                — |

---

## 6. Large Expressions (100 / 300 terms) (medians)

### 100 Terms

| Operation                 |   SymbAnaFis |  Symbolica |      Speedup |
| ------------------------- |-------------:|-----------:| -----------: |
| Parse                     |   82.912 µs  | 90.282 µs  | **SA 1.09x** |
| Diff (no simplify)        |   67.962 µs  | 78.357 µs  | **SA 1.15x** |
| Diff+Simplify             |    1.7730 ms |          — |           —  |
| Compile (raw)             |   77.873 µs  |  1.7626 ms |**SA 22.63x** |
| Compile (simplified)      |   79.847 µs  |  1.7626 ms |**SA 22.07x** |
| Eval 1000pts (raw)        |    1.6055 ms |  1.6591 ms |      1.03x   |
| Eval 1000pts (simplified) |    1.6096 ms |  1.6591 ms |      1.03x   |

### 300 Terms

| Operation                 |   SymbAnaFis |   Symbolica |       Speedup |
| ------------------------- | -----------: | ----------: |  -----------: |
| Parse                     |  263.28 µs   | 272.72 µs   |  **SA 1.04x** |
| Diff (no simplify)        |  213.90 µs   | 233.41 µs   |  **SA 1.09x** |
| Diff+Simplify             |    5.3748 ms |         —   |             — |
| Compile (raw)             |  242.82 µs   |   9.3637 ms | **SA 38.55x** |
| Compile (simplified)      |  249.58 µs   |   9.3637 ms | **SA 37.51x** |
| Eval 1000pts (raw)        |    4.4117 ms |   4.7579 ms |  **SA 1.08x** |
| Eval 1000pts (simplified) |    4.3524 ms |   4.7579 ms |  **SA 1.09x** |

---

## 7. Tree-Walk vs Compiled Evaluation (medians)

| Expression        | Tree-Walk (1000 pts) |  Compiled (1000 pts) |    Speedup |
| ----------------- | -------------------: | -------------------: | ---------: |
| Normal PDF        |            482.80 µs |            24.347 µs | **19.83x** |
| Gaussian 2D       |            498.14 µs |            23.832 µs | **20.91x** |
| Maxwell-Boltzmann |            574.86 µs |            35.121 µs | **16.38x** |
| Lorentz Factor    |            295.32 µs |            16.926 µs | **17.45x** |
| Lennard-Jones     |            180.17 µs |            27.327 µs |  **6.59x** |
| Logistic Sigmoid  |            489.09 µs |            20.873 µs | **23.43x** |
| Damped Oscillator |            443.54 µs |            33.473 µs | **13.25x** |
| Planck Blackbody  |            877.51 µs |            47.392 µs | **18.52x** |
| Bessel Wave       |            575.76 µs |            58.426 µs |  **9.86x** |

---

## 8. Batch Evaluation Performance (SIMD)

| Points  |    loop_evaluate |  eval_batch (SIMD) |   Speedup |
| ------- | ---------------: | -----------------: | --------: |
| 100     |       1.1223 µs  |        329.62 ns   | **3.40x** |
| 1,000   |      11.039 µs   |          3.2360 µs | **3.41x** |
| 10,000  |      110.17 µs   |         32.510 µs  | **3.39x** |
| 100,000 |        1.1265 ms |        324.23 µs   | **3.47x** |

---

## 9. Multi-Expression Batch Evaluation

| Method                            |      Time | vs Sequential |
| --------------------------------- | --------: | ------------: |
| sequential_loops                  | 3.0044 ms |      baseline |
| eval_batch_per_expr (SIMD)        | 1.4234 ms |  2.11x faster |
| eval_f64_per_expr (SIMD+parallel) | 511.55 µs |  5.87x faster |

---

## 10. eval_f64 vs evaluate_parallel APIs

| API                        |       Time |
| -------------------------- | ---------: |
| `eval_f64` (SIMD+parallel) |  25.388 µs |
| `evaluate_parallel`        | 170.46 µs  |

---

## Use Cases and Recommendations

### When to Use Full Simplification
- **Complex/large expressions**: Where deep algebraic restructuring can significantly reduce evaluation time.
- **High-performance scenarios**: When expression reuse amortizes simplification cost.

### When to Skip Full Simplification
- **Small/simple expressions**: Where simplification overhead outweighs runtime savings.
- **One-off evaluations**: If expressions are evaluated only a few times.
- **Latency-sensitive diff/compile paths**: Diff-only and raw compile are already strong.
