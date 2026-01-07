use crate::simplification::rules::Rule;
use std::sync::Arc;

pub mod abs_sign;
pub mod canonicalization;
pub mod combination;
pub mod expansion;
pub mod factoring;
pub mod fractions;
/// Algebraic simplification rules
pub mod identities;
pub mod powers;

/// Get all algebraic rules in priority order
pub fn get_algebraic_rules() -> Vec<Arc<dyn Rule + Send + Sync>> {
    vec![
        // Exponential/logarithmic identities
        Arc::new(identities::ExpLnRule),
        Arc::new(identities::LnExpRule),
        Arc::new(identities::ExpMulLnRule),
        Arc::new(identities::EPowLnRule),
        Arc::new(identities::EPowMulLnRule),
        // Power rules
        Arc::new(powers::PowerZeroRule),
        Arc::new(powers::PowerOneRule),
        Arc::new(powers::PowerPowerRule),
        Arc::new(powers::PowerProductRule),
        Arc::new(powers::PowerDivRule),
        Arc::new(powers::PowerCollectionRule),
        Arc::new(powers::CommonExponentDivRule),
        Arc::new(powers::CommonExponentProductRule),
        Arc::new(powers::NegativeExponentToFractionRule),
        Arc::new(powers::PowerOfQuotientRule), // (a/b)^n -> a^n / b^n
        // Fraction rules
        Arc::new(fractions::DivSelfRule),
        Arc::new(fractions::DivDivRule),
        Arc::new(fractions::CombineNestedFractionRule),
        Arc::new(fractions::AddFractionRule),
        Arc::new(fractions::FractionToEndRule),
        // Absolute value and sign rules
        Arc::new(abs_sign::AbsNumericRule),
        Arc::new(abs_sign::SignNumericRule),
        Arc::new(abs_sign::AbsAbsRule),
        Arc::new(abs_sign::AbsNegRule),
        Arc::new(abs_sign::AbsSquareRule),
        Arc::new(abs_sign::AbsPowEvenRule),
        Arc::new(abs_sign::SignSignRule),
        Arc::new(abs_sign::SignAbsRule),
        Arc::new(abs_sign::AbsSignMulRule),
        // Expansion rules
        Arc::new(expansion::ExpandPowerForCancellationRule),
        Arc::new(expansion::PowerExpansionRule),
        // Factoring rules
        Arc::new(factoring::FractionCancellationRule),
        Arc::new(factoring::PerfectSquareRule),
        Arc::new(factoring::FactorDifferenceOfSquaresRule),
        Arc::new(factoring::PerfectCubeRule),
        Arc::new(factoring::NumericGcdFactoringRule),
        Arc::new(factoring::CommonTermFactoringRule),
        Arc::new(factoring::CommonPowerFactoringRule),
        Arc::new(factoring::PolyGcdSimplifyRule),
        // Canonicalization rules (simplified for n-ary)
        Arc::new(canonicalization::CanonicalizeProductRule),
        Arc::new(canonicalization::CanonicalizeSumRule),
        Arc::new(canonicalization::SimplifyNegativeProductRule),
        // Combination rules
        Arc::new(combination::ProductDivCombinationRule),
        Arc::new(combination::CombineTermsRule),
        Arc::new(combination::CombineFactorsRule),
        Arc::new(combination::CombineLikeTermsInSumRule),
    ]
}
