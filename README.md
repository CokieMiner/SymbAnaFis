# SymbAnaFis: Symbolic Analysis for Physics

A fast, focused Rust library for symbolic differentiation and analysis, designed for physics and engineering applications.

## Features

- **Symbolic Differentiation**: Compute derivatives of complex mathematical expressions.
- **Context-Aware Parsing**: Support for fixed variables (constants) and custom functions.
- **Simplification**: Comprehensive algebraic, trigonometric, hyperbolic, and logarithmic simplification rules.
- **Extensible**: Easily add new functions and simplification rules.
- **Safe**: Built with Rust's safety guarantees, with limits on recursion depth and node count.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
symb_anafis = "0.1.0"
```

## Quick Start

```rust
use symb_anafis::{diff, simplify};

fn main() {
    // Differentiate sin(x) * x with respect to x
    let derivative = diff(
        "sin(x) * x".to_string(),
        "x".to_string(),
        None, // No fixed variables
        None  // No custom functions
    ).unwrap();

    println!("Derivative: {}", derivative);
    // Output: cos(x) * x + sin(x)

    // Simplify an expression
    let simplified = simplify(
        "x^2 + 2*x + 1".to_string(),
        None, // No fixed variables
        None  // No custom functions
    ).unwrap();

    println!("Simplified: {}", simplified);
    // Output: (x + 1)^2
}
```

## Configuration

You can configure safety limits using environment variables:

- `SYMB_ANAFIS_MAX_DEPTH`: Maximum AST depth (default: 100)
- `SYMB_ANAFIS_MAX_NODES`: Maximum AST node count (default: 10000)

```bash
export SYMB_ANAFIS_MAX_DEPTH=200
export SYMB_ANAFIS_MAX_NODES=50000
```

## Advanced Usage

### Expression Simplification

You can simplify expressions without differentiation:

```rust
use symb_anafis::simplify;

fn main() {
    // Simplify a complex expression
    let result = simplify(
        "sin(x)^2 + cos(x)^2".to_string(),
        None, // No fixed variables
        None  // No custom functions
    ).unwrap();

    println!("Simplified: {}", result);
    // Output: 1
}
```

### Multi-Character Symbols

For symbols with multiple characters (like "sigma", "alpha", etc.), pass them as fixed variables to ensure they're treated as single symbols:

```rust
use symb_anafis::simplify;

fn main() {
    // This treats "sigma" as one symbol
    let result = simplify(
        "(sigma^2)^2".to_string(),
        Some(&["sigma".to_string()]),
        None
    ).unwrap();
    println!("Simplified: {}", result);
    // Output: sigma^4
}
```

### Fixed Variables and Custom Functions

You can define constants (like `a`, `b`) and custom functions (like `f(x)`) that are treated correctly during differentiation.

```rust
use symb_anafis::diff;

fn main() {
    // Differentiate a * f(x) with respect to x
    let result = diff(
        "a * f(x)".to_string(),
        "x".to_string(),
        Some(&["a".to_string()]),       // 'a' is a constant
        Some(&["f".to_string()])        // 'f' is a custom function
    ).unwrap();

    println!("Derivative: {}", result);
    // Output: a * ∂_f(x)/∂_x
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

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
