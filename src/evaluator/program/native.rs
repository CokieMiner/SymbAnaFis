//! Native in-memory machine-code compilation attached to [`Program`] via [`Program::with_native`].

#![cfg_attr(
    native_jit_host,
    expect(
        clippy::self_named_module_files,
        reason = "native compilation owns its architecture encoders, lowering passes, and runtime"
    )
)]

use core::fmt::{Debug, Formatter, Result as FmtResult};
#[cfg(not(native_jit_host))]
use core::{convert::Infallible, marker::PhantomData};

use alloc::sync::Arc;

use num_anafis::Complex;

use crate::errors::EvalError;

use super::{BatchTarget, EvaluationValue, MultiProgram, Program, ProgramData, Workspace};
#[cfg(native_jit_host)]
use super::{
    Combine, Fold, Fused, InlineBinary, InlineUnary, Instruction, Operation, PairedFunction,
    STACK_REGS,
};

#[cfg(all(native_jit_host, not(test), target_arch = "aarch64"))]
use arch::AArch64Backend;
#[cfg(native_jit_host)]
use arch::FusedMemOperand;
#[cfg(all(native_jit_host, not(test), target_arch = "riscv64"))]
use arch::Riscv64Backend;
#[cfg(all(native_jit_host, not(test), target_arch = "x86_64"))]
use arch::X86_64Backend;
#[cfg(all(native_jit_host, not(test)))]
use arch::{ArchBackend, FloatPrecision, HwReg};
#[cfg(native_jit_host)]
use batch::BatchKernel;
#[cfg(all(native_jit_host, not(test)))]
use emit::NativeEmitter;
#[cfg(native_jit_host)]
use host::{NativeKernel, SlotEntry};
#[cfg(all(native_jit_host, not(test)))]
use memory::ExecutableBuffer;
#[cfg(all(native_jit_host, not(test)))]
use trampolines::Trampolines;

#[cfg(native_jit_host)]
mod arch;
#[cfg(native_jit_host)]
mod batch;
#[cfg(native_jit_host)]
mod emit;
#[cfg(native_jit_host)]
mod host;
#[cfg(native_jit_host)]
mod memory;
#[cfg(native_jit_host)]
mod trampolines;

#[cfg(all(test, native_jit_host))]
pub use arch::{AArch64Backend, ArchBackend, FloatPrecision, HwReg, Riscv64Backend};
#[cfg(all(test, native_jit_host, target_arch = "x86_64"))]
pub use arch::{FEAT_AVX, FEAT_AVX2, FEAT_AVX512, FEAT_FMA, FEAT_SSE41, X86_64Backend};
#[cfg(all(test, native_jit_host))]
pub use emit::{LivenessPlan, NativeEmitter};
#[cfg(all(test, native_jit_host))]
pub use memory::ExecutableBuffer;
#[cfg(all(test, native_jit_host))]
pub use trampolines::Trampolines;

/// Fixed binary register layout for native machine-code lowering (`mod native` is private).
#[cfg(native_jit_host)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeLayout {
    F64,
    F32,
    ComplexF64,
    ComplexF32,
}

/// Sealed supertrait restricting [`NativeScalar`] to hardware-layout types (`mod native` is private).
pub trait NativeSealed {
    #[cfg(native_jit_host)]
    const LAYOUT: NativeLayout;
}

/// Fixed-layout numerical scalars supported by native machine-code lowering.
///
/// Only hardware-register scalars (`f64`, `f32`, `Complex<f64>`, `Complex<f32>`)
/// implement this sealed trait, guaranteeing at compile time that arbitrary-precision
/// or heap-allocated types cannot be lowered to raw assembly.
pub trait NativeScalar: EvaluationValue + NativeSealed + Copy {}

macro_rules! impl_native_scalar {
    ($($ty:ty => $layout:ident),* $(,)?) => {
        $(
            impl NativeSealed for $ty {
                #[cfg(native_jit_host)]
                const LAYOUT: NativeLayout = NativeLayout::$layout;
            }
            impl NativeScalar for $ty {}
        )*
    };
}

impl_native_scalar!(
    f64 => F64,
    f32 => F32,
    Complex<f64> => ComplexF64,
    Complex<f32> => ComplexF32,
);

/// Optional native JIT kernel slot embedded in [`Program<Value>`] and [`MultiProgram<Value>`].
#[cfg(native_jit_host)]
pub struct NativeSlot<Value>(Option<Arc<NativeKernel<Value>>>);

#[cfg(native_jit_host)]
impl<Value> Clone for NativeSlot<Value> {
    fn clone(&self) -> Self {
        Self(self.0.as_ref().map(Arc::clone))
    }
}

#[cfg(native_jit_host)]
impl<Value> NativeSlot<Value> {
    #[must_use]
    pub const fn none() -> Self {
        Self(None)
    }

    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.0.as_deref().map(|kernel| kernel.buffer.as_bytes())
    }

    fn compile(data: &ProgramData<Value>, store_to_slots: bool) -> Result<Self, EvalError>
    where
        Value: NativeScalar,
    {
        Ok(Self(Some(Arc::new(NativeKernel::compile(
            data,
            store_to_slots,
        )?))))
    }
}

#[cfg(native_jit_host)]
impl<Value: EvaluationValue> NativeSlot<Value> {
    #[inline]
    pub fn eval_single(
        &self,
        data: &Arc<ProgramData<Value>>,
        workspace: Option<&mut Workspace<Value>>,
        parameters: &[Value],
    ) -> Option<Result<Value, EvalError>> {
        Some(self.0.as_deref()?.eval_single(data, workspace, parameters))
    }

    #[inline]
    pub fn eval_multi(
        &self,
        data: &Arc<ProgramData<Value>>,
        workspace: Option<&mut Workspace<Value>>,
        parameters: &[Value],
        outputs: &mut [Value],
    ) -> Option<Result<(), EvalError>> {
        Some(
            self.0
                .as_deref()?
                .eval_multi(data, workspace, parameters, outputs),
        )
    }

    #[inline]
    pub fn eval_batch_with<Target: ?Sized + BatchTarget<Value>>(
        &self,
        data: &Arc<ProgramData<Value>>,
        workspace: &mut Workspace<Value>,
        transient: bool,
        columns: &[&[Value]],
        target: &mut Target,
    ) -> Option<Result<(), EvalError>> {
        Some(
            self.0
                .as_deref()?
                .eval_batch_with(data, workspace, transient, columns, target),
        )
    }

    #[cfg(feature = "rayon")]
    #[inline]
    pub fn eval_batch_par<Target: ?Sized + BatchTarget<Value>>(
        &self,
        data: &Arc<ProgramData<Value>>,
        columns: &[&[Value]],
        target: &mut Target,
    ) -> Option<Result<(), EvalError>>
    where
        Value: Send + Sync,
    {
        Some(self.0.as_deref()?.eval_batch_par(data, columns, target))
    }
}

/// Zero-sized placeholder when the host JIT backend is unavailable.
#[cfg(not(native_jit_host))]
pub struct NativeSlot<Value>(Option<Infallible>, PhantomData<Value>);

#[cfg(not(native_jit_host))]
impl<Value> Clone for NativeSlot<Value> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

#[cfg(not(native_jit_host))]
impl<Value> NativeSlot<Value> {
    #[must_use]
    pub const fn none() -> Self {
        Self(None, PhantomData)
    }

    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.0.map(|never| match never {})
    }

    const fn compile(_data: &ProgramData<Value>, _store_to_slots: bool) -> Result<Self, EvalError>
    where
        Value: NativeScalar,
    {
        Err(EvalError::NativeCompilationUnavailable)
    }
}

#[cfg(not(native_jit_host))]
impl<Value: EvaluationValue> NativeSlot<Value> {
    #[inline]
    pub fn eval_single(
        &self,
        _data: &Arc<ProgramData<Value>>,
        _workspace: Option<&mut Workspace<Value>>,
        _parameters: &[Value],
    ) -> Option<Result<Value, EvalError>> {
        self.0.map(|never| match never {})
    }

    #[inline]
    pub fn eval_multi(
        &self,
        _data: &Arc<ProgramData<Value>>,
        _workspace: Option<&mut Workspace<Value>>,
        _parameters: &[Value],
        _outputs: &mut [Value],
    ) -> Option<Result<(), EvalError>> {
        self.0.map(|never| match never {})
    }

    #[inline]
    pub fn eval_batch_with<Target: ?Sized + BatchTarget<Value>>(
        &self,
        _data: &Arc<ProgramData<Value>>,
        _workspace: &mut Workspace<Value>,
        _transient: bool,
        _columns: &[&[Value]],
        _target: &mut Target,
    ) -> Option<Result<(), EvalError>> {
        self.0.map(|never| match never {})
    }

    #[cfg(feature = "rayon")]
    #[inline]
    pub fn eval_batch_par<Target: ?Sized + BatchTarget<Value>>(
        &self,
        _data: &Arc<ProgramData<Value>>,
        _columns: &[&[Value]],
        _target: &mut Target,
    ) -> Option<Result<(), EvalError>>
    where
        Value: Send + Sync,
    {
        self.0.map(|never| match never {})
    }
}

impl<Value> Debug for NativeSlot<Value> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self.as_bytes() {
            Some(bytes) => formatter.debug_tuple("Native").field(&bytes.len()).finish(),
            None => formatter.write_str("None"),
        }
    }
}

impl<Value: NativeScalar> Program<Value> {
    /// Return a handle to this program backed by native host machine code.
    ///
    /// Shares the validated bytecode metadata with `self` and compiles an
    /// immutable `W^X` executable kernel for the host architecture (`x86_64`,
    /// `aarch64`, or `riscv64`), with lazy multi-lane SIMD batch compilation on
    /// supported CPUs. Calling `with_native` on a program that already carries a
    /// native kernel clones it without recompiling.
    ///
    /// # Errors
    /// Returns [`EvalError::NativeCompilationUnavailable`] when the `native_jit`
    /// feature is disabled, the target lacks a native backend, or executable
    /// virtual memory allocation is denied; returns [`EvalError::ProgramTooLarge`]
    /// if register byte offsets exceed 32-bit signed displacements.
    ///
    /// ```
    /// use symb_anafis::{Context, Program};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let program = Program::<f64>::compile(&(x.pow(2) + 2 * x + 1), &[x])?;
    /// let program = program.with_native().unwrap_or(program);
    /// assert_eq!(program.eval(&[3.0])?, 16.0);
    /// # #[cfg(feature = "native_jit")]
    /// # assert!(program.native_bytes().is_some());
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn with_native(&self) -> Result<Self, EvalError> {
        if self.native.as_bytes().is_some() {
            return Ok(self.clone());
        }
        Ok(Self {
            data: Arc::clone(&self.data),
            native: NativeSlot::compile(&self.data, false)?,
        })
    }
}

impl<Value: NativeScalar> MultiProgram<Value> {
    /// Return a handle to this multi-output program backed by native host machine code.
    ///
    /// # Errors
    /// Returns [`EvalError::NativeCompilationUnavailable`] when the `native_jit`
    /// feature is disabled, the target lacks a native backend, or executable memory
    /// allocation is denied; returns [`EvalError::ProgramTooLarge`] if register byte
    /// offsets exceed 32-bit signed displacements.
    ///
    /// ```
    /// use symb_anafis::{Context, MultiProgram};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let system = MultiProgram::<f64>::compile(&[x.sin(), x.cos()], &[x])?;
    /// let system = system.with_native().unwrap_or(system);
    /// assert_eq!(system.eval(&[0.0])?, vec![0.0, 1.0]);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn with_native(&self) -> Result<Self, EvalError> {
        if self.native.as_bytes().is_some() {
            return Ok(self.clone());
        }
        Ok(Self {
            data: Arc::clone(&self.data),
            native: NativeSlot::compile(&self.data, true)?,
        })
    }
}
