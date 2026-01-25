//! SIMD batch evaluation for the bytecode evaluator.
//!
//! This module provides vectorized evaluation using `wide::f64x4` (4-wide SIMD).
//! Batch evaluation processes 4 data points simultaneously, providing ~2-4x
//! speedup over scalar evaluation for large datasets.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                     eval_batch                           │
//! ├─────────────────────────────────────────────────────────┤
//! │  Data:   [p0, p1, p2, p3, p4, p5, p6, p7, p8, p9]       │
//! │                │                   │           │         │
//! │          ┌─────┴─────┐       ┌─────┴─────┐   ┌─┴─┐      │
//! │          │ SIMD Chunk│       │ SIMD Chunk│   │Rem│      │
//! │          │ [p0-p3]   │       │ [p4-p7]   │   │p8-│      │
//! │          │  f64x4    │       │  f64x4    │   │p9 │      │
//! │          └───────────┘       └───────────┘   └───┘      │
//! │                                               │          │
//! │                                          Scalar path     │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Performance Notes
//!
//! - Full SIMD chunks (4 points) use vectorized operations
//! - Remainder points (1-3) fall back to scalar evaluation
//! - CSE cache is maintained per-chunk for correctness
//! - Memory layout is columnar for cache-friendly access

use super::CompiledEvaluator;
use super::instruction::Instruction;
use super::stack;
use crate::core::error::DiffError;
use std::mem::MaybeUninit;
use wide::f64x4;

impl CompiledEvaluator {
    /// Batch evaluation - evaluate expression at multiple data points.
    ///
    /// This method processes all data points in a single call, moving the evaluation
    /// loop inside the VM for better cache locality. Data is expected in columnar format:
    /// each slice in `columns` corresponds to one parameter (in `param_names()` order),
    /// and each element within a column is a data point.
    ///
    /// # Parameters
    ///
    /// - `columns`: Columnar data, where `columns[param_idx][point_idx]` gives the value
    ///   of parameter `param_idx` at data point `point_idx`
    /// - `output`: Mutable slice to write results, must have length >= number of data points
    /// - `simd_buffer`: Optional pre-allocated SIMD stack buffer for reuse. When `Some`,
    ///   the provided buffer is reused across calls (ideal for parallel evaluation with
    ///   `map_init`). When `None`, a temporary buffer is allocated per call.
    ///
    /// # Performance
    ///
    /// Pass `Some(&mut buffer)` when calling in a loop or parallel context to eliminate
    /// repeated memory allocations. Use `None` for one-off evaluations.
    ///
    /// # Errors
    ///
    /// - `EvalColumnMismatch` if `columns.len()` != `param_count()`
    /// - `EvalColumnLengthMismatch` if column lengths don't all match
    /// - `EvalOutputTooSmall` if `output.len()` < number of data points
    ///
    /// # Example
    ///
    /// ```
    /// use symb_anafis::{symb, CompiledEvaluator};
    ///
    /// let x = symb("x");
    /// let expr = x.pow(2.0) + 1.0;
    /// let eval = expr.compile().expect("compile");
    ///
    /// let x_vals = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    /// let columns: Vec<&[f64]> = vec![&x_vals];
    /// let mut output = vec![0.0; 8];
    ///
    /// eval.eval_batch(&columns, &mut output, None).expect("eval");
    /// // output = [2.0, 5.0, 10.0, 17.0, 26.0, 37.0, 50.0, 65.0]
    /// ```
    #[allow(clippy::too_many_lines)]
    #[inline]
    pub fn eval_batch(
        &self,
        columns: &[&[f64]],
        output: &mut [f64],
        simd_buffer: Option<&mut Vec<f64x4>>,
    ) -> Result<(), DiffError> {
        // Validate input dimensions
        if columns.len() != self.param_names.len() {
            return Err(DiffError::EvalColumnMismatch {
                expected: self.param_names.len(),
                got: columns.len(),
            });
        }

        let n_points = if columns.is_empty() {
            1
        } else {
            columns[0].len()
        };

        if !columns.iter().all(|c| c.len() == n_points) {
            return Err(DiffError::EvalColumnLengthMismatch);
        }
        if output.len() < n_points {
            return Err(DiffError::EvalOutputTooSmall {
                needed: n_points,
                got: output.len(),
            });
        }

        // Process in chunks of 4 using SIMD
        // Intentional integer division for chunking
        #[allow(clippy::integer_division)]
        let full_chunks = n_points / 4;

        // Use provided buffer or create local one
        let mut local_stack;
        // Match is clearer for mutable reference handling
        #[allow(clippy::option_if_let_else, clippy::single_match_else)]
        let simd_stack: &mut Vec<f64x4> = match simd_buffer {
            Some(buf) => buf,
            None => {
                local_stack = Vec::with_capacity(self.stack_size);
                &mut local_stack
            }
        };

        // CSE cache for SIMD evaluation - allocated once outside loop
        // Avoiding "Zero Tax" by using with_capacity + set_len on MaybeUninit
        let mut simd_cache: Vec<MaybeUninit<f64x4>> = Vec::with_capacity(self.cache_size);
        // SAFETY: The compiler ensures that bytecode only reads from cache slots
        // that have been previously written to by a StoreCached instruction.
        unsafe {
            simd_cache.set_len(self.cache_size);
        }
        // SAFETY: Casting MaybeUninit slice to initialized slice is safe here because
        // the bytecode guarantees all reads are preceded by writes.
        let simd_cache_slice: &mut [f64x4] = unsafe {
            std::slice::from_raw_parts_mut(simd_cache.as_mut_ptr().cast::<f64x4>(), self.cache_size)
        };

        // Pre-fetch constants for better cache locality
        let constants = &self.constants;
        let instructions = &self.instructions;

        // Process full SIMD chunks
        for chunk in 0..full_chunks {
            let base = chunk * 4;
            simd_stack.clear();

            for instr in instructions {
                Self::exec_simd_instruction(
                    *instr,
                    simd_stack,
                    simd_cache_slice,
                    columns,
                    constants,
                    base,
                );
            }

            // Extract results from SIMD vector
            debug_assert!(!simd_stack.is_empty(), "SIMD stack empty after evaluation");
            // SAFETY: Stack is non-empty, bytecode always leaves one result
            let result = unsafe { *simd_stack.get_unchecked(simd_stack.len() - 1) };
            let arr = result.to_array();
            // SAFETY: output.len() >= n_points validated at function entry
            unsafe {
                *output.get_unchecked_mut(base) = arr[0];
                *output.get_unchecked_mut(base + 1) = arr[1];
                *output.get_unchecked_mut(base + 2) = arr[2];
                *output.get_unchecked_mut(base + 3) = arr[3];
            }
        }

        // Handle remainder with scalar path (1-3 points)
        let remainder_start = full_chunks * 4;
        if remainder_start < n_points {
            // Define limits for stack-allocated buffers to avoid per-batch heap tax
            const LOCAL_MAX: usize = 64;

            // Local buffers on CPU stack
            let mut stack_buf: [std::mem::MaybeUninit<f64>; LOCAL_MAX] =
                [std::mem::MaybeUninit::uninit(); LOCAL_MAX];
            let mut cache_buf: [std::mem::MaybeUninit<f64>; LOCAL_MAX] =
                [std::mem::MaybeUninit::uninit(); LOCAL_MAX];

            // Buffer pointers and sizes - declared with explicit types to resolve inference issues
            let mut heap_stack: Vec<MaybeUninit<f64>>;
            let mut heap_cache: Vec<MaybeUninit<f64>>;

            let (stack_slice, cache_slice): (&mut [f64], &mut [f64]) =
                if self.stack_size <= LOCAL_MAX && self.cache_size <= LOCAL_MAX {
                    // SAFETY: The compiler ensures stack_size and cache_size are within limits.
                    // We only initialize the active portion of the buffers needed for this bytecode.
                    unsafe {
                        (
                            std::slice::from_raw_parts_mut(
                                stack_buf.as_mut_ptr().cast::<f64>(),
                                self.stack_size,
                            ),
                            std::slice::from_raw_parts_mut(
                                cache_buf.as_mut_ptr().cast::<f64>(),
                                self.cache_size,
                            ),
                        )
                    }
                } else {
                    heap_stack = Vec::with_capacity(self.stack_size);
                    heap_cache = Vec::with_capacity(self.cache_size);
                    // SAFETY: Buffers are reserved and will only be read from after being written
                    // to by the evaluation logic, as guaranteed by the compiler's depth tracking.
                    // Using MaybeUninit pointers for cast to satisfy Clippy.
                    unsafe {
                        heap_stack.set_len(self.stack_size);
                        heap_cache.set_len(self.cache_size);
                    }
                    (
                        // SAFETY: Slices are created from uniquely owned Vecs, guaranteed to be valid
                        // for the duration of this evaluation block.
                        unsafe {
                            std::slice::from_raw_parts_mut(
                                heap_stack.as_mut_ptr().cast::<f64>(),
                                self.stack_size,
                            )
                        },
                        // SAFETY: Same as above for the cache slice.
                        unsafe {
                            std::slice::from_raw_parts_mut(
                                heap_cache.as_mut_ptr().cast::<f64>(),
                                self.cache_size,
                            )
                        },
                    )
                };

            for (i, out) in output[remainder_start..n_points].iter_mut().enumerate() {
                let point_idx = remainder_start + i;
                let mut len = 0_usize;

                for instr in instructions {
                    Self::exec_scalar_batch_instruction(
                        *instr,
                        stack_slice,
                        &mut len,
                        cache_slice,
                        columns,
                        constants,
                        point_idx,
                    );
                }
                *out = if len > 0 {
                    stack_slice[len - 1]
                } else {
                    f64::NAN
                };
            }
        }

        Ok(())
    }

    /// Parallel batch evaluation - evaluate expression at multiple data points in parallel.
    ///
    /// Similar to `eval_batch`, but processes data points in parallel using Rayon.
    /// Best for large datasets (>256 points) where parallel overhead is justified.
    ///
    /// # Parameters
    ///
    /// - `columns`: Columnar data, where `columns[param_idx][point_idx]` gives the value
    ///   of parameter `param_idx` at data point `point_idx`
    ///
    /// # Returns
    ///
    /// Vec of evaluation results for each data point.
    ///
    /// # Errors
    ///
    /// - `EvalColumnMismatch` if `columns.len()` != `param_count()`
    /// - `EvalColumnLengthMismatch` if column lengths don't all match
    ///
    /// # Panics
    ///
    /// Panics if `eval_batch` fails internally (indicates a compiler bug).
    #[allow(clippy::items_after_statements)] // MIN_PARALLEL_SIZE placed near its usage for clarity
    #[cfg(feature = "parallel")]
    pub fn eval_batch_parallel(&self, columns: &[&[f64]]) -> Result<Vec<f64>, DiffError> {
        use rayon::prelude::*;

        if columns.len() != self.param_names.len() {
            return Err(DiffError::EvalColumnMismatch {
                expected: self.param_names.len(),
                got: columns.len(),
            });
        }

        let n_points = if columns.is_empty() {
            1
        } else {
            columns[0].len()
        };

        if !columns.iter().all(|c| c.len() == n_points) {
            return Err(DiffError::EvalColumnLengthMismatch);
        }

        // For small point counts, fall back to sequential to avoid overhead
        // 256 * 8 bytes = 2KB per column, fits in L1 cache for better locality
        const MIN_PARALLEL_SIZE: usize = 256;
        if n_points < MIN_PARALLEL_SIZE {
            let mut output = vec![0.0; n_points];
            self.eval_batch(columns, &mut output, None)?;
            return Ok(output);
        }

        // Parallel chunked evaluation with thread-local SIMD buffer reuse
        let mut output = vec![0.0; n_points];
        let chunk_indices: Vec<usize> = (0..n_points).step_by(MIN_PARALLEL_SIZE).collect();

        let chunk_results: Vec<(usize, Vec<f64>)> = chunk_indices
            .into_par_iter()
            .map_init(
                || Vec::with_capacity(self.stack_size),
                |simd_buffer, start| {
                    let end = (start + MIN_PARALLEL_SIZE).min(n_points);
                    let len = end - start;
                    let col_slices: Vec<&[f64]> =
                        columns.iter().map(|col| &col[start..end]).collect();
                    let mut chunk_out = vec![0.0; len];
                    // SAFETY: Column count and lengths validated at function entry.
                    // eval_batch can only fail on those conditions, so this is unreachable.
                    self.eval_batch(&col_slices, &mut chunk_out, Some(simd_buffer))
                        .expect("eval_batch failed: column validation should have caught this");
                    (start, chunk_out)
                },
            )
            .collect();

        for (start, chunk_out) in chunk_results {
            output[start..start + chunk_out.len()].copy_from_slice(&chunk_out);
        }
        Ok(output)
    }

    /// Execute a SIMD instruction on the stack.
    ///
    /// Most common instructions are inlined here for performance.
    /// Less common instructions fall through to the slow path.
    ///
    /// # Safety Invariant
    ///
    /// All unsafe stack operations have identical safety requirements:
    /// - Stack depth validated at compile time by the `Compiler`
    /// - Uses same bytecode as scalar path with verified stack effects
    /// - Debug builds include runtime assertions
    //
    // Allow undocumented_unsafe_blocks: All ~30 unsafe blocks share the same invariant.
    // Allow too_many_lines: This is a dispatch function, splitting would hurt cache locality.
    #[allow(clippy::undocumented_unsafe_blocks)]
    #[allow(clippy::too_many_lines)]
    #[inline]
    fn exec_simd_instruction(
        instr: Instruction,
        stack: &mut Vec<f64x4>,
        cache: &mut [f64x4],
        columns: &[&[f64]],
        constants: &[f64],
        base: usize,
    ) {
        match instr {
            // Memory operations
            Instruction::LoadConst(idx) => {
                stack.push(f64x4::splat(constants[idx as usize]));
            }
            Instruction::LoadParam(p) => {
                let col = columns[p as usize];
                stack.push(f64x4::new([
                    col[base],
                    col[base + 1],
                    col[base + 2],
                    col[base + 3],
                ]));
            }

            // Binary operations
            Instruction::Add => unsafe { stack::simd_stack_binop_add(stack) },
            Instruction::Mul => unsafe { stack::simd_stack_binop_mul(stack) },
            Instruction::Div => unsafe { stack::simd_stack_binop_div(stack) },
            Instruction::Pow => unsafe { stack::simd_stack_binop_pow(stack) },

            // Unary operations
            Instruction::Neg => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                *top = -*top;
            }
            Instruction::Sin => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                *top = top.sin();
            }
            Instruction::Cos => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                *top = top.cos();
            }
            Instruction::Tan => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                *top = top.tan();
            }
            Instruction::Sqrt => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                *top = top.sqrt();
            }
            Instruction::Exp => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                *top = top.exp();
            }
            Instruction::Ln => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                *top = top.ln();
            }
            Instruction::Abs => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                *top = top.abs();
            }

            // Fused operations
            Instruction::Square => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                *top = *top * *top;
            }
            Instruction::Cube => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                let x = *top;
                *top = x * x * x;
            }
            Instruction::Pow4 => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                let x = *top;
                let x2 = x * x;
                *top = x2 * x2;
            }
            Instruction::Recip => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                *top = f64x4::splat(1.0) / *top;
            }
            Instruction::Powi(n) => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                let arr = top.to_array();
                let n_i32 = n;
                *top = f64x4::new([
                    arr[0].powi(n_i32),
                    arr[1].powi(n_i32),
                    arr[2].powi(n_i32),
                    arr[3].powi(n_i32),
                ]);
            }
            Instruction::SinCos => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                let arr = top.to_array();
                let (s0, c0) = arr[0].sin_cos();
                let (s1, c1) = arr[1].sin_cos();
                let (s2, c2) = arr[2].sin_cos();
                let (s3, c3) = arr[3].sin_cos();
                *top = f64x4::new([c0, c1, c2, c3]);
                stack.push(f64x4::new([s0, s1, s2, s3]));
            }
            Instruction::MulAdd => unsafe { stack::simd_stack_muladd(stack) },

            // Hyperbolic - common in physics
            Instruction::Sinh => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                let arr = top.to_array();
                *top = f64x4::new([arr[0].sinh(), arr[1].sinh(), arr[2].sinh(), arr[3].sinh()]);
            }
            Instruction::Cosh => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                let arr = top.to_array();
                *top = f64x4::new([arr[0].cosh(), arr[1].cosh(), arr[2].cosh(), arr[3].cosh()]);
            }
            Instruction::Tanh => {
                let top = unsafe { stack::simd_stack_top_mut(stack) };
                let arr = top.to_array();
                *top = f64x4::new([arr[0].tanh(), arr[1].tanh(), arr[2].tanh(), arr[3].tanh()]);
            }

            // CSE instructions
            Instruction::Dup => {
                let top = unsafe { *stack::simd_stack_top_mut(stack) };
                stack.push(top);
            }
            Instruction::StoreCached(slot) => {
                cache[slot as usize] = unsafe { *stack::simd_stack_top_mut(stack) };
            }
            Instruction::LoadCached(slot) => {
                stack.push(cache[slot as usize]);
            }

            // Slow path for less common instructions
            _ => Self::exec_simd_slow_instruction(instr, stack),
        }
    }

    /// SIMD slow path for handling less common instructions.
    ///
    /// Falls back to scalar computation for each of the 4 lanes.
    ///
    /// # Safety Invariant
    ///
    /// Stack is guaranteed non-empty by compile-time validation. The `debug_assert!`
    /// at function entry catches any compiler bugs in debug builds.
    //
    // Allow too_many_lines: This is a dispatch function covering all special functions.
    // Splitting would hurt code locality without meaningful abstraction benefit.
    // Allow undocumented_unsafe_blocks: All unsafe blocks share the same invariant
    // documented above - stack non-empty validated by compiler at compile time.
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::undocumented_unsafe_blocks)]
    #[inline(never)]
    #[cold]
    fn exec_simd_slow_instruction(instr: Instruction, stack: &mut Vec<f64x4>) {
        debug_assert!(!stack.is_empty(), "Stack empty in SIMD slow path");

        // Helper to get top of stack - safety validated by debug_assert above
        // Using unsafe here avoids 49 bounds checks in this cold path
        macro_rules! top {
            () => {
                unsafe { stack::simd_stack_top_mut(stack) }
            };
        }

        // Helper to pop from stack - safety validated by debug_assert above
        macro_rules! pop {
            () => {
                unsafe { stack::simd_stack_pop(stack) }
            };
        }

        match instr {
            // Inverse trig
            Instruction::Asin => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([arr[0].asin(), arr[1].asin(), arr[2].asin(), arr[3].asin()]);
            }
            Instruction::Acos => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([arr[0].acos(), arr[1].acos(), arr[2].acos(), arr[3].acos()]);
            }
            Instruction::Atan => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([arr[0].atan(), arr[1].atan(), arr[2].atan(), arr[3].atan()]);
            }
            Instruction::Acot => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    stack::eval_acot(arr[0]),
                    stack::eval_acot(arr[1]),
                    stack::eval_acot(arr[2]),
                    stack::eval_acot(arr[3]),
                ]);
            }
            Instruction::Asec => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    (1.0 / arr[0]).acos(),
                    (1.0 / arr[1]).acos(),
                    (1.0 / arr[2]).acos(),
                    (1.0 / arr[3]).acos(),
                ]);
            }
            Instruction::Acsc => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    (1.0 / arr[0]).asin(),
                    (1.0 / arr[1]).asin(),
                    (1.0 / arr[2]).asin(),
                    (1.0 / arr[3]).asin(),
                ]);
            }
            Instruction::Cot => {
                let top = top!();
                *top = f64x4::ONE / top.tan();
            }
            Instruction::Sec => {
                let top = top!();
                *top = f64x4::ONE / top.cos();
            }
            Instruction::Csc => {
                let top = top!();
                *top = f64x4::ONE / top.sin();
            }

            // Inverse hyperbolic
            Instruction::Asinh => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    arr[0].asinh(),
                    arr[1].asinh(),
                    arr[2].asinh(),
                    arr[3].asinh(),
                ]);
            }
            Instruction::Acosh => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    arr[0].acosh(),
                    arr[1].acosh(),
                    arr[2].acosh(),
                    arr[3].acosh(),
                ]);
            }
            Instruction::Atanh => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    arr[0].atanh(),
                    arr[1].atanh(),
                    arr[2].atanh(),
                    arr[3].atanh(),
                ]);
            }
            Instruction::Coth => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    1.0 / arr[0].tanh(),
                    1.0 / arr[1].tanh(),
                    1.0 / arr[2].tanh(),
                    1.0 / arr[3].tanh(),
                ]);
            }
            Instruction::Sech => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    1.0 / arr[0].cosh(),
                    1.0 / arr[1].cosh(),
                    1.0 / arr[2].cosh(),
                    1.0 / arr[3].cosh(),
                ]);
            }
            Instruction::Csch => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    1.0 / arr[0].sinh(),
                    1.0 / arr[1].sinh(),
                    1.0 / arr[2].sinh(),
                    1.0 / arr[3].sinh(),
                ]);
            }
            Instruction::Acoth => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    stack::eval_acoth(arr[0]),
                    stack::eval_acoth(arr[1]),
                    stack::eval_acoth(arr[2]),
                    stack::eval_acoth(arr[3]),
                ]);
            }
            Instruction::Asech => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    (1.0 / arr[0]).acosh(),
                    (1.0 / arr[1]).acosh(),
                    (1.0 / arr[2]).acosh(),
                    (1.0 / arr[3]).acosh(),
                ]);
            }
            Instruction::Acsch => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    (1.0 / arr[0]).asinh(),
                    (1.0 / arr[1]).asinh(),
                    (1.0 / arr[2]).asinh(),
                    (1.0 / arr[3]).asinh(),
                ]);
            }

            // Log functions
            Instruction::Log10 => {
                let top = top!();
                *top = top.log10();
            }
            Instruction::Log2 => {
                let top = top!();
                *top = top.log2();
            }
            Instruction::Cbrt => {
                let top = top!();
                *top = top.pow_f64x4(f64x4::splat(1.0 / 3.0));
            }

            // Rounding
            Instruction::Floor => {
                let top = top!();
                *top = top.floor();
            }
            Instruction::Ceil => {
                let top = top!();
                *top = top.ceil();
            }
            Instruction::Round => {
                let top = top!();
                *top = top.round();
            }
            Instruction::Signum => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    arr[0].signum(),
                    arr[1].signum(),
                    arr[2].signum(),
                    arr[3].signum(),
                ]);
            }

            // Special functions - compute per-lane
            Instruction::Erf => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_erf(arr[0]),
                    crate::math::eval_erf(arr[1]),
                    crate::math::eval_erf(arr[2]),
                    crate::math::eval_erf(arr[3]),
                ]);
            }
            Instruction::Erfc => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    1.0 - crate::math::eval_erf(arr[0]),
                    1.0 - crate::math::eval_erf(arr[1]),
                    1.0 - crate::math::eval_erf(arr[2]),
                    1.0 - crate::math::eval_erf(arr[3]),
                ]);
            }
            Instruction::Gamma => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_gamma(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_gamma(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_gamma(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_gamma(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Digamma => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_digamma(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_digamma(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_digamma(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_digamma(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Trigamma => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_trigamma(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_trigamma(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_trigamma(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_trigamma(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Tetragamma => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_polygamma(3, arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_polygamma(3, arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_polygamma(3, arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_polygamma(3, arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Sinc => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    stack::eval_sinc(arr[0]),
                    stack::eval_sinc(arr[1]),
                    stack::eval_sinc(arr[2]),
                    stack::eval_sinc(arr[3]),
                ]);
            }
            Instruction::LambertW => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_lambert_w(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_lambert_w(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_lambert_w(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_lambert_w(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::EllipticK => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_elliptic_k(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_k(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_k(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_k(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::EllipticE => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_elliptic_e(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_e(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_e(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_elliptic_e(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::Zeta => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_zeta(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_zeta(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_zeta(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_zeta(arr[3]).unwrap_or(f64::NAN),
                ]);
            }
            Instruction::ExpPolar => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([
                    crate::math::eval_exp_polar(arr[0]),
                    crate::math::eval_exp_polar(arr[1]),
                    crate::math::eval_exp_polar(arr[2]),
                    crate::math::eval_exp_polar(arr[3]),
                ]);
            }

            // Two-argument functions
            Instruction::Log => {
                let x = pop!();
                let base = top!();
                let x_arr = x.to_array();
                let base_arr = base.to_array();
                let log_fn = |b: f64, v: f64| -> f64 {
                    #[allow(clippy::float_cmp)]
                    if b <= 0.0 || b == 1.0 || v <= 0.0 {
                        f64::NAN
                    } else {
                        v.log(b)
                    }
                };
                *base = f64x4::new([
                    log_fn(base_arr[0], x_arr[0]),
                    log_fn(base_arr[1], x_arr[1]),
                    log_fn(base_arr[2], x_arr[2]),
                    log_fn(base_arr[3], x_arr[3]),
                ]);
            }
            Instruction::Atan2 => {
                let x = pop!();
                let y = top!();
                let x_arr = x.to_array();
                let y_arr = y.to_array();
                *y = f64x4::new([
                    y_arr[0].atan2(x_arr[0]),
                    y_arr[1].atan2(x_arr[1]),
                    y_arr[2].atan2(x_arr[2]),
                    y_arr[3].atan2(x_arr[3]),
                ]);
            }
            Instruction::BesselJ => {
                let x = pop!();
                let n = top!();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                #[allow(clippy::cast_possible_truncation)]
                {
                    *n = f64x4::new([
                        crate::math::bessel_j(n_arr[0].round() as i32, x_arr[0])
                            .unwrap_or(f64::NAN),
                        crate::math::bessel_j(n_arr[1].round() as i32, x_arr[1])
                            .unwrap_or(f64::NAN),
                        crate::math::bessel_j(n_arr[2].round() as i32, x_arr[2])
                            .unwrap_or(f64::NAN),
                        crate::math::bessel_j(n_arr[3].round() as i32, x_arr[3])
                            .unwrap_or(f64::NAN),
                    ]);
                }
            }
            Instruction::BesselY => {
                let x = pop!();
                let n = top!();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                #[allow(clippy::cast_possible_truncation)]
                {
                    *n = f64x4::new([
                        crate::math::bessel_y(n_arr[0].round() as i32, x_arr[0])
                            .unwrap_or(f64::NAN),
                        crate::math::bessel_y(n_arr[1].round() as i32, x_arr[1])
                            .unwrap_or(f64::NAN),
                        crate::math::bessel_y(n_arr[2].round() as i32, x_arr[2])
                            .unwrap_or(f64::NAN),
                        crate::math::bessel_y(n_arr[3].round() as i32, x_arr[3])
                            .unwrap_or(f64::NAN),
                    ]);
                }
            }
            Instruction::BesselI => {
                let x = pop!();
                let n = top!();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                #[allow(clippy::cast_possible_truncation)]
                {
                    *n = f64x4::new([
                        crate::math::bessel_i(n_arr[0].round() as i32, x_arr[0]),
                        crate::math::bessel_i(n_arr[1].round() as i32, x_arr[1]),
                        crate::math::bessel_i(n_arr[2].round() as i32, x_arr[2]),
                        crate::math::bessel_i(n_arr[3].round() as i32, x_arr[3]),
                    ]);
                }
            }
            Instruction::BesselK => {
                let x = pop!();
                let n = top!();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                #[allow(clippy::cast_possible_truncation)]
                {
                    *n = f64x4::new([
                        crate::math::bessel_k(n_arr[0].round() as i32, x_arr[0])
                            .unwrap_or(f64::NAN),
                        crate::math::bessel_k(n_arr[1].round() as i32, x_arr[1])
                            .unwrap_or(f64::NAN),
                        crate::math::bessel_k(n_arr[2].round() as i32, x_arr[2])
                            .unwrap_or(f64::NAN),
                        crate::math::bessel_k(n_arr[3].round() as i32, x_arr[3])
                            .unwrap_or(f64::NAN),
                    ]);
                }
            }
            Instruction::Polygamma => {
                let x = pop!();
                let n = top!();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                #[allow(clippy::cast_possible_truncation)]
                {
                    *n = f64x4::new([
                        crate::math::eval_polygamma(n_arr[0].round() as i32, x_arr[0])
                            .unwrap_or(f64::NAN),
                        crate::math::eval_polygamma(n_arr[1].round() as i32, x_arr[1])
                            .unwrap_or(f64::NAN),
                        crate::math::eval_polygamma(n_arr[2].round() as i32, x_arr[2])
                            .unwrap_or(f64::NAN),
                        crate::math::eval_polygamma(n_arr[3].round() as i32, x_arr[3])
                            .unwrap_or(f64::NAN),
                    ]);
                }
            }
            Instruction::Beta => {
                let b = pop!();
                let a = top!();
                let a_arr = a.to_array();
                let b_arr = b.to_array();
                let beta = |a: f64, b: f64| -> f64 {
                    match (
                        crate::math::eval_gamma(a),
                        crate::math::eval_gamma(b),
                        crate::math::eval_gamma(a + b),
                    ) {
                        (Some(ga), Some(gb), Some(gab)) => ga * gb / gab,
                        _ => f64::NAN,
                    }
                };
                *a = f64x4::new([
                    beta(a_arr[0], b_arr[0]),
                    beta(a_arr[1], b_arr[1]),
                    beta(a_arr[2], b_arr[2]),
                    beta(a_arr[3], b_arr[3]),
                ]);
            }
            Instruction::ZetaDeriv => {
                let s = pop!();
                let n = top!();
                let s_arr = s.to_array();
                let n_arr = n.to_array();
                #[allow(clippy::cast_possible_truncation)]
                {
                    *n = f64x4::new([
                        crate::math::eval_zeta_deriv(n_arr[0].round() as i32, s_arr[0])
                            .unwrap_or(f64::NAN),
                        crate::math::eval_zeta_deriv(n_arr[1].round() as i32, s_arr[1])
                            .unwrap_or(f64::NAN),
                        crate::math::eval_zeta_deriv(n_arr[2].round() as i32, s_arr[2])
                            .unwrap_or(f64::NAN),
                        crate::math::eval_zeta_deriv(n_arr[3].round() as i32, s_arr[3])
                            .unwrap_or(f64::NAN),
                    ]);
                }
            }
            Instruction::Hermite => {
                let x = pop!();
                let n = top!();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                #[allow(clippy::cast_possible_truncation)]
                {
                    *n = f64x4::new([
                        crate::math::eval_hermite(n_arr[0].round() as i32, x_arr[0])
                            .unwrap_or(f64::NAN),
                        crate::math::eval_hermite(n_arr[1].round() as i32, x_arr[1])
                            .unwrap_or(f64::NAN),
                        crate::math::eval_hermite(n_arr[2].round() as i32, x_arr[2])
                            .unwrap_or(f64::NAN),
                        crate::math::eval_hermite(n_arr[3].round() as i32, x_arr[3])
                            .unwrap_or(f64::NAN),
                    ]);
                }
            }

            // Three-argument functions
            Instruction::AssocLegendre => {
                let x = pop!();
                let m = pop!();
                let l = top!();
                let x_arr = x.to_array();
                let m_arr = m.to_array();
                let l_arr = l.to_array();
                #[allow(clippy::cast_possible_truncation)]
                {
                    *l = f64x4::new([
                        crate::math::eval_assoc_legendre(
                            l_arr[0].round() as i32,
                            m_arr[0].round() as i32,
                            x_arr[0],
                        )
                        .unwrap_or(f64::NAN),
                        crate::math::eval_assoc_legendre(
                            l_arr[1].round() as i32,
                            m_arr[1].round() as i32,
                            x_arr[1],
                        )
                        .unwrap_or(f64::NAN),
                        crate::math::eval_assoc_legendre(
                            l_arr[2].round() as i32,
                            m_arr[2].round() as i32,
                            x_arr[2],
                        )
                        .unwrap_or(f64::NAN),
                        crate::math::eval_assoc_legendre(
                            l_arr[3].round() as i32,
                            m_arr[3].round() as i32,
                            x_arr[3],
                        )
                        .unwrap_or(f64::NAN),
                    ]);
                }
            }

            // Four-argument functions
            Instruction::SphericalHarmonic => {
                let phi = pop!();
                let theta = pop!();
                let m = pop!();
                let l = top!();
                let phi_arr = phi.to_array();
                let theta_arr = theta.to_array();
                let m_arr = m.to_array();
                let l_arr = l.to_array();
                #[allow(clippy::cast_possible_truncation)]
                {
                    *l = f64x4::new([
                        crate::math::eval_spherical_harmonic(
                            l_arr[0].round() as i32,
                            m_arr[0].round() as i32,
                            theta_arr[0],
                            phi_arr[0],
                        )
                        .unwrap_or(f64::NAN),
                        crate::math::eval_spherical_harmonic(
                            l_arr[1].round() as i32,
                            m_arr[1].round() as i32,
                            theta_arr[1],
                            phi_arr[1],
                        )
                        .unwrap_or(f64::NAN),
                        crate::math::eval_spherical_harmonic(
                            l_arr[2].round() as i32,
                            m_arr[2].round() as i32,
                            theta_arr[2],
                            phi_arr[2],
                        )
                        .unwrap_or(f64::NAN),
                        crate::math::eval_spherical_harmonic(
                            l_arr[3].round() as i32,
                            m_arr[3].round() as i32,
                            theta_arr[3],
                            phi_arr[3],
                        )
                        .unwrap_or(f64::NAN),
                    ]);
                }
            }

            // Unhandled instructions - poison with NaN
            _ => {
                debug_assert!(false, "Unhandled SIMD instruction {instr:?}");
                let poison = f64x4::splat(f64::NAN);
                if let Some(top) = stack.last_mut() {
                    *top = poison;
                } else {
                    stack.push(poison);
                }
            }
        }
    }

    /// Execute a scalar instruction for batch remainder.
    ///
    /// This is used for the 1-3 points that don't fit in a SIMD chunk.
    /// It avoids `Vec` method overhead by using a raw slice and tracked length.
    ///
    /// # Safety Invariant
    ///
    /// Stack operations validated at compile time, same as `exec_instruction`.
    //
    // Allow undocumented_unsafe_blocks: Same invariant as other exec functions.
    #[allow(clippy::undocumented_unsafe_blocks)]
    #[allow(clippy::too_many_lines)]
    #[inline]
    fn exec_scalar_batch_instruction(
        instr: Instruction,
        stack: &mut [f64],
        len: &mut usize,
        cache: &mut [f64],
        columns: &[&[f64]],
        constants: &[f64],
        point_idx: usize,
    ) {
        #[inline]
        fn push(stack: &mut [f64], len: &mut usize, val: f64) {
            unsafe {
                *stack.get_unchecked_mut(*len) = val;
                *len += 1;
            }
        }

        #[inline]
        fn pop(stack: &[f64], len: &mut usize) -> f64 {
            unsafe {
                *len -= 1;
                *stack.get_unchecked(*len)
            }
        }

        #[inline]
        fn top_mut(stack: &mut [f64], len: usize) -> &mut f64 {
            unsafe { stack.get_unchecked_mut(len - 1) }
        }

        match instr {
            // Hot instructions first (identical order to exec_instruction)
            Instruction::Add => {
                let b = pop(stack, len);
                *top_mut(stack, *len) += b;
            }
            Instruction::Mul => {
                let b = pop(stack, len);
                *top_mut(stack, *len) *= b;
            }
            Instruction::Div => {
                let b = pop(stack, len);
                *top_mut(stack, *len) /= b;
            }
            Instruction::Pow => {
                let exp = pop(stack, len);
                let base = top_mut(stack, *len);
                *base = base.powf(exp);
            }

            // Memory operations
            Instruction::LoadConst(idx) => push(stack, len, constants[idx as usize]),
            Instruction::LoadParam(p) => push(stack, len, columns[p as usize][point_idx]),

            // CSE operations
            Instruction::Dup => {
                let val = *top_mut(stack, *len);
                push(stack, len, val);
            }
            Instruction::StoreCached(slot) => {
                cache[slot as usize] = *top_mut(stack, *len);
            }
            Instruction::LoadCached(slot) => {
                push(stack, len, cache[slot as usize]);
            }

            // Truncated list from before, now completed to avoid Vec fallback
            Instruction::Neg => {
                let top = top_mut(stack, *len);
                *top = -*top;
            }
            Instruction::Abs => {
                let top = top_mut(stack, *len);
                *top = top.abs();
            }
            Instruction::Sqrt => {
                let top = top_mut(stack, *len);
                *top = top.sqrt();
            }
            Instruction::Exp => {
                let top = top_mut(stack, *len);
                *top = top.exp();
            }
            Instruction::Ln => {
                let top = top_mut(stack, *len);
                *top = top.ln();
            }
            Instruction::Sin => {
                let top = top_mut(stack, *len);
                *top = top.sin();
            }
            Instruction::Cos => {
                let top = top_mut(stack, *len);
                *top = top.cos();
            }
            Instruction::Tan => {
                let top = top_mut(stack, *len);
                *top = top.tan();
            }
            Instruction::SinCos => {
                let x = *top_mut(stack, *len);
                let (s, c) = x.sin_cos();
                *top_mut(stack, *len) = c;
                push(stack, len, s);
            }

            // Fused & Optimized
            Instruction::Square => {
                let top = top_mut(stack, *len);
                *top *= *top;
            }
            Instruction::Cube => {
                let top = top_mut(stack, *len);
                let x = *top;
                *top = x * x * x;
            }
            Instruction::Pow4 => {
                let top = top_mut(stack, *len);
                let x = *top;
                let x2 = x * x;
                *top = x2 * x2;
            }
            Instruction::Recip => {
                let top = top_mut(stack, *len);
                *top = 1.0 / *top;
            }
            Instruction::Powi(n) => {
                let top = top_mut(stack, *len);
                *top = top.powi(n);
            }
            // TODO: v0.8.0 Hierarchical Power Optimizations (Pow3_2, InvPow3_2, Root2k, Cbrt)
            Instruction::MulAdd => {
                let c = pop(stack, len);
                let b = pop(stack, len);
                let a = top_mut(stack, *len);
                *a = a.mul_add(b, c);
            }

            // Trigonometric & Hyperbolic (Common ones)
            Instruction::Asin => {
                let t = top_mut(stack, *len);
                *t = t.asin();
            }
            Instruction::Acos => {
                let t = top_mut(stack, *len);
                *t = t.acos();
            }
            Instruction::Atan => {
                let t = top_mut(stack, *len);
                *t = t.atan();
            }
            Instruction::Sinh => {
                let t = top_mut(stack, *len);
                *t = t.sinh();
            }
            Instruction::Cosh => {
                let t = top_mut(stack, *len);
                *t = t.cosh();
            }
            Instruction::Tanh => {
                let t = top_mut(stack, *len);
                *t = t.tanh();
            }

            // Fallback for rare instructions (completing the set)
            _ => {
                match instr {
                    Instruction::Asinh => {
                        let t = top_mut(stack, *len);
                        *t = t.asinh();
                    }
                    Instruction::Acosh => {
                        let t = top_mut(stack, *len);
                        *t = t.acosh();
                    }
                    Instruction::Atanh => {
                        let t = top_mut(stack, *len);
                        *t = t.atanh();
                    }
                    Instruction::Log10 => {
                        let t = top_mut(stack, *len);
                        *t = t.log10();
                    }
                    Instruction::Log2 => {
                        let t = top_mut(stack, *len);
                        *t = t.log2();
                    }
                    Instruction::Erf => {
                        let t = top_mut(stack, *len);
                        *t = crate::math::eval_erf(*t);
                    }
                    Instruction::Gamma => {
                        let t = top_mut(stack, *len);
                        *t = crate::math::eval_gamma(*t).unwrap_or(f64::NAN);
                    }
                    Instruction::Log => {
                        let x = pop(stack, len);
                        let base = top_mut(stack, *len);
                        #[allow(clippy::float_cmp)]
                        let invalid = *base <= 0.0 || *base == 1.0 || x <= 0.0;
                        *base = if invalid { f64::NAN } else { x.log(*base) };
                    }
                    Instruction::Atan2 => {
                        let x = pop(stack, len);
                        let y = top_mut(stack, *len);
                        *y = y.atan2(x);
                    }
                    _ => {
                        // Poison with NaN and delegate to slower path if absolutely necessary
                        // but actually we've covered almost all instructions here.
                        *top_mut(stack, *len) = f64::NAN;
                    }
                }
            }
        }
    }
}
