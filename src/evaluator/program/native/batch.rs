//! Lazy multi-lane SIMD and Rayon batch execution for [`NativeKernel`].

#![expect(
    unsafe_code,
    reason = "invoking generated SIMD machine-code entry points and writing validated batch slices requires unsafe FFI calls"
)]

use core::{mem::transmute, ptr::write};

use alloc::sync::Arc;

use crate::errors::EvalError;

#[cfg(target_arch = "aarch64")]
use super::AArch64Backend;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use super::FloatPrecision;
#[cfg(target_arch = "x86_64")]
use super::X86_64Backend;
use super::{
    BatchTarget, EvaluationValue, ExecutableBuffer, NativeEmitter, NativeKernel, NativeLayout,
    ProgramData, SlotEntry, Trampolines, Workspace,
};

pub struct BatchKernel<Value> {
    _buffer: Arc<ExecutableBuffer>,
    entry: SlotEntry<Value>,
    trampolines: *const (),
    lanes: usize,
}

// SAFETY: `BatchKernel` references immutable executable pages and a static trampoline table.
unsafe impl<Value> Send for BatchKernel<Value> {}

// SAFETY: Concurrent threads may invoke the immutable batch entry simultaneously.
unsafe impl<Value> Sync for BatchKernel<Value> {}

impl<Value> BatchKernel<Value> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn compile(layout: NativeLayout, data: &ProgramData<Value>) -> Option<Self> {
        let precision = match layout {
            NativeLayout::F64 => FloatPrecision::F64,
            NativeLayout::F32 => FloatPrecision::F32,
            NativeLayout::ComplexF64 | NativeLayout::ComplexF32 => return None,
        };
        #[cfg(target_arch = "x86_64")]
        let (mut backend, lanes) = X86_64Backend::new().into_batch(precision)?;
        #[cfg(target_arch = "aarch64")]
        let (mut backend, lanes) = AArch64Backend::new().into_batch(precision);
        let table = Trampolines::vector_table(precision, lanes)?;
        let (code, start) =
            NativeEmitter::compile_vector(data, precision, lanes, &mut backend).ok()?;
        let code_slice = code.get(start..)?;
        let buffer = Arc::new(ExecutableBuffer::new(code_slice)?);
        // SAFETY: `buffer.as_ptr()` points to a read-execute SIMD kernel adhering to `SlotEntry<Value>`.
        let entry: SlotEntry<Value> = unsafe { transmute(buffer.as_ptr()) };
        Some(Self {
            _buffer: buffer,
            entry,
            trampolines: table.as_ptr().cast::<()>(),
            lanes,
        })
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    const fn compile(_layout: NativeLayout, _data: &ProgramData<Value>) -> Option<Self> {
        None
    }
}

impl<Value: EvaluationValue> NativeKernel<Value> {
    pub fn eval_batch_with<Target: ?Sized + BatchTarget<Value>>(
        &self,
        data: &Arc<ProgramData<Value>>,
        workspace: &mut Workspace<Value>,
        transient: bool,
        columns: &[&[Value]],
        target: &mut Target,
    ) -> Result<(), EvalError> {
        let rows = target.validate(data, columns)?;
        if rows == 0 {
            return Ok(());
        }
        if rows > 1 {
            let _cached = self
                .batch
                .get_or_init(|| BatchKernel::compile(self.layout, data));
        }
        // SAFETY: `target.validate` proved all columns and outputs match `rows`.
        unsafe {
            self.dispatch_batch(data, workspace, transient, columns, (0, rows), target);
        }
        Ok(())
    }

    #[cfg(feature = "rayon")]
    pub fn eval_batch_par<Target: ?Sized + BatchTarget<Value>>(
        &self,
        data: &Arc<ProgramData<Value>>,
        columns: &[&[Value]],
        target: &mut Target,
    ) -> Result<(), EvalError>
    where
        Value: Send + Sync,
    {
        ProgramData::run_batch_par(
            data,
            columns,
            target,
            || {
                self.batch
                    .get_or_init(|| BatchKernel::compile(self.layout, data))
                    .as_ref()
                    .map_or(1, |kernel| kernel.lanes)
            },
            |workspace, single_target| {
                // SAFETY: `run_batch_par` validated 1 row against `columns` and `single_target`.
                unsafe {
                    self.dispatch_batch(data, workspace, true, columns, (0, 1), single_target);
                }
            },
            |workspace, transient, range, chunk| {
                // SAFETY: `run_batch_par` validated `range` against `columns` and `chunk`.
                unsafe {
                    self.dispatch_batch(data, workspace, transient, columns, range, chunk);
                }
            },
        )
    }

    unsafe fn dispatch_batch<Target: ?Sized + BatchTarget<Value>>(
        &self,
        data: &Arc<ProgramData<Value>>,
        workspace: &mut Workspace<Value>,
        transient: bool,
        columns: &[&[Value]],
        (offset, rows): (usize, usize),
        target: &mut Target,
    ) {
        if rows > 1
            && let Some(Some(kernel)) = self.batch.get()
        {
            macro_rules! run_simd {
                ($lanes:literal) => {
                    ProgramData::eval_lanes::<$lanes, Target>(
                        data,
                        workspace,
                        transient,
                        columns,
                        (offset, rows),
                        target,
                        |registers| {
                            let regs_ptr = registers.as_mut_ptr().cast::<Value>();
                            (kernel.entry)(regs_ptr, kernel.trampolines);
                        },
                    )
                };
            }
            // SAFETY: caller's contract proves column/output bounds and `kernel.lanes`.
            unsafe {
                match kernel.lanes {
                    2 => {
                        run_simd!(2);
                        return;
                    }
                    4 => {
                        run_simd!(4);
                        return;
                    }
                    8 => {
                        run_simd!(8);
                        return;
                    }
                    16 => {
                        run_simd!(16);
                        return;
                    }
                    _ => {}
                }
            }
        }
        // SAFETY: caller's contract proves column and output bounds.
        unsafe {
            self.batch_rows(data, workspace, transient, columns, (offset, rows), target);
        }
    }

    unsafe fn batch_rows<Target: ?Sized + BatchTarget<Value>>(
        &self,
        data: &Arc<ProgramData<Value>>,
        workspace: &mut Workspace<Value>,
        transient: bool,
        columns: &[&[Value]],
        (offset, rows): (usize, usize),
        target: &mut Target,
    ) {
        if data.results.is_empty() || rows == 0 {
            return;
        }
        let all_active = data.active_parameters.len() == data.parameters.len();
        let single_res_idx = data
            .results
            .first()
            .map_or(0, |&r| usize::try_from(r).unwrap_or(0));
        let registers = self.prepare_scalar(data, workspace, transient);
        for row in 0..rows {
            let col_idx = offset.saturating_add(row);
            if all_active {
                for ([slot], column) in registers.iter_mut().zip(columns) {
                    // SAFETY: `col_idx < column.len()`.
                    slot.clone_from(unsafe { column.get_unchecked(col_idx) });
                }
            } else {
                for &param_u32 in &data.active_parameters {
                    let idx = usize::try_from(param_u32).unwrap_or(0);
                    // SAFETY: `idx` is bounded by `validate` and `col_idx < column.len()`.
                    unsafe {
                        let [slot] = registers.get_unchecked_mut(idx);
                        slot.clone_from(columns.get_unchecked(idx).get_unchecked(col_idx));
                    }
                }
            }
            let regs_ptr = registers.as_mut_ptr().cast::<Value>();
            // SAFETY: `regs_ptr` holds `register_count` (+2 for complex) slots; for scalar
            // direct-return kernels (`!store_to_slots`), write the returned value into
            // `registers[single_res_idx]` so `Target::write_full::<1>` reads it uniformly.
            unsafe {
                if self.store_to_slots {
                    let func: SlotEntry<Value> = transmute(self.scalar_entry);
                    func(regs_ptr, self.trampolines);
                } else {
                    let val = self.invoke_scalar(regs_ptr, single_res_idx);
                    write(regs_ptr.add(single_res_idx), val);
                }
                target.write_full::<1>(&data.results, registers, row);
            }
        }
    }
}
