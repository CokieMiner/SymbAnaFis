//! Source locations used by textual diagnostics.

use alloc::{format, string::String};

/// A half-open range of byte offsets in a source string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    /// Format nonempty spans with one-based byte positions.
    #[must_use]
    pub fn display(&self) -> String {
        if !self.is_valid() {
            String::new()
        } else if self.end.checked_sub(self.start) == Some(1) {
            // A valid span proves start < end, so the increment fits usize.
            format!(" at position {}", self.start.saturating_add(1))
        } else {
            format!(
                " at positions {}-{}",
                self.start.saturating_add(1),
                self.end
            )
        }
    }

    /// Create an ordered span, swapping the endpoints when needed.
    #[inline]
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        if end < start {
            Self {
                start: end,
                end: start,
            }
        } else {
            Self { start, end }
        }
    }

    /// Create a span covering one byte.
    ///
    /// # Panics
    /// Panics at `usize::MAX`, whose exclusive end is unrepresentable.
    #[inline]
    #[must_use]
    pub const fn at(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos
                .checked_add(1)
                .expect("a single-position span needs a representable exclusive end"),
        }
    }

    /// Create an empty span without source information.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    /// Get the inclusive start byte offset.
    #[inline]
    #[must_use]
    pub const fn start(&self) -> usize {
        self.start
    }

    /// Get the exclusive end byte offset.
    #[inline]
    #[must_use]
    pub const fn end(&self) -> usize {
        self.end
    }

    /// Test whether the span covers at least one byte.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.end > self.start
    }
}
