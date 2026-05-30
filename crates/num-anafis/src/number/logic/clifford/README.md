# Clifford (Geometric) Algebra Module

This module provides a highly optimized, dynamically typed, and mathematically rigorous implementation of Clifford (Geometric) Algebras, specifically focusing on **Conformal Geometric Algebra (CGA)** — $\text{Cl}(4,1)$.

By seamlessly integrating numeric operations with a robust spectral decomposition engine, this module grants you the ability to compute $50+$ transcendental functions (`sin`, `exp`, `sqrt`, `zeta`, etc.) on **any arbitrary multivector**.

## Core Architecture

The system is designed with a strict `#![no_std]` compliant core (requiring only `alloc`) and heavily optimizes for both low-dimensional inline algebras and theoretically unbounded heap-allocated algebras.

### 1. Representation & Memory Allocation (`types.rs`)
At the heart of the algebraic representation is the `CliffordNumber` struct:
- **`Buffer` Enum**: Automatically switches between an inline stack array `[Scalar; 32]` for algebras with $\le 5$ generators (perfect for 5D CGA) and a dynamic heap-allocated `Vec<Scalar>` for higher-dimensional spaces.
- **`GeneratorSet`**: Defines the specific geometry of the algebra (the metrics $P, Q, R$ mapped to specific basis vectors).
  - Contains a pre-computed $O(1)$ **Cayley Sign Cache** stored inside an `Arc<[i8]>`. This completely eliminates bit-twiddling and permutation loop overhead during repeated geometric products.

### 2. Core Algebraic Arithmetic (`core.rs` & `arithmetic.rs`)
The fundamental Geometric Product is evaluated in `core.rs`. Rather than relying on symbolic manipulation or slow dynamic bitwise parity checks at runtime, the $O(1)$ Cayley Sign Cache dictates exactly how base elements combine.
`arithmetic.rs` wraps these core routines into the standard Rust operator traits (`Add`, `Sub`, `Mul`, `Div`, `Neg`).

### 3. Spectral Decomposition Pipeline (`spectral.rs`, `matrix.rs`, `large_mat.rs`)
To compute advanced mathematical functions on Multivectors, we use spectral decomposition. We embed the Clifford algebra into complex matrices, compute the function on the matrix eigenvalues, and project it back.

- **`spectral.rs`**: Houses the main routing function `apply_via_spectral`. It acts as the backbone for the `Number` trait implementations on `CliffordNumber`, intercepting calls to `sin()`, `exp()`, etc.
  - Safely implements **IEEE-754 failure paths**: If a multivector has degenerate or nilpotent generators that cannot be safely matrix-embedded, it gracefully emits `NaN` (Not a Number) multivectors to poison the calculation stream rather than failing silently.
- **`matrix.rs`**: Optimized specifically for $2 \times 2$ Complex Matrix eigendecompositions, handling algebras with $\le 3$ generators lightning-fast.
- **`large_mat.rs`**: Handles complex matrices for larger generator sets (4+ dimensions). It leverages:
  - **Upper Hessenberg Reduction** via Householder reflections.
  - $O(N^2)$ **Givens Rotations** for optimal QR steps to compute the Schur decomposition.

### 4. Zero-Allocation Compile-Time Representation (`fast.rs`)
For domains requiring maximum throughput and pure stack representations without the flexibility of `CliffordNumber`'s dynamic generation, `FastClifford<const P, const Q, const R>` exists.
- Fully const-generic and heap-free.
- Implements the complete `Number` trait through an internal macro that strategically delegates complex spectral computations up to the dynamic `CliffordNumber` pipeline.

### 5. Constants and Convenience Constructors (`constructors.rs`)
Provides immediate access to the standard elements of Euclidean space and Conformal Geometric Algebra:
- **Euclidean Vectors**: `e1`, `e2`, `e3`
- **Extra Dimensions**: `e_plus` ($e_+$), `e_minus` ($e_-$)
- **Conformal Identifiers**: `orig` (Origin $n_o$) and `inf` (Infinity $n_\infty$).
- **Sub-algebras**: Complex numbers (`ci`), Quaternions (`qi`, `qj`, `qk`), and Dual numbers (`eps` / $\epsilon$).

### 6. Formatting (`display.rs`)
Implements `Display` to print multivectors using standard mathematical nomenclature (e.g., `2.5 + 3.0*e12 - 1.0*e_plus`).

---

## Design Philosophy Highlights

1. **Matrix Mapping for Transcendental Capabilities**: Unlike most Geometric Algebra libraries that hardcode geometric operations and give up on functions like `sin()` or logarithms, `num-anafis` can map any multivector space into a complex matrix, compute its spectral decomposition, apply the mathematical function to the eigenvalues, and seamlessly map the resulting complex matrix back into the Clifford space.
2. **IEEE 754 Safety First**: Mathematical undefinedness ($0/0$) or nilpotent geometric explosions are strictly mapped to `NaN`.
3. **No `std` Required**: Purely algorithmic logic capable of running on embedded hardware using only `core` and `alloc`.
