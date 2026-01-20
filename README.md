# SymbAnaFis

[![Crates.io](https://img.shields.io/crates/v/symb_anafis.svg)](https://crates.io/crates/symb_anafis)
[![PyPI](https://img.shields.io/pypi/v/symb-anafis.svg)](https://pypi.org/project/symb-anafis/)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

**High-performance symbolic mathematics** library written in Rust with Python bindings.

SymbAnaFis provides a robust engine for symbolic differentiation, simplification, and evaluation, designed for performance-critical applications in physics, engineering, and machine learning.

## Key Capabilities

*   **⚡ High-Performance Architecture**: Built on Rust for speed and memory safety, with interned strings and optimized memory layout.
*   **∂ Symbolic Differentiation**: Supports product, chain, and quotient rules for a vast array of mathematical functions.
*   **✨ Algebraic Simplification**: Intelligent simplification engine covering trigonometric identities, constant folding, and algebraic expansion.
*   **📊 Uncertainty Propagation**: Comprehensive support for calculating uncertainty propagation with full covariance matrix integration.
*   **∇ Vector Calculus**: Native symbolic computation of Gradients, Hessians, and Jacobian matrices.
*   **🚀 Parallel Processing**: Optional parallel evaluation engine using Rayon for massive batch operations.
*   **📦 Python Bindings**: Seamless Python integration via `maturin`, offering the speed of Rust with the ease of Python.

## Visual Benchmarks

SymbAnaFis performance is validated against SymPy and SymEngine using complex physical simulations.
Each benchmark features a **Quad-View Dashboard** comparing real-time simulations side-by-side.

### Aizawa Attractor
[Watch Video](videos/aizawa_quad.mp4)
Simulates 500,000 particles flowing through the Aizawa field, comparing Euler integration throughput.

### Clifford Attractor
[Watch Video](videos/clifford_quad.mp4)
Visualizes 1,000,000 particles evolving in the Clifford map, testing discrete map evaluation speed.

### Double Pendulum Matrix
[Watch Video](videos/double_pendulum_quad.mp4)
Simulates 50,000 chaotic double pendulums, verifying symbolic differentiation correctness and RK4 integration stability.

### Gradient Descent Avalanche
[Watch Video](videos/gradient_descent_quad.mp4)
Massive particle descent on a complex terrain function, showcasing symbolic differentiation of compiled functions.

## Installation

```bash
# Python
pip install symb-anafis

# Rust
cargo add symb_anafis
```

## Quick Start

### Python

```python
import symb_anafis

# Differentiate complex expressions
result = symb_anafis.diff("x^3 + sin(x)", "x")
# → "3*x^2 + cos(x)"

# Algebraic Simplification
result = symb_anafis.simplify("sin(x)^2 + cos(x)^2")
# → "1"

# Handle constants automatically
result = symb_anafis.diff("a*x^2", "x", fixed_vars=["a"])
# → "2*a*x"
```

### Rust

```rust
use symb_anafis::{diff, simplify, symb};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // String API for ease of use
    let result = diff("sin(x) * x", "x", None, None)?;
    println!("{result}");  // cos(x)*x + sin(x)

    // Type-safe API (Symbol is Copy - no clone needed!)
    let x = symb("x");
    let expr = x.pow(2.0) + x.sin();  // x² + sin(x)
    
    // Export to LaTeX
    println!("{}", expr.to_latex());  // x^{2} + \sin(x)
    
    Ok(())
}
```

## Advanced Features

### 🔍 Fine-Grained Control
Use the Builder pattern to configure safety limits and behavior.

```rust
use symb_anafis::{Diff, Simplify};

Diff::new()
    .domain_safe(true)     // Prevent unsafe simplifications (e.g., x/x != 1 if x=0)
    .max_depth(200)        // Prevent stack overflows on massive expressions
    .diff_str("sqrt(x^2)", "x")?; // Result: abs(x)/x
```

### 📉 Uncertainty Propagation
Calculate error propagation symbolically, supporting correlated variables.

```rust
use symb_anafis::uncertainty_propagation;

// Calculate uncertainty formula for f = x + y with full covariance support
let sigma = uncertainty_propagation(&expr, &["x", "y"], None)?;
// → sqrt(sigma_x^2 + 2*sigma_x*sigma_y*rho_xy + sigma_y^2)
```

### ⚡ Parallel Evaluation
Evaluate expressions over large datasets in parallel (requires `parallel` feature).

```rust
// Evaluate symbolic expressions across thousands of data points efficiently
let results = evaluate_parallel(&inputs, &data);
```

### 🛠️ Custom Functions
Register custom functions with their own derivative rules.

```rust
use symb_anafis::{Diff, UserFunction};

Diff::new()
    .user_fn("f", UserFunction::new(1..=1).partial(0, |args| {
        // Define ∂f/∂u = 2u for f(u)
        2.0 * args[0].clone()
    }))
    .diff_str("f(x^2)", "x")?; // → 4*x^3
```

## Supported Functions

SymbAnaFis supports over 50 built-in mathematical functions:

| Category                   | Functions                                                                     |
| -------------------------- | ----------------------------------------------------------------------------- |
| **Trig**                   | `sin`, `cos`, `tan`, `cot`, `sec`, `csc`                                      |
| **Inverse Trig**           | `asin`, `acos`, `atan`, `atan2`, `acot`, `asec`, `acsc`                       |
| **Hyperbolic**             | `sinh`, `cosh`, `tanh`, `coth`, `sech`, `csch`                                |
| **Inverse Hyperbolic**     | `asinh`, `acosh`, `atanh`, `acoth`, `asech`, `acsch`                          |
| **Exp/Log**                | `exp`, `ln`, `log(b, x)`, `log10`, `log2`, `exp_polar`(acts as exp for now)   |
| **Roots**                  | `sqrt`, `cbrt`                                                                |
| **Error Functions**        | `erf`, `erfc`                                                                 |
| **Gamma Family**           | `gamma`, `digamma`, `trigamma`, `tetragamma`, `polygamma(n, x)`, `beta(a, b)` |
| **Zeta**                   | `zeta`, `zeta_deriv(n, s)`                                                    |
| **Bessel**                 | `besselj(n, x)`, `bessely(n, x)`, `besseli(n, x)`, `besselk(n, x)`            |
| **Elliptic Integrals**     | `elliptic_k`, `elliptic_e`                                                    |
| **Orthogonal Polynomials** | `hermite(n, x)`, `assoc_legendre(l, m, x)`                                    |
| **Spherical Harmonics**    | `spherical_harmonic(l, m, θ, φ)`, `ynm(l, m, θ, φ)`                           |
| **Other**                  | `abs`, `signum`, `sinc`, `lambertw`, `floor`, `ceil`, `round`                 |

## Documentation

- **[API Reference](docs/API_REFERENCE.md)** - Detailed guide to all functions and modules.
- **[docs.rs](https://docs.rs/symb_anafis)** - Full Rust crate documentation.

## License

Apache License 2.0 - see [LICENSE](LICENSE)

## Citation

If you use SymbAnaFis in academic work, please cite:

```bibtex
@software{symbanafis,
  author       = {Martins, Pedro},
  title        = {SymbAnaFis: High-Performance Symbolic Mathematics Library},
  year         = {2026},
  url          = {https://github.com/CokieMiner/SymbAnaFis},
  version      = {0.7.0}
}
```

---

**Built with ❤️ in Rust** 🚀
