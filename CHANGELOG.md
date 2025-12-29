# Changelog

All notable changes to SymbAnaFis will be documented in this file.

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
