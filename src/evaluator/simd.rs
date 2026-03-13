#![allow(unsafe_op_in_unsafe_fn, reason = "Internal unsafe operations allowed")]
//! SIMD batch evaluation for the register-based evaluator.
//!
//! This module provides vectorized evaluation using `wide::f64x4` (4-wide SIMD).

use super::CompiledEvaluator;
use super::eval_math;
use super::instruction::{FnOp, Instruction};
use crate::core::error::DiffError;
use wide::f64x4;

const INLINE_SIMD_REGISTERS: usize = 64;

#[inline]
fn round_to_i32(x: f64) -> Option<i32> {
    let rounded = x.round();
    if !(rounded >= f64::from(i32::MIN) && rounded <= f64::from(i32::MAX)) {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "Range checked above before casting"
    )]
    Some(rounded as i32)
}

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
            for p in 0..p_count {
                // SAFETY: We verified that `columns.len() == p_count` and each `col.len() >= base + 3`.
                unsafe {
                    let col = *columns.get_unchecked(p);
                    *stack_ptr.add(p) = f64x4::new([
                        *col.get_unchecked(base),
                        *col.get_unchecked(base + 1),
                        *col.get_unchecked(base + 2),
                        *col.get_unchecked(base + 3),
                    ]);
                }
            }

            // Execute instructions
            // SAFETY: Instructions are well-formed during compilation.
            unsafe {
                Self::exec_simd_instructions(instructions, stack_ptr, simd_constants, constants);
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

        // Handle remainder with scalar path (1-3 points)
        let remainder_start = full_chunks * 4;
        if remainder_start < n_points {
            let mut scalar_registers = vec![0.0; self.register_count];
            let mut params_row = vec![0.0; p_count];
            for (i, out) in output[remainder_start..n_points].iter_mut().enumerate() {
                let point_idx = remainder_start + i;

                for p in 0..p_count {
                    params_row[p] = columns[p][point_idx];
                }

                *out = self.evaluate_heap(&params_row, &mut scalar_registers);
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
    ) {
        let one = f64x4::splat(1.0);
        for instr in instructions {
            match *instr {
                Instruction::LoadConst { dest, const_idx } => {
                    *registers.add(dest as usize) = simd_constants[const_idx as usize];
                }
                Instruction::Copy { dest, src } => {
                    *registers.add(dest as usize) = *registers.add(src as usize);
                }
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
                Instruction::Div { dest, num, den } => {
                    *registers.add(dest as usize) =
                        *registers.add(num as usize) / *registers.add(den as usize);
                }
                Instruction::Pow { dest, base, exp } => {
                    *registers.add(dest as usize) =
                        (*registers.add(base as usize)).pow_f64x4(*registers.add(exp as usize));
                }
                Instruction::Neg { dest, src } => {
                    *registers.add(dest as usize) = -*registers.add(src as usize);
                }
                Instruction::Builtin1 { dest, op, arg } => {
                    let dest_ptr = registers.add(dest as usize);
                    let x = *registers.add(arg as usize);
                    match op {
                        FnOp::Sin => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].sin(),
                                arr[1].sin(),
                                arr[2].sin(),
                                arr[3].sin(),
                            ]);
                        }
                        FnOp::Cos => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].cos(),
                                arr[1].cos(),
                                arr[2].cos(),
                                arr[3].cos(),
                            ]);
                        }
                        FnOp::Tan => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].tan(),
                                arr[1].tan(),
                                arr[2].tan(),
                                arr[3].tan(),
                            ]);
                        }
                        FnOp::Cot => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                1.0 / arr[0].tan(),
                                1.0 / arr[1].tan(),
                                1.0 / arr[2].tan(),
                                1.0 / arr[3].tan(),
                            ]);
                        }
                        FnOp::Sec => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                1.0 / arr[0].cos(),
                                1.0 / arr[1].cos(),
                                1.0 / arr[2].cos(),
                                1.0 / arr[3].cos(),
                            ]);
                        }
                        FnOp::Csc => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                1.0 / arr[0].sin(),
                                1.0 / arr[1].sin(),
                                1.0 / arr[2].sin(),
                                1.0 / arr[3].sin(),
                            ]);
                        }
                        FnOp::Asin => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].asin(),
                                arr[1].asin(),
                                arr[2].asin(),
                                arr[3].asin(),
                            ]);
                        }
                        FnOp::Acos => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].acos(),
                                arr[1].acos(),
                                arr[2].acos(),
                                arr[3].acos(),
                            ]);
                        }
                        FnOp::Atan => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].atan(),
                                arr[1].atan(),
                                arr[2].atan(),
                                arr[3].atan(),
                            ]);
                        }
                        FnOp::Acot => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                eval_math::eval_acot(arr[0]),
                                eval_math::eval_acot(arr[1]),
                                eval_math::eval_acot(arr[2]),
                                eval_math::eval_acot(arr[3]),
                            ]);
                        }
                        FnOp::Asec => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                (1.0 / arr[0]).acos(),
                                (1.0 / arr[1]).acos(),
                                (1.0 / arr[2]).acos(),
                                (1.0 / arr[3]).acos(),
                            ]);
                        }
                        FnOp::Acsc => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                (1.0 / arr[0]).asin(),
                                (1.0 / arr[1]).asin(),
                                (1.0 / arr[2]).asin(),
                                (1.0 / arr[3]).asin(),
                            ]);
                        }
                        FnOp::Sinh => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].sinh(),
                                arr[1].sinh(),
                                arr[2].sinh(),
                                arr[3].sinh(),
                            ]);
                        }
                        FnOp::Cosh => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].cosh(),
                                arr[1].cosh(),
                                arr[2].cosh(),
                                arr[3].cosh(),
                            ]);
                        }
                        FnOp::Tanh => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].tanh(),
                                arr[1].tanh(),
                                arr[2].tanh(),
                                arr[3].tanh(),
                            ]);
                        }
                        FnOp::Coth => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                1.0 / arr[0].tanh(),
                                1.0 / arr[1].tanh(),
                                1.0 / arr[2].tanh(),
                                1.0 / arr[3].tanh(),
                            ]);
                        }
                        FnOp::Sech => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                1.0 / arr[0].cosh(),
                                1.0 / arr[1].cosh(),
                                1.0 / arr[2].cosh(),
                                1.0 / arr[3].cosh(),
                            ]);
                        }
                        FnOp::Csch => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                1.0 / arr[0].sinh(),
                                1.0 / arr[1].sinh(),
                                1.0 / arr[2].sinh(),
                                1.0 / arr[3].sinh(),
                            ]);
                        }
                        FnOp::Asinh => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].asinh(),
                                arr[1].asinh(),
                                arr[2].asinh(),
                                arr[3].asinh(),
                            ]);
                        }
                        FnOp::Acosh => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].acosh(),
                                arr[1].acosh(),
                                arr[2].acosh(),
                                arr[3].acosh(),
                            ]);
                        }
                        FnOp::Atanh => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].atanh(),
                                arr[1].atanh(),
                                arr[2].atanh(),
                                arr[3].atanh(),
                            ]);
                        }
                        FnOp::Acoth => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                eval_math::eval_acoth(arr[0]),
                                eval_math::eval_acoth(arr[1]),
                                eval_math::eval_acoth(arr[2]),
                                eval_math::eval_acoth(arr[3]),
                            ]);
                        }
                        FnOp::Acsch => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                eval_math::eval_acsch(arr[0]),
                                eval_math::eval_acsch(arr[1]),
                                eval_math::eval_acsch(arr[2]),
                                eval_math::eval_acsch(arr[3]),
                            ]);
                        }
                        FnOp::Asech => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                eval_math::eval_asech(arr[0]),
                                eval_math::eval_asech(arr[1]),
                                eval_math::eval_asech(arr[2]),
                                eval_math::eval_asech(arr[3]),
                            ]);
                        }
                        FnOp::Exp => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].exp(),
                                arr[1].exp(),
                                arr[2].exp(),
                                arr[3].exp(),
                            ]);
                        }
                        FnOp::Expm1 => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].exp_m1(),
                                arr[1].exp_m1(),
                                arr[2].exp_m1(),
                                arr[3].exp_m1(),
                            ]);
                        }
                        FnOp::ExpNeg => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                (-arr[0]).exp(),
                                (-arr[1]).exp(),
                                (-arr[2]).exp(),
                                (-arr[3]).exp(),
                            ]);
                        }
                        FnOp::Ln => {
                            let arr = x.to_array();
                            *dest_ptr =
                                f64x4::new([arr[0].ln(), arr[1].ln(), arr[2].ln(), arr[3].ln()]);
                        }
                        FnOp::Log1p => {
                            let arr = x.to_array();
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
                            let arr = x.to_array();
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
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].signum(),
                                arr[1].signum(),
                                arr[2].signum(),
                                arr[3].signum(),
                            ]);
                        }
                        FnOp::Floor => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].floor(),
                                arr[1].floor(),
                                arr[2].floor(),
                                arr[3].floor(),
                            ]);
                        }
                        FnOp::Ceil => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].ceil(),
                                arr[1].ceil(),
                                arr[2].ceil(),
                                arr[3].ceil(),
                            ]);
                        }
                        FnOp::Round => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                arr[0].round(),
                                arr[1].round(),
                                arr[2].round(),
                                arr[3].round(),
                            ]);
                        }
                        FnOp::Erf => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                crate::math::eval_erf(arr[0]),
                                crate::math::eval_erf(arr[1]),
                                crate::math::eval_erf(arr[2]),
                                crate::math::eval_erf(arr[3]),
                            ]);
                        }
                        FnOp::Erfc => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                crate::math::eval_erfc(arr[0]),
                                crate::math::eval_erfc(arr[1]),
                                crate::math::eval_erfc(arr[2]),
                                crate::math::eval_erfc(arr[3]),
                            ]);
                        }
                        FnOp::Gamma => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                crate::math::eval_gamma(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_gamma(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_gamma(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_gamma(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::Digamma => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                crate::math::eval_digamma(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_digamma(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_digamma(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_digamma(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::Trigamma => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                crate::math::eval_trigamma(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_trigamma(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_trigamma(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_trigamma(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::Tetragamma => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                crate::math::eval_tetragamma(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_tetragamma(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_tetragamma(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_tetragamma(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::Sinc => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                eval_math::eval_sinc(arr[0]),
                                eval_math::eval_sinc(arr[1]),
                                eval_math::eval_sinc(arr[2]),
                                eval_math::eval_sinc(arr[3]),
                            ]);
                        }
                        FnOp::LambertW => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                crate::math::eval_lambert_w(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_lambert_w(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_lambert_w(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_lambert_w(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::EllipticK => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                crate::math::eval_elliptic_k(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_k(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_k(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_k(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::EllipticE => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                crate::math::eval_elliptic_e(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_e(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_e(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_elliptic_e(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::Zeta => {
                            let arr = x.to_array();
                            *dest_ptr = f64x4::new([
                                crate::math::eval_zeta(arr[0]).unwrap_or(f64::NAN),
                                crate::math::eval_zeta(arr[1]).unwrap_or(f64::NAN),
                                crate::math::eval_zeta(arr[2]).unwrap_or(f64::NAN),
                                crate::math::eval_zeta(arr[3]).unwrap_or(f64::NAN),
                            ]);
                        }
                        FnOp::ExpPolar => {
                            let arr = x.to_array();
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
                                round_to_i32(n_f).map_or(f64::NAN, |n| {
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
                                round_to_i32(n_f).map_or(f64::NAN, |n| {
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
                                round_to_i32(n_f)
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
                                round_to_i32(n_f).map_or(f64::NAN, |n| {
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
                                round_to_i32(n_f).map_or(f64::NAN, |n| {
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
                                round_to_i32(n_f).map_or(f64::NAN, |n| {
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
                                round_to_i32(n_f).map_or(f64::NAN, |n| {
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
                    arg1,
                    arg2,
                    arg3,
                } => {
                    let dest_ptr = registers.add(dest as usize);
                    let x1 = *registers.add(arg1 as usize);
                    let x2 = *registers.add(arg2 as usize);
                    let x3 = *registers.add(arg3 as usize);
                    let arr1 = x1.to_array();
                    let arr2 = x2.to_array();
                    let arr3 = x3.to_array();
                    if op == FnOp::AssocLegendre {
                        let f = |l_f: f64, m_f: f64, val: f64| match (
                            round_to_i32(l_f),
                            round_to_i32(m_f),
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
                    } else {
                        debug_assert!(false, "Reached unreachable SIMD Builtin3 op: {op:?}");
                        // SAFETY: All Builtin3 ops are exhaustively handled above.
                        unsafe { std::hint::unreachable_unchecked() }
                    }
                }
                Instruction::Builtin4 {
                    dest,
                    op,
                    arg1,
                    arg2,
                    arg3,
                    arg4,
                } => {
                    let dest_ptr = registers.add(dest as usize);
                    let x1 = *registers.add(arg1 as usize);
                    let x2 = *registers.add(arg2 as usize);
                    let x3 = *registers.add(arg3 as usize);
                    let x4 = *registers.add(arg4 as usize);
                    let arr1 = x1.to_array();
                    let arr2 = x2.to_array();
                    let arr3 = x3.to_array();
                    let arr4 = x4.to_array();
                    if op == FnOp::SphericalHarmonic {
                        let f = |l_f: f64, m_f: f64, t: f64, p: f64| match (
                            round_to_i32(l_f),
                            round_to_i32(m_f),
                        ) {
                            (Some(l), Some(m)) => {
                                crate::math::eval_spherical_harmonic(l, m, t, p).unwrap_or(f64::NAN)
                            }
                            _ => f64::NAN,
                        };
                        *dest_ptr = f64x4::new([
                            f(arr1[0], arr2[0], arr3[0], arr4[0]),
                            f(arr1[1], arr2[1], arr3[1], arr4[1]),
                            f(arr1[2], arr2[2], arr3[2], arr4[2]),
                            f(arr1[3], arr2[3], arr3[3], arr4[3]),
                        ]);
                    } else {
                        debug_assert!(false, "Reached unreachable SIMD Builtin4 op: {op:?}");
                        // SAFETY: All Builtin4 ops are exhaustively handled above.
                        unsafe { std::hint::unreachable_unchecked() }
                    }
                }
                Instruction::Square { dest, src } => {
                    let val = *registers.add(src as usize);
                    *registers.add(dest as usize) = val * val;
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
                Instruction::Recip { dest, src } => {
                    *registers.add(dest as usize) = one / *registers.add(src as usize);
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
                Instruction::MulAdd { dest, a, b, c } => {
                    *registers.add(dest as usize) = (*registers.add(a as usize))
                        .mul_add(*registers.add(b as usize), *registers.add(c as usize));
                }
                Instruction::MulSub { dest, a, b, c } => {
                    *registers.add(dest as usize) = (*registers.add(a as usize))
                        .mul_add(*registers.add(b as usize), -*registers.add(c as usize));
                }
                Instruction::NegMulAdd { dest, a, b, c } => {
                    *registers.add(dest as usize) = (-*registers.add(a as usize))
                        .mul_add(*registers.add(b as usize), *registers.add(c as usize));
                }
                Instruction::PolyEval { dest, x, const_idx } => {
                    let start = const_idx as usize;
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "Polynomial degree fits in usize"
                    )]
                    let degree = constants[start] as usize;
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
                Instruction::RecipExpm1 { dest, src } => {
                    let arr = (*registers.add(src as usize)).to_array();
                    *registers.add(dest as usize) = f64x4::new([
                        1.0 / arr[0].exp_m1(),
                        1.0 / arr[1].exp_m1(),
                        1.0 / arr[2].exp_m1(),
                        1.0 / arr[3].exp_m1(),
                    ]);
                }
                Instruction::ExpSqr { dest, src } => {
                    let arr = (*registers.add(src as usize)).to_array();
                    *registers.add(dest as usize) = f64x4::new([
                        (arr[0] * arr[0]).exp(),
                        (arr[1] * arr[1]).exp(),
                        (arr[2] * arr[2]).exp(),
                        (arr[3] * arr[3]).exp(),
                    ]);
                }
                Instruction::ExpSqrNeg { dest, src } => {
                    let arr = (*registers.add(src as usize)).to_array();
                    *registers.add(dest as usize) = f64x4::new([
                        (-(arr[0] * arr[0])).exp(),
                        (-(arr[1] * arr[1])).exp(),
                        (-(arr[2] * arr[2])).exp(),
                        (-(arr[3] * arr[3])).exp(),
                    ]);
                }
            }
        }
    }
}
