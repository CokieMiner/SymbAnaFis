//! Domain contracts for builtin simplification rules.

/// Which builtin rule contracts a sequence may use.
///
/// Custom rewrite rules and mapping callbacks retain their caller-owned
/// correctness contract in both modes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomainSafety {
    /// Preserve poles and principal branches using conservative input proofs.
    #[default]
    Preserving,
    /// Also use identities whose domain and branch conditions the caller assumes.
    ///
    /// These rules may remove zero exclusions or require real inputs, positive
    /// bases, or a principal inverse-function range. Selecting this mode asserts
    /// those rule-specific hypotheses; it does not establish them.
    AssumptionDependent,
}

impl DomainSafety {
    /// Whether this mode admits a builtin rule carrying the supplied tag.
    #[must_use]
    pub const fn permits(self, rule: Self) -> bool {
        matches!(self, Self::AssumptionDependent) || matches!(rule, Self::Preserving)
    }
}
