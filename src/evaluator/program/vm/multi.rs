//! Public multi-output scalar, sequential-batch, and parallel-batch program execution.

use alloc::{vec, vec::Vec};

use crate::errors::EvalError;

use super::{EvaluationValue, MultiProgram, ProgramData, Workspace};

impl<Value: EvaluationValue> MultiProgram<Value> {
    /// Evaluate one parameter row using transient storage, allocating a `Vec`
    /// of all outputs. Reuse a [`Workspace`] and output slice via [`Self::eval_with`]
    /// for zero-allocation repeated calls.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`] for missing or extra inputs.
    ///
    /// # Panics
    /// Propagates VM backend panics; native JIT kernels abort across `extern "C"`.
    pub fn eval(&self, parameters: &[Value]) -> Result<Vec<Value>, EvalError> {
        let mut outputs = vec![self.data.initial.clone(); self.data.results.len()];
        self.eval_scalar(None, parameters, &mut outputs)?;
        Ok(outputs)
    }

    /// Evaluate one parameter row into caller-owned `outputs` using a reusable workspace.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`] or [`EvalError::OutputCount`] without
    /// modifying `workspace` or `outputs`.
    ///
    /// # Panics
    /// Propagates VM backend panics; native JIT kernels abort across `extern "C"`.
    pub fn eval_with(
        &self,
        workspace: &mut Workspace<Value>,
        parameters: &[Value],
        outputs: &mut [Value],
    ) -> Result<(), EvalError> {
        self.eval_scalar(Some(workspace), parameters, outputs)
    }

    #[inline]
    fn eval_scalar(
        &self,
        mut workspace: Option<&mut Workspace<Value>>,
        parameters: &[Value],
        outputs: &mut [Value],
    ) -> Result<(), EvalError> {
        if let Some(result) =
            self.native
                .eval_multi(&self.data, workspace.as_deref_mut(), parameters, outputs)
        {
            return result;
        }
        if parameters.len() != self.data.parameters.len() {
            return Err(EvalError::ParameterCount {
                expected: self.data.parameters.len(),
                actual: parameters.len(),
            });
        }
        if outputs.len() != self.data.results.len() {
            return Err(EvalError::OutputCount {
                expected: self.data.results.len(),
                actual: outputs.len(),
            });
        }
        if outputs.is_empty() {
            return Ok(());
        }
        self.data.eval_scalar(workspace, parameters, |registers| {
            for (destination, &res_u32) in outputs.iter_mut().zip(&self.data.results) {
                let idx = usize::try_from(res_u32).unwrap_or(0);
                if let Some([value]) = registers.get(idx) {
                    destination.clone_from(value);
                }
            }
        })
    }

    /// Evaluate parameter columns into caller-owned output columns using a transient workspace.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`], [`EvalError::OutputCount`],
    /// [`EvalError::OutputLength`], or [`EvalError::BatchLength`] before writing.
    ///
    /// # Panics
    /// Propagates numerical backend panics; previously written rows remain valid.
    pub fn eval_batch(
        &self,
        columns: &[&[Value]],
        outputs: &mut [&mut [Value]],
    ) -> Result<(), EvalError> {
        let mut workspace = Workspace::new();
        if let Some(result) =
            self.native
                .eval_batch_with(&self.data, &mut workspace, true, columns, outputs)
        {
            return result;
        }
        ProgramData::run_vm_batch(&self.data, &mut workspace, true, columns, outputs)
    }

    /// Evaluate parameter columns into caller-owned output columns using a reusable workspace.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`], [`EvalError::OutputCount`],
    /// [`EvalError::OutputLength`], or [`EvalError::BatchLength`] before writing.
    ///
    /// # Panics
    /// Propagates numerical backend panics; previously written rows remain valid.
    pub fn eval_batch_with(
        &self,
        workspace: &mut Workspace<Value>,
        columns: &[&[Value]],
        outputs: &mut [&mut [Value]],
    ) -> Result<(), EvalError> {
        if let Some(result) = self
            .native
            .eval_batch_with(&self.data, workspace, false, columns, outputs)
        {
            return result;
        }
        ProgramData::run_vm_batch(&self.data, workspace, false, columns, outputs)
    }

    /// Evaluate parameter columns into caller-owned output columns in parallel across
    /// the Rayon thread pool, computing all outputs in one parallel dispatch.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`], [`EvalError::OutputCount`],
    /// [`EvalError::OutputLength`], or [`EvalError::BatchLength`] before writing.
    ///
    /// # Panics
    /// Propagates numerical backend panics; completed rows remain valid.
    ///
    /// ```
    /// use symb_anafis::{Context, MultiProgram};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let system = MultiProgram::<f64>::compile(&[x + 1, x * 2], &[x])?;
    /// let xs = [1.0, 2.0, 3.0, 4.0];
    /// let mut out_a = [0.0; 4];
    /// let mut out_b = [0.0; 4];
    /// system.eval_batch_par(&[&xs], &mut [&mut out_a, &mut out_b])?;
    /// assert_eq!(out_a, [2.0, 3.0, 4.0, 5.0]);
    /// assert_eq!(out_b, [2.0, 4.0, 6.0, 8.0]);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    #[cfg(feature = "rayon")]
    pub fn eval_batch_par(
        &self,
        columns: &[&[Value]],
        outputs: &mut [&mut [Value]],
    ) -> Result<(), EvalError>
    where
        Value: Send + Sync,
    {
        if let Some(result) = self.native.eval_batch_par(&self.data, columns, outputs) {
            return result;
        }
        ProgramData::run_vm_batch_par(&self.data, columns, outputs)
    }
}
