//! Python bindings for `symb_anafis` using `PyO3`
//!
//! # Installation
//! ```bash
//! pip install symb_anafis
//! ```
//!
//! # Quick Start
//! ```python
//! from symb_anafis import Expr, Diff, Simplify, diff, simplify
//!
//! # Create symbolic expressions
//! x = Expr("x")
//! y = Expr("y")
//! expr = x ** 2 + y
//!
//! # Differentiate
//! result = diff("x^2 + sin(x)", "x")
//! print(result)  # "2*x + cos(x)"
//!
//! # Use builder API for more control
//! d = Diff().fixed_var("a").domain_safe(True)
//! result = d.diff_str("x^2 + a*x", "x")  # "2*x + a"
//!
//! # Simplify expressions
//! result = simplify("x + x + 0")  # "2*x"
//! ```
//!
//! # Available Classes
//! - `Expr` - Symbolic expression wrapper
//! - `Diff` - Differentiation builder
//! - `Simplify` - Simplification builder
//! - `Context` - Isolated symbol and function registry
//!
//! # Available Functions
//! - `diff(formula, var, known_symbols?, custom_functions?)` - Differentiate string formula
//! - `simplify(formula, known_symbols?, custom_functions?)` - Simplify string formula
//! - `parse(formula, known_symbols?, custom_functions?)` - Parse formula to string

// Submodules
mod api;
mod builder;
mod context;
mod dual;
mod error;
mod evaluator;
mod expr;
mod functions;
mod parallel;
mod symbol;
mod utilities;
mod visitor;

// Re-exports
pub use builder::*;
pub use context::*;
pub use dual::*;
pub use evaluator::*;
pub use expr::*;
pub use functions::*;
#[cfg(feature = "parallel")]
pub use parallel::*;
pub use symbol::*;
pub use utilities::*;
pub use visitor::*;
