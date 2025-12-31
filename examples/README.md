# Examples

This directory contains examples demonstrating SymbAnaFis capabilities.

## Quick Reference

| Example | Description | Run With |
|---------|-------------|----------|
| **quickstart** | Minimal 25-line demo | `cargo run --example quickstart` |
| **api_showcase** | Complete API tour (15 sections) | `cargo run --example api_showcase` |
| **dual_autodiff** | Automatic differentiation with dual numbers | `cargo run --example dual_autodiff` |
| **applications** | Physics & engineering | `cargo run --example applications` |
| **simplification_comparison** | Compare against Symbolica CAS | `cargo run --example simplification_comparison` |

## quickstart.rs - Get Started Fast

Core features in 25 lines: differentiation, simplification, Symbol Copy, LaTeX/Unicode output.

```bash
cargo run --example quickstart
```

## api_showcase.rs - Complete API Tour

Comprehensive demo of ALL features across 15 sections:

| Section | Feature |
|---------|---------|
| 1 | Quick Start |
| 2 | Symbol Management |
| 3 | Core Functions |
| 4 | Builder Pattern API |
| 5 | Expression Output |
| 6 | Uncertainty Propagation |
| 7 | Custom Functions |
| 8 | Evaluation |
| 9 | Vector Calculus |
| 10 | Automatic Differentiation |
| 11 | Parallel Evaluation |
| 12 | Compilation & Performance |
| 13 | Built-in Functions |
| 14 | Expression Syntax |
| 15 | Error Handling |

```bash
cargo run --example api_showcase
cargo run --example api_showcase --features parallel  # Include Section 11
```

Python version available: `python examples/api_showcase.py`

## dual_autodiff.rs - Automatic Differentiation

Demonstrates SymbAnaFis's `Dual<T>` number implementation for exact derivative computation through algebraic manipulation.

**Features shown:**
- Basic dual number arithmetic
- Transcendental functions (sin, exp)
- Higher-order derivatives
- Chain rule application
- Comparison with symbolic differentiation

**Sample Output:**
```text
=== Quadratic Function: f(x) = x² + 3x + 1 ===
f(2) = 11
f'(2) = 7
Expected: f(2) = 11, f'(2) = 7

=== Transcendental Function: f(x) = sin(x) * exp(x) ===
f(1) = 2.287355
f'(1) = 5.435620
Expected f'(1) = 5.435620
Error: 0.00e0
```

```bash
cargo run --example dual_autodiff
```

Python version available: `python examples/dual_autodiff.py`

## applications.rs - Real-World Physics

20 physics & engineering applications including:
- Kinematics, thermodynamics, quantum mechanics
- Fluid dynamics, optics, control systems
- Electromagnetism, relativity, astrophysics

```bash
cargo run --example applications
```

## simplification_comparison.rs - Benchmarking Accuracy

Compares SymbAnaFis differentiation quality against the commercial Symbolica engine across complex physics expressions (Maxwell-Boltzmann, Planck Blackbody, etc.).

**Sample Output:**
```text
--- Logistic Sigmoid (d/dx) ---
Input: 1/(1+exp(-k*(x-x0)))
SymbAnaFis (Simplified): exp(-k*(x - x0))*k/(1 + exp(-k*(x - x0)))^2
Symbolica:               k*(exp(-k*(x-x0))+1)^-2*exp(-k*(x-x0))
```