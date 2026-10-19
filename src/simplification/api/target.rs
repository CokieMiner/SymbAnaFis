//! Builtin lexicographic objectives for candidate selection.

/// The desired form among candidates visited by a sequence.
///
/// Presets pair a target with suitable passes; selecting a target alone does
/// not add passes or perform a global search over equivalent expressions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SimplificationTarget {
    /// Minimize logical expression size, counting repeated edges.
    #[default]
    Compact,
    /// First minimize distributable products and positive integer powers of sums.
    Expanded,
    /// First minimize composite polynomial terms under sums, then logical size.
    Factored,
    /// Minimize estimated native operation cost over distinct logical DAG nodes.
    Compilation,
}
