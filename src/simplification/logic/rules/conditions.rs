//! Conservative capture predicates for builtin identities.

use alloc::sync::Arc;

use crate::core::{Expr, Pattern, Property};

use super::PreparedStep;

impl PreparedStep<'_> {
    pub fn capture(name: &str, property: Property) -> Pattern {
        Pattern::capture(
            name,
            Some(Arc::new(move |node| Self::proved(node, property))),
        )
    }

    pub fn conditional(
        name: &str,
        predicate: impl Fn(&Expr) -> bool + Send + Sync + 'static,
    ) -> Pattern {
        Pattern::capture(name, Some(Arc::new(predicate)))
    }
}
