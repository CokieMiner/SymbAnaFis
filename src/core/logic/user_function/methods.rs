//! User function contracts.

use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    ops::RangeInclusive,
};

use alloc::{sync::Arc, vec::Vec};

use super::{BodyFn, FunctionError, PartialFn, UserFunction};

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
            .field("has_body", &self.has_body())
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

    /// Register a shared partial derivative for an argument index.
    ///
    /// # Errors
    /// Returns `FunctionError::InvalidPartialIndex` when no accepted arity contains the index.
    pub fn partial(mut self, index: usize, partial: PartialFn) -> Result<Self, FunctionError> {
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

    /// Test whether a body is registered.
    #[must_use]
    pub const fn has_body(&self) -> bool {
        self.body.is_some()
    }

    /// Test whether a partial is registered for an index.
    #[must_use]
    pub fn has_partial(&self, index: usize) -> bool {
        self.partials
            .binary_search_by_key(&index, |(stored, _)| *stored)
            .is_ok()
    }

    /// Test whether the argument count is accepted.
    #[must_use]
    pub fn accepts_arity(&self, count: usize) -> bool {
        self.arity.contains(&count)
    }

    /// Return the range of accepted argument counts.
    #[must_use]
    pub fn arity(&self) -> RangeInclusive<usize> {
        self.arity.clone()
    }

    /// Share the registered body.
    #[must_use]
    pub fn body_fn(&self) -> Option<BodyFn> {
        self.body.as_ref().map(Arc::clone)
    }

    /// Share a registered partial derivative.
    #[must_use]
    pub fn partial_fn(&self, index: usize) -> Option<PartialFn> {
        self.partials
            .binary_search_by_key(&index, |(stored, _)| *stored)
            .ok()
            .and_then(|position| self.partials.get(position))
            .map(|(_, partial)| Arc::clone(partial))
    }
}
