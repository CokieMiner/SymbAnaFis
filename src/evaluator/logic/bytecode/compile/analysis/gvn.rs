use std::hash::{Hash, Hasher};
use std::mem::take;
use std::ptr::from_ref;

use rustc_hash::FxHashMap;

use super::ConstantPool;
use super::vir::{VInstruction, VReg};
use crate::core::Expr;

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
                    // However, for pure symbolic expressions in scientific fitting,
                    // we often assume variables are finite. We'll be conservative here.
                    if srcs.is_empty() {
                        Some(emplace_const!(if prod.is_nan() { f64::NAN } else { 0.0 }))
                    } else if prod.is_nan() {
                        Some(emplace_const!(f64::NAN))
                    } else {
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
                } else if is_zero(*a, &pool) {
                    get_const_val(*b, &pool).map(|vb| emplace_const!(0.0 * vb))
                } else if is_zero(*b, &pool) {
                    get_const_val(*a, &pool).map(|va| emplace_const!(va * 0.0))
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
                } else if *a == *b {
                    // Mul2{x, x} → Square{x}
                    instr = VInstruction::Square {
                        dest: *dest,
                        src: *a,
                    };
                    None
                } else {
                    None
                }
            }
            VInstruction::Sub { a, b, dest } => {
                if is_zero(*a, &pool) {
                    // 0 - x -> Neg(x)
                    instr = VInstruction::Neg {
                        dest: *dest,
                        src: *b,
                    };
                    None
                } else if is_zero(*b, &pool) {
                    Some(*a)
                } else if let (Some(va), Some(vb)) =
                    (get_const_val(*a, &pool), get_const_val(*b, &pool))
                {
                    Some(emplace_const!(va - vb))
                } else {
                    None
                }
            }
            VInstruction::Div { num, den, dest } => {
                if let (Some(vnum), Some(vden)) =
                    (get_const_val(*num, &pool), get_const_val(*den, &pool))
                {
                    Some(emplace_const!(vnum / vden))
                } else if is_one(*num, &pool) {
                    // 1 / x → Recip(x)
                    instr = VInstruction::Recip {
                        dest: *dest,
                        src: *den,
                    };
                    None
                } else if is_one(*den, &pool) {
                    Some(*num)
                } else if let Some(vden) = get_const_val(*den, &pool) {
                    if vden != 0.0 && vden.is_finite() {
                        let recip = 1.0 / vden;
                        let recip_vreg = emplace_const!(recip);
                        instr = VInstruction::Mul2 {
                            dest: *dest,
                            a: *num,
                            b: recip_vreg,
                        };
                    }
                    None
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
                    get_const_val(*exp, &pool).map(|vexp| {
                        if vexp > 0.0 {
                            emplace_const!(0.0)
                        } else {
                            emplace_const!(f64::INFINITY)
                        }
                    })
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
            VInstruction::NegMul { a, b, .. } => {
                if let (Some(va), Some(vb)) = (get_const_val(*a, &pool), get_const_val(*b, &pool)) {
                    Some(emplace_const!(-(va * vb)))
                } else {
                    None
                }
            }
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
                get_const_val(*arg, &pool).and_then(|v| op.fold1(v).map(|val| emplace_const!(val)))
            }
            VInstruction::Builtin2 { op, arg1, arg2, .. } => {
                if let (Some(v1), Some(v2)) =
                    (get_const_val(*arg1, &pool), get_const_val(*arg2, &pool))
                {
                    op.fold2(v1, v2).map(|val| emplace_const!(val))
                } else {
                    None
                }
            }
            VInstruction::BuiltinFun { op, args, .. } => {
                let mut c_args = Vec::with_capacity(args.len());
                for a in args.iter() {
                    if let Some(c) = get_const_val(*a, &pool) {
                        c_args.push(c);
                    } else {
                        break;
                    }
                }

                if c_args.len() == args.len() {
                    op.fold_n(&c_args).map(|val| emplace_const!(val))
                } else {
                    None
                }
            }
            VInstruction::MulAdd { a, b, c, .. } => {
                if is_zero(*a, &pool) || is_zero(*b, &pool) {
                    Some(*c)
                } else if let (Some(va), Some(vb), Some(vc)) = (
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
                if is_zero(*a, &pool) || is_zero(*b, &pool) {
                    Some(emplace_const!(-get_const_val(*c, &pool).unwrap_or(0.0)))
                } else if let (Some(va), Some(vb), Some(vc)) = (
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
                if is_zero(*a, &pool) || is_zero(*b, &pool) {
                    Some(*c)
                } else if let (Some(va), Some(vb), Some(vc)) = (
                    get_const_val(*a, &pool),
                    get_const_val(*b, &pool),
                    get_const_val(*c, &pool),
                ) {
                    Some(emplace_const!(-(va * vb) + vc))
                } else {
                    None
                }
            }
            VInstruction::NegMulSub { a, b, c, .. } => {
                if is_zero(*a, &pool) || is_zero(*b, &pool) {
                    Some(emplace_const!(-get_const_val(*c, &pool).unwrap_or(0.0)))
                } else if let (Some(va), Some(vb), Some(vc)) = (
                    get_const_val(*a, &pool),
                    get_const_val(*b, &pool),
                    get_const_val(*c, &pool),
                ) {
                    Some(emplace_const!(-(va * vb) - vc))
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
