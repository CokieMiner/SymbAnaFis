pub mod vir_dce;
pub mod gvn;

pub(super) use vir_dce::eliminate_vir_dead_code;
pub(super) use gvn::GvnKey;
pub(super) use gvn::optimize_vir_gvn;

pub(super) use super::optimize::ConstantPool;
pub use super::vir;
