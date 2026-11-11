//! Scalar algebraic, power, root, logarithmic, trigonometric, and hyperbolic passes.

use super::{Domain, DomainSafety};

mod algebraic;
mod branches;
mod identities;
mod powers;

pub use algebraic::Algebraic;
pub use branches::Branches;
pub use identities::Identities;
pub use powers::Powers;
