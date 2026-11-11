//! Shared batch validation, active-parameter loading, and Rayon partitioning
//! across single-output and multi-output VM and native JIT execution.

#![expect(
    unsafe_code,
    reason = "validated batch lengths and register counts prove every column slice, output slice, and register slot read"
)]

use alloc::sync::Arc;
#[cfg(feature = "rayon")]
use alloc::vec::Vec;

#[cfg(feature = "rayon")]
use rayon::{
    current_num_threads,
    iter::{IndexedParallelIterator, ParallelIterator},
    slice::ParallelSliceMut,
};

use crate::errors::EvalError;

use super::{EvaluationValue, ProgramData, Workspace};

/// Destination storage for single-output (`[Value]`) and multi-output
/// (`[&mut [Value]]`) batch evaluations.
pub trait BatchTarget<Value> {
    /// Sub-chunk slice type yielded during parallel Rayon partitioning.
    #[cfg(feature = "rayon")]
    type Chunk<'chunk>: ?Sized + BatchTarget<Value> + Send
    where
        Self: 'chunk,
        Value: Send + Sync;

    /// Validate `columns` and `self` against `data`, returning the number of rows
    /// to evaluate (or `0` when no output writes are needed).
    ///
    /// # Errors
    /// Returns [`EvalError`] when parameter counts, output counts, or column lengths disagree.
    fn validate(&self, data: &ProgramData<Value>, columns: &[&[Value]])
    -> Result<usize, EvalError>;

    /// Copy one full `LANES`-row block from `registers` into block `block` of `self`.
    ///
    /// # Safety
    /// `(block + 1) * LANES <= rows`, `self` was validated for `rows`, and every
    /// entry in `results` is `< registers.len()`.
    unsafe fn write_full<const LANES: usize>(
        &mut self,
        results: &[u32],
        registers: &[[Value; LANES]],
        block: usize,
    ) where
        Value: Clone;

    /// Copy `tail_len` (`1..LANES`) rows from `registers` into `self[tail_start..]`.
    ///
    /// # Safety
    /// `tail_start + tail_len == rows`, `tail_len < LANES`, and every entry in
    /// `results` is `< registers.len()`.
    unsafe fn write_tail<const LANES: usize>(
        &mut self,
        results: &[u32],
        registers: &[[Value; LANES]],
        tail_start: usize,
        tail_len: usize,
    ) where
        Value: Clone;

    /// Partition `self` into aligned chunks of `chunk_len` rows and invoke `worker`
    /// in parallel on the active Rayon pool with one transient workspace per worker thread.
    ///
    /// # Safety
    /// `rows > 0`, `chunk_len > 0`, and `self` has `rows` elements per output.
    #[cfg(feature = "rayon")]
    unsafe fn par_for_each(
        &mut self,
        rows: usize,
        chunk_len: usize,
        worker: impl Fn(&mut Workspace<Value>, usize, &mut Self::Chunk<'_>) + Send + Sync,
    ) where
        Value: Send + Sync;
}

impl<Value> BatchTarget<Value> for [Value] {
    #[cfg(feature = "rayon")]
    type Chunk<'chunk>
        = Self
    where
        Self: 'chunk,
        Value: Send + Sync;

    fn validate(
        &self,
        data: &ProgramData<Value>,
        columns: &[&[Value]],
    ) -> Result<usize, EvalError> {
        data.validate_batch(columns, self.len())?;
        Ok(self.len())
    }

    #[inline]
    unsafe fn write_full<const LANES: usize>(
        &mut self,
        results: &[u32],
        registers: &[[Value; LANES]],
        block: usize,
    ) where
        Value: Clone,
    {
        let Some(&res_u32) = results.first() else {
            return;
        };
        let res_idx = usize::try_from(res_u32).unwrap_or(0);
        // SAFETY: caller's contract proves `block < self.len() / LANES` and `res_idx < registers.len()`.
        unsafe {
            self.as_chunks_mut::<LANES>()
                .0
                .get_unchecked_mut(block)
                .clone_from_slice(registers.get_unchecked(res_idx));
        }
    }

    #[inline]
    unsafe fn write_tail<const LANES: usize>(
        &mut self,
        results: &[u32],
        registers: &[[Value; LANES]],
        tail_start: usize,
        tail_len: usize,
    ) where
        Value: Clone,
    {
        let Some(&res_u32) = results.first() else {
            return;
        };
        let res_idx = usize::try_from(res_u32).unwrap_or(0);
        // SAFETY: caller's contract proves `tail_start + tail_len == self.len()` and `tail_len < LANES`.
        unsafe {
            self.get_unchecked_mut(tail_start..)
                .clone_from_slice(registers.get_unchecked(res_idx).get_unchecked(..tail_len));
        }
    }

    #[cfg(feature = "rayon")]
    unsafe fn par_for_each(
        &mut self,
        _rows: usize,
        chunk_len: usize,
        worker: impl Fn(&mut Workspace<Value>, usize, &mut Self) + Send + Sync,
    ) where
        Value: Send + Sync,
    {
        self.par_chunks_mut(chunk_len).enumerate().for_each_init(
            Workspace::new,
            |workspace, (index, chunk)| {
                worker(workspace, index.saturating_mul(chunk_len), chunk);
            },
        );
    }
}

impl<Value> BatchTarget<Value> for [&mut [Value]] {
    #[cfg(feature = "rayon")]
    type Chunk<'chunk>
        = [&'chunk mut [Value]]
    where
        Self: 'chunk,
        Value: Send + Sync;

    fn validate(
        &self,
        data: &ProgramData<Value>,
        columns: &[&[Value]],
    ) -> Result<usize, EvalError> {
        let rows = data.validate_multi_batch(columns, self)?;
        Ok(if self.is_empty() { 0 } else { rows })
    }

    #[inline]
    unsafe fn write_full<const LANES: usize>(
        &mut self,
        results: &[u32],
        registers: &[[Value; LANES]],
        block: usize,
    ) where
        Value: Clone,
    {
        for (destination, &res_u32) in self.iter_mut().zip(results) {
            let res_idx = usize::try_from(res_u32).unwrap_or(0);
            // SAFETY: caller's contract proves `block < destination.len() / LANES` and `res_idx < registers.len()`.
            unsafe {
                destination
                    .as_chunks_mut::<LANES>()
                    .0
                    .get_unchecked_mut(block)
                    .clone_from_slice(registers.get_unchecked(res_idx));
            }
        }
    }

    #[inline]
    unsafe fn write_tail<const LANES: usize>(
        &mut self,
        results: &[u32],
        registers: &[[Value; LANES]],
        tail_start: usize,
        tail_len: usize,
    ) where
        Value: Clone,
    {
        for (destination, &res_u32) in self.iter_mut().zip(results) {
            let res_idx = usize::try_from(res_u32).unwrap_or(0);
            // SAFETY: caller's contract proves `tail_start + tail_len == destination.len()` and `tail_len < LANES`.
            unsafe {
                destination
                    .get_unchecked_mut(tail_start..)
                    .clone_from_slice(registers.get_unchecked(res_idx).get_unchecked(..tail_len));
            }
        }
    }

    #[cfg(feature = "rayon")]
    unsafe fn par_for_each(
        &mut self,
        rows: usize,
        chunk_len: usize,
        worker: impl Fn(&mut Workspace<Value>, usize, &mut [&mut [Value]]) + Send + Sync,
    ) where
        Value: Send + Sync,
    {
        let n_outputs = self.len();
        let mut flat = ProgramData::partition_multi_outputs(self, rows, chunk_len);
        flat.par_chunks_mut(n_outputs).enumerate().for_each_init(
            Workspace::new,
            |workspace, (index, chunk_outputs)| {
                worker(workspace, index.saturating_mul(chunk_len), chunk_outputs);
            },
        );
    }
}

impl<Value> ProgramData<Value> {
    fn validate_columns(columns: &[&[Value]], expected_rows: usize) -> Result<(), EvalError> {
        for (parameter, column) in columns.iter().enumerate() {
            if column.len() != expected_rows {
                return Err(EvalError::BatchLength {
                    parameter,
                    expected: expected_rows,
                    actual: column.len(),
                });
            }
        }
        Ok(())
    }

    pub fn validate_batch(&self, columns: &[&[Value]], output_len: usize) -> Result<(), EvalError> {
        if columns.len() != self.parameters.len() {
            return Err(EvalError::ParameterCount {
                expected: self.parameters.len(),
                actual: columns.len(),
            });
        }
        Self::validate_columns(columns, output_len)
    }

    pub fn validate_multi_batch(
        &self,
        columns: &[&[Value]],
        outputs: &[&mut [Value]],
    ) -> Result<usize, EvalError> {
        if columns.len() != self.parameters.len() {
            return Err(EvalError::ParameterCount {
                expected: self.parameters.len(),
                actual: columns.len(),
            });
        }
        if outputs.len() != self.results.len() {
            return Err(EvalError::OutputCount {
                expected: self.results.len(),
                actual: outputs.len(),
            });
        }
        let rows = if let Some(first_output) = outputs.first() {
            let expected = first_output.len();
            for (output, slice) in outputs.iter().enumerate().skip(1) {
                if slice.len() != expected {
                    return Err(EvalError::OutputLength {
                        output,
                        expected,
                        actual: slice.len(),
                    });
                }
            }
            expected
        } else {
            columns.first().map_or(0, |column| column.len())
        };
        Self::validate_columns(columns, rows)?;
        Ok(rows)
    }

    /// Copy only the parameters actually read by `self` into `registers`.
    ///
    /// # Safety
    /// `parameters.len() == self.parameters.len()`, `registers.len() >= self.register_count`,
    /// and every entry in `self.active_parameters` is `< self.parameters.len() <= self.register_count`.
    #[inline]
    pub unsafe fn load_scalar_parameters(&self, registers: &mut [[Value; 1]], parameters: &[Value])
    where
        Value: Clone,
    {
        if self.active_parameters.len() == self.parameters.len() {
            for ([slot], value) in registers.iter_mut().zip(parameters) {
                slot.clone_from(value);
            }
        } else {
            for &param_u32 in &self.active_parameters {
                let idx = usize::try_from(param_u32).unwrap_or(0);
                // SAFETY: caller's contract and `validate` bound `idx` in both slices.
                unsafe {
                    let [slot] = registers.get_unchecked_mut(idx);
                    slot.clone_from(parameters.get_unchecked(idx));
                }
            }
        }
    }

    #[cfg(feature = "rayon")]
    #[must_use]
    pub fn par_chunk_len(rows: usize, lanes: usize) -> usize {
        let threads = current_num_threads().max(1);
        let raw = rows
            .checked_div(threads.saturating_mul(4))
            .unwrap_or(1)
            .max(1);
        if lanes <= 1 {
            raw
        } else if raw >= lanes {
            raw & !lanes.saturating_sub(1)
        } else if rows >= lanes.saturating_mul(threads) {
            lanes
        } else {
            rows.saturating_sub(1)
                .checked_div(threads)
                .unwrap_or(0)
                .saturating_add(1)
        }
    }

    #[cfg(feature = "rayon")]
    pub fn partition_multi_outputs<'out>(
        outputs: &'out mut [&mut [Value]],
        rows: usize,
        chunk_len: usize,
    ) -> Vec<&'out mut [Value]> {
        let n_outputs = outputs.len();
        if rows == 0 || n_outputs == 0 {
            return Vec::new();
        }
        let step = chunk_len.max(1);
        let n_chunks = rows
            .saturating_sub(1)
            .checked_div(step)
            .map_or(0, |quotient| quotient.saturating_add(1));
        let mut flat: Vec<&'out mut [Value]> = Vec::new();
        flat.resize_with(n_chunks.saturating_mul(n_outputs), || &mut []);
        for (out_idx, output) in outputs.iter_mut().enumerate() {
            for (chunk_idx, chunk) in output.chunks_mut(step).enumerate() {
                let slot_idx = chunk_idx.saturating_mul(n_outputs).saturating_add(out_idx);
                if let Some(slot) = flat.get_mut(slot_idx) {
                    *slot = chunk;
                }
            }
        }
        flat
    }
}

impl<Value: EvaluationValue> ProgramData<Value> {
    /// Shared parallel Rayon batch orchestration for both VM and native JIT backends.
    #[cfg(feature = "rayon")]
    pub fn run_batch_par<Target: ?Sized + BatchTarget<Value>>(
        data: &Arc<Self>,
        columns: &[&[Value]],
        target: &mut Target,
        prepare_lanes: impl FnOnce() -> usize,
        run_single_row: impl FnOnce(&mut Workspace<Value>, &mut Target),
        run_chunk: impl Fn(&mut Workspace<Value>, bool, (usize, usize), &mut Target::Chunk<'_>)
        + Send
        + Sync,
    ) -> Result<(), EvalError>
    where
        Value: Send + Sync,
    {
        let rows = target.validate(data, columns)?;
        if rows == 0 {
            return Ok(());
        }
        if rows == 1 {
            let mut workspace = Workspace::new();
            run_single_row(&mut workspace, target);
            return Ok(());
        }
        let lanes = prepare_lanes();
        let chunk_len = Self::par_chunk_len(rows, lanes);
        // SAFETY: `target.validate` proved all column and output lengths equal `rows`.
        unsafe {
            target.par_for_each(rows, chunk_len, |workspace, offset, chunk| {
                let chunk_rows = rows.saturating_sub(offset).min(chunk_len);
                run_chunk(
                    workspace,
                    offset == 0 && chunk_rows == rows,
                    (offset, chunk_rows),
                    chunk,
                );
            });
        }
        Ok(())
    }

    /// Evaluate validated `columns` at rows `offset..offset + rows` in blocks of
    /// `LANES` rows into `target`, copying only active parameter columns.
    ///
    /// # Safety
    /// `columns.len() == data.parameters.len()`, `rows > 0`, `target` was validated
    /// for `rows`, and every column has at least `offset + rows` entries.
    #[inline]
    pub unsafe fn eval_lanes<const LANES: usize, Target: ?Sized + BatchTarget<Value>>(
        data: &Arc<Self>,
        workspace: &mut Workspace<Value>,
        transient: bool,
        columns: &[&[Value]],
        (offset, rows): (usize, usize),
        target: &mut Target,
        mut run: impl FnMut(&mut [[Value; LANES]]),
    ) {
        if data.results.is_empty() || rows == 0 {
            return;
        }
        let all_active = data.active_parameters.len() == data.parameters.len();
        let registers = if transient {
            workspace.prepare_transient::<LANES>(data)
        } else {
            workspace.prepare::<LANES>(data)
        };
        let full_blocks = rows.checked_div(LANES).unwrap_or(0);
        let rem = rows.checked_rem(LANES).unwrap_or(0);
        for block in 0..full_blocks {
            let start = offset.saturating_add(block.saturating_mul(LANES));
            let end = start.saturating_add(LANES);
            if all_active {
                for (slots, column) in registers.iter_mut().zip(columns) {
                    // SAFETY: caller's contract proves `end <= offset + rows <= column.len()`.
                    let slice = unsafe { column.get_unchecked(start..end) };
                    slots.clone_from_slice(slice);
                }
            } else {
                for &param_u32 in &data.active_parameters {
                    let idx = usize::try_from(param_u32).unwrap_or(0);
                    // SAFETY: `idx` is bounded by `validate` and `end <= column.len()`.
                    unsafe {
                        registers
                            .get_unchecked_mut(idx)
                            .clone_from_slice(columns.get_unchecked(idx).get_unchecked(start..end));
                    }
                }
            }
            run(registers);
            // SAFETY: `(block + 1) * LANES <= rows` and `data.results` was validated.
            unsafe {
                target.write_full::<LANES>(&data.results, registers, block);
            }
        }
        if rem != 0 {
            let local_start = full_blocks.saturating_mul(LANES);
            let start = offset.saturating_add(local_start);
            let last = offset.saturating_add(rows.saturating_sub(1));
            if all_active {
                for (slots, column) in registers.iter_mut().zip(columns) {
                    for (lane, slot) in slots.iter_mut().enumerate() {
                        let row_idx = start.saturating_add(lane).min(last);
                        // SAFETY: `rows > 0` and `last < column.len()`.
                        let src = unsafe { column.get_unchecked(row_idx) };
                        slot.clone_from(src);
                    }
                }
            } else {
                for &param_u32 in &data.active_parameters {
                    let idx = usize::try_from(param_u32).unwrap_or(0);
                    // SAFETY: `idx` is bounded by `validate` and `last < column.len()`.
                    unsafe {
                        let column = columns.get_unchecked(idx);
                        for (lane, slot) in registers.get_unchecked_mut(idx).iter_mut().enumerate()
                        {
                            slot.clone_from(
                                column.get_unchecked(start.saturating_add(lane).min(last)),
                            );
                        }
                    }
                }
            }
            run(registers);
            // SAFETY: `local_start + rem == rows` and `rem < LANES`.
            unsafe {
                target.write_tail::<LANES>(&data.results, registers, local_start, rem);
            }
        }
    }
}
