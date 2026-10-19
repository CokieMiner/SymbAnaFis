//! User-defined symbolic bodies and partial derivatives.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]

use core::ops::RangeInclusive;

use alloc::{sync::Arc, vec::Vec};

use super::{Expr, FunctionError};

/// A symbolic body operating directly on shared expression handles.
type BodyFn = Arc<dyn Fn(&[Expr]) -> Expr + Send + Sync>;

/// A custom definition with an arity, optional body, and selected partial derivatives.
///
/// Builder methods consume the draft. Registration moves the definition into
/// a shared immutable function identity; handles and expressions retain it after
/// removal or context destruction. Cloning a definition creates an independent
/// draft sharing callback allocations.
/// Callbacks return deterministic expressions for unchanged arguments.
/// Differentiation memoizes shared calls and can skip repeated higher-order
/// states; callback side effects do not define mathematical behavior.
/// Capturing a clone of the context storing its function handle can form an
/// ownership cycle; remove its bindings before dropping external owners, or
/// capture a weak owner.
///
/// ```
/// use std::sync::Arc;
/// use symb_anafis::{Context, Expr, UserFunction};
/// let context = Context::new();
/// let definition = UserFunction::new(2..=2)
///     .body(Arc::new(|args| Expr::sum(args.to_vec())))
///     .partial(0, Arc::new(|_| Expr::number(1)))?
///     .partial(1, Arc::new(|_| Expr::number(1)))?;
/// let function = context.register_function("add", definition)?;
/// let x = context.symb("x")?;
/// let y = context.symb("y")?;
/// let expression = function.call(vec![x.into(), y.into()])?;
/// assert!(expression.contains_symbol(x));
/// # Ok::<(), Box<dyn core::error::Error>>(())
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
    partials: Vec<(usize, BodyFn)>,
}

mod methods;

#[cfg(test)]
mod tests;
