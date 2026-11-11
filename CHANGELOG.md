# Changelog

All notable changes to `symb_anafis` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## Index
- [0.9.0](#090---2026-11-10)
- [Historical Archive (v0.4.0 – v0.8.1)](docs/CHANGELOG_ARCHIVE.md)

---

## [0.9.0] - 2026-11-10

Version 0.9.0 is a ground-up redesign of `symb_anafis` focused on high performance, immutable DAG semantics, exact rational arithmetic via `NumAnaFis`, a single canonical owning-type API, and aligned Rust and Python bindings.

### Added
- **Immutable hash-consed expression DAG:** `Expr` nodes are interned with $O(1)$ cloning, equality, and hashing, canonical construction, and domain-preserving simplification (`DomainSafety`).
- **Exact numeric backend (`NumAnaFis`):** re-exports `Number`, `Real`, `Complex`, and `Numeric`. Integers and reduced rationals stay exact without silent float coercion; operations that exceed native numeric capacity remain symbolic.
- **Compiled evaluator, Native JIT, and WebAssembly export:**
  - `Program` and `MultiProgram` compile single- and multi-output expressions into register bytecode with reusable per-thread `Workspace`s, scalar/batch evaluation, SIMD dispatch (`std`), and parallel batch evaluation (`rayon`).
  - `with_native()` (`native_jit` feature) lowers compiled programs to host machine code on `x86_64`, `aarch64`, and `riscv64` (Linux, macOS, Windows).
  - `to_wasm_bytes()` exports standalone WebAssembly modules.
- **Pattern matching, rewriting, and assumptions:** `Pattern`, `Replacement`, `RewriteRule`, `Substitution`, `Substitutions`, and symbol `Properties` / `Property` / `Truth`.
- **Configurable simplification:** `Expr::compact`, `Expr::expand`, `Expr::factor`, and `SimplificationSequence` with custom steps, targets, termination policies, and domain safety.
- **Multivariate polynomial engine:** canonical sparse polynomials over exact coefficients with Johnson heap division and multi-prime NTT convolution via `NTTAnaFis`.
- **Zero-copy parser diagnostics:** `Context::parse` with byte-range `Span` error reporting (`ParseError`).
- **`no_std + alloc` default build:** core Rust crate builds without `std`; `std`, `native_jit`, `rayon`, and `num-traits` are opt-in features.
- **Typed Python bindings:** aligned owning-class API with `py.typed` and complete `_native.pyi` type stubs.

### Changed
- **Single canonical API:** replaced the overlapping string, free-function, and builder APIs (`diff`, `Diff`, `simplify`, `Simplify`, `evaluate_str`, etc.) with methods on owning types (`Context`, `Symbol`, `Expr`, `Program`, `MultiProgram`, `SimplificationSequence`, `Uncertainty`).
- **Scoped namespaces:** `Context` owns symbol and `UserFunction` registrations cleanly instead of relying on global mutable symbol tables.
- **Structured error types:** dedicated `ParseError`, `EvalError`, `SimplifyError`, `RewriteError`, `SubstitutionError`, `SymbolError`, `FunctionError`, and `UncertaintyError`.

### Removed / Breaking Changes
- **Cargo features:** renamed `parallel` to `rayon` (implies `std`), moved Python bindings out of the `python` feature on the root crate into the dedicated Python package, and added `std`, `native_jit`, and `num-traits`.
- **Legacy Rust exports:** removed `DiffError` (replaced by structured error types), `MathScalar`, `ToParamName`, `CovEntry`, `Dual`, `CompiledEvaluator`, `eval_f64`, the `visitor` and `parallel` modules, string-based free functions (`diff`, `simplify`, `parse`, `evaluate_str`, `gradient_str`, `hessian_str`, `jacobian_str`), legacy builders (`Diff`, `Simplify`), and global symbol-table helpers (`symb`, `symb_new`, `symb_get`, `clear_symbols`, etc.).
- **Uncertainty API:** removed `uncertainty_propagation` and `relative_uncertainty` free functions in favor of `Uncertainty` and `CovarianceMatrix`.
- **Python exports:** removed top-level `evaluate_parallel`, `eval_f64`, `Dual`, `Diff`, `Simplify`, `CompiledEvaluator`, and string/global-symbol free functions in favor of the owning-class API (`Context`, `Expr`, `Evaluator`, `SimplificationSequence`, `Uncertainty`).

### Migrating from 0.8.x to 0.9.0

#### Rust

| 0.8.x | 0.9.0 |
|---|---|
| Feature `parallel` | Feature `rayon` |
| `DiffError` | Specific error types (`ParseError`, `EvalError`, `SimplifyError`, etc.) |
| `symb("x")` | `let ctx = Context::new()?; let x = ctx.symb("x")?;` (or `Symbol::intern("x")?`) |
| `parse("x^2 + sin(x)", ...)` | `ctx.parse("x^2 + sin(x)")?` |
| `diff("x^2", "x", &[], None)` / `Diff::new().differentiate(&e, &x)` | `e.derivative(x, 1)` (or `Expr::derivative(e, x, 1)`) |
| `gradient(&f, &[&x, &y])` / `hessian(&f, ...)` | `f.gradient(&[x, y])` / `f.hessian(&[x, y])` |
| `jacobian(&[&f1, &f2], &[&x, &y])` | `[f1, f2].jacobian(&[x, y])` (via `Jacobian` trait) |
| `simplify(...)` / `Simplify::new().simplify(&e)` | `e.compact()?`, `e.expand()?`, `e.factor()?`, or `SimplificationSequence` |
| `CompiledEvaluator::compile(&e, &[&x], None)` | `Program::compile(&e, &[x])?` (or `MultiProgram::compile(&exprs, &[x])?`) |
| `evaluator.evaluate(&[0.5])` | `program.eval(&[0.5])` / `program.with_native()?.eval(&[0.5])` |
| `uncertainty_propagation(...)` | `Uncertainty::propagate(...)` with `CovarianceMatrix` |

#### Python

| 0.8.x | 0.9.0 |
|---|---|
| `symb_anafis.symb("x")` / `symb_anafis.parse("x^2")` | `ctx = Context(); x = ctx.symb("x"); expr = ctx.parse("x^2")` |
| `symb_anafis.diff("x^2", "x")` / `Diff().differentiate(e, x)` | `expr.derivative(x)` |
| `gradient(f, [x, y])` / `hessian(f, [x, y])` | `f.gradient([x, y])` / `f.hessian([x, y])` |
| `simplify("sin(x)^2 + cos(x)^2")` / `Simplify().simplify(e)` | `expr.compact()` / `expr.expand()` / `expr.factor()` / `SimplificationSequence` |
| `CompiledEvaluator.compile(e, [x])` | `ev = Evaluator(expr, [x])` (supports single or multi-output, `.with_native()`, `.to_wasm_bytes()`) |
| `evaluate_parallel(...)` / `eval_f64(...)` | `ev.evaluate_batch(inputs, parallel=True)` |
| `uncertainty_propagation(...)` | `Uncertainty.propagate(...)` with `CovarianceMatrix` |

---

## Older Releases

For releases prior to version 0.9.0 (v0.4.0 through v0.8.1), please refer to the [Changelog Archive](docs/CHANGELOG_ARCHIVE.md).

---

[0.9.0]: https://github.com/CokieMiner/SymbAnaFis/releases/tag/v0.9.0
