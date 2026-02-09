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
//! assert_eq!(result, "cos(x) + 3*x^2");
//!
//! // Type-safe API - powerful and composable
//! let x = symb("x");
//! let expr = x.pow(3.0) + x.sin();
//! let derivative = Diff::new().differentiate(&expr, &x).unwrap();
//! assert_eq!(derivative.to_string(), "cos(x) + 3*x^2");
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
//! let derivative = Diff::new().with_context(&ctx)
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
mod core; // Core types: Expr, Symbol, Error, Display, Visitor
mod parser; // String-to-AST parsing

// Computation engines
mod diff; // Differentiation engine
mod evaluator; // Compiled evaluator
mod simplification; // Expression simplification

// Function and math support
mod functions; // Function definitions and registry
mod math; // Mathematical function implementations
mod uncertainty; // Uncertainty propagation

// User-facing APIs
mod api; // Builder APIs: Diff, Simplify, helpers
mod bindings; // External bindings (Python, parallel)

// ============================================================================
// Test Module
// ============================================================================

#[cfg(test)]
#[allow(missing_docs)]
#[allow(clippy::pedantic, clippy::nursery, clippy::restriction)]
#[allow(clippy::cast_possible_truncation, clippy::float_cmp)]
#[allow(clippy::print_stdout, clippy::unwrap_used)] // Standard in tests
mod tests;

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

// === Core Types ===

/// The main expression type for building and manipulating mathematical expressions.
///
/// See the [crate documentation](crate) for usage examples.
pub use core::{DiffError, Expr, Span, Symbol, SymbolError};

/// Mathematical scalar trait and compiled evaluator for high-performance computation.
pub use core::traits::MathScalar;
pub use evaluator::{CompiledEvaluator, ToParamName};

/// Dual number type for automatic differentiation.
pub use math::dual::Dual;

// === Symbol Management ===

/// Functions for creating and managing symbols in the global registry.
///
/// ## Symbol Creation
///
/// - [`symb`] - Get or create a symbol (idempotent, never errors)
/// - [`symb_new`] - Create new symbol only (errors if exists)
/// - [`symb_get`] - Get existing symbol only (errors if not found)
/// - [`symbol_count`], [`symbol_exists`] - Registry inspection
/// - [`clear_symbols`] - Reset the symbol registry (testing only)
///
/// ## Examples
///
/// ```rust
/// # use symb_anafis::{symb, symb_new, symb_get, symbol_exists};
/// // symb() - always works, idempotent
/// let x1 = symb("x");  // Creates "x"
/// let x2 = symb("x");  // Returns same "x", no error
/// assert_eq!(x1.id(), x2.id());  // true - same symbol!
///
/// // symb_new() - strict create
/// let y = symb_new("y").unwrap();     // Ok - creates "y"
/// assert!(symb_new("y").is_err());    // Err(DuplicateName)
///
/// // symb_get() - strict get
/// assert!(symb_get("z").is_err());     // Err(NotFound)
/// let y2 = symb_get("y").unwrap();     // Ok - same as y
///
/// // Check if symbol exists
/// assert!(symbol_exists("x"));
/// assert!(!symbol_exists("nonexistent"));
/// ```
///
/// ## Copy Semantics
///
/// `Symbol` implements `Copy`, enabling natural mathematical syntax:
///
/// ```rust
/// # use symb_anafis::symb;
/// let x = symb("x");
/// let expr = x + x;  // No .clone() needed!
/// let expr2 = x.pow(2.0) + x.sin();  // Symbol can be reused freely
/// ```
pub use core::{
    ArcExprExt, clear_symbols, remove_symbol, symb, symb_get, symb_new, symbol_count,
    symbol_exists, symbol_names,
};

// === Context and Functions ===

/// Context system for custom functions and parsing.
///
/// - [`Context`] - Unified context for symbols and custom functions
/// - [`UserFunction`] - Definition of user-defined functions with partials
/// - [`parse`] - String → AST parsing with context support
pub use core::unified_context::{Context, UserFunction};
pub use parser::parse;

// === Builder APIs ===

/// Fluent APIs for differentiation and simplification.
///
/// - [`Diff`] - Builder for differentiation with options (`domain_safe`, custom functions)
/// - [`Simplify`] - Builder for simplification with iteration/depth limits
pub use api::{Diff, Simplify};

// === Calculus Functions ===

/// Vector calculus operations for computing gradients, Jacobians, and Hessians.
///
/// - [`gradient`] - Compute ∇f for scalar functions: `[∂f/∂x₁, ∂f/∂x₂, ...]`
/// - [`jacobian`] - Compute J for vector functions: `[[∂f₁/∂x₁, ∂f₁/∂x₂], [∂f₂/∂x₁, ∂f₂/∂x₂]]`  
/// - [`hessian`] - Compute H for scalar functions: `[[∂²f/∂x₁², ∂²f/∂x₁∂x₂], ...]`
/// - [`gradient_str`], [`jacobian_str`], [`hessian_str`] - String-based versions
///
/// ## Examples
///
/// ```rust
/// # use symb_anafis::{symb, gradient_str, jacobian_str, hessian_str};
/// // Gradient: ∇f for scalar function
/// let grad = gradient_str("x^2 + y^2", &["x", "y"]).unwrap();
/// // grad = ["2*x", "2*y"]
///
/// // Hessian: second derivatives matrix  
/// let hess = hessian_str("x^2 * y", &["x", "y"]).unwrap();
/// // hess = [["2*y", "2*x"], ["2*x", "0"]]
///
/// // Jacobian: for vector functions
/// let jac = jacobian_str(&["x^2 + y", "x * y"], &["x", "y"]).unwrap();
/// // jac = [["2*x", "1"], ["y", "x"]]
/// ```
pub use api::{evaluate_str, gradient, gradient_str, hessian, hessian_str, jacobian, jacobian_str};

// === Uncertainty Analysis ===

/// Uncertainty propagation and error analysis for experimental data.
///
/// Compute uncertainty propagation using the standard formula:
/// **`σ_f` = √(Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ))**
///
/// - [`uncertainty_propagation`] - Propagate uncertainties through expressions
/// - [`CovarianceMatrix`] - Handle correlated input uncertainties  
/// - [`relative_uncertainty`] - Compute `σ_f` / |f|
///
/// ## Examples
///
/// ```rust
/// # use symb_anafis::{symb, uncertainty_propagation, CovarianceMatrix, CovEntry};
/// let x = symb("x");
/// let y = symb("y");
/// let expr = x + y;
///
/// // Basic: returns sqrt(sigma_x^2 + sigma_y^2)
/// let sigma = uncertainty_propagation(&expr, &["x", "y"], None).unwrap();
///
/// // With numeric covariance matrix
/// let cov = CovarianceMatrix::diagonal(vec![
///     CovEntry::Num(1.0),  // σ_x² = 1
///     CovEntry::Num(4.0),  // σ_y² = 4
/// ]);
/// let sigma = uncertainty_propagation(&expr, &["x", "y"], Some(&cov)).unwrap();
/// // For f = x + y: σ_f = sqrt(1 + 4) = sqrt(5)
/// ```
pub use uncertainty::{CovEntry, CovarianceMatrix, relative_uncertainty, uncertainty_propagation};

// === Advanced Features ===

/// Advanced APIs for extending functionality.
///
/// - [`visitor`] - Visitor pattern for custom AST traversal and transformation
/// - [`visitor::ExprView`] - Pattern-matchable view of expressions
/// - [`Expr::view()`] - Get an `ExprView` for pattern matching without exposing internals
///
/// # Expression Inspection
///
/// Use `expr.view()` to pattern match on expression structure:
///
/// ```rust
/// # use symb_anafis::{symb, visitor::ExprView};
/// let x = symb("inspect_x");
/// let expr = x.pow(2.0) + x;
///
/// match expr.view() {
///     ExprView::Sum(terms) => { /* handle sum */ }
///     ExprView::Number(n) => { /* handle number */ }
///     _ => { /* other cases */ }
/// }
/// ```
pub use core::visitor;

// === Optional Features ===

/// High-performance parallel evaluation (requires `parallel` feature).
///
/// Enable with: `symb_anafis = { features = ["parallel"] }`
///
/// - [`eval_f64`] - SIMD+parallel evaluation for maximum performance on large datasets
/// - [`parallel`] - Batch evaluation utilities and helper types
///
/// ## Examples
///
/// ```rust
/// # #[cfg(feature = "parallel")]
/// # {
/// # use symb_anafis::{eval_f64, symb};
/// let x = symb("x");
/// let expr = x.pow(2.0);
/// let x_data = vec![1.0, 2.0, 3.0, 4.0];
///
/// // High-performance evaluation across large datasets
/// let results = eval_f64(
///     &[&expr],
///     &[&["x"]],
///     &[&[&x_data[..]]]
/// ).unwrap();
/// // results[0] = [1.0, 4.0, 9.0, 16.0]
/// # }
/// ```
///
/// Features automatic chunked parallelism with SIMD vectorization.
#[cfg(feature = "parallel")]
pub use bindings::eval_f64::eval_f64;
#[cfg(feature = "parallel")]
pub use bindings::parallel;

// ============================================================================
// Constants
// ============================================================================

/// Default maximum AST depth.
/// This limit prevents stack overflow from deeply nested expressions.
pub(crate) const DEFAULT_MAX_DEPTH: usize = 100;

/// Default maximum AST node count.
/// This limit prevents memory exhaustion from extremely large expressions.
pub(crate) const DEFAULT_MAX_NODES: usize = 10_000;

// ============================================================================
// Convenience Functions
// ============================================================================

/// Main API function for symbolic differentiation
///
/// This function provides the simplest interface for computing derivatives.
/// For advanced use cases, consider the [`Diff`] builder pattern which offers
/// more control over domain safety, simplification, and custom functions.
///
/// # Arguments
/// * `formula` - Mathematical expression to differentiate (e.g., "x^2 + sin(y)")
/// * `var_to_diff` - Variable to differentiate with respect to (e.g., "x")
/// * `known_symbols` - Multi-character symbols for parsing (e.g., `&["alpha", "beta"]`)
/// * `custom_functions` - User-defined function names (e.g., `Some(&["f", "g"])`)
///
/// # Returns
/// The derivative as a simplified string, or an error if parsing/differentiation fails
///
/// # Errors
/// Returns `DiffError` if:
/// - **Syntax error**: Formula cannot be parsed (e.g., unmatched parentheses)
/// - **Unknown variable**: Variable to differentiate is not found in the expression
/// - **Unsupported operation**: Rare edge cases in differentiation rules
///
/// # Examples
///
/// ## Basic differentiation
/// ```
/// use symb_anafis::diff;
///
/// // Polynomial
/// let result = diff("x^3 + 2*x + 1", "x", &[], None)?;
/// assert_eq!(result, "2 + 3*x^2");
///
/// // Trigonometric  
/// let result = diff("sin(x) * cos(x)", "x", &[], None)?;
/// assert_eq!(result, "cos(2*x)"); // Simplified to double angle
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// ## Multi-character symbols
/// ```
/// # use symb_anafis::diff;
///
/// let result = diff("alpha * sin(beta)", "alpha", &["beta"], None)?; // only beta is known/fixed
/// assert_eq!(result, "sin(beta)");
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// ## Custom functions
/// ```
/// use symb_anafis::diff;
///
/// // f and g are treated as arbitrary functions
/// let result = diff("f(x) + g(x,y)", "x", &[], Some(&["f", "g"]))?;
/// // Returns: d/dx[f](x) + d/dx[g](x, y)
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// ## Performance and Limits
/// This function applies safety limits to prevent resource exhaustion:
/// - **Max depth**: 100 (default) - prevents stack overflow
/// - **Max nodes**: 10,000 (default) - prevents memory exhaustion
///
/// For expressions exceeding these limits, use the [`Diff`] builder:
/// ```
/// use symb_anafis::Diff;
///
/// let result = Diff::new()
///     .max_depth(500)
///     .max_nodes(50_000)
///     .diff_str("very_large_expression", "x", &[])?;
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// ## Common Patterns
///
/// | Pattern | Example | Output |
/// |---------|---------|--------|
/// | **Polynomial** | `"x^3 + x^2 + x + 1"` | `"3*x^2 + 2*x + 1"` |
/// | **Product rule** | `"x * sin(x)"` | `"sin(x) + x*cos(x)"` |
/// | **Chain rule** | `"sin(x^2)"` | `"2*x*cos(x^2)"` |
/// | **Quotient rule** | `"x/sin(x)"` | `"(sin(x) - x*cos(x))/sin(x)^2"` |
/// | **Logarithmic** | `"ln(x^2)"` | `"2/x"` |
/// | **Exponential** | `"e^(x^2)"` | `"2*x*e^(x^2)"` |
///
/// # See Also
/// - [`Diff`]: Builder pattern for advanced differentiation control
/// - [`simplify`]: Simplification without differentiation
/// - [`gradient`]: Compute multiple partial derivatives
pub fn diff(
    formula: &str,
    var_to_diff: &str,
    known_symbols: &[&str],
    custom_functions: Option<&[&str]>,
) -> Result<String, DiffError> {
    let mut builder = Diff::new();

    if let Some(funcs) = custom_functions {
        builder = funcs
            .iter()
            .fold(builder, |b, f| b.user_fn(*f, UserFunction::any_arity()));
    }

    builder
        .max_depth(DEFAULT_MAX_DEPTH)
        .max_nodes(DEFAULT_MAX_NODES)
        .diff_str(formula, var_to_diff, known_symbols)
}

/// Simplify a mathematical expression
///
/// This function applies algebraic, trigonometric, and other mathematical rules to
/// reduce expressions to their simplest form. For advanced simplification control,
/// use the [`Simplify`] builder which offers domain safety and iteration limits.
///
/// # Arguments
/// * `formula` - Mathematical expression to simplify (e.g., "x + x + sin(x)^2 + cos(x)^2")
/// * `known_symbols` - Multi-character symbols for parsing (e.g., `&["alpha", "beta"]`)
/// * `custom_functions` - User-defined function names (e.g., `Some(&["f", "g"])`)
///
/// # Returns
/// The simplified expression as a string, or an error if parsing fails
///
/// # Errors
/// Returns `DiffError` if:
/// - **Syntax error**: Formula cannot be parsed
/// - **Complexity limits**: Expression exceeds safety limits (rare)
///
/// # Examples
///
/// ## Algebraic simplification
/// ```
/// # use symb_anafis::simplify;
///
/// // Like terms
/// let result = simplify("x + x + x", &[], None)?;
/// assert_eq!(result, "3*x");
///
/// // Polynomial expansion
/// let result = simplify("(x + 1)^2", &[], None)?;
/// assert_eq!(result, "(1 + x)^2"); // May not expand automatically
///
/// // Fraction reduction
/// let result = simplify("(x^2 - 1)/(x - 1)", &[], None)?;
/// // Complex expression - may not simplify without domain assumptions
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// ## Trigonometric identities
/// ```
/// # use symb_anafis::simplify;
///
/// // Pythagorean identity
/// let result = simplify("sin(x)^2 + cos(x)^2", &[], None)?;
/// assert_eq!(result, "1");
///
/// // Double angle
/// let result = simplify("2*sin(x)*cos(x)", &[], None)?;
/// assert_eq!(result, "sin(2*x)");
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// ## Exponential and logarithmic
/// ```
/// use symb_anafis::simplify;
///
/// // Log properties
/// let result = simplify("ln(e^x)", &[], None)?;
/// assert_eq!(result, "x");
///
/// // Exponential properties
/// let result = simplify("e^(ln(x))", &[], None)?;
/// assert_eq!(result, "x");
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// ## Multi-character symbols
/// ```
/// # use symb_anafis::simplify;
///
/// let result = simplify("alpha + alpha + beta", &["alpha", "beta"], None)?;
/// assert_eq!(result, "beta + (2*alpha)"); // Order may vary
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// # Simplification Categories
///
/// | Category | Rules Applied | Example |
/// |----------|---------------|----------|
/// | **Algebraic** | Like terms, polynomial operations | `x + 2*x` → `3*x` |
/// | **Trigonometric** | Pythagorean, double angle, etc. | `sin²(x) + cos²(x)` → `1` |
/// | **Exponential** | Log/exp inverses, power rules | `ln(e^x)` → `x` |
/// | **Rational** | Common factors, fraction reduction | `x²/(x*y)` → `x/y` |
/// | **Constants** | Arithmetic with numbers | `2 + 3*x + 1` → `3 + 3*x` |
///
/// # Performance and Limits
/// Default safety limits prevent infinite simplification loops:
/// - **Max depth**: 100 (default)
/// - **Max nodes**: 10,000 (default)
/// - **Max iterations**: 1000 simplification passes
///
/// For complex expressions, use the [`Simplify`] builder:
/// ```
/// use symb_anafis::Simplify;
///
/// let result = Simplify::new()
///     .domain_safe(true)  // Avoid division by zero transformations
///     .simplify_str("x + x", &[])?;
/// assert_eq!(result, "2*x");
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// # Domain Safety
/// By default, simplification may apply transformations that change the expression's domain.
/// For domain-preserving simplification, use the builder:
/// ```
/// use symb_anafis::Simplify;
///
/// // This avoids simplifying x^2/x → x (which is undefined at x=0)
/// let result = Simplify::new()
///     .domain_safe(true)
///     .simplify_str("x^2/x", &[])?;
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// # See Also
/// - [`Simplify`]: Builder pattern for advanced simplification control
/// - [`diff`]: Differentiation with automatic simplification
/// - [`Diff::skip_simplification`]: Raw derivatives without simplification
pub fn simplify(
    formula: &str,
    known_symbols: &[&str],
    custom_functions: Option<&[&str]>,
) -> Result<String, DiffError> {
    let mut builder = Simplify::new();

    if let Some(funcs) = custom_functions {
        builder = funcs
            .iter()
            .fold(builder, |b, f| b.user_fn(*f, UserFunction::any_arity()));
    }

    builder
        .max_depth(DEFAULT_MAX_DEPTH)
        .max_nodes(DEFAULT_MAX_NODES)
        .simplify_str(formula, known_symbols)
}

// ============================================================================
// Internal Re-exports (for crate use only)
// ============================================================================

// ExprKind is NOT re-exported at the crate root to encourage use of Expr constructors
// (Expr::sum, Expr::product, etc.) instead of direct ExprKind construction.
// IMPORTANT: ExprKind is currently NOT accessible from outside this crate because
// the `core` module is private. This means users cannot pattern match on ExprKind
// variants or write custom formatters/backparsers. See issue for discussion and for future plans to expose ExprKind in a controlled way if needed:
// This pub(crate) re-export is only for internal crate usage.
pub(crate) use core::ExprKind;
