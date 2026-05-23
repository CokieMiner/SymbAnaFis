//! Internal convenience helper implementations.

pub(super) mod calculus;
pub(super) mod evaluation;

pub(super) use calculus::{gradient, gradient_str, hessian, hessian_str, jacobian, jacobian_str};
pub(super) use evaluation::evaluate_str;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::indexing_slicing,
    clippy::single_call_fn,
    clippy::wildcard_enum_match_arm,
    reason = "Standard test relaxations"
)]
mod tests;
