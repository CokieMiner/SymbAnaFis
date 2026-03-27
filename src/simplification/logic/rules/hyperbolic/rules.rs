use super::Rule;
use super::conversions::{
    CoshFromExpRule, CothFromExpRule, CschFromExpRule, SechFromExpRule, SinhFromExpRule,
    TanhFromExpRule,
};
use super::identities::{
    CoshAcoshIdentityRule, CoshNegationRule, CoshZeroRule, HyperbolicIdentityRule,
    HyperbolicTripleAngleRule, SinhAsinhIdentityRule, SinhNegationRule, SinhZeroRule,
    TanhAtanhIdentityRule, TanhNegationRule,
};
use super::ratios::{
    CoshSinhToCothRule, OneCoshToSechRule, OneSinhToCschRule, OneTanhToCothRule, SinhCoshToTanhRule,
};
use std::sync::Arc;

/// Get all hyperbolic rules in priority order
pub fn get_hyperbolic_rules() -> Vec<Arc<dyn Rule + Send + Sync>> {
    vec![
        // High priority rules first
        Arc::new(SinhZeroRule),
        Arc::new(CoshZeroRule),
        Arc::new(SinhAsinhIdentityRule),
        Arc::new(CoshAcoshIdentityRule),
        Arc::new(TanhAtanhIdentityRule),
        Arc::new(SinhNegationRule),
        Arc::new(CoshNegationRule),
        Arc::new(TanhNegationRule),
        // Identity rules
        Arc::new(HyperbolicIdentityRule),
        // Ratio rules - convert to tanh, coth, sech, csch
        Arc::new(SinhCoshToTanhRule),
        Arc::new(CoshSinhToCothRule),
        Arc::new(OneCoshToSechRule),
        Arc::new(OneSinhToCschRule),
        Arc::new(OneTanhToCothRule),
        // Conversion from exponential forms
        Arc::new(SinhFromExpRule),
        Arc::new(CoshFromExpRule),
        Arc::new(TanhFromExpRule),
        Arc::new(SechFromExpRule),
        Arc::new(CschFromExpRule),
        Arc::new(CothFromExpRule),
        Arc::new(HyperbolicTripleAngleRule),
    ]
}
