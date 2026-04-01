# Changelog

All notable changes to symb_anafis will be documented in this file.

## [working]

### Working Update - 2026-04-01

- Added a new workspace member crate, num-anafis, as the numeric core for SymbAnaFis.
- Added backend feature routing in workspace Cargo.toml:
  - backend32
  - backend64
  - backend_big_astro
  - backend_big_rug
- Added public ergonomic helpers in num-anafis:
  - s(value) for scalar conversion
  - r(num, den) for rational construction with generic integer inputs
  - imported Clifford constructors I, J, EPS, E1, E2, E3
- Added full CliffordNumber API and removed Multivector naming from public/source usage.
- Added deep capabilities showcase example:
  - crates/num-anafis/examples/examples.rs
- Added strict lint policy for num-anafis and fixed clippy issues across backend combinations.
- Added import audit tooling for num-anafis and updated organization guidance for import style.

### Breaking Changes

- **Builder API Renames**:
  - `Diff::with_context()` has been renamed to `Diff::context()`.
  - `Simplify::with_context()` has been renamed to `Simplify::context()`.
- **Python Bindings**:
  - Similar method renames in Python builders (e.g., `.with_context()` -> `.context()`).
- **Visitor API Removal**:
  - The `symb_anafis::visitor` module has been removed (its functionality is superseded by the new `ExprView` API).
  - The `ExprVisitor` trait and `walk_expr` function have been removed.
  - Users should use `ExprView` API at the root/core level for pattern-matchable expression traversal.

- **Batch Evaluation**:
  - Batch and SIMD-based evaluation are now gated behind the `parallel` feature flag.


### Added

- **Architectural Refactor**: Complete reorganization of the codebase into a strict tiered structure to improve modularity and maintainability.
  - New hierarchy: `core/`, `parser/`, `simplification/`, `diff/`, `evaluator/`, `uncertainty/`, `functions/`, `math/`, and `convenience/`.
  - Each major component now follows a clear separation between `api` (public traits/wrappers) and `logic` (internal implementation).
  - **`src/convenience`**: New module for high-level logic (calculus, evaluation helpers).
  - **`src/core/context`** and **`src/core/expr`**: Modular core structures with separate logic and test files.
  - **Submodule Organization**: Re-organized `src/evaluator/logic/execute/` flat files into structured `engine/`, `drivers/`, and `tree/` subdirectories for better modularity.
  - **Constant Consolidation**: Moved global crate constants (`DEFAULT_MAX_DEPTH`, `DEFAULT_MAX_NODES`, and `EPSILON`) into `src/lib.rs` and refactored dependent imports to use the crate root directly.
  - **Encapsulate Boundaries**: Moved `ExprKind` out of the root level back into `crate::core::ExprKind` to enforce strict architectural separation between core items and high-level APIs.
- **Enhanced Evaluator Pipeline**:
  - Division of compilation into discrete stages: `expand` → `reg_alloc` → `optimize`.
  - New optimization passes: `dce` (Dead Code Elimination), `fusion` (Instruction Fusion), `strength_reduction`, and `power_chain` optimization.
  - Introduction of **`vir` (Virtual Intermediate Representation)** for evaluator-specific optimizations before final bytecode emission.
- **Advanced Math Support**:
  - Expansion of `src/math` with dedicated logic for `bessel`, `beta`, `erf`, `gamma`, `lambert_w`, and `zeta` functions.
  - Added support for `polar` and `polygamma` coordinates.
- **`Expr::variables_ordered()`**: New method to collect all variable names in order of first appearance (pre-order traversal).
- **Improved `erfc` accuracy**: Implemented `eval_erfc` using continued fractions for large arguments ($x \ge 1.5$) to maintain ~15 digits of relative precision and avoid catastrophic cancellation.
- **Flamegraph Profiling**: Added dedicated benchmarking and profiling examples (`flamegraph_benchmarks.rs`, `flamegraph_compile_eval.rs`, `flamegraph_profile.rs`).
- **Paradigm-Oriented Architectural Refactor**: Reorganized `src/evaluator/logic/` by execution paradigm:
  - `bytecode/`: Master-stream consolidating `compile/` and `execute/` with Virtual IR hooks.
  - `tree/`: Dedicated module layer isolating tree evaluator interpreters.
- **Granular Dispatcher Pipeline**: Decomposed the large monolithic `codegen/lower.rs` file into a modular suite (`sum.rs`, `product.rs`, `pow.rs`, etc.) to enforce declarative maintenance.
- **Strict Architectural Boundaries (Staircase Rule)**: Enforced a project-wide tiered import structure. Eliminated all self-referential `crate::` imports and deep relative imports (e.g., `super::super::`) in favor of single-level `super::` imports through intermediate re-exports in `mod.rs` files.


### Changed

- **Internal Directory Structure**: Massive movement of logic files into `logic/` subdirectories across all modules.
- **Unified `core/context` State**: Re-engineered `Context` mapping to share internal state via a single unified `Arc<RwLock<ContextInner>>`, making `Context::clone()` an $O(1)$ operation.
- **Decomposed monolithic `core/expr/constructors.rs`**: Segmented heavy AST core generators into thematic isolated files (`binary.rs`, `nary.rs`, `functions.rs`).
- **Python Bindings**: Updated to the new tiered architecture, with logic moved to `src/bindings/python`.
- **Benchmark & Example Organization**:
  - Python examples moved to `examples/python/`.
  - Heavy benchmarks moved to `examples/benchmarks/`.
- **Register-based Evaluator**: Major architectural refactor of the bytecode evaluator from a stack-based to a register-based virtual machine.
  - Merged the CSE cache into the general register file (removed `cache_size`).
  - Updated metadata: `stack_size` renamed to `register_count`.
- **Compiler Register Allocation**: The bytecode compiler now performs a single-pass register allocation with precise live-range analysis to minimize register usage and improve cache locality.
- **Mathematical Domain Errors**: Refactored special functions to return `Some(NaN)` instead of `None` on domain errors (e.g., negative arguments for logarithms or Bessel Y/K). This ensures IEEE-754 style propagation through arithmetic rather than aborting evaluation.
- **Mathematical Poles**: Functions with known poles (`gamma`, `digamma`, `trigamma`, `tetragamma`, `zeta`) now return `Some(±infinity)` with the correct directional sign instead of `None` when evaluated exactly at the pole.
- **Dual Number Arithmetic**: Automatic differentiation (`Dual` numbers) now propagates `NaN` values for derivatives at domain boundaries instead of short-circuiting with `None`.
- **Strict Encapsulation Boundary**: Tightened high-level backend modularity from fully public to internal crate visibility (`pub(crate) mod logic;` inside `evaluator/mod.rs`).
- **Inlining Traversal Optimization**: Added `#[inline]` guidelines on `vir/node.rs` micro patterns to minimize function setups on massive trees.

### Fixed

- **`acot` compiler/interpreter formula mismatch**: Compiler previously emitted `Recip → Atan` for `acot(x)`, always computing `atan(1/x)`. The interpreter uses `atan(1/x) + π` for x < 0 to produce range (0, π). Added a dedicated `Acot` bytecode instruction with the correct branch and consolidated the implementation in `stack::eval_acot` (alongside `eval_sinc`). Fixes scalar mismatch for expressions like `zeta(acot(-2.128...))`.
- **SIMD `exp`/`ln` swallowing NaN**: `wide::f64x4::exp()` and `wide::f64x4::ln()` use polynomial approximations that do not propagate NaN (e.g. `exp(NaN) → 0`, `ln(0) → NaN` instead of `−∞`). Changed `Exp`, `ExpNeg`, `ExpSqr`, `ExpSqrNeg`, and `Ln` SIMD fast-paths to per-lane scalar calls (`f64::exp` / `f64::ln`) which correctly follow IEEE 754. This fixes SIMD batch mismatches involving NaN-producing sub-expressions.
- **`hermite(n, NaN)` ignoring NaN argument**: All four `Hermite` instruction paths (inline/heap scalar, SIMD packed, SIMD scalar fallback) only guarded against `n.is_nan()` but not `x.is_nan()`. Added `|| x.is_nan()` check so `hermite(0, NaN) → NaN` instead of `1`.
- **Product evaluator swallowing NaN (`0 * NaN → 0`)**: The tree-walking interpreter had an early-exit optimization that returned `0.0` immediately upon seeing a zero numeric factor, skipping evaluation of remaining factors. This caused `signum(…) * undefined_expr → 0` instead of NaN when the second factor produced NaN. Removed the early exit to be IEEE 754 compliant: all factors are now always evaluated and multiplied.
- **CSE collision correctness**: `cse_cache` in the compiler changed from `FxHashMap<u64, (Expr, usize)>` to `FxHashMap<u64, Vec<(Expr, usize)>>`. Hash collisions previously silently overwrote the first entry, making the second expression uncacheable and potentially emitting a wrong `LoadCached` slot. Now all colliding entries are stored and searched by structural equality.
- **Product display truncation**: Products with more than 2 factors now print all factors; previously only the first two were displayed, silently dropping the rest.
- **cos(x) factor dropped during simplification**: Fixed a regression where `(-(x) + x*x) * -(cos(x))` lost the `cos(x)` factor after simplification. Added regression test `test_regression_cos_factor_not_dropped`.

### Code Quality

- **Lint Configuration Hardening**: Upgraded several static analysis lints in `Cargo.toml` to `deny` (e.g., `suspicious_xor_used_as_pow`, `try_err`, `unseparated_literal_suffix`) to enforce strict safety and clean code standards.
- **Reorganized `core/logic` gadgets**: Migrated unclassified internal tools into a standardized `core/helpers` space with bounded `api_user.rs` and `api_crate.rs` gates following architecture compliance.
- **Staircase Import Normalization**: Rescoped self-referencing absolute addresses across `core/`, `evaluator/`, and `simplification/` to relative `super::` step-ups.
- **Redundant Visibility Cleanup**: Optimized internal module and re-export visibility from `pub(crate)` to `pub` in restricted scopes to satisfy Clippy's `redundant-pub-crate` lint while maintaining boundary isolation.


- **Deleted legacy `stack.rs`**: Removed legacy stack-based primitive operations following the transition to the register-based evaluator.
- **Added `eval_math.rs`**: Centralized evaluator-specific mathematical helper functions (e.g., `acot`, `sinc`, `erfc`) to improve modularity.
- **Consolidated `eval_acot`**: Moved `eval_acot` from duplicated definitions in `execution.rs` and `simd.rs` into `eval_math.rs` (formerly `stack.rs`), co-located with `eval_sinc`. All call sites updated.
- **Visitor Silence**: Silenced deep-tree warnings in release builds to reduce library noise. Traversal remains truncated for safety.
- **`push_children` / `push_children_rev` helpers on `Expr`**: Factored out the repeated child-pushing match block shared by `node_count`, `contains_var_id`, `contains_var_str`, `has_free_variables`, `collect_variables`, and `fold`. Merged `FunctionCall`/`Sum`/`Product` arms into a single arm using the common `args` binding, reducing ~100 lines of duplication in `analysis.rs`.

### Performance

- **Reduced memory traffic**: Register-based evaluation eliminates stack pointer manipulation overhead and improves cache efficiency for complex expressions.
- **Enhanced Instruction Fusion**: Optimized peephole optimizer with new fusion patterns for `MulAdd`, `MulSub`, `NegMulAdd`, and `ExpNeg` instructions in the register-based evaluator.
- **Iterative Compilation**: The bytecode compiler now uses an iterative traversal for expression compilation, enabling the processing of extremely large Sum/Product chains (millions of terms) without risking host stack overflow.
- **Fused Load-Operate Instructions**: Added `AddParam`, `MulParam`, `SubParam`, `DivParam`, `AddCached`, and `MulCached` bytecode instructions to the compiled evaluator. This optimizes common evaluation patterns by executing operations directly against parameters and cached values, reducing stack `Push` and `Pop` overhead.
- **Compiler Constant Folding**: Added compile-time merging of consecutive `AddConst` and `MulConst` instructions. This reduces instruction count and stack manipulation overhead during batch evaluation.
- **Cached `Arc<Expr>` constants (`arc_number`)**: Added `arc_number(n)` helper and four static `LazyLock<Arc<Expr>>` statics for `0.0`, `1.0`, `-1.0`, and `2.0`. All ~114 `Arc::new(Expr::number(N))` call sites in simplification rules replaced with `arc_number(N)` — hot-path constant construction is now a single atomic refcount bump instead of a heap allocation.
- **`term_hash` field on `Expr`**: Pre-computed coefficient-insensitive hash cached at construction. Used by like-term grouping in simplification to skip repeated full-tree traversals.
- **Two-map generational cache eviction** (`HashKeyedCache`): Replaced O(n) `Vec::retain` eviction with O(1) swap of `current`/`previous` maps. Old generation is dropped in bulk when the cache exceeds capacity.
- **`extract_coeff_arc` fast-path**: Changed signature from `&Expr` to `&Arc<Expr>`. Added early-return for products with no numeric factors (returns `Arc::clone` — refcount bump only, no allocation). Fall-through for non-product terms also returns `Arc::clone` instead of `Arc::new(expr.clone())`.
- **Hash fast-path in `expr_cmp_type_strict`**: Added a single `_ if a.hash != b.hash => a.hash.cmp(&b.hash)` guard arm after the Symbol arms. All composite-type comparisons (Sum, Product, Pow, Div, FunctionCall, Derivative, Poly) now short-circuit in O(1) when hashes differ (the common case), removing seven redundant per-arm hash checks.
- **DUMMY_ARC pre-clone in `drain_children`**: For `Div` and `Pow` branches in the iterative `Drop` impl, clone `DUMMY_ARC` once and move it into the second `replace()` call instead of cloning twice — saves one atomic increment/decrement per node dropped.
- **FxHashMap in `Compiler`**: Replaced `std::collections::HashMap` with `FxHashMap` for `param_index`, `cse_cache`, and `const_map` in the evaluator compiler.
- **Iterative AST traversals**: Converted recursive traversals to explicit-stack iterative versions, eliminating stack-overflow risk on deeply nested expressions:
  - `collect_symbol_names` (display), `node_count`, `max_depth`, `contains_var_id`, `contains_var_str`, `has_free_variables`, `collect_variables`, `fold` (analysis).
  - `prettify_roots`, `normalize_for_comparison`, `is_known_non_negative` (simplification helpers) — use a Visit/Assemble two-stack post-order pattern for owned transforms.
- **`can_apply()` pre-check on `Rule` trait**: New default method returns `true`; the engine calls it before `apply()` on cache-miss. Avoids building the call frame and cache insertion for non-matching expressions. Implemented for `PerfectSquareRule` (skip sums with ≠2–3 terms) and `PolyGcdSimplifyRule` (skip plain-number numerator/denominator).
- **Simplifier `max_depth` raised to 200**: Up from 50 to accommodate deeper expressions that previously hit the limit prematurely.
- **Streamlined Polynomial Evaluation**: Removed opaque `PolyEval` instruction from entire pipeline in favor of in-line arithmetic chains to improve downstream fusion transparency.
- **Estrin's Scheme Tree Expansion**: Reconfigured polynomial expansion for degree $\ge 4$ to build a balanced, logarithmic arithmetic tree for better superscalar pipeline depth utilization.
- **Instruction-Level CSE (GVN)**: Implemented forward-iteration Global Value Numbering pass on Virtual IR to deduplicate algebraic redundancies escaping top-level AST-structural caches.
- **Commutative Normalization**: Enabled GVN to sort commutative operands of virtual instructions, boosting redundancy matches for commutative pairs.


## [0.8.1] - 2026-02-15

### Added
- **Math module fuzz coverage**: Added `src/tests/fuzz_math_modules.rs` with comprehensive stress tests for Bessel functions (J, Y, I, K), elliptic integrals, gamma functions, Lambert W, associated Legendre polynomials, and spherical harmonics. Tests verify parity relations, recurrence formulas, and domain-specific edge cases.
- **Evaluator fuzz coverage**: Added `src/tests/fuzz.rs` and `src/tests/fuzz_evaluator.rs` with stress tests for simplification and evaluator parity across scalar, SIMD batch, and parallel code paths.
- **Normalization diagnostic test**: Added `src/tests/normalization_check.rs` for parser normalization inspection and expression memory-size sanity checks.
- **Large-expression benchmark example**: Added `examples/expr_with_4649517_caracters-comparison.rs` for multi-phase SymbAnaFis vs Symbolica comparison on very large input.
- **Video export helper**: Added `examples/video_writer.py` with NVENC detection and automatic CPU fallback for Matplotlib animations.
- **Large expression fixture tracked**: Added `examples/symblica_exp/big_expr.txt` to stage the real benchmark input used by the new comparison flow.

### Changed
- **Compiler parameter lookup optimization**: Parameter index lookup changed from O(n) linear search to O(1) HashMap lookup via new `param_index` field in `Compiler` struct.
- **Evaluator parameter semantics**: `CompiledEvaluator::evaluate` now treats missing parameters as `0.0` and ignores extra trailing parameters.
- **Evaluator fast paths**:
  - Added constant-expression fast path (`LoadConst` only) to bypass stack execution.
  - Simplified `Powi` instruction handling to delegate directly to `f64::powi` (removed specialized inline exponent patterns).
  - Changed `Tetragamma` instruction to call dedicated `eval_tetragamma` instead of generic `eval_polygamma(3, ...)`.
  - Expanded inline execution coverage for rare/special instructions (Gamma family, Bessel family, Zeta family, Legendre, spherical harmonic, etc.) to reduce heap fallback.
- **Compiler/CSE pipeline**:
  - Removed hard `MAX_STACK_DEPTH` overflow limit.
  - CSE cache now stores `(Expr, slot)` and verifies structural equality on hash hits to avoid collision miscompiles.
  - Adjusted expensive-expression heuristics (including caching of powers over symbols).
  - Reworked large-sum compilation to an iterative "emit-all-then-fold" strategy with MulAdd/NegMulAdd pattern lowering and no recursive sum-chain growth.
- **Constant handling by symbol ID**:
  - Compiler constant folding now uses ID-based constant lookup.
  - Added ID-based helpers in known symbols (`is_known_constant_by_id`, `get_constant_value_by_id`) and support for `pi`/`PI`/`Pi` and `e`/`E`.
- **Context and custom function plumbing**:
  - `Context` user functions moved from string-keyed maps to interned-ID keyed maps.
  - Added explicit name↔ID tracking (`fn_name_to_id`) for lookup/serialization paths.
  - Diff/Simplify builders now pass custom bodies as ID-keyed maps.
- **Simplification context model**:
  - `CustomBodyMap` and `RuleContext.custom_bodies` changed to `HashMap<u64, BodyFn>`.
  - Removed `variables`/`known_symbols` from rule context and simplified rule API around UID-based constant behavior.
  - `known_symbols` is now explicitly treated as parser hints (not simplifier behavior switches) in docs and API comments.
- **Core expression/canonicalization updates**:
  - `Expr::substitute` now compares symbols by interned ID instead of string.
  - Product finalization now merges exponent groups (`x * x^2 * x^a`) via base-aware grouping and exponent summation.
  - Sum/poly merge paths now include structural compatibility checks before polynomial fusion.
  - Ordering logic now compares base → exponent → coefficient to keep like terms adjacent.
  - Polynomial API updated with `base_arc()`/`with_base()` support and Arc-oriented base access.
- **Differentiation engine**: `derive` pre-interns the differentiation variable once and threads `var_id` through recursive derivation for O(1) symbol comparisons.
- **Benchmark/examples workflow**:
  - Updated `benches/BENCHMARK_RESULTS.md` to version `0.8.1` data dated `2026-02-11`.
  - Updated `benches/rust/benchmark_symbolica.rs` to reuse evaluation buffers for fairer cross-engine measurement.
  - Updated Python benchmarks (`aizawa_flow.py`, `clifford_benchmark.py`, `double_pendulum_benchmark.py`, `gradient_descent.py`) with explicit prep/run metrics, optional video generation toggle, strict "all engines must pass" mode, and shared MP4 writing helper.
- **Dev dependencies**: Added `ahash` and `rand`;

### Fixed
- **Custom function expansion mismatch**: Numeric simplification now resolves custom bodies by function symbol ID (`name.id()`), fixing missed expansions caused by string-key lookups.
- **CSE hash-collision safety**: Prevented incorrect cache reuse by validating structural equality after hash match.
- **Like-term collision correctness**: Algebraic combination now verifies structural bases inside hash buckets, avoiding false merges on hash collisions.
- **Polynomial merge correctness**: Sum/product canonicalization now checks structural compatibility before merging terms sharing hashes.
- **Factoring collision correctness**: Exponent factoring grouping now keys by structural base expression instead of hash-only grouping.
- **SIMD/scalar parity regressions**: Added regression tests covering non-finite trig behavior and a complex NaN-producing special-function expression to ensure batch paths match scalar semantics.

## [0.8.0] - 2026-02-09

### This version release notes are partial; performance improved and multiple correctness/stability fixes landed. The API is intended to remain compatible with 0.7.0; please report regressions.

### Added
- **View API**: New stable, pattern-matchable API for expression inspection (`ExprView` and `Expr::view()`).
  - Allows external tools to safely traverse expression structure.
  - Transparently handles internal `Poly` optimizations by presenting them as `Sum`.
  - Supports zero-cost viewing of Symbols and Numbers via `Cow`.
  - Guarantees backward compatibility even if internal representation changes (e.g., to multivariate polynomials).
  - Explicit handling of anonymous symbols via ID-based string generation (e.g., "$123").
  - **Python bindings**: Full `ExprView` support with properties (`kind`, `children`, `value`, `name`) and helper methods (`is_sum()`, `is_product()`, etc.).
  - **Symbol.anon()**: Exposed `Symbol::anon()` method in Python bindings for creating anonymous symbols.
  - Examples: `view_api_demo.rs` (Rust) and `view_api_demo.py` (Python) demonstrating pattern matching and custom format conversion.
- **Known Symbols (KS)**: Global `KS` registry for centralized management of mathematical functions and constants.
- **Display System**: `DisplayContext` and expanded `Display` trait supporting sophisticated LaTeX, Unicode, and plain text formatting.
- **UserFunction API**: Added `UserFunction::any_arity()` helper for registering variadic custom functions (0..=usize::MAX).
- **Rule Registry Tests**: Added comprehensive test suite `rule_registry_tests` for category loading, priority sorting, and uniqueness validation.
- **Builtins**: Added `spherical_harmonic` to lexer builtins.
- **Helpers**: Added `is_known_constant` helper in `known_symbols`.

### Changed
- **Core Refactoring**: Refactored `ExprKind::FunctionCall` and `ExprKind::Symbol` to use `InternedSymbol`.
- **Evaluator Promotion**: Moved `core::evaluator` to top-level `src/evaluator` and updated internal references.
- **Engine Optimization**: Updated differentiation engine, evaluator/compiler, and all simplification rules to use the `KS` registry for ID-based dispatch.
- **Simplification Tolerance**: Standardized floating-point comparisons to use `EPSILON` (1e-14) across all rule categories.
- **LaTeX Display**: Improved LaTeX output with operator precedence and special symbol formatting (e.g., `\exp`, `\ln`).
- **Python Bindings**: Improved `__eq__` implementations and clarified docstrings regarding type-preserving behavior.
- **Renames**: Renamed `perfect_square` -> `perfect_square_factoring` and `perfect_cube` -> `perfect_cube_factoring`.
- **Evaluator Performance Optimizations**: 
  - Increased inline stack/cache sizes (32→48 elements, 16→32 slots) for better coverage of complex expressions.
  - Switched to `MaybeUninit` arrays to avoid zero-initialization overhead ("zero tax").
  - Introduced raw pointer arithmetic for stack operations to eliminate bounds checks in hot path.
  - Pre-loaded instruction and constant slices to reduce indirection.
  - Optimized `Square` instruction to use in-place multiplication (`*=`) instead of temporary variables.
  - Inlined additional mathematical functions in batch evaluation to reduce function call overhead.
  - Reordered match arms in execution paths to prioritize hot instructions (arithmetic first).
  - Changed parameter mapping from `HashMap` to `Vec` for faster linear search in compilation.

### Fixed
- **Derivative Edge Cases**: Improved fast-path for `x^0` and graceful handling of empty argument lists.
- **Simplification Safety**: Added `max_depth`/`max_nodes` checks and name collision validation in `simplify_str`.
- **Display Robustness**: Improved handling of negative factors in product expressions.
- **Context Safety**: Enhanced name collision detection in `Context`.
- **Documentation**: Added warnings to vector calculus helpers regarding custom function support.
- **Cosmetic**: Replaced Unicode box-drawing characters with ASCII in examples for better compatibility.
- **CRITICAL: Python Binding Stability**: Fixed Python interpreter crash from `expect("PyO3 object conversion failed")` - now properly raises appropriate Python exceptions instead of aborting on OOM or conversion failures.
- **CRITICAL: Parallel Evaluation Panic**: Fixed panic on empty columns in `evaluate_parallel` - now returns `EvalColumnLengthMismatch` error when n_points > 0 but any column is empty. Non-empty columns of different lengths continue to broadcast from the last value as before.
- **Correctness: Dimension Validation**: Fixed `evaluate_parallel` to return proper `EvalColumnMismatch` error instead of silently returning `Ok(vec![])` when variable/value dimensions don't match (now matches documented API behavior).
- **Correctness: Empty Column Handling**: Fixed slow path in `evaluate_parallel` to return proper error instead of silently producing NaN when accessing values from an empty column.
- **PyO3 0.28.0 Compatibility**: Updated `#[pyclass]` attributes to add `from_py_object` option to resolve deprecation warnings for types implementing `Clone`.

## [0.7.0] - 2026-01-20

### Added
- **Developer Documentation**: Added detailed guides for extending the library:
  - `CONTRIBUTING.md`: Comprehensive checklists for adding new mathematical functions (12+ locations) and simplification rules (2-3 locations).
  - `.agent/workflows/add-function.md`: Automated workflow for agentic AI assistance when adding functions.
  - `.agent/workflows/add-simplification-rule.md`: Automated workflow for adding simplification rules.
- **CSE (Common Subexpression Elimination)**: Bytecode compiler now detects and caches duplicate subexpressions for reuse:
  - Single-pass detection using 64-bit structural hashing
  - New bytecode instructions: `StoreCached`, `LoadCached`
  - SIMD and scalar cache support for batch evaluation

### Code Quality - Evaluator Refactoring
- **Modular Evaluator Architecture**: Split monolithic `evaluator.rs` (3,287 lines) into 7 focused modules:
  - `mod.rs` (464 lines): Public API and `CompiledEvaluator`
  - `compiler.rs` (941 lines): Bytecode compilation with CSE and Horner optimization
  - `execution.rs` (668 lines): Scalar evaluation hot path
  - `simd.rs` (1,106 lines): SIMD batch evaluation with `f64x4`
  - `stack.rs` (380 lines): Unsafe stack primitives with safety documentation
  - `instruction.rs` (615 lines): Bytecode instruction definitions
  - `tests.rs` (402 lines): Unit tests

### Performance
- **CSE Compilation**: 5-14% faster compilation due to single-pass optimization
- **CSE Evaluation**: Up to 28% faster evaluation for expressions with repeated subexpressions
- **Eliminated 64 `.expect()` calls** in SIMD evaluation paths:
  - Added `top!()` and `pop!()` macros for unchecked stack access
  - Added `simd_stack_pop()` unsafe function with debug assertions
  - All stack operations now use unsafe unchecked access with compile-time validation
- **Safety Model**: Two-phase stack safety:
  1. Compiler validates stack depth at compile time
  2. `debug_assert!` catches bugs in debug builds
  3. Zero-cost unchecked access in release builds

### Fixed
- **Visitor Silent Failure**: `walk_expr` now prints warning to stderr when expression tree exceeds maximum depth (1000 levels) instead of silently truncating traversal
- **Visibility Warnings**: Fixed 7 `redundant_pub_crate` clippy warnings by changing internal module visibility from `pub(crate)` to `mod`
- **CommonTermFactoringRule Bug**: Fixed incorrect factorization of expressions like `y*y + y`:
  - **Before (wrong)**: `y*y + y` → `y²*(1 + y)` (factor count not tracked properly)
  - **After (correct)**: `y*y + y` → `y*(y + 1)`
  - Root cause: Algorithm counted each factor occurrence in the first term but only checked presence (not count) in other terms
  - Fix: Now tracks minimum factor count across all terms before factoring
- **Parallel Evaluation CSE Cache**: Fixed panic in `evaluate_parallel` when CSE was enabled - now properly allocates cache buffer per-thread
- **Python Example API Call**: Fixed `api_showcase.py` using `x.id()` method call instead of `x.id` property

### Documentation
- **Safety Documentation**: All unsafe blocks now have comprehensive `// SAFETY:` comments explaining invariants
- **Module-level docs**: Each evaluator submodule has architecture diagrams and usage examples


## [0.6.0] - 2026-01-09

### Added
- **Compile-Time Singularity Detection**: Compiler now detects removable singularities and emits safe bytecode:
  - `E/E` pattern → `LoadConst(1.0)` (avoids NaN at x=0 even without simplification)
  - `sin(E)/E` pattern → `Sinc` instruction (handles x=0 correctly)
- **Numeric Literal Underscores**: Parser now supports underscores in numeric literals for improved readability (e.g., `1_000_000` instead of `1000000`)
- **Comprehensive Property-Based Test Suite** (790 total lines):
  - Rust: 595 lines (`src/tests/property_tests.rs`) - fuzz testing and property validation using quickcheck
  - Python: 195 lines (`tests/python/test_property_based.py`) - algebraic invariant testing
  - Coverage: Parser robustness (1000+ randomly generated expressions), derivative rules (chain, product, quotient), function combinations, algebraic identities

### Fixed
- **Parser**: Fixed implicit multiplication between numbers and functions, e.g., `4 sin(x)` now correctly parses as `4 * sin(x)`.
- **Simplification Bug**: Fixed `remove_factors` helper to only remove ONE occurrence per factor instead of ALL occurrences. This bug caused incorrect simplification: `y + y*y` → `2*y` (wrong) instead of `y*(1+y)` (correct).
- **Python FFI Error Handling**: Added `impl From<DiffError> for PyErr` and `impl From<SymbolError> for PyErr` for cleaner error conversion across FFI boundaries. Replaced 27 verbose `PyErr::new` calls with `.map_err(Into::into)`.

### Code Quality
- **Pedantic Clippy Compliance**: Fixed all pedantic clippy warnings with targeted `#[allow(...)]` attributes and code refactoring:
  - Added `#[allow(clippy::needless_pass_by_value)]` to 15+ PyO3 functions where owned types are required by Python bindings
  - Added `#[allow(clippy::cast_precision_loss)]` to i64→f64 casts (intentional for Python integer interop)
  - Added `#[allow(clippy::cast_possible_wrap)]` to `__hash__` functions (Python requires isize)
  - Added `#[allow(clippy::too_many_lines)]` to large dispatch functions in display, evaluator, and simplification
  - Refactored closures in `user_fn` and `with_function` to use idiomatic `let...else` and `map_or_else` patterns
- **Clippy float_cmp and similar_names**: Resolved remaining strict lint warnings:
  - Added justification comments to all `#[allow(clippy::float_cmp)]` attributes for exact constant comparisons
  - Refactored `matches!` macro usage in `LnERule` to if-let chains (attributes on `matches!` are ignored by rustc)
  - Renamed tuple-destructured variables in `HyperbolicTripleAngleRule` to avoid `similar_names` lint
- **Allow Attribute Audit**: Added inline justification comments to all ~200 `#[allow(clippy::...)]` attributes across 78 files:
  - `float_cmp` (~40): Exact math constant comparisons (1.0, -1.0, PI)
  - `cast_possible_truncation` (~50): Bessel/Legendre orders, verified `fract()==0.0`
  - `needless_pass_by_value` (~20): PyO3 requires owned types from Python
  - `panic` (5): Explicit unwrap APIs and unreachable code guards

### Performance
- **Polynomial operations**: Added `Arc::ptr_eq` short-circuit optimization to all base comparisons (`try_add_assign`, `add`, `mul`, `gcd`, `try_mul`). Provides O(1) pointer comparison (~1 CPU cycle) before falling back to O(N) deep equality, benefiting:
  - **Squaring**: `p.mul(&p)` now skips redundant self-comparison
  - **Cloned polynomials**: Operations on `Arc::clone`d bases avoid deep tree traversal
  - **Simplifier reuse**: Expressions with shared sub-structures get fast-path matching
- **Parallel evaluation**: Implemented thread-local buffer optimization using Rayon's `map_init`. The SIMD stack buffer is now allocated once per thread instead of once per chunk, reducing allocations by ~N/threads factor (e.g., 8x fewer allocations on 8-core systems for 1M points).

### Changed
- **Breaking**: `eval_f64_py` and `CompiledEvaluator.eval_batch` return `numpy.ndarray` instead of `list` when input contains NumPy arrays (Type-Preserving).
- **Build Configuration - Strict Linting**:
  - Added comprehensive lints configuration to Cargo.toml applying to all targets (lib, tests, examples, benches)
  - 13 new "deny" level rules: `dbg_macro`, `lossy_float_literal`, `mutex_atomic`, `panic_in_result_fn`, `print_stdout`, `rc_buffer`, `todo`, `undocumented_unsafe_blocks`, `unimplemented`, `suboptimal_flops`, `large_stack_arrays`
  - Additional pedantic, nursery, and restriction-level checks enabled for safety, performance, and mathematical correctness (HPC-focused)
  - ⚠️ **Breaking**: May require downstream crates to adjust their lint levels when depending on symb_anafis

## [0.5.1] - 2025-12-31

### Fixed
- **CI/CD**: Fixed Linux ARM builds by specifying Python interpreter path for cross-compilation in GitHub Actions.
- **CI/CD**: Fixed macOS x86 builds by pinning runner to `macos-15-intel` (Intel) to avoid generating duplicate ARM64 wheels on `macos-latest` (Apple Silicon), which caused artifacts conflicts and PyPI upload failure (Error 400).
- **CI/CD**: Allow crates.io publish step to fail gracefully if the version already exists (enables split releases where PyPI failed but crates.io succeeded).

## [0.5.0] - 2025-12-31

### Added
- **Python Bindings - Parallel Evaluation Optimizations**:
  - `evaluate_parallel` now accepts `Expr` or `str` for expressions (Full Hybrid input).
  - `evaluate_parallel` now accepts NumPy arrays (`ndarray`) or Python lists for values (zero-copy when possible).
  - Output is type-preserving: `float` for numeric results, `Expr` if input was `Expr` and result is symbolic, `str` if input was `str` and result is symbolic.
  - Added `evaluate_parallel_with_hint()` internal function to skip O(N) numeric check when Python binding pre-computes the hint.
- **Python Bindings - NumPy Hybrid Input**:
  - `CompiledEvaluator.eval_batch()` and `eval_f64_py()` now accept both NumPy arrays (zero-copy via `PyReadonlyArray1`) and Python lists.
  - Added `DataInput` enum and `extract_data_input()` helper for hybrid input handling.
- **Python Bindings - Domain Validation**:
  - Added early domain validation for special functions (`gamma`, `digamma`, `trigamma`, `tetragamma`, `zeta`, `lambertw`, `bessely`, `besselk`, `elliptic_k`, `elliptic_e`, `polygamma`, `beta`, `hermite`, `assoc_legendre`, `spherical_harmonic`, `log`).
  - Functions now raise `PyValueError` with descriptive messages when numeric arguments are at poles or outside valid domain.
- **Python Bindings - Duck Typing**:
  - `diff()`, `simplify()`, `parse()` now return `PyExpr` objects instead of strings for better composability.
  - All Python API functions (`diff`, `simplify`, `gradient`, `hessian`, `jacobian`, `evaluate`, `uncertainty_propagation`, `relative_uncertainty`) now accept both `str` and `Expr` inputs.
  - Added `extract_to_expr()` helper enabling Python operators/functions to accept `Expr`, `Symbol`, `int`, `float`, and `str` interchangeably.
  - Implemented full reverse operators (`__radd__`, `__rsub__`, `__rmul__`, `__rtruediv__`, `__rpow__`) for both `PyExpr` and `PySymbol`.
  - Added `__eq__`, `__hash__`, `__float__` for `PyExpr` (enables dictionary keys, equality, numeric conversion).
  - Extended `PySymbol` with full math method suite (trig, hyperbolic, exp/log, special functions).
  - Updated `.pyi` type stubs to reflect duck typing across 50+ function signatures.
- **Python Bindings - Builders**:
  - `fixed_var()` and `fixed_vars()` now accept both strings and `Symbol` objects.
  - `differentiate()` accepts `Symbol` or `str` for the variable argument.
  - Added `body_callback` support to `user_fn()` and `Context.with_function()` for custom function evaluation.
- **Rust API - Builder Pattern**:
  - `Diff::fixed_var()` and `Diff::fixed_vars()` using `ToParamName` trait (accepts `&str`, `String`, or `&Symbol`).
  - `Simplify::fixed_var()` and `Simplify::fixed_vars()` with same flexibility.
  - Builder methods now validate that fixed variables don't conflict with differentiation variables.
- **Rust API - Operator Extensions**:
  - Added comprehensive i32 operator support: `Symbol +/- i32`, `i32 +/- Symbol`, `Expr +/- i32`, etc.
  - Added f64 operators with `&Symbol` and `&Expr` references.
  - Added 47 doc comments to `ArcExprExt` trait methods for rustdoc compliance.
- **Rust API - CompiledEvaluator**:
  - `compile()` and `compile_auto()` now accept optional `Context` parameter for custom functions.
  - Added convenience methods: `Expr::compile()` and `Expr::compile_with_params()`.
- **Testing**:
  - New `test_comprehensive_api.py`: gradient/hessian/jacobian, builder patterns, integrated workflows.
  - New `test_special_functions.py`: Bessel, Gamma, Beta, Lambert W, erf, elliptic integrals, Hermite.
  - New `test_derivative_oracle.py`: SymPy-verified derivative tests (polynomial, trig, exp/log, chain/product/quotient rules).
  - New `test_parallel_eval.py`: Full Hybrid evaluate_parallel tests with NumPy arrays.
  - New `test_property_based.py`: Mathematical invariants, derivative rules, trig/exp/hyperbolic identities.
  - New `test_compiled_evaluator.py`: CompiledEvaluator batch evaluation tests.
  - New property-based test suite (`src/tests/property_tests.rs`) with 15+ tests for operator combinations.
  - Python fuzz testing (`tests/python/fuzz_test.py`) for crash detection.
  - Python stress testing (`tests/python/stress_test.py`) for memory stability.
- **Documentation**:
  - Comprehensive `eval_f64` documentation in API Reference with function signature, multi-variable examples, performance characteristics table, error handling, and Python API section.
  - Updated API Reference to 15-section structure (~630 lines of additions).
  - Rewrote `examples/api_showcase.py` (759 lines) and `examples/api_showcase.rs` (1095 lines) to match new API structure.

### Changed
- **Breaking: Rust API Signatures**:
  - `Diff::differentiate()` now takes `&Expr` instead of `Expr` (avoid unnecessary cloning).
  - `Simplify::simplify()` now takes `&Expr` instead of `Expr`.
- **Python Return Types**:
  - `parse()` returns `PyExpr` instead of `str`.
  - `diff()` and `simplify()` return `Expr`/`PyExpr` objects instead of strings.
- **eval_f64**: Changed trait bound from `AsRef<str>` to `ToParamName` for Symbol support.
- **Code Quality**:
  - Removed 11 clippy warnings (needless borrows, redundant closures, useless conversions, type complexity).
  - Added `BodyFn` type alias import to reduce complex type annotations.
  - Simplified `ABS_CAP` references to just `ABS` in simplification rules.

### Fixed
- **Critical: Parallel chunk ordering bug** - Fixed `par_bridge()` usage in chunked SIMD path which did not preserve order. Replaced with `into_par_iter()` on pre-collected chunk indices to guarantee result ordering matches input order.
- Fixed rustdoc warning: escaped `Option<Arc<Expr>>` in macro comment.
- Fixed clippy `needless_borrows_for_generic_args` warnings (4 instances).
- Fixed clippy `type_complexity` warnings using `BodyFn` alias (2 instances).
- Fixed clippy `redundant_closure` (1 instance) and `useless_conversion` (6 instances).
- Added missing doc comments to 47 trait methods in `ArcExprExt`.
- Fixed `test_depth_limit` to only run in debug builds (`#[cfg(debug_assertions)]`) since it relies on `debug_assert!`.

### Internal
- **LaTeX Display**: Added proper `log` with base formatting (`\log_{base}(x)`) in LaTeX output.
- **Known Symbols**: Added `LOG` symbol ID for parametric log function; removed deprecated `ABS_CAP` alias.

### Documentation
- **API Reference** (`docs/API_REFERENCE.md`): Major overhaul with 630+ lines of additions including:
  - Comprehensive `eval_f64` section with signature, examples, performance table
  - Updated Context API table with 18 methods
  - Python API examples for custom functions, vector calculus, compilation
  - Extended error handling section with 17 error variants
  - Fixed code examples to use `&expr` instead of `expr`
- **Examples README**: Updated to reflect 15-section structure.
- **Python Type Stubs**: 137+ lines of additions for duck typing support.

## [0.4.1] - 2025-12-29

### Changed
- **License**: Changed from MIT to Apache 2.0
- **`log` function signature**: Changed from `log(x)` to `log(base, x)` for arbitrary base logarithms
  - Use `ln(x)` for natural logarithm (unchanged)
  - Use `log(20, x)` for base-20, `log(4, x)` for base-4, etc.
  - For base 2 and 10 you can still use `log2(x)` and `log10(x)`

### Added
- **Spherical Harmonics Tests**: Comprehensive test suite for spherical harmonics and associated Legendre polynomials
- **`log(b, x)` Simplification Rules**: New rules for simplifying logarithms with specific bases and powers
- **Enhanced Math Functions**: Improved accuracy for `zeta`, `gamma`, and `erf` implementations

### Performance
- **Fast-path quotient rule**: 9-18% faster raw differentiation for patterns like `1/f(x)` and `u/n`
  - Constant numerator: `n/f` → `-n*f'/f²` (skips computing u')
  - Constant denominator: `u/n` → `u'/n` (skips computing v')
- **Simplification engine refactoring**: Reduced deep clones with more Arc usage

### Fixed
- Removed deprecated validation test module
- Cleaned up outdated comments in rules engine

[0.4.1]: https://github.com/CokieMiner/SymbAnaFis/compare/v0.4.0...v0.4.1

## [0.4.0] - 2025-12-28

### Added
- **Unified Context System**: Introduced `Context` for isolated symbol namespaces and unified function registries
- **Enhanced Visitor Pattern**: Added comprehensive visitor utilities for expression traversal and manipulation
- **SIMD-Optimized Batch Evaluation**: Implemented `wide`-based SIMD operations for ~30% faster parallel evaluation
- **Custom Function Support**: Full support for user-defined functions with partial derivatives via `UserFunction` API
- **Python Bindings Enhancements**:
  - `Dual` number support for automatic differentiation
  - `CompiledEvaluator` for faster repeated evaluations
  - `Symbol` and `Context` classes exposed to Python
  - Parallel evaluation bindings with `evaluate_parallel` and `eval_f64`
- **Automatic Differentiation**: Complete dual number implementation with transcendental functions
- **Benchmark Test Suite**: Converted all benchmarks to verified tests for reliability
- **New Examples**: Added `dual_autodiff.rs` and `dual_autodiff.py` demonstrating AD capabilities

### Performance
- **12% faster single expression evaluation** via fast-path macro optimization
- **Stack-based f64 evaluator** replacing heap allocations for numerical evaluation
- **Structural hashing** for O(1) expression equality checks
- **Parallel evaluation** with Rayon for batch operations (requires `parallel` feature)

### Changed
- **Architecture Refactor**: Reorganized codebase into `core/`, `api/`, and `bindings/` directories
- **Migrated to N-ary AST**: Changed from binary Add/Mul to Sum/Product with multiple terms
- **Sparse Polynomial Representation**: Adopts (exponent, coefficient) pairs for memory efficiency
- **Symbol Management**: Rewritten with global interning using `InternedSymbol` for O(1) comparisons
- **Error Handling**: Improved evaluation error reporting and context propagation
- **Renamed API**: `sym()` → `symb()` for consistency across Rust and Python
- **Edition 2024**: Updated Cargo.toml to use Rust edition 2024

### Fixed
- Improved simplification rules for trigonometric, hyperbolic, and algebraic expressions
- Resolved clippy warnings and compilation errors across the codebase
- Fixed division bug causing infinite loops in certain edge cases
- Corrected `abs` derivative implementation for `Dual` numbers
- Updated benchmark infrastructure with accurate timing methodologies

### Documentation
- Overhauled README with modern design and comprehensive examples
- Added ARCHITECTURE.md documenting internal design decisions
- Expanded API_REFERENCE.md with Context, Dual numbers, and custom functions
- Updated BENCHMARK_RESULTS.md with latest performance metrics
- Added inline documentation for all public APIs

[0.4.0]: https://github.com/CokieMiner/SymbAnaFis/compare/v0.3.0...v0.4.0
