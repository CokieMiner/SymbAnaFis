# SymbAnaFis

[![Crates.io](https://img.shields.io/crates/v/symb_anafis.svg)](https://crates.io/crates/symb_anafis)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![no-std](https://img.shields.io/badge/no--std-alloc-brightgreen.svg)](#cargo-features)
[![Docs.rs](https://docs.rs/symb_anafis/badge.svg)](https://docs.rs/symb_anafis)

SymbAnaFis is a Rust library for symbolic mathematics: it builds, transforms,
differentiates, and evaluates mathematical expressions. Numbers come from
[NumAnaFis](https://github.com/CokieMiner/NumAnaFis): integers and rationals stay
exact, and floats keep their IEEE 754 representation.

## Features

- **Expressions.** Expressions are immutable and hash-consed: equal
  subexpressions share one node, so cloning, equality, and hashing take
  constant time. Sums and products are normalized and collect equal terms.
- **Domain properties.** Conservative scalar proofs (`Property`), mutable symbol
  assumptions (`Symbol::set_properties`), and custom function return-domain
  declarations (`UserFunction::returns`).
- **Parsing.** Text such as `2x + sin(x)^2`, with implicit multiplication, exact
  integer literals, registered functions, and byte-range error spans.
- **Calculus.** Arbitrary-order and mixed partial derivatives, directional
  derivatives, gradients, Jacobians, Hessians, and Laplacians. Unsupported
  derivatives stay symbolic.
- **Substitution and rewriting.** Concrete substitutions, including groups inside
  larger sums and products, and pattern rules with captures, conditions, and
  callback replacements.
- **Simplification.** Compact, expanded, factored, and compilation presets, or
  custom sequences of passes, rules, and callbacks ranked by a custom score.
- **Evaluation.** Direct evaluation, compiled register VM programs for `f64`,
  `f32`, `Complex<f64>`, `Complex<f32>`, exact and complex `Number`, or any
  compatible `Numeric` type, optional host W^X machine-code JIT (`native_jit` on
  `x86_64`, `aarch64`, and `riscv64`, with lazy AVX2/AVX-512 SIMD batch kernels
  on `x86_64`), parallel Rayon batches (`rayon`), and standalone WebAssembly
  (`.wasm`) export.
- **Uncertainty.** First-order propagation from independent variances, Cholesky
  factors (`from_factor`), or symbolic covariance matrices, plus sensitivity
  recontraction (`recontract`).
- **Custom functions.** Fixed or variable arity (`min..=max`), optional symbolic
  bodies, partial derivatives, and return-domain properties.
- **Python bindings.** Owning Python classes with optional zero-copy NumPy
  buffers (`f64`, `f32`, `complex128`, `complex64`), N-D grid broadcasting,
  `__array_ufunc__`, native JIT, Rayon parallel batches, and `.wasm` export.

Normalizing keeps domain exclusions: `x/x` stays `x/x` and `x - x` becomes
`0*x`. Identities that depend on assumptions apply only through explicit rules,
proved symbol properties, or opt-in simplification modes. An exact result that
does not fit the numeric backend keeps its operation symbolic.

## Installation

Add `symb_anafis` to your `Cargo.toml`:

```toml
[dependencies]
symb_anafis = "0.9"
```

For standard library integration or full native JIT and parallel batch execution
(see [Cargo Features](#cargo-features)):

```toml
[dependencies]
symb_anafis = { version = "0.9", features = ["std", "native_jit", "rayon"] }
```

## Quick Start

```rust
use symb_anafis::{Context, Substitution};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let context = Context::new()?;
    let x = context.symb("x")?;
    let y = context.symb("y")?;
    let u = context.symb("u")?;

    let expression = (x + y).pow(2) + (x + y).sin();
    let substitution = Substitution::new(x + y, u.into());
    assert_eq!(substitution.apply_all(&expression), u.pow(2) + u.sin());

    assert_eq!(context.parse("x^3")?.derivative(x, 1), 3 * x.pow(2));
    Ok(())
}
```

## Rewriting

A rule applies an identity inside a larger expression and keeps the unmatched
terms:

```rust
use symb_anafis::{Context, Pattern, RewriteRule};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let context = Context::new()?;
    let x = context.symb("x")?;
    let y = context.symb("y")?;
    let angle = Pattern::capture("angle", None);
    let identity = RewriteRule::new(
        angle.sin().pow(2) + angle.cos().pow(2),
        Pattern::exact(1),
    )?;

    let expression = x.sin().pow(2) + x.cos().pow(2) + y;
    assert_eq!(identity.apply_all(&expression, usize::MAX, None)?, y + 1);
    Ok(())
}
```

Captures bind whole subexpressions, so the rule also applies to composite
angles. Conditions can require domain proofs, and replacement callbacks can
build results whose size depends on the captures. The
[API reference](https://github.com/CokieMiner/SymbAnaFis/blob/main/docs/API.md)
has examples of simultaneous substitutions, custom partial derivatives, guarded
identities, and a
[binomial expansion of variable order](https://github.com/CokieMiner/SymbAnaFis/blob/main/docs/api/rewriting.md#6-binomial-expansion-with-a-variable-order).

## Simplification

Canonical transformations are available directly on expressions:

```rust
use symb_anafis::{Context, SimplificationSequence, SimplificationStep};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let context = Context::new()?;
    let x = context.symb("x")?;
    let y = context.symb("y")?;
    let expression = context.parse("sin(x)^2 + cos(x)^2")?;

    // Direct, canonical methods on Expr:
    assert_eq!(expression.compact()?, 1.into());
    assert_eq!((x + 1).pow(2).expand()?, x.pow(2) + 2 * x + 1);
    assert_eq!((x.pow(2) + x * y).factor()?, x * (x + y));

    // Custom multi-step pipelines:
    let custom = SimplificationSequence::new()
        .then(SimplificationStep::Trigonometric)
        .then(SimplificationStep::Factor);
    assert_eq!(custom.simplify(&expression)?.into_expression(), 1.into());

    // Numerical compilation preparation:
    let compiled = SimplificationSequence::compilation().simplify(&(x.exp() - 1))?;
    assert_eq!(compiled.into_expression(), x.exp_m1());
    Ok(())
}
```

By default, sequences keep proved domains and principal branches.
`DomainSafety::AssumptionDependent` also admits tagged identities whose
assumptions belong to the caller. The runner keeps the best visited form,
detects cycles, and enforces configurable limits. See
[simplification](https://github.com/CokieMiner/SymbAnaFis/blob/main/docs/api/simplification.md)
and the
[rule reference](https://github.com/CokieMiner/SymbAnaFis/blob/main/src/simplification/RULES.md).

## Calculus and Uncertainty

```rust
use symb_anafis::{Context, CovarianceMatrix, Expr, Jacobian, Uncertainty};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let context = Context::new()?;
    let x = context.symb("x")?;
    let y = context.symb("y")?;
    let model = x + 2 * y;
    assert_eq!(model.gradient(&[x, y]), vec![Expr::number(1), Expr::number(2)]);
    assert_eq!([model.clone(), x * y].jacobian(&[x, y]), vec![
        vec![Expr::number(1), Expr::number(2)],
        vec![y.to_expr(), x.to_expr()],
    ]);

    let covariance = CovarianceMatrix::diagonal(vec![Expr::number(4), Expr::number(9)])?;
    let uncertainty = Uncertainty::propagate(&model, &[x, y], &covariance)?;
    assert_eq!(uncertainty.variance(), &Expr::number(40));
    assert_eq!(uncertainty.standard_deviation(), Expr::number(40).sqrt());
    Ok(())
}
```

Diagonal inputs are variances. Propagation assumes real coordinates, a regular
model, a positive semidefinite covariance, and an adequate linear
approximation. See
[multivariable calculus](https://github.com/CokieMiner/SymbAnaFis/blob/main/docs/api/calculus.md#2-multivariable-calculus)
and
[uncertainty propagation](https://github.com/CokieMiner/SymbAnaFis/blob/main/docs/api/calculus.md#3-uncertainty-propagation).

## Numerical Evaluation

```rust
use symb_anafis::{Context, Program, Workspace};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let context = Context::new()?;
    let x = context.symb("x")?;
    let y = context.symb("y")?;
    let expression = x.sin() + y.pow(2);

    // Compile once to register bytecode, or upgrade to native JIT when `native_jit` is enabled:
    let program = Program::<f64>::compile(&expression, &[x, y])?;
    let program = program.with_native().unwrap_or(program);

    let mut workspace = Workspace::new();
    assert!((program.eval_with(&mut workspace, &[0.0, 3.0])? - 9.0).abs() < 1e-12);

    let mut out = [0.0; 2];
    program.eval_batch_with(&mut workspace, &[&[0.0, 0.0], &[2.0, 4.0]], &mut out)?;
    assert_eq!(out, [4.0, 16.0]);

    // Pure-Rust standalone WebAssembly export (no feature flag required):
    let wasm = program.to_wasm_bytes()?;
    assert_eq!(&wasm[..8], b"\0asm\x01\0\0\0");
    Ok(())
}
```

With the `rayon` feature enabled, `eval_batch_par` evaluates columns in parallel
across the active Rayon thread pool. See
[numerical evaluation](https://github.com/CokieMiner/SymbAnaFis/blob/main/docs/api/evaluation.md).

## Python

Build the extension from this checkout with `maturin develop --release` in a
Python virtual environment (NumPy is optional via `symb_anafis[numpy]`):

```python
from symb_anafis import Context, Evaluator, Expr, Property

context = Context()
x, y = context.symbols("x", "y")
expression = x**2 + x*y
assert expression.gradient([x, y]) == [2*x+y, x]
assert Expr.jacobian([expression, x*y], [x, y]) == [[2*x+y, x], [y, x]]
assert Evaluator(expression, [x, y]).evaluate([2, 3]) == 10
assert Evaluator(expression, [x, y], backend="f64", native=True).evaluate([2.0, 3.0]) == 10.0
assert (x.sin()**2 + x.cos()**2).compact() == Expr(1)
assert ((x + 1)**2).expand() == x**2 + 2*x + 1
assert (x**2 + x*y).factor() == x * (x + y)
x.set_properties([Property.POSITIVE])
assert (x**2).sqrt().compact() == x
assert "x" in expression
```

See the [Python API](https://github.com/CokieMiner/SymbAnaFis/blob/main/docs/PYTHON.md)
for zero-copy NumPy N-D batches, `__array_ufunc__`, parallel execution, and
WebAssembly export.

## Cargo Features

The default build is `no_std` with `alloc`; the target needs a global allocator
and pointer-width atomics.

| Feature | Description |
| --- | --- |
| `std` | Blocking registry locks, runtime VM SIMD dispatch, and the NumAnaFis `std` backend. |
| `native_jit` | Enables `std` and in-memory host machine-code JIT compilation (`Program::with_native`) on `x86_64`, `aarch64`, and `riscv64` (Linux, macOS, Windows), plus lazy AVX2/AVX-512 SIMD batch kernels on `x86_64`. |
| `rayon` | Enables `std`, parallel batch evaluation (`Program::eval_batch_par` / `MultiProgram::eval_batch_par`), and parallel NTT convolutions. |
| `num-traits` | `Zero` and `One` for `Expr`. |

## Documentation

- [Examples](https://github.com/CokieMiner/SymbAnaFis/blob/main/examples/README.md): runnable introductions, DAG inspection, bytecode listings, and scientific applications.
- [API reference](https://github.com/CokieMiner/SymbAnaFis/blob/main/docs/API.md): operations, contracts, and examples.
- [Python API](https://github.com/CokieMiner/SymbAnaFis/blob/main/docs/PYTHON.md): classes, NumPy integration, build, and examples.
- [Project guidelines](https://github.com/CokieMiner/SymbAnaFis/blob/main/AGENTS.md): architecture, contribution rules, and verification.

## Citation

```bibtex
@software{symbanafis,
  author       = {Martins, Pedro},
  orcid        = {0009-0001-8170-2930},
  title        = {SymbAnaFis: High-performance symbolic mathematics},
  year         = {2026},
  url          = {https://github.com/CokieMiner/SymbAnaFis},
  version      = {0.9.0}
}
```

## License

SymbAnaFis is licensed under the Apache License, Version 2.0. See
[LICENSE](https://github.com/CokieMiner/SymbAnaFis/blob/main/LICENSE).
