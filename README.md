# SymbAnaFis

[![Crates.io](https://img.shields.io/crates/v/symb_anafis.svg)](https://crates.io/crates/symb_anafis)
[![PyPI](https://img.shields.io/pypi/v/symb-anafis.svg)](https://pypi.org/project/symb-anafis/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**High-performance symbolic mathematics** written in Rust with Python bindings.

> 🚀 **18-21x faster parsing** and **4-43x faster simplification** than SymPy

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

# Differentiate
result = symb_anafis.diff("x^3 + sin(x)", "x")
# → "3*x^2 + cos(x)"

# Simplify
result = symb_anafis.simplify("sin(x)^2 + cos(x)^2")
# → "1"

# With constants
result = symb_anafis.diff("a*x^2", "x", fixed_vars=["a"])
# → "2*a*x"
```

### Rust

```rust
use symb_anafis::{diff, simplify, symb};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // String API
    let result = diff("sin(x) * x", "x", None, None)?;
    println!("{result}");  // cos(x)*x + sin(x)

    // Type-safe API (Symbol is Copy - no clone needed!)
    let x = symb("x");
    let expr = x.pow(2.0) + x.sin();  // x² + sin(x)
    println!("{}", expr.to_latex());  // x^{2} + \sin(x)
    
    Ok(())
}
```

## Features

| Feature | Description |
|---------|-------------|
| **Fast Differentiation** | Product, chain, quotient rules for all standard functions |
| **Powerful Simplification** | Trig identities, constant folding, algebraic factoring |
| **Vector Calculus** | Gradient, Hessian, Jacobian computation |
| **Uncertainty Propagation** | Full covariance matrix support |
| **Beautiful Output** | LaTeX (`to_latex()`) and Unicode (`to_unicode()`) export |
| **Parallel Evaluation** | Batch evaluation via Rayon (optional `parallel` feature) |
| **50+ Built-in Functions** | Trig, hyperbolic, Bessel, gamma, polygamma, zeta, error functions |

## Core API

### Differentiation

```rust
// String API
diff("a * x^2", "x", Some(&["a"]), None)?;  // 2*a*x

// Builder API for fine-grained control
Diff::new()
    .domain_safe(true)
    .max_depth(200)
    .diff_str("sqrt(x^2)", "x")?;  // abs(x)/x with domain_safe
```

### Simplification

```rust
simplify("x^2 + 2*x + 1", None, None)?;  // (x+1)^2

// Trigonometric identities
simplify("sin(x)^2 + cos(x)^2", None, None)?;  // 1
```

### Evaluation

```rust
use symb_anafis::evaluate_str;

// Full evaluation
evaluate_str("x*y + 1", &[("x", 3.0), ("y", 2.0)])?;  // "7"

// Partial evaluation (y stays symbolic)
evaluate_str("x*y + 1", &[("x", 3.0)])?;  // "3*y + 1"
```

### Vector Calculus

```rust
use symb_anafis::{gradient_str, hessian_str, jacobian_str};

gradient_str("x^2 + y^2", &["x", "y"])?;  // ["2*x", "2*y"]
hessian_str("x^2*y", &["x", "y"])?;       // [["2*y", "2*x"], ["2*x", "0"]]
```

### Uncertainty Propagation

```rust
use symb_anafis::{symb, uncertainty_propagation};

let x = symb("x");
let y = symb("y");
let expr = x + y;

let sigma = uncertainty_propagation(&expr, &["x", "y"], None)?;
// → sqrt(sigma_x^2 + sigma_y^2)
```

## Examples

### Physics: RC Circuit

```python
# V(t) = V₀ · e^(-t/RC)
current = symb_anafis.diff(
    "V0 * exp(-t / (R * C))",
    "t",
    fixed_vars=["V0", "R", "C"]
)
```

### Statistics: Normal PDF Derivative

```python
pdf = "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)"
derivative = symb_anafis.diff(pdf, "x", fixed_vars=["mu", "sigma"])
```

### Custom Functions

```rust
use symb_anafis::{Diff, Expr};

let diff = Diff::new()
    .custom_derivative("f", |inner, _var, inner_prime| {
        // d/dx[f(u)] = 2u · u'  (chain rule automatic!)
        Expr::number(2.0) * inner.clone() * inner_prime.clone()
    });

diff.diff_str("f(x^2)", "x")?;  // 4*x^3
```

## Built-in Functions

| Category | Functions |
|----------|-----------|
| **Trig** | `sin`, `cos`, `tan`, `cot`, `sec`, `csc` + inverses |
| **Hyperbolic** | `sinh`, `cosh`, `tanh`, `coth`, `sech`, `csch` + inverses |
| **Exp/Log** | `exp`, `ln`, `log`, `log10`, `log2` |
| **Roots** | `sqrt`, `cbrt` |
| **Special** | `gamma`, `digamma`, `trigamma`, `polygamma`, `beta`, `erf`, `erfc` |
| **Bessel** | `besselj`, `bessely`, `besseli`, `besselk` |
| **Other** | `zeta`, `LambertW`, `abs`, `sign`, `sinc` |

## Expression Syntax

```
x + y          # Addition
x - y          # Subtraction  
x * y          # Multiplication (also: 2x, xy)
x / y          # Division
x^2            # Power
sin(x)         # Function call
pi, e          # Built-in constants
```

## Performance

Benchmarked against SymPy 1.14.0 (Python bindings):

| Operation | Speedup |
|-----------|---------|
| Parsing | **17-21x faster** |
| Simplification | **4-43x faster** |

See [`benches/BENCHMARK_COMPARISON.md`](benches/BENCHMARK_COMPARISON.md) for detailed comparisons with SymPy and Symbolica.

## Configuration

Configure safety limits and domain-safe mode using the builder pattern:

```rust
use symb_anafis::{Diff, Simplify};

// Differentiation with limits
Diff::new()
    .max_depth(200)        // AST depth limit
    .max_nodes(50000)      // Node count limit
    .domain_safe(true)     // Domain-safe simplification
    .diff_str("sqrt(x^2)", "x")?;  // → abs(x)/x

// Simplification with limits
Simplify::new()
    .domain_safe(true)
    .simplify_str("sqrt(x^2)")?;  // → abs(x)
```

**Domain-safe mode** prevents unsafe simplifications:
- `sqrt(x^2)` → `abs(x)` (instead of just `x`)

## Documentation

- **[API Reference](docs/API_REFERENCE.md)** - Complete function documentation
- **[Benchmark Comparison](benches/BENCHMARK_COMPARISON.md)** - Performance vs SymPy/Symbolica
- **[docs.rs](https://docs.rs/symb_anafis)** - Rust API documentation

## Building from Source

```bash
# Clone
git clone https://github.com/CokieMiner/SymbAnaFis.git
cd SymbAnaFis

# Python bindings
pip install maturin
maturin develop --release

# Rust
cargo build --release
cargo test --release
```

## License

MIT License - see [LICENSE](LICENSE)

## Citation

```bibtex
@software{symb_anafis,
  author = {Pedro Martins},
  title = {SymbAnaFis: Fast Symbolic Differentiation Library},
  url = {https://github.com/CokieMiner/SymbAnaFis},
  year = {2025}
}
```

---

**Built with ❤️ in Rust** 🚀
