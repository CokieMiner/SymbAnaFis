use std::f64::consts::FRAC_PI_2;
use std::hash::{Hash, Hasher};
use std::mem::take;
use std::ptr::from_ref;

use rustc_hash::FxHashMap;

use super::ConstantPool;
use super::vir::{VInstruction, VReg};
use crate::EPSILON;
use crate::core::Expr;
use crate::evaluator::FnOp;
use crate::math::{
    bessel_i, bessel_j, bessel_k, bessel_y, eval_assoc_legendre, eval_beta, eval_digamma,
    eval_elliptic_e, eval_elliptic_k, eval_erf, eval_erfc, eval_exp_polar, eval_gamma,
    eval_hermite, eval_lambert_w, eval_lgamma, eval_polygamma, eval_spherical_harmonic,
    eval_tetragamma, eval_trigamma, eval_zeta, eval_zeta_deriv,
};

/// Key for the AST-level GVN cache used during VIR generation.
///
/// Wraps a raw pointer to an `Expr` node and uses the pre-computed structural
/// hash for O(1) equality rejection.
///
/// # Lifetime invariant
///
/// Pointers stored in `GvnKey` are valid for the lifetime of the `Arc<Expr>`
/// tree currently being compiled. The tree is immutable and pinned in memory
/// through `Arc`, so the pointers remain valid for the entire compilation.
#[derive(Clone, Copy, Debug)]
pub struct GvnKey(*const Expr);

impl GvnKey {
    /// Create a new GVN key from a reference to an expression node.
    ///
    /// The reference must come from the expression tree currently being
    /// compiled, ensuring the pointer remains valid for the duration of
    /// compilation.
    #[inline]
    pub(in crate::evaluator::logic::bytecode::compile) const fn new(expr: &Expr) -> Self {
        Self(from_ref(expr))
    }
}

impl PartialEq for GvnKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        #[allow(
            unsafe_code,
            reason = "Pointers in GvnKey are derived from valid Expr nodes in the tree currently being compiled. Using pointers and precomputed hashes is critical for GVN performance."
        )]
        // SAFETY: Pointers are derived from Arc-held Expr nodes in the root expression tree
        // and are valid for the duration of the GVN pass. Since Expr nodes are immutable
        // once created, pointer equality implies structural equality, and we fall back
        // to structural equality (which also checks the hash) if pointers differ.
        unsafe {
            self.0 == other.0 || *self.0 == *other.0
        }
    }
}

impl Eq for GvnKey {}

impl Hash for GvnKey {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        #[allow(
            unsafe_code,
            reason = "GvnKey pointers are guaranteed valid during compilation. Using the precomputed hash field avoids expensive recursive structural hashing."
        )]
        // SAFETY: The pointer is valid as it comes from the tree currently being compiled.
        unsafe {
            (*self.0).hash.hash(state);
        }
    }
}

/// Perform VIR-level Global Value Numbering.
///
/// This pass operates on the `VInstruction` stream after VIR generation.
/// It performs:
/// 1. Algebraic identity simplification (e.g. `x + 0 -> x`, `x * 1 -> x`)
/// 2. Constant folding (e.g. `2 * 3 -> 6`)
/// 3. Duplicate instruction elimination (Local Value Numbering)
#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::float_cmp,
    reason = "GVN operates across many instruction variants sequentially, and requires exact floating-point equality for algebraic identities"
)]
pub(in crate::evaluator::logic::bytecode::compile) fn optimize_vir_gvn(
    vinstrs: &mut Vec<VInstruction>,
    final_vreg: &mut Option<VReg>,
    constants: &mut Vec<f64>,
    const_map: &mut FxHashMap<u64, u32>,
    param_count: u32,
) {
    let mut pool = ConstantPool::with_index(constants, take(const_map), param_count);
    let mut seen = FxHashMap::default();
    let mut alias = FxHashMap::default();
    let mut optimized = Vec::with_capacity(vinstrs.len());

    macro_rules! emplace_const {
        ($val:expr) => {{
            let val: f64 = $val;
            // ConstantPool::get_or_insert returns physical register index (param_count + rel_idx).
            // We must subtract param_count to store the relative index in VReg::Const.
            VReg::Const(pool.get_or_insert(val) - param_count)
        }};
    }

    let get_const_val = |r: VReg, p: &ConstantPool| -> Option<f64> {
        if let VReg::Const(idx) = r {
            // VReg::Const(idx) stores the relative index.
            // ConstantPool::get_at expects the relative index.
            Some(p.get_at(idx))
        } else {
            None
        }
    };

    let is_zero =
        |r: VReg, p: &ConstantPool| -> bool { get_const_val(r, p).is_some_and(|v| v == 0.0) };

    let is_one =
        |r: VReg, p: &ConstantPool| -> bool { get_const_val(r, p).is_some_and(|v| v == 1.0) };

    for mut instr in take(vinstrs) {
        // Resolve Read Aliases
        instr.for_each_read_mut(|r| {
            while let Some(&canonical) = alias.get(r) {
                *r = canonical;
            }
        });

        // 1. Constant folding / Identity Simplification
        let replacement: Option<VReg> = match &mut instr {
            VInstruction::Add { srcs, .. } => {
                let mut sum = 0.0;
                let mut n_const = 0;
                srcs.retain(|r| {
                    get_const_val(*r, &pool).is_none_or(|val| {
                        sum += val;
                        n_const += 1;
                        false
                    })
                });

                if n_const > 0 && sum != 0.0 {
                    srcs.push(emplace_const!(sum));
                }

                if srcs.is_empty() {
                    Some(emplace_const!(0.0))
                } else if srcs.len() == 1 {
                    Some(srcs[0])
                } else {
                    None
                }
            }
            VInstruction::Add2 { a, b, .. } => {
                if is_zero(*a, &pool) {
                    Some(*b)
                } else if is_zero(*b, &pool) {
                    Some(*a)
                } else if let (Some(va), Some(vb)) =
                    (get_const_val(*a, &pool), get_const_val(*b, &pool))
                {
                    Some(emplace_const!(va + vb))
                } else {
                    None
                }
            }
            VInstruction::Mul { srcs, .. } => {
                let mut prod = 1.0;
                let mut n_const = 0;
                let mut is_zero_flag = false;

                srcs.retain(|r| {
                    get_const_val(*r, &pool).is_none_or(|val| {
                        if val == 0.0 {
                            is_zero_flag = true;
                        }
                        prod *= val;
                        n_const += 1;
                        false
                    })
                });

                if is_zero_flag {
                    // IEEE 754: 0*NaN=NaN, 0*Inf=NaN. We can only fold to 0 if
                    // all constant factors produced a finite product AND there are
                    // no remaining runtime operands that could be NaN/Inf.
                    if srcs.is_empty() {
                        Some(emplace_const!(if prod.is_nan() { f64::NAN } else { 0.0 }))
                    } else if prod.is_nan() {
                        Some(emplace_const!(f64::NAN))
                    } else {
                        // Runtime operands remain; can't fold since they might
                        // be NaN/Inf. Keep the zero constant as a factor.
                        srcs.push(emplace_const!(0.0));
                        None
                    }
                } else {
                    if n_const > 0 && prod != 1.0 {
                        srcs.push(emplace_const!(prod));
                    }

                    if srcs.is_empty() {
                        Some(emplace_const!(1.0))
                    } else if srcs.len() == 1 {
                        Some(srcs[0])
                    } else {
                        None
                    }
                }
            }
            VInstruction::Mul2 { a, b, dest } => {
                if let (Some(va), Some(vb)) = (get_const_val(*a, &pool), get_const_val(*b, &pool)) {
                    Some(emplace_const!(va * vb))
                } else if is_one(*a, &pool) {
                    Some(*b)
                } else if is_one(*b, &pool) {
                    Some(*a)
                } else if get_const_val(*a, &pool).is_some_and(|v| v == -1.0) {
                    // P11: Mul2{Const(-1), x} → Neg{x}
                    // Rewrite in-place so downstream fusion can match Neg patterns.
                    instr = VInstruction::Neg {
                        dest: *dest,
                        src: *b,
                    };
                    None
                } else if get_const_val(*b, &pool).is_some_and(|v| v == -1.0) {
                    // P11: Mul2{x, Const(-1)} → Neg{x}
                    instr = VInstruction::Neg {
                        dest: *dest,
                        src: *a,
                    };
                    None
                } else {
                    None
                }
            }
            VInstruction::Sub { a, b, .. } => {
                if let (Some(va), Some(vb)) = (get_const_val(*a, &pool), get_const_val(*b, &pool)) {
                    Some(emplace_const!(va - vb))
                } else if is_zero(*b, &pool) {
                    Some(*a)
                } else {
                    None
                }
            }
            VInstruction::Div { num, den, .. } => {
                if let (Some(vnum), Some(vden)) =
                    (get_const_val(*num, &pool), get_const_val(*den, &pool))
                {
                    Some(emplace_const!(vnum / vden))
                } else if is_one(*den, &pool) {
                    Some(*num)
                } else {
                    None
                }
            }
            VInstruction::Pow { base, exp, .. } => {
                if is_one(*exp, &pool) {
                    Some(*base)
                } else if is_zero(*exp, &pool) {
                    Some(emplace_const!(1.0))
                } else if is_zero(*base, &pool) {
                    Some(emplace_const!(0.0))
                } else if is_one(*base, &pool) {
                    Some(emplace_const!(1.0))
                } else if let (Some(vbase), Some(vexp)) =
                    (get_const_val(*base, &pool), get_const_val(*exp, &pool))
                {
                    Some(emplace_const!(vbase.powf(vexp)))
                } else {
                    None
                }
            }
            VInstruction::Neg { src, .. } => get_const_val(*src, &pool).map(|v| emplace_const!(-v)),
            VInstruction::Square { src, .. } => {
                get_const_val(*src, &pool).map(|v| emplace_const!(v * v))
            }
            VInstruction::Cube { src, .. } => {
                get_const_val(*src, &pool).map(|v| emplace_const!(v * v * v))
            }
            VInstruction::Pow4 { src, .. } => get_const_val(*src, &pool).map(|v| {
                let sq = v * v;
                emplace_const!(sq * sq)
            }),
            VInstruction::Pow3_2 { src, .. } => {
                get_const_val(*src, &pool).map(|v| emplace_const!(v.powf(1.5)))
            }
            VInstruction::InvPow3_2 { src, .. } => {
                get_const_val(*src, &pool).map(|v| emplace_const!(1.0 / v.powf(1.5)))
            }
            VInstruction::InvSqrt { src, .. } => {
                get_const_val(*src, &pool).map(|v| emplace_const!(1.0 / v.sqrt()))
            }
            VInstruction::InvSquare { src, .. } => {
                get_const_val(*src, &pool).map(|v| emplace_const!(1.0 / (v * v)))
            }
            VInstruction::InvCube { src, .. } => {
                get_const_val(*src, &pool).map(|v| emplace_const!(1.0 / (v * v * v)))
            }
            VInstruction::Recip { src, .. } => {
                get_const_val(*src, &pool).map(|v| emplace_const!(1.0 / v))
            }
            VInstruction::RecipExpm1 { src, .. } => {
                get_const_val(*src, &pool).map(|v| emplace_const!(1.0 / v.exp_m1()))
            }
            VInstruction::Powi { src, n, .. } => {
                if *n == 0 {
                    Some(emplace_const!(1.0))
                } else if *n == 1 {
                    Some(*src)
                } else {
                    get_const_val(*src, &pool).map(|v| emplace_const!(v.powi(*n)))
                }
            }
            VInstruction::Builtin1 { op, arg, .. } => {
                get_const_val(*arg, &pool).and_then(|v| {
                    let result = match *op {
                        FnOp::Sin => Some(v.sin()),
                        FnOp::Cos => Some(v.cos()),
                        FnOp::Tan => Some(v.tan()),
                        FnOp::Asin => Some(v.asin()),
                        FnOp::Acos => Some(v.acos()),
                        FnOp::Atan => Some(v.atan()),
                        FnOp::Sinh => Some(v.sinh()),
                        FnOp::Cosh => Some(v.cosh()),
                        FnOp::Tanh => Some(v.tanh()),
                        FnOp::Asinh => Some(v.asinh()),
                        FnOp::Acosh => Some(v.acosh()),
                        FnOp::Atanh => Some(v.atanh()),
                        FnOp::Exp => Some(v.exp()),
                        FnOp::Expm1 => Some(v.exp_m1()),
                        FnOp::ExpNeg => Some((-v).exp()),
                        FnOp::Ln => Some(v.ln()),
                        FnOp::Log1p => Some(v.ln_1p()),
                        FnOp::Sqrt => Some(v.sqrt()),
                        FnOp::Cbrt => Some(v.cbrt()),
                        FnOp::Abs => Some(v.abs()),
                        FnOp::Floor => Some(v.floor()),
                        FnOp::Ceil => Some(v.ceil()),
                        FnOp::Round => Some(v.round()),
                        FnOp::Signum => Some(v.signum()),
                        FnOp::Cot => Some(1.0 / v.tan()),
                        FnOp::Sec => Some(1.0 / v.cos()),
                        FnOp::Csc => Some(1.0 / v.sin()),
                        FnOp::Acot => Some(FRAC_PI_2 - v.atan()),
                        FnOp::Asec => Some((1.0 / v).acos()),
                        FnOp::Acsc => Some((1.0 / v).asin()),
                        FnOp::Coth => Some(1.0 / v.tanh()),
                        FnOp::Sech => Some(1.0 / v.cosh()),
                        FnOp::Csch => Some(1.0 / v.sinh()),
                        FnOp::Acoth => Some(0.5 * ((v + 1.0) / (v - 1.0)).ln()),
                        FnOp::Acsch => Some((1.0 / v + v.mul_add(v, 1.0).sqrt() / v.abs()).ln()), // Note: Acsch simplified mathematically
                        FnOp::Asech => Some(((1.0 + v.mul_add(-v, 1.0).sqrt()) / v).ln()),
                        FnOp::Sinc => Some(if v.abs() < EPSILON { 1.0 } else { v.sin() / v }),

                        // special functions
                        FnOp::Erf => Some(eval_erf(v)),
                        FnOp::Erfc => Some(eval_erfc(v)),
                        FnOp::Gamma => Some(eval_gamma(v)),
                        FnOp::Lgamma => Some(eval_lgamma(v)),
                        FnOp::Digamma => Some(eval_digamma(v)),
                        FnOp::Trigamma => Some(eval_trigamma(v)),
                        FnOp::Tetragamma => Some(eval_tetragamma(v)),
                        FnOp::LambertW => Some(eval_lambert_w(v)),
                        FnOp::EllipticK => Some(eval_elliptic_k(v)),
                        FnOp::EllipticE => Some(eval_elliptic_e(v)),
                        FnOp::Zeta => Some(eval_zeta(v)),
                        FnOp::ExpPolar => Some(eval_exp_polar(v)),

                        // The following functions belong to FnOp, but they have arity > 1.
                        // We must match them here to satisfy Rust's exhaustive pattern matching rules,
                        // even though they should realistically never appear inside a Builtin1 instruction.
                        FnOp::Atan2
                        | FnOp::Log
                        | FnOp::BesselJ
                        | FnOp::BesselY
                        | FnOp::BesselI
                        | FnOp::BesselK
                        | FnOp::Polygamma
                        | FnOp::Beta
                        | FnOp::ZetaDeriv
                        | FnOp::Hermite
                        | FnOp::AssocLegendre
                        | FnOp::SphericalHarmonic => None,
                    };
                    result.map(|val| emplace_const!(val))
                })
            }
            VInstruction::Builtin2 { op, arg1, arg2, .. } => {
                if let (Some(v1), Some(v2)) =
                    (get_const_val(*arg1, &pool), get_const_val(*arg2, &pool))
                {
                    let result = match *op {
                        FnOp::Atan2 => Some(v1.atan2(v2)),
                        FnOp::Log => Some(v2.log(v1)),
                        FnOp::Beta => Some(eval_beta(v1, v2)),
                        op @ (FnOp::BesselJ
                        | FnOp::BesselY
                        | FnOp::BesselI
                        | FnOp::BesselK
                        | FnOp::Polygamma
                        | FnOp::ZetaDeriv
                        | FnOp::Hermite) => {
                            // These require the first argument to be an integer mathematically.
                            // We round silently to match runtime behavior.
                            let v1_r = v1.round();
                            (v1_r >= f64::from(i32::MIN) && v1_r <= f64::from(i32::MAX)).then(
                                || {
                                    #[allow(
                                        clippy::cast_possible_truncation,
                                        reason = "Bounds checked"
                                    )]
                                    let n = v1_r as i32;
                                    match op {
                                        FnOp::BesselJ => bessel_j(n, v2),
                                        FnOp::BesselY => bessel_y(n, v2),
                                        FnOp::BesselI => bessel_i(n, v2),
                                        FnOp::BesselK => bessel_k(n, v2),
                                        FnOp::Polygamma => eval_polygamma(n, v2),
                                        FnOp::ZetaDeriv => eval_zeta_deriv(n, v2),
                                        FnOp::Hermite => eval_hermite(n, v2),
                                        _ => f64::NAN,
                                    }
                                },
                            )
                        }

                        // The following functions belong to FnOp, but they have arity != 2.
                        // We must match them here to satisfy Rust's exhaustive pattern matching rules,
                        // even though they should realistically never appear inside a Builtin2 instruction.
                        FnOp::Sin
                        | FnOp::Cos
                        | FnOp::Tan
                        | FnOp::Cot
                        | FnOp::Sec
                        | FnOp::Csc
                        | FnOp::Asin
                        | FnOp::Acos
                        | FnOp::Atan
                        | FnOp::Acot
                        | FnOp::Asec
                        | FnOp::Acsc
                        | FnOp::Sinh
                        | FnOp::Cosh
                        | FnOp::Tanh
                        | FnOp::Coth
                        | FnOp::Sech
                        | FnOp::Csch
                        | FnOp::Asinh
                        | FnOp::Acosh
                        | FnOp::Atanh
                        | FnOp::Acoth
                        | FnOp::Acsch
                        | FnOp::Asech
                        | FnOp::Exp
                        | FnOp::Expm1
                        | FnOp::ExpNeg
                        | FnOp::Ln
                        | FnOp::Log1p
                        | FnOp::Sqrt
                        | FnOp::Cbrt
                        | FnOp::Abs
                        | FnOp::Signum
                        | FnOp::Floor
                        | FnOp::Ceil
                        | FnOp::Round
                        | FnOp::Erf
                        | FnOp::Erfc
                        | FnOp::Gamma
                        | FnOp::Lgamma
                        | FnOp::Digamma
                        | FnOp::Trigamma
                        | FnOp::Tetragamma
                        | FnOp::Sinc
                        | FnOp::LambertW
                        | FnOp::EllipticK
                        | FnOp::EllipticE
                        | FnOp::Zeta
                        | FnOp::ExpPolar
                        | FnOp::AssocLegendre
                        | FnOp::SphericalHarmonic => None,
                    };
                    result.map(|val| emplace_const!(val))
                } else {
                    None
                }
            }
            VInstruction::BuiltinFun { op, args, .. } => {
                let mut all_consts = true;
                let mut c_args = Vec::with_capacity(args.len());
                for a in args.iter() {
                    if let Some(c) = get_const_val(*a, &pool) {
                        c_args.push(c);
                    } else {
                        all_consts = false;
                        break;
                    }
                }

                if all_consts {
                    let res = match (*op, c_args.as_slice()) {
                        (FnOp::AssocLegendre, &[l, m, x]) => {
                            let lr = l.round();
                            let mr = m.round();
                            (lr >= f64::from(i32::MIN)
                                && lr <= f64::from(i32::MAX)
                                && mr >= f64::from(i32::MIN)
                                && mr <= f64::from(i32::MAX))
                            .then(|| {
                                #[allow(
                                    clippy::cast_possible_truncation,
                                    reason = "Bounds checked"
                                )]
                                eval_assoc_legendre(lr as i32, mr as i32, x)
                            })
                        }
                        (FnOp::SphericalHarmonic, &[l, m, theta, phi]) => {
                            let lr = l.round();
                            let mr = m.round();
                            (lr >= f64::from(i32::MIN)
                                && lr <= f64::from(i32::MAX)
                                && mr >= f64::from(i32::MIN)
                                && mr <= f64::from(i32::MAX))
                            .then(|| {
                                #[allow(
                                    clippy::cast_possible_truncation,
                                    reason = "Bounds checked"
                                )]
                                eval_spherical_harmonic(lr as i32, mr as i32, theta, phi)
                            })
                        }
                        _ => None,
                    };
                    res.map(|val| emplace_const!(val))
                } else {
                    None
                }
            }
            VInstruction::MulAdd { a, b, c, .. } => {
                if let (Some(va), Some(vb), Some(vc)) = (
                    get_const_val(*a, &pool),
                    get_const_val(*b, &pool),
                    get_const_val(*c, &pool),
                ) {
                    Some(emplace_const!(va.mul_add(vb, vc)))
                } else {
                    None
                }
            }
            VInstruction::MulSub { a, b, c, .. } => {
                if let (Some(va), Some(vb), Some(vc)) = (
                    get_const_val(*a, &pool),
                    get_const_val(*b, &pool),
                    get_const_val(*c, &pool),
                ) {
                    Some(emplace_const!(va.mul_add(vb, -vc)))
                } else {
                    None
                }
            }
            VInstruction::NegMulAdd { a, b, c, .. } => {
                if let (Some(va), Some(vb), Some(vc)) = (
                    get_const_val(*a, &pool),
                    get_const_val(*b, &pool),
                    get_const_val(*c, &pool),
                ) {
                    Some(emplace_const!(-(va * vb) + vc))
                } else {
                    None
                }
            }

            VInstruction::ExpSqr { src, .. } => {
                get_const_val(*src, &pool).map(|v| emplace_const!((v * v).exp()))
            }
            VInstruction::ExpSqrNeg { src, .. } => {
                get_const_val(*src, &pool).map(|v| emplace_const!((-v * v).exp()))
            }
        };

        if let Some(trivial_val) = replacement {
            alias.insert(instr.dest(), trivial_val);
            continue;
        }

        // 2. Local Value Numbering / Exact Instruction matching
        instr.sort_operands();

        // Modify dest in-place for lookup, avoiding a clone on cache hits.
        // On a miss we clone once for the `seen` map. On a hit (the common
        // case for shared subexpressions) zero heap allocation occurs.
        let real_dest = instr.dest();
        instr.set_dest(VReg::Temp(u32::MAX));

        if let Some(&existing_vreg) = seen.get(&instr) {
            alias.insert(real_dest, existing_vreg);
        } else {
            seen.insert(instr.clone(), real_dest);
            instr.set_dest(real_dest);
            optimized.push(instr);
        }
    }
    *vinstrs = optimized;

    if let Some(f) = final_vreg {
        while let Some(&canonical) = alias.get(f) {
            *f = canonical;
        }
    }

    let (_, final_index) = pool.into_parts();
    *const_map = final_index;
}
