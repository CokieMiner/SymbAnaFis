# CI Benchmark Results

**SymbAnaFis Version:** 0.6.0  
**Date:** Fri Jan  9 01:41:08 UTC 2026  
**Commit:** `59d8cecd4be8`  
**Rust:** 1.92.0  

> Auto-generated from Criterion benchmark output

## 1. Parsing (String → AST)

| Expression | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **3.27 µs** | 5.01 µs | **SymbAnaFis** (1.53x) |
| Damped Oscillator | **3.70 µs** | 5.63 µs | **SymbAnaFis** (1.52x) |
| Gaussian 2D | **6.49 µs** | 14.07 µs | **SymbAnaFis** (2.17x) |
| Lennard-Jones | **3.78 µs** | 7.71 µs | **SymbAnaFis** (2.04x) |
| Logistic Sigmoid | **2.97 µs** | 4.76 µs | **SymbAnaFis** (1.60x) |
| Lorentz Factor | **2.56 µs** | 5.02 µs | **SymbAnaFis** (1.96x) |
| Maxwell-Boltzmann | **7.74 µs** | 13.25 µs | **SymbAnaFis** (1.71x) |
| Normal PDF | **5.05 µs** | 10.07 µs | **SymbAnaFis** (2.00x) |
| Planck Blackbody | **4.99 µs** | 8.87 µs | **SymbAnaFis** (1.78x) |

---

## 2. Differentiation

| Expression | SymbAnaFis (Light) | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **2.18 µs** | 3.58 µs | **SymbAnaFis (Light)** (1.65x) |
| Damped Oscillator | **1.51 µs** | 3.51 µs | **SymbAnaFis (Light)** (2.33x) |
| Gaussian 2D | **1.49 µs** | 4.53 µs | **SymbAnaFis (Light)** (3.04x) |
| Lennard-Jones | **1.81 µs** | 3.95 µs | **SymbAnaFis (Light)** (2.18x) |
| Logistic Sigmoid | **860.22 ns** | 2.36 µs | **SymbAnaFis (Light)** (2.74x) |
| Lorentz Factor | **1.63 µs** | 3.78 µs | **SymbAnaFis (Light)** (2.31x) |
| Maxwell-Boltzmann | **2.45 µs** | 6.85 µs | **SymbAnaFis (Light)** (2.80x) |
| Normal PDF | **1.70 µs** | 3.40 µs | **SymbAnaFis (Light)** (2.00x) |
| Planck Blackbody | **2.12 µs** | 6.52 µs | **SymbAnaFis (Light)** (3.07x) |

---

## 3. Differentiation + Simplification

| Expression | SymbAnaFis (Full) | Speedup |
| :--- | :---: | :---: |
| Bessel Wave | 133.30 µs | - |
| Damped Oscillator | 154.78 µs | - |
| Gaussian 2D | 136.65 µs | - |
| Lennard-Jones | 29.23 µs | - |
| Logistic Sigmoid | 119.84 µs | - |
| Lorentz Factor | 267.78 µs | - |
| Maxwell-Boltzmann | 335.65 µs | - |
| Normal PDF | 148.44 µs | - |
| Planck Blackbody | 338.71 µs | - |

---

## 4. Simplification Only

| Expression | SymbAnaFis | Speedup |
| :--- | :---: | :---: |
| Bessel Wave | 129.67 µs | - |
| Damped Oscillator | 152.51 µs | - |
| Gaussian 2D | 132.73 µs | - |
| Lennard-Jones | 26.81 µs | - |
| Logistic Sigmoid | 118.55 µs | - |
| Lorentz Factor | 262.24 µs | - |
| Maxwell-Boltzmann | 331.95 µs | - |
| Normal PDF | 143.65 µs | - |
| Planck Blackbody | 334.23 µs | - |

---

## 5. Compilation

| Expression | SA (Simplified) | Symbolica | SA (Raw) | Speedup |
| :--- | :---: | :---: | :---: | :---: |
| Bessel Wave | **1.80 µs** | - | 2.11 µs | - |
| Damped Oscillator | **1.82 µs** | 14.91 µs | 2.04 µs | **SA (Simplified)** (8.21x) |
| Gaussian 2D | **1.98 µs** | 33.91 µs | 2.14 µs | **SA (Simplified)** (17.08x) |
| Lennard-Jones | 1.21 µs | 27.05 µs | **1.10 µs** | **SA (Simplified)** (22.31x) |
| Logistic Sigmoid | 1.53 µs | 9.88 µs | **1.34 µs** | **SA (Simplified)** (6.46x) |
| Lorentz Factor | **1.23 µs** | 9.73 µs | 1.50 µs | **SA (Simplified)** (7.91x) |
| Maxwell-Boltzmann | **2.02 µs** | 17.66 µs | 2.98 µs | **SA (Simplified)** (8.75x) |
| Normal PDF | **1.55 µs** | 18.11 µs | 1.69 µs | **SA (Simplified)** (11.67x) |
| Planck Blackbody | 2.95 µs | 10.99 µs | **2.91 µs** | **SA (Simplified)** (3.73x) |

---

## 6. Evaluation (1000 points)

| Expression | SA (Simplified) | Symbolica | SA (Raw) | Speedup |
| :--- | :---: | :---: | :---: | :---: |
| Bessel Wave | **197.66 µs** | - | 258.28 µs | - |
| Damped Oscillator | 89.31 µs | **58.16 µs** | 116.07 µs | **Symbolica** (1.54x) |
| Gaussian 2D | 109.37 µs | **64.43 µs** | 146.58 µs | **Symbolica** (1.70x) |
| Lennard-Jones | 93.77 µs | **63.15 µs** | 98.84 µs | **Symbolica** (1.48x) |
| Logistic Sigmoid | 131.57 µs | **51.09 µs** | 103.17 µs | **Symbolica** (2.58x) |
| Lorentz Factor | 72.69 µs | **54.48 µs** | 125.55 µs | **Symbolica** (1.33x) |
| Maxwell-Boltzmann | 131.58 µs | **83.34 µs** | 296.17 µs | **Symbolica** (1.58x) |
| Normal PDF | 100.47 µs | **65.14 µs** | 126.89 µs | **Symbolica** (1.54x) |
| Planck Blackbody | 166.75 µs | **63.45 µs** | 165.58 µs | **Symbolica** (2.63x) |

---

## 7. Full Pipeline

| Expression | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | 337.71 µs | - | - |
| Damped Oscillator | 268.24 µs | **175.91 µs** | **Symbolica** (1.52x) |
| Gaussian 2D | 284.99 µs | **171.10 µs** | **Symbolica** (1.67x) |
| Lennard-Jones | **127.32 µs** | 131.30 µs | **SymbAnaFis** (1.03x) |
| Logistic Sigmoid | 251.95 µs | **94.04 µs** | **Symbolica** (2.68x) |
| Lorentz Factor | 367.52 µs | **120.40 µs** | **Symbolica** (3.05x) |
| Maxwell-Boltzmann | 520.92 µs | **255.18 µs** | **Symbolica** (2.04x) |
| Normal PDF | 281.91 µs | **128.15 µs** | **Symbolica** (2.20x) |
| Planck Blackbody | 569.53 µs | **217.31 µs** | **Symbolica** (2.62x) |

---

## Parallel: Evaluation Methods (1k pts)

| Expression | Compiled Loop | Tree Walk | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **186.74 µs** | 1.26 ms | **Compiled Loop** (6.74x) |
| Damped Oscillator | **83.19 µs** | 1.03 ms | **Compiled Loop** (12.37x) |
| Gaussian 2D | **104.71 µs** | 1.83 ms | **Compiled Loop** (17.50x) |
| Lennard-Jones | **87.52 µs** | 682.19 µs | **Compiled Loop** (7.79x) |
| Logistic Sigmoid | **126.31 µs** | 1.09 ms | **Compiled Loop** (8.66x) |
| Lorentz Factor | **65.98 µs** | 826.81 µs | **Compiled Loop** (12.53x) |
| Maxwell-Boltzmann | **144.67 µs** | 1.31 ms | **Compiled Loop** (9.07x) |
| Normal PDF | **94.26 µs** | 1.05 ms | **Compiled Loop** (11.17x) |
| Planck Blackbody | **203.50 µs** | 2.02 ms | **Compiled Loop** (9.95x) |

---

## Parallel: Scaling (Points)

| Points | Eval Batch (SIMD) | Loop | Speedup |
| :--- | :---: | :---: | :---: |
| 100 | **2.17 µs** | 6.33 µs | **Eval Batch (SIMD)** (2.92x) |
| 1000 | **21.68 µs** | 63.38 µs | **Eval Batch (SIMD)** (2.92x) |
| 10000 | **217.33 µs** | 636.80 µs | **Eval Batch (SIMD)** (2.93x) |
| 100000 | **370.46 µs** | 6.35 ms | **Eval Batch (SIMD)** (17.13x) |

---

## Large Expressions (100 terms)

| Operation | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Parse | **144.45 µs** | 233.63 µs | **SA** (1.62x) |
| Diff (no simplify) | **84.09 µs** | 249.92 µs | **SA** (2.97x) |
| Diff+Simplify | 7.47 ms | — | — |
| Compile (simplified) | **35.71 µs** | 2.08 ms | **SA** (58.29x) |
| Eval 1000pts (simplified) | 4.77 ms | **3.75 ms** | **SY** (1.27x) |

---

## Large Expressions (300 terms)

| Operation | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Parse | **454.13 µs** | 722.22 µs | **SA** (1.59x) |
| Diff (no simplify) | **265.57 µs** | 788.66 µs | **SA** (2.97x) |
| Diff+Simplify | 21.91 ms | — | — |
| Compile (simplified) | **126.66 µs** | 14.45 ms | **SA** (114.11x) |
| Eval 1000pts (simplified) | 14.09 ms | **10.27 ms** | **SY** (1.37x) |

---