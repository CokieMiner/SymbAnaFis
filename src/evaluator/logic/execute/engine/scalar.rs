// Allow unsafe code for high-performance virtual machine execution.
#![allow(
    unsafe_code,
    reason = "The scalar evaluator achieves high performance through raw pointer arithmetic and unchecked register access. Safety is guaranteed by the compiler, which pre-validates all indices and stack requirements during the bytecode generation phase."
)]
//! Scalar evaluation implementation for the register-based evaluator.
//!
//! This module provides the `evaluate` method for single-point evaluation.
//! It uses a register-based virtual machine to execute bytecode instructions.

use crate::evaluator::CompiledEvaluator;
use crate::evaluator::logic::instruction::{FnOp, Instruction};
use std::cell::RefCell;

const INLINE_REGISTER_SIZE_SMALL: usize = 64;
const INLINE_REGISTER_SIZE_MEDIUM: usize = 256;

thread_local! {
    static HEAP_REGISTERS: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}

#[cold]
#[inline(never)]
fn unreachable_builtin(arity: usize, op: FnOp) -> f64 {
    debug_assert!(false, "Reached unreachable Builtin{arity} op: {op:?}");
    f64::NAN
}

impl CompiledEvaluator {
    /// Evaluates the compiled expression at a single point.
    #[inline]
    #[must_use]
    pub fn evaluate(&self, params: &[f64]) -> f64 {
        if self.workspace_size <= INLINE_REGISTER_SIZE_SMALL {
            self.evaluate_inline::<INLINE_REGISTER_SIZE_SMALL>(params)
        } else if self.workspace_size <= INLINE_REGISTER_SIZE_MEDIUM {
            self.evaluate_inline::<INLINE_REGISTER_SIZE_MEDIUM>(params)
        } else {
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
    }

    #[inline]
    fn setup_registers(&self, params: &[f64], registers: &mut [f64]) {
        registers.copy_from_slice(&self.registers_template);

        // Fill in actual parameters for this call.
        let p = self.param_count.min(params.len());
        registers[..p].copy_from_slice(&params[..p]);
    }

    #[inline(always)]
    fn evaluate_inline<const N: usize>(&self, params: &[f64]) -> f64 {
        use std::mem::MaybeUninit;
        let mut raw = [MaybeUninit::<f64>::uninit(); N];
        // SAFETY: `setup_registers` initializes all registers from template and params.
        let registers = unsafe {
            let ptr = raw.as_mut_ptr().cast::<f64>();
            let slice = std::slice::from_raw_parts_mut(ptr, self.workspace_size);
            self.setup_registers(params, slice);
            slice
        };
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
        clippy::undocumented_unsafe_blocks,
        reason = "Single loop handles all instruction variants for performance"
    )]
    #[allow(
        unsafe_op_in_unsafe_fn,
        reason = "Internal unsafe operations allowed in performance loop"
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
                Instruction::NegMul { dest, a, b } => unsafe {
                    let va = *registers.get_unchecked(a as usize);
                    let vb = *registers.get_unchecked(b as usize);
                    *registers.get_unchecked_mut(dest as usize) = -(va * vb);
                },
                Instruction::NegMulConst {
                    dest,
                    src,
                    const_idx,
                } => unsafe {
                    let v = *registers.get_unchecked(src as usize);
                    let c = *self.constants.get_unchecked(const_idx as usize);
                    *registers.get_unchecked_mut(dest as usize) = -(v * c);
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
                        FnOp::Acot => std::f64::consts::FRAC_PI_2 - x.atan(),
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
                        FnOp::Acoth => (1.0 / x).atanh(),
                        FnOp::Acsch => (1.0 / x).asinh(),
                        FnOp::Asech => (1.0 / x).acosh(),
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
                        FnOp::Gamma => crate::math::eval_gamma(x),
                        FnOp::Lgamma => crate::math::eval_lgamma(x),
                        FnOp::Digamma => crate::math::eval_digamma(x),
                        FnOp::Trigamma => crate::math::eval_trigamma(x),
                        FnOp::Tetragamma => crate::math::eval_tetragamma(x),
                        FnOp::Sinc => super::math::eval_sinc(x),
                        FnOp::LambertW => crate::math::eval_lambert_w(x),
                        FnOp::EllipticK => crate::math::eval_elliptic_k(x),
                        FnOp::EllipticE => crate::math::eval_elliptic_e(x),
                        FnOp::Zeta => crate::math::eval_zeta(x),
                        FnOp::ExpPolar => crate::math::eval_exp_polar(x),
                        _ => unreachable_builtin(1, op),
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
                            if x1 <= 0.0 || x1 == 1.0 || x2 < 0.0 {
                                f64::NAN
                            } else {
                                x2.log(x1)
                            }
                        }
                        FnOp::BesselJ => super::math::round_to_i32(x1)
                            .map_or(f64::NAN, |n| crate::math::bessel_j(n, x2)),
                        FnOp::BesselY => super::math::round_to_i32(x1)
                            .map_or(f64::NAN, |n| crate::math::bessel_y(n, x2)),
                        FnOp::BesselI => super::math::round_to_i32(x1)
                            .map_or(f64::NAN, |n| crate::math::bessel_i(n, x2)),
                        FnOp::BesselK => super::math::round_to_i32(x1)
                            .map_or(f64::NAN, |n| crate::math::bessel_k(n, x2)),
                        FnOp::Polygamma => super::math::round_to_i32(x1)
                            .map_or(f64::NAN, |n| crate::math::eval_polygamma(n, x2)),
                        FnOp::Beta => {
                            let ga = crate::math::eval_gamma(x1);
                            let gb = crate::math::eval_gamma(x2);
                            let gab = crate::math::eval_gamma(x1 + x2);
                            ga * gb / gab
                        }
                        FnOp::ZetaDeriv => super::math::round_to_i32(x1)
                            .map_or(f64::NAN, |n| crate::math::eval_zeta_deriv(n, x2)),
                        FnOp::Hermite => super::math::round_to_i32(x1)
                            .map_or(f64::NAN, |n| crate::math::eval_hermite(n, x2)),
                        _ => unreachable_builtin(2, op),
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
                            match (super::math::round_to_i32(x1), super::math::round_to_i32(x2)) {
                                (Some(l), Some(m)) => crate::math::eval_assoc_legendre(l, m, x3),
                                _ => f64::NAN,
                            }
                        }
                        _ => unreachable_builtin(3, op),
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
                            match (super::math::round_to_i32(x1), super::math::round_to_i32(x2)) {
                                (Some(l), Some(m)) => {
                                    crate::math::eval_spherical_harmonic(l, m, x3, x4)
                                }
                                _ => f64::NAN,
                            }
                        }
                        _ => unreachable_builtin(4, op),
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
                Instruction::NegMulAdd { dest, a, b, c } => unsafe {
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
                } => unsafe {
                    let va = *registers.get_unchecked(a as usize);
                    let vb = *registers.get_unchecked(b as usize);
                    let c = *self.constants.get_unchecked(const_idx as usize);
                    *registers.get_unchecked_mut(dest as usize) = (-va).mul_add(vb, c);
                },
                Instruction::PolyEval {
                    dest,
                    x,
                    const_idx,
                    degree,
                } => unsafe {
                    let val_x = *registers.get_unchecked(x as usize);
                    let start = const_idx as usize;
                    let mut res = *self.constants.get_unchecked(start);
                    for i in 0..degree {
                        res = res
                            .mul_add(val_x, *self.constants.get_unchecked(start + 1 + i as usize));
                    }
                    *registers.get_unchecked_mut(dest as usize) = res;
                },
                Instruction::Pow3_2 { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    *registers.get_unchecked_mut(dest as usize) = x * x.sqrt();
                },
                Instruction::InvPow3_2 { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    // NOTE: This could be optimized using hardware reciprocal square root (e.g., RSQRTSS on x86)
                    // if targeted explicitly or via intrinsics.
                    *registers.get_unchecked_mut(dest as usize) = 1.0 / (x * x.sqrt());
                },
                Instruction::InvSqrt { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    // NOTE: Hardware optimization potential (RSQRTSS/VRSQRTSS).
                    *registers.get_unchecked_mut(dest as usize) = 1.0 / x.sqrt();
                },
                Instruction::InvSquare { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    // NOTE: Reciprocal optimization potential (RCPSS/VRCPSS).
                    *registers.get_unchecked_mut(dest as usize) = 1.0 / (x * x);
                },
                Instruction::InvCube { dest, src } => unsafe {
                    let x = *registers.get_unchecked(src as usize);
                    // NOTE: Reciprocal optimization potential.
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
