//! Symbolic expressions, differentiation, and numerical evaluation for `SymbAnaFis`.
//!
//! Canonical expressions support scoped symbols, registered functions, scalar
//! property analysis, differentiation, rewriting, simultaneous substitutions,
//! simplification sequences, and text parsing through [`Context::parse`].
//! [`Expr::gradient`], [`Expr::jacobian`], and [`Expr::hessian`] expose ordered
//! multivariable derivatives. [`Uncertainty`] propagates real first-order
//! covariance models with explicit caller-owned mathematical assumptions.
//! [`Expr::evaluate`] computes individual values directly; [`Evaluator`] compiles
//! shared scalar register programs for repeated numerical evaluation.
//! Literals and coefficients use `NumAnaFis`'s
//! [`Number`], retaining exact integers, rationals, and complex components.
//!
//! The default build uses `no_std + alloc`. Targets must provide a global
//! allocator and pointer-width atomics for shared expression ownership.
//! The opt-in `std` feature enables `NumAnaFis`'s `std` backend and blocking
//! registry locks; `num-traits` enables expression traits independently.
//! Without `std`, shared registries use internal spin locks. Their owner must
//! be able to run. Reentry from an interrupt or signal that suspends an owner
//! is unsupported.
//!
//! ```rust
//! use symb_anafis::{ExprView, Symbol};
//!
//! let x = Symbol::intern("core_example_x");
//! let expression = x.sin() + x.pow(2);
//! assert!(expression.contains_var("core_example_x"));
//! assert!(matches!(expression.view(), ExprView::Sum(_)));
//! ```
//!
//! Custom calls and templates use registered function handles:
//!
//! ```
//! use symb_anafis::{Context, Expr, Pattern, RewriteRule, UserFunction};
//! let context = Context::new();
//! let f = context.register_function("f", UserFunction::new(1..=1))?;
//! let x = context.symb("x")?;
//! let a = Pattern::capture("a", None);
//! let rule = RewriteRule::new(
//!     Pattern::apply(f.clone(), vec![a.sqrt()])?,
//!     a.exp(),
//! )?;
//! let input = f.call(vec![x.to_expr().sqrt()])?;
//! assert_eq!(rule.apply_all(&input, None, None)?, x.exp());
//! assert_eq!(Expr::derivative(x.sin(), x, 1).symbols().len(), 1);
//! # Ok::<(), Box<dyn core::error::Error>>(())
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
extern crate std;

mod collections;
mod core;
mod diff;
mod errors;
mod evaluator;
mod parser;
mod simplification;
mod uncertainty;

pub use collections::{HashMap, HashSet};
pub use core::{
    Context, Expr, ExprChildren, ExprView, Function, Pattern, Properties, Property, Replacement,
    RewriteRule, Substitution, Substitutions, Symbol, Truth, UserFunction,
};
pub use errors::{
    DiffError, EvalError, FunctionError, ParseError, RewriteError, SimplifyError, Span,
    SubstitutionError, SymbolError, UncertaintyError,
};
pub use evaluator::{EvaluationValue, Evaluator};
pub use num_anafis::{Complex, Number, Numeric, Real};
pub use simplification::{
    DomainSafety, Simplification, SimplificationSequence, SimplificationStep, SimplificationTarget,
    SimplificationTermination,
};
pub use uncertainty::{CovarianceMatrix, Uncertainty};
