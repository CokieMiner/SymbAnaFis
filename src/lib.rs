//! Symbolic mathematics: expressions, calculus, rewriting, and evaluation.
//!
//! - [`Expr`] is an immutable expression. Equal expressions share one node, so
//!   cloning, equality, and hashing take constant time.
//! - [`Context`] owns a namespace of symbols and functions and parses text with
//!   [`Context::parse`].
//! - [`Expr::derivative`], [`Expr::gradient`], [`Expr::jacobian`], and
//!   [`Expr::hessian`] differentiate symbolically.
//! - [`Substitution`], [`RewriteRule`], and [`SimplificationSequence`]
//!   transform expressions.
//! - [`Expr::evaluate`] evaluates once; [`Program`] compiles a reusable register
//!   program, and each thread evaluates it with its own [`Workspace`].
//! - [`Uncertainty`] propagates first-order uncertainties through a model.
//!
//! Literals are `NumAnaFis` [`Number`]s. Integers and rationals stay exact; an
//! exact result that does not fit the numeric backend keeps its operation
//! symbolic instead of overflowing.
//!
//! # Features
//!
//! The default build is `no_std` with `alloc`; the target needs a global
//! allocator and pointer-width atomics. `std` selects blocking registry locks and
//! the `NumAnaFis` `std` backend. `num-traits` implements `Zero` and `One` for
//! [`Expr`]. Without `std`, registries use spin locks, so an interrupt handler
//! must not use the library while it interrupts a lock holder.
//!
//! ```rust
//! use symb_anafis::{ExprView, Symbol};
//!
//! let x = Symbol::intern("core_example_x")?;
//! let expression = x.sin() + x.pow(2);
//! assert!(expression.contains_var("core_example_x"));
//! assert!(matches!(expression.view(), ExprView::Sum(_)));
//! # Ok::<(), symb_anafis::SymbolError>(())
//! ```
//!
//! Registered functions work in expressions and rewrite patterns:
//!
//! ```
//! use symb_anafis::{Context, Expr, Pattern, RewriteRule, UserFunction};
//! let context = Context::new()?;
//! let f = context.register_function("f", UserFunction::new(1..=1))?;
//! let x = context.symb("x")?;
//! let a = Pattern::capture("a", None);
//! let rule = RewriteRule::new(
//!     Pattern::apply(f.clone(), vec![a.sqrt()])?,
//!     a.exp(),
//! )?;
//! let input = f.call(vec![x.to_expr().sqrt()])?;
//! assert_eq!(rule.apply_all(&input, usize::MAX, None)?, x.exp());
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
mod polynomial;
mod rewrite;
mod simplification;
mod uncertainty;

pub use collections::{HashMap, HashSet};
pub use core::{
    Context, Expr, ExprChildren, ExprView, Function, Properties, Property, Substitution,
    Substitutions, Symbol, Truth, UserFunction,
};
pub use diff::Jacobian;
pub use errors::{
    EvalError, FunctionError, ParseError, RewriteError, SimplifyError, Span, SubstitutionError,
    SymbolError, UncertaintyError,
};
pub use evaluator::{EvaluationValue, Program, Workspace};
pub use num_anafis::{Complex, Number, Numeric, Real};
pub use rewrite::{Pattern, Replacement, RewriteRule};
pub use simplification::{
    DomainSafety, Simplification, SimplificationSequence, SimplificationStep, SimplificationTarget,
    SimplificationTermination,
};
pub use uncertainty::{CovarianceMatrix, Uncertainty};
