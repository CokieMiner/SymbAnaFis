#![allow(unsafe_op_in_unsafe_fn, reason = "Internal unsafe operations allowed")]
#![allow(
    unsafe_code,
    reason = "Vectorized evaluation uses raw pointers and unchecked indexing to maximize throughput. Safety is maintained by the compiler's static analysis of register bounds."
)]
#![allow(
    clippy::undocumented_unsafe_blocks,
    reason = "Internal unsafe operations allowed"
)]

use super::CompiledEvaluator;
use super::builtins::{
    eval_builtin1_simd, eval_builtin2_simd, eval_builtin3_simd, eval_builtin4_simd,
};
use crate::evaluator::FnOp;
use wide::f64x4;

impl CompiledEvaluator {
    /// Internal SIMD execution loop.
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::inline_always,
        reason = "Unified dispatch loop"
    )]
    #[inline(always)]
    pub(crate) unsafe fn exec_simd_instructions(
        bytecode: &[u32],
        registers: *mut f64x4,
        arg_pool: &[u32],
    ) {
        let one = f64x4::splat(1.0);
        dispatch_loop!(
            bytecode,
            registers,
            arg_pool,
            simd,
            one,
            eval_builtin1_simd,
            eval_builtin2_simd,
            eval_builtin3_simd,
            eval_builtin4_simd
        );
    }

    /// Evaluate a batch of data points using SIMD.
    ///
    /// # Panics
    ///
    /// Panics if the internal slice chunking fails to provide exactly 4 elements
    /// to the SIMD vectors, which is statically guarded by the chunk loop.
    #[cfg(feature = "parallel")]
    pub fn eval_batch_simd(&self, columns: &[&[f64]], output: &mut [f64], workspace: &mut [f64x4]) {
        let n_points = output.len();
        // SIMD operates on 4 lanes at a time, so we process in chunks of 4.
        let n_lanes = 4;

        // Prepare workspace with constants (no need to zero the rest)
        // Constants are located immediately after parameters.
        for (i, &val) in self.constants.iter().enumerate() {
            workspace[self.param_count + i] = f64x4::splat(val);
        }

        let mut i = 0;
        let provided_cols = self.param_count.min(columns.len());

        // Fill missing parameter columns with 0.0 once per batch
        for out_val in &mut workspace[provided_cols..self.param_count] {
            *out_val = f64x4::splat(0.0);
        }

        while i + n_lanes <= n_points {
            // Fill provided parameter columns
            for (col_idx, out_val) in workspace[..provided_cols].iter_mut().enumerate() {
                let col = unsafe { *columns.get_unchecked(col_idx) };
                if i + n_lanes <= col.len() {
                    *out_val = f64x4::from(unsafe { *(col.as_ptr().add(i).cast::<[f64; 4]>()) });
                } else {
                    *out_val = f64x4::splat(unsafe { *col.get_unchecked(col.len() - 1) });
                }
            }

            unsafe {
                Self::exec_simd_instructions(
                    &self.flat_bytecode,
                    workspace.as_mut_ptr(),
                    &self.arg_pool,
                );
            }

            let res: [f64; 4] = workspace[self.result_reg as usize].to_array();
            output[i..i + n_lanes].copy_from_slice(&res);
            i += n_lanes;
        }

        // Tail handling
        if i < n_points {
            let tail_cols: Vec<&[f64]> = columns
                .iter()
                .map(|c| {
                    if i < c.len() {
                        &c[i..]
                    } else if !c.is_empty() {
                        &c[c.len() - 1..]
                    } else {
                        &[]
                    }
                })
                .collect();

            // Just use scalar for the tail without allocating a new output buffer
            self.eval_batch_scalar(&tail_cols, &mut output[i..]);
        }
    }
}
