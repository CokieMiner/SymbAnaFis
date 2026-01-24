# CI Benchmark Results

**SymbAnaFis Version:** 0.7.0  
**Date:** Sat Jan 24 03:13:58 UTC 2026  
**Commit:** `b8edc0369dd6`  
**Rust:** 1.93.0  

> Auto-generated from Criterion benchmark output

## 1. Parsing (String → AST)

| Expression | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **3.30 µs** | 4.78 µs | **SymbAnaFis** (1.45x) |
| Damped Oscillator | **3.65 µs** | 5.24 µs | **SymbAnaFis** (1.44x) |
| Gaussian 2D | **6.57 µs** | 13.59 µs | **SymbAnaFis** (2.07x) |
| Lennard-Jones | **3.80 µs** | 7.38 µs | **SymbAnaFis** (1.94x) |
| Logistic Sigmoid | **2.98 µs** | 4.55 µs | **SymbAnaFis** (1.53x) |
| Lorentz Factor | **2.55 µs** | 4.61 µs | **SymbAnaFis** (1.81x) |
| Maxwell-Boltzmann | **7.89 µs** | 12.71 µs | **SymbAnaFis** (1.61x) |
| Normal PDF | **5.06 µs** | 9.76 µs | **SymbAnaFis** (1.93x) |
| Planck Blackbody | **4.90 µs** | 8.61 µs | **SymbAnaFis** (1.76x) |

---

## 2. Differentiation

| Expression | SymbAnaFis (Light) | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **2.09 µs** | 3.50 µs | **SymbAnaFis (Light)** (1.67x) |
| Damped Oscillator | **1.52 µs** | 3.38 µs | **SymbAnaFis (Light)** (2.23x) |
| Gaussian 2D | **1.30 µs** | 4.40 µs | **SymbAnaFis (Light)** (3.38x) |
| Lennard-Jones | **1.77 µs** | 3.84 µs | **SymbAnaFis (Light)** (2.17x) |
| Logistic Sigmoid | **828.50 ns** | 2.32 µs | **SymbAnaFis (Light)** (2.80x) |
| Lorentz Factor | **1.55 µs** | 3.69 µs | **SymbAnaFis (Light)** (2.38x) |
| Maxwell-Boltzmann | **2.33 µs** | 6.75 µs | **SymbAnaFis (Light)** (2.89x) |
| Normal PDF | **1.56 µs** | 3.35 µs | **SymbAnaFis (Light)** (2.14x) |
| Planck Blackbody | **1.95 µs** | 6.48 µs | **SymbAnaFis (Light)** (3.32x) |

---

## 3. Differentiation + Simplification

| Expression | SymbAnaFis (Full) | Speedup |
| :--- | :---: | :---: |
| Bessel Wave | 136.82 µs | - |
| Damped Oscillator | 163.09 µs | - |
| Gaussian 2D | 140.83 µs | - |
| Lennard-Jones | 30.34 µs | - |
| Logistic Sigmoid | 123.23 µs | - |
| Lorentz Factor | 270.85 µs | - |
| Maxwell-Boltzmann | 343.49 µs | - |
| Normal PDF | 152.74 µs | - |
| Planck Blackbody | 344.14 µs | - |

---

## 4. Simplification Only

| Expression | SymbAnaFis | Speedup |
| :--- | :---: | :---: |
| Bessel Wave | 133.39 µs | - |
| Damped Oscillator | 159.87 µs | - |
| Gaussian 2D | 139.14 µs | - |
| Lennard-Jones | 27.70 µs | - |
| Logistic Sigmoid | 122.61 µs | - |
| Lorentz Factor | 265.56 µs | - |
| Maxwell-Boltzmann | 339.37 µs | - |
| Normal PDF | 149.23 µs | - |
| Planck Blackbody | 341.51 µs | - |

---

## 5. Compilation

| Expression | SA (Simplified) | Symbolica | SA (Raw) | Speedup |
| :--- | :---: | :---: | :---: | :---: |
| Bessel Wave | **1.58 µs** | - | 1.62 µs | - |
| Damped Oscillator | **1.14 µs** | 15.04 µs | 1.37 µs | **SA (Simplified)** (13.14x) |
| Gaussian 2D | **1.41 µs** | 34.85 µs | 1.86 µs | **SA (Simplified)** (24.66x) |
| Lennard-Jones | **1.07 µs** | 27.93 µs | 1.11 µs | **SA (Simplified)** (26.13x) |
| Logistic Sigmoid | 961.64 ns | 9.78 µs | **948.62 ns** | **SA (Simplified)** (10.17x) |
| Lorentz Factor | **999.93 ns** | 9.53 µs | 1.55 µs | **SA (Simplified)** (9.53x) |
| Maxwell-Boltzmann | **2.11 µs** | 17.79 µs | 2.70 µs | **SA (Simplified)** (8.41x) |
| Normal PDF | **1.36 µs** | 18.45 µs | 1.69 µs | **SA (Simplified)** (13.56x) |
| Planck Blackbody | **1.91 µs** | 10.66 µs | 1.99 µs | **SA (Simplified)** (5.59x) |

---

## 6. Evaluation (1000 points)

| Expression | SA (Simplified) | Symbolica | SA (Raw) | Speedup |
| :--- | :---: | :---: | :---: | :---: |
| Bessel Wave | **256.57 µs** | - | 313.91 µs | - |
| Damped Oscillator | 95.88 µs | **57.75 µs** | 108.33 µs | **Symbolica** (1.66x) |
| Gaussian 2D | 99.37 µs | **64.47 µs** | 119.39 µs | **Symbolica** (1.54x) |
| Lennard-Jones | 69.49 µs | **63.29 µs** | 105.53 µs | **Symbolica** (1.10x) |
| Logistic Sigmoid | 70.26 µs | **50.76 µs** | 67.60 µs | **Symbolica** (1.38x) |
| Lorentz Factor | 73.94 µs | **54.52 µs** | 82.61 µs | **Symbolica** (1.36x) |
| Maxwell-Boltzmann | 139.37 µs | **81.72 µs** | 174.37 µs | **Symbolica** (1.71x) |
| Normal PDF | 87.36 µs | **65.64 µs** | 106.62 µs | **Symbolica** (1.33x) |
| Planck Blackbody | 133.62 µs | **63.01 µs** | 131.81 µs | **Symbolica** (2.12x) |

---

## 7. Full Pipeline

| Expression | SymbAnaFis | SA (No Diff Simp) | Symbolica | Speedup |
| :--- | :---: | :---: | :---: | :---: |
| Bessel Wave | 404.85 µs | **310.46 µs** | - | - |
| Damped Oscillator | 264.82 µs | **101.33 µs** | 165.76 µs | **Symbolica** (1.60x) |
| Gaussian 2D | 249.10 µs | **123.66 µs** | 149.49 µs | **Symbolica** (1.67x) |
| Lennard-Jones | 118.07 µs | **93.59 µs** | 118.31 µs | **SymbAnaFis** (1.00x) |
| Logistic Sigmoid | 198.30 µs | **60.82 µs** | 84.03 µs | **Symbolica** (2.36x) |
| Lorentz Factor | 336.37 µs | **76.21 µs** | 107.23 µs | **Symbolica** (3.14x) |
| Maxwell-Boltzmann | 498.66 µs | **178.55 µs** | 233.97 µs | **Symbolica** (2.13x) |
| Normal PDF | 248.97 µs | **100.40 µs** | 112.64 µs | **Symbolica** (2.21x) |
| Planck Blackbody | 489.22 µs | **122.72 µs** | 201.83 µs | **Symbolica** (2.42x) |

---

## Parallel: Evaluation Methods (1k pts)

| Expression | Compiled Loop | Tree Walk | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **225.45 µs** | 1.28 ms | **Compiled Loop** (5.66x) |
| Damped Oscillator | **87.82 µs** | 1.04 ms | **Compiled Loop** (11.80x) |
| Gaussian 2D | **92.15 µs** | 1.04 ms | **Compiled Loop** (11.33x) |
| Lennard-Jones | **64.97 µs** | 684.06 µs | **Compiled Loop** (10.53x) |
| Logistic Sigmoid | **53.54 µs** | 1.10 ms | **Compiled Loop** (20.56x) |
| Lorentz Factor | **66.38 µs** | 832.09 µs | **Compiled Loop** (12.54x) |
| Maxwell-Boltzmann | **133.71 µs** | 1.32 ms | **Compiled Loop** (9.88x) |
| Normal PDF | **81.51 µs** | 1.06 ms | **Compiled Loop** (13.01x) |
| Planck Blackbody | **130.83 µs** | 2.01 ms | **Compiled Loop** (15.33x) |

---

## Parallel: Scaling (Points)

| Points | Eval Batch (SIMD) | Loop | Speedup |
| :--- | :---: | :---: | :---: |
| 100 | **2.26 µs** | 5.64 µs | **Eval Batch (SIMD)** (2.49x) |
| 1000 | **22.40 µs** | 56.44 µs | **Eval Batch (SIMD)** (2.52x) |
| 10000 | **225.03 µs** | 563.05 µs | **Eval Batch (SIMD)** (2.50x) |
| 100000 | **373.50 µs** | 5.64 ms | **Eval Batch (SIMD)** (15.09x) |

---

## Large Expressions (100 terms)

| Operation | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Parse | **143.29 µs** | 238.17 µs | **SA** (1.66x) |
| Diff (no simplify) | **79.60 µs** | 248.16 µs | **SA** (3.12x) |
| Diff+Simplify | 7.68 ms | — | — |
| Compile (simplified) | **47.82 µs** | 2.21 ms | **SA** (46.21x) |
| Eval 1000pts (simplified) | 4.07 ms | **3.68 ms** | **SY** (1.11x) |

---

## Large Expressions (300 terms)

| Operation | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Parse | **456.66 µs** | 734.58 µs | **SA** (1.61x) |
| Diff (no simplify) | **249.78 µs** | 779.14 µs | **SA** (3.12x) |
| Diff+Simplify | 22.52 ms | — | — |
| Compile (simplified) | **205.15 µs** | 17.29 ms | **SA** (84.27x) |
| Eval 1000pts (simplified) | 11.82 ms | **10.34 ms** | **SY** (1.14x) |

---

