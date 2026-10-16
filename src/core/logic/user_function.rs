//! User-defined symbolic bodies and partial derivatives.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]

use core::ops::RangeInclusive;

use alloc::{sync::Arc, vec::Vec};

use super::{Expr, FunctionError};

/// A symbolic body operating directly on shared expression handles.
pub type BodyFn = Arc<dyn Fn(&[Expr]) -> Expr + Send + Sync>;

/// A symbolic partial derivative operating on shared expression handles.
pub type PartialFn = BodyFn;

/// An arity contract with an optional body and selected partial derivatives.
///
/// Registered callbacks retain their captured owners until the definition is
/// released. Capturing a clone of the context storing this definition can form
/// an ownership cycle; remove its definitions before dropping external owners,
/// or capture a weak owner.
///
/// ```
/// use std::sync::Arc;
/// use symb_anafis::{Context, Expr, FunctionError, UserFunction};
/// let context = Context::new();
/// let definition = UserFunction::new(2..=2)
///     .body(Arc::new(|args| Expr::sum(args.to_vec())))
///     .partial(0, Arc::new(|_| Expr::number(1)))?
///     .partial(1, Arc::new(|_| Expr::number(1)))?;
/// let function = context.register_function("add", definition)?;
/// let x = context.symb("x");
/// let y = context.symb("y");
/// let expression = function.call(vec![x.into(), y.into()])?;
/// assert!(expression.contains_symbol(x));
/// # Ok::<(), FunctionError>(())
/// ```
#[derive(Clone)]
pub struct UserFunction {
    arity: RangeInclusive<usize>,
    body: Option<BodyFn>,
    /// Partial callbacks sorted by their unique argument index.
    ///
    /// Partial derivative sets are normally small. Keeping them sparse avoids
    /// allocating a dense table for a large argument index and removes a
    /// standard-library map from this core type.
    partials: Vec<(usize, PartialFn)>,
}

mod methods;

#[cfg(test)]
mod tests;
