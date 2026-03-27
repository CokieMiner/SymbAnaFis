//! # `SymbAnaFis`: Fast Symbolic Differentiation for Rust
//!
//! `SymbAnaFis` is a high-performance symbolic mathematics library focused on differentiation,
//! simplification, and fast numeric evaluation. It provides both string-based and type-safe APIs
//! with extensive optimization for real-world mathematical computing.
//!
//! ## Quick Start
//!
//! ```rust
//! use symb_anafis::{diff, simplify, symb, Diff};
//!
//! // String-based API - quick and familiar
//! let result = diff("x^3 + sin(x)", "x", &[], None).unwrap();
//! assert_eq!(result, "3*x^2 + cos(x)");
//!
//! // Type-safe API - powerful and composable
//! let x = symb("x");
//! let expr = x.pow(3.0) + x.sin();
//! let derivative = Diff::new().differentiate(&expr, &x).unwrap();
//! assert_eq!(derivative.to_string(), "3*x^2 + cos(x)");
//! ```
//!
//! ## Key Features
//!
//! ### 🚀 **High Performance**
//! - **Compiled evaluation**: Expressions compile to optimized bytecode
//! - **SIMD vectorization**: Batch evaluation with f64x4 operations  
//! - **Parallel evaluation**: Multi-threaded computation with Rayon
//! - **Smart simplification**: Rule-based engine with memoization
//!
//! ### 🔧 **Developer Experience**
//! - **Type-safe expressions**: Rust's type system prevents runtime errors
//! - **Operator overloading**: Natural mathematical syntax (`x.pow(2) + x.sin()`)
//! - **Copy symbols**: No `.clone()` needed for symbol reuse
//! - **Builder patterns**: Fluent APIs for differentiation and simplification
//!
//! ### 📚 **Mathematical Features**
//! - **Symbolic differentiation**: Automatic derivatives with simplification
//! - **Vector calculus**: Gradients, Jacobians, and Hessian matrices
//! - **Custom functions**: User-defined functions with partial derivatives
//! - **Uncertainty propagation**: Error analysis with covariance matrices
//!
//! ## Core APIs
//!
//! Different API styles for the same operations, choose based on your use case:
//!
//! | Operation | String API | Type-safe API | Builder API |
//! |-----------|------------|---------------|-------------|
//! | **Differentiation** | `diff("x^2 + sin(x)", "x", &[], None)` | `Diff::new().differentiate(&expr, &x)` | `Diff::new().domain_safe(true).differentiate(&expr, &x)` |
//! | **Simplification** | `simplify("x + x + x", &[], None)` | Use `Simplify::new().simplify(&expr)` | `Simplify::new().max_iterations(100).simplify(&expr)` |
//! | **Evaluation** | `evaluate_str("x^2", &[("x", 2.0)])` | `expr.evaluate(&vars, &custom_evals)` | `CompiledEvaluator::compile(&expr, &["x"], None)` |
//!
//! ## Examples by Use Case
//!
//! ### Basic Differentiation
//! ```rust
//! # use symb_anafis::{symb, diff, Diff};
//! // String API
//! let result = diff("x^2 + 3*x + 1", "x", &[], None).unwrap();
//! assert_eq!(result, "3 + 2*x"); // Order may vary
//!
//! // Type-safe API
//! let x = symb("x");
//! let poly = x.pow(2.0) + 3.0 * x + 1.0;
//! let derivative = Diff::new().differentiate(&poly, &x).unwrap();
//! assert_eq!(derivative.to_string(), "3 + 2*x"); // Order may vary
//! ```
//!
//! ### Vector Calculus
//! ```rust
//! # use symb_anafis::{symb, gradient};
//! let x = symb("x");
//! let y = symb("y");
//! let f = x.pow(2.0) + y.pow(2.0); // f(x,y) = x² + y²
//!
//! let grad = gradient(&f, &[&x, &y]).unwrap();
//! // Returns [2*x, 2*y]
//! ```
//!
//! ### High-Performance Evaluation
//! ```rust
//! # use symb_anafis::{symb, CompiledEvaluator};
//! let x = symb("x");
//! let expr = x.sin() * x.cos() + x.pow(2.0);
//!
//! // Compile once, evaluate many times
//! let evaluator = CompiledEvaluator::compile(&expr, &[&x], None).unwrap();
//!
//! // Fast numerical evaluation
//! let result = evaluator.evaluate(&[0.5]); // ~0.479...
//! ```
//!
//! ### Custom Functions
//! ```rust
//! # use symb_anafis::{symb, Context, UserFunction, Diff, parse};
//! # use std::collections::HashSet;
//! let x = symb("x");
//!
//! // Define f(x) with derivative f'(x) = 2x
//! let ctx = Context::new().with_function("f", UserFunction::new(1..=1)
//!     .partial(0, |args| 2.0 * (*args[0]).clone()).unwrap());
//!
//! let expr = parse("f(x^2)", &HashSet::new(), &HashSet::new(), Some(&ctx)).unwrap();
//! let derivative = Diff::new().context(&ctx)
//!     .differentiate(&expr, &x).unwrap(); // Chain rule: f'(x²) * 2x
//! ```
//!
//! ### Output Formatting
//! ```rust
//! # use symb_anafis::symb;
//! let x = symb("x");
//! let expr = x.pow(2.0) / (x + 1.0);
//!
//! println!("{}", expr);           // x^2/(x + 1)
//! println!("{}", expr.to_latex()); // \\frac{x^{2}}{x + 1}
//! println!("{}", expr.to_unicode()); // x²/(x + 1)
//! ```
//!
//! ## Feature Flags
//!
//! `SymbAnaFis` supports optional features for specialized use cases:
//!
//! ```toml
//! [dependencies]
//! symb_anafis = { version = "0.7", features = ["parallel"] }
//! ```
//!
//! - **`parallel`**: Enables parallel evaluation with Rayon
//!   - Adds `eval_f64()` for SIMD+parallel evaluation  
//!   - Enables `evaluate_parallel()` for batch operations
//!   - Required for high-performance numerical workloads
//!
//! - **`python`**: Python bindings via `PyO3` (separate crate)
//!   - Type-safe integration with `NumPy` arrays
//!   - Automatic GIL management for performance
//!   - See `symb-anafis-python` crate for usage

//! ## Architecture Overview
//!
//! `SymbAnaFis` is built with a layered architecture for performance and maintainability:
//!
//! ```text
//! ┌─ PUBLIC APIs ─────────────────────────────────────────────┐
//! │                                                           │
//! │  String API      Type-safe API      Builder API           │
//! │  -----------     --------------      -----------          │
//! │  diff()          x.pow(2)           Diff::new()           │
//! │  simplify()      expr + expr        Simplify::new()       │
//! │                                                           │
//! ├─ CORE ENGINE ─────────────────────────────────────────────┤
//! │                                                           │
//! │  Parser          Differentiator     Simplifier            │
//! │  -------         --------------     ----------            │
//! │  "x^2" → AST     AST → AST          AST → AST             │
//! │                                                           │
//! ├─ EVALUATION ──────────────────────────────────────────────┤
//! │                                                           │
//! │  Interpreter     Compiler           SIMD Evaluator        │
//! │  -----------     --------           ---------------       │
//! │  AST → f64       AST → Bytecode     Bytecode → [f64; N]   │
//! │                                                           │
//! └───────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Module Organization
//!
//! The crate is organized into logical layers:
//!
//! - **Core**: [`Expr`], [`Symbol`], error types, visitor pattern
//! - **Parsing**: String → AST conversion with error reporting  
//! - **Computation**: Differentiation, simplification, evaluation engines
//! - **Functions**: Built-in function registry and mathematical implementations
//! - **APIs**: User-facing builders and utility functions
//! - **Bindings**: Python integration and parallel evaluation
//!
//! ## Getting Started
//!
//! 1. **Add dependency**: `cargo add symb_anafis`
//! 2. **Import symbols**: `use symb_anafis::{diff, symb, Diff};`
//! 3. **Create expressions**: `let x = symb("x"); let expr = x.pow(2);`
//! 4. **Compute derivatives**: `let result = diff("x^2", "x", &[], None)?;`
//!
//! ## Performance Notes
//!
//! - **Compilation**: Use `CompiledEvaluator` for repeated numeric evaluation
//! - **Batch operations**: Enable `parallel` feature for SIMD and multi-threading  
//! - **Memory efficiency**: Expressions use `Arc` sharing for common subexpressions
//! - **Simplification**: Automatic during differentiation, manual via `simplify()`
//!
//! ## Safety and Limits
//!
//! - **Stack safety**: Compile-time validation prevents stack overflow in evaluation
//! - **Memory limits**: Default max 10,000 nodes and depth 100 prevent resource exhaustion
//! - **Error handling**: All operations return `Result` with descriptive error messages
//! - **Thread safety**: All public types are `Send + Sync` for parallel usage

// ============================================================================
// Module Declarations
// ============================================================================

// Core infrastructure
mod core;
mod parser;

// Computation engines
mod diff;
mod evaluator;
mod simplification;

// Function and math support
mod functions;
mod math;
mod uncertainty;

// User-facing APIs
mod bindings;
mod convenience;

// ============================================================================
// Feature Flags Documentation
// ============================================================================
//
// SymbAnaFis supports optional features for specialized use cases:
//
// - **`parallel`**: Enables parallel evaluation with Rayon
//   - Adds `eval_f64()` for SIMD+parallel evaluation
//   - Enables `evaluate_parallel()` for batch operations
//
// - **`python`**: Python bindings via PyO3 (separate crate)
//   - Type-safe integration with NumPy arrays
//   - Automatic GIL management for performance
//
// Add to `Cargo.toml`:
// ```toml
// [dependencies]
// symb_anafis = { version = "0.7", features = ["parallel"] }
// ```

// ============================================================================
// Public API Re-exports
// ============================================================================

// === 1. Foundation & Core Models ===

/// The main expression type for building and manipulating mathematical expressions.
/// See the [crate documentation](crate) for usage examples.
pub use core::{DiffError, Expr, Span, Symbol, SymbolError};

/// Mathematical scalar trait for high-performance computation.
pub use core::MathScalar;

/// Dual number type for automatic differentiation.
pub use math::Dual;

/// Functions for creating and managing symbols in the global registry.
///
/// ## Copy Semantics
/// `Symbol` implements `Copy`, enabling natural mathematical syntax:
/// ```rust
/// # use symb_anafis::symb;
/// let x = symb("x");
/// let expr = x + x;  // No .clone() needed!
/// ```
pub use core::{
    ArcExprExt, clear_symbols, remove_symbol, symb, symb_get, symb_new, symbol_count,
    symbol_exists, symbol_names,
};

// === 2. Ingestion & Rules ===

/// Context system for custom functions and parsing.
pub use core::{Context, UserFunction};

/// String → AST parsing with context support.
pub use parser::parse;

// === 3. Operations & Calculus ===

/// Fluent APIs for differentiation and simplification.
pub use diff::{Diff, diff};
pub use simplification::{Simplify, simplify};

/// Vector calculus operations for computing gradients, Jacobians, and Hessians.
pub use convenience::{
    evaluate_str, gradient, gradient_str, hessian, hessian_str, jacobian, jacobian_str,
};

// === 4. Advanced Analysis ===

/// Uncertainty propagation and error analysis for experimental data.
pub use uncertainty::{
    CovEntry, CovarianceMatrix, Uncertainty, relative_uncertainty, uncertainty_propagation,
};

pub use core::ExprView;
// === 5. High-Performance Evaluation ===

/// High-performance compiled evaluator for repeated numeric computations.
pub use evaluator::{CompiledEvaluator, EvaluatorBuilder, ToParamName, VarLookup};

/// High-performance parallel evaluation (requires `parallel` feature).
/// Enables automatic chunked parallel execution with SIMD vectorization.
#[cfg(feature = "parallel")]
pub use evaluator::eval_f64;
#[cfg(feature = "parallel")]
pub use evaluator::{EvalResult, ExprInput, SKIP, Value, VarInput, evaluate_parallel};

// ============================================================================
// Constants
// ============================================================================

/// Default maximum AST depth.
/// This limit prevents stack overflow from deeply nested expressions.
pub(crate) const DEFAULT_MAX_DEPTH: usize = 100;

/// Default maximum AST node count.
/// This limit prevents memory exhaustion from extremely large expressions.
pub(crate) const DEFAULT_MAX_NODES: usize = 10_000;

/// Tolerance for floating-point comparisons (used throughout expression operations)
pub const EPSILON: f64 = 1e-14;

#[cfg(test)]
#[allow(missing_docs)]
#[allow(clippy::pedantic, clippy::nursery, clippy::restriction)]
#[allow(clippy::cast_possible_truncation, clippy::float_cmp)]
#[allow(clippy::print_stdout, clippy::unwrap_used)]
mod tests;
