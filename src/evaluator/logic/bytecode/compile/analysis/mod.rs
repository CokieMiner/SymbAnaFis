//! VIR-level analysis passes run during the first compilation phase.
//!
//! These operate on [`VInstruction`] streams with virtual registers before
//! physical register allocation.
//!
//! Pipeline order (orchestrated by [`super::VirGenerator::into_parts`]):
//!  1. [`optimize_vir_gvn`] — Global Value Numbering + constant folding
//!  2. [`optimize_div_to_recip`] — division-to-reciprocal conversion
//!  3. [`fuse_vir`] — pre-scheduling VIR fusion
//!  4. [`greedy_schedule`] — Sethi-Ullman instruction scheduling
//!  5. [`eliminate_vir_dead_code`] — backward-liveness DCE

pub mod dce;
pub mod fusion;
pub mod gvn;
pub mod schedule;

pub(super) use dce::eliminate_vir_dead_code;
pub(super) use fusion::{fuse_vir, optimize_div_to_recip};
pub(super) use gvn::GvnKey;
pub(super) use gvn::optimize_vir_gvn;
pub(super) use schedule::greedy_schedule;

pub(super) use super::optimize::ConstantPool;
pub use super::vir;
pub(super) use super::vir::{VInstruction, VReg};
