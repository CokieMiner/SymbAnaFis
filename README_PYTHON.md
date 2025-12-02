# SymbAnaFis - Fast Symbolic Differentiation for Python

[![Crates.io](https://img.shields.io/crates/v/symb_anafis.svg)](https://crates.io/crates/symb_anafis)
[![PyPI](https://img.shields.io/pypi/v/symb-anafis.svg)](https://pypi.org/project/symb-anafis/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A **high-performance symbolic mathematics library** written in Rust with Python bindings. SymbAnaFis provides fast symbolic differentiation and simplification - **3-15x faster than SymPy** for real-world use cases.

## Performance

Benchmark comparison (1000 iterations):

| Operation | SymbAnaFis | SymPy | Speedup |
|-----------|------------|-------|---------|
| `d/dx[sin(x)²]` | 600 µs | 2,853 µs | **4.8x faster** |
| `d/dx[(x²+1)/(x-1)]` | 1,774 µs | 6,652 µs | **3.7x faster** |
| `sin(x)^2 + cos(x)^2` → `1` | 245 µs | 3,959 µs | **16x faster** |
| Complex PDF derivative | 9,850 µs | 11,942 µs | **1.2x faster** |

**Why the difference?** SymbAnaFis uses a rule-based simplification engine with expression-kind filtering for O(1) rule lookup, while SymPy uses a more general but slower approach.

## Installation

```bash
pip install symb-anafis
```

## Quick Start

```python
import symb_anafis

# Differentiation
result = symb_anafis.diff("x^3 + 2*x^2 + x", "x")
print(result)  # Output: 3 * x^2 + 4 * x + 1

# Simplification
result = symb_anafis.simplify("sin(x)^2 + cos(x)^2")
print(result)  # Output: 1

# With constants
result = symb_anafis.diff("a * x^2", "x", fixed_vars=["a"])
print(result)  # Output: 2 * a * x
```

## Features

✅ **Fast Differentiation**
- Supports all standard calculus rules (product, chain, quotient, power)
- Handles trigonometric, exponential, and logarithmic functions
- Support for custom functions and implicit differentiation

✅ **Powerful Simplification**
- Automatic constant folding
- Trigonometric identities (Pythagorean, double angle, etc.)
- Algebraic simplification (factoring, expanding)
- Fraction cancellation and rationalization

✅ **Flexible API**
- Fixed variables (constants that aren't differentiated)
- Custom function definitions
- Domain-safety mode to avoid incorrect simplifications

## Examples

### Physics: RC Circuit
```python
# Voltage in RC circuit: V(t) = V₀ * exp(-t/(R*C))
voltage = "V0 * exp(-t / (R * C))"
current = symb_anafis.diff(
    voltage, 
    "t", 
    fixed_vars=["V0", "R", "C"]
)
print(current)  # Current: dV/dt
```

### Statistics: Normal Distribution
```python
# Normal PDF: f(x) = exp(-(x-μ)²/(2σ²)) / √(2πσ²)
pdf = "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)"
derivative = symb_anafis.diff(
    pdf,
    "x",
    fixed_vars=["mu", "sigma"]
)
print(derivative)  # Derivative with respect to x
```

### Calculus: Chain Rule
```python
# Chain rule: d/dx[sin(cos(tan(x)))]
result = symb_anafis.diff("sin(cos(tan(x)))", "x")
print(result)
```

## API Reference

### `diff(formula, var, fixed_vars=None, custom_functions=None) -> str`

Differentiate a mathematical expression.

**Parameters:**
- `formula` (str): Mathematical expression (e.g., `"x^2 + sin(x)"`)
- `var` (str): Variable to differentiate with respect to
- `fixed_vars` (list, optional): Variables that are constants
- `custom_functions` (list, optional): User-defined function names

**Returns:** Simplified derivative as a string

**Raises:** `ValueError` if parsing/differentiation fails

### `simplify(formula, fixed_vars=None, custom_functions=None) -> str`

Simplify a mathematical expression.

**Parameters:**
- `formula` (str): Expression to simplify
- `fixed_vars` (list, optional): Variables that are constants
- `custom_functions` (list, optional): User-defined function names

**Returns:** Simplified expression as a string

### `parse(formula, fixed_vars=None, custom_functions=None) -> str`

Parse and normalize an expression.

**Parameters:**
- `formula` (str): Expression to parse
- `fixed_vars` (list, optional): Variables that are constants
- `custom_functions` (list, optional): User-defined function names

**Returns:** Normalized expression string

## Supported Functions

### Trigonometric
`sin`, `cos`, `tan`, `sec`, `csc`, `cot`

### Inverse Trigonometric
`asin`, `acos`, `atan`, `asec`, `acsc`, `acot`

### Hyperbolic
`sinh`, `cosh`, `tanh`, `sech`, `csch`, `coth`

### Inverse Hyperbolic
`asinh`, `acosh`, `atanh`, `asech`, `acsch`, `acoth`

### Exponential & Logarithmic
`exp`, `ln`, `log`, `log2`, `log10`

### Special Functions
`sqrt`, `cbrt`, `abs`, `erf`, `erfc`, `gamma`, `lambertw`, `sinc`

## Expression Syntax

- **Variables:** `x`, `y`, `sigma`, etc.
- **Numbers:** `1`, `3.14`, `1e-5`, `2.5e3`
- **Operations:** `+`, `-`, `*`, `/`, `^` (power)
- **Functions:** `sin()`, `cos()`, `exp()`, `ln()`, `sqrt()`
- **Constants:** `pi`, `e` (automatically recognized)
- **Implicit multiplication:** `2x` = `2*x`, `(x+1)(x-1)` = `(x+1)*(x-1)`

## Comparison with SymPy

| Feature | SymbAnaFis | SymPy |
|---------|-----------|-------|
| Speed (diff+simplify) | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| Differentiation | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Simplification | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| Python Integration | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Symbolic solving | ⭐ | ⭐⭐⭐⭐⭐ |
| Maturity | Newer | Established |

**When to use SymbAnaFis:**
- You need fast differentiation + simplification
- Performance is critical
- You're working with real-world physics/engineering expressions

**When to use SymPy:**
- You need symbolic equation solving
- You need broader symbolic capabilities
- You prefer pure Python implementation

## Building from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone repository
git clone https://github.com/CokieMiner/symb_anafis.git
cd symb_anafis

# Build Python bindings
pip install maturin
maturin develop --release

# Run tests
cargo test --release
```

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! Please submit pull requests to the [GitHub repository](https://github.com/CokieMiner/symb_anafis).

## Citation

If you use SymbAnaFis in academic work, please cite:

```bibtex
@software{symb_anafis,
  author = {CokieMiner},
  title = {SymbAnaFis: Fast Symbolic Differentiation Library},
  url = {https://github.com/CokieMiner/symb_anafis},
  year = {2025}
}
```

## Resources

- **GitHub:** https://github.com/CokieMiner/symb_anafis
- **Crates.io:** https://crates.io/crates/symb_anafis
- **PyPI:** https://pypi.org/project/symb-anafis/
- **Documentation:** https://docs.rs/symb_anafis

---

**Built with ❤️ in Rust for Python** 🚀
