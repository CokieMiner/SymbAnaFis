# SymbAnaFis

[![Crates.io](https://img.shields.io/crates/v/symb_anafis.svg)](https://crates.io/crates/symb_anafis)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![no-std](https://img.shields.io/badge/no--std-alloc-brightgreen.svg)](#cargo-features)
[![Docs.rs](https://docs.rs/symb_anafis/badge.svg)](https://docs.rs/symb_anafis)

SymbAnaFis is a Rust library for building and transforming symbolic mathematical expressions. It combines natural arithmetic syntax, exact numbers, and programmable rewrite rules, with an emphasis on performance and memory efficiency.

It is intended for scientific software, algebraic tools, and applications that need to manipulate formulas programmatically. Expressions can combine variables, mathematical constants, elementary and special functions, and user-defined functions.

## Capabilities

- **Expression construction and normalization.** Write formulas with arithmetic operators and mathematical methods. Sums and products collect equal terms and repeated factors automatically.
- **Text expressions.** Parse formulas such as `2x + sin(x)^2` through a context, with exact integer literals, implicit multiplication, registered functions, and source locations for errors.
- **Symbolic differentiation.** Compute derivatives, gradients, Jacobians, and Hessians through the shared expression DAG. Jacobian columns reuse work across outputs; unsupported derivatives remain symbolic.
- **Uncertainty propagation.** Construct first-order combined variance, standard deviation, and relative uncertainty from independent variances or a correlated symbolic covariance matrix, with explicit real-model and covariance assumptions.
- **Exact and approximate numbers.** Work with integers, reduced rationals, floats, and scalar complex values through [NumAnaFis](https://github.com/CokieMiner/NumAnaFis). Supported exact results remain exact; approximate inputs can be evaluated numerically.
- **Numerical evaluation.** Evaluate a formula directly or compile a reusable scalar register program. Select native `f64`, exact and complex `Number`, or a compatible `Numeric` backend.
- **Concrete substitutions.** Replace variables or compound expressions throughout a formula, including groups inside larger sums and products. Apply multiple replacements simultaneously when order must not cause cascading changes.
- **Pattern-based rewriting.** Describe families of expressions with named captures, restrict matches with conditions, and construct replacements from templates or callbacks.
- **Target based simplification.** Use compact, expanded, factored, or compilation presets, or build an ordered sequence from builtin passes, existing rewrite rules, and mapping callbacks. Supply a custom score to choose the preferred visited form.
- **Custom functions.** Register functions with an accepted argument count, optional symbolic bodies, and custom partial derivatives. Keep separate symbol and function namespaces for independent calculations.
- **Inspection and domain analysis.** Inspect mathematical structure, traverse shared expressions, and query conservative facts such as realness, positivity, and integrality.
- **Python API.** Use owning classes for expressions, scoped contexts, rewriting, simplification sequences, numerical programs, multivariable calculus, and uncertainties.

Expressions are immutable and share repeated subexpressions, allowing common algebraic pieces to be reused across formulas. Normalization preserves domain exclusions; explicit rules let applications choose the additional identities and assumptions used in a transformation.

## Quick Start

```rust
use symb_anafis::{Context, Substitution};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let u = ctx.symb("u")?;

    let expression = (x + y).pow(2) + (x + y).sin();
    let substitution = Substitution::new(x + y, u.into());
    let result = substitution.apply_all(&expression);

    assert_eq!(result, u.pow(2) + u.sin());
    Ok(())
}
```

The substitution replaces both occurrences of `x + y`, including the ones inside the power and sine call.

## Rewriting Expressions

A rule can describe a mathematical identity and apply it inside a larger expression. Unmatched terms remain in place:

```rust
use symb_anafis::{Context, Pattern, RewriteRule};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let angle = Pattern::capture("angle", None);
    let identity = RewriteRule::new(
        angle.sin().pow(2) + angle.cos().pow(2),
        Pattern::exact(1),
    )?;

    let expression = x.sin().pow(2) + x.cos().pow(2) + y;
    let result = identity.apply_all(&expression, None, None)?;

    assert_eq!(result, y + 1);
    Ok(())
}
```

Captures can represent entire expressions, so the same rule also applies to composite angles. Conditions can require domain proofs before accepting a match, while replacement callbacks can generate expressions whose size depends on the captures, such as the terms of a binomial expansion.

The [API reference](docs/API.md) includes examples of simultaneous substitutions, custom functions and partial derivatives, guarded identities, and [binomial expansion with a variable order](docs/API.md#16-binomial-expansion-with-a-variable-order).

## Simplification Sequences

Presets use the same steps available to custom sequences:

```rust
use symb_anafis::{Context, SimplificationSequence, SimplificationStep};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new();
    let x = ctx.symb("x")?;
    let expression = ctx.parse("sin(x)^2 + cos(x)^2")?;
    let compact = SimplificationSequence::compact();
    assert_eq!(compact.simplify(&expression)?.into_expression(), 1.into());

    let custom = SimplificationSequence::new()
        .then(SimplificationStep::Trigonometric)
        .then(SimplificationStep::Factor);
    assert_eq!(custom.simplify(&expression)?.into_expression(), 1.into());

    let expanded = SimplificationSequence::expanded().simplify(&(x + 1).pow(2))?;
    assert_eq!(expanded.into_expression(), x.pow(2) + 2 * x + 1);

    let compiled = SimplificationSequence::compilation().simplify(&(x.exp() - 1))?;
    assert_eq!(compiled.into_expression(), x.exp_m1());
    Ok(())
}
```

Sequences preserve proved domains and principal branches by default. `domain_safety(DomainSafety::AssumptionDependent)` additionally permits explicitly tagged identities whose domain and branch assumptions belong to the caller. Custom rules keep their caller-owned contract in either mode. The runner keeps the best visited candidate, permits temporary growth between steps, detects cycles, and enforces configurable limits. Compiled rule plans are reused across calls. The compilation preset selects native numerical forms such as `exp_m1` and `ln_1p` using an operation cost that counts shared DAG nodes once. See [simplification](docs/API.md#simplification) and the [complete rule reference](src/simplification/RULES.md) for sequences, scoring, domain conditions, and legacy coverage.

## Calculus and Uncertainties

```rust
use symb_anafis::{Context, CovarianceMatrix, Expr, Uncertainty};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let context = Context::new();
    let x = context.symb("x")?;
    let y = context.symb("y")?;
    let model = x + 2 * y;
    assert_eq!(model.gradient(&[x, y]), vec![Expr::number(1), Expr::number(2)]);

    let covariance = CovarianceMatrix::diagonal(vec![Expr::number(4), Expr::number(9)])?;
    let uncertainty = Uncertainty::propagate(&model, &[x, y], &covariance)?;
    assert_eq!(uncertainty.variance(), &Expr::number(40));
    assert_eq!(uncertainty.standard_deviation(), Expr::number(40).sqrt());
    Ok(())
}
```

Diagonal inputs are variances. Propagation assumes real coordinates, a regular
model, a positive semidefinite covariance, and an adequate linear approximation.
Unknown symbolic hypotheses belong to the caller. See the [multivariable
calculus](docs/API.md#multivariable-calculus) and [uncertainty
reference](docs/API.md#uncertainty-propagation) for matrix conventions, correlated
inputs, domain conditions, and relative uncertainty.

## Python

Build the extension from this checkout with `maturin develop --release` in a
Python virtual environment. The API exposes classes and methods:

```python
from symb_anafis import Context, SimplificationSequence

context = Context()
x, y = context.symbols("x", "y")
expression = x**2 + x*y
assert expression.gradient([x, y]) == [2*x+y, x]
assert expression.compile([x, y]).evaluate([2, 3]) == 10
assert SimplificationSequence.compact().simplify(x.sin()**2+x.cos()**2).expression.value == 1
```

See the [Python API](docs/PYTHON.md) for installation, exact numbers, callbacks,
custom sequences, compiled batches, covariance propagation, and error contracts.

## Cargo Features

The default build uses `no_std + alloc`, requiring a global allocator and pointer-width atomics. Both integration features are optional:

| Feature | Description |
| --- | --- |
| `std` | Enables standard-library integration and the NumAnaFis `std` backend. |
| `num-traits` | Implements the `Zero` and `One` traits for `Expr`. |

## Documentation

- [Rust examples](examples/README.md): Runnable introductions, DAG inspection, bytecode listings, and scientific applications.
- [API reference](docs/API.md): Core operations, contracts, and examples, including binomial expansion through callbacks.
- [Python API](docs/PYTHON.md): Owning Python classes, package build, examples, and verification.
- [Roadmap](docs/ROADMAP.md): Symbolic engine milestones, numerical foundations, and ecosystem projects.
- [Project guidelines](AGENTS.md): Architecture, contribution rules, and verification commands.

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

`SymbAnaFis` is licensed under the Apache License, Version 2.0.
See [LICENSE](LICENSE) for details.
