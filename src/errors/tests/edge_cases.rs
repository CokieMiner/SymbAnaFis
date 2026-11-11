//! Empty locations and the exclusive-end capacity limit.

use alloc::string::ToString;

use super::Span;

#[test]
fn empty_locations_and_unrepresentable_exclusive_ends_are_explicit() {
    assert!(!Span::empty().is_valid(), "an empty span has no location");
    assert_eq!(
        Span::empty().to_string(),
        "",
        "unknown locations add no suffix"
    );
    assert_eq!(
        Span::at(usize::MAX),
        Span::new(usize::MAX, usize::MAX),
        "usize::MAX has no representable exclusive end, so its span is empty"
    );
}
