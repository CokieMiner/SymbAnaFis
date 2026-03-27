use super::Rule;
use super::angles::{
    CosDoubleAngleDifferenceRule, SinProductToDoubleAngleRule, TrigProductToDoubleAngleRule,
    TrigSumDifferenceRule,
};
use super::basic::{
    CosPiOverTwoRule, CosPiRule, CosSinToCotRule, CosZeroRule, OneCosToSecRule, OneSinToCscRule,
    SinCosToTanRule, SinPiOverTwoRule, SinPiRule, SinZeroRule, TanZeroRule, TrigExactValuesRule,
};
use super::identities::{
    PythagoreanComplementsRule, PythagoreanIdentityRule, PythagoreanTangentRule,
};
use super::inverse::{InverseTrigCompositionRule, InverseTrigIdentityRule};
use super::transformations::{
    CofunctionIdentityRule, TrigNegArgRule, TrigPeriodicityRule, TrigReflectionRule,
    TrigThreePiOverTwoRule,
};
use super::triple_angle::TrigTripleAngleRule;
use std::sync::Arc;

/// Get all trigonometric rules in priority order
pub fn get_trigonometric_rules() -> Vec<Arc<dyn Rule + Send + Sync>> {
    vec![
        // Basic rules: special values and constants
        Arc::new(SinZeroRule),
        Arc::new(CosZeroRule),
        Arc::new(TanZeroRule),
        Arc::new(SinPiRule),
        Arc::new(CosPiRule),
        Arc::new(SinPiOverTwoRule),
        Arc::new(CosPiOverTwoRule),
        Arc::new(TrigExactValuesRule),
        // Pythagorean and complementary identities
        Arc::new(PythagoreanIdentityRule),
        Arc::new(PythagoreanComplementsRule),
        Arc::new(PythagoreanTangentRule),
        // Inverse trig functions
        Arc::new(InverseTrigIdentityRule),
        Arc::new(InverseTrigCompositionRule),
        // Cofunction, periodicity, reflection, and negation
        Arc::new(CofunctionIdentityRule),
        Arc::new(TrigPeriodicityRule),
        Arc::new(TrigReflectionRule),
        Arc::new(TrigThreePiOverTwoRule),
        Arc::new(TrigNegArgRule),
        // Angle-based: double angle, sum/difference, product-to-sum
        Arc::new(CosDoubleAngleDifferenceRule),
        Arc::new(TrigSumDifferenceRule),
        Arc::new(TrigProductToDoubleAngleRule),
        Arc::new(SinProductToDoubleAngleRule),
        // Triple angle formulas
        Arc::new(TrigTripleAngleRule),
        // Ratio rules: convert fractions to canonical trig functions
        Arc::new(OneCosToSecRule),
        Arc::new(OneSinToCscRule),
        Arc::new(SinCosToTanRule),
        Arc::new(CosSinToCotRule),
    ]
}
