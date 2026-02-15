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
    #[allow(
        clippy::too_many_lines,
        reason = "Batch evaluation function handles multiple cases and optimizations"
    )]
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
        #[allow(
            clippy::integer_division,
            reason = "Intentional integer division for chunking"
        )]
        let full_chunks = n_points / 4;

        // SIMD stack: use inline MaybeUninit buffer for zero-overhead stack access,
        // matching the pattern used in evaluate_inline for scalar evaluation.
        // The Vec parameter is kept for API compatibility but we use raw pointers internally.
        #[allow(
            clippy::items_after_statements,
            reason = "Placed after validation code for locality with its usage"
        )]
        const INLINE_SIMD_STACK: usize = 64;

        // Use provided buffer or create local one — reserve enough capacity
        let mut local_stack;
        #[allow(
            clippy::option_if_let_else,
            clippy::single_match_else,
            reason = "Match is clearer for mutable reference handling"
        )]
        let simd_stack_buf: &mut Vec<f64x4> = match simd_buffer {
            Some(buf) => {
                buf.reserve(self.stack_size.saturating_sub(buf.capacity()));
                buf
            }
            None => {
                local_stack = Vec::with_capacity(self.stack_size);
                &mut local_stack
            }
        };

        // For small stacks, use an inline MaybeUninit buffer on the CPU stack.
        // For large stacks, use the Vec's heap buffer via raw pointers.
        let mut inline_simd_stack: [MaybeUninit<f64x4>; INLINE_SIMD_STACK] =
            [MaybeUninit::uninit(); INLINE_SIMD_STACK];

        let use_inline = self.stack_size <= INLINE_SIMD_STACK;
        // SAFETY: For inline path, buffer is stack-allocated and large enough.
        // For heap path, Vec has enough capacity reserved above.
        // Bytecode guarantees all reads are preceded by writes (compiler-validated).
        let stack_ptr: *mut f64x4 = if use_inline {
            inline_simd_stack.as_mut_ptr().cast::<f64x4>()
        } else {
            simd_stack_buf.as_mut_ptr()
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

        // Cached per evaluator; avoids rebuilding splatted constants on each batch call.
        let simd_constants = self.simd_constants();

        // Process full SIMD chunks using raw pointer + len for zero-overhead dispatch
        for chunk in 0..full_chunks {
            let base = chunk * 4;
            let mut len = 0_usize;

            for instr in instructions {
                // SAFETY: Compiler ensures max_stack <= buffer capacity.
                // Bytecode guarantees all reads are preceded by writes.
                unsafe {
                    Self::exec_simd_instruction(
                        *instr,
                        stack_ptr,
                        &mut len,
                        simd_cache_slice,
                        columns,
                        simd_constants,
                        constants,
                        base,
                    );
                }
            }

            // Extract results from SIMD vector
            debug_assert!(len > 0, "SIMD stack empty after evaluation");
            // SAFETY: len > 0, bytecode always leaves one result on stack
            let result = unsafe { *stack_ptr.add(len - 1) };
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
    #[allow(
        clippy::items_after_statements,
        reason = "MIN_PARALLEL_SIZE placed near its usage for clarity"
    )]
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

        // Parallel chunked evaluation with thread-local SIMD buffer reuse.
        // Write each chunk directly into the output slice to avoid extra allocations/copies.
        let mut output = vec![0.0; n_points];
        output
            .par_chunks_mut(MIN_PARALLEL_SIZE)
            .enumerate()
            .try_for_each_init(
                || Vec::with_capacity(self.stack_size),
                |simd_buffer, (chunk_idx, out_chunk)| {
                    let start = chunk_idx * MIN_PARALLEL_SIZE;
                    let end = start + out_chunk.len();
                    let col_slices: Vec<&[f64]> =
                        columns.iter().map(|col| &col[start..end]).collect();
                    self.eval_batch(&col_slices, out_chunk, Some(simd_buffer))
                },
            )?;
        Ok(output)
    }

    /// Execute a SIMD instruction on the stack using raw pointer + length.
    ///
    /// Most common instructions are inlined here for performance.
    /// Less common instructions fall through to the slow path.
    ///
    /// # Safety
    ///
    /// - `stack_ptr` must point to a buffer with capacity >= max stack depth
    /// - `*len` must accurately track the number of initialized elements
    /// - Stack depth validated at compile time by the `Compiler`
    /// - Uses same bytecode as scalar path with verified stack effects
    /// - Debug builds include runtime assertions
    //
    // Allow undocumented_unsafe_blocks: All ~30 unsafe blocks share the same invariant.
    // Allow too_many_lines: This is a dispatch function, splitting would hurt cache locality.
    #[allow(
        clippy::undocumented_unsafe_blocks,
        reason = "All ~30 unsafe blocks share the same invariant"
    )]
    #[allow(
        clippy::too_many_lines,
        reason = "This is a dispatch function, splitting would hurt cache locality"
    )]
    #[allow(
        clippy::too_many_arguments,
        reason = "Raw pointer + len replaced single &mut Vec param; net +1 arg"
    )]
    #[inline]
    unsafe fn exec_simd_instruction(
        instr: Instruction,
        stack_ptr: *mut f64x4,
        len: &mut usize,
        cache: &mut [f64x4],
        columns: &[&[f64]],
        simd_constants: &[f64x4],
        constants: &[f64],
        base: usize,
    ) {
        // Raw pointer helpers — zero overhead, no Vec metadata access.
        // All macro invocations occur inside the `unsafe { match ... }` block below.
        macro_rules! push {
            ($val:expr) => {
                stack_ptr.add(*len).write($val);
                *len += 1;
            };
        }

        macro_rules! pop {
            () => {{
                *len -= 1;
                stack_ptr.add(*len).read()
            }};
        }

        macro_rules! top_mut {
            () => {
                &mut *stack_ptr.add(*len - 1)
            };
        }

        // SAFETY: All index accesses (columns, constants, simd_constants, cache)
        // are validated by compile-time bytecode verification. Column indices,
        // constant indices, and stack depth are guaranteed correct.
        unsafe {
            match instr {
                // Memory operations
                Instruction::LoadConst(idx) => {
                    push!(*simd_constants.get_unchecked(idx as usize));
                }
                Instruction::LoadParam(p) => {
                    let col = *columns.get_unchecked(p as usize);
                    push!(f64x4::new([
                        *col.get_unchecked(base),
                        *col.get_unchecked(base + 1),
                        *col.get_unchecked(base + 2),
                        *col.get_unchecked(base + 3),
                    ]));
                }

                Instruction::Pop => {
                    *len -= 1;
                }
                Instruction::Swap => {
                    std::ptr::swap(stack_ptr.add(*len - 1), stack_ptr.add(*len - 2));
                }

                // Binary operations
                Instruction::Add => {
                    let b = pop!();
                    *top_mut!() += b;
                }
                Instruction::Mul => {
                    let b = pop!();
                    *top_mut!() *= b;
                }
                Instruction::MulConst(idx) => {
                    *top_mut!() *= *simd_constants.get_unchecked(idx as usize);
                }
                Instruction::AddConst(idx) => {
                    *top_mut!() += *simd_constants.get_unchecked(idx as usize);
                }
                Instruction::SubConst(idx) => {
                    *top_mut!() -= *simd_constants.get_unchecked(idx as usize);
                }
                Instruction::ConstSub(idx) => {
                    let top = top_mut!();
                    *top = *simd_constants.get_unchecked(idx as usize) - *top;
                }
                Instruction::Div => {
                    let b = pop!();
                    *top_mut!() /= b;
                }
                Instruction::Sub => {
                    let b = pop!();
                    *top_mut!() -= b;
                }
                Instruction::Pow => {
                    let exp = pop!();
                    let top = top_mut!();
                    *top = top.pow_f64x4(exp);
                }

                // Unary operations
                Instruction::Neg => {
                    let top = top_mut!();
                    *top = -*top;
                }
                Instruction::Sin => {
                    let top = top_mut!();
                    let arr = top.to_array();
                    *top = f64x4::new([arr[0].sin(), arr[1].sin(), arr[2].sin(), arr[3].sin()]);
                }
                Instruction::Cos => {
                    let top = top_mut!();
                    let arr = top.to_array();
                    *top = f64x4::new([arr[0].cos(), arr[1].cos(), arr[2].cos(), arr[3].cos()]);
                }
                Instruction::Tan => {
                    let top = top_mut!();
                    let arr = top.to_array();
                    *top = f64x4::new([arr[0].tan(), arr[1].tan(), arr[2].tan(), arr[3].tan()]);
                }
                Instruction::Sqrt => {
                    let top = top_mut!();
                    *top = top.sqrt();
                }
                Instruction::Exp => {
                    let top = top_mut!();
                    *top = top.exp();
                }
                Instruction::Expm1 => {
                    let top = top_mut!();
                    let arr = top.to_array();
                    *top = f64x4::new([
                        arr[0].exp_m1(),
                        arr[1].exp_m1(),
                        arr[2].exp_m1(),
                        arr[3].exp_m1(),
                    ]);
                }
                Instruction::ExpNeg => {
                    let top = top_mut!();
                    *top = (-*top).exp();
                }
                Instruction::Pow3_2 => {
                    let top = top_mut!();
                    let x = *top;
                    *top = x * x.sqrt();
                }
                Instruction::Ln => {
                    let top = top_mut!();
                    *top = top.ln();
                }
                Instruction::Log1p => {
                    let top = top_mut!();
                    let arr = top.to_array();
                    *top = f64x4::new([
                        arr[0].ln_1p(),
                        arr[1].ln_1p(),
                        arr[2].ln_1p(),
                        arr[3].ln_1p(),
                    ]);
                }
                Instruction::Abs => {
                    let top = top_mut!();
                    *top = top.abs();
                }

                // Fused operations
                Instruction::Square => {
                    let top = top_mut!();
                    *top = *top * *top;
                }
                Instruction::Cube => {
                    let top = top_mut!();
                    let x = *top;
                    *top = x * x * x;
                }
                Instruction::Pow4 => {
                    let top = top_mut!();
                    let x = *top;
                    let x2 = x * x;
                    *top = x2 * x2;
                }
                Instruction::Recip => {
                    let top = top_mut!();
                    *top = f64x4::splat(1.0) / *top;
                }
                Instruction::Powi(n) => {
                    let top = top_mut!();
                    let x = *top;
                    let arr = x.to_array();
                    *top = f64x4::new([
                        arr[0].powi(n),
                        arr[1].powi(n),
                        arr[2].powi(n),
                        arr[3].powi(n),
                    ]);
                }
                Instruction::MulAdd => {
                    let c = pop!();
                    let b = pop!();
                    let a = top_mut!();
                    *a = a.mul_add(b, c);
                }
                Instruction::MulSub => {
                    let c = pop!();
                    let b = pop!();
                    let a = top_mut!();
                    *a = a.mul_add(b, -c);
                }
                Instruction::NegMulAdd => {
                    let c = pop!();
                    let b = pop!();
                    let a = top_mut!();
                    *a = (-*a).mul_add(b, c);
                }
                Instruction::InvSqrt => {
                    let top = top_mut!();
                    *top = f64x4::splat(1.0) / top.sqrt();
                }
                Instruction::InvSquare => {
                    let top = top_mut!();
                    let x = *top;
                    *top = f64x4::splat(1.0) / (x * x);
                }
                Instruction::InvCube => {
                    let top = top_mut!();
                    let x = *top;
                    *top = f64x4::splat(1.0) / (x * x * x);
                }
                Instruction::InvPow3_2 => {
                    let top = top_mut!();
                    let x = *top;
                    *top = f64x4::splat(1.0) / (x * x.sqrt());
                }
                Instruction::ExpSqr => {
                    let top = top_mut!();
                    let x = *top;
                    *top = (x * x).exp();
                }
                Instruction::ExpSqrNeg => {
                    let top = top_mut!();
                    let x = *top;
                    *top = (-(x * x)).exp();
                }
                Instruction::RecipExpm1 => {
                    let top = top_mut!();
                    let arr = top.to_array();
                    *top = f64x4::new([
                        1.0 / arr[0].exp_m1(),
                        1.0 / arr[1].exp_m1(),
                        1.0 / arr[2].exp_m1(),
                        1.0 / arr[3].exp_m1(),
                    ]);
                }
                Instruction::Sinc => {
                    let top = top_mut!();
                    let arr = top.to_array();
                    *top = f64x4::new([
                        stack::eval_sinc(arr[0]),
                        stack::eval_sinc(arr[1]),
                        stack::eval_sinc(arr[2]),
                        stack::eval_sinc(arr[3]),
                    ]);
                }

                Instruction::Log => {
                    let x = pop!();
                    let base_slot = top_mut!();
                    let x_arr = x.to_array();
                    let base_arr = base_slot.to_array();
                    let log_fn = |b: f64, v: f64| -> f64 {
                        #[allow(
                            clippy::float_cmp,
                            reason = "Comparing against exact constant 1.0 for logarithm base validation"
                        )]
                        if b <= 0.0 || b == 1.0 || v <= 0.0 {
                            f64::NAN
                        } else {
                            v.log(b)
                        }
                    };
                    *base_slot = f64x4::new([
                        log_fn(base_arr[0], x_arr[0]),
                        log_fn(base_arr[1], x_arr[1]),
                        log_fn(base_arr[2], x_arr[2]),
                        log_fn(base_arr[3], x_arr[3]),
                    ]);
                }

                // Hyperbolic - vectorized via SIMD exp identities
                Instruction::Sinh => {
                    let top = top_mut!();
                    let arr = top.to_array();
                    *top = f64x4::new([arr[0].sinh(), arr[1].sinh(), arr[2].sinh(), arr[3].sinh()]);
                }
                Instruction::Cosh => {
                    let top = top_mut!();
                    let arr = top.to_array();
                    *top = f64x4::new([arr[0].cosh(), arr[1].cosh(), arr[2].cosh(), arr[3].cosh()]);
                }
                Instruction::Tanh => {
                    let top = top_mut!();
                    let arr = top.to_array();
                    *top = f64x4::new([arr[0].tanh(), arr[1].tanh(), arr[2].tanh(), arr[3].tanh()]);
                }

                // CSE instructions
                Instruction::PolyEval(idx) => {
                    let start = idx as usize;
                    // Constants are pre-validated by compiler
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "Degree is stored as f64 but always an integer, so cast is safe"
                    )]
                    let degree = *constants.get_unchecked(start) as usize;

                    // Initialize with c_N
                    let mut acc = *simd_constants.get_unchecked(start + 1);
                    let x_ptr = top_mut!();
                    let x = *x_ptr;

                    for i in 0..degree {
                        let coeff = *simd_constants.get_unchecked(start + 2 + i);
                        acc = acc.mul_add(x, coeff);
                    }
                    *x_ptr = acc;
                }
                Instruction::Dup => {
                    let top = *top_mut!();
                    push!(top);
                }
                Instruction::StoreCached(slot) => {
                    *cache.get_unchecked_mut(slot as usize) = *top_mut!();
                }
                Instruction::LoadCached(slot) => {
                    push!(*cache.get_unchecked(slot as usize));
                }

                // Slow path for less common instructions
                _ => Self::exec_simd_slow_instruction(instr, stack_ptr, len),
            }
        }
    }

    /// SIMD slow path for handling less common instructions.
    ///
    /// Falls back to scalar computation for each of the 4 lanes.
    ///
    /// # Safety
    ///
    /// - `stack_ptr` must point to a buffer with capacity >= max stack depth
    /// - `*len` must accurately track the number of initialized elements
    /// - Stack is guaranteed non-empty by compile-time validation. The `debug_assert!`
    ///   at function entry catches any compiler bugs in debug builds.
    //
    // Allow too_many_lines: This is a dispatch function covering all special functions.
    // Splitting would hurt code locality without meaningful abstraction benefit.
    // Allow undocumented_unsafe_blocks: All unsafe blocks share the same invariant
    // documented above - stack non-empty validated by compiler at compile time.
    #[allow(
        clippy::too_many_lines,
        reason = "This is a dispatch function covering all special functions. Splitting would hurt code locality without meaningful abstraction benefit"
    )]
    #[allow(
        clippy::undocumented_unsafe_blocks,
        reason = "All unsafe blocks share the same invariant documented above - stack non-empty validated by compiler at compile time"
    )]
    #[inline(never)]
    #[cold]
    unsafe fn exec_simd_slow_instruction(
        instr: Instruction,
        stack_ptr: *mut f64x4,
        len: &mut usize,
    ) {
        debug_assert!(*len > 0, "Stack empty in SIMD slow path");

        // Raw pointer helpers — mirror those in exec_simd_instruction
        macro_rules! top {
            () => {
                unsafe { &mut *stack_ptr.add(*len - 1) }
            };
        }

        macro_rules! pop {
            () => {{
                *len -= 1;
                unsafe { stack_ptr.add(*len).read() }
            }};
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

            // Log functions
            Instruction::Cbrt => {
                let top = top!();
                let arr = top.to_array();
                *top = f64x4::new([arr[0].cbrt(), arr[1].cbrt(), arr[2].cbrt(), arr[3].cbrt()]);
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
                    crate::math::eval_tetragamma(arr[0]).unwrap_or(f64::NAN),
                    crate::math::eval_tetragamma(arr[1]).unwrap_or(f64::NAN),
                    crate::math::eval_tetragamma(arr[2]).unwrap_or(f64::NAN),
                    crate::math::eval_tetragamma(arr[3]).unwrap_or(f64::NAN),
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
                    #[allow(
                        clippy::float_cmp,
                        reason = "Comparing against exact constant 1.0 for logarithm base validation"
                    )]
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
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Bessel function order is always a small integer"
                )]
                {
                    let f = |ni: f64, xi: f64| -> f64 {
                        if ni.is_nan() {
                            return f64::NAN;
                        }
                        crate::math::bessel_j(ni.round() as i32, xi).unwrap_or(f64::NAN)
                    };
                    *n = f64x4::new([
                        f(n_arr[0], x_arr[0]),
                        f(n_arr[1], x_arr[1]),
                        f(n_arr[2], x_arr[2]),
                        f(n_arr[3], x_arr[3]),
                    ]);
                }
            }
            Instruction::BesselY => {
                let x = pop!();
                let n = top!();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Bessel function order is always a small integer"
                )]
                {
                    let f = |ni: f64, xi: f64| -> f64 {
                        if ni.is_nan() {
                            return f64::NAN;
                        }
                        crate::math::bessel_y(ni.round() as i32, xi).unwrap_or(f64::NAN)
                    };
                    *n = f64x4::new([
                        f(n_arr[0], x_arr[0]),
                        f(n_arr[1], x_arr[1]),
                        f(n_arr[2], x_arr[2]),
                        f(n_arr[3], x_arr[3]),
                    ]);
                }
            }
            Instruction::BesselI => {
                let x = pop!();
                let n = top!();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Bessel function order is always a small integer"
                )]
                {
                    let f = |ni: f64, xi: f64| -> f64 {
                        if ni.is_nan() {
                            return f64::NAN;
                        }
                        crate::math::bessel_i(ni.round() as i32, xi)
                    };
                    *n = f64x4::new([
                        f(n_arr[0], x_arr[0]),
                        f(n_arr[1], x_arr[1]),
                        f(n_arr[2], x_arr[2]),
                        f(n_arr[3], x_arr[3]),
                    ]);
                }
            }
            Instruction::BesselK => {
                let x = pop!();
                let n = top!();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Bessel function order is always a small integer"
                )]
                {
                    let f = |ni: f64, xi: f64| -> f64 {
                        if ni.is_nan() {
                            return f64::NAN;
                        }
                        crate::math::bessel_k(ni.round() as i32, xi).unwrap_or(f64::NAN)
                    };
                    *n = f64x4::new([
                        f(n_arr[0], x_arr[0]),
                        f(n_arr[1], x_arr[1]),
                        f(n_arr[2], x_arr[2]),
                        f(n_arr[3], x_arr[3]),
                    ]);
                }
            }
            Instruction::Polygamma => {
                let x = pop!();
                let n = top!();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Polygamma order is always a small integer"
                )]
                {
                    let f = |ni: f64, xi: f64| -> f64 {
                        if ni.is_nan() {
                            return f64::NAN;
                        }
                        crate::math::eval_polygamma(ni.round() as i32, xi).unwrap_or(f64::NAN)
                    };
                    *n = f64x4::new([
                        f(n_arr[0], x_arr[0]),
                        f(n_arr[1], x_arr[1]),
                        f(n_arr[2], x_arr[2]),
                        f(n_arr[3], x_arr[3]),
                    ]);
                }
            }
            Instruction::Beta => {
                let b = pop!();
                let a = top!();
                let a_arr = a.to_array();
                let b_arr = b.to_array();
                let beta = |val_a: f64, val_b: f64| -> f64 {
                    match (
                        crate::math::eval_gamma(val_a),
                        crate::math::eval_gamma(val_b),
                        crate::math::eval_gamma(val_a + val_b),
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
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Zeta derivative order is always a small integer"
                )]
                {
                    let f = |ni: f64, si: f64| -> f64 {
                        if ni.is_nan() {
                            return f64::NAN;
                        }
                        crate::math::eval_zeta_deriv(ni.round() as i32, si).unwrap_or(f64::NAN)
                    };
                    *n = f64x4::new([
                        f(n_arr[0], s_arr[0]),
                        f(n_arr[1], s_arr[1]),
                        f(n_arr[2], s_arr[2]),
                        f(n_arr[3], s_arr[3]),
                    ]);
                }
            }
            Instruction::Hermite => {
                let x = pop!();
                let n = top!();
                let x_arr = x.to_array();
                let n_arr = n.to_array();
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Hermite polynomial degree is always a small integer"
                )]
                {
                    let f = |ni: f64, xi: f64| -> f64 {
                        if ni.is_nan() {
                            return f64::NAN;
                        }
                        crate::math::eval_hermite(ni.round() as i32, xi).unwrap_or(f64::NAN)
                    };
                    *n = f64x4::new([
                        f(n_arr[0], x_arr[0]),
                        f(n_arr[1], x_arr[1]),
                        f(n_arr[2], x_arr[2]),
                        f(n_arr[3], x_arr[3]),
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
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Associated Legendre degree/order is always a small integer"
                )]
                {
                    let f = |li: f64, mi: f64, xi: f64| -> f64 {
                        if li.is_nan() || mi.is_nan() {
                            return f64::NAN;
                        }
                        crate::math::eval_assoc_legendre(li.round() as i32, mi.round() as i32, xi)
                            .unwrap_or(f64::NAN)
                    };
                    *l = f64x4::new([
                        f(l_arr[0], m_arr[0], x_arr[0]),
                        f(l_arr[1], m_arr[1], x_arr[1]),
                        f(l_arr[2], m_arr[2], x_arr[2]),
                        f(l_arr[3], m_arr[3], x_arr[3]),
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
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Spherical harmonic degree/order is always a small integer"
                )]
                {
                    let f = |li: f64, mi: f64, ti: f64, pi: f64| -> f64 {
                        if li.is_nan() || mi.is_nan() {
                            return f64::NAN;
                        }
                        crate::math::eval_spherical_harmonic(
                            li.round() as i32,
                            mi.round() as i32,
                            ti,
                            pi,
                        )
                        .unwrap_or(f64::NAN)
                    };
                    *l = f64x4::new([
                        f(l_arr[0], m_arr[0], theta_arr[0], phi_arr[0]),
                        f(l_arr[1], m_arr[1], theta_arr[1], phi_arr[1]),
                        f(l_arr[2], m_arr[2], theta_arr[2], phi_arr[2]),
                        f(l_arr[3], m_arr[3], theta_arr[3], phi_arr[3]),
                    ]);
                }
            }

            // Unhandled in slow path - fail fast so SIMD coverage gaps are explicit.
            _ => {
                std::process::abort();
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
    #[allow(
        clippy::undocumented_unsafe_blocks,
        reason = "Same invariant as other exec functions"
    )]
    #[allow(
        clippy::too_many_lines,
        reason = "Complex evaluation logic requires all instructions in one function"
    )]
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
            Instruction::MulConst(idx) => {
                *top_mut(stack, *len) *= constants[idx as usize];
            }
            Instruction::AddConst(idx) => {
                *top_mut(stack, *len) += constants[idx as usize];
            }
            Instruction::SubConst(idx) => {
                *top_mut(stack, *len) -= constants[idx as usize];
            }
            Instruction::ConstSub(idx) => {
                let c = constants[idx as usize];
                *top_mut(stack, *len) = c - *top_mut(stack, *len);
            }
            Instruction::Sub => {
                let b = pop(stack, len);
                *top_mut(stack, *len) -= b;
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

            Instruction::Pop => {
                pop(stack, len);
            }
            Instruction::Swap => unsafe {
                // Inline swap for scalar batch
                let ptr = stack.as_mut_ptr();
                std::ptr::swap(ptr.add(*len - 1), ptr.add(*len - 2));
            },

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
            Instruction::Expm1 => {
                let top = top_mut(stack, *len);
                *top = top.exp_m1();
            }
            Instruction::ExpNeg => {
                let top = top_mut(stack, *len);
                *top = (-*top).exp();
            }
            Instruction::Ln => {
                let top = top_mut(stack, *len);
                *top = top.ln();
            }
            Instruction::Log1p => {
                let top = top_mut(stack, *len);
                *top = top.ln_1p();
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
            Instruction::Sinh => {
                let top = top_mut(stack, *len);
                *top = top.sinh();
            }
            Instruction::Cosh => {
                let top = top_mut(stack, *len);
                *top = top.cosh();
            }
            Instruction::Tanh => {
                let top = top_mut(stack, *len);
                *top = top.tanh();
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
            Instruction::Pow3_2 => {
                let top = top_mut(stack, *len);
                let x = *top;
                *top = x * x.sqrt();
            }
            Instruction::Recip => {
                let top = top_mut(stack, *len);
                *top = 1.0 / *top;
            }
            Instruction::Powi(n) => {
                let top = top_mut(stack, *len);
                let x = *top;
                *top = match n {
                    5 => {
                        let x2 = x * x;
                        x2 * x2 * x
                    }
                    6 => {
                        let x2 = x * x;
                        let x3 = x2 * x;
                        x3 * x3
                    }
                    7 => {
                        let x2 = x * x;
                        let x3 = x2 * x;
                        x3 * x3 * x
                    }
                    -1 => 1.0 / x,
                    -2 => {
                        let x2 = x * x;
                        1.0 / x2
                    }
                    -3 => {
                        let x3 = x * x * x;
                        1.0 / x3
                    }
                    -4 => {
                        let x2 = x * x;
                        1.0 / (x2 * x2)
                    }
                    _ => x.powi(n),
                };
            }
            Instruction::MulAdd => {
                let c = pop(stack, len);
                let b = pop(stack, len);
                let a = top_mut(stack, *len);
                *a = a.mul_add(b, c);
            }
            Instruction::MulSub => {
                let c = pop(stack, len);
                let b = pop(stack, len);
                let a = top_mut(stack, *len);
                *a = a.mul_add(b, -c);
            }
            Instruction::NegMulAdd => {
                let c = pop(stack, len);
                let b = pop(stack, len);
                let a = top_mut(stack, *len);
                *a = (-*a).mul_add(b, c);
            }
            Instruction::InvSqrt => {
                let top = top_mut(stack, *len);
                *top = 1.0 / top.sqrt();
            }
            Instruction::InvSquare => {
                let top = top_mut(stack, *len);
                let x = *top;
                *top = 1.0 / (x * x);
            }
            Instruction::InvCube => {
                let top = top_mut(stack, *len);
                let x = *top;
                *top = 1.0 / (x * x * x);
            }
            Instruction::InvPow3_2 => {
                let top = top_mut(stack, *len);
                let x = *top;
                *top = 1.0 / (x * x.sqrt());
            }
            Instruction::ExpSqr => {
                let top = top_mut(stack, *len);
                let x = *top;
                *top = (x * x).exp();
            }
            Instruction::ExpSqrNeg => {
                let top = top_mut(stack, *len);
                let x = *top;
                *top = (-(x * x)).exp();
            }
            Instruction::RecipExpm1 => {
                let top = top_mut(stack, *len);
                *top = 1.0 / top.exp_m1();
            }
            Instruction::Sinc => {
                let top = top_mut(stack, *len);
                *top = stack::eval_sinc(*top);
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
            // Inverse hyperbolic
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

            // Special functions
            Instruction::Erf => {
                let t = top_mut(stack, *len);
                *t = crate::math::eval_erf(*t);
            }
            Instruction::Erfc => {
                let t = top_mut(stack, *len);
                *t = 1.0 - crate::math::eval_erf(*t);
            }
            Instruction::Gamma => {
                let t = top_mut(stack, *len);
                *t = crate::math::eval_gamma(*t).unwrap_or(f64::NAN);
            }
            Instruction::Digamma => {
                let t = top_mut(stack, *len);
                *t = crate::math::eval_digamma(*t).unwrap_or(f64::NAN);
            }
            Instruction::Trigamma => {
                let t = top_mut(stack, *len);
                *t = crate::math::eval_trigamma(*t).unwrap_or(f64::NAN);
            }
            Instruction::Tetragamma => {
                let t = top_mut(stack, *len);
                *t = crate::math::eval_tetragamma(*t).unwrap_or(f64::NAN);
            }
            Instruction::LambertW => {
                let t = top_mut(stack, *len);
                *t = crate::math::eval_lambert_w(*t).unwrap_or(f64::NAN);
            }
            Instruction::EllipticK => {
                let t = top_mut(stack, *len);
                *t = crate::math::eval_elliptic_k(*t).unwrap_or(f64::NAN);
            }
            Instruction::EllipticE => {
                let t = top_mut(stack, *len);
                *t = crate::math::eval_elliptic_e(*t).unwrap_or(f64::NAN);
            }
            Instruction::Zeta => {
                let t = top_mut(stack, *len);
                *t = crate::math::eval_zeta(*t).unwrap_or(f64::NAN);
            }
            Instruction::ExpPolar => {
                let t = top_mut(stack, *len);
                *t = crate::math::eval_exp_polar(*t);
            }
            Instruction::Cbrt => {
                let t = top_mut(stack, *len);
                *t = t.cbrt();
            }
            Instruction::Signum => {
                let t = top_mut(stack, *len);
                *t = t.signum();
            }
            Instruction::Floor => {
                let t = top_mut(stack, *len);
                *t = t.floor();
            }
            Instruction::Ceil => {
                let t = top_mut(stack, *len);
                *t = t.ceil();
            }
            Instruction::Round => {
                let t = top_mut(stack, *len);
                *t = t.round();
            }

            // Two-argument functions
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
            Instruction::Log => {
                let x = pop(stack, len);
                let base = top_mut(stack, *len);
                let invalid = *base <= 0.0 || *base == 1.0 || x <= 0.0;
                *base = if invalid { f64::NAN } else { x.log(*base) };
            }
            Instruction::Atan2 => {
                let x = pop(stack, len);
                let y = top_mut(stack, *len);
                *y = y.atan2(x);
            }
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "Degree is stored as f64 but always an integer, so cast is safe"
            )]
            Instruction::PolyEval(idx) => {
                let x_ptr = top_mut(stack, *len);
                let x = *x_ptr;
                let start = idx as usize;
                let degree = constants[start] as usize;

                let mut res = constants[start + 1];
                for i in 0..degree {
                    res = res.mul_add(x, constants[start + 2 + i]);
                }
                *x_ptr = res;
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Special functions take i32 parameters, truncation is expected for invalid inputs"
            )]
            Instruction::BesselJ => {
                let x = pop(stack, len);
                let n = top_mut(stack, *len);
                *n = if (*n).is_nan() {
                    f64::NAN
                } else {
                    crate::math::bessel_j((*n).round() as i32, x).unwrap_or(f64::NAN)
                };
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Bessel order is always a small integer"
            )]
            Instruction::BesselY => {
                let x = pop(stack, len);
                let n = top_mut(stack, *len);
                *n = if (*n).is_nan() {
                    f64::NAN
                } else {
                    crate::math::bessel_y((*n).round() as i32, x).unwrap_or(f64::NAN)
                };
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Bessel order is always a small integer"
            )]
            Instruction::BesselI => {
                let x = pop(stack, len);
                let n = top_mut(stack, *len);
                *n = if (*n).is_nan() {
                    f64::NAN
                } else {
                    crate::math::bessel_i((*n).round() as i32, x)
                };
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Bessel order is always a small integer"
            )]
            Instruction::BesselK => {
                let x = pop(stack, len);
                let n = top_mut(stack, *len);
                *n = if (*n).is_nan() {
                    f64::NAN
                } else {
                    crate::math::bessel_k((*n).round() as i32, x).unwrap_or(f64::NAN)
                };
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Polygamma order is always a small integer"
            )]
            Instruction::Polygamma => {
                let x = pop(stack, len);
                let n = top_mut(stack, *len);
                *n = if (*n).is_nan() {
                    f64::NAN
                } else {
                    crate::math::eval_polygamma((*n).round() as i32, x).unwrap_or(f64::NAN)
                };
            }
            Instruction::Beta => {
                let b = pop(stack, len);
                let a = top_mut(stack, *len);
                let ga = crate::math::eval_gamma(*a);
                let gb = crate::math::eval_gamma(b);
                let gab = crate::math::eval_gamma(*a + b);
                *a = match (ga, gb, gab) {
                    (Some(ga), Some(gb), Some(gab)) => ga * gb / gab,
                    _ => f64::NAN,
                };
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Zeta derivative order is always a small integer"
            )]
            Instruction::ZetaDeriv => {
                let s = pop(stack, len);
                let n = top_mut(stack, *len);
                *n = if (*n).is_nan() {
                    f64::NAN
                } else {
                    crate::math::eval_zeta_deriv((*n).round() as i32, s).unwrap_or(f64::NAN)
                };
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Hermite degree is always a small integer"
            )]
            Instruction::Hermite => {
                let x = pop(stack, len);
                let n = top_mut(stack, *len);
                *n = if (*n).is_nan() {
                    f64::NAN
                } else {
                    crate::math::eval_hermite((*n).round() as i32, x).unwrap_or(f64::NAN)
                };
            }

            // Three-argument functions
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Associated Legendre degree/order is always a small integer"
            )]
            Instruction::AssocLegendre => {
                let x = pop(stack, len);
                let m = pop(stack, len);
                let l = top_mut(stack, *len);
                *l = if (*l).is_nan() || m.is_nan() {
                    f64::NAN
                } else {
                    crate::math::eval_assoc_legendre((*l).round() as i32, m.round() as i32, x)
                        .unwrap_or(f64::NAN)
                };
            }

            // Four-argument functions
            #[allow(
                clippy::cast_possible_truncation,
                reason = "Spherical harmonic degree/order is always a small integer"
            )]
            Instruction::SphericalHarmonic => {
                let phi = pop(stack, len);
                let theta = pop(stack, len);
                let m = pop(stack, len);
                let l = top_mut(stack, *len);
                *l = if (*l).is_nan() || m.is_nan() {
                    f64::NAN
                } else {
                    crate::math::eval_spherical_harmonic(
                        (*l).round() as i32,
                        m.round() as i32,
                        theta,
                        phi,
                    )
                    .unwrap_or(f64::NAN)
                };
            }
        }
    }
}
