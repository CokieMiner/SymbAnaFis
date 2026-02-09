# Benchmark Results

**SymbAnaFis Version:** 0.7.1  
**Date:** 2026-02-09
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

| Category                  | Avg Speedup (SA vs SY) | Notes                                   |
|---------------------------|------------------------|-----------------------------------------|
| Parsing                   |                  1.43x | SA 1.26x-1.57x faster                   |
| Differentiation           |                  1.35x | SA 0.92x-2.00x faster                   |
| Compilation               |                  4.3x  | SA 1.3x-22.0x faster                    |
| Evaluation                |                  1.21x | SA competitive, faster on 6/8           |
| Full Pipeline (No Simp)   |                  1.82x | SA beats SY on all expressions          |
| Full Pipeline (With Simp) |                  0.60x | SY faster due to SA deep simplification |
---

## 1. Parsing (String → AST)

| Expression        | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | --------------: | -------------: | -----------------: |
| Normal PDF        |        2.814 µs |       4.378 µs |          **1.56x** |
| Gaussian 2D       |        3.884 µs |       6.081 µs |          **1.57x** |
| Maxwell-Boltzmann |        4.529 µs |       5.901 µs |          **1.30x** |
| Lorentz Factor    |        1.574 µs |       2.227 µs |          **1.41x** |
| Lennard-Jones     |        2.303 µs |       3.400 µs |          **1.48x** |
| Logistic Sigmoid  |        1.639 µs |       2.063 µs |          **1.26x** |
| Damped Oscillator |        1.743 µs |       2.487 µs |          **1.43x** |
| Planck Blackbody  |        2.823 µs |       3.898 µs |          **1.38x** |
| Bessel Wave       |        1.538 µs |       2.253 µs |          **1.47x** |

> **Result:** SymbAnaFis parses **1.3x - 1.6x** faster than Symbolica.

---

## 2. Differentiation (Light)

| Expression        | SA (diff_only) | Symbolica (diff) | SA Speedup |
| ----------------- | -------------: | ---------------: | ---------: |
| Normal PDF        |       1.380 µs |         1.614 µs |  **1.17x** |
| Gaussian 2D       |       1.084 µs |         2.174 µs |  **2.00x** |
| Maxwell-Boltzmann |       1.711 µs |         2.999 µs |  **1.75x** |
| Lorentz Factor    |       1.484 µs |         1.793 µs |  **1.21x** |
| Lennard-Jones     |       1.683 µs |         1.717 µs |  **1.02x** |
| Logistic Sigmoid  |       0.855 µs |         1.119 µs |  **1.31x** |
| Damped Oscillator |       1.202 µs |         1.472 µs |  **1.22x** |
| Planck Blackbody  |       1.787 µs |         2.814 µs |  **1.58x** |
| Bessel Wave       |       1.642 µs |         1.510 µs |  **0.92x** |

### SymbAnaFis Full Simplification Cost

| Expression        | SA diff_only | SA diff+simplify |
| ----------------- | -----------: | ---------------: |
| Normal PDF        |     1.380 µs |        69.66 µs |
| Gaussian 2D       |     1.084 µs |        66.25 µs |
| Maxwell-Boltzmann |     1.711 µs |       127.25 µs |
| Lorentz Factor    |     1.484 µs |       111.51 µs |
| Lennard-Jones     |     1.683 µs |        17.28 µs |
| Logistic Sigmoid  |     0.855 µs |        57.69 µs |
| Damped Oscillator |     1.202 µs |        54.54 µs |
| Planck Blackbody  |     1.787 µs |       150.10 µs |
| Bessel Wave       |     1.642 µs |        63.50 µs |

---

## 3. Compilation (Raw vs Simplified)

### Raw Compilation

| Expression        | SymbAnaFis (Raw) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | ---------------: | -------------: | -----------------: |
| Normal PDF        |         1.443 µs |       4.367 µs |           **3.0x** |
| Gaussian 2D       |         1.592 µs |       4.736 µs |           **3.0x** |
| Maxwell-Boltzmann |         2.140 µs |       7.722 µs |           **3.6x** |
| Lorentz Factor    |         1.410 µs |       2.133 µs |           **1.5x** |
| Lennard-Jones     |         0.651 µs |       12.83 µs |          **19.7x** |
| Logistic Sigmoid  |         0.968 µs |       2.548 µs |           **2.6x** |
| Damped Oscillator |         1.376 µs |       2.418 µs |           **1.8x** |
| Planck Blackbody  |         1.916 µs |       4.779 µs |           **2.5x** |
| Bessel Wave       |         1.707 µs |       2.211 µs |           **1.3x** |

### Simplified Compilation

| Expression        | SymbAnaFis (Simplified) | Symbolica (SY) | Speedup (SA vs SY) |
| ----------------- | ----------------------: | -------------: | -----------------: |
| Normal PDF        |                1.193 µs |       4.367 µs |           **3.7x** |
| Gaussian 2D       |                1.331 µs |       4.736 µs |           **3.6x** |
| Maxwell-Boltzmann |                1.300 µs |       7.722 µs |           **5.9x** |
| Lorentz Factor    |                1.148 µs |       2.133 µs |           **1.9x** |
| Lennard-Jones     |                0.583 µs |       12.83 µs |          **22.0x** |
| Logistic Sigmoid  |                1.117 µs |       2.548 µs |           **2.3x** |
| Damped Oscillator |                1.154 µs |       2.418 µs |           **2.1x** |
| Planck Blackbody  |                2.310 µs |       4.779 µs |           **2.1x** |
| Bessel Wave       |                1.468 µs |       2.211 µs |           **1.5x** |

> **Result:** SymbAnaFis compiles **2.6x - 19.4x** faster than Symbolica (avg ~7.0x).

## 4. Evaluation (Compiled, 1000 points)

### Raw Evaluation

| Expression        | SymbAnaFis (Raw) | Symbolica (SY) | SA vs SY |
| ----------------- | ---------------: | -------------: | -------: |
| Normal PDF        |         21.72 µs |       32.35 µs |    1.49x |
| Gaussian 2D       |         24.57 µs |       33.98 µs |    1.38x |
| Maxwell-Boltzmann |         39.49 µs |       41.50 µs |    1.05x |
| Lorentz Factor    |         21.93 µs |       32.31 µs |    1.47x |
| Lennard-Jones     |         32.23 µs |       34.83 µs |    1.08x |
| Logistic Sigmoid  |         18.05 µs |       30.61 µs |    1.70x |
| Damped Oscillator |         39.84 µs |       33.26 µs |    0.84x |
| Planck Blackbody  |         48.62 µs |       32.22 µs |    0.66x |
| Bessel Wave       |         91.54 µs |    *(skipped)* |        — |

### Simplified Evaluation

| Expression        | SymbAnaFis (Simplified) | Symbolica (SY) | SA vs SY |
| ----------------- | ----------------------: | -------------: | -------: |
| Normal PDF        |                25.20 µs |       32.35 µs |    1.28x |
| Gaussian 2D       |                25.87 µs |       33.98 µs |    1.31x |
| Maxwell-Boltzmann |                40.41 µs |       41.50 µs |    1.03x |
| Lorentz Factor    |                20.72 µs |       32.31 µs |    1.56x |
| Lennard-Jones     |                31.71 µs |       34.83 µs |    1.10x |
| Logistic Sigmoid  |                22.56 µs |       30.61 µs |    1.36x |
| Damped Oscillator |                35.04 µs |       33.26 µs |    0.95x |
| Planck Blackbody  |                51.67 µs |       32.22 µs |    0.62x |
| Bessel Wave       |                81.78 µs |    *(skipped)* |        — |

> **Result:** SymbAnaFis evaluation is competitive, with SA faster on 6/8 expressions (avg 1.29x vs SY).

---

## 5. Full Pipeline (Parse → Diff → Simplify → Compile → Eval 1000 pts)

| Expression        | SA (Full Simp) | SA (No Simp) | Symbolica (SY) | SA Full vs SY | SA No-Simp vs SY |
| ----------------- | -------------: | -----------: | -------------: | ------------: | ---------------: |
| Normal PDF        |      112.04 µs |     27.84 µs |       48.02 µs |         0.43x |        **1.72x** |
| Gaussian 2D       |      106.53 µs |     31.75 µs |       54.29 µs |         0.51x |        **1.71x** |
| Maxwell-Boltzmann |      179.36 µs |     48.03 µs |      108.55 µs |         0.60x |        **2.26x** |
| Lorentz Factor    |      138.29 µs |     26.88 µs |       53.40 µs |         0.39x |        **1.99x** |
| Lennard-Jones     |       52.68 µs |     37.03 µs |       57.71 µs |         1.10x |        **1.56x** |
| Logistic Sigmoid  |       82.96 µs |     21.87 µs |       42.69 µs |         0.52x |        **1.95x** |
| Damped Oscillator |       94.76 µs |     44.70 µs |       73.26 µs |         0.77x |        **1.64x** |
| Planck Blackbody  |      207.38 µs |     57.71 µs |      100.03 µs |         0.48x |        **1.73x** |
| Bessel Wave       |      154.79 µs |     99.91 µs |    *(skipped)* |             — |                — |

> **Key Finding:** Without full simplification, SymbAnaFis beats Symbolica on **all 8 expressions** (avg **2.0x faster**).
> The performance gap with full simplification is entirely due to deep algebraic restructuring (60-180µs overhead).

---

## 6. Large Expressions (100 / 300 terms) (medians)

### 100 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |   80.47 µs |  90.27 µs | **SA 1.12x** |
| Diff (no simplify)        |   64.93 µs |  79.46 µs | **SA 1.22x** |
| Diff+Simplify             |   1.630 ms |         — |            — |
| Compile (raw)             |  188.48 µs |  1.805 ms | **SA 9.58x** |
| Compile (simplified)      |   81.52 µs |  1.805 ms | **SA 22.1x** |
| Eval 1000pts (raw)        |   1.941 ms |  1.674 ms |      0.86x   |
| Eval 1000pts (simplified) |   1.066 ms |  1.674 ms | **SA 1.57x** |

### 300 Terms

| Operation                 | SymbAnaFis | Symbolica |      Speedup |
| ------------------------- | ---------: | --------: | -----------: |
| Parse                     |  263.68 µs | 273.34 µs | **SA 1.04x** |
| Diff (no simplify)        |  204.02 µs | 234.66 µs | **SA 1.15x** |
| Diff+Simplify             |   5.208 ms |         — |            — |
| Compile (raw)             |   1.388 ms |   9.942 ms | **SA 7.16x** |
| Compile (simplified)      |  445.63 µs |   9.942 ms | **SA 22.3x** |
| Eval 1000pts (raw)        |   5.130 ms |   4.871 ms |      0.95x   |
| Eval 1000pts (simplified) |   2.968 ms |   4.871 ms | **SA 1.64x** |

---

## 7. Tree-Walk vs Compiled Evaluation (medians)

| Expression        | Tree-Walk (1000 pts) | Compiled (1000 pts) |   Speedup |
| ----------------- | -------------------: | ------------------: | --------: |
| Normal PDF        |            495.60 µs |            26.05 µs | **19.0x** |
| Gaussian 2D       |            503.30 µs |            26.35 µs | **19.1x** |
| Maxwell-Boltzmann |            583.32 µs |            42.02 µs | **13.9x** |
| Lorentz Factor    |            373.72 µs |            21.80 µs | **17.1x** |
| Lennard-Jones     |            180.37 µs |            31.31 µs |  **5.8x** |
| Logistic Sigmoid  |            491.96 µs |            22.98 µs | **21.4x** |
| Damped Oscillator |            451.07 µs |            35.90 µs | **12.6x** |
| Planck Blackbody  |            884.91 µs |            52.27 µs | **16.9x** |
| Bessel Wave       |            575.84 µs |            82.74 µs |  **7.0x** |

---

## 8. Batch Evaluation Performance (SIMD)

| Points  | loop_evaluate | eval_batch (SIMD) |  Speedup |
| ------- | ------------: | ----------------: | -------: |
| 100     |      1.736 µs |          0.925 µs | **1.9x** |
| 1,000   |      18.82 µs |          9.209 µs | **2.0x** |
| 10,000  |     176.46 µs |         91.91 µs | **1.9x** |
| 100,000 |      1.803 ms |          0.920 ms | **2.0x** |

---

## 9. Multi-Expression Batch Evaluation

| Method                            |     Time | vs Sequential |
| --------------------------------- | -------: | ------------: |
| sequential_loops                  | 3.697 ms |      baseline |
| eval_batch_per_expr (SIMD)        | 1.872 ms |   2.0x faster |
| eval_f64_per_expr (SIMD+parallel) | 529.0 µs |   7.0x faster |

---

## 10. eval_f64 vs evaluate_parallel APIs

| API                        |      Time |
| -------------------------- | --------: |
| `eval_f64` (SIMD+parallel) |  32.86 µs |
| `evaluate_parallel`        | 176.29 µs |

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
