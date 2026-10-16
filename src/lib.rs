//! Symbolic expression core for `SymbAnaFis`.
//!
//! This development stage compiles only the core expression, symbol, context,
//! and error infrastructure. Literals and coefficients use `NumAnaFis`'s
//! [`Number`], retaining exact integers, rationals, and complex components.
//!
//! The default build uses `no_std + alloc`. Targets must provide a global
//! allocator and pointer-width atomics for shared expression ownership.
//! The opt-in `std` feature enables `NumAnaFis`'s `std` backend and blocking
//! registry locks; `num-traits` enables expression traits independently.
//! Without `std`, shared registries use internal spin locks. Their owner must
//! be able to run;
//! reentry from an interrupt or signal that suspends an owner is unsupported.
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
//! Parsing, differentiation, simplification, evaluation, convenience APIs,
//! uncertainty propagation, and Python bindings are parked during this stage.
//! Their source files remain available for incremental reintegration.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
extern crate std;

mod collections;
mod core;
mod errors;

pub use collections::{HashMap, HashSet};
pub use core::{
    BodyFn, Context, Expr, ExprChildren, ExprView, Function, PartialFn, Pattern, RewriteGuard,
    RewriteRule, Substitution, Substitutions, Symbol, UserFunction,
};
pub use errors::{DiffError, FunctionError, RewriteError, Span, SubstitutionError, SymbolError};
pub use num_anafis::{Complex, Number, Numeric, Real};
