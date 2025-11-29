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
use symb_anafis::diff;

fn main() {
    // Differentiate sin(x) * x with respect to x
    let result = diff(
        "sin(x) * x".to_string(),
        "x".to_string(),
        None, // No fixed variables
        None  // No custom functions
    ).unwrap();

    println!("Derivative: {}", result);
    // Output: cos(x) * x + sin(x)
}
```

## Advanced Usage

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
- **Special**: `sinc`, `erf`, `erfc`, `gamma`, `digamma`, `trigamma`, `tetragamma`, `polygamma`, `beta`, `LambertW`, `besselj`, `bessely`, `besseli`, `besselk`

Note: The `polygamma(n, x)` function provides derivatives for all polygamma functions. Functions with non-elementary derivatives use custom notation.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
