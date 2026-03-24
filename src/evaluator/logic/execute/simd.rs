#![allow(unsafe_op_in_unsafe_fn, reason = "Internal unsafe operations allowed")]
#![allow(
    unsafe_code,
    reason = "Vectorized evaluation uses raw pointers and unchecked indexing to maximize throughput. Safety is maintained by the compiler's static analysis of register bounds."
)]
#![allow(
    clippy::undocumented_unsafe_blocks,
    reason = "Internal unsafe operations allowed"
)]
//! SIMD batch evaluation for the register-based evaluator.
//!
//! This module provides vectorized evaluation using `wide::f64x4` (4-wide SIMD).

use super::math as eval_math;
use crate::core::error::DiffError;
use crate::evaluator::CompiledEvaluator;
use crate::evaluator::logic::instruction::{FnOp, Instruction};
use wide::f64x4;

const INLINE_SIMD_REGISTERS: usize = 64;

#[cold]
#[inline(never)]
fn unreachable_simd_builtin(arity: usize, op: FnOp) -> f64x4 {
    debug_assert!(false, "Reached unreachable SIMD Builtin{arity} op: {op:?}");
    f64x4::splat(f64::NAN)
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
    /// Returns `DiffError` if the used column lengths are mismatched,
    /// or if the output slice is too small.
    ///
    /// Missing parameter columns are treated as zero, matching [`Self::evaluate`].
    /// Extra columns are ignored.
    #[inline]
    pub fn eval_batch(
        &self,
        columns: &[&[f64]],
        output: &mut [f64],
        simd_buffer: Option<&mut Vec<f64x4>>,
    ) -> Result<(), DiffError> {
        let used_columns = &columns[..columns.len().min(self.param_count)];
        let n_points = if columns.is_empty() {
            1
        } else {
            columns[0].len()
        };

        if !used_columns.iter().all(|c| c.len() == n_points) {
            #[cold]
            #[inline(never)]
            const fn length_mismatch() -> DiffError {
                DiffError::EvalColumnLengthMismatch
            }
            return Err(length_mismatch());
        }
        if output.len() < n_points {
            #[cold]
            #[inline(never)]
            const fn output_too_small(needed: usize, got: usize) -> DiffError {
                DiffError::EvalOutputTooSmall { needed, got }
            }
            return Err(output_too_small(n_points, output.len()));
        }

        #[allow(
            clippy::integer_division,
            reason = "We specifically want integer division to find chunk count"
        )]
        let full_chunks = n_points / 4;
        let simd_constants = self.simd_constants();

        let use_inline = self.workspace_size <= INLINE_SIMD_REGISTERS;

        // Use provided buffer or create local one
        let mut local_stack: Vec<std::mem::MaybeUninit<f64x4>> = Vec::new();
        let mut inline_simd_stack =
            [std::mem::MaybeUninit::<f64x4>::uninit(); INLINE_SIMD_REGISTERS];

        let stack_ptr: *mut f64x4 = if use_inline {
            inline_simd_stack.as_mut_ptr().cast::<f64x4>()
        } else {
            simd_buffer.map_or_else(
                || {
                    // Use spare capacity to avoid zeroing
                    local_stack = vec![std::mem::MaybeUninit::uninit(); self.workspace_size];
                    local_stack.as_mut_ptr().cast::<f64x4>()
                },
                |b| {
                    // Reuse buffer without zeroing — constants and params are
                    // overwritten immediately, and temp registers are always
                    // written before read by the compiled instruction stream.
                    if b.len() < self.workspace_size {
                        b.clear();
                        b.reserve(self.workspace_size);
                        // SAFETY: Constants and params are overwritten immediately,
                        // and temp registers are always written before read.
                        #[allow(
                            clippy::uninit_vec,
                            reason = "Internal buffer initialized by instruction stream"
                        )]
                        unsafe {
                            b.set_len(self.workspace_size);
                        }
                    }
                    b.as_mut_ptr()
                },
            )
        };

        let instructions = &self.instructions;
        let p_count = self.param_count;
        let c_len = simd_constants.len();

        // Load constants into registers once so each chunk only updates params.
        if c_len > 0 {
            for c in 0..c_len {
                // SAFETY: `c_len` matches `self.simd_constants.len()`.
                unsafe {
                    *stack_ptr.add(p_count + c) = *simd_constants.get_unchecked(c);
                }
            }
        }

        for chunk in 0..full_chunks {
            let base = chunk * 4;

            // Load parameters into registers
            for (p, col) in used_columns.iter().enumerate() {
                // SAFETY: We verified that `columns.len() == p_count` and each `col.len() >= base + 4`.
                unsafe {
                    let ptr = col.as_ptr().add(base).cast::<[f64; 4]>();
                    *stack_ptr.add(p) = f64x4::from(std::ptr::read_unaligned(ptr));
                }
            }
            if used_columns.len() < p_count {
                for p in used_columns.len()..p_count {
                    unsafe {
                        *stack_ptr.add(p) = f64x4::splat(0.0);
                    }
                }
            }

            // Execute instructions
            // SAFETY: Instructions are well-formed during compilation.
            unsafe {
                Self::exec_simd_instructions(
                    instructions,
                    stack_ptr,
                    simd_constants,
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
            for (p, col) in used_columns.iter().enumerate() {
                let mut vals = [0.0; 4];
                for (i, val) in vals.iter_mut().enumerate().take(remainder_len) {
                    *val = col[remainder_start + i];
                }
                unsafe {
                    *stack_ptr.add(p) = f64x4::new(vals);
                }
            }
            if used_columns.len() < p_count {
                for p in used_columns.len()..p_count {
                    unsafe {
                        *stack_ptr.add(p) = f64x4::splat(0.0);
                    }
                }
            }

            // Execute instructions
            unsafe {
                Self::exec_simd_instructions(
                    instructions,
                    stack_ptr,
                    simd_constants,
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
    /// Returns `DiffError` if the used column lengths are mismatched.
    #[cfg(feature = "parallel")]
    pub fn eval_batch_parallel(&self, columns: &[&[f64]]) -> Result<Vec<f64>, DiffError> {
        use rayon::prelude::*;
        const MIN_PARALLEL_SIZE: usize = 256;

        let used_columns = &columns[..columns.len().min(self.param_count)];
        let n_points = if columns.is_empty() {
            1
        } else {
            columns[0].len()
        };

        if !used_columns.iter().all(|c| c.len() == n_points) {
            #[cold]
            #[inline(never)]
            const fn length_mismatch() -> DiffError {
                DiffError::EvalColumnLengthMismatch
            }
            return Err(length_mismatch());
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
                || {
                    (
                        Vec::with_capacity(self.workspace_size),
                        Vec::<&[f64]>::with_capacity(used_columns.len()),
                    )
                },
                |(simd_buffer, column_slices), (chunk_idx, out_chunk)| {
                    let start = chunk_idx * MIN_PARALLEL_SIZE;
                    let end = start + out_chunk.len();
                    column_slices.clear();
                    column_slices.extend(used_columns.iter().map(|col| &col[start..end]));
                    self.eval_batch(column_slices.as_slice(), out_chunk, Some(simd_buffer))
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
        arg_pool: &[u32],
    ) {
        let one = f64x4::splat(1.0);
        let pi_half = f64x4::splat(std::f64::consts::FRAC_PI_2);
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
                Instruction::NegMul { dest, a, b } => {
                    *registers.add(dest as usize) =
                        -(*registers.add(a as usize) * *registers.add(b as usize));
                }
                Instruction::NegMulConst {
                    dest,
                    src,
                    const_idx,
                } => {
                    let v = *registers.add(src as usize);
                    let c = simd_constants[const_idx as usize];
                    *registers.add(dest as usize) = -(v * c);
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
                            *dest_ptr = f64x4::new(arr.map(f64::sin));
                        }
                        FnOp::Cos => {
                            *dest_ptr = f64x4::new(arr.map(f64::cos));
                        }
                        FnOp::Tan => {
                            *dest_ptr = f64x4::new(arr.map(f64::tan));
                        }
                        FnOp::Cot => {
                            let t = f64x4::new(arr.map(f64::tan));
                            *dest_ptr = one / t;
                        }
                        FnOp::Sec => {
                            let c = f64x4::new(arr.map(f64::cos));
                            *dest_ptr = one / c;
                        }
                        FnOp::Csc => {
                            let s = f64x4::new(arr.map(f64::sin));
                            *dest_ptr = one / s;
                        }
                        FnOp::Asin => {
                            *dest_ptr = f64x4::new(arr.map(f64::asin));
                        }
                        FnOp::Acos => {
                            *dest_ptr = f64x4::new(arr.map(f64::acos));
                        }
                        FnOp::Atan => {
                            *dest_ptr = f64x4::new(arr.map(f64::atan));
                        }
                        FnOp::Acot => {
                            *dest_ptr = pi_half - x.atan();
                        }
                        FnOp::Asec => {
                            let inv = (one / x).to_array();
                            *dest_ptr = f64x4::new(inv.map(f64::acos));
                        }
                        FnOp::Acsc => {
                            let inv = (one / x).to_array();
                            *dest_ptr = f64x4::new(inv.map(f64::asin));
                        }
                        FnOp::Sinh => {
                            *dest_ptr = f64x4::new(arr.map(f64::sinh));
                        }
                        FnOp::Cosh => {
                            *dest_ptr = f64x4::new(arr.map(f64::cosh));
                        }
                        FnOp::Tanh => {
                            *dest_ptr = f64x4::new(arr.map(f64::tanh));
                        }
                        FnOp::Coth => {
                            let t = f64x4::new(arr.map(f64::tanh));
                            *dest_ptr = one / t;
                        }
                        FnOp::Sech => {
                            let c = f64x4::new(arr.map(f64::cosh));
                            *dest_ptr = one / c;
                        }
                        FnOp::Csch => {
                            let s = f64x4::new(arr.map(f64::sinh));
                            *dest_ptr = one / s;
                        }
                        FnOp::Asinh => {
                            *dest_ptr = f64x4::new(arr.map(f64::asinh));
                        }
                        FnOp::Acosh => {
                            *dest_ptr = f64x4::new(arr.map(f64::acosh));
                        }
                        FnOp::Atanh => {
                            *dest_ptr = f64x4::new(arr.map(f64::atanh));
                        }
                        FnOp::Acoth => {
                            let inv = (one / x).to_array();
                            *dest_ptr = f64x4::new(inv.map(f64::atanh));
                        }
                        FnOp::Acsch => {
                            let inv = (one / x).to_array();
                            *dest_ptr = f64x4::new(inv.map(f64::asinh));
                        }
                        FnOp::Asech => {
                            let inv = (one / x).to_array();
                            *dest_ptr = f64x4::new(inv.map(f64::acosh));
                        }
                        FnOp::Exp => {
                            *dest_ptr = f64x4::new(arr.map(f64::exp));
                        }
                        FnOp::Expm1 => {
                            *dest_ptr = f64x4::new(arr.map(f64::exp_m1));
                        }
                        FnOp::ExpNeg => {
                            let neg_arr = (-x).to_array();
                            *dest_ptr = f64x4::new(neg_arr.map(f64::exp));
                        }
                        FnOp::Ln => {
                            *dest_ptr = f64x4::new(arr.map(f64::ln));
                        }
                        FnOp::Log1p => {
                            *dest_ptr = f64x4::new(arr.map(f64::ln_1p));
                        }
                        FnOp::Sqrt => {
                            *dest_ptr = x.sqrt();
                        }
                        FnOp::Cbrt => {
                            *dest_ptr = f64x4::new(arr.map(f64::cbrt));
                        }
                        FnOp::Abs => {
                            *dest_ptr = x.abs();
                        }
                        FnOp::Signum => {
                            *dest_ptr = f64x4::new(arr.map(f64::signum));
                        }
                        FnOp::Floor => {
                            *dest_ptr = f64x4::new(arr.map(f64::floor));
                        }
                        FnOp::Ceil => {
                            *dest_ptr = f64x4::new(arr.map(f64::ceil));
                        }
                        FnOp::Round => {
                            *dest_ptr = f64x4::new(arr.map(f64::round));
                        }
                        FnOp::Erf => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_erf));
                        }
                        FnOp::Erfc => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_erfc));
                        }
                        FnOp::Gamma => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_gamma));
                        }
                        FnOp::Lgamma => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_lgamma));
                        }
                        FnOp::Digamma => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_digamma));
                        }
                        FnOp::Trigamma => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_trigamma));
                        }
                        FnOp::Tetragamma => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_tetragamma));
                        }
                        FnOp::Sinc => {
                            *dest_ptr = f64x4::new(arr.map(eval_math::eval_sinc));
                        }
                        FnOp::LambertW => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_lambert_w));
                        }
                        FnOp::EllipticK => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_elliptic_k));
                        }
                        FnOp::EllipticE => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_elliptic_e));
                        }
                        FnOp::Zeta => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_zeta));
                        }
                        FnOp::ExpPolar => {
                            *dest_ptr = f64x4::new(arr.map(crate::math::eval_exp_polar));
                        }
                        _ => {
                            *dest_ptr = unreachable_simd_builtin(1, op);
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
                            *dest_ptr = f64x4::new(std::array::from_fn(|i| arr1[i].atan2(arr2[i])));
                        }
                        FnOp::Log => {
                            #[allow(
                                clippy::float_cmp,
                                reason = "Comparing exactly with 0.0/1.0 is intentional to handle signed zero correctly"
                            )]
                            let l = |base: f64, val: f64| {
                                if base <= 0.0 || base == 1.0 || val < 0.0 {
                                    f64::NAN
                                } else {
                                    val.log(base)
                                }
                            };
                            *dest_ptr = f64x4::new(std::array::from_fn(|i| l(arr1[i], arr2[i])));
                        }
                        FnOp::BesselJ => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f)
                                    .map_or(f64::NAN, |n| crate::math::bessel_j(n, val))
                            };
                            *dest_ptr = f64x4::new(std::array::from_fn(|i| f(arr1[i], arr2[i])));
                        }
                        FnOp::BesselY => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f)
                                    .map_or(f64::NAN, |n| crate::math::bessel_y(n, val))
                            };
                            *dest_ptr = f64x4::new(std::array::from_fn(|i| f(arr1[i], arr2[i])));
                        }
                        FnOp::BesselI => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f)
                                    .map_or(f64::NAN, |n| crate::math::bessel_i(n, val))
                            };
                            *dest_ptr = f64x4::new(std::array::from_fn(|i| f(arr1[i], arr2[i])));
                        }
                        FnOp::BesselK => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f)
                                    .map_or(f64::NAN, |n| crate::math::bessel_k(n, val))
                            };
                            *dest_ptr = f64x4::new(std::array::from_fn(|i| f(arr1[i], arr2[i])));
                        }
                        FnOp::Polygamma => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f)
                                    .map_or(f64::NAN, |n| crate::math::eval_polygamma(n, val))
                            };
                            *dest_ptr = f64x4::new(std::array::from_fn(|i| f(arr1[i], arr2[i])));
                        }
                        FnOp::Beta => {
                            let f = |a: f64, b: f64| crate::math::eval_beta(a, b);
                            *dest_ptr = f64x4::new(std::array::from_fn(|i| f(arr1[i], arr2[i])));
                        }
                        FnOp::ZetaDeriv => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f)
                                    .map_or(f64::NAN, |n| crate::math::eval_zeta_deriv(n, val))
                            };
                            *dest_ptr = f64x4::new(std::array::from_fn(|i| f(arr1[i], arr2[i])));
                        }
                        FnOp::Hermite => {
                            let f = |n_f: f64, val: f64| {
                                eval_math::round_to_i32(n_f)
                                    .map_or(f64::NAN, |n| crate::math::eval_hermite(n, val))
                            };
                            *dest_ptr = f64x4::new(std::array::from_fn(|i| f(arr1[i], arr2[i])));
                        }
                        _ => {
                            *dest_ptr = unreachable_simd_builtin(2, op);
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
                                (Some(l), Some(m)) => crate::math::eval_assoc_legendre(l, m, val),
                                _ => f64::NAN,
                            };
                            *dest_ptr =
                                f64x4::new(std::array::from_fn(|i| f(arr1[i], arr2[i], arr3[i])));
                        }
                        _ => {
                            *dest_ptr = unreachable_simd_builtin(3, op);
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
                                }
                                _ => f64::NAN,
                            };
                            *dest_ptr = f64x4::new(std::array::from_fn(|i| {
                                f(arr1[i], arr2[i], arr3[i], arr4[i])
                            }));
                        }
                        _ => {
                            *dest_ptr = unreachable_simd_builtin(4, op);
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
                Instruction::NegMulAdd { dest, a, b, c } => {
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
                } => {
                    let va = *registers.add(a as usize);
                    let vb = *registers.add(b as usize);
                    let c = *simd_constants.get_unchecked(const_idx as usize);
                    *registers.add(dest as usize) = (-va).mul_add(vb, c);
                }
                Instruction::PolyEval {
                    dest,
                    x,
                    const_idx,
                    degree,
                } => {
                    let start = const_idx as usize;
                    let mut acc = *simd_constants.get_unchecked(start);
                    let val_x = *registers.add(x as usize);
                    for i in 0..degree {
                        let coeff = *simd_constants.get_unchecked(start + 1 + i as usize);
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
                    // NOTE: Hardware optimization potential (e.g., VRSQRTPS + Newton-Raphson).
                    *registers.add(dest as usize) = one / (val * val.sqrt());
                }
                Instruction::InvSqrt { dest, src } => {
                    // NOTE: Hardware optimization potential (e.g., VRSQRTPS).
                    *registers.add(dest as usize) = one / (*registers.add(src as usize)).sqrt();
                }
                Instruction::InvSquare { dest, src } => {
                    let val = *registers.add(src as usize);
                    // NOTE: Hardware optimization potential (e.g., VRCPPS).
                    *registers.add(dest as usize) = one / (val * val);
                }
                Instruction::InvCube { dest, src } => {
                    let val = *registers.add(src as usize);
                    // NOTE: Hardware optimization potential.
                    *registers.add(dest as usize) = one / (val * val * val);
                }
                Instruction::Powi { dest, src, n } => {
                    let arr = (*registers.add(src as usize)).to_array();
                    *registers.add(dest as usize) = f64x4::new(arr.map(|x| x.powi(n)));
                }
                Instruction::RecipExpm1 { dest, src } => {
                    let arr = (*registers.add(src as usize)).to_array();
                    let expm1 = f64x4::new(arr.map(f64::exp_m1));
                    *registers.add(dest as usize) = one / expm1;
                }
                Instruction::ExpSqr { dest, src } => {
                    let val = *registers.add(src as usize);
                    let val2 = val * val;
                    let arr = val2.to_array();
                    *registers.add(dest as usize) = f64x4::new(arr.map(f64::exp));
                }
                Instruction::ExpSqrNeg { dest, src } => {
                    let val = *registers.add(src as usize);
                    let val2 = -(val * val);
                    let arr = val2.to_array();
                    *registers.add(dest as usize) = f64x4::new(arr.map(f64::exp));
                }
            }
        }
    }
}
