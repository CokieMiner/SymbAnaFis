#![allow(unsafe_op_in_unsafe_fn, reason = "Internal unsafe operations allowed")]
#![allow(
    clippy::undocumented_unsafe_blocks,
    reason = "Internal unsafe operations allowed"
)]
//! Scalar evaluation implementation for the register-based evaluator.
//!
//! This module provides the `evaluate` method for single-point evaluation.
//! It uses a register-based virtual machine to execute bytecode instructions.

use super::CompiledEvaluator;
use super::instruction::{FnOp, Instruction};

const INLINE_REGISTER_SIZE_SMALL: usize = 64;
const INLINE_REGISTER_SIZE_MEDIUM: usize = 256;

impl CompiledEvaluator {
    /// Evaluates the compiled expression at a single point.
    #[inline]
    #[must_use]
    pub fn evaluate(&self, params: &[f64]) -> f64 {
        if self.register_count <= INLINE_REGISTER_SIZE_SMALL {
            self.evaluate_inline_small(params)
        } else if self.register_count <= INLINE_REGISTER_SIZE_MEDIUM {
            self.evaluate_inline_medium(params)
        } else {
            let mut registers = vec![0.0; self.register_count];
            self.evaluate_heap(params, &mut registers)
        }
    }

    #[inline]
    fn setup_registers(&self, params: &[f64], registers: &mut [f64]) {
        let p_count = self.param_count.min(params.len());
        if p_count > 0 {
            registers[..p_count].copy_from_slice(&params[..p_count]);
        }
        // Fill missing params with 0.0
        if p_count < self.param_count {
            registers[p_count..self.param_count].fill(0.0);
        }
        let c_len = self.constants.len();
        if c_len > 0 {
            registers[self.param_count..(self.param_count + c_len)]
                .copy_from_slice(&self.constants);
        }
    }

    #[inline]
    fn evaluate_inline_small(&self, params: &[f64]) -> f64 {
        let mut inline_registers = [0.0; INLINE_REGISTER_SIZE_SMALL];
        let registers = &mut inline_registers[..self.register_count];
        self.setup_registers(params, registers);
        self.exec_instructions(registers)
    }

    #[inline]
    fn evaluate_inline_medium(&self, params: &[f64]) -> f64 {
        let mut inline_registers = [0.0; INLINE_REGISTER_SIZE_MEDIUM];
        let registers = &mut inline_registers[..self.register_count];
        self.setup_registers(params, registers);
        self.exec_instructions(registers)
    }

    #[inline]
    pub(crate) fn evaluate_heap(&self, params: &[f64], registers: &mut [f64]) -> f64 {
        self.setup_registers(params, registers);
        self.exec_instructions(registers)
    }

    #[inline]
    #[allow(
        clippy::too_many_lines,
        reason = "Single loop handles all instruction variants for performance"
    )]
    fn exec_instructions(&self, registers: &mut [f64]) -> f64 {
        let mut pc = self.instructions.as_ptr();
        let end = unsafe { pc.add(self.instructions.len()) };

        while pc < end {
            let instr = unsafe { &*pc };
            pc = unsafe { pc.add(1) };

            match *instr {
                Instruction::Add { dest, a, b } => unsafe {
                    let va = *registers.get_unchecked(a as usize);
                    let vb = *registers.get_unchecked(b as usize);
                    *registers.get_unchecked_mut(dest as usize) = va + vb;
                },
                Instruction::Mul { dest, a, b } => unsafe {
                    let va = *registers.get_unchecked(a as usize);
                    let vb = *registers.get_unchecked(b as usize);
                    *registers.get_unchecked_mut(dest as usize) = va * vb;
                },
                Instruction::AddN {
                    dest,
                    start_idx,
                    count,
                } => unsafe {
                    let mut sum = 0.0;
                    for i in 0..count {
                        let reg_idx = *self.arg_pool.get_unchecked((start_idx + i) as usize);
                        sum += *registers.get_unchecked(reg_idx as usize);
                    }
                    *registers.get_unchecked_mut(dest as usize) = sum;
                },
                Instruction::MulN {
                    dest,
                    start_idx,
                    count,
                } => unsafe {
                    let mut prod = 1.0;
                    for i in 0..count {
                        let reg_idx = *self.arg_pool.get_unchecked((start_idx + i) as usize);
                        prod *= *registers.get_unchecked(reg_idx as usize);
                    }
                    *registers.get_unchecked_mut(dest as usize) = prod;
                },
                Instruction::Sub { dest, a, b } => unsafe {
                    let va = *registers.get_unchecked(a as usize);
                    let vb = *registers.get_unchecked(b as usize);
                    *registers.get_unchecked_mut(dest as usize) = va - vb;
                },
                Instruction::LoadConst { dest, const_idx } => unsafe {
                    *registers.get_unchecked_mut(dest as usize) =
                        *self.constants.get_unchecked(const_idx as usize);
                },
                Instruction::Copy { dest, src } => unsafe {
                    *registers.get_unchecked_mut(dest as usize) =
                        *registers.get_unchecked(src as usize);
                },
                Instruction::Square { dest, src } => unsafe {
                    let val = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = val * val;
                },
                Instruction::Neg { dest, src } => unsafe {
                    *registers.get_unchecked_mut(dest as usize) =
                        -*registers.get_unchecked(src as usize);
                },
                Instruction::MulAdd { dest, a, b, c } => unsafe {
                    let va = *registers.get_unchecked(a as usize);
                    let vb = *registers.get_unchecked(b as usize);
                    let vc = *registers.get_unchecked(c as usize);
                    *registers.get_unchecked_mut(dest as usize) = va.mul_add(vb, vc);
                },
                Instruction::MulAddConst {
                    dest,
                    a,
                    b,
                    const_idx,
                } => unsafe {
                    let va = *registers.get_unchecked(a as usize);
                    let vb = *registers.get_unchecked(b as usize);
                    let c = *self.constants.get_unchecked(const_idx as usize);
                    *registers.get_unchecked_mut(dest as usize) = va.mul_add(vb, c);
                },
                Instruction::AddConst {
                    dest,
                    src,
                    const_idx,
                } => unsafe {
                    let vs = *registers.get_unchecked(src as usize);
                    let c = *self.constants.get_unchecked(const_idx as usize);
                    *registers.get_unchecked_mut(dest as usize) = vs + c;
                },
                Instruction::MulConst {
                    dest,
                    src,
                    const_idx,
                } => unsafe {
                    let vs = *registers.get_unchecked(src as usize);
                    let c = *self.constants.get_unchecked(const_idx as usize);
                    *registers.get_unchecked_mut(dest as usize) = vs * c;
                },
                Instruction::SubConst {
                    dest,
                    src,
                    const_idx,
                } => unsafe {
                    let vs = *registers.get_unchecked(src as usize);
                    let c = *self.constants.get_unchecked(const_idx as usize);
                    *registers.get_unchecked_mut(dest as usize) = vs - c;
                },
                Instruction::ConstSub {
                    dest,
                    src,
                    const_idx,
                } => unsafe {
                    let vs = *registers.get_unchecked(src as usize);
                    let c = *self.constants.get_unchecked(const_idx as usize);
                    *registers.get_unchecked_mut(dest as usize) = c - vs;
                },
                Instruction::DivConst {
                    dest,
                    src,
                    const_idx,
                } => unsafe {
                    let vs = *registers.get_unchecked(src as usize);
                    let c = *self.constants.get_unchecked(const_idx as usize);
                    *registers.get_unchecked_mut(dest as usize) = vs / c;
                },
                Instruction::ConstDiv {
                    dest,
                    src,
                    const_idx,
                } => unsafe {
                    let vs = *registers.get_unchecked(src as usize);
                    let c = *self.constants.get_unchecked(const_idx as usize);
                    *registers.get_unchecked_mut(dest as usize) = c / vs;
                },
                Instruction::MulSub { dest, a, b, c } => unsafe {
                    let va = *registers.get_unchecked(a as usize);
                    let vb = *registers.get_unchecked(b as usize);
                    let vc = *registers.get_unchecked(c as usize);
                    *registers.get_unchecked_mut(dest as usize) = va.mul_add(vb, -vc);
                },
                Instruction::MulSubConst {
                    dest,
                    a,
                    b,
                    const_idx,
                } => unsafe {
                    let va = *registers.get_unchecked(a as usize);
                    let vb = *registers.get_unchecked(b as usize);
                    let c = *self.constants.get_unchecked(const_idx as usize);
                    *registers.get_unchecked_mut(dest as usize) = va.mul_add(vb, -c);
                },
                Instruction::Div { dest, num, den } => unsafe {
                    let va = *registers.get_unchecked(num as usize);
                    let vb = *registers.get_unchecked(den as usize);
                    *registers.get_unchecked_mut(dest as usize) = va / vb;
                },
                Instruction::Pow { dest, base, exp } => unsafe {
                    let va = *registers.get_unchecked(base as usize);
                    let vb = *registers.get_unchecked(exp as usize);
                    *registers.get_unchecked_mut(dest as usize) = va.powf(vb);
                },
                Instruction::Builtin1 { dest, op, arg } => unsafe {
                    let x = *registers.get_unchecked(arg as usize);
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
                        FnOp::Sinc => super::eval_math::eval_sinc(x),
                        FnOp::LambertW => crate::math::eval_lambert_w(x).unwrap_or(f64::NAN),
                        FnOp::EllipticK => crate::math::eval_elliptic_k(x).unwrap_or(f64::NAN),
                        FnOp::EllipticE => crate::math::eval_elliptic_e(x).unwrap_or(f64::NAN),
                        FnOp::Zeta => crate::math::eval_zeta(x).unwrap_or(f64::NAN),
                        FnOp::ExpPolar => crate::math::eval_exp_polar(x),
                        _ => {
                            debug_assert!(false, "Reached unreachable Builtin1 op: {op:?}");
                            // SAFETY: All Builtin1 ops are exhaustively handled above.
                            std::hint::unreachable_unchecked()
                        }
                    };
                    *registers.get_unchecked_mut(dest as usize) = res;
                },
                Instruction::Builtin2 {
                    dest,
                    op,
                    arg1,
                    arg2,
                } => unsafe {
                    let x1 = *registers.get_unchecked(arg1 as usize);
                    let x2 = *registers.get_unchecked(arg2 as usize);
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
                        FnOp::BesselJ => super::eval_math::round_to_i32(x1).map_or(f64::NAN, |n| {
                            crate::math::bessel_j(n, x2).unwrap_or(f64::NAN)
                        }),
                        FnOp::BesselY => super::eval_math::round_to_i32(x1).map_or(f64::NAN, |n| {
                            crate::math::bessel_y(n, x2).unwrap_or(f64::NAN)
                        }),
                        FnOp::BesselI => super::eval_math::round_to_i32(x1)
                            .map_or(f64::NAN, |n| crate::math::bessel_i(n, x2)),
                        FnOp::BesselK => super::eval_math::round_to_i32(x1).map_or(f64::NAN, |n| {
                            crate::math::bessel_k(n, x2).unwrap_or(f64::NAN)
                        }),
                        FnOp::Polygamma => super::eval_math::round_to_i32(x1)
                            .map_or(f64::NAN, |n| {
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
                        FnOp::ZetaDeriv => super::eval_math::round_to_i32(x1)
                            .map_or(f64::NAN, |n| {
                                crate::math::eval_zeta_deriv(n, x2).unwrap_or(f64::NAN)
                            }),
                        FnOp::Hermite => super::eval_math::round_to_i32(x1).map_or(f64::NAN, |n| {
                            crate::math::eval_hermite(n, x2).unwrap_or(f64::NAN)
                        }),
                        _ => {
                            debug_assert!(false, "Reached unreachable Builtin2 op: {op:?}");
                            // SAFETY: All Builtin2 ops are exhaustively handled above.
                            std::hint::unreachable_unchecked()
                        }
                    };
                    *registers.get_unchecked_mut(dest as usize) = res;
                },
                Instruction::Builtin3 {
                    dest,
                    op,
                    start_idx,
                } => unsafe {
                    let arg1_idx = *self.arg_pool.get_unchecked(start_idx as usize);
                    let arg2_idx = *self.arg_pool.get_unchecked((start_idx + 1) as usize);
                    let arg3_idx = *self.arg_pool.get_unchecked((start_idx + 2) as usize);
                    let x1 = *registers.get_unchecked(arg1_idx as usize);
                    let x2 = *registers.get_unchecked(arg2_idx as usize);
                    let x3 = *registers.get_unchecked(arg3_idx as usize);
                    #[allow(
                        clippy::single_match_else,
                        reason = "Match is used for architectural consistency with Builtin1/2 and ease of future expansion"
                    )]
                    let res = match op {
                        FnOp::AssocLegendre => {
                            match (
                                super::eval_math::round_to_i32(x1),
                                super::eval_math::round_to_i32(x2),
                            ) {
                                (Some(l), Some(m)) => {
                                    crate::math::eval_assoc_legendre(l, m, x3).unwrap_or(f64::NAN)
                                }
                                _ => f64::NAN,
                            }
                        }
                        _ => {
                            debug_assert!(false, "Reached unreachable Builtin3 op: {op:?}");
                            // SAFETY: All Builtin3 ops are exhaustively handled above.
                            std::hint::unreachable_unchecked()
                        }
                    };
                    *registers.get_unchecked_mut(dest as usize) = res;
                },
                Instruction::Builtin4 {
                    dest,
                    op,
                    start_idx,
                } => unsafe {
                    let arg1_idx = *self.arg_pool.get_unchecked(start_idx as usize);
                    let arg2_idx = *self.arg_pool.get_unchecked((start_idx + 1) as usize);
                    let arg3_idx = *self.arg_pool.get_unchecked((start_idx + 2) as usize);
                    let arg4_idx = *self.arg_pool.get_unchecked((start_idx + 3) as usize);
                    let x1 = *registers.get_unchecked(arg1_idx as usize);
                    let x2 = *registers.get_unchecked(arg2_idx as usize);
                    let x3 = *registers.get_unchecked(arg3_idx as usize);
                    let x4 = *registers.get_unchecked(arg4_idx as usize);
                    #[allow(
                        clippy::single_match_else,
                        reason = "Match is used for architectural consistency with Builtin1/2 and ease of future expansion"
                    )]
                    let res = match op {
                        FnOp::SphericalHarmonic => {
                            match (
                                super::eval_math::round_to_i32(x1),
                                super::eval_math::round_to_i32(x2),
                            ) {
                                (Some(l), Some(m)) => {
                                    crate::math::eval_spherical_harmonic(l, m, x3, x4)
                                        .unwrap_or(f64::NAN)
                                }
                                _ => f64::NAN,
                            }
                        }
                        _ => {
                            debug_assert!(false, "Reached unreachable Builtin4 op: {op:?}");
                            // SAFETY: All Builtin4 ops are exhaustively handled above.
                            std::hint::unreachable_unchecked()
                        }
                    };
                    *registers.get_unchecked_mut(dest as usize) = res;
                },
                Instruction::Cube { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = x * x * x;
                },
                Instruction::Pow4 { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    let x2 = x * x;
                    *registers.get_unchecked_mut(dest as usize) = x2 * x2;
                },
                Instruction::Recip { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = 1.0 / x;
                },
                Instruction::NegMulAdd { dest, a, b, c }
                | Instruction::MulSubRev { dest, a, b, c } => unsafe {
                    let va = *registers.get_unchecked(a as usize);
                    let vb = *registers.get_unchecked(b as usize);
                    let vc = *registers.get_unchecked(c as usize);
                    *registers.get_unchecked_mut(dest as usize) = (-va).mul_add(vb, vc);
                },
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
                } => unsafe {
                    let va = *registers.get_unchecked(a as usize);
                    let vb = *registers.get_unchecked(b as usize);
                    let c = *self.constants.get_unchecked(const_idx as usize);
                    *registers.get_unchecked_mut(dest as usize) = (-va).mul_add(vb, c);
                },
                Instruction::PolyEval { dest, x, const_idx } => unsafe {
                    let val_x = *registers.get_unchecked(x as usize);
                    let start = const_idx as usize;
                    #[allow(clippy::cast_possible_truncation, reason = "Degree fits in usize")]
                    let degree = (*self.constants.get_unchecked(start)).to_bits() as usize;
                    let mut res = *self.constants.get_unchecked(start + 1);
                    for i in 0..degree {
                        res = res.mul_add(val_x, *self.constants.get_unchecked(start + 2 + i));
                    }
                    *registers.get_unchecked_mut(dest as usize) = res;
                },
                Instruction::Pow3_2 { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = x * x.sqrt();
                },
                Instruction::InvPow3_2 { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = 1.0 / (x * x.sqrt());
                },
                Instruction::InvSqrt { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = 1.0 / x.sqrt();
                },
                Instruction::InvSquare { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = 1.0 / (x * x);
                },
                Instruction::InvCube { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = 1.0 / (x * x * x);
                },
                Instruction::Powi { dest, src, n } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = x.powi(n);
                },
                Instruction::RecipExpm1 { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = 1.0 / x.exp_m1();
                },
                Instruction::ExpSqr { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = (x * x).exp();
                },
                Instruction::ExpSqrNeg { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = (-(x * x)).exp();
                },
            }
        }

        // Result is always left in register 0
        registers[0]
    }
}
