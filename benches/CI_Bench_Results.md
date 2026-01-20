# CI Benchmark Results

**SymbAnaFis Version:** 0.7.0  
**Date:** Tue Jan 20 18:21:56 UTC 2026  
**Commit:** `d7d7f6e4e241`  
**Rust:** 1.92.0  

> Auto-generated from Criterion benchmark output

## 1. Parsing (String → AST)

| Expression | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **3.39 µs** | 4.80 µs | **SymbAnaFis** (1.42x) |
| Damped Oscillator | **3.68 µs** | 5.35 µs | **SymbAnaFis** (1.45x) |
| Gaussian 2D | **6.80 µs** | 13.68 µs | **SymbAnaFis** (2.01x) |
| Lennard-Jones | **3.96 µs** | 7.62 µs | **SymbAnaFis** (1.92x) |
| Logistic Sigmoid | **2.94 µs** | 4.59 µs | **SymbAnaFis** (1.56x) |
| Lorentz Factor | **2.61 µs** | 4.74 µs | **SymbAnaFis** (1.82x) |
| Maxwell-Boltzmann | **8.17 µs** | 13.02 µs | **SymbAnaFis** (1.59x) |
| Normal PDF | **5.22 µs** | 9.86 µs | **SymbAnaFis** (1.89x) |
| Planck Blackbody | **5.18 µs** | 8.70 µs | **SymbAnaFis** (1.68x) |

---

## 2. Differentiation

| Expression | SymbAnaFis (Light) | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **2.28 µs** | 3.63 µs | **SymbAnaFis (Light)** (1.59x) |
| Damped Oscillator | **1.57 µs** | 3.52 µs | **SymbAnaFis (Light)** (2.25x) |
| Gaussian 2D | **1.46 µs** | 4.66 µs | **SymbAnaFis (Light)** (3.19x) |
| Lennard-Jones | **1.81 µs** | 4.03 µs | **SymbAnaFis (Light)** (2.22x) |
| Logistic Sigmoid | **860.55 ns** | 2.41 µs | **SymbAnaFis (Light)** (2.80x) |
| Lorentz Factor | **1.64 µs** | 3.77 µs | **SymbAnaFis (Light)** (2.30x) |
| Maxwell-Boltzmann | **2.50 µs** | 6.92 µs | **SymbAnaFis (Light)** (2.77x) |
| Normal PDF | **1.74 µs** | 3.54 µs | **SymbAnaFis (Light)** (2.04x) |
| Planck Blackbody | **2.08 µs** | 6.68 µs | **SymbAnaFis (Light)** (3.21x) |

---

## 3. Differentiation + Simplification

| Expression | SymbAnaFis (Full) | Speedup |
| :--- | :---: | :---: |
| Bessel Wave | 134.27 µs | - |
| Damped Oscillator | 155.71 µs | - |
| Gaussian 2D | 135.79 µs | - |
| Lennard-Jones | 29.77 µs | - |
| Logistic Sigmoid | 120.35 µs | - |
| Lorentz Factor | 264.53 µs | - |
| Maxwell-Boltzmann | 338.22 µs | - |
| Normal PDF | 146.74 µs | - |
| Planck Blackbody | 340.86 µs | - |

---

## 4. Simplification Only

| Expression | SymbAnaFis | Speedup |
| :--- | :---: | :---: |
| Bessel Wave | 130.25 µs | - |
| Damped Oscillator | 152.72 µs | - |
| Gaussian 2D | 133.91 µs | - |
| Lennard-Jones | 27.03 µs | - |
| Logistic Sigmoid | 117.86 µs | - |
| Lorentz Factor | 260.98 µs | - |
| Maxwell-Boltzmann | 334.30 µs | - |
| Normal PDF | 143.93 µs | - |
| Planck Blackbody | 335.99 µs | - |

---

## 5. Compilation

| Expression | SA (Simplified) | Symbolica | SA (Raw) | Speedup |
| :--- | :---: | :---: | :---: | :---: |
| Bessel Wave | **2.40 µs** | - | 2.60 µs | - |
| Damped Oscillator | **2.24 µs** | 15.14 µs | 2.50 µs | **SA (Simplified)** (6.76x) |
| Gaussian 2D | **2.71 µs** | 34.72 µs | 2.96 µs | **SA (Simplified)** (12.80x) |
| Lennard-Jones | **1.74 µs** | 27.53 µs | 1.85 µs | **SA (Simplified)** (15.86x) |
| Logistic Sigmoid | 1.70 µs | 9.91 µs | **1.64 µs** | **SA (Simplified)** (5.83x) |
| Lorentz Factor | **1.51 µs** | 9.62 µs | 2.03 µs | **SA (Simplified)** (6.36x) |
| Maxwell-Boltzmann | **3.14 µs** | 17.84 µs | 3.73 µs | **SA (Simplified)** (5.68x) |
| Normal PDF | **2.19 µs** | 18.30 µs | 2.46 µs | **SA (Simplified)** (8.35x) |
| Planck Blackbody | 3.53 µs | 10.83 µs | **3.52 µs** | **SA (Simplified)** (3.07x) |

---

## 6. Evaluation (1000 points)

| Expression | SA (Simplified) | Symbolica | SA (Raw) | Speedup |
| :--- | :---: | :---: | :---: | :---: |
| Bessel Wave | **251.04 µs** | - | 312.25 µs | - |
| Damped Oscillator | 96.32 µs | **60.15 µs** | 109.73 µs | **Symbolica** (1.60x) |
| Gaussian 2D | 111.11 µs | **64.75 µs** | 119.61 µs | **Symbolica** (1.72x) |
| Lennard-Jones | 75.09 µs | **63.15 µs** | 109.91 µs | **Symbolica** (1.19x) |
| Logistic Sigmoid | 80.43 µs | **51.09 µs** | 71.56 µs | **Symbolica** (1.57x) |
| Lorentz Factor | 74.58 µs | **54.59 µs** | 88.10 µs | **Symbolica** (1.37x) |
| Maxwell-Boltzmann | 150.76 µs | **81.87 µs** | 171.98 µs | **Symbolica** (1.84x) |
| Normal PDF | 103.02 µs | **63.60 µs** | 102.09 µs | **Symbolica** (1.62x) |
| Planck Blackbody | 138.73 µs | **63.75 µs** | 139.48 µs | **Symbolica** (2.18x) |

---

## 7. Full Pipeline

| Expression | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | 399.97 µs | - | - |
| Damped Oscillator | 281.32 µs | **182.21 µs** | **Symbolica** (1.54x) |
| Gaussian 2D | 276.30 µs | **176.04 µs** | **Symbolica** (1.57x) |
| Lennard-Jones | **127.53 µs** | 140.89 µs | **SymbAnaFis** (1.10x) |
| Logistic Sigmoid | 220.06 µs | **97.04 µs** | **Symbolica** (2.27x) |
| Lorentz Factor | 372.63 µs | **120.50 µs** | **Symbolica** (3.09x) |
| Maxwell-Boltzmann | 544.40 µs | **266.67 µs** | **Symbolica** (2.04x) |
| Normal PDF | 288.15 µs | **133.53 µs** | **Symbolica** (2.16x) |
| Planck Blackbody | 562.49 µs | **231.57 µs** | **Symbolica** (2.43x) |

---

## Parallel: Evaluation Methods (1k pts)

| Expression | Compiled Loop | Tree Walk | Speedup |
| :--- | :---: | :---: | :---: |
| Bessel Wave | **274.18 µs** | 1.26 ms | **Compiled Loop** (4.59x) |
| Damped Oscillator | **83.15 µs** | 1.04 ms | **Compiled Loop** (12.54x) |
| Gaussian 2D | **102.38 µs** | 1.86 ms | **Compiled Loop** (18.19x) |
| Lennard-Jones | **65.04 µs** | 669.14 µs | **Compiled Loop** (10.29x) |
| Logistic Sigmoid | **73.33 µs** | 1.13 ms | **Compiled Loop** (15.45x) |
| Lorentz Factor | **69.20 µs** | 846.20 µs | **Compiled Loop** (12.23x) |
| Maxwell-Boltzmann | **136.00 µs** | 1.33 ms | **Compiled Loop** (9.80x) |
| Normal PDF | **97.04 µs** | 1.07 ms | **Compiled Loop** (11.05x) |
| Planck Blackbody | **139.51 µs** | 2.04 ms | **Compiled Loop** (14.60x) |

---

## Parallel: Scaling (Points)

| Points | Eval Batch (SIMD) | Loop | Speedup |
| :--- | :---: | :---: | :---: |
| 100 | **2.34 µs** | 6.46 µs | **Eval Batch (SIMD)** (2.76x) |
| 1000 | **23.17 µs** | 64.46 µs | **Eval Batch (SIMD)** (2.78x) |
| 10000 | **232.41 µs** | 645.77 µs | **Eval Batch (SIMD)** (2.78x) |
| 100000 | **367.33 µs** | 6.43 ms | **Eval Batch (SIMD)** (17.51x) |

---

## Large Expressions (100 terms)

| Operation | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Parse | **148.26 µs** | 236.55 µs | **SA** (1.60x) |
| Diff (no simplify) | **81.45 µs** | 250.52 µs | **SA** (3.08x) |
| Diff+Simplify | 7.43 ms | — | — |
| Compile (simplified) | **51.67 µs** | 2.09 ms | **SA** (40.49x) |
| Eval 1000pts (simplified) | **3.75 ms** | 3.89 ms | **SA** (1.04x) |

---

## Large Expressions (300 terms)

| Operation | SymbAnaFis | Symbolica | Speedup |
| :--- | :---: | :---: | :---: |
| Parse | **471.15 µs** | 724.77 µs | **SA** (1.54x) |
| Diff (no simplify) | **259.53 µs** | 785.56 µs | **SA** (3.03x) |
| Diff+Simplify | 21.68 ms | — | — |
| Compile (simplified) | **181.42 µs** | 15.21 ms | **SA** (83.83x) |
| Eval 1000pts (simplified) | 10.96 ms | **10.63 ms** | **SY** (1.03x) |

---

