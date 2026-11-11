//! Target architecture contracts and ISA machine-code encoders.

use super::{Combine, Fused, InlineBinary, InlineUnary};

#[cfg(any(target_arch = "aarch64", test))]
mod aarch64;
mod backend;
#[cfg(any(target_arch = "riscv64", test))]
mod riscv64;
#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(any(target_arch = "aarch64", test))]
pub use aarch64::AArch64Backend;
pub use backend::{ArchBackend, FloatPrecision, FusedMemOperand, HwReg};
#[cfg(any(target_arch = "riscv64", test))]
pub use riscv64::Riscv64Backend;
#[cfg(target_arch = "x86_64")]
pub use x86_64::X86_64Backend;
#[cfg(all(test, target_arch = "x86_64"))]
pub use x86_64::{FEAT_AVX, FEAT_AVX2, FEAT_AVX512, FEAT_FMA, FEAT_SSE41};
