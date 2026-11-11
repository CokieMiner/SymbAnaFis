//! Host-executable [`NativeKernel`] backed by W^X virtual memory, with direct
//! scalar register return and zero-allocation stack execution for transient calls.

#![expect(
    unsafe_code,
    reason = "invoking generated machine-code entry points and reading validated register slots requires unsafe FFI calls"
)]

use core::{
    mem::{MaybeUninit, transmute},
    ptr::{copy_nonoverlapping, read, write},
};
use std::sync::OnceLock;

use alloc::sync::Arc;

use crate::errors::EvalError;

#[cfg(target_arch = "aarch64")]
use super::AArch64Backend;
#[cfg(target_arch = "riscv64")]
use super::Riscv64Backend;
#[cfg(target_arch = "x86_64")]
use super::X86_64Backend;
use super::{
    BatchKernel, EvaluationValue, ExecutableBuffer, NativeEmitter, NativeLayout, NativeScalar,
    ProgramData, STACK_REGS, Trampolines, Workspace,
};

pub type SlotEntry<Value> = unsafe extern "C" fn(regs: *mut Value, trampolines: *const ());

/// Stack slot capacity (`STACK_REGS + 2` trailing complex scratch slots) for
/// zero-allocation transient calls.
const NATIVE_STACK_SLOTS: usize = STACK_REGS + 2;

/// Immutable host machine-code kernel attached to a [`super::Program`] or [`super::MultiProgram`].
///
/// Invariant: constructed exclusively via [`Self::compile`] where `Value: NativeScalar`,
/// proving `Value` is `Copy` with the exact binary layout of `self.layout`.
pub struct NativeKernel<Value> {
    pub buffer: Arc<ExecutableBuffer>,
    pub scalar_entry: *const u8,
    pub trampolines: *const (),
    pub layout: NativeLayout,
    pub store_to_slots: bool,
    pub batch: OnceLock<Option<BatchKernel<Value>>>,
}

// SAFETY: `NativeKernel` holds immutable executable pages, static trampoline
// pointers, a fixed layout tag, and a synchronized lazy batch cache.
unsafe impl<Value: Send + Sync> Send for NativeKernel<Value> {}

// SAFETY: Concurrent evaluations read only immutable machine code and metadata;
// mutable register state lives in each caller's stack buffer or `Workspace<Value>`.
unsafe impl<Value: Send + Sync> Sync for NativeKernel<Value> {}

impl<Value> NativeKernel<Value> {
    pub fn compile(data: &ProgramData<Value>, store_to_slots: bool) -> Result<Self, EvalError>
    where
        Value: NativeScalar,
    {
        #[cfg(target_arch = "x86_64")]
        let mut backend = X86_64Backend::new();
        #[cfg(target_arch = "aarch64")]
        let mut backend = AArch64Backend::new();
        #[cfg(target_arch = "riscv64")]
        let mut backend = Riscv64Backend::new();

        let (code, start) =
            NativeEmitter::compile(data, Value::LAYOUT, store_to_slots, &mut backend)?;
        let code_slice = code.get(start..).ok_or(EvalError::InvalidProgram)?;
        let trampolines = Trampolines::table(Value::LAYOUT).as_ptr().cast::<()>();
        let buffer = Arc::new(
            ExecutableBuffer::new(code_slice).ok_or(EvalError::NativeCompilationUnavailable)?,
        );
        let scalar_entry = buffer.as_ptr();
        Ok(Self {
            buffer,
            scalar_entry,
            trampolines,
            layout: Value::LAYOUT,
            store_to_slots,
            batch: OnceLock::new(),
        })
    }
}

impl<Value: EvaluationValue> NativeKernel<Value> {
    /// Evaluate a single-output kernel using `workspace` or transient stack/heap storage.
    #[inline]
    pub fn eval_single(
        &self,
        data: &Arc<ProgramData<Value>>,
        workspace: Option<&mut Workspace<Value>>,
        parameters: &[Value],
    ) -> Result<Value, EvalError> {
        let Some(&result_u32) = data.results.first() else {
            return Err(EvalError::InvalidProgram);
        };
        let result_idx = usize::try_from(result_u32).unwrap_or(0);
        self.eval_frame(data, workspace, parameters, |regs| {
            // SAFETY: `eval_frame` initialized `regs` with `register_count` (+2 for complex) slots.
            unsafe { self.invoke_scalar(regs, result_idx) }
        })
    }

    /// Evaluate a multi-output kernel into `outputs` using `workspace` or transient storage.
    #[inline]
    pub fn eval_multi(
        &self,
        data: &Arc<ProgramData<Value>>,
        workspace: Option<&mut Workspace<Value>>,
        parameters: &[Value],
        outputs: &mut [Value],
    ) -> Result<(), EvalError> {
        if parameters.len() != data.parameters.len() {
            return Err(EvalError::ParameterCount {
                expected: data.parameters.len(),
                actual: parameters.len(),
            });
        }
        if outputs.len() != data.results.len() {
            return Err(EvalError::OutputCount {
                expected: data.results.len(),
                actual: outputs.len(),
            });
        }
        if outputs.is_empty() {
            return Ok(());
        }
        self.eval_frame(data, workspace, parameters, |regs| {
            // SAFETY: `eval_frame` initialized `regs` and `Value` is `Copy`.
            unsafe {
                let func: SlotEntry<Value> = transmute(self.scalar_entry);
                func(regs, self.trampolines);
                for (dst, &res_u32) in outputs.iter_mut().zip(&data.results) {
                    *dst = read(regs.add(usize::try_from(res_u32).unwrap_or(0)));
                }
            }
        })
    }

    #[inline]
    fn eval_frame<R>(
        &self,
        data: &Arc<ProgramData<Value>>,
        workspace: Option<&mut Workspace<Value>>,
        parameters: &[Value],
        invoke: impl FnOnce(*mut Value) -> R,
    ) -> Result<R, EvalError> {
        if parameters.len() != data.parameters.len() {
            return Err(EvalError::ParameterCount {
                expected: data.parameters.len(),
                actual: parameters.len(),
            });
        }
        if let Some(ws) = workspace {
            let registers = self.prepare_scalar(data, ws, false);
            // SAFETY: `registers` holds the initialized register file and `parameters` was validated.
            unsafe {
                data.load_scalar_parameters(registers, parameters);
            }
            return Ok(invoke(registers.as_mut_ptr().cast::<Value>()));
        }
        if data.register_count <= STACK_REGS {
            let mut stack: [MaybeUninit<Value>; NATIVE_STACK_SLOTS] =
                [const { MaybeUninit::uninit() }; NATIVE_STACK_SLOTS];
            let regs_ptr = stack.as_mut_ptr().cast::<Value>();
            // SAFETY: `Value: NativeScalar` (`Copy`), `register_count + 2 <= NATIVE_STACK_SLOTS`,
            // and `init_stack_slots` initializes all required parameter and constant slots.
            unsafe {
                Self::init_stack_slots(data, regs_ptr, parameters);
            }
            return Ok(invoke(regs_ptr));
        }
        let mut transient_ws = Workspace::new();
        let registers = self.prepare_scalar(data, &mut transient_ws, true);
        // SAFETY: `registers` holds `register_count` (+2 for complex) slots and `parameters` was validated.
        unsafe {
            data.load_scalar_parameters(registers, parameters);
        }
        Ok(invoke(registers.as_mut_ptr().cast::<Value>()))
    }

    /// Initialize stack slots `0..register_count` (+2 for complex) for `Value: Copy`.
    ///
    /// # Safety
    /// `Value` is `Copy`, `regs_ptr` points to at least `data.register_count + 2` slots,
    /// and `parameters.len() == data.parameters.len()`.
    #[inline]
    unsafe fn init_stack_slots(
        data: &ProgramData<Value>,
        regs_ptr: *mut Value,
        parameters: &[Value],
    ) {
        let n_params = data.parameters.len();
        let n_consts = data.constants.len();
        // SAFETY: `Value` is `Copy` (`NativeScalar`), and `regs_ptr` has `>= register_count + 2` slots.
        unsafe {
            if data.active_parameters.len() == n_params {
                copy_nonoverlapping(parameters.as_ptr(), regs_ptr, n_params);
            } else {
                for &param_u32 in &data.active_parameters {
                    let idx = usize::try_from(param_u32).unwrap_or(0);
                    write(regs_ptr.add(idx), read(parameters.as_ptr().add(idx)));
                }
            }
            copy_nonoverlapping(data.constants.as_ptr(), regs_ptr.add(n_params), n_consts);
        }
    }

    pub fn prepare_scalar<'ws>(
        &self,
        data: &Arc<ProgramData<Value>>,
        workspace: &'ws mut Workspace<Value>,
        transient: bool,
    ) -> &'ws mut [[Value; 1]] {
        if transient {
            let _ = workspace.prepare_transient::<1>(data);
        } else {
            let _ = workspace.prepare::<1>(data);
        }
        if matches!(
            self.layout,
            NativeLayout::ComplexF64 | NativeLayout::ComplexF32
        ) {
            let required = data.register_count.saturating_add(2);
            if workspace.registers.len() < required {
                workspace.registers.resize(required, data.initial.clone());
            }
        }
        workspace.registers.as_chunks_mut().0
    }

    /// Invoke `self.scalar_entry` on `regs` and read back the scalar output.
    ///
    /// # Safety
    /// `regs` points to an initialized workspace of `data.register_count` (+2 for complex) slots.
    #[inline]
    pub unsafe fn invoke_scalar(&self, regs: *mut Value, result_idx: usize) -> Value {
        match self.layout {
            NativeLayout::F64 => {
                // SAFETY: `NativeKernel::compile` proved `Value` is `f64`.
                unsafe {
                    let func: unsafe extern "C" fn(*mut f64, *const ()) -> f64 =
                        transmute(self.scalar_entry);
                    let value = func(regs.cast::<f64>(), self.trampolines);
                    read((&raw const value).cast::<Value>())
                }
            }
            NativeLayout::F32 => {
                // SAFETY: `NativeKernel::compile` proved `Value` is `f32`.
                unsafe {
                    let func: unsafe extern "C" fn(*mut f32, *const ()) -> f32 =
                        transmute(self.scalar_entry);
                    let value = func(regs.cast::<f32>(), self.trampolines);
                    read((&raw const value).cast::<Value>())
                }
            }
            NativeLayout::ComplexF64 | NativeLayout::ComplexF32 => {
                // SAFETY: complex kernels write `regs[result_idx]` before returning.
                unsafe {
                    let func: SlotEntry<Value> = transmute(self.scalar_entry);
                    func(regs, self.trampolines);
                    read(regs.add(result_idx))
                }
            }
        }
    }
}
