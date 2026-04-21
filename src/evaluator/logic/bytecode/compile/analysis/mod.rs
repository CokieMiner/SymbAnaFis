pub mod dce;
pub mod gvn;

pub(super) use dce::eliminate_vir_dead_code;
pub(super) use gvn::GvnKey;
pub(super) use gvn::optimize_vir_gvn;

pub(super) use super::optimize::ConstantPool;
pub use super::vir;
