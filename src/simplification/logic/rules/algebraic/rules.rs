use super::Rule;
use super::abs_sign::{
    AbsAbsRule, AbsNegRule, AbsNumericRule, AbsPowEvenRule, AbsSignMulRule, AbsSquareRule,
    SignAbsRule, SignNumericRule, SignSignRule,
};
use super::canonicalization::{
    CanonicalizeProductRule, CanonicalizeSumRule, SimplifyNegativeProductRule,
};
use super::combination::{
    CombineFactorsRule, CombineLikeTermsInSumRule, CombineTermsRule, ProductDivCombinationRule,
};
use super::expansion::{ExpandPowerForCancellationRule, PowerExpansionRule};
use super::factoring::{
    CommonPowerFactoringRule, CommonTermFactoringRule, FactorDifferenceOfSquaresRule,
    FractionCancellationRule, NumericGcdFactoringRule, PerfectCubeRule, PerfectSquareRule,
    PolyGcdSimplifyRule,
};
use super::fractions::{
    AddFractionRule, CombineNestedFractionRule, DivDivRule, DivSelfRule, FractionToEndRule,
};
use super::identities::{EPowLnRule, EPowMulLnRule, ExpLnRule, ExpMulLnRule, LnExpRule};
use super::powers::{
    CommonExponentDivRule, CommonExponentProductRule, NegativeExponentToFractionRule,
    PowerCollectionRule, PowerDivRule, PowerOfQuotientRule, PowerOneRule, PowerPowerRule,
    PowerProductRule, PowerZeroRule,
};
use std::sync::Arc;

/// Get all algebraic rules in priority order
pub fn get_algebraic_rules() -> Vec<Arc<dyn Rule + Send + Sync>> {
    vec![
        // Exponential/logarithmic identities
        Arc::new(ExpLnRule),
        Arc::new(LnExpRule),
        Arc::new(ExpMulLnRule),
        Arc::new(EPowLnRule),
        Arc::new(EPowMulLnRule),
        // Power rules
        Arc::new(PowerZeroRule),
        Arc::new(PowerOneRule),
        Arc::new(PowerPowerRule),
        Arc::new(PowerProductRule),
        Arc::new(PowerDivRule),
        Arc::new(PowerCollectionRule),
        Arc::new(CommonExponentDivRule),
        Arc::new(CommonExponentProductRule),
        Arc::new(NegativeExponentToFractionRule),
        Arc::new(PowerOfQuotientRule), // (a/b)^n -> a^n / b^n
        // Fraction rules
        Arc::new(DivSelfRule),
        Arc::new(DivDivRule),
        Arc::new(CombineNestedFractionRule),
        Arc::new(AddFractionRule),
        Arc::new(FractionToEndRule),
        // Absolute value and sign rules
        Arc::new(AbsNumericRule),
        Arc::new(SignNumericRule),
        Arc::new(AbsAbsRule),
        Arc::new(AbsNegRule),
        Arc::new(AbsSquareRule),
        Arc::new(AbsPowEvenRule),
        Arc::new(SignSignRule),
        Arc::new(SignAbsRule),
        Arc::new(AbsSignMulRule),
        // Expansion rules
        Arc::new(ExpandPowerForCancellationRule),
        Arc::new(PowerExpansionRule),
        // Factoring rules
        Arc::new(FractionCancellationRule),
        Arc::new(PerfectSquareRule),
        Arc::new(FactorDifferenceOfSquaresRule),
        Arc::new(PerfectCubeRule),
        Arc::new(NumericGcdFactoringRule),
        Arc::new(CommonTermFactoringRule),
        Arc::new(CommonPowerFactoringRule),
        Arc::new(PolyGcdSimplifyRule),
        // Canonicalization rules (simplified for n-ary)
        Arc::new(CanonicalizeProductRule),
        Arc::new(CanonicalizeSumRule),
        Arc::new(SimplifyNegativeProductRule),
        // Combination rules
        Arc::new(ProductDivCombinationRule),
        Arc::new(CombineTermsRule),
        Arc::new(CombineFactorsRule),
        Arc::new(CombineLikeTermsInSumRule),
    ]
}
