# SymbAnaFis API Reference

`symb_anafis` is a high-performance library for symbolic mathematics, differentiation, pattern rewriting, and numerical compilation. Numbers are provided by `NumAnaFis`: integers and rationals stay exact, and floats preserve their IEEE 754 bit representations.

---

### Installation & Features

Add `symb_anafis` to your `Cargo.toml`:

```toml
[dependencies]
symb_anafis = "0.9"
```

For standard library integration (enabling blocking mutexes and `num_anafis/std`):

```toml
[dependencies]
symb_anafis = { version = "0.9", features = ["std"] }
```

| Feature | Behavior |
| --- | --- |
| Default (`no_std`) | Uses `alloc`; dynamically allocated collections and internal spin mutexes. |
| `std` | Uses `std::sync` blocking mutexes and standard library numerical backends. |
| `num-traits` | Implements `Zero` and `One` from the `num-traits` crate for `Expr`. |

---

### Quickstart

```rust
use symb_anafis::{Context, Expr, Jacobian, Substitution};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let ctx = Context::new()?;
    let x = ctx.symb("x")?;
    let y = ctx.symb("y")?;
    let u = ctx.symb("u")?;

    // 1. Immutable hash-consed expressions
    let expr = (x + y).pow(2) + (x + y).sin();

    // 2. Concrete substitution
    let sub = Substitution::new(x + y, u.into());
    assert_eq!(sub.apply_all(&expr), u.pow(2) + u.sin());

    // 3. Symbolic differentiation
    assert_eq!(Expr::derivative(x.pow(3), x, 1), 3 * x.pow(2));

    // 4. Multivariable calculus (gradient & Jacobian)
    let model = x.pow(2) + x * y;
    assert_eq!(model.gradient(&[x, y]), vec![2 * x + y, x.to_expr()]);
    assert_eq!([model, x * y].jacobian(&[x, y]), vec![
        vec![2 * x + y, x.to_expr()],
        vec![y.to_expr(), x.to_expr()],
    ]);

    Ok(())
}
```

---

### Modular API Guides

The API reference is organized into focused modular guides:

| Module | Topic | Description |
| --- | --- | --- |
| [**Core Expressions**](api/core.md) | Expressions & Representations | Node DAG, hash-consing, symbols, constants, properties, normalization, formatting, and limits. |
| [**Rewriting & Patterns**](api/rewriting.md) | Pattern Matching & Rules | Concrete substitutions, captures, predicates, replacement callbacks, rule sets, and binomial expansions. |
| [**Calculus & Uncertainty**](api/calculus.md) | Differentiation & Uncertainty | Arbitrary-order derivatives, gradients, Jacobians, Hessians, and first-order covariance propagation. |
| [**Simplification**](api/simplification.md) | Transformations & Presets | Sequences, presets (`compact`, `expanded`, `factored`, `compilation`), scoring, limits, and domain safety modes. |
| [**Parsing**](api/parsing.md) | Mathematical Grammar | Syntax parser, implicit multiplication, custom function dispatch, byte-range diagnostics (`Span`). |
| [**Numerical Evaluation**](api/evaluation.md) | Lowering & Execution | Direct evaluation, compiled register VM programs, bytecode disassembly, batch runs, and generic backends. |

---

### Key Architectural Principles

1. **Immutable & Hash-Consed DAG**: Every expression node is immutable and canonicalized through an internal table. Subexpression sharing is automatic; equality, cloning, and hashing are $O(1)$ operations.
2. **Explicit Domain Safety**: Principal branch cuts, pole exclusions, and removable singularities are strictly respected. `x/x` remains an unsimplified quotient by default unless explicit assumptions or `DomainSafety::AssumptionDependent` are specified.
3. **No Float Promotion**: Exact integers and rationals are never implicitly coerced to floating-point numbers during simplification or differentiation.
4. **Separate Compilation**: Differentiation and simplification are purely symbolic. Numerical evaluation can run directly on the DAG or compile to flat bytecode register instructions executed via thread-local [`Workspace`](api/evaluation.md#2-compiled-scalar-programs).
