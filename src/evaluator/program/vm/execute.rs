//! Public single-output scalar, sequential-batch, and parallel-batch program execution.

#![expect(
    unsafe_code,
    reason = "validated batch lengths and register counts prove every slice and register slot read"
)]

use core::{
    mem::{MaybeUninit, needs_drop},
    ptr::{drop_in_place, slice_from_raw_parts_mut},
};

use alloc::sync::Arc;

use crate::errors::EvalError;

use super::{BLOCK, BatchTarget, EvaluationValue, Program, ProgramData, STACK_REGS, Workspace};

/// Unwind-safe stack register buffer that initializes only `0..reg_count`.
struct StackRegs<Value> {
    slots: [MaybeUninit<[Value; 1]>; STACK_REGS],
    init_len: usize,
}

impl<Value> StackRegs<Value> {
    #[inline]
    const fn new() -> Self {
        Self {
            slots: [const { MaybeUninit::uninit() }; STACK_REGS],
            init_len: 0,
        }
    }

    #[inline]
    unsafe fn push(&mut self, value: Value) {
        // SAFETY: caller guarantees `self.init_len < STACK_REGS`.
        unsafe {
            self.slots.get_unchecked_mut(self.init_len).write([value]);
        }
        self.init_len = self.init_len.saturating_add(1);
    }

    #[inline]
    unsafe fn as_slice_mut(&mut self) -> &mut [[Value; 1]] {
        // SAFETY: slots `0..self.init_len` were initialized via `push`.
        unsafe { &mut *slice_from_raw_parts_mut(self.slots.as_mut_ptr().cast(), self.init_len) }
    }
}

impl<Value> Drop for StackRegs<Value> {
    #[inline]
    fn drop(&mut self) {
        if needs_drop::<Value>() {
            // SAFETY: slots `0..self.init_len` were initialized in order.
            unsafe {
                drop_in_place(slice_from_raw_parts_mut(
                    self.slots.as_mut_ptr().cast::<[Value; 1]>(),
                    self.init_len,
                ));
            }
        }
    }
}

impl<Value: EvaluationValue> Program<Value> {
    /// Evaluate one complete parameter row using transient stack or heap storage.
    ///
    /// Small and medium programs (`<= 32` registers) execute directly on the call
    /// stack with zero heap allocations and zero atomic reference-count operations.
    /// For repeated evaluations of larger programs, reuse a [`Workspace`] via
    /// [`Self::eval_with`].
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`] for missing or extra inputs.
    ///
    /// # Panics
    /// Propagates numerical backend panics in the portable VM; native JIT
    /// kernels (`Program::with_native`) cross an `extern "C"` boundary, where
    /// an unexpected backend panic aborts instead of unwinding.
    ///
    /// ```
    /// use symb_anafis::{Context, Program};
    /// let context = Context::new()?;
    /// let x = context.symb("eval_x")?;
    /// let program = Program::<f64>::compile(&(x + 10), &[x])?;
    /// assert_eq!(program.eval(&[5.0])?, 15.0);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn eval(&self, parameters: &[Value]) -> Result<Value, EvalError> {
        self.eval_scalar(None, parameters)
    }

    /// Evaluate one complete parameter row into a caller-owned workspace.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`] for missing or extra inputs. The
    /// workspace is not modified on this error.
    ///
    /// # Panics
    /// Propagates numerical backend panics in the portable VM; native JIT
    /// kernels cross an `extern "C"` boundary and abort on backend panics.
    pub fn eval_with(
        &self,
        workspace: &mut Workspace<Value>,
        parameters: &[Value],
    ) -> Result<Value, EvalError> {
        self.eval_scalar(Some(workspace), parameters)
    }

    #[inline]
    fn eval_scalar(
        &self,
        mut workspace: Option<&mut Workspace<Value>>,
        parameters: &[Value],
    ) -> Result<Value, EvalError> {
        if let Some(result) =
            self.native
                .eval_single(&self.data, workspace.as_deref_mut(), parameters)
        {
            return result;
        }
        let Some(&result_u32) = self.data.results.first() else {
            return Err(EvalError::InvalidProgram);
        };
        let result_idx = usize::try_from(result_u32).unwrap_or(0);
        self.data.eval_scalar(workspace, parameters, |registers| {
            // SAFETY: `validate` guarantees `result_idx < register_count`.
            unsafe { registers.get_unchecked(result_idx)[0].clone() }
        })
    }

    /// Evaluate parameter columns into caller-owned output using a transient workspace.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`] or [`EvalError::BatchLength`]
    /// before writing any output when column shapes disagree.
    ///
    /// # Panics
    /// Propagates numerical backend panics; previously written rows remain valid.
    pub fn eval_batch(&self, columns: &[&[Value]], output: &mut [Value]) -> Result<(), EvalError> {
        let mut workspace = Workspace::new();
        if let Some(result) =
            self.native
                .eval_batch_with(&self.data, &mut workspace, true, columns, output)
        {
            return result;
        }
        ProgramData::run_vm_batch(&self.data, &mut workspace, true, columns, output)
    }

    /// Evaluate parameter columns into caller-owned output using a reusable workspace.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`] or [`EvalError::BatchLength`]
    /// before writing any output when column shapes disagree.
    ///
    /// # Panics
    /// Propagates numerical backend panics; previously written rows remain valid.
    pub fn eval_batch_with(
        &self,
        workspace: &mut Workspace<Value>,
        columns: &[&[Value]],
        output: &mut [Value],
    ) -> Result<(), EvalError> {
        if let Some(result) = self
            .native
            .eval_batch_with(&self.data, workspace, false, columns, output)
        {
            return result;
        }
        ProgramData::run_vm_batch(&self.data, workspace, false, columns, output)
    }

    /// Evaluate parameter columns into caller-owned output in parallel across
    /// the active Rayon thread pool using transient per-thread workspaces.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`] or [`EvalError::BatchLength`]
    /// before writing any output when column shapes disagree.
    ///
    /// # Panics
    /// Propagates numerical backend panics; completed rows remain valid.
    ///
    /// ```
    /// use symb_anafis::{Context, Program};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let program = Program::<f64>::compile(&(x.pow(2) + 1), &[x])?;
    /// let xs = [1.0, 2.0, 3.0, 4.0];
    /// let mut output = [0.0; 4];
    /// program.eval_batch_par(&[&xs], &mut output)?;
    /// assert_eq!(output, [2.0, 5.0, 10.0, 17.0]);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    #[cfg(feature = "rayon")]
    pub fn eval_batch_par(
        &self,
        columns: &[&[Value]],
        output: &mut [Value],
    ) -> Result<(), EvalError>
    where
        Value: Send + Sync,
    {
        if let Some(result) = self.native.eval_batch_par(&self.data, columns, output) {
            return result;
        }
        ProgramData::run_vm_batch_par(&self.data, columns, output)
    }
}

impl<Value: EvaluationValue> ProgramData<Value> {
    /// Validate `parameters`, execute `self` into `workspace` (or a transient stack/heap
    /// register frame when `None`), and project outputs via `read_out`.
    #[inline]
    pub fn eval_scalar<R>(
        self: &Arc<Self>,
        workspace: Option<&mut Workspace<Value>>,
        parameters: &[Value],
        read_out: impl FnOnce(&[[Value; 1]]) -> R,
    ) -> Result<R, EvalError> {
        if parameters.len() != self.parameters.len() {
            return Err(EvalError::ParameterCount {
                expected: self.parameters.len(),
                actual: parameters.len(),
            });
        }
        if let Some(ws) = workspace {
            let registers = ws.prepare::<1>(self);
            // SAFETY: `prepare` initialized `register_count` slots and validated `parameters`.
            unsafe {
                self.load_scalar_parameters(registers, parameters);
                self.execute(registers);
                return Ok(read_out(registers));
            }
        }
        let reg_count = self.register_count;
        if reg_count <= STACK_REGS {
            let n_params = self.parameters.len();
            let prefix = n_params.saturating_add(self.constants.len());
            let all_active = self.active_parameters.len() == n_params;
            let mut stack = StackRegs::new();
            // SAFETY: `validate` guarantees `n_params + constants.len() <= reg_count <= STACK_REGS`.
            unsafe {
                if all_active {
                    for value in parameters {
                        stack.push(value.clone());
                    }
                } else {
                    for _ in 0..n_params {
                        stack.push(self.initial.clone());
                    }
                }
                for constant in &self.constants {
                    stack.push(constant.clone());
                }
                for _ in prefix..reg_count {
                    stack.push(self.initial.clone());
                }
                let registers = stack.as_slice_mut();
                if !all_active {
                    self.load_scalar_parameters(registers, parameters);
                }
                self.execute(registers);
                return Ok(read_out(registers));
            }
        }
        let mut transient_ws = Workspace::new();
        let registers = transient_ws.prepare_transient::<1>(self);
        // SAFETY: `prepare_transient` sized `registers` to `register_count` and validated `parameters`.
        unsafe {
            self.load_scalar_parameters(registers, parameters);
            self.execute(registers);
            Ok(read_out(registers))
        }
    }

    #[inline]
    unsafe fn execute_vm_chunk<Target: ?Sized + BatchTarget<Value>>(
        data: &Arc<Self>,
        workspace: &mut Workspace<Value>,
        transient: bool,
        columns: &[&[Value]],
        range: (usize, usize),
        target: &mut Target,
        blocked: bool,
    ) {
        // SAFETY: caller validated `columns`, `range`, and `target`.
        unsafe {
            if blocked {
                Self::eval_lanes::<BLOCK, Target>(
                    data,
                    workspace,
                    transient,
                    columns,
                    range,
                    target,
                    |regs| data.execute(regs),
                );
            } else {
                Self::eval_lanes::<1, Target>(
                    data,
                    workspace,
                    transient,
                    columns,
                    range,
                    target,
                    |regs| data.execute(regs),
                );
            }
        }
    }

    pub fn run_vm_batch<Target: ?Sized + BatchTarget<Value>>(
        data: &Arc<Self>,
        workspace: &mut Workspace<Value>,
        transient: bool,
        columns: &[&[Value]],
        target: &mut Target,
    ) -> Result<(), EvalError> {
        let rows = target.validate(data, columns)?;
        if rows > 0 {
            // SAFETY: `target.validate` proved all column and output lengths match `rows`.
            unsafe {
                Self::execute_vm_chunk(
                    data,
                    workspace,
                    transient,
                    columns,
                    (0, rows),
                    target,
                    Value::BATCH_BLOCKED && rows > 1,
                );
            }
        }
        Ok(())
    }

    #[cfg(feature = "rayon")]
    pub fn run_vm_batch_par<Target: ?Sized + BatchTarget<Value>>(
        data: &Arc<Self>,
        columns: &[&[Value]],
        target: &mut Target,
    ) -> Result<(), EvalError>
    where
        Value: Send + Sync,
    {
        Self::run_batch_par(
            data,
            columns,
            target,
            || if Value::BATCH_BLOCKED { BLOCK } else { 1 },
            |workspace, single_target| {
                // SAFETY: `run_batch_par` validated 1 row against `columns` and `single_target`.
                unsafe {
                    Self::execute_vm_chunk(
                        data,
                        workspace,
                        true,
                        columns,
                        (0, 1),
                        single_target,
                        false,
                    );
                }
            },
            |workspace, transient, (offset, chunk_rows), chunk| {
                // SAFETY: `run_batch_par` validated `(offset, chunk_rows)` against `columns` and `chunk`.
                unsafe {
                    Self::execute_vm_chunk(
                        data,
                        workspace,
                        transient,
                        columns,
                        (offset, chunk_rows),
                        chunk,
                        Value::BATCH_BLOCKED && chunk_rows > 1,
                    );
                }
            },
        )
    }
}
