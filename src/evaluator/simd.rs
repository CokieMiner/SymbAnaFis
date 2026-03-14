#![allow(unsafe_op_in_unsafe_fn, reason = "Internal unsafe operations allowed")]
#![allow(
    clippy::undocumented_unsafe_blocks,
    reason = "Internal unsafe operations allowed"
)]
//! SIMD batch evaluation for the register-based evaluator.
//!
//! This module provides vectorized evaluation using `wide::f64x4` (4-wide SIMD).

use super::CompiledEvaluator;
use super::eval_math;
use super::instruction::{FnOp, Instruction};
use crate::core::error::DiffError;
use wide::f64x4;

const INLINE_SIMD_REGISTERS: usize = 64;

impl CompiledEvaluator {
    /// Evaluates the compiled expression for multiple points in batch.
    #[allow(
        clippy::too_many_lines,
        reason = "Batch evaluation function handles multiple cases and optimizations"
    )]
    /// Evaluates a batch of rows.
    ///
    /// # Errors
    /// Returns `DiffError` if the column counts or lengths are mismatched,
    /// or if the output slice is too small.
    #[inline]
    pub fn eval_batch(
        &self,
        columns: &[&[f64]],
        output: &mut [f64],
        simd_buffer: Option<&mut Vec<f64x4>>,
    ) -> Result<(), DiffError> {
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

        #[allow(
            clippy::integer_division,
            reason = "We specifically want integer division to find chunk count"
        )]
        let full_chunks = n_points / 4;
        let simd_constants = self.simd_constants();

        let use_inline = self.register_count <= INLINE_SIMD_REGISTERS;

        // Use provided buffer or create local one
        let mut local_stack: Vec<f64x4> = Vec::new();
        let mut inline_simd_stack = [f64x4::splat(0.0); INLINE_SIMD_REGISTERS];

        let stack_ptr: *mut f64x4 = if use_inline {
            inline_simd_stack.as_mut_ptr()
        } else {
            let buf = simd_buffer.map_or_else(
                || {
                    local_stack = vec![f64x4::splat(0.0); self.register_count];
                    &mut local_stack
                },
                |b| {
                    b.clear();
                    b.resize(self.register_count, f64x4::splat(0.0));
                    b
                },
            );
            buf.as_mut_ptr()
        };

        let constants = &self.constants;
        let instructions = &self.instructions;
        let p_count = self.param_count;
        let c_len = constants.len();

        // Load constants into registers once so each chunk only updates params.
        if c_len > 0 {
            for c in 0..c_len {
                // SAFETY: `c_len` is `self.constants.len()` which matches `self.simd_constants.len()`.
                unsafe {
                    *stack_ptr.add(p_count + c) = *simd_constants.get_unchecked(c);
                }
            }
        }

        for chunk in 0..full_chunks {
            let base = chunk * 4;

            // Load parameters into registers
            for (p, col) in columns.iter().enumerate().take(p_count) {
                // SAFETY: We verified that `columns.len() == p_count` and each `col.len() >= base + 4`.
                unsafe {
                    let ptr = col.as_ptr().add(base).cast::<[f64; 4]>();
                    *stack_ptr.add(p) = f64x4::from(std::ptr::read_unaligned(ptr));
                }
            }

            // Execute instructions
            // SAFETY: Instructions are well-formed during compilation.
            unsafe {
                Self::exec_simd_instructions(
                    instructions,
                    stack_ptr,
                    simd_constants,
                    constants,
                    &self.arg_pool,
                );
            }

            // Result is in register 0
            // SAFETY: `stack_ptr` has space for at least 1 register.
            let result = unsafe { *stack_ptr };
            let arr = result.to_array();
            // SAFETY: `output` length was checked to be >= `n_points`.
            unsafe {
                *output.get_unchecked_mut(base) = arr[0];
                *output.get_unchecked_mut(base + 1) = arr[1];
                *output.get_unchecked_mut(base + 2) = arr[2];
                *output.get_unchecked_mut(base + 3) = arr[3];
            }
        }

        // Handle remainder using SIMD (1-3 points)
        let remainder_start = full_chunks * 4;
        let remainder_len = n_points - remainder_start;
        if remainder_len > 0 {
            // Load parameters for remainder, padding with 0.0
            for (p, col) in columns.iter().enumerate().take(p_count) {
                let mut vals = [0.0; 4];
                for (i, val) in vals.iter_mut().enumerate().take(remainder_len) {
                    *val = col[remainder_start + i];
                }
                unsafe {
                    *stack_ptr.add(p) = f64x4::new(vals);
                }
            }

            // Execute instructions
            unsafe {
                Self::exec_simd_instructions(
                    instructions,
                    stack_ptr,
                    simd_constants,
                    constants,
                    &self.arg_pool,
                );
            }

            // Result is in register 0
            let result = unsafe { *stack_ptr };
            let arr = result.to_array();
            for (i, val) in arr.iter().enumerate().take(remainder_len) {
                output[remainder_start + i] = *val;
            }
        }

        Ok(())
    }

    /// Evaluates the compiled expression for multiple points in parallel.
    ///
    /// # Errors
    /// Returns `DiffError` if the column counts or lengths are mismatched.
    #[cfg(feature = "parallel")]
    pub fn eval_batch_parallel(&self, columns: &[&[f64]]) -> Result<Vec<f64>, DiffError> {
        use rayon::prelude::*;
        const MIN_PARALLEL_SIZE: usize = 256;

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

        if n_points < MIN_PARALLEL_SIZE {
            let mut output = vec![0.0; n_points];
            self.eval_batch(columns, &mut output, None)?;
            return Ok(output);
        }

        let mut output = vec![0.0; n_points];
        output
            .par_chunks_mut(MIN_PARALLEL_SIZE)
            .enumerate()
            .try_for_each_init(
                || Vec::with_capacity(self.register_count),
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

    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        reason = "Dispatch loop with all instruction variants"
    )]
    #[inline]
    unsafe fn exec_simd_instructions(
        instructions: &[Instruction],
        registers: *mut f64x4,
        simd_constants: &[f64x4],
        constants: &[f64],
        arg_pool: &[u32],
    ) {
        let one = f64x4::splat(1.0);
        let mut pc = instructions.as_ptr();
        let end = pc.add(instructions.len());

        while pc < end {
            let instr = &*pc;
            pc = pc.add(1);

            match *instr {
                Instruction::Add { dest, a, b } => {
                    *registers.add(dest as usize) =
                        *registers.add(a as usize) + *registers.add(b as usize);
                }
                Instruction::Mul { dest, a, b } => {
                    *registers.add(dest as usize) =
                        *registers.add(a as usize) * *registers.add(b as usize);
                }
                Instruction::Sub { dest, a, b } => {
                    *registers.add(dest as usize) =
                        *registers.add(a as usize) - *registers.add(b as usize);
                }
                Instruction::AddN {
                    dest,
                    start_idx,
                    count,
                } => {
                    let mut sum = f64x4::splat(0.0);
                    for i in 0..count {
                        let reg_idx = *arg_pool.get_unchecked((start_idx + i) as usize);
                        sum += *registers.add(reg_idx as usize);
                    }
                    *registers.add(dest as usize) = sum;
                }
                Instruction::MulN {
                    dest,
                    start_idx,
                    count,
                } => {
                    let mut prod = f64x4::splat(1.0);
                    for i in 0..count {
                        let reg_idx = *arg_pool.get_unchecked((start_idx + i) as usize);
                        prod *= *registers.add(reg_idx as usize);
                    }
                    *registers.add(dest as usize) = prod;
                }
                Instruction::LoadConst { dest, const_idx } => {
                    *registers.add(dest as usize) = simd_constants[const_idx as usize];
                }
                Instruction::Copy { dest, src } => {
                    *registers.add(dest as usize) = *registers.add(src as usize);
                }
                Instruction::Square { dest, src } => {
                    let val = *registers.add(src as usize);
                    *registers.add(dest as usize) = val * val;
                }
                Instruction::Neg { dest, src } => {
                    *registers.add(dest as usize) = -*registers.add(src as usize);
                }
                Instruction::MulAdd { dest, a, b, c } => {
                    *registers.add(dest as usize) = (*registers.add(a as usize))
                        .mul_add(*registers.add(b as usize), *registers.add(c as usize));
                }
                Instruction::MulAddConst {
                    dest,
                    a,
                    b,
                    const_idx,
                } => {
                    let c = simd_constants[const_idx as usize];
                    *registers.add(dest as usize) =
                        (*registers.add(a as usize)).mul_add(*registers.add(b as usize), c);
                }
                Instruction::AddConst {
                    dest,
                    src,
                    const_idx,
                } => {
                    *registers.add(dest as usize) =
                        *registers.add(src as usize) + simd_constants[const_idx as usize];
                }
                Instruction::MulConst {
                    dest,
                    src,
                    const_idx,
                } => {
                    *registers.add(dest as usize) =
                        *registers.add(src as usize) * simd_constants[const_idx as usize];
                }
                Instruction::SubConst {
                    dest,
                    src,
                    const_idx,
                } => {
                    *registers.add(dest as usize) =
                        *registers.add(src as usize) - simd_constants[const_idx as usize];
                }
                Instruction::ConstSub {
                    dest,
                    src,
                    const_idx,
                } => {
                    *registers.add(dest as usize) =
                        simd_constants[const_idx as usize] - *registers.add(src as usize);
                }
                Instruction::DivConst {
                    dest,
                    src,
                    const_idx,
                } => {
                    *registers.add(dest as usize) =
                        *registers.add(src as usize) / simd_constants[const_idx as usize];
                }
                Instruction::ConstDiv {
                    dest,
                    src,
                    const_idx,
                } => {
                    *registers.add(dest as usize) =
                        simd_constants[const_idx as usize] / *registers.add(src as usize);
                }
                Instruction::MulSub { dest, a, b, c } => {
                    *registers.add(dest as usize) = (*registers.add(a as usize))
                        .mul_add(*registers.add(b as usize), -*registers.add(c as usize));
                }
                Instruction::MulSubConst {
                    dest,
                    a,
                    b,
                    const_idx,
                } => {
                    let c = simd_constants[const_idx as usize];
                    *registers.add(dest as usize) =
                        (*registers.add(a as usize)).mul_add(*registers.add(b as usize), -c);
                }
                Instruction::Div { dest, num, den } => {
                    *registers.add(dest as usize) =
                        *registers.add(num as usize) / *registers.add(den as usize);
                }
                Instruction::Pow { dest, base, exp } => {
                    *registers.add(dest as usize) =
                        (*registers.add(base as usize)).pow_f64x4(*registers.add(exp as usize));
                }
                Instruction::Builtin1 { dest, op, arg } => {
                    let dest_ptr = registers.add(dest as usize);
                    let x = *registers.add(arg as usize);
                    let arr = x.to_array();
                    match op {
                        FnOp::Sin => {
                            *dest_ptr = f64x4::new([
                                arr[0].sin(),
                                arr[1].sin(),
                                arr[2].sin(),
                                arr[3].sin(),
                            ]);
                        }
                        FnOp::Cos => {
                            *dest_ptr = f64x4::new([
                                arr[0].cos(),
                                arr[1].cos(),
                                arr[2].cos(),
                                arr[3].cos(),
                            ]);
                        }
                        FnOp::Tan => {
                            *dest_ptr = f64x4::new([
                                arr[0].tan(),
                                arr[1].tan(),
                                arr[2].tan(),
                                arr[3].tan(),
                            ]);
                        }
                        FnOp::Cot => {
                            let t = f64x4::new([
                                arr[0].tan(),
                                arr[1].tan(),
                                arr[2].tan(),
                                arr[3].tan(),
                            ]);
                            *dest_ptr = one / t;
                        }
                        FnOp::Sec => {
                            let c = f64x4::new([
                                arr[0].cos(),
                                arr[1].cos(),
                                arr[2].cos(),
                                arr[3].cos(),
                            ]);
                            *dest_ptr = one / c;
                        }
                        FnOp::Csc => {
                            let s = f64x4::new([
                                arr[0].sin(),
                                arr[1].sin(),
                                arr[2].sin(),
                                arr[3].sin(),
                            ]);
                            *dest_ptr = one / s;
                        }
                        FnOp::Asin => {
                            *dest_ptr = f64x4::new([
                                arr[0].asin(),
                                arr[1].asin(),
                                arr[2].asin(),
                                arr[3].asin(),
                            ]);
                        }
                        FnOp::Acos => {
                            *dest_ptr = f64x4::new([
                                arr[0].acos(),
                                arr[1].acos(),
                                arr[2].acos(),
                                arr[3].acos(),
                            ]);
                        }
                        FnOp::Atan => {
                            *dest_ptr = f64x4::new([
                                arr[0].atan(),
                                arr[1].atan(),
                                arr[2].atan(),
                                arr[3].atan(),
                            ]);
                        }
                        FnOp::Acot => {
                            *dest_ptr = f64x4::new([
                                eval_math::eval_acot(arr[0]),
                                eval_math::eval_acot(arr[1]),
                                eval_math::eval_acot(arr[2]),
                                eval_math::eval_acot(arr[3]),
                            ]);
                        }
                        FnOp::Asec => {
                            let inv = (one / x).to_array();
                            *dest_ptr = f64x4::new([
                                inv[0].acos(),
                                inv[1].acos(),
                                inv[2].acos(),
                                inv[3].acos(),
                            ]);
                        }
                        FnOp::Acsc => {
                            let inv = (one / x).to_array();
                            *dest_ptr = f64x4::new([
                                inv[0].asin(),
                                inv[1].asin(),
                                inv[2].asin(),
                                inv[3].asin(),
                            ]);
                        }
                        FnOp::Sinh => {
                            *dest_ptr = f64x4::new([
                                arr[0].sinh(),
                                arr[1].sinh(),
                                arr[2].sinh(),
                                arr[3].sinh(),
                            ]);
                        }
                        FnOp::Cosh => {
                            *dest_ptr = f64x4::new([
                                arr[0].cosh(),
                                arr[1].cosh(),
                                arr[2].cosh(),
                                arr[3].cosh(),
                            ]);
                        }
                        FnOp::Tanh => {
                            *dest_ptr = f64x4::new([
                                arr[0].tanh(),
                                arr[1].tanh(),
                                arr[2].tanh(),
                                arr[3].tanh(),
                            ]);
                        }
                        FnOp::Coth => {
                            let t = f64x4::new([
                                arr[0].tanh(),
                                arr[1].tanh(),
                                arr[2].tanh(),
                                arr[3].tanh(),
                            ]);
                            *dest_ptr = one / t;
                        }
                        FnOp::Sech => {
                            let c = f64x4::new([
                                arr[0].cosh(),
                                arr[1].cosh(),
                                arr[2].cosh(),
                                arr[3].cosh(),
                            ]);
                            *dest_ptr = one / c;
                        }
                        FnOp::Csch => {
                            let s = f64x4::new([
                                arr[0].sinh(),
                                arr[1].sinh(),
                                arr[2].sinh(),
                                arr[3].sinh(),
                            ]);
                            *dest_ptr = one / s;
                        }
                        FnOp::Asinh => {
                            *dest_ptr = f64x4::new([
                                arr[0].asinh(),
                                arr[1].asinh(),
                                arr[2].asinh(),
                                arr[3].asinh(),
                            ]);
                        }
                        FnOp::Acosh => {
                            *dest_ptr = f64x4::new([
                                arr[0].acosh(),
                                arr[1].acosh(),
                                arr[2].acosh(),
                                arr[3].acosh(),
                            ]);
                        }
                        FnOp::Atanh => {
                            *dest_ptr = f64x4::new([
                                arr[0].atanh(),
                                arr[1].atanh(),
                                arr[2].atanh(),
                                arr[3].atanh(),
                            ]);
                        }
                        FnOp::Acoth => {
                            *dest_ptr = f64x4::new([
                                eval_math::eval_acoth(arr[0]),
                                eval_math::eval_acoth(arr[1]),
                                eval_math::eval_acoth(arr[2]),
                                eval_math::eval_acoth(arr[3]),
                            ]);
                        }
                        FnOp::Acsch => {
                            *dest_ptr = f64x4::new([
                                eval_math::eval_acsch(arr[0]),
                                eval_math::eval_acsch(arr[1]),
                                eval_math::eval_acsch(arr[2]),
                                eval_math::eval_acsch(arr[3]),
                            ]);
                        }
                        FnOp::Asech => {
                            *dest_ptr = f64x4::new([
                                eval_math::eval_asech(arr[0]),
                                eval_math::eval_asech(arr[1]),
                                eval_math::eval_asech(arr[2]),
                                eval_math::eval_asech(arr[3]),
                            ]);
                        }
                        FnOp::Exp => {
                            *dest_ptr = f64x4::new([
                                arr[0].exp(),
                                arr[1].exp(),
                                arr[2].exp(),
                                arr[3].exp(),
                            ]);
                        }
                        FnOp::Expm1 => {
                            *dest_ptr = f64x4::new([
                                arr[0].exp_m1(),
                                arr[1].exp_m1(),
                                arr[2].exp_m1(),
                                arr[3].exp_m1(),
                            ]);
                        }
                        FnOp::ExpNeg => {
                            let neg_arr = (-x).to_array();
                            *dest_ptr = f64x4::new([
                                neg_arr[0].exp(),
                                neg_arr[1].exp(),
                                neg_arr[2].exp(),
                                neg_arr[3].exp(),
                            ]);
                        }
                        FnOp::Ln => {
                            *dest_ptr =
                                f64x4::new([arr[0].ln(), arr[1].ln(), arr[2].ln(), arr[3].ln()]);
                        }
                        FnOp::Log1p => {
                            *dest_ptr = f64x4::new([
                                arr[0].ln_1p(),
                                arr[1].ln_1p(),
                                arr[2].ln_1p(),
                                arr[3].ln_1p(),
                            ]);
                        }
                        FnOp::Sqrt => {
                            *dest_ptr = x.sqrt();
                        }
                        FnOp::Cbrt => {
                            *dest_ptr = f64x4::new([
                                arr[0].cbrt(),
                                arr[1].cbrt(),
                                arr[2].cbrt(),
                                arr[3].cbrt(),
                            ]);
                        }
                        FnOp::Abs => {
                            *dest_ptr = x.abs();
                        }
                        FnOp::Signum => {
                            *dest_ptr = f64x4::new([
                                arr[0].signum(),
                                arr[1].signum(),
                                arr[2].signum(),
                                arr[3].signum(),
                            ]);
                        }
                        FnOp::Floor => {
                            *dest_ptr = f64x4::new([
                                arr[0].floor(),
                                arr[1].floor(),
                                arr[2].floor(),
                                arr[3].floor(),
                            ]);
                        }
                        FnOp::Ceil => {
                            *dest_ptr = f64x4::new([
                                arr[0].ceil(),
                                arr[1].ceil(),
                                arr[2].ceil(),
                                arr[3].ceil(),
                            ]);
                        }
                        FnOp::Round => {
                            *dest_ptr = f64x4::new([
                                arr[0].round(),
                                arr[1].round(),
                                arr[2].round(),
                                arr[3].round(),
                            ]);
                        }
                        FnOp::Erf => {
                            *dest_ptr = f64x4::new([
                                crate::math::eval_erf(arr[0]),
                                crate::math::eval_erf(arr[1]),
                                crate::math::eval_erf(arr[2]),
                                crate::math::eval_erf(arr[3]),
                            ]);
                        }
                        FnOp::Erfc => {
                            *dest_ptr = f64x4::new([
                                crate::math::eval_erfc(arr[0]),
                                crate::math::eval_erfc(arr[1]),
                                crate::math::eval_erfc(arr[2]),
                                crate::math::eval_erfc(arr[3]),
                            ]);
                        }
                        FnOp::Gamma => {
                            *dest_ptr = f64x4::new([
                                crate::math::eval_gamma(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_gamma(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_gamma(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_gamma(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::Digamma => {
                            *dest_ptr = f64x4::new([
                                crate::math::eval_digamma(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_digamma(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_digamma(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_digamma(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::Trigamma => {
                            *dest_ptr = f64x4::new([
                                crate::math::eval_trigamma(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_trigamma(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_trigamma(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_trigamma(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::Tetragamma => {
                            *dest_ptr = f64x4::new([
                                crate::math::eval_tetragamma(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_tetragamma(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_tetragamma(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_tetragamma(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::Sinc => {
                            *dest_ptr = f64x4::new([
                                eval_math::eval_sinc(arr[0]),
                                eval_math::eval_sinc(arr[1]),
                                eval_math::eval_sinc(arr[2]),
                                eval_math::eval_sinc(arr[3]),
                            ]);
                        }
                        FnOp::LambertW => {
                            *dest_ptr = f64x4::new([
                                crate::math::eval_lambert_w(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_lambert_w(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_lambert_w(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_lambert_w(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::EllipticK => {
                            *dest_ptr = f64x4::new([
                                crate::math::eval_elliptic_k(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_k(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_k(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_k(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::EllipticE => {
                            *dest_ptr = f64x4::new([
                                crate::math::eval_elliptic_e(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_e(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_e(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_e(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::Zeta => {
                            *dest_ptr = f64x4::new([
                                crate::math::eval_zeta(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_zeta(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_zeta(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_zeta(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::ExpPolar => {
                            *dest_ptr = f64x4::new([
                                crate::math::eval_exp_polar(arr[0]),
                                crate::math::eval_exp_polar(arr[1]),
                                crate::math::eval_exp_polar(arr[2]),
                                crate::math::eval_exp_polar(arr[3]),
                            ]);
                        }
                        _ => {
                            debug_assert!(false, "Reached unreachable SIMD Builtin1 op: {op:?}");
                            // SAFETY: All Builtin1 ops are exhaustively handled above.
                            unsafe { std::hint::unreachable_unchecked() }
                        }
                    }
                }
                Instruction::Builtin2 {
                    dest,
                    op,
                    arg1,
                    arg2,
                } => {
                    let dest_ptr = registers.add(dest as usize);
                    let x1 = *registers.add(arg1 as usize);
                    let x2 = *registers.add(arg2 as usize);
                    let arr1 = x1.to_array();
                    let arr2 = x2.to_array();
                    match op {
                        FnOp::Atan2 => {
                            *dest_ptr = f64x4::new([
                                arr1[0].atan2(arr2[0]),
                                arr1[1].atan2(arr2[1]),
                                arr1[2].atan2(arr2[2]),
                                arr1[3].atan2(arr2[3]),
                            ]);
                        }
                        FnOp::Log => {
                            #[allow(
                                clippy::float_cmp,
                                reason = "Comparing exactly with 0.0/1.0 is intentional to handle signed zero correctly"
                            )]
                            let l = |base: f64, val: f64| {
                                if base <= 0.0 || base == 1.0 || val <= 0.0 {
                                    f64::NAN
                                } else {
                                    val.log(base)
                                }
                            };
                            *dest_ptr = f64x4::new([
                                l(arr1[0], arr2[0]),
                                l(arr1[1], arr2[1]),
                                l(arr1[2], arr2[2]),
                                l(arr1[3], arr2[3]),
                            ]);
                        }
                        FnOp::BesselJ => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f).map_or(f64::NAN, |n| {
                                    crate::math::bessel_j(n, val).unwrap_or(f64::NAN)
                                })
                            };
                            *dest_ptr = f64x4::new([
                                f(arr1[0], arr2[0]),
                                f(arr1[1], arr2[1]),
                                f(arr1[2], arr2[2]),
                                f(arr1[3], arr2[3]),
                            ]);
                        }
                        FnOp::BesselY => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f).map_or(f64::NAN, |n| {
                                    crate::math::bessel_y(n, val).unwrap_or(f64::NAN)
                                })
                            };
                            *dest_ptr = f64x4::new([
                                f(arr1[0], arr2[0]),
                                f(arr1[1], arr2[1]),
                                f(arr1[2], arr2[2]),
                                f(arr1[3], arr2[3]),
                            ]);
                        }
                        FnOp::BesselI => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f)
                                    .map_or(f64::NAN, |n| crate::math::bessel_i(n, val))
                            };
                            *dest_ptr = f64x4::new([
                                f(arr1[0], arr2[0]),
                                f(arr1[1], arr2[1]),
                                f(arr1[2], arr2[2]),
                                f(arr1[3], arr2[3]),
                            ]);
                        }
                        FnOp::BesselK => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f).map_or(f64::NAN, |n| {
                                    crate::math::bessel_k(n, val).unwrap_or(f64::NAN)
                                })
                            };
                            *dest_ptr = f64x4::new([
                                f(arr1[0], arr2[0]),
                                f(arr1[1], arr2[1]),
                                f(arr1[2], arr2[2]),
                                f(arr1[3], arr2[3]),
                            ]);
                        }
                        FnOp::Polygamma => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f).map_or(f64::NAN, |n| {
                                    crate::math::eval_polygamma(n, val).unwrap_or(f64::NAN)
                                })
                            };
                            *dest_ptr = f64x4::new([
                                f(arr1[0], arr2[0]),
                                f(arr1[1], arr2[1]),
                                f(arr1[2], arr2[2]),
                                f(arr1[3], arr2[3]),
                            ]);
                        }
                        FnOp::Beta => {
                            let f = |a: f64, b: f64| {
                                let ga = crate::math::eval_gamma(a);
                                let gb = crate::math::eval_gamma(b);
                                let gab = crate::math::eval_gamma(a + b);
                                match (ga, gb, gab) {
                                    (Some(va), Some(vb), Some(vab)) => va * vb / vab,
                                    _ => f64::NAN,
                                }
                            };
                            *dest_ptr = f64x4::new([
                                f(arr1[0], arr2[0]),
                                f(arr1[1], arr2[1]),
                                f(arr1[2], arr2[2]),
                                f(arr1[3], arr2[3]),
                            ]);
                        }
                        FnOp::ZetaDeriv => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f).map_or(f64::NAN, |n| {
                                    crate::math::eval_zeta_deriv(n, val).unwrap_or(f64::NAN)
                                })
                            };
                            *dest_ptr = f64x4::new([
                                f(arr1[0], arr2[0]),
                                f(arr1[1], arr2[1]),
                                f(arr1[2], arr2[2]),
                                f(arr1[3], arr2[3]),
                            ]);
                        }
                        FnOp::Hermite => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f).map_or(f64::NAN, |n| {
                                    crate::math::eval_hermite(n, val).unwrap_or(f64::NAN)
                                })
                            };
                            *dest_ptr = f64x4::new([
                                f(arr1[0], arr2[0]),
                                f(arr1[1], arr2[1]),
                                f(arr1[2], arr2[2]),
                                f(arr1[3], arr2[3]),
                            ]);
                        }
                        _ => {
                            debug_assert!(false, "Reached unreachable SIMD Builtin2 op: {op:?}");
                            // SAFETY: All Builtin2 ops are exhaustively handled above.
                            unsafe { std::hint::unreachable_unchecked() }
                        }
                    }
                }
                Instruction::Builtin3 {
                    dest,
                    op,
                    start_idx,
                } => {
                    let dest_ptr = registers.add(dest as usize);
                    let arg1_idx = *arg_pool.get_unchecked(start_idx as usize);
                    let arg2_idx = *arg_pool.get_unchecked((start_idx + 1) as usize);
                    let arg3_idx = *arg_pool.get_unchecked((start_idx + 2) as usize);
                    let x1 = *registers.add(arg1_idx as usize);
                    let x2 = *registers.add(arg2_idx as usize);
                    let x3 = *registers.add(arg3_idx as usize);
                    let arr1 = x1.to_array();
                    let arr2 = x2.to_array();
                    let arr3 = x3.to_array();
                    #[allow(
                        clippy::single_match_else,
                        reason = "Match is used for architectural consistency with Builtin1/2 and ease of future expansion"
                    )]
                    match op {
                        FnOp::AssocLegendre => {
                            let f = |l_f: f64, m_f: f64, val: f64| match (
                                eval_math::round_to_i32(l_f),
                                eval_math::round_to_i32(m_f),
                            ) {
                                (Some(l), Some(m)) => {
                                    crate::math::eval_assoc_legendre(l, m, val).unwrap_or(f64::NAN)
                                }
                                _ => f64::NAN,
                            };
                            *dest_ptr = f64x4::new([
                                f(arr1[0], arr2[0], arr3[0]),
                                f(arr1[1], arr2[1], arr3[1]),
                                f(arr1[2], arr2[2], arr3[2]),
                                f(arr1[3], arr2[3], arr3[3]),
                            ]);
                        }
                        _ => {
                            debug_assert!(false, "Reached unreachable SIMD Builtin3 op: {op:?}");
                            // SAFETY: All Builtin3 ops are exhaustively handled above.
                            unsafe { std::hint::unreachable_unchecked() }
                        }
                    }
                }
                Instruction::Builtin4 {
                    dest,
                    op,
                    start_idx,
                } => {
                    let dest_ptr = registers.add(dest as usize);
                    let arg1_idx = *arg_pool.get_unchecked(start_idx as usize);
                    let arg2_idx = *arg_pool.get_unchecked((start_idx + 1) as usize);
                    let arg3_idx = *arg_pool.get_unchecked((start_idx + 2) as usize);
                    let arg4_idx = *arg_pool.get_unchecked((start_idx + 3) as usize);
                    let x1 = *registers.add(arg1_idx as usize);
                    let x2 = *registers.add(arg2_idx as usize);
                    let x3 = *registers.add(arg3_idx as usize);
                    let x4 = *registers.add(arg4_idx as usize);
                    let arr1 = x1.to_array();
                    let arr2 = x2.to_array();
                    let arr3 = x3.to_array();
                    let arr4 = x4.to_array();
                    #[allow(
                        clippy::single_match_else,
                        reason = "Match is used for architectural consistency with Builtin1/2 and ease of future expansion"
                    )]
                    match op {
                        FnOp::SphericalHarmonic => {
                            let f = |l_f: f64, m_f: f64, t: f64, p: f64| match (
                                eval_math::round_to_i32(l_f),
                                eval_math::round_to_i32(m_f),
                            ) {
                                (Some(l), Some(m)) => {
                                    crate::math::eval_spherical_harmonic(l, m, t, p)
                                        .unwrap_or(f64::NAN)
                                }
                                _ => f64::NAN,
                            };
                            *dest_ptr = f64x4::new([
                                f(arr1[0], arr2[0], arr3[0], arr4[0]),
                                f(arr1[1], arr2[1], arr3[1], arr4[1]),
                                f(arr1[2], arr2[2], arr3[2], arr4[2]),
                                f(arr1[3], arr2[3], arr3[3], arr4[3]),
                            ]);
                        }
                        _ => {
                            debug_assert!(false, "Reached unreachable SIMD Builtin4 op: {op:?}");
                            // SAFETY: All Builtin4 ops are exhaustively handled above.
                            unsafe { std::hint::unreachable_unchecked() }
                        }
                    }
                }
                Instruction::Cube { dest, src } => {
                    let val = *registers.add(src as usize);
                    *registers.add(dest as usize) = val * val * val;
                }
                Instruction::Pow4 { dest, src } => {
                    let val = *registers.add(src as usize);
                    let val2 = val * val;
                    *registers.add(dest as usize) = val2 * val2;
                }
                Instruction::Recip { dest, src } => {
                    *registers.add(dest as usize) = one / *registers.add(src as usize);
                }
                Instruction::NegMulAdd { dest, a, b, c }
                | Instruction::MulSubRev { dest, a, b, c } => {
                    let va = *registers.add(a as usize);
                    let vb = *registers.add(b as usize);
                    let vc = *registers.add(c as usize);
                    *registers.add(dest as usize) = (-va).mul_add(vb, vc);
                }
                Instruction::NegMulAddConst {
                    dest,
                    a,
                    b,
                    const_idx,
                }
                | Instruction::MulSubRevConst {
                    dest,
                    a,
                    b,
                    const_idx,
                } => {
                    let va = *registers.add(a as usize);
                    let vb = *registers.add(b as usize);
                    let c = *simd_constants.get_unchecked(const_idx as usize);
                    *registers.add(dest as usize) = (-va).mul_add(vb, c);
                }
                Instruction::PolyEval { dest, x, const_idx } => {
                    let start = const_idx as usize;
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "Polynomial degree fits in usize"
                    )]
                    let degree = constants[start].to_bits() as usize;
                    // Safety: `compile_polynomial_with_base` stores [degree, c0..cN] contiguously
                    // in the constants pool, so `start + 1 + degree` is always in-bounds here.
                    let mut acc = *simd_constants.get_unchecked(start + 1);
                    let val_x = *registers.add(x as usize);
                    for i in 0..degree {
                        let coeff = *simd_constants.get_unchecked(start + 2 + i);
                        acc = acc.mul_add(val_x, coeff);
                    }
                    *registers.add(dest as usize) = acc;
                }
                Instruction::Pow3_2 { dest, src } => {
                    let val = *registers.add(src as usize);
                    *registers.add(dest as usize) = val * val.sqrt();
                }
                Instruction::InvPow3_2 { dest, src } => {
                    let val = *registers.add(src as usize);
                    *registers.add(dest as usize) = one / (val * val.sqrt());
                }
                Instruction::InvSqrt { dest, src } => {
                    *registers.add(dest as usize) = one / (*registers.add(src as usize)).sqrt();
                }
                Instruction::InvSquare { dest, src } => {
                    let val = *registers.add(src as usize);
                    *registers.add(dest as usize) = one / (val * val);
                }
                Instruction::InvCube { dest, src } => {
                    let val = *registers.add(src as usize);
                    *registers.add(dest as usize) = one / (val * val * val);
                }
                Instruction::Powi { dest, src, n } => {
                    let arr = (*registers.add(src as usize)).to_array();
                    *registers.add(dest as usize) = f64x4::new([
                        arr[0].powi(n),
                        arr[1].powi(n),
                        arr[2].powi(n),
                        arr[3].powi(n),
                    ]);
                }
                Instruction::RecipExpm1 { dest, src } => {
                    let arr = (*registers.add(src as usize)).to_array();
                    let expm1 = f64x4::new([
                        arr[0].exp_m1(),
                        arr[1].exp_m1(),
                        arr[2].exp_m1(),
                        arr[3].exp_m1(),
                    ]);
                    *registers.add(dest as usize) = one / expm1;
                }
                Instruction::ExpSqr { dest, src } => {
                    let val = *registers.add(src as usize);
                    let val2 = val * val;
                    let arr = val2.to_array();
                    *registers.add(dest as usize) =
                        f64x4::new([arr[0].exp(), arr[1].exp(), arr[2].exp(), arr[3].exp()]);
                }
                Instruction::ExpSqrNeg { dest, src } => {
                    let val = *registers.add(src as usize);
                    let val2 = -(val * val);
                    let arr = val2.to_array();
                    *registers.add(dest as usize) =
                        f64x4::new([arr[0].exp(), arr[1].exp(), arr[2].exp(), arr[3].exp()]);
                }
            }
        }
    }
}
