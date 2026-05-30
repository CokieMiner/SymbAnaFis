# `num-anafis`

`num-anafis` is the core numerical and algebraic computation engine of the **SymbAnaFis** library. It provides a highly generic, multi-backend scalar math implementation coupled with a robust dense **Conformal Geometric Algebra (CGA)** engine.

This document outlines the organization and architecture of the crate.

---

## Architecture Overview

The crate is structured around a three-tier mathematical representation:
1. **Backends (`float_ops`, `int_math`, `rational_math`)**: The lowest level. Abstracts operations over native primitives (`f64`, `f32`) or arbitrary-precision libraries (`rug`).
2. **Scalars (`scalar.rs`)**: A dynamic exact/approximate hybrid number type (`Int` → `Rational` → `Float`) that automatically promotes to the best representation based on the requested mathematical operation.
3. **Clifford Algebra (`clifford/`)**: The highest level. Extends scalar operations to dense multi-vectors using the spectral mapping theorem via custom complex matrix eigendecomposition.

---

## Directory Structure (`src/`)

### 1. The Core Data Types (`src/number/logic/`)
This is where the mathematical rules are defined independently of the high-level API or Python bindings.

- **`traits.rs`**: The single source of truth for all mathematical capabilities. It defines the `Number` trait, representing the strict contract of supported unary, binary, trigonometric, exponential, and special functions (like Gamma, Zeta, Bessel). Both `Scalar` and `CliffordNumber` implement this.
- **`scalar.rs`**: Implements the `Scalar` type, which wraps `Int`, `Rational`, or `Float` variants. It attempts to perform math exactly using integers and rationals, falling back to floats when precision is inevitably lost (e.g., `sin` or `sqrt(2)`).

### 2. Backend Abstractions (`src/number/logic/float_ops/`, `int_math/`, `rational_math/`)
These modules isolate the codebase from the underlying numeric implementations. The active backend is selected via Cargo features.

- **`backend64` (Default)**: Maps integers to `i64`, floats to `f64`.
- **`backend32`**: Maps integers to `i32`, floats to `f32` (memory-optimized).
- **`backendrug`**: Maps everything to MPFR/GMP-based arbitrary-precision representations via the `rug` crate.
- **`float_ops/special/`**: Contains pure-Rust approximations for advanced special mathematical functions (e.g., Riemann Zeta, Error functions, Lambert W) that are used when fixed hardware backends are active. If `backendrug` is used, these seamlessly delegate to MPFR's highly optimized native functions.

### 3. Conformal Geometric Algebra (`src/number/logic/clifford/`)
The algebraic engine. Instead of hardcoding separate types for Complex numbers, Quaternions, or Dual numbers, this module embeds everything into a single generalized **Cl(4,1) Conformal Geometric Algebra**.

- **`types.rs` & `core.rs`**: Defines `GeneratorSet` (to track basis dimensions) and `CliffordNumber` (a dense multi-vector over scalars).
- **`constructors.rs`**: High-level builder methods defining constants like Euclidean dimensions (`e1`, `e2`, `e3`), infinity (`inf`), origin (`orig`), the complex unit (`ci`), and dual unit (`eps`).
- **`arithmetic.rs`**: Implements the geometric product and basic algebraic rules across generators, handling signs and anti-commutativity.
- **`matrix.rs` & `large_mat.rs`**: Custom linear algebra engines. Because generators can be embedded into complex matrices (using Pauli/Dirac matrix representations), these modules allow converting any multivector into a complex matrix.
- **`spectral.rs`**: The bridge that extends the `Number` trait to the `CliffordNumber`. By decomposing the matrix representation into its Schur/Eigen form (via our custom QR algorithm in `large_mat`), we apply scalar functions (like `exp` or `sin`) to the eigenvalues, and map the matrix back to a multi-vector. This is the **Spectral Mapping Theorem**.

### 4. Bindings and Integrations (`src/python_binding.rs`, `src/lib.rs`)
- Uses `pyo3` to expose the `Scalar` and `CliffordNumber` structures directly to Python environments if the `python` feature is enabled.
- Ensures all errors (e.g., incompatible generator sets or unsupported algebraic conversions) are elegantly translated to Python exceptions.

---

## Key Features and Capabilities

1. **Write Once, Compute Anywhere**:
   The `FloatType`, `IntType`, and `RationalType` type aliases abstract away the precision. Switching from `f64` hardware execution to 1000-bit arbitrary precision requires exactly zero changes to the logic. `Scalar::epsilon()` is dynamically computed based on the selected backend's active mantissa width.
2. **Spectral Generalization**:
   Instead of writing custom code to compute the exponent of a quaternion, dual number, or 5D conformal point, the `clifford` module treats them identically. It maps them to a complex matrix, decomposes the eigenvalues, applies the real/complex exponentiation, and returns the mathematically perfect multi-vector result.
3. **Dependency-Free Linear Algebra**:
   The eigendecomposition explicitly uses a built-in, unshifted & Wilkinson-shifted QR algorithm (`large_mat.rs`). This guarantees that matrix mathematics strictly respects the arbitrary-precision `Scalar` type, rather than defaulting to `f64` boundaries natively found in external crates like `nalgebra`.
