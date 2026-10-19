//! Explicit reasons for returning a selected simplification candidate.

/// Why execution stopped. The returned expression is the best visited candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SimplificationTermination {
    /// Every step in the final sweep left its input unchanged, or the sequence is empty.
    FixedPoint,
    /// A complete sweep returned to a previously visited sweep boundary.
    Cycle,
    /// The configured number of complete sweeps was consumed.
    IterationLimit,
}
