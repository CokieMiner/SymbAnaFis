//! Source-span bounds and displayed error positions.

use alloc::format;

use proptest::prelude::{any, prop_assert, prop_assert_eq, proptest};

use super::{DiffError, Span};

proptest! {
    #[test]
    fn span_bounds_are_ordered_and_independent_of_input_order(
        start in any::<usize>(), end in any::<usize>(),
    ) {
        let span = Span::new(start, end);
        prop_assert_eq!(span.start(), start.min(end));
        prop_assert_eq!(span.end(), start.max(end));
        prop_assert_eq!(span.is_valid(), start != end);
        prop_assert_eq!(span, Span::new(end, start));
    }

    #[test]
    fn single_position_spans_have_one_byte_and_one_based_display(
        position in 0_usize..usize::MAX,
    ) {
        let end = position.checked_add(1).expect("the generated position is below usize::MAX");
        let span = Span::at(position);
        prop_assert_eq!(span.start(), position);
        prop_assert_eq!(span.end(), end);
        prop_assert!(span.is_valid(), "a one-byte span has a location");
        prop_assert_eq!(span.display(), format!(" at position {end}"));
    }

    #[test]
    fn syntax_errors_preserve_generated_messages_and_source_ranges(
        message in "[a-z ]{1,32}", start in 0_usize..4096, length in 2_usize..128,
    ) {
        let end = start.checked_add(length).expect("the bounded span fits usize");
        let first = start.checked_add(1).expect("the bounded span start fits usize");
        let span = Span::new(start, end);
        let error = DiffError::invalid_syntax_at(&message, span);
        prop_assert_eq!(span.display(), format!(" at positions {first}-{end}"));
        prop_assert_eq!(format!("{error}"), format!("Invalid syntax: {message} at positions {first}-{end}"));
    }
}
