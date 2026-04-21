pub mod gvn;
//pub mod nary_extract;
pub mod vir_dce;

//pub use super::{VInstruction, VReg};
pub(super) use gvn::GvnKey;
pub(super) use gvn::optimize_vir_gvn;
//pub(super) use nary_extract::optimize_nary_extraction;
pub(super) use vir_dce::eliminate_vir_dead_code;

pub(super) use super::optimize::ConstantPool;
pub use super::vir;
