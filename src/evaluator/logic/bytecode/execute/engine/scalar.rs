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

use super::CompiledEvaluator;
use super::builtins::{eval_builtin1, eval_builtin2, eval_builtin3, eval_builtin4};
use crate::evaluator::FnOp;
use std::cell::RefCell;
use std::ptr::{copy_nonoverlapping, write_bytes};

const INLINE_REGISTER_SIZE_SMALL: usize = 64;
const INLINE_REGISTER_SIZE_MEDIUM: usize = 128;
const INLINE_REGISTER_SIZE_LARGE: usize = 256;

thread_local! {
    static HEAP_REGISTERS: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}
#[cfg(feature = "parallel")]
use crate::core::error::DiffError;
#[cfg(feature = "parallel")]
use wide::f64x4;

impl CompiledEvaluator {
    /// Internal scalar execution loop.
    #[allow(
        clippy::too_many_lines,
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
                |_| {
                    let mut fallback_registers = vec![0.0; self.workspace_size];
                    self.evaluate_heap(params, &mut fallback_registers)
                },
                |mut registers| {
                    if registers.len() < self.workspace_size {
                        registers.resize(self.workspace_size, 0.0);
                    }
                    self.evaluate_heap(params, &mut registers[..self.workspace_size])
                },
            )
        })
    }

    #[allow(
        clippy::inline_always,
        reason = "Critical path optimization to avoid call overhead in evaluation setup"
    )]
    #[inline(always)]
    fn setup_registers(&self, params: &[f64], registers: *mut f64) {
        unsafe {
            let p = self.param_count.min(params.len());
            copy_nonoverlapping(params.as_ptr(), registers, p);

            // Restore pre-optimization safety: missing parameters must default to 0.0
            if p < self.param_count {
                write_bytes(registers.add(p), 0, self.param_count - p);
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
        use std::mem::MaybeUninit;

        let mut raw = [MaybeUninit::<f64>::uninit(); N];
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
    pub fn evaluate_heap(&self, params: &[f64], registers: &mut [f64]) -> f64 {
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
    /// Returns `DiffError` if the input columns do not match the expected parameter count
    /// or if the columns have inconsistent lengths.
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
        let mut eval_inner = |workspace: &mut [f64]| {
            let ptr = workspace.as_mut_ptr();
            let c = self.constants.len();
            unsafe {
                if c > 0 {
                    copy_nonoverlapping(self.constants.as_ptr(), ptr.add(self.param_count), c);
                }
            }

            let provided_cols = self.param_count.min(columns.len());

            // Pre-fetch column pointers to avoid redundant slicing/indexing in the inner loop.
            let mut col_ptrs = Vec::with_capacity(provided_cols);
            let mut col_lens = Vec::with_capacity(provided_cols);
            for column in columns.iter().take(provided_cols) {
                col_ptrs.push(column.as_ptr());
                col_lens.push(column.len());
            }

            // Zero out missing parameter columns once per batch
            for col_idx in provided_cols..self.param_count {
                unsafe {
                    *ptr.add(col_idx) = 0.0;
                }
            }

            for (i, out) in output.iter_mut().enumerate() {
                for col_idx in 0..provided_cols {
                    let col_ptr = col_ptrs[col_idx];
                    let col_len = col_lens[col_idx];
                    let val = if i < col_len {
                        unsafe { *col_ptr.add(i) }
                    } else {
                        unsafe { *col_ptr.add(col_len - 1) }
                    };
                    unsafe {
                        *ptr.add(col_idx) = val;
                    }
                }
                unsafe {
                    Self::exec_instructions(&self.flat_bytecode, ptr, &self.arg_pool);
                    *out = *ptr.add(self.result_reg as usize);
                }
            }
        };

        HEAP_REGISTERS.with(|heap_registers| {
            if let Ok(mut registers) = heap_registers.try_borrow_mut() {
                if registers.len() < self.workspace_size {
                    registers.resize(self.workspace_size, 0.0);
                }
                eval_inner(&mut registers[..self.workspace_size]);
            } else {
                let mut workspace = vec![0.0; self.workspace_size];
                eval_inner(&mut workspace);
            }
        });
    }
}
