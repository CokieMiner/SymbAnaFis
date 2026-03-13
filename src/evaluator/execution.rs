#![allow(unsafe_op_in_unsafe_fn, reason = "Internal unsafe operations allowed")]
//! Scalar evaluation implementation for the register-based evaluator.
//!
//! This module provides the `evaluate` method for single-point evaluation.
//! It uses a register-based virtual machine to execute bytecode instructions.

use super::CompiledEvaluator;
use super::instruction::{FnOp, Instruction};
use crate::core::traits::EPSILON;

const INLINE_REGISTER_SIZE: usize = 256;

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
    /// Evaluates the compiled expression at a single point.
    #[inline]
    #[must_use]
    pub fn evaluate(&self, params: &[f64]) -> f64 {
        if self.register_count <= INLINE_REGISTER_SIZE {
            self.evaluate_inline(params)
        } else {
            let mut registers = vec![0.0; self.register_count];
            self.evaluate_heap(params, &mut registers)
        }
    }

    #[inline]
    fn evaluate_inline(&self, params: &[f64]) -> f64 {
        let mut inline_registers = [0.0; INLINE_REGISTER_SIZE];
        let p_count = self.param_count.min(params.len());
        if p_count > 0 {
            inline_registers[..p_count].copy_from_slice(&params[..p_count]);
        }
        // Fill missing params with 0.0
        if p_count < self.param_count {
            inline_registers[p_count..self.param_count].fill(0.0);
        }
        let c_len = self.constants.len();
        if c_len > 0 {
            inline_registers[self.param_count..self.param_count + c_len]
                .copy_from_slice(&self.constants);
        }
        self.exec_instructions(&mut inline_registers)
    }

    #[inline]
    pub(crate) fn evaluate_heap(&self, params: &[f64], registers: &mut [f64]) -> f64 {
        let p_count = self.param_count.min(params.len());
        if p_count > 0 {
            registers[..p_count].copy_from_slice(&params[..p_count]);
        }
        let c_len = self.constants.len();
        if c_len > 0 {
            registers[self.param_count..self.param_count + c_len].copy_from_slice(&self.constants);
        }
        // Fill any missing params with 0.0
        registers[p_count..self.param_count].fill(0.0);

        self.exec_instructions(registers)
    }

    #[inline]
    #[allow(
        clippy::too_many_lines,
        reason = "Single loop handles all instruction variants for performance"
    )]
    fn exec_instructions(&self, registers: &mut [f64]) -> f64 {
        for instr in &*self.instructions {
            match *instr {
                Instruction::LoadConst { dest, const_idx } => {
                    registers[dest as usize] = self.constants[const_idx as usize];
                }
                Instruction::Copy { dest, src } => {
                    registers[dest as usize] = registers[src as usize];
                }
                Instruction::Add { dest, a, b } => {
                    let va = registers[a as usize];
                    let vb = registers[b as usize];
                    registers[dest as usize] = va + vb;
                }
                Instruction::Mul { dest, a, b } => {
                    let va = registers[a as usize];
                    let vb = registers[b as usize];
                    registers[dest as usize] = va * vb;
                }
                Instruction::Sub { dest, a, b } => {
                    let va = registers[a as usize];
                    let vb = registers[b as usize];
                    registers[dest as usize] = va - vb;
                }
                Instruction::Div { dest, num, den } => {
                    let va = registers[num as usize];
                    let vb = registers[den as usize];
                    registers[dest as usize] = va / vb;
                }
                Instruction::Pow { dest, base, exp } => {
                    let va = registers[base as usize];
                    let vb = registers[exp as usize];
                    registers[dest as usize] = va.powf(vb);
                }
                Instruction::Neg { dest, src } => {
                    registers[dest as usize] = -registers[src as usize];
                }
                Instruction::Builtin1 { dest, op, arg } => {
                    let x = registers[arg as usize];
                    let res = match op {
                        FnOp::Sin => x.sin(),
                        FnOp::Cos => x.cos(),
                        FnOp::Tan => x.tan(),
                        FnOp::Cot => 1.0 / x.tan(),
                        FnOp::Sec => 1.0 / x.cos(),
                        FnOp::Csc => 1.0 / x.sin(),
                        FnOp::Asin => x.asin(),
                        FnOp::Acos => x.acos(),
                        FnOp::Atan => x.atan(),
                        FnOp::Acot => super::eval_math::eval_acot(x),
                        FnOp::Asec => (1.0 / x).acos(),
                        FnOp::Acsc => (1.0 / x).asin(),
                        FnOp::Sinh => x.sinh(),
                        FnOp::Cosh => x.cosh(),
                        FnOp::Tanh => x.tanh(),
                        FnOp::Coth => 1.0 / x.tanh(),
                        FnOp::Sech => 1.0 / x.cosh(),
                        FnOp::Csch => 1.0 / x.sinh(),
                        FnOp::Asinh => x.asinh(),
                        FnOp::Acosh => x.acosh(),
                        FnOp::Atanh => x.atanh(),
                        FnOp::Acoth => super::eval_math::eval_acoth(x),
                        FnOp::Acsch => super::eval_math::eval_acsch(x),
                        FnOp::Asech => super::eval_math::eval_asech(x),
                        FnOp::Exp => x.exp(),
                        FnOp::Expm1 => x.exp_m1(),
                        FnOp::ExpNeg => (-x).exp(),
                        FnOp::Ln => x.ln(),
                        FnOp::Log1p => x.ln_1p(),
                        FnOp::Sqrt => x.sqrt(),
                        FnOp::Cbrt => x.cbrt(),
                        FnOp::Abs => x.abs(),
                        FnOp::Signum => x.signum(),
                        FnOp::Floor => x.floor(),
                        FnOp::Ceil => x.ceil(),
                        FnOp::Round => x.round(),
                        FnOp::Erf => crate::math::eval_erf(x),
                        FnOp::Erfc => crate::math::eval_erfc(x),
                        FnOp::Gamma => crate::math::eval_gamma(x).unwrap_or(f64::NAN),
                        FnOp::Digamma => crate::math::eval_digamma(x).unwrap_or(f64::NAN),
                        FnOp::Trigamma => crate::math::eval_trigamma(x).unwrap_or(f64::NAN),
                        FnOp::Tetragamma => crate::math::eval_tetragamma(x).unwrap_or(f64::NAN),
                        FnOp::Sinc => {
                            if x.abs() < EPSILON {
                                1.0
                            } else {
                                x.sin() / x
                            }
                        }
                        FnOp::LambertW => crate::math::eval_lambert_w(x).unwrap_or(f64::NAN),
                        FnOp::EllipticK => crate::math::eval_elliptic_k(x).unwrap_or(f64::NAN),
                        FnOp::EllipticE => crate::math::eval_elliptic_e(x).unwrap_or(f64::NAN),
                        FnOp::Zeta => crate::math::eval_zeta(x).unwrap_or(f64::NAN),
                        FnOp::ExpPolar => crate::math::eval_exp_polar(x),
                        _ => {
                            debug_assert!(false, "Reached unreachable Builtin1 op: {op:?}");
                            // SAFETY: All Builtin1 ops are exhaustively handled above.
                            unsafe { std::hint::unreachable_unchecked() }
                        }
                    };
                    registers[dest as usize] = res;
                }
                Instruction::Builtin2 {
                    dest,
                    op,
                    arg1,
                    arg2,
                } => {
                    let x1 = registers[arg1 as usize];
                    let x2 = registers[arg2 as usize];
                    let res = match op {
                        FnOp::Atan2 => x1.atan2(x2),
                        FnOp::Log =>
                        {
                            #[allow(
                                clippy::float_cmp,
                                reason = "Comparing exactly with 0.0 is intentional to handle positive/negative zero correctly"
                            )]
                            if x1 <= 0.0 || x1 == 1.0 || x2 <= 0.0 {
                                f64::NAN
                            } else {
                                x2.log(x1)
                            }
                        }
                        FnOp::BesselJ => round_to_i32(x1).map_or(f64::NAN, |n| {
                            crate::math::bessel_j(n, x2).unwrap_or(f64::NAN)
                        }),
                        FnOp::BesselY => round_to_i32(x1).map_or(f64::NAN, |n| {
                            crate::math::bessel_y(n, x2).unwrap_or(f64::NAN)
                        }),
                        FnOp::BesselI => {
                            round_to_i32(x1).map_or(f64::NAN, |n| crate::math::bessel_i(n, x2))
                        }
                        FnOp::BesselK => round_to_i32(x1).map_or(f64::NAN, |n| {
                            crate::math::bessel_k(n, x2).unwrap_or(f64::NAN)
                        }),
                        FnOp::Polygamma => round_to_i32(x1).map_or(f64::NAN, |n| {
                            crate::math::eval_polygamma(n, x2).unwrap_or(f64::NAN)
                        }),
                        FnOp::Beta => {
                            let ga = crate::math::eval_gamma(x1);
                            let gb = crate::math::eval_gamma(x2);
                            let gab = crate::math::eval_gamma(x1 + x2);
                            match (ga, gb, gab) {
                                (Some(a), Some(b), Some(ab)) => a * b / ab,
                                _ => f64::NAN,
                            }
                        }
                        FnOp::ZetaDeriv => round_to_i32(x1).map_or(f64::NAN, |n| {
                            crate::math::eval_zeta_deriv(n, x2).unwrap_or(f64::NAN)
                        }),
                        FnOp::Hermite => round_to_i32(x1).map_or(f64::NAN, |n| {
                            crate::math::eval_hermite(n, x2).unwrap_or(f64::NAN)
                        }),
                        _ => {
                            debug_assert!(false, "Reached unreachable Builtin2 op: {op:?}");
                            // SAFETY: All Builtin2 ops are exhaustively handled above.
                            unsafe { std::hint::unreachable_unchecked() }
                        }
                    };
                    registers[dest as usize] = res;
                }
                Instruction::Builtin3 {
                    dest,
                    op,
                    arg1,
                    arg2,
                    arg3,
                } => {
                    let x1 = registers[arg1 as usize];
                    let x2 = registers[arg2 as usize];
                    let x3 = registers[arg3 as usize];
                    let res = if op == FnOp::AssocLegendre {
                        match (round_to_i32(x1), round_to_i32(x2)) {
                            (Some(l), Some(m)) => {
                                crate::math::eval_assoc_legendre(l, m, x3).unwrap_or(f64::NAN)
                            }
                            _ => f64::NAN,
                        }
                    } else {
                        debug_assert!(false, "Reached unreachable Builtin3 op: {op:?}");
                        // SAFETY: All Builtin3 ops are exhaustively handled above.
                        unsafe { std::hint::unreachable_unchecked() }
                    };
                    registers[dest as usize] = res;
                }
                Instruction::Builtin4 {
                    dest,
                    op,
                    arg1,
                    arg2,
                    arg3,
                    arg4,
                } => {
                    let x1 = registers[arg1 as usize];
                    let x2 = registers[arg2 as usize];
                    let x3 = registers[arg3 as usize];
                    let x4 = registers[arg4 as usize];
                    let res = if op == FnOp::SphericalHarmonic {
                        match (round_to_i32(x1), round_to_i32(x2)) {
                            (Some(l), Some(m)) => {
                                crate::math::eval_spherical_harmonic(l, m, x3, x4)
                                    .unwrap_or(f64::NAN)
                            }
                            _ => f64::NAN,
                        }
                    } else {
                        debug_assert!(false, "Reached unreachable Builtin4 op: {op:?}");
                        // SAFETY: All Builtin4 ops are exhaustively handled above.
                        unsafe { std::hint::unreachable_unchecked() }
                    };
                    registers[dest as usize] = res;
                }
                Instruction::Square { dest, src } => {
                    registers[dest as usize] = registers[src as usize] * registers[src as usize];
                }
                Instruction::Cube { dest, src } => {
                    let x = registers[src as usize];
                    registers[dest as usize] = x * x * x;
                }
                Instruction::Pow4 { dest, src } => {
                    let x = registers[src as usize];
                    let x2 = x * x;
                    registers[dest as usize] = x2 * x2;
                }
                Instruction::Pow3_2 { dest, src } => {
                    let x = registers[src as usize];
                    registers[dest as usize] = x * x.sqrt();
                }
                Instruction::InvPow3_2 { dest, src } => {
                    let x = registers[src as usize];
                    registers[dest as usize] = 1.0 / (x * x.sqrt());
                }
                Instruction::InvSqrt { dest, src } => {
                    registers[dest as usize] = 1.0 / registers[src as usize].sqrt();
                }
                Instruction::InvSquare { dest, src } => {
                    let x = registers[src as usize];
                    registers[dest as usize] = 1.0 / (x * x);
                }
                Instruction::InvCube { dest, src } => {
                    let x = registers[src as usize];
                    registers[dest as usize] = 1.0 / (x * x * x);
                }
                Instruction::Recip { dest, src } => {
                    registers[dest as usize] = 1.0 / registers[src as usize];
                }
                Instruction::Powi { dest, src, n } => {
                    registers[dest as usize] = registers[src as usize].powi(n);
                }
                Instruction::MulAdd { dest, a, b, c } => {
                    registers[dest as usize] =
                        registers[a as usize].mul_add(registers[b as usize], registers[c as usize]);
                }
                Instruction::MulSub { dest, a, b, c } => {
                    registers[dest as usize] = registers[a as usize]
                        .mul_add(registers[b as usize], -registers[c as usize]);
                }
                Instruction::NegMulAdd { dest, a, b, c } => {
                    registers[dest as usize] = (-registers[a as usize])
                        .mul_add(registers[b as usize], registers[c as usize]);
                }
                Instruction::PolyEval { dest, x, const_idx } => {
                    let val_x = registers[x as usize];
                    let start = const_idx as usize;
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "Degree fits in usize"
                    )]
                    let degree = self.constants[start] as usize;
                    let mut res = self.constants[start + 1];
                    for i in 1..=degree {
                        res = res.mul_add(val_x, self.constants[start + 1 + i]);
                    }
                    registers[dest as usize] = res;
                }
                Instruction::RecipExpm1 { dest, src } => {
                    registers[dest as usize] = 1.0 / registers[src as usize].exp_m1();
                }
                Instruction::ExpSqr { dest, src } => {
                    let x = registers[src as usize];
                    registers[dest as usize] = (x * x).exp();
                }
                Instruction::ExpSqrNeg { dest, src } => {
                    let x = registers[src as usize];
                    registers[dest as usize] = (-(x * x)).exp();
                } // Handle unmapped patterns if any
            }
        }
        // Result is always left in register 0
        registers[0]
    }
}
