use super::super::core::Rule;
use super::{conversions, identities, ratios};
use std::sync::Arc;

/// Get all hyperbolic rules in priority order
pub fn get_hyperbolic_rules() -> Vec<Arc<dyn Rule + Send + Sync>> {
    vec![
        // High priority rules first
        Arc::new(identities::SinhZeroRule),
        Arc::new(identities::CoshZeroRule),
        Arc::new(identities::SinhAsinhIdentityRule),
        Arc::new(identities::CoshAcoshIdentityRule),
        Arc::new(identities::TanhAtanhIdentityRule),
        Arc::new(identities::SinhNegationRule),
        Arc::new(identities::CoshNegationRule),
        Arc::new(identities::TanhNegationRule),
        // Identity rules
        Arc::new(identities::HyperbolicIdentityRule),
        // Ratio rules - convert to tanh, coth, sech, csch
        Arc::new(ratios::SinhCoshToTanhRule),
        Arc::new(ratios::CoshSinhToCothRule),
        Arc::new(ratios::OneCoshToSechRule),
        Arc::new(ratios::OneSinhToCschRule),
        Arc::new(ratios::OneTanhToCothRule),
        // Conversion from exponential forms
        Arc::new(conversions::SinhFromExpRule),
        Arc::new(conversions::CoshFromExpRule),
        Arc::new(conversions::TanhFromExpRule),
        Arc::new(conversions::SechFromExpRule),
        Arc::new(conversions::CschFromExpRule),
        Arc::new(conversions::CothFromExpRule),
        Arc::new(identities::HyperbolicTripleAngleRule),
    ]
}
