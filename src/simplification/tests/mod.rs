//! Presets, composition, resource contracts, and mathematical regression evidence.

use super::{Limits, PreparedPlan, PreparedStep};

mod algebra;
mod branches;
mod contracts;
mod domains;
mod functions;
mod polynomials;
mod support;

use support::{evaluate, simplify};
