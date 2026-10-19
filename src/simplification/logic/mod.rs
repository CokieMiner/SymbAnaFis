//! Bounded DAG transformations and guarded mathematical identities.

use super::{DomainSafety, SimplificationStep, SimplificationTarget};

mod algebraic;
mod angles;
mod expansion;
mod factoring;
mod fractions;
mod identities;
mod limits;
mod logarithms;
mod monomial;
mod polynomial;
mod polynomial_gcd;
mod polynomial_input;
mod polynomial_work;
mod powers;
mod prepared;
mod prepared_plan;
mod prepared_rule;
mod rules;
mod scoring;

pub use limits::Limits;
pub use monomial::Monomial;
pub use polynomial::SparsePolynomial;
pub use polynomial_input::PolynomialInput;
pub use polynomial_work::PolynomialWork;
pub use prepared::PreparedStep;
pub use prepared_plan::PreparedPlan;
pub use prepared_rule::PreparedRule;
pub use scoring::CandidateCost;
