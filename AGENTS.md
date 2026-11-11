# SymbAnaFis contribution guide

SymbAnaFis (`symb_anafis`) is a symbolic mathematics library: exact algebraic
manipulation, term rewriting, differentiation, and numerical evaluation. Numbers
come from `NumAnaFis`. The legacy implementation under `old/` is a behavioral
reference; active subsystems live under `src/`.

## Priorities

In strict order:

1. **Mathematical correctness.** An unsound transformation is a critical bug; performance never excuses an unproved transform.
2. **Memory safety.** No undefined behavior; lifetimes prevent dangling symbols and data races.
3. **Portability.** Pointer-width-safe counts and capacities on 16-, 32-, and 64-bit targets; `no_std + alloc` by default.
4. **Efficiency.** Reduce mathematical work, traversals, allocations, and memory traffic before micro-optimizing.
5. **Ergonomic APIs.** Composable Rust interfaces with explicit errors.

## Core contracts

- **`Expr`** is one pointer, `ManuallyDrop<Arc<Stored>>`, to an immutable node and its cached property facts, interned in a sharded weak hash-consing table. Equal live subtrees share one node, so equality and hashing compare pointers. Dropping the last handle releases descendants iteratively.
- **Normal forms.** Constructors normalize: sums and products flatten, order, fold finite literals, and collect terms; `sqrt(x)` for exact `x^(1/2)`, `exp(x)` for `e^x`, unary logarithms for bases `e`, `2`, `10`. Known negative real powers become denominators. Normalization never cancels a symbolic factor or removes a domain exclusion without proof.
- **Literals.** `NumericValue` gives `Number` representation identity: `2 != 2.0` and `0.0 != -0.0`. Exact values stay exact and floats stay floats. Normalization, matched multiples, and rendering omit only the exact identities `0` and `1`: `1.0*x` and `x + 0.0` keep their approximate literal. Folding uses checked arithmetic: an exact result that exceeds the backend keeps its operation symbolic and is never wrapped or promoted to a float.
- **Compact polynomials** (`Node::Poly`) are private. Each distinct base is stored once in structural order, and terms name bases by position, so a polynomial holds one handle per base; its stored edges are the bases and then the coefficients. Only `Terms`, `Monomials`, and `Powers` in `polynomial.rs` resolve positions. Views, `map`, and `fold` present compact polynomials as ordinary sums and visit each logical monomial once. The logical terms, one canonical product per stored term in structural order, are built on first use and kept with the storage (`Polynomial::projected`), so later traversals borrow them; counting terms never builds them, and releasing the storage releases them iteratively.
- **`Symbol<'owner>`** is a `Copy` pointer to `Arc`-owned metadata, so identity, name, and expression conversion need no registry lookup. Global metadata is leaked once and indexed by identity for `Symbol::from_id`; local metadata is owned by its context's owner map and by expressions, and never enters a global index. Local symbols borrow a `Context` or `Expr` and cannot outlive it. Identities are never reused.
- **`FunctionName`** is a builtin enum or a shared immutable custom definition. Builtins dispatch statically without locks or lookups; arities are immutable after registration.
- **Patterns and rules** match commutative operand groups with backtracking and transactional undo. `match_limit` bounds candidate attempts for a whole pass; exhaustion returns `RewriteError::MatchLimitExceeded` and publishes nothing. Replaced destinations are not revisited in the same pass.
- **Substitutions** match original nodes by identity, apply entries simultaneously, and decompose exact powers only when principal branches and zero exclusions are preserved.
- **Properties** are conservative facts proved once per interned node from its children's facts and kept with the node. `Unknown` never authorizes a guarded transformation.
- **Settled stages.** A builtin simplification step's direct identities and its factoring read only the node and the domain contract, and an exhausted budget is an error, never a kept node. Each such map therefore records on every node it keeps one bit per step and contract (`Postorder::settled`), and later runs, in any sequence, skip those nodes with their descendants. The rule pass reads the builtins below each node and visits every node.
- **Derivatives** evaluate a `Postorder::compact` numbering once per variable on regular principal-branch domains: a gradient shares one numbering, and a Jacobian numbers all rows once as a `Forest`. Compact polynomials in scalar bases, alone or as the group of a sum, are differentiated from their stored terms, and a node whose children are constant is constant. Results equal those of the logical form. Unsupported cases keep derivative nodes, and no separate simplifier runs.

## Subsystems

- `src/collections.rs`: `no_std` facade over `hashbrown` and `foldhash`. `HashMap` and `HashSet` are the public types; internal scratch tables use the fixed-seed `FixedMap` and `FixedSet`, whose construction does no seeding. Add no dependency that the crate does not already compile.
- `src/core/`: `api.rs` the public `ExprView`; `logic/` type owners, construction, interning, registries, traversal, substitution, and the operand-matching kernels that substitution and rewrite rules share. Everything that reads the private `Node` lives here; other subsystems use `ExprView` and the crate-internal traits (`AdditiveOperands`, `CompactStorage`, `ExactIdentity`, `StoredForm`).
- `src/rewrite/`: `Pattern`, `RewriteRule`, `RuleSet`, and `Replacement`: template normalization, commutative matching, and rule passes.
- `src/diff/`: differentiation, gradients, Jacobians, Hessians, including the public `Expr` methods and the `Jacobian` trait.
- `src/polynomial/`: sparse exact polynomials over expression indeterminates: conversion to and from expressions, products, exact division, and certified GCDs.
- `src/parser/`: streaming UTF-8 lexer and iterative precedence parser with byte spans.
- `src/simplification/`: ordered simplification sequences and presets, and the policy of when, under which domain contract, and within which budget each identity or polynomial operation applies; `RULES.md` lists every rule and its domain mode; `Expr::compact`, `Expr::expand`, and `Expr::factor`.
- `src/evaluator/`: direct evaluation and compiled scalar register programs for `f64` and `Numeric` backends.
- `src/uncertainty/`: first-order propagation through gradient covariance contraction.
- `src/errors/`: one error owner per subsystem.
- `bindings/python/`, `python/symb_anafis/`: PyO3 extension and Python facade; the default Rust build does not depend on Python.

Core imports only `collections` and `errors`; a subsystem that adds public `Expr` methods implements them in its own module. Numerical function kernels belong to `NumAnaFis`; symbolic derivative formulas belong to `src/diff/`, except the derivative of compact storage (`CompactStorage::polynomial_derivative`), which builds storage from private fields, and the derivative-node constructor `SymbolData::derivative_of`; builtin identities, arities, and folding belong to `src/core/logic/`. The builtin method table in `core/logic/expression/functions.rs` is repeated for `Pattern` in `rewrite/pattern/constructors.rs`; `BuiltinFunction::is_method` and an exhaustive match fail the build when the copies differ.

## Code discipline

- **Flow.** Validate once, dispatch once, establish invariants, then run an infallible kernel. Internal kernels do not return unreachable `Option` or `Result`.
- **Invariants in types.** Prefer `NonZero*`, typed ranges, and exact-division kernels (`NonZeroU64Ext`) over runtime checks, so LLVM can remove redundant branches. Write code whose IR and assembly carry no redundant checks; measure only at the end.
- **Unsafe.** Allowed when it removes a check or copy that safe code cannot express, such as `get_unchecked` on an index proved by construction. Every `unsafe` block has an immediately preceding `// SAFETY:` proof. A file with unsafe code has one file-level `#![expect(unsafe_code, reason = "...")]`. Express invariants through structure and types (ownership of a per-node buffer, a value returned where it is known); do not state them to LLVM with `assert_unchecked` hints.
- **Arithmetic.** Counts, offsets, and capacities use checked arithmetic or a local bounds proof. Wrapping is only for proved modular reductions and hashing. Casts need range proofs that hold on 32- and 64-bit targets.
- **Lints.** Every `allow` or `expect` has a `reason` and the narrowest scope; prefer `expect`. No `allow(dead_code)` or `expect(dead_code)`.
- **Functions.** Do not add a function of a few lines without a reason such as an API boundary, a private field, or a single proof site; a little repetition is preferable.
- **Traversal.** Never recurse over an expression without memoization or an explicit stack. A traversal that computes one value per node uses `Postorder`: discovery costs one hash lookup per edge to a node with several handles, and children's values are then read by position. Builtin simplification, scoring, and differentiation use the compact numbering, which keeps compact storage in scalar bases whole and never builds its terms. Selection-driven substitution uses `SubstitutionPass`, whose frames receive one value per scheduled child; pattern DAGs are numbered by `PatternNode::template_order`.
- **Completeness.** No `todo!()`, `unimplemented!()`, or dead placeholder branches in production code.

## Modules and visibility

- `mod.rs` is a registry only: `//!` docs, inner attributes, `use super::{...}`, `mod ...;`, `pub use ...;`, and a final `#[cfg(test)] mod tests;`.
- Never use `pub mod`, `pub(crate)`, `pub(super)`, or `pub(in ...)`. Items shared across modules are plain `pub`; facades control external reach.
- Import through facades: `crate::subsystem::Type` across subsystems and `super::Thing` within one, never `super::sibling::Thing`. No wildcard imports.
- A type owner lives in its own file (`expression.rs`) with implementation children in `expression/`, under a reasoned `clippy::self_named_module_files` expectation. No `#[path]`.
- Import groups, separated by blank lines: `core`/`std`, `alloc`, external crates, `crate::`, `super::`, child modules.
- Keep production files focused, normally under 500 lines, without splitting only to meet a count.

## Source order and documentation

- Files follow execution order: module docs, attributes, imports, types; entry points; algorithm stages; helpers; leaf kernels.
- Documentation is direct, descriptive scientific prose about present behavior, domains, assumptions, and proofs. Mathematical identities use LaTeX (`$...$`). Cite primary sources where an algorithm needs one. No history, change logs, or commented-out code.
- Public items document purpose, domain, errors, panics, and preconditions.

## Tests

- Production files contain no test code. Each module has one `tests.rs` or `tests/` registry; tests that need private fields are descendants of the type owner.
- Core transformations need `proptest` coverage of mathematical equivalence, structural invariants, and ownership. Fixed tests cover boundaries and exact counterexamples.
- Each test file opens with a `//!` summary and lists helpers, then one `proptest!` block with the most general properties, then the fixed edge tests.

## Verification

```text
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --lib --no-default-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc --all-features
python3 tools/structure_audit.py
python3 tools/import_audit.py
python3 tools/check_allows.py --all-lints
```

## Scope

- Changes must improve correctness, performance, or guideline compliance; no unrelated churn.
- Change stable public APIs only with a mathematical or architectural reason.
- Edit Rust sources directly; no bulk regex rewrite scripts.
- Migrated subsystems meet every rule from their first commit.
