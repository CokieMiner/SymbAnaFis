# Benchmark Results

**SymbAnaFis Version:** 0.4.0  
**Date:** 2025-12-22  
**System:** Linux (Plotters Backend)

## Test Expressions

| Name | Expression | Nodes | Domain |
|------|------------|-------|--------|
| Normal PDF | `exp(-(x-μ)²/(2σ²))/√(2πσ²)` | ~30 | Statistics |
| Gaussian 2D | `exp(-((x-x₀)²+(y-y₀)²)/(2s²))/(2πs²)` | ~40 | ML/Physics |
| Maxwell-Boltzmann | `4π(m/(2πkT))^(3/2) v² exp(-mv²/(2kT))` | ~50 | Physics |
| Lorentz Factor | `1/√(1-v²/c²)` | ~15 | Relativity |
| Lennard-Jones | `4ε((σ/r)¹² - (σ/r)⁶)` | ~25 | Chemistry |
| Logistic Sigmoid | `1/(1+exp(-k(x-x₀)))` | ~15 | ML |
| Damped Oscillator | `A·exp(-γt)·cos(ωt+φ)` | ~25 | Physics |
| Planck Blackbody | `2hν³/c² · 1/(exp(hν/(kT))-1)` | ~35 | Physics |

---

## 1. Parsing (String → AST)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------|
| Normal PDF | 2.62 µs | 4.49 µs | **1.71x** |
| Gaussian 2D | 3.62 µs | 6.45 µs | **1.78x** |
| Maxwell-Boltzmann | 4.15 µs | 6.09 µs | **1.46x** |
| Lorentz Factor | 1.36 µs | 2.37 µs | **1.74x** |
| Lennard-Jones | 2.00 µs | 3.63 µs | **1.81x** |
| Logistic Sigmoid | 1.68 µs | 2.17 µs | **1.29x** |
| Damped Oscillator | 2.05 µs | 2.53 µs | **1.23x** |
| Planck Blackbody | 2.75 µs | 4.04 µs | **1.46x** |

> **Result:** SymbAnaFis parses 1.2x - 1.8x faster than Symbolica.

---

## 2. Differentiation (Raw - No Simplification)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------|
| Normal PDF | 1.23 µs | 1.64 µs | **1.33x** |
| Gaussian 2D | 1.17 µs | 2.20 µs | **1.88x** |
| Maxwell-Boltzmann | 1.61 µs | 3.24 µs | **2.01x** |
| Lorentz Factor | 1.09 µs | 1.84 µs | **1.69x** |
| Lennard-Jones | 1.14 µs | 1.89 µs | **1.65x** |
| Logistic Sigmoid | 0.60 µs | 1.37 µs | **2.28x** |
| Damped Oscillator | 1.02 µs | 1.65 µs | **1.61x** |
| Planck Blackbody | 1.56 µs | 3.07 µs | **1.96x** |

> **Result:** SymbAnaFis raw differentiation is 1.3x - 2.3x faster.

---

## 3. Differentiation + Simplification

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SY vs SA) |
|------------|-----------------|----------------|--------------------|
| Normal PDF | 174 µs | 7.80 µs | **22.3x** |
| Gaussian 2D | 124 µs | 10.02 µs | **12.3x** |
| Maxwell-Boltzmann | 213 µs | 11.97 µs | **17.8x** |
| Lorentz Factor | 191 µs | 5.66 µs | **33.7x** |
| Lennard-Jones | 118 µs | 6.54 µs | **18.0x** |
| Logistic Sigmoid | 61 µs | 4.64 µs | **13.1x** |
| Damped Oscillator | 87 µs | 5.62 µs | **15.4x** |
| Planck Blackbody | 268 µs | 9.49 µs | **28.2x** |

> **Result:** Symbolica performs lighter simplification (e.g., collecting terms `3x+2x`), while **SymbAnaFis performs complete restructuring** including symbolic function equivalence (e.g., trig identities), explaining the difference in execution time.

---

## 4. Simplification Only

| Expression | SymbAnaFis |
|------------|------------|
| Normal PDF | 164 µs |
| Gaussian 2D | 116 µs |
| Maxwell-Boltzmann | 202 µs |
| Lorentz Factor | 187 µs |
| Lennard-Jones | 113 µs |
| Logistic Sigmoid | 56 µs |
| Damped Oscillator | 82 µs |
| Planck Blackbody | 258 µs |

---

## 5. Compilation (AST → Bytecode/Evaluator)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SA vs SY) |
|------------|-----------------|----------------|--------------------|
| Normal PDF | 0.56 µs | 8.82 µs | **15x** |
| Gaussian 2D | 0.77 µs | 16.33 µs | **21x** |
| Maxwell-Boltzmann | 0.86 µs | 8.35 µs | **9x** |
| Lorentz Factor | 0.48 µs | 4.82 µs | **10x** |
| Lennard-Jones | 0.48 µs | 12.84 µs | **26x** |
| Logistic Sigmoid | 0.48 µs | 4.93 µs | **10x** |
| Damped Oscillator | 0.78 µs | 7.57 µs | **9x** |
| Planck Blackbody | 1.35 µs | 4.98 µs | **3x** |

> **Result:** SymbAnaFis compilation (AST to Bytecode) is significantly faster than Symbolica's evaluator creation.

---

## 6. Evaluation (Compiled, 1000 points)

| Expression | SymbAnaFis (Simpl) | Symbolica (SY) | Speedup (SY vs SA) |
|------------|--------------------|----------------|--------------------|
| Normal PDF | 82 µs | 29.2 µs | **2.8x** |
| Gaussian 2D | 78 µs | 35.1 µs | **2.2x** |
| Maxwell-Boltzmann | 89 µs | 41.6 µs | **2.1x** |
| Lorentz Factor | 44 µs | 21.3 µs | **2.0x** |
| Lennard-Jones | 53 µs | 31.2 µs | **1.7x** |
| Logistic Sigmoid | 65 µs | 21.6 µs | **3.0x** |
| Damped Oscillator | 52 µs | 26.8 µs | **1.9x** |
| Planck Blackbody | 143 µs | 31.5 µs | **4.5x** |

> **Result:** Symbolica's evaluator is 1.7x - 4.5x faster at runtime execution.

---

## 7. Full Pipeline Scenarios

**Scenario:** End-to-End Execution (Parse → Diff → Compile → Eval 1000 pts)

| Expression | SymbAnaFis (SA) | Symbolica (SY) | Speedup (SY vs SA) |
|------------|-----------------|----------------|--------------------|
| Normal PDF | 258 µs | 54.26 µs | **4.75x** |
| Gaussian 2D | 209 µs | 72.33 µs | **2.89x** |
| Maxwell-Boltzmann | 313 µs | 116.45 µs | **2.69x** |
| Lorentz Factor | 240 µs | 58.51 µs | **4.10x** |
| Lennard-Jones | 177 µs | 62.24 µs | **2.84x** |
| Logistic Sigmoid | 129 µs | 48.14 µs | **2.68x** |
| Damped Oscillator | 148 µs | 79.92 µs | **1.85x** |
| Planck Blackbody | 428 µs | 96.07 µs | **4.45x** |

> **Result:** Symbolica is **1.8x - 4.7x** faster in the full end-to-end pipeline, largely due to its faster evaluation engine handling the 1000-point loop more efficiently.

---

## Summary

- **Parsing:** SymbAnaFis is **1.5x - 1.8x** faster.
- **Differentiation:** SymbAnaFis is **1.3x - 2.3x** faster (raw).
- **Compilation:** SymbAnaFis is **9x - 26x** faster.
- **Simplification:** Symbolica is significantly faster (**12x - 30x**) due to a lighter strategy (term collection) vs SymbAnaFis's rigorous structural simplification (trig identities, etc.).
- **Evaluation:** Symbolica is **2x - 4.5x** faster at runtime execution.
