//! User-defined symbolic bodies and partial derivatives.

use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::RangeInclusive,
};

use alloc::{sync::Arc, vec::Vec};

use super::{Expr, FunctionError};

/// A symbolic body over shared argument handles.
pub type BodyFn = Arc<dyn Fn(&[Expr]) -> Expr + Send + Sync>;

/// A custom function definition: an arity, an optional symbolic body, and
/// partial derivatives for selected arguments.
///
/// Builder methods consume the draft; a clone is an independent draft that
/// shares the callbacks. Registration moves the definition into an immutable
/// function identity, which handles and expressions keep after name removal
/// or context destruction.
///
/// Callbacks must return the same expression for the same arguments, because
/// differentiation memoizes shared calls and may skip repeated higher-order
/// states. A callback that captures a clone of the context holding its own
/// handle forms a reference cycle; remove the binding before dropping the
/// other owners, or capture a weak reference.
///
/// ```
/// use std::sync::Arc;
/// use symb_anafis::{Context, Expr, UserFunction};
/// let context = Context::new()?;
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
    /// Partial callbacks sorted by unique argument index. The sparse list
    /// avoids a dense table for a large index.
    partials: Vec<(usize, BodyFn)>,
}

impl Default for UserFunction {
    fn default() -> Self {
        Self::new(0..=usize::MAX)
    }
}

impl Debug for UserFunction {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("UserFunction")
            .field("arity", &self.arity)
            .field("has_body", &self.body.is_some())
            .field("partial_count", &self.partials.len())
            .finish_non_exhaustive()
    }
}

impl UserFunction {
    /// Create a function with the given accepted argument counts.
    #[must_use]
    pub fn new(arity: RangeInclusive<usize>) -> Self {
        Self {
            arity,
            body: None,
            partials: Vec::new(),
        }
    }

    /// Set a shared symbolic body without allocating another callback wrapper.
    #[must_use]
    pub fn body(mut self, body: BodyFn) -> Self {
        self.body = Some(body);
        self
    }

    /// Set a shared partial derivative for an argument index in this draft.
    /// Setting the same index replaces its callback. Registered function
    /// identities retain their immutable definitions.
    ///
    /// # Errors
    /// Returns `FunctionError::InvalidPartialIndex` when no accepted arity contains the index.
    pub fn partial(mut self, index: usize, partial: BodyFn) -> Result<Self, FunctionError> {
        let maximum = *self.arity.end();
        if self.arity.is_empty() || index >= maximum {
            return Err(FunctionError::InvalidPartialIndex {
                index,
                max_arity: maximum,
            });
        }
        match self
            .partials
            .binary_search_by_key(&index, |(stored, _)| *stored)
        {
            Ok(position) => {
                // The search borrowed this same vector before any mutation.
                if let Some((_, stored)) = self.partials.get_mut(position) {
                    *stored = partial;
                }
            }
            Err(position) => self.partials.insert(position, (index, partial)),
        }
        Ok(self)
    }

    /// Return the range of accepted argument counts.
    #[must_use]
    pub fn arity(&self) -> RangeInclusive<usize> {
        self.arity.clone()
    }

    /// Borrow the registered body, if any.
    #[must_use]
    pub const fn body_fn(&self) -> Option<&BodyFn> {
        self.body.as_ref()
    }

    /// Borrow the partial derivative registered for argument `index`, if any.
    #[must_use]
    pub fn partial_fn(&self, index: usize) -> Option<&BodyFn> {
        self.partials
            .binary_search_by_key(&index, |(stored, _)| *stored)
            .ok()
            .and_then(|position| self.partials.get(position))
            .map(|(_, partial)| partial)
    }
}
