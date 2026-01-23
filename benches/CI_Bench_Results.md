# CI Benchmark Results

**SymbAnaFis Version:** 0.7.0  
**Date:** Fri Jan 23 20:26:25 UTC 2026  
**Commit:** `2a6ccb9f6379`  
**Rust:** 1.93.0  

> Auto-generated from Criterion benchmark output

## 1. Parsing (String → AST)

| Expression | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **3.16 µs** | 4.76 µs | **SymbAnaFis** (1.51x) |
| Damped Oscillator | **3.57 µs** | 5.33 µs | **SymbAnaFis** (1.49x) |
| Gaussian 2D | **6.40 µs** | 13.57 µs | **SymbAnaFis** (2.12x) |
| Lennard-Jones | **3.76 µs** | 7.54 µs | **SymbAnaFis** (2.00x) |
| Logistic Sigmoid | **2.85 µs** | 4.67 µs | **SymbAnaFis** (1.64x) |
| Lorentz Factor | **2.39 µs** | 4.79 µs | **SymbAnaFis** (2.01x) |
| Maxwell-Boltzmann | **7.85 µs** | 12.95 µs | **SymbAnaFis** (1.65x) |
| Normal PDF | **4.98 µs** | 9.68 µs | **SymbAnaFis** (1.94x) |
| Planck Blackbody | **4.78 µs** | 8.60 µs | **SymbAnaFis** (1.80x) |

---

## 2. Differentiation

| Expression | SymbAnaFis (Light) | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **2.07 µs** | 3.50 µs | **SymbAnaFis (Light)** (1.69x) |
| Damped Oscillator | **1.47 µs** | 3.42 µs | **SymbAnaFis (Light)** (2.33x) |
| Gaussian 2D | **1.32 µs** | 4.45 µs | **SymbAnaFis (Light)** (3.37x) |
| Lennard-Jones | **1.77 µs** | 3.84 µs | **SymbAnaFis (Light)** (2.17x) |
| Logistic Sigmoid | **819.02 ns** | 2.35 µs | **SymbAnaFis (Light)** (2.87x) |
| Lorentz Factor | **1.52 µs** | 3.77 µs | **SymbAnaFis (Light)** (2.47x) |
| Maxwell-Boltzmann | **2.31 µs** | 6.82 µs | **SymbAnaFis (Light)** (2.96x) |
| Normal PDF | **1.55 µs** | 3.37 µs | **SymbAnaFis (Light)** (2.18x) |
| Planck Blackbody | **1.94 µs** | 6.52 µs | **SymbAnaFis (Light)** (3.36x) |

---

## 3. Differentiation + Simplification

| Expression | SymbAnaFis (Full) | Speedup |
| :--- | :---: | :---: |
| Bessel Wave | 131.33 µs | - |
| Damped Oscillator | 157.64 µs | - |
| Gaussian 2D | 134.00 µs | - |
| Lennard-Jones | 28.54 µs | - |
| Logistic Sigmoid | 118.39 µs | - |
| Lorentz Factor | 261.11 µs | - |
| Maxwell-Boltzmann | 332.09 µs | - |
| Normal PDF | 145.20 µs | - |
| Planck Blackbody | 335.18 µs | - |

---

## 4. Simplification Only

| Expression | SymbAnaFis | Speedup |
| :--- | :---: | :---: |
| Bessel Wave | 127.28 µs | - |
| Damped Oscillator | 154.89 µs | - |
| Gaussian 2D | 131.87 µs | - |
| Lennard-Jones | 26.19 µs | - |
| Logistic Sigmoid | 116.34 µs | - |
| Lorentz Factor | 256.70 µs | - |
| Maxwell-Boltzmann | 328.39 µs | - |
| Normal PDF | 143.08 µs | - |
| Planck Blackbody | 332.11 µs | - |

---

## 5. Compilation

| Expression | SA (Simplified) | Symbolica | SA (Raw) | Speedup |
| :--- | :---: | :---: | :---: | :---: |
| Bessel Wave | **1.56 µs** | - | 1.62 µs | - |
| Damped Oscillator | **1.26 µs** | 14.98 µs | 1.42 µs | **SA (Simplified)** (11.92x) |
| Gaussian 2D | **1.88 µs** | 34.94 µs | 2.04 µs | **SA (Simplified)** (18.61x) |
| Lennard-Jones | **1.18 µs** | 27.78 µs | 1.33 µs | **SA (Simplified)** (23.62x) |
| Logistic Sigmoid | **959.39 ns** | 9.98 µs | 1.08 µs | **SA (Simplified)** (10.41x) |
| Lorentz Factor | **995.79 ns** | 9.73 µs | 1.63 µs | **SA (Simplified)** (9.77x) |
| Maxwell-Boltzmann | **2.24 µs** | 17.94 µs | 2.49 µs | **SA (Simplified)** (7.99x) |
| Normal PDF | **1.59 µs** | 18.34 µs | 1.83 µs | **SA (Simplified)** (11.51x) |
| Planck Blackbody | **2.08 µs** | 10.90 µs | 2.19 µs | **SA (Simplified)** (5.24x) |

---

## 6. Evaluation (1000 points)

| Expression | SA (Simplified) | Symbolica | SA (Raw) | Speedup |
| :--- | :---: | :---: | :---: | :---: |
| Bessel Wave | **243.50 µs** | - | 303.30 µs | - |
| Damped Oscillator | 95.06 µs | **57.90 µs** | 110.14 µs | **Symbolica** (1.64x) |
| Gaussian 2D | 110.10 µs | **64.68 µs** | 116.63 µs | **Symbolica** (1.70x) |
| Lennard-Jones | 72.87 µs | **63.25 µs** | 109.96 µs | **Symbolica** (1.15x) |
| Logistic Sigmoid | 79.99 µs | **50.81 µs** | 71.50 µs | **Symbolica** (1.57x) |
| Lorentz Factor | 75.54 µs | **54.51 µs** | 85.57 µs | **Symbolica** (1.39x) |
| Maxwell-Boltzmann | 150.95 µs | **82.33 µs** | 160.72 µs | **Symbolica** (1.83x) |
| Normal PDF | 102.35 µs | **63.44 µs** | 102.78 µs | **Symbolica** (1.61x) |
| Planck Blackbody | 138.26 µs | **63.67 µs** | 139.02 µs | **Symbolica** (2.17x) |

---

## 7. Full Pipeline

| Expression | SymbAnaFis | SA (No Diff Simp) | Symbolica | Speedup |
| :--- | :---: | :---: | :---: | :---: |
| Bessel Wave | 402.97 µs | **290.89 µs** | - | - |
| Damped Oscillator | 270.82 µs | **116.88 µs** | 165.07 µs | **Symbolica** (1.64x) |
| Gaussian 2D | 256.66 µs | **120.88 µs** | 148.42 µs | **Symbolica** (1.73x) |
| Lennard-Jones | 126.60 µs | **99.23 µs** | 118.45 µs | **Symbolica** (1.07x) |
| Logistic Sigmoid | 206.00 µs | **70.44 µs** | 83.45 µs | **Symbolica** (2.47x) |
| Lorentz Factor | 340.83 µs | **85.53 µs** | 106.84 µs | **Symbolica** (3.19x) |
| Maxwell-Boltzmann | 499.86 µs | **188.39 µs** | 233.98 µs | **Symbolica** (2.14x) |
| Normal PDF | 262.97 µs | **100.77 µs** | 112.67 µs | **Symbolica** (2.33x) |
| Planck Blackbody | 494.47 µs | **132.48 µs** | 201.71 µs | **Symbolica** (2.45x) |

---

## Parallel: Evaluation Methods (1k pts)

| Expression | Compiled Loop | Tree Walk | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **276.00 µs** | 1.24 ms | **Compiled Loop** (4.49x) |
| Damped Oscillator | **82.89 µs** | 1.02 ms | **Compiled Loop** (12.33x) |
| Gaussian 2D | **102.68 µs** | 1.04 ms | **Compiled Loop** (10.09x) |
| Lennard-Jones | **63.30 µs** | 670.38 µs | **Compiled Loop** (10.59x) |
| Logistic Sigmoid | **73.48 µs** | 1.11 ms | **Compiled Loop** (15.05x) |
| Lorentz Factor | **69.33 µs** | 827.66 µs | **Compiled Loop** (11.94x) |
| Maxwell-Boltzmann | **123.37 µs** | 1.31 ms | **Compiled Loop** (10.58x) |
| Normal PDF | **97.63 µs** | 1.07 ms | **Compiled Loop** (11.00x) |
| Planck Blackbody | **126.07 µs** | 2.00 ms | **Compiled Loop** (15.90x) |

---

## Parallel: Scaling (Points)

| Points | Eval Batch (SIMD) | Loop | Speedup |
| :--- | :---: | :---: | :---: |
| 100 | **2.31 µs** | 6.44 µs | **Eval Batch (SIMD)** (2.79x) |
| 1000 | **22.90 µs** | 64.39 µs | **Eval Batch (SIMD)** (2.81x) |
| 10000 | **227.84 µs** | 644.06 µs | **Eval Batch (SIMD)** (2.83x) |
| 100000 | **371.24 µs** | 6.46 ms | **Eval Batch (SIMD)** (17.40x) |

---

## Large Expressions (100 terms)

| Operation | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Parse | **139.76 µs** | 238.49 µs | **SA** (1.71x) |
| Diff (no simplify) | **79.67 µs** | 247.71 µs | **SA** (3.11x) |
| Diff+Simplify | 7.43 ms | — | — |
| Compile (simplified) | **39.07 µs** | 2.22 ms | **SA** (56.78x) |
| Eval 1000pts (simplified) | **3.58 ms** | 3.72 ms | **SA** (1.04x) |

---

## Large Expressions (300 terms)

| Operation | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Parse | **442.70 µs** | 735.61 µs | **SA** (1.66x) |
| Diff (no simplify) | **251.34 µs** | 784.84 µs | **SA** (3.12x) |
| Diff+Simplify | 21.46 ms | — | — |
| Compile (simplified) | **130.43 µs** | 14.84 ms | **SA** (113.74x) |
| Eval 1000pts (simplified) | 10.61 ms | **10.50 ms** | **SY** (1.01x) |

---

