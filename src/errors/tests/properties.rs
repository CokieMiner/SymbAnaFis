//! Source-span bounds and displayed positions.

use alloc::{format, string::ToString};

use proptest::prelude::{any, prop_assert, prop_assert_eq, proptest};

use super::Span;

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
        prop_assert_eq!(span.to_string(), format!(" at position {end}"));
    }
}
