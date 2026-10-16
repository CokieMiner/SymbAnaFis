# SymbAnaFis

[![Crates.io](https://img.shields.io/crates/v/symb_anafis.svg)](https://crates.io/crates/symb_anafis)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![no-std](https://img.shields.io/badge/no--std-alloc-brightgreen.svg)](#features)

`SymbAnaFis` provides high-performance symbolic mathematics, exact expression manipulation, and term rewriting in pure Rust.

Designed for speed, mathematical soundness, and minimal memory footprints across complex algebraic workloads:
- **Globally interned expression DAG (`Expr`)**: One-pointer owning handles utilizing the null-pointer niche; $O(1)$ structural equality and hashing via collision-safe weak interning.
- **Embedded sparse multivariate polynomials (`Polynomial`)**: Native canonical polynomial normal forms inside the AST for fast like-term reductions and coefficient quotients.
- **Associative & commutative pattern rewriting**: Commutative subset matching for sum and product groups with transactional backtracking, bounded search limits (`match_limit`), and caller guards (`RewriteGuard`).
- **Exact & representation-preserving numerics**: Built on `NumAnaFis` (`Number`), keeping exact integers, reduced rationals, bitwise-faithful IEEE floats, and scalar complex values.
- **Zero-overhead ownership & memory safety**: Scoped contexts (`Context`) and zero-sized `'owner` symbol lifetimes prevent dangling symbols, aliasing bugs, and memory leaks.
- **Zero external dependencies by default**, targeting `no_std` environments (requiring only `alloc`).

---

## Quick Start

Add `symb_anafis` to your `Cargo.toml`:

```toml
[dependencies]
symb_anafis = "0.9.0"
```

### Basic Symbolic Manipulation & Rewriting

```rust
use symb_anafis::{Context, Expr, ExprView, Pattern, RewriteRule, Substitution};

// 1. Thread-safe, scoped symbol creation
let ctx = Context::new();
let x = ctx.symb("x");
let y = ctx.symb("y");

// 2. Natural arithmetic on Copy symbols and Expr handles
let expr = x.pow(2) + x.sin() * y;
assert!(matches!(expr.view(), ExprView::Sum(_)));

// 3. Associative & commutative pattern rewriting
let u = Pattern::capture("u");
let sin_cos_rule = RewriteRule::new(
    u.sin().pow(2) + u.cos().pow(2),
    Pattern::exact(&Expr::number(1)),
).unwrap();

let target = x.sin().pow(2) + x.cos().pow(2) + y;
let simplified = sin_cos_rule.apply_all(&target, None, None).unwrap();
assert_eq!(simplified, Expr::number(1) + y);

// 4. Concrete expression substitutions
let sub = Substitution::new(x.sin(), y.to_expr());
let substituted = sub.apply_all(&expr);
assert_eq!(substituted, x.pow(2) + y * y);
```

---

## Architectural Highlights

### 1. Globally Interned Expression DAG (`Expr`)

Every expression is an interned, immutable Directed Acyclic Graph (DAG) node wrapped in a 1-pointer handle:
- **Null-Pointer Niche Optimization**: `Expr(Option<Arc<Node>>)` uses the null-pointer niche; size equals a single machine word (`usize`).
- **$O(1)$ Equality and Hashing**: Identical live subtrees share exactly one physical allocation. Structural equality and hashing reduce to instantaneous pointer comparisons.
- **Lock-Free Fast Paths & Weak Interning**: Storage uses sharded, collision-safe weak tables that automatically reclaim deallocated nodes without global tracing garbage collectors.

### 2. Associative & Commutative Term Rewriting

Rewriting expressions containing commutative and associative operations ($\sum$ and $\prod$) presents combinatorial challenges. `SymbAnaFis` provides:
- **Disjoint Subset Matching**: Root sum and product patterns automatically match subsets of operands without requiring permutation explosion; unmatched operands survive unmodified.
- **Transactional Backtracking**: Variable captures and trial expansions maintain an undo log, reverting state cleanly on failed branches.
- **Bounded Search Limits**: Rewriting accepts an optional `match_limit: Option<usize>` parameter. Reaching the candidate limit stops traversal immediately with `RewriteError::MatchLimitExceeded`, guaranteeing execution bounds on massive trees.
- **Caller Guards (`RewriteGuard`)**: Enforces domain conditions, sign requirements, or integrality constraints directly during match exploration.

### 3. Canonical Sparse Multivariate Polynomials

Polynomial expressions (such as $a x^2 y + b x + c$) are recognized and embedded as specialized AST variants (`Node::Poly`):
- **Sorted Monomial Order**: Monomial bases are uniquely ordered by structural base identity; exponents are stored as packed positive integers.
- **Shared Literal Coefficients**: Numeric coefficients retain shared literal handles without cloning payload values.
- **Fast Logical Projections**: Iteration and monomial visits (`map_logical`, `fold_monomials`) present a unified view across ordinary sums and polynomial representations.

### 4. Representation-Preserving Numeric Tiers

Through native integration with [`NumAnaFis`](https://github.com/CokieMiner/NumAnaFis), literal values retain their true algebraic tier:
- **Exact Stays Exact**: Integers and reduced rationals perform exact arithmetic without unwanted floating-point coercion.
- **Bitwise Float Fidelity**: Floating-point values preserve IEEE 754 bitwise identity, including signed zeros and distinct NaN payloads.
- **Complex Numbers**: Continuous branch cuts and scalar imaginary units ($i$) are preserved across algebraic operations.

### 5. Scoped Symbol Lifetimes & Contexts

Symbols avoid error-prone stringly-typed lookups and dangling handles:
- **`Symbol<'owner>`**: Represents a native-width `Copy` identifier bound by lifetime to its creator (`Context` or owning `Expr`).
- **Global & Local Separation**: Global handles (`Symbol<'static>`) provide permanent process-wide identities; local handles allow lightweight isolated evaluation contexts without name collision.
- **Sound Unbinding**: Unbinding a name preserves issued handles within their lifetime; rebinding creates a fresh unique identity.

---

## Cargo Features

All features are optional; the crate compiles in `no_std + alloc` by default:

| Feature | Description |
| --- | --- |
| `std` | Links the standard library, selects poison-recovering registry mutexes, and enables `NumAnaFis`'s `std` backend. |
| `num-traits` | Implements `num_traits` ecosystem traits (`Zero`, `One`, `Num`, etc.) for `Expr`. |

Without `std`, shared registries employ private non-poisoning spin mutexes suitable for bare-metal and embedded targets with atomic support.

---

## Ongoing Re-engineering (v0.9.0 Refactor)

The crate is undergoing an architectural overhaul transitioning from legacy dynamic AST trees to interned DAG nodes:
- **Active Core (`src/core/`)**: Production-ready interned expressions, polynomials, symbols, functions, associative rewriting, and concrete substitutions.
- **Diagnostics (`src/errors/`)**: Modularized, zero-allocation error diagnostics (`core.rs`, `diff.rs`, `parser.rs`).
- **Staged Reintegration (`old/`)**: Symbolic differentiation (`diff`), Pratt parser (`parser`), algebraic simplification (`simplification`), and bytecode virtual machine evaluation (`evaluator`) are archived under `old/` and being re-engineered under strict project guidelines.

---

## Documentation & Architecture

- [Project Guidelines](AGENTS.md): Architectural invariants, monotonic call graphs, safety contracts, and verification protocols.
- [NumAnaFis](https://github.com/CokieMiner/NumAnaFis): Numeric scalar core and special mathematical functions.
- [MpAnaFis](https://github.com/CokieMiner/MpAnaFis): High-performance arbitrary-precision arithmetic.

---

## Development & Verification

To verify the library locally:

```sh
cargo check --lib
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib --all-features
cargo test --doc --all-features
```

When audit tools are active:

```sh
python3 tools/structure_audit.py
python3 tools/import_audit.py
python3 tools/check_allows.py
```

---

## Citation

Academic citation:

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

---

## License

`SymbAnaFis` is licensed under the Apache License, Version 2.0.
See [LICENSE](LICENSE) for details.
