//! Empty locations and the exclusive-end capacity limit.

use std::panic::catch_unwind;

use alloc::format;

use super::{DiffError, Span};

#[test]
fn empty_locations_and_unrepresentable_exclusive_ends_are_explicit() {
    assert!(!Span::empty().is_valid(), "an empty span has no location");
    assert_eq!(
        Span::empty().display(),
        "",
        "unknown locations add no suffix"
    );
    assert_eq!(
        format!("{}", DiffError::EmptyFormula),
        "Formula cannot be empty",
        "the empty-input error has a fixed message"
    );
    assert!(
        catch_unwind(|| Span::at(usize::MAX)).is_err(),
        "usize::MAX has no representable exclusive end"
    );
}
