# Benchmark Results

**SymbAnaFis Version:** 0.8.1  
**Date:** 2026-02-11
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
| Parsing                   |                  1.52x | SA 1.39x-1.69x faster                          |
| Differentiation           |                  1.32x | SA 0.93x-1.92x faster                          |
| Compilation (Raw)         |                  3.47x | SA 0.95x-15.40x faster                         |
| Evaluation (Raw)          |                  1.03x | SA competitive (faster on 5/8 comparable)      |
| Full Pipeline (No Simp)   |                  1.85x | SA beats SY on all 8 comparable expressions    |
| Full Pipeline (With Simp) |                  0.64x | SY faster due to SA deep simplification cost   |

---

## 1. Parsing (String → AST)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------: | -------------: | -----------------: |
| Normal PDF        |        2.770 µs |       4.299 µs |          **1.55x** |
| Gaussian 2D       |        3.504 µs |       5.906 µs |          **1.69x** |
| Maxwell-Boltzmann |        4.099 µs |       5.822 µs |          **1.42x** |
| Lorentz Factor    |        1.366 µs |       2.206 µs |          **1.62x** |
| Lennard-Jones     |        2.114 µs |       3.349 µs |          **1.58x** |
| Logistic Sigmoid  |        1.496 µs |       2.074 µs |          **1.39x** |
| Damped Oscillator |        1.646 µs |       2.425 µs |          **1.47x** |
| Planck Blackbody  |        2.540 µs |       3.848 µs |          **1.51x** |
| Bessel Wave       |        1.535 µs |       2.221 µs |          **1.45x** |

---

## 2. Differentiation (Light)

| Expression        | SA (diff_only) | Symbolica (diff) | SA Speedup |
| ----------------- | -------------: | ---------------: | ---------: |
| Normal PDF        |       1.339 µs |         1.605 µs |  **1.20x** |
| Gaussian 2D       |       1.104 µs |         2.122 µs |  **1.92x** |
| Maxwell-Boltzmann |       1.802 µs |         2.957 µs |  **1.64x** |
| Lorentz Factor    |       1.446 µs |         1.760 µs |  **1.22x** |
| Lennard-Jones     |       1.655 µs |         1.687 µs |  **1.02x** |
| Logistic Sigmoid  |     853.00 ns  |         1.095 µs |  **1.28x** |
| Damped Oscillator |       1.252 µs |         1.459 µs |  **1.17x** |
| Planck Blackbody  |       1.835 µs |         2.809 µs |  **1.53x** |
| Bessel Wave       |       1.612 µs |         1.493 µs |  **0.93x** |

### SymbAnaFis Full Simplification Cost

| Expression        | SA diff_only | SA diff+simplify |
| ----------------- | -----------: | ---------------: |
| Normal PDF        |     1.339 µs |        67.17 µs  |
| Gaussian 2D       |     1.104 µs |        53.14 µs  |
| Maxwell-Boltzmann |     1.802 µs |       121.86 µs  |
| Lorentz Factor    |     1.446 µs |        90.04 µs  |
| Lennard-Jones     |     1.655 µs |        16.34 µs  |
| Logistic Sigmoid  |   853.00 ns  |        55.09 µs  |
| Damped Oscillator |     1.252 µs |        52.44 µs  |
| Planck Blackbody  |     1.835 µs |       144.94 µs  |
| Bessel Wave       |     1.612 µs |        62.10 µs  |

---

## 3. Compilation (Raw vs Simplified)

### Raw Compilation

| Expression        | SymbAnaFis (Raw) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | ---------------: | -------------: | -----------------: |
| Normal PDF        |         2.162 µs |       4.386 µs |           **2.03x** |
| Gaussian 2D       |         2.359 µs |       4.743 µs |           **2.01x** |
| Maxwell-Boltzmann |         3.083 µs |       7.791 µs |           **2.53x** |
| Lorentz Factor    |         2.232 µs |       2.124 µs |            0.95x   |
| Lennard-Jones     |       843.40 ns  |      12.985 µs |          **15.40x** |
| Logistic Sigmoid  |         1.316 µs |       2.546 µs |           **1.94x** |
| Damped Oscillator |         1.638 µs |       2.387 µs |           **1.46x** |
| Planck Blackbody  |         3.375 µs |       4.781 µs |           **1.42x** |
| Bessel Wave       |         2.168 µs |    *(skipped)* |                —   |

### Simplified Compilation

| Expression        | SymbAnaFis (Simplified) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | ----------------------: | -------------: | -----------------: |
| Normal PDF        |                1.744 µs |       4.386 µs |           **2.51x** |
| Gaussian 2D       |                1.954 µs |       4.743 µs |           **2.43x** |
| Maxwell-Boltzmann |                2.244 µs |       7.791 µs |           **3.47x** |
| Lorentz Factor    |                1.411 µs |       2.124 µs |           **1.51x** |
| Lennard-Jones     |              843.17 ns  |      12.985 µs |          **15.40x** |
| Logistic Sigmoid  |                1.611 µs |       2.546 µs |           **1.58x** |
| Damped Oscillator |                1.435 µs |       2.387 µs |           **1.66x** |
| Planck Blackbody  |                3.421 µs |       4.781 µs |           **1.40x** |
| Bessel Wave       |                1.980 µs |    *(skipped)* |                —   |

---

## 4. Evaluation (Compiled, 1000 points)

### Raw Evaluation

| Expression        | SymbAnaFis (Raw) | Symbolica (SY) | SA vs SY |
| ----------------- | ---------------: | -------------: | -------: |
| Normal PDF        |         21.328 µs |       27.673 µs |    1.30x |
| Gaussian 2D       |         24.299 µs |       29.091 µs |    1.20x |
| Maxwell-Boltzmann |         39.229 µs |       34.480 µs |    0.88x |
| Lorentz Factor    |         22.314 µs |       27.517 µs |    1.23x |
| Lennard-Jones     |         32.006 µs |       30.185 µs |    0.94x |
| Logistic Sigmoid  |         17.587 µs |       25.138 µs |    1.43x |
| Damped Oscillator |         37.486 µs |       28.175 µs |    0.75x |
| Planck Blackbody  |         50.397 µs |       26.749 µs |    0.53x |
| Bessel Wave       |         66.219 µs |    *(skipped)* |        — |

### Simplified Evaluation

| Expression        | SymbAnaFis (Simplified) | Symbolica (SY) | SA vs SY |
| ----------------- | ----------------------: | -------------: | -------: |
| Normal PDF        |                25.250 µs |       27.673 µs |    1.10x |
| Gaussian 2D       |                25.149 µs |       29.091 µs |    1.16x |
| Maxwell-Boltzmann |                39.083 µs |       34.480 µs |    0.88x |
| Lorentz Factor    |                17.354 µs |       27.517 µs |    1.59x |
| Lennard-Jones     |                31.455 µs |       30.185 µs |    0.96x |
| Logistic Sigmoid  |                21.223 µs |       25.138 µs |    1.18x |
| Damped Oscillator |                34.535 µs |       28.175 µs |    0.82x |
| Planck Blackbody  |                51.523 µs |       26.749 µs |    0.52x |
| Bessel Wave       |                60.073 µs |    *(skipped)* |        — |

---

## 5. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression        | SA (Full Simp) | SA (No Simp) | Symbolica (SY) | SA Full vs SY | SA No-Simp vs SY |
| ----------------- | -------------: | -----------: | -------------: | ------------: | ---------------: |
| Normal PDF        |      108.22 µs |     27.915 µs |       48.819 µs |         0.45x |        **1.75x** |
| Gaussian 2D       |       90.804 µs |     31.825 µs |       54.217 µs |         0.60x |        **1.70x** |
| Maxwell-Boltzmann |      171.03 µs |     48.331 µs |      110.34 µs |         0.65x |        **2.28x** |
| Lorentz Factor    |      113.10 µs |     27.133 µs |       54.234 µs |         0.48x |        **2.00x** |
| Lennard-Jones     |       50.756 µs |     37.000 µs |       57.841 µs |      **1.14x** |        **1.56x** |
| Logistic Sigmoid  |       79.914 µs |     21.380 µs |       42.785 µs |         0.54x |        **2.00x** |
| Damped Oscillator |       92.074 µs |     42.837 µs |       73.670 µs |         0.80x |        **1.72x** |
| Planck Blackbody  |      199.30 µs |     57.156 µs |      100.10 µs |         0.50x |        **1.75x** |
| Bessel Wave       |      125.82 µs |     71.715 µs |    *(skipped)* |             — |                — |

---

## 6. Large Expressions (100 / 300 terms) (medians)

### 100 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |   84.261 µs |  91.886 µs | **SA 1.09x** |
| Diff (no simplify)        |   67.660 µs |  78.134 µs | **SA 1.15x** |
| Diff+Simplify             |    1.7817 ms |         — |            — |
| Compile (raw)             |   76.152 µs |   1.7657 ms | **SA 23.19x** |
| Compile (simplified)      |   78.314 µs |   1.7657 ms | **SA 22.55x** |
| Eval 1000pts (raw)        |    1.7116 ms |   1.6921 ms |      0.99x   |
| Eval 1000pts (simplified) |    1.7729 ms |   1.6921 ms |      0.95x   |

### 300 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |  266.73 µs | 274.03 µs | **SA 1.03x** |
| Diff (no simplify)        |  212.58 µs | 233.57 µs | **SA 1.10x** |
| Diff+Simplify             |    5.3511 ms |         — |            — |
| Compile (raw)             |  237.12 µs |   9.4299 ms | **SA 39.77x** |
| Compile (simplified)      |  243.25 µs |   9.4299 ms | **SA 38.77x** |
| Eval 1000pts (raw)        |    4.7112 ms |   4.8462 ms | **SA 1.03x** |
| Eval 1000pts (simplified) |    4.8548 ms |   4.8462 ms |      1.00x   |

---

## 7. Tree-Walk vs Compiled Evaluation (medians)

| Expression        | Tree-Walk (1000 pts) | Compiled (1000 pts) |   Speedup |
| ----------------- | -------------------: | ------------------: | --------: |
| Normal PDF        |            492.01 µs |            25.932 µs | **18.98x** |
| Gaussian 2D       |            498.14 µs |            25.407 µs | **19.61x** |
| Maxwell-Boltzmann |            574.86 µs |            39.217 µs | **14.66x** |
| Lorentz Factor    |            295.32 µs |            19.360 µs | **15.25x** |
| Lennard-Jones     |            180.17 µs |            30.606 µs |  **5.89x** |
| Logistic Sigmoid  |            489.09 µs |            21.883 µs | **22.35x** |
| Damped Oscillator |            443.54 µs |            34.425 µs | **12.89x** |
| Planck Blackbody  |            877.51 µs |            49.911 µs | **17.58x** |
| Bessel Wave       |            575.76 µs |            61.379 µs |  **9.38x** |

---

## 8. Batch Evaluation Performance (SIMD)

| Points  | loop_evaluate | eval_batch (SIMD) |  Speedup |
| ------- | ------------: | ----------------: | -------: |
| 100     |      1.1223 µs |         329.62 ns | **3.40x** |
| 1,000   |      11.039 µs |          3.2360 µs | **3.41x** |
| 10,000  |      110.17 µs |         32.510 µs | **3.39x** |
| 100,000 |       1.1265 ms |         324.23 µs | **3.47x** |

---

## 9. Multi-Expression Batch Evaluation

| Method                            |     Time | vs Sequential |
| --------------------------------- | -------: | ------------: |
| sequential_loops                  | 3.0044 ms |      baseline |
| eval_batch_per_expr (SIMD)        | 1.4234 ms |   2.11x faster |
| eval_f64_per_expr (SIMD+parallel) | 511.55 µs |   5.87x faster |

---

## 10. eval_f64 vs evaluate_parallel APIs

| API                        |      Time |
| -------------------------- | --------: |
| `eval_f64` (SIMD+parallel) |  25.388 µs |
| `evaluate_parallel`        | 170.46 µs |

---

## Use Cases and Recommendations

### When to Use Full Simplification
- **Complex/large expressions**: Where deep algebraic restructuring can significantly reduce evaluation time.
- **High-performance scenarios**: When expression reuse amortizes simplification cost.

### When to Skip Full Simplification
- **Small/simple expressions**: Where simplification overhead outweighs runtime savings.
- **One-off evaluations**: If expressions are evaluated only a few times.
- **Latency-sensitive diff/compile paths**: Diff-only and raw compile are already strong.
