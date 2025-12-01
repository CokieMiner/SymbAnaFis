# Examples

This directory contains comprehensive examples demonstrating the capabilities of SymbAnaFis.

## Organized Examples

### Simplification Examples

#### **[simplification.rs](simplification.rs)** - COMPREHENSIVE ⭐
Complete showcase of all simplification rules including:
- Algebraic (factoring, fractions, powers, signs)
- Trigonometric identities (basic + advanced, multiple angles)
- Hyperbolic identities  
- Logarithmic/Exponential properties (advanced combinations)
- Root simplifications
- Division simplifications
- Numeric simplifications
- Complex expressions and edge cases

```bash
cargo run --example simplification
```

### Differentiation Examples

#### **[differentiation.rs](differentiation.rs)** - COMPREHENSIVE ⭐
Comprehensive demonstration of symbolic differentiation:
- Basic rules (power, product, quotient)
- Chain rule applications (simple + complex)
- Trigonometric derivatives (standard + inverse)
- Hyperbolic derivatives
- Exponential and logarithmic derivatives (including complex powers)
- Special functions (sqrt, erf)
- Complex multi-rule expressions (extremely nested)
- Higher-order derivatives
- Custom functions
- Partial derivatives and multivariable calculus

```bash
cargo run --example differentiation
```

### Application Examples

#### **[applications.rs](applications.rs)** - COMPREHENSIVE ⭐
Real-world physics, engineering, and calculus applications:
- Kinematics, electricity, thermodynamics, quantum mechanics
- Fluid dynamics, optics, control systems, statistics
- Chemical kinetics, acoustics, electromagnetism
- Harmonic oscillators, relativity, astrophysics
- Related rates, optimization, Taylor series, differential equations
- Vector calculus, linear approximation, mean value theorem
- L'Hôpital's rule, arc length, surface area

```bash
cargo run --example applications
```

### Feature Examples

#### **[features.rs](features.rs)** - COMPREHENSIVE ⭐
Working with advanced features:
- Custom functions and fixed variables (complex chains)
- Implicit functions
- Complex nested expressions
- Polynomials
- Partial derivatives
- Multivariable functions
- Vector calculus (gradients, divergence)

```bash
cargo run --example features
```

### Basic Examples

#### **[basics.rs](basics.rs)** - COMPREHENSIVE ⭐
Simple getting-started example showing the core API:
- Basic differentiation (power rule through complex polynomials)
- Basic simplification (like terms through nested expressions)
- Fixed variables & custom functions (advanced combinations)
- Multi-character symbols (Greek letters and complex expressions)

```bash
cargo run --example basics
```

## Running Examples

Run any example with:
```bash
cargo run --example <example_name>
```

For example:
```bash
cargo run --example basics
```

## Example Categories

| Category | Example |
|----------|---------|
| **Simplification** | `simplification` |
| **Differentiation** | `differentiation` |
| **Applications** | `applications` |
| **Features** | `features` |
| **Basic** | `basics` |

