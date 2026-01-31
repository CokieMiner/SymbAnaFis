# Benchmark Results

**SymbAnaFis Version:** Unreleased Dev Build  
**Date:** 2026-01-31
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
| Parsing                   |                  1.45x | SA 1.3x-1.6x faster                     |
| Differentiation           |                  1.43x | SA 1.05x-2.06x faster                   |
| Compilation               |                  7.3x  | SA 2.2x-20.4x faster                    |
| Evaluation                |                  1.11x | SA competitive, faster on 5/8           |
| Full Pipeline (No Simp)   |                  1.82x | SA beats SY on all expressions          |
| Full Pipeline (With Simp) |                  0.58x | SY faster due to SA deep simplification |
---

## 1. Parsing (String → AST)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------: | -------------: | -----------------: |
| Normal PDF        |        2.859 µs |       4.473 µs |          **1.57x** |
| Gaussian 2D       |        3.870 µs |       6.230 µs |          **1.61x** |
| Maxwell-Boltzmann |        4.507 µs |       5.988 µs |          **1.33x** |
| Lorentz Factor    |        1.557 µs |       2.306 µs |          **1.48x** |
| Lennard-Jones     |        2.291 µs |       3.595 µs |          **1.57x** |
| Logistic Sigmoid  |        1.658 µs |       2.201 µs |          **1.33x** |
| Damped Oscillator |        1.875 µs |       2.612 µs |          **1.39x** |
| Planck Blackbody  |        2.779 µs |       3.994 µs |          **1.44x** |
| Bessel Wave       |        1.676 µs |       2.340 µs |          **1.40x** |

> **Result:** SymbAnaFis parses **1.3x - 1.6x** faster than Symbolica.

---

## 2. Differentiation (Light)

| Expression        | SA (diff_only) | Symbolica (diff) | SA Speedup |
| ----------------- | -------------: | ---------------: | ---------: |
| Normal PDF        |       1.308 µs |         1.673 µs |  **1.28x** |
| Gaussian 2D       |       1.076 µs |         2.212 µs |  **2.06x** |
| Maxwell-Boltzmann |       1.717 µs |         3.269 µs |  **1.90x** |
| Lorentz Factor    |       1.431 µs |         1.813 µs |  **1.27x** |
| Lennard-Jones     |       1.694 µs |         1.888 µs |  **1.11x** |
| Logistic Sigmoid  |       0.876 µs |         1.154 µs |  **1.32x** |
| Damped Oscillator |       1.169 µs |         1.629 µs |  **1.39x** |
| Planck Blackbody  |       1.784 µs |         3.065 µs |  **1.72x** |
| Bessel Wave       |       1.591 µs |         1.667 µs |  **1.05x** |

### SymbAnaFis Full Simplification Cost

| Expression        | SA diff_only | SA diff+simplify |
| ----------------- | -----------: | ---------------: |
| Normal PDF        |     1.308 µs |        78.967 µs |
| Gaussian 2D       |     1.076 µs |        76.584 µs |
| Maxwell-Boltzmann |     1.717 µs |       140.930 µs |
| Lorentz Factor    |     1.431 µs |       119.890 µs |
| Lennard-Jones     |     1.694 µs |        17.876 µs |
| Logistic Sigmoid  |     0.876 µs |        63.295 µs |
| Damped Oscillator |     1.169 µs |        62.966 µs |
| Planck Blackbody  |     1.784 µs |       161.320 µs |
| Bessel Wave       |     1.591 µs |        69.965 µs |

---

## 3. Compilation (Raw vs Simplified)

### Raw Compilation

| Expression        | SymbAnaFis (Raw) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | ---------------: | -------------: | -----------------: |
| Normal PDF        |         1.217 µs |       8.829 µs |           **7.3x** |
| Gaussian 2D       |         1.372 µs |       16.48 µs |          **12.0x** |
| Maxwell-Boltzmann |         1.334 µs |       8.388 µs |           **6.3x** |
| Lorentz Factor    |         1.179 µs |       4.864 µs |           **4.1x** |
| Lennard-Jones     |         0.637 µs |       13.01 µs |          **20.4x** |
| Logistic Sigmoid  |         1.139 µs |       4.966 µs |           **4.4x** |
| Damped Oscillator |         1.170 µs |       7.327 µs |           **6.3x** |
| Planck Blackbody  |         2.324 µs |       5.013 µs |           **2.2x** |
| Bessel Wave       |         1.481 µs |    *(skipped)* |                  — |

### Simplified Compilation

| Expression        | SymbAnaFis (Simplified) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | ----------------------: | -------------: | -----------------: |
| Normal PDF        |                1.217 µs |       8.829 µs |           **7.3x** |
| Gaussian 2D       |                1.372 µs |       16.48 µs |          **12.0x** |
| Maxwell-Boltzmann |                1.334 µs |       8.388 µs |           **6.3x** |
| Lorentz Factor    |                1.179 µs |       4.864 µs |           **4.1x** |
| Lennard-Jones     |                0.637 µs |       13.01 µs |          **20.4x** |
| Logistic Sigmoid  |                1.139 µs |       4.966 µs |           **4.4x** |
| Damped Oscillator |                1.170 µs |       7.327 µs |           **6.3x** |
| Planck Blackbody  |                2.324 µs |       5.013 µs |           **2.2x** |
| Bessel Wave       |                1.481 µs |    *(skipped)* |                  — |

> **Result:** SymbAnaFis compiles **2.2x - 20.4x** faster than Symbolica (avg ~7.3x).

## 4. Evaluation (Compiled, 1000 points)

### Raw Evaluation

| Expression        | SymbAnaFis (Raw) | Symbolica (SY) | SA vs SY |
| ----------------- | ---------------: | -------------: | -------: |
| Normal PDF        |         26.18 µs |       32.86 µs |    1.26x |
| Gaussian 2D       |         26.27 µs |       33.65 µs |    1.28x |
| Maxwell-Boltzmann |         40.52 µs |       45.05 µs |    1.11x |
| Lorentz Factor    |         20.96 µs |       32.38 µs |    1.54x |
| Lennard-Jones     |         32.05 µs |       34.64 µs |    1.08x |
| Logistic Sigmoid  |         23.45 µs |       29.89 µs |    1.27x |
| Damped Oscillator |         33.86 µs |       33.29 µs |    0.98x |
| Planck Blackbody  |         51.37 µs |       32.26 µs |    0.63x |
| Bessel Wave       |         82.24 µs |    *(skipped)* |        — |

### Simplified Evaluation

| Expression        | SymbAnaFis (Simplified) | Symbolica (SY) | SA vs SY |
| ----------------- | ----------------------: | -------------: | -------: |
| Normal PDF        |                26.18 µs |       32.86 µs |    1.26x |
| Gaussian 2D       |                26.27 µs |       33.65 µs |    1.28x |
| Maxwell-Boltzmann |                40.52 µs |       45.05 µs |    1.11x |
| Lorentz Factor    |                20.96 µs |       32.38 µs |    1.54x |
| Lennard-Jones     |                32.05 µs |       34.64 µs |    1.08x |
| Logistic Sigmoid  |                23.45 µs |       29.89 µs |    1.27x |
| Damped Oscillator |                33.86 µs |       33.29 µs |    0.98x |
| Planck Blackbody  |                51.37 µs |       32.26 µs |    0.63x |
| Bessel Wave       |                82.24 µs |    *(skipped)* |        — |

> **Result:** SymbAnaFis evaluation is competitive, with SA faster on 5/8 expressions (avg 1.11x vs SY).

---

## 5. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression        | SA (Full Simp) | SA (No Simp) | Symbolica (SY) | SA Full vs SY | SA No-Simp vs SY |
| ----------------- | -------------: | -----------: | -------------: | ------------: | ---------------: |
| Normal PDF        |      121.51 µs |     28.96 µs |       49.30 µs |         0.41x |        **1.70x** |
| Gaussian 2D       |      117.13 µs |     32.27 µs |       65.37 µs |         0.56x |        **2.03x** |
| Maxwell-Boltzmann |      191.42 µs |     48.20 µs |      109.02 µs |         0.57x |        **2.26x** |
| Lorentz Factor    |      144.70 µs |     28.36 µs |       53.71 µs |         0.37x |        **1.89x** |
| Lennard-Jones     |       54.64 µs |     37.67 µs |       56.69 µs |         1.04x |        **1.50x** |
| Logistic Sigmoid  |       88.56 µs |     22.33 µs |       43.13 µs |         0.49x |        **1.93x** |
| Damped Oscillator |      100.26 µs |     45.05 µs |       74.60 µs |         0.74x |        **1.66x** |
| Planck Blackbody  |      218.74 µs |     57.59 µs |       91.05 µs |         0.42x |        **1.58x** |
| Bessel Wave       |      158.00 µs |     98.09 µs |    *(skipped)* |             — |                — |

> **Key Finding:** Without full simplification, SymbAnaFis beats Symbolica on **all 8 expressions** (avg **1.82x faster**).
> The performance gap with full simplification is entirely due to deep algebraic restructuring (60-180µs overhead).

---

## 6. Large Expressions (100 / 300 terms) (medians)

### 100 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |   84.79 µs | 107.34 µs | **SA 1.27x** |
| Diff (no simplify)        |   65.68 µs | 114.41 µs | **SA 1.74x** |
| Diff+Simplify             |   2.294 ms |         — |            — |
| Compile (raw)             |  186.98 µs |  1.039 ms | **SA 5.56x** |
| Compile (simplified)      |   81.22 µs |  1.039 ms | **SA 12.8x** |
| Eval 1000pts (raw)        |   1.937 ms |  1.826 ms |      0.94x   |
| Eval 1000pts (simplified) |   1.143 ms |  1.826 ms | **SA 1.60x** |

### 300 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |  273.19 µs | 341.88 µs | **SA 1.25x** |
| Diff (no simplify)        |  207.25 µs | 374.08 µs | **SA 1.80x** |
| Diff+Simplify             |   6.976 ms |         — |            — |
| Compile (raw)             |   1.363 ms |  12.97 ms | **SA 9.50x** |
| Compile (simplified)      |  442.30 µs |  12.97 ms | **SA 29.3x** |
| Eval 1000pts (raw)        |   5.217 ms |  5.309 ms | **SA 1.02x** |
| Eval 1000pts (simplified) |   3.015 ms |  5.309 ms | **SA 1.76x** |

---

## 7. Tree-Walk vs Compiled Evaluation (medians)

| Expression        | Tree-Walk (1000 pts) | Compiled (1000 pts) |   Speedup |
| ----------------- | -------------------: | ------------------: | --------: |
| Normal PDF        |            510.09 µs |            27.40 µs | **18.6x** |
| Gaussian 2D       |            510.77 µs |            26.58 µs | **19.2x** |
| Maxwell-Boltzmann |            588.88 µs |            42.40 µs | **13.9x** |
| Lorentz Factor    |            373.11 µs |            21.47 µs | **17.4x** |
| Lennard-Jones     |            182.11 µs |            32.29 µs |  **5.6x** |
| Logistic Sigmoid  |            493.21 µs |            24.01 µs | **20.5x** |
| Damped Oscillator |            452.56 µs |            35.02 µs | **12.9x** |
| Planck Blackbody  |            893.24 µs |            54.08 µs | **16.5x** |
| Bessel Wave       |            575.77 µs |            82.62 µs |  **7.0x** |

---

## 8. Batch Evaluation Performance (SIMD)

| Points  | loop_evaluate | eval_batch (SIMD) |  Speedup |
| ------- | ------------: | ----------------: | -------: |
| 100     |      1.923 µs |          0.906 µs | **2.1x** |
| 1,000   |      19.13 µs |          9.066 µs | **2.1x** |
| 10,000  |     183.11 µs |         90.70  µs | **2.0x** |
| 100,000 |      1.914 ms |          0.908 ms | **2.1x** |

---

## 9. Multi-Expression Batch Evaluation

| Method                            |     Time | vs Sequential |
| --------------------------------- | -------: | ------------: |
| sequential_loops                  | 3.615 ms |      baseline |
| eval_batch_per_expr (SIMD)        | 1.864 ms |   1.9x faster |
| eval_f64_per_expr (SIMD+parallel) | 542.7 µs |   6.7x faster |

---

## 10. eval_f64 vs evaluate_parallel APIs

| API                        |      Time |
| -------------------------- | --------: |
| `eval_f64` (SIMD+parallel) |  32.94 µs |
| `evaluate_parallel`        | 230.01 µs |

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

