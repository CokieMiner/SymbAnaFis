# Changelog

All notable changes to SymbAnaFis will be documented in this file.

## [Unreleased]

### Fixed
- **Parser**: Fixed implicit multiplication between numbers and functions, e.g., `4 sin(x)` now correctly parses as `4 * sin(x)`.

### Performance
- **Polynomial operations**: Added `Arc::ptr_eq` short-circuit optimization to all base comparisons (`try_add_assign`, `add`, `mul`, `gcd`, `try_mul`). Provides O(1) pointer comparison (~1 CPU cycle) before falling back to O(N) deep equality, benefiting:
  - **Squaring**: `p.mul(&p)` now skips redundant self-comparison
  - **Cloned polynomials**: Operations on `Arc::clone`d bases avoid deep tree traversal
  - **Simplifier reuse**: Expressions with shared sub-structures get fast-path matching
- **Parallel evaluation**: Implemented thread-local buffer optimization using Rayon's `map_init`. The SIMD stack buffer is now allocated once per thread instead of once per chunk, reducing allocations by ~N/threads factor (e.g., 8x fewer allocations on 8-core systems for 1M points).

### Changed
- **Breaking**: `eval_f64_py` and `CompiledEvaluator.eval_batch` return `numpy.ndarray` instead of `list` when input contains NumPy arrays (Type-Preserving).

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
