use crate::simplification::rules::Rule;
use std::sync::Arc;

pub mod conversions;
mod helpers;
pub mod identities;
pub mod ratios;

pub use conversions::*;
pub use identities::*;
pub use ratios::*;

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
