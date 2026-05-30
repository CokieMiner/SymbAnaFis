//! Scalar evaluation engine for the register-based evaluator.
#![allow(unsafe_op_in_unsafe_fn, reason = "Internal unsafe operations allowed")]
#![allow(
    unsafe_code,
    reason = "High-performance evaluation uses raw pointers and unchecked indexing to maximize throughput. Safety is maintained by the compiler's static analysis of register bounds."
)]
#![allow(
    clippy::undocumented_unsafe_blocks,
    reason = "Internal unsafe operations allowed"
)]

#[cfg(feature = "parallel")]
use super::MAX_INLINE_PARAMS;
use super::VmEvaluator;
use super::builtins::{eval_builtin1, eval_builtin2, eval_builtin3, eval_builtin4};
use crate::evaluator::FnOp;
use std::cell::RefCell;
use std::mem::MaybeUninit;
use std::ptr::copy_nonoverlapping;
#[cfg(feature = "parallel")]
use std::ptr::null;
const INLINE_REGISTER_SIZE_SMALL: usize = 64;
const INLINE_REGISTER_SIZE_MEDIUM: usize = 128;
const INLINE_REGISTER_SIZE_LARGE: usize = 256;

#[cfg(feature = "parallel")]
pub(super) struct ColumnRefs {
    ptrs_buf: [*const f64; MAX_INLINE_PARAMS],
    lens_buf: [usize; MAX_INLINE_PARAMS],
    ptrs_vec: Vec<*const f64>,
    lens_vec: Vec<usize>,
    provided: usize,
}

#[cfg(feature = "parallel")]
impl ColumnRefs {
    pub(super) fn new(columns: &[&[f64]], provided_cols: usize) -> Self {
        if provided_cols <= MAX_INLINE_PARAMS {
            let mut ptrs_buf = [null(); MAX_INLINE_PARAMS];
            let mut lens_buf = [0_usize; MAX_INLINE_PARAMS];
            for (i, col) in columns.iter().take(provided_cols).enumerate() {
                ptrs_buf[i] = col.as_ptr();
                lens_buf[i] = col.len();
            }
            Self {
                ptrs_buf,
                lens_buf,
                ptrs_vec: Vec::new(),
                lens_vec: Vec::new(),
                provided: provided_cols,
            }
        } else {
            let mut ptrs_vec = Vec::with_capacity(provided_cols);
            let mut lens_vec = Vec::with_capacity(provided_cols);
            for col in columns.iter().take(provided_cols) {
                ptrs_vec.push(col.as_ptr());
                lens_vec.push(col.len());
            }
            Self {
                ptrs_buf: [null(); MAX_INLINE_PARAMS],
                lens_buf: [0; MAX_INLINE_PARAMS],
                ptrs_vec,
                lens_vec,
                provided: provided_cols,
            }
        }
    }

    pub(super) fn as_refs(&self) -> (&[*const f64], &[usize]) {
        if self.provided <= MAX_INLINE_PARAMS {
            (
                &self.ptrs_buf[..self.provided],
                &self.lens_buf[..self.provided],
            )
        } else {
            (&self.ptrs_vec, &self.lens_vec)
        }
    }
}

thread_local! {
    /// Thread-local workspace for scalar evaluation.
    ///
    /// # Safety & Re-entrancy
    ///
    /// This workspace uses a `RefCell` to ensure exclusive access during evaluation.
    /// In case of re-entrant evaluation (e.g. a custom builtin function calling back into
    /// the evaluator), `try_borrow_mut` will fail, and the evaluator will fall back to
    /// an on-stack heap allocation. This prevents `BorrowError` while maintaining
    /// high performance for the common non-recursive case.
    static HEAP_REGISTERS: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}
#[cfg(feature = "parallel")]
use crate::core::error::DiffError;
#[cfg(feature = "parallel")]
use wide::f64x4;

impl VmEvaluator {
    /// Internal scalar execution loop.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::inline_always,
        reason = "Unified dispatch loop"
    )]
    #[inline(always)]
    pub(crate) unsafe fn exec_instructions(
        bytecode: &[u32],
        registers: *mut f64,
        arg_pool: &[u32],
    ) {
        let one = 1.0_f64;
        dispatch_loop!(
            bytecode,
            registers,
            arg_pool,
            scalar,
            one,
            eval_builtin1,
            eval_builtin2,
            eval_builtin3,
            eval_builtin4
        );
    }

    /// Evaluates the compiled expression at a single point.
    #[inline]
    #[must_use]
    pub fn evaluate(&self, params: &[f64]) -> f64 {
        evaluate_staircase!(
            self,
            params,
            [
                INLINE_REGISTER_SIZE_SMALL,
                INLINE_REGISTER_SIZE_MEDIUM,
                INLINE_REGISTER_SIZE_LARGE
            ]
        );

        HEAP_REGISTERS.with(|heap_registers| {
            heap_registers.try_borrow_mut().map_or_else(
                |_| self.evaluate_with_fallback(params),
                |mut registers| {
                    if registers.len() < self.workspace_size {
                        registers.resize(self.workspace_size, 0.0);
                    }
                    self.evaluate_heap(params, &mut registers[..self.workspace_size])
                },
            )
        })
    }

    /// Cold path: re-entrant evaluation fallback (e.g. custom builtin calling back into evaluator).
    #[cold]
    #[inline(never)]
    fn evaluate_with_fallback(&self, params: &[f64]) -> f64 {
        let mut fallback_registers = vec![0.0; self.workspace_size];
        self.evaluate_heap(params, &mut fallback_registers)
    }

    #[allow(
        clippy::inline_always,
        reason = "Critical path optimization to avoid call overhead in evaluation setup"
    )]
    #[inline(always)]
    fn setup_registers(&self, params: &[f64], registers: *mut f64) {
        assert!(
            self.param_count + self.constants.len() <= self.workspace_size,
            "setup_registers: buffer overflow — workspace_size {} < required {}",
            self.workspace_size,
            self.param_count + self.constants.len(),
        );
        unsafe {
            let p = self.param_count.min(params.len());
            copy_nonoverlapping(params.as_ptr(), registers, p);

            // Restore pre-optimization safety: missing parameters must default to 0.0
            for i in p..self.param_count {
                registers.add(i).write(0.0);
            }

            let c = self.constants.len();
            if c > 0 {
                copy_nonoverlapping(self.constants.as_ptr(), registers.add(self.param_count), c);
            }
        }
    }

    #[allow(
        clippy::inline_always,
        reason = "Staircase dispatch relies on forced inlining to avoid call overhead"
    )]
    #[inline(always)]
    fn evaluate_inline<const N: usize>(&self, params: &[f64]) -> f64 {
        let mut raw: [MaybeUninit<f64>; N] = unsafe { MaybeUninit::uninit().assume_init() };
        // Safety net in debug builds: zero the register file so that any
        // compiler bug causing a read-before-write produces a predictable
        // 0.0 rather than silent UB (uninitialized memory read).
        #[cfg(debug_assertions)]
        {
            for el in &mut raw {
                el.write(0.0);
            }
        }
        // SAFETY: The compiler's register allocator guarantees that any register
        // is written to before it is read. Parameters and constants are initialized below.
        let ptr = raw.as_mut_ptr().cast::<f64>();
        self.setup_registers(params, ptr);
        unsafe {
            Self::exec_instructions(&self.flat_bytecode, ptr, &self.arg_pool);
            *ptr.add(self.result_reg as usize)
        }
    }

    /// Evaluates the expression using a provided mutable workspace.
    ///
    /// This is used for large expressions that exceed the stack-allocation staircase,
    /// or in parallel workloads where workspaces are managed by a driver.
    #[inline]
    pub(crate) fn evaluate_heap(&self, params: &[f64], registers: &mut [f64]) -> f64 {
        assert!(
            registers.len() >= self.workspace_size,
            "evaluate_heap: workspace too small: got {}, need {}",
            registers.len(),
            self.workspace_size
        );
        let ptr = registers.as_mut_ptr();
        self.setup_registers(params, ptr);
        unsafe {
            Self::exec_instructions(&self.flat_bytecode, ptr, &self.arg_pool);
            *ptr.add(self.result_reg as usize)
        }
    }

    /// Evaluate a batch of data points.
    ///
    /// # Errors
    ///
    /// Missing parameter columns and short columns are filled with `0.0`; values
    /// past `output.len()` are ignored.
    #[cfg(feature = "parallel")]
    pub fn eval_batch(
        &self,
        columns: &[&[f64]],
        output: &mut [f64],
        simd_workspace: Option<&mut [f64x4]>,
    ) -> Result<(), DiffError> {
        let n_points = output.len();
        if n_points == 0 {
            return Ok(());
        }

        // Use SIMD if available and we have a workspace
        if let Some(workspace) = simd_workspace {
            self.eval_batch_simd(columns, output, workspace);
        } else {
            self.eval_batch_scalar(columns, output);
        }

        Ok(())
    }

    #[cfg(feature = "parallel")]
    pub(crate) fn eval_batch_scalar(&self, columns: &[&[f64]], output: &mut [f64]) {
        let provided_cols = self.param_count.min(columns.len());

        let col_refs = ColumnRefs::new(columns, provided_cols);
        let (col_ptrs, col_lens) = col_refs.as_refs();

        HEAP_REGISTERS.with(|heap_registers| {
            if let Ok(mut registers) = heap_registers.try_borrow_mut() {
                if registers.len() < self.workspace_size {
                    registers.resize(self.workspace_size, 0.0);
                }
                self.eval_points_into(
                    col_ptrs,
                    col_lens,
                    output,
                    &mut registers[..self.workspace_size],
                    0,
                );
            } else {
                self.eval_batch_with_fallback(col_ptrs, col_lens, output);
            }
        });
    }

    /// Cold path: re-entrant batch evaluation fallback.
    #[cfg(feature = "parallel")]
    #[cold]
    #[inline(never)]
    fn eval_batch_with_fallback(
        &self,
        col_ptrs: &[*const f64],
        col_lens: &[usize],
        output: &mut [f64],
    ) {
        let mut workspace = vec![0.0; self.workspace_size];
        self.eval_points_into(col_ptrs, col_lens, output, &mut workspace, 0);
    }

    /// Evaluate a slice of points into `output`, reading column data starting at
    /// absolute row offset `offset`. All workspace and column initialization is
    /// handled internally.
    #[cfg(feature = "parallel")]
    pub(crate) fn eval_points_into(
        &self,
        col_ptrs: &[*const f64],
        col_lens: &[usize],
        output: &mut [f64],
        workspace: &mut [f64],
        offset: usize,
    ) {
        let provided_cols = col_ptrs.len();
        let ptr = workspace.as_mut_ptr();
        let c = self.constants.len();
        unsafe {
            if c > 0 {
                copy_nonoverlapping(self.constants.as_ptr(), ptr.add(self.param_count), c);
            }
        }
        for col_idx in provided_cols..self.param_count {
            unsafe {
                *ptr.add(col_idx) = 0.0;
            }
        }
        for (k, out) in output.iter_mut().enumerate() {
            let j = offset + k;
            for ci in 0..provided_cols {
                let cl = col_lens[ci];
                let val = if j < cl {
                    unsafe { *col_ptrs[ci].add(j) }
                } else {
                    0.0
                };
                unsafe {
                    *ptr.add(ci) = val;
                }
            }
            unsafe {
                Self::exec_instructions(&self.flat_bytecode, ptr, &self.arg_pool);
            }
            *out = unsafe { *ptr.add(self.result_reg as usize) };
        }
    }
}
