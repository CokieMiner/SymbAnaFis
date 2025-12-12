# SymbAnaFis: Fast Symbolic Differentiation for Python & Rust

[![Crates.io](https://img.shields.io/crates/v/symb_anafis.svg)](https://crates.io/crates/symb_anafis)
[![PyPI](https://img.shields.io/pypi/v/symb-anafis.svg)](https://pypi.org/project/symb-anafis/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A **high-performance symbolic mathematics library** written in Rust with Python bindings. SymbAnaFis provides fast symbolic differentiation and simplification.


## Installation

### Python (Recommended)

```bash
pip install symb-anafis
```

### Rust

Add this to your `Cargo.toml`:

```toml
[dependencies]
symb_anafis = "0.3.0"

# Optional: Enable parallel evaluation
symb_anafis = { version = "0.3.0", features = ["parallel"] }
```

## Quick Start

### Python

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

### Rust

```rust
use symb_anafis::{diff, simplify};

fn main() {
    // Differentiate sin(x) * x with respect to x
    let derivative = diff(
        "sin(x) * x",
        "x",
        None, // No fixed variables
        None  // No custom functions
    ).unwrap();

    println!("Derivative: {}", derivative);
    // Output: cos(x) * x + sin(x)

    // Simplify an expression
    let simplified = simplify(
        "x^2 + 2*x + 1",
        None, // No fixed variables
        None  // No custom functions
    ).unwrap();

    println!("Simplified: {}", simplified);
    // Output: (x + 1)^2
}
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

✅ **Multi-Variable Calculus**
- Gradient, Hessian, and Jacobian computation
- Uncertainty propagation with covariance support
- Parallel batch evaluation (with `parallel` feature)

✅ **Beautiful Output**
- LaTeX export (`to_latex()`)
- Unicode output with Greek symbols (`to_unicode()`)

✅ **Flexible API**
- Type-safe Symbol/Expr API (no `.clone()` needed!)
- String-based API for quick scripts
- Fixed variables (constants that aren't differentiated)
- Custom function definitions
- Domain-safety mode

## Configuration

You can configure safety limits using environment variables:

- `SYMB_ANAFIS_MAX_DEPTH`: Maximum AST depth (default: 100)
- `SYMB_ANAFIS_MAX_NODES`: Maximum AST node count (default: 10000)
- `SYMB_ANAFIS_DOMAIN_SAFETY`: Enable domain-safe mode (default: false)

```bash
export SYMB_ANAFIS_MAX_DEPTH=200
export SYMB_ANAFIS_MAX_NODES=50000
export SYMB_ANAFIS_DOMAIN_SAFETY=true
```

### Domain-Safe Mode

Domain-safe mode prevents mathematically incorrect simplifications by skipping rules that could alter the domain of expressions. For example:

- **Without domain safety**: `sqrt(x^2)` → `x` (incorrect when x < 0)
- **With domain safety**: `sqrt(x^2)` → `abs(x)` (correct for all real x)

Enable domain-safe mode when you need guaranteed mathematical correctness:

#### Python
```bash
export SYMB_ANAFIS_DOMAIN_SAFETY=true
python -c "import symb_anafis; print(symb_anafis.simplify('sqrt(x^2)'))"
# Output: abs(x)
```

#### Rust
```rust
use symb_anafis::simplify;

fn main() {
    // Domain safety is controlled by SYMB_ANAFIS_DOMAIN_SAFETY environment variable
    let result = simplify(
        "sqrt(x^2)",
        Some(&["x"]),
        None
    ).unwrap();
    println!("Result: {}", result);  // With SYMB_ANAFIS_DOMAIN_SAFETY=true: abs(x)
}
```

## Examples

### Physics: RC Circuit

#### Python
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

#### Rust
```rust
use symb_anafis::diff;

fn main() {
    let current = diff(
        "V0 * exp(-t / (R * C))",
        "t",
        Some(&["V0", "R", "C"]),
        None
    ).unwrap();
    println!("Current: {}", current);
}
```

### Statistics: Normal Distribution

#### Python
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

#### Rust
```rust
use symb_anafis::diff;

fn main() {
    let derivative = diff(
        "exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)",
        "x",
        Some(&["mu", "sigma"]),
        None
    ).unwrap();
    println!("Derivative: {}", derivative);
}
```

### Calculus: Chain Rule

#### Python
```python
# Chain rule: d/dx[sin(cos(tan(x)))]
result = symb_anafis.diff("sin(cos(tan(x)))", "x")
print(result)
```

#### Rust
```rust
use symb_anafis::diff;

fn main() {
    let result = diff(
        "sin(cos(tan(x)))",
        "x",
        None,
        None
    ).unwrap();
    println!("Result: {}", result);
}
```

## API Reference

### Python API

#### `diff(formula, var, fixed_vars=None, custom_functions=None) -> str`

Differentiate a mathematical expression.

**Parameters:**
- `formula` (str): Mathematical expression (e.g., `"x^2 + sin(x)"`)
- `var` (str): Variable to differentiate with respect to
- `fixed_vars` (list, optional): Variables that are constants
- `custom_functions` (list, optional): User-defined function names

**Returns:** Simplified derivative as a string

**Raises:** `ValueError` if parsing/differentiation fails

**Note:** Set environment variable `SYMB_ANAFIS_DOMAIN_SAFETY=true` for domain-safe simplification.

#### `simplify(formula, fixed_vars=None, custom_functions=None) -> str`

Simplify a mathematical expression.

**Parameters:**
- `formula` (str): Expression to simplify
- `fixed_vars` (list, optional): Variables that are constants
- `custom_functions` (list, optional): User-defined function names

**Returns:** Simplified expression as a string

**Note:** Set environment variable `SYMB_ANAFIS_DOMAIN_SAFETY=true` for domain-safe simplification.

#### `parse(formula, fixed_vars=None, custom_functions=None) -> str`

Parse and normalize an expression.

**Parameters:**
- `formula` (str): Expression to parse
- `fixed_vars` (list, optional): Variables that are constants
- `custom_functions` (list, optional): User-defined function names

**Returns:** Normalized expression string

### Rust API

#### `diff(formula: &str, var: &str, fixed_vars: Option<&[&str]>, custom_functions: Option<&[&str]>) -> Result<String, DiffError>`

Differentiate a mathematical expression.

#### `simplify(formula: &str, fixed_vars: Option<&[&str]>, custom_functions: Option<&[&str]>) -> Result<String, DiffError>`

Simplify a mathematical expression. Set environment variable `SYMB_ANAFIS_DOMAIN_SAFETY=true` for domain-safe mode.

#### `symb(name: &str) -> Symbol`

Create a symbolic variable for building expressions programmatically.

#### `gradient_str(formula: &str, vars: &[&str]) -> Result<Vec<String>, DiffError>`

Compute the gradient of a scalar expression.

#### `hessian_str(formula: &str, vars: &[&str]) -> Result<Vec<Vec<String>>, DiffError>`

Compute the Hessian matrix of a scalar expression.

#### `jacobian_str(formulas: &[&str], vars: &[&str]) -> Result<Vec<Vec<String>>, DiffError>`

Compute the Jacobian matrix of a vector function.

## Advanced Usage

### Expression Simplification

You can simplify expressions without differentiation:

#### Python
```python
result = symb_anafis.simplify("sin(x)^2 + cos(x)^2")
print(result)  # Output: 1
```

#### Rust
```rust
use symb_anafis::simplify;

fn main() {
    let result = simplify(
        "sin(x)^2 + cos(x)^2",
        None,
        None
    ).unwrap();
    println!("Simplified: {}", result);  // Output: 1
}
```

### Multi-Character Symbols

For symbols with multiple characters (like "sigma", "alpha", etc.), pass them as fixed variables to ensure they're treated as single symbols:

#### Python
```python
# This treats "sigma" as one symbol
result = symb_anafis.simplify("(sigma^2)^2", fixed_vars=["sigma"])
print(result)  # Output: sigma^4
```

#### Rust
```rust
use symb_anafis::simplify;

fn main() {
    let result = simplify(
        "(sigma^2)^2",
        Some(&["sigma"]),
        None
    ).unwrap();
    println!("Simplified: {}", result);  // Output: sigma^4
}
```

### Fixed Variables and Custom Functions

You can define constants (like `a`, `b`) and custom functions (like `f(x)`) that are treated correctly during differentiation.

#### Python
```python
result = symb_anafis.diff("a * f(x)", "x", fixed_vars=["a"], custom_functions=["f"])
print(result)  # Output: a * ∂_f(x)/∂_x
```

#### Rust
```rust
use symb_anafis::diff;

fn main() {
    let result = diff(
        "a * f(x)",
        "x",
        Some(&["a"]),
        Some(&["f"])
    ).unwrap();
    println!("Derivative: {}", result);  // Output: a * ∂_f(x)/∂_x
}
```

### Domain-Safe Simplification

Domain-safe mode prevents mathematically incorrect simplifications:

#### Python
```bash
export SYMB_ANAFIS_DOMAIN_SAFETY=true
python -c "import symb_anafis; print(symb_anafis.simplify('sqrt(x^2)'))"
# Output: abs(x)  (correct for all real x)
```

#### Rust
```rust
use symb_anafis::simplify;

fn main() {
    // Domain safety is controlled by SYMB_ANAFIS_DOMAIN_SAFETY environment variable
    let result = simplify(
        "sqrt(x^2)",
        Some(&["x"]),
        None
    ).unwrap();
    println!("Result: {}", result);  // abs(x) when domain safety is enabled
}
```

## Supported Functions

- **Trigonometric**: `sin`, `cos`, `tan`, `cot`, `sec`, `csc`, `asin`, `acos`, `atan`, `acot`, `asec`, `acsc`
- **Hyperbolic**: `sinh`, `cosh`, `tanh`, `coth`, `sech`, `csch`, `asinh`, `acosh`, `atanh`, `acoth`, `asech`, `acsch`
- **Exponential/Logarithmic**: `exp`, `ln`, `log`, `log10`, `log2`
- **Roots**: `sqrt`, `cbrt`
- **Absolute Value/Sign**: `abs`, `sign`
- **Special**: `sinc`, `erf`, `erfc`, `gamma`, `digamma`, `trigamma`, `tetragamma`, `polygamma`, `beta`, `LambertW`, `besselj`, `bessely`, `besseli`, `besselk`

Note: The `polygamma(n, x)` function provides derivatives for all polygamma functions. Functions with non-elementary derivatives use custom notation.

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
git clone https://github.com/CokieMiner/SymbAnaFis.git
cd SymbAnaFis

# Build Python bindings
pip install maturin
maturin develop --release

# Build Rust library
cargo build --release

# Run tests
cargo test --release
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Citation

If you use SymbAnaFis in academic work, please cite:

```bibtex
@software{symb_anafis,
  author = {Pedro Martins},
  title = {SymbAnaFis: Fast Symbolic Differentiation Library},
  url = {https://github.com/CokieMiner/SymbAnaFis},
  year = {2025}
}
```

## Resources

- **GitHub:** https://github.com/CokieMiner/SymbAnaFis
- **Crates.io:** https://crates.io/crates/symb_anafis
- **PyPI:** https://pypi.org/project/symb-anafis/
- **Documentation:** https://docs.rs/symb_anafis

---

**Built with ❤️ in Rust for Python** 🚀
