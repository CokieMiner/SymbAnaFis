# Examples

This directory contains examples demonstrating SymbAnaFis capabilities.

## Quick Reference

| Example | Description | Run With |
|---------|-------------|----------|
| **quickstart** | Minimal 25-line demo | `cargo run --example quickstart` |
| **api_showcase** | Complete API tour (10 parts) | `cargo run --example api_showcase` |
| **applications** | Physics & engineering | `cargo run --example applications` |

## quickstart.rs - Get Started Fast

Core features in 25 lines: differentiation, simplification, Symbol Copy, LaTeX/Unicode output.

```bash
cargo run --example quickstart
```

## api_showcase.rs - Complete API Tour

Comprehensive demo of ALL features:

| Part | Feature |
|------|---------|
| 1 | String-based API (`diff`, `simplify`) |
| 2 | Type-safe API + Symbol Copy |
| 3 | Numerical evaluation |
| 4 | Multi-variable calculus (gradient, hessian, jacobian) |
| 5 | All 60+ supported functions |
| 6 | Custom derivatives & evaluation |
| 7 | Safety features (max depth, domain safety) |
| 8 | Expression output (LaTeX, Unicode) |
| 9 | Uncertainty propagation |
| 10 | Parallel evaluation (requires `parallel` feature) |

```bash
cargo run --example api_showcase
cargo run --example api_showcase --features parallel  # Include Part 10
```

## applications.rs - Real-World Physics

20 physics & engineering applications including:
- Kinematics, thermodynamics, quantum mechanics
- Fluid dynamics, optics, control systems
- Electromagnetism, relativity, astrophysics

```bash
cargo run --example applications
```
