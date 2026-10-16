//! User-function arity boundaries.

use alloc::sync::Arc;

use super::super::{Expr, UserFunction};

#[test]
fn zero_and_empty_arities_reject_partial_indices() {
    assert!(
        UserFunction::new(0..=0)
            .partial(0, Arc::new(|_args| Expr::number(0)))
            .is_err()
    );
    assert!(
        UserFunction::new(core::ops::RangeInclusive::new(usize::MAX, 0))
            .partial(0, Arc::new(|_args| Expr::number(0)))
            .is_err()
    );
    assert!(
        UserFunction::default()
            .partial(usize::MAX, Arc::new(|_args| Expr::number(0)))
            .is_err()
    );
}
