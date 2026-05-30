# Deep API Audit Findings — SymbAnaFis

Date: 2026-04-28
Auditor: Kilo (automated review)
Scope: api.rs exposure, lib.rs re-exports, logic/ encapsulation, API_REFERENCE.md coverage, Python bindings

---

## 1. Executive Summary

The project follows an `api.rs` + `logic/` separation pattern across all modules. The **outer firewall** (`mod logic;` private) consistently prevents logic items from being accessible outside each module. However, there are significant issues in three areas:

1. **Internal leaks through `api.rs`**: Several api.rs files re-export crate-internal items as `pub` that should be `pub(crate)` or `pub(super)`.
2. **API_REFERENCE.md gaps**: ~50+ public API items are undocumented, 8 incorrect entries exist, and 18 documentation sections are missing.
3. **Python binding bugs**: Name mismatches in `__init__.py` will cause import errors at runtime.

---

## 2. api.rs Exposure Audit

### 2.1 Clean api.rs Files (No Issues)

| File | Assessment |
|------|-----------|
| `src/core/context/api.rs` | Clean — only `Context`, `UserFunction`, `BodyFn`, `PartialFn` |
| `src/parser/api.rs` | Exemplary — single `pub fn parse`, logic imported privately |
| `src/convenience/api.rs` | Exemplary — all logic imported privately with renamed aliases |
| `src/uncertainty/api.rs` | Clean — all items are user-facing, logic imported privately |
| `src/bindings/python/api.rs` | Clean — PyO3 module registration only |
| `crates/num-anafis/src/number/api.rs` | Perfect — only `Scalar` and `Number` |
| `crates/num-anafis/src/number/logic/float_ops/api.rs` | Well-designed — all functions `pub(in crate::number)` |
| `crates/num-anafis/src/number/logic/rational_math/api.rs` | Well-designed — all functions `pub(in crate::number)` |
| `crates/num-anafis/src/number/logic/int_math/api.rs` | Well-designed — all functions `pub(in crate::number)` |

### 2.2 api.rs Files With Leaks

#### CRITICAL: `src/evaluator/api.rs`

**Leaked items (VM implementation details exposed as `pub`):**

| Item | Kind | Why It Shouldn't Be Public |
|------|------|---------------------------|
| `FnOp` | enum | Bytecode opcode — pure VM implementation detail |
| `Instruction` | struct | Bytecode instruction — pure VM implementation detail |
| `VirGenerator` | struct | Compiler internal type |
| `assemble_flat_bytecode` | fn | Internal function to flatten bytecode |
| `expand_user_functions` | fn | Internal compilation step |
| `evaluate_parallel_with_hint` | fn | Python-specific variant leaked through `pub use` |

**Status**: These are NOT re-exported in `lib.rs` at the crate root, but ARE accessible as `symb_anafis::evaluator::FnOp` since `evaluator` module uses `pub use api::*`. External users can reach them through the module path.

**Recommendation**: Change all these to `pub(crate)` visibility or move them to a separate `pub(crate)` module.

#### HIGH: `src/core/expr/api.rs`

| Item | Kind | Issue |
|------|------|-------|
| `compute_expr_hash` | fn | Internal hash computation, not user-facing |
| `compute_term_hash` | fn | Internal hash computation, not user-facing |
| `next_id()` | fn | Exposes internal ID generation mechanism |
| `CustomEvalMap` | type alias | Only used internally by evaluator |
| `Polynomial` | struct | Borderline — needed for `ExprKind::Poly` but reveals internal representation |

#### HIGH: `src/core/symbol/api.rs`

| Item | Kind | Issue |
|------|------|-------|
| `InternedSymbol` | struct | Internal slotmap handle, not user-facing |
| `key_from_id` | fn | Crate-internal registry function |
| `lookup_by_id` | fn | Crate-internal registry function |
| `symb_interned` | fn | Crate-internal registry function |
| `symb_new_isolated` | fn | Crate-internal registry function |
| `Symbol::key()` | method | Returns `DefaultKey`, leaks slotmap implementation type |

#### MEDIUM: `src/core/helpers/api.rs`

| Item | Kind | Issue |
|------|------|-------|
| `known_symbols` | module | Re-exports entire `known_symbols` submodule (internal symbol ID constants) |
| `traits` | module | Re-exports entire `traits` submodule (only `MathScalar` is user-facing) |

#### MEDIUM: `src/math/api.rs`

| Item | Kind | Issue |
|------|------|-------|
| 22 `eval_*`/`bessel_*` functions | fns | Comment says "crate-internal" but they are `pub`, not `pub(crate)`. Not re-exported in `lib.rs` but accessible as `symb_anafis::math::eval_gamma` etc. |

#### LOW: `src/simplification/api.rs`

| Item | Kind | Issue |
|------|------|-------|
| `simplify_expr()` | fn | Low-level function requiring internal symbol IDs, should be `pub(crate)` |
| `CustomBodyMap` | type alias | Internal type alias `HashMap<u64, BodyFn>`, not user-facing |

#### LOW: `src/functions/api.rs`

| Item | Kind | Issue |
|------|------|-------|
| `Registry` | struct | May be internal — users interact with functions through `Context` |

---

## 3. lib.rs Re-export Audit

### 3.1 Correct Re-exports (Match api.rs Public Items)

The following items are correctly re-exported from `lib.rs`:

```rust
pub use core::{DiffError, Expr, Span, Symbol, SymbolError};
pub use core::MathScalar;
pub use math::Dual;
pub use core::{ArcExprExt, clear_symbols, remove_symbol, symb, symb_get, symb_new, symbol_count, symbol_exists, symbol_names};
pub use core::{Context, UserFunction};
pub use parser::parse;
pub use diff::{Diff, diff};
pub use simplification::{Simplify, simplify};
pub use convenience::{evaluate_str, gradient, gradient_str, hessian, hessian_str, jacobian, jacobian_str};
pub use uncertainty::{CovEntry, CovarianceMatrix, Uncertainty, relative_uncertainty, uncertainty_propagation};
pub use core::ExprView;
pub use evaluator::{EvaluatorBuilder, ToParamName, VarLookup, VmEvaluator};
#[cfg(feature = "parallel")]
pub use evaluator::eval_f64;
#[cfg(feature = "parallel")]
pub use evaluator::{EvalResult, ExprInput, SKIP, Value, VarInput, evaluate_parallel};
pub const EPSILON: f64 = 1e-14;
```

### 3.2 Items in api.rs But NOT in lib.rs (Module-Path Accessible)

These items are `pub` within their module but not re-exported at the crate root. They are accessible via module paths like `symb_anafis::evaluator::FnOp`:

| Module Path | Item | Should Be |
|-------------|------|-----------|
| `evaluator::FnOp` | Bytecode opcode | `pub(crate)` |
| `evaluator::Instruction` | Bytecode instruction | `pub(crate)` |
| `evaluator::VirGenerator` | Compiler internal | `pub(crate)` |
| `evaluator::assemble_flat_bytecode` | Internal function | `pub(crate)` |
| `evaluator::expand_user_functions` | Internal function | `pub(crate)` |
| `evaluator::evaluate_parallel_with_hint` | Python-internal | `pub(crate)` or feature-gated |
| `math::eval_*` (22 functions) | Numerical eval | `pub(crate)` or documented as public |
| `functions::Registry` | Function registry | `pub(crate)` |
| `simplification::CustomBodyMap` | Type alias | `pub(crate)` |
| `simplification::simplify_expr` | Low-level fn | `pub(crate)` |
| `core::compute_expr_hash` | Hash function | `pub(crate)` |
| `core::compute_term_hash` | Hash function | `pub(crate)` |
| `core::InternedSymbol` | Slotmap handle | `pub(crate)` |
| `core::key_from_id` | Registry fn | `pub(crate)` |
| `core::known_symbols` | Module | `pub(crate)` |

### 3.3 Missing from lib.rs (Should Be Re-exported)

| Item | Currently Accessible As | Issue |
|------|------------------------|-------|
| `BodyFn` | `core::context::BodyFn` | Not at crate root; needed for custom function definitions |
| `PartialFn` | `core::context::PartialFn` | Not at crate root; needed for custom function definitions |

---

## 4. logic/ Encapsulation Audit

### 4.1 Exemplary Modules (Perfect Encapsulation)

| Module | Pattern Used |
|--------|-------------|
| `parser/logic/` | All sub-modules `mod` (private), all re-exports `pub(super)` |
| `diff/logic/` | Sub-modules `pub(super) mod`, re-exports `pub(super) use` |
| `simplification/logic/` | Same as diff — perfect |
| `uncertainty/logic/` | Same — perfect |
| `convenience/logic/` | Same — perfect |
| `num-anafis/number/logic/` | Uses `pub(in crate::number)` — gold standard |

### 4.2 Modules With Weak Defense-in-Depth

| Module | Issue | Severity |
|--------|-------|----------|
| `core/helpers/logic/mod.rs` | Uses `pub mod` for all 4 sub-modules (`error`, `known_symbols`, `traits`, `view`) instead of `pub(super) mod` | **HIGH** |
| `core/context/logic/mod.rs` | Uses `pub mod context;` instead of `pub(super) mod context;` | **MEDIUM** |
| `evaluator/logic/bytecode/mod.rs` | Deep chain of `pub mod` in `bytecode/`, `compile/`, `execute/` | **MEDIUM** |
| `core/expr/logic/mod.rs` | Staircase re-exports use bare `pub` instead of `pub(super)` | **MEDIUM** |
| `core/symbol/logic/mod.rs` | Same staircase `pub` issue | **MEDIUM** |
| `math/logic/mod.rs` | `pub use functions::*;` and `pub use number_types::*;` — wildcard re-exports | **MEDIUM** |
| `functions/logic/mod.rs` | `pub use registry::Registry;` instead of `pub(super) use` | **LOW** |

**Note**: All of these are currently safe because the parent `mod.rs` declares `mod logic;` (private), creating an outer firewall. The concern is defense-in-depth — if any `mod logic;` were changed to `pub(crate) mod logic;`, these weak inner boundaries would leak implementation details.

---

## 5. API_REFERENCE.md Coverage Audit

### 5.1 Completely Undocumented Public Items

| Item | Kind | Impact |
|------|------|--------|
| `Uncertainty` | struct (builder) | Entire builder pattern for uncertainty is missing |
| `Span` | struct | Error location tracking not documented |
| `SymbolError` | enum | Symbol error handling not documented |
| `BodyFn` | type alias | Custom function signature not documented |
| `PartialFn` | type alias | Partial derivative signature not documented |
| `EPSILON` | const f64 | Tolerance constant not documented |
| `MathScalar` | trait | Numeric trait not documented |
| `ToParamName` | trait | Parameter conversion trait not formally documented |
| `VarLookup` | trait | Variable lookup trait not formally documented |
| `Expr::new()`, `Expr::into_kind()`, etc. | methods | ~20 Expr methods undocumented |
| `Symbol::from_id()`, `Symbol::key()`, etc. | methods | ~4 Symbol methods undocumented |
| `VmEvaluator::builder()`, `param_names()`, etc. | methods | 6 VmEvaluator methods undocumented |
| `Simplify::user_fn()`, `max_depth()`, etc. | methods | 5 Simplify methods undocumented |
| `Diff::fixed_vars()` | method | 1 Diff method undocumented |
| `CovEntry::to_expr()`, `is_zero()`, `From` impls | methods | 4 CovEntry items undocumented |
| `CovarianceMatrix::diagonal_symbolic()`, `get()`, `dim()` | methods | 3 methods undocumented |
| `Context` (12+ methods) | methods | Half of Context methods undocumented |
| `UserFunction` (6 methods) | methods | Not in a formal API table |
| `ExprView` (4 variants, 6 helper methods) | enum | Partially documented, many items missing |
| `EvaluatorBuilder` methods | methods | No formal methods table |
| `DiffError` constructors | methods | Not documented |
| `DiffError` `#[non_exhaustive]` | attribute | Critical — users must use wildcard match arm |

### 5.2 Incorrect Documentation

| Location | Issue | Correct Value |
|----------|-------|---------------|
| Line 397-399 | `CovarianceMatrix::diagonal()` shown returning `Result` with `?` | Returns `Self` (no Result). Only `CovarianceMatrix::new()` returns `Result` |
| Lines 869-910 | `eval_parallel!` macro syntax shown | This macro doesn't exist. Only `evaluate_parallel` function exists |
| Lines 900-901 | `use symb_anafis::parallel::SKIP` | Should be `use symb_anafis::SKIP` (no `parallel` submodule) |
| Lines 928-935 | `symb_anafis::parallel::{evaluate_parallel, ...}` | Should be `symb_anafis::{evaluate_parallel, ...}` |
| Line 257 | "Simplify supports maximum iterations" | `Simplify` builder has no `max_iterations` method |
| Line 1316 | "All functions return `Result<T, DiffError>`" | Oversimplified — `gradient` returns `Vec<Expr>`, many methods don't return Result |
| Line 1322 | Error handling example without wildcard arm | `DiffError` is `#[non_exhaustive]` — requires `_` arm |

### 5.3 Documented Items NOT in Public API

| Item | Section | Issue |
|------|---------|-------|
| `eval_parallel!` macro | Parallel Evaluation | Does not exist in Rust API |
| `FunctionContext` (Python) | Custom Functions | Python-only, not Rust API |
| `count_nodes` | Expression Introspection | Python-only helper |
| `collect_variables` | Expression Introspection | Python-only helper |
| `symb_anafis::parallel::*` | Module paths | No `parallel` submodule in crate root |

### 5.4 Missing Documentation Sections

1. `Uncertainty` Builder reference
2. `Span` struct reference
3. `SymbolError` enum reference
4. `BodyFn` / `PartialFn` type aliases
5. `EPSILON` constant
6. `MathScalar` trait reference
7. `Expr` Constructors reference (full list)
8. `Expr` Analysis Methods reference
9. `Expr` Accessors reference
10. `Symbol` Full API reference
11. `ExprView` Full Reference (all variants + methods)
12. `VmEvaluator` Full Methods reference
13. `EvaluatorBuilder` Methods Table
14. `VarLookup` Trait Reference
15. `ToParamName` Trait Reference
16. Feature Flags formal section
17. Thread Safety section (Send + Sync guarantees)
18. `#[non_exhaustive]` notice for `DiffError`

---

## 6. Python Bindings Audit

### 6.1 Architecture Check

**PASS** — All Python binding imports go through module-level `api.rs` files. Zero direct imports from `logic/` modules were found.

### 6.2 Critical Python Bugs

| # | Severity | File | Issue |
|---|----------|------|-------|
| 1 | **HIGH** | `python/symb_anafis/__init__.py:34` | Import of `CompiledEvaluator` will fail — Rust registers the name `VmEvaluator` |
| 2 | **MEDIUM** | `python/symb_anafis/__init__.pyi:1100` | Type stub defines `CompiledEvaluator` but Rust exposes `VmEvaluator` |
| 3 | **MEDIUM** | `python/symb_anafis/__init__.py:82` | `__all__` lists `"PyExprView"` but the actual class name is `ExprView` |
| 4 | **LOW** | `python/symb_anafis/__init__.py` | `EvaluatorBuilder` class not imported/exported |
| 5 | **LOW** | `python/symb_anafis/__init__.py` | `FunctionContext` class not imported/exported |
| 6 | **LOW** | `python/symb_anafis/__init__.pyi` | `VmEvaluator.disassemble()` method missing from type stubs |

### 6.3 Python API Gaps (Not Bugs, But Limitations)

| Gap | Impact |
|-----|--------|
| `CovarianceMatrix` not exposed | Python users cannot specify correlated (off-diagonal) uncertainties |
| `Uncertainty` builder not exposed | Python users cannot pass `Context` to uncertainty operations |
| `EPSILON` not exposed | Minor — users rarely need this |

---

## 7. Remediation Priority

### Priority 1 — Fix Now (Breaking Bugs)

1. Fix `CompiledEvaluator` → `VmEvaluator` name mismatch in `python/symb_anafis/__init__.py`
2. Fix `PyExprView` → `ExprView` in `__all__` list
3. Fix `CompiledEvaluator` → `VmEvaluator` in type stubs

### Priority 2 — High (API Leaks)

4. Change `evaluator/api.rs` leaked items (`FnOp`, `Instruction`, `VirGenerator`, `assemble_flat_bytecode`, `expand_user_functions`, `evaluate_parallel_with_hint`) to `pub(crate)`
5. Change `core/expr/api.rs` leaked items (`compute_expr_hash`, `compute_term_hash`, `next_id`, `CustomEvalMap`) to `pub(crate)` or `pub(super)`
6. Change `core/symbol/api.rs` leaked items (`InternedSymbol`, `key_from_id`, `lookup_by_id`, `symb_interned`, `symb_new_isolated`) to `pub(crate)`
7. Change `core/helpers/api.rs` — stop re-exporting `known_symbols` and `traits` modules publicly; use `pub(crate)` instead
8. Change `math/api.rs` 22 `eval_*` functions to `pub(crate)` or document them as intentional public API

### Priority 3 — Medium (Defense-in-Depth)

9. Change `core/helpers/logic/mod.rs` `pub mod` to `pub(super) mod` for all 4 sub-modules
10. Change `core/context/logic/mod.rs` `pub mod context;` to `pub(super) mod context;`
11. Change all staircase `pub use` in `logic/mod.rs` files to `pub(super) use`
12. Change `math/logic/mod.rs` wildcard `pub use` to explicit `pub(super) use` of only needed items
13. Change `evaluator/logic/bytecode/mod.rs` deep `pub mod` chain to `pub(super) mod`

### Priority 4 — Medium (Documentation)

14. Add `Uncertainty` builder documentation section
15. Add `Span`, `SymbolError` documentation
16. Fix incorrect `CovarianceMatrix::diagonal()` Result documentation
17. Remove or correct `eval_parallel!` macro documentation
18. Fix `symb_anafis::parallel::*` module paths (should be crate root)
19. Add `#[non_exhaustive]` notice for `DiffError`
20. Add missing `Simplify` builder methods documentation
21. Add missing `VmEvaluator` methods documentation
22. Add `ExprView` full variant and method documentation
23. Add `Expr` constructors and analysis methods documentation
24. Fix "maximum iterations" claim for `Simplify`

### Priority 5 — Low (Nice to Have)

25. Add `EvaluatorBuilder` and `FunctionContext` to Python `__init__.py`
26. Add `VmEvaluator.disassemble()` to Python type stubs
27. Expose `CovarianceMatrix` to Python for correlated uncertainties
28. Add `EPSILON` to Python module
29. Add `BodyFn`/`PartialFn` re-export to `lib.rs` crate root
30. Document thread safety (Send + Sync) guarantees
31. Document `MathScalar`, `ToParamName`, `VarLookup` traits formally

---

## 8. Architecture Pattern Comparison

The **num-anafis** crate demonstrates the correct pattern:

```rust
// num-anafis: logic/mod.rs
pub(in crate::number) mod float_ops;    // ✅ Restricted to number module
pub(in crate::number) mod int_math;     // ✅ Restricted to number module
pub(in crate::number) mod rational_math; // ✅ Restricted to number module
```

The **main crate** should adopt the same pattern:

```rust
// Current (weak):
pub mod error;          // ❌ Accessible if logic becomes pub
pub use registry::Registry; // ❌ Unrestricted staircase

// Target (strong):
pub(super) mod error;   // ✅ Restricted to parent module
pub(super) use registry::Registry; // ✅ Restricted staircase
```

---

## 9. Module-by-Module Verdict

| Module | api.rs | logic/ Encapsulation | lib.rs Re-export | Docs Coverage |
|--------|--------|---------------------|-----------------|---------------|
| `core/expr` | Issues (hash fns leaked) | Conditional Pass | Pass | Partial |
| `core/symbol` | Issues (InternedSymbol leaked) | Conditional Pass | Pass | Good |
| `core/helpers` | Issues (modules leaked) | HIGH concern | Pass | Partial |
| `core/context` | Clean | Conditional Pass | Pass | Good |
| `parser` | Exemplary | Exemplary | Pass | Good |
| `diff` | Clean | Exemplary | Pass | Good |
| `simplification` | Issues (simplify_expr) | Exemplary | Pass | Partial |
| `evaluator` | CRITICAL (6 leaked items) | Conditional Pass | Pass | Partial |
| `functions` | Minor (Registry) | Conditional Pass | Pass | N/A |
| `math` | Issues (22 fns should be pub(crate)) | Conditional Pass | Pass | Missing |
| `uncertainty` | Clean | Exemplary | Pass | Partial |
| `convenience` | Exemplary | Exemplary | Pass | Good |
| `bindings/python` | Clean | N/A | N/A | N/A |
| `num-anafis/number` | Perfect | Exemplary | Pass | N/A |
