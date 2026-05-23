use super::Instruction;
use super::helper::ConstantPool;
use rustc_hash::FxHashMap;

/// Performs strength reduction on physical instructions.
///
/// Only handles patterns that the VIR-level GVN pass cannot catch:
/// - `Mul{a, a}` → `Square{a}` (redundant with GVN's `Mul2(a,a)→Square` but kept
///   as defense-in-depth since N-ary product lowering may reintroduce the pattern)
/// - `Mul{x, 2.0}` → `Add{x, x}`
/// - `Powi{n}` → specialized opcodes (`Square`, `Cube`, `Pow4`, `Recip`, `InvSquare`, `InvCube`)
///
/// All other constant folding and identity simplifications (e.g. `x + 0 → x`,
/// `x * 1 → x`, `Div{x, c} → Mul{x, 1/c}`) are already performed by the VIR GVN
/// pass and will never reach the physical instruction stream.
#[allow(
    clippy::float_cmp,
    reason = "Exact floating point comparison is necessary for identifying algebraic identities (e.g. x * 2.0)"
)]
pub(super) fn reduce_strength(instructions: &mut [Instruction], pool: &mut ConstantPool<'_>) {
    for instr in instructions {
        match *instr {
            Instruction::Mul { dest, a, b } if a == b => {
                if pool.is_constant(a) {
                    let v = pool.get(a);
                    let sq = pool.get_or_insert(v * v);
                    *instr = Instruction::Copy { dest, src: sq };
                } else {
                    *instr = Instruction::Square { dest, src: a };
                }
            }
            Instruction::Mul { dest, a, b } => {
                // x * 2.0 → Add(x, x)
                if pool.is_constant(b) && pool.get(b) == 2.0 {
                    *instr = Instruction::Add { dest, a, b: a };
                } else if pool.is_constant(a) && pool.get(a) == 2.0 {
                    *instr = Instruction::Add { dest, a: b, b };
                }
            }
            Instruction::Powi {
                dest,
                src,
                n: pow_n,
            } => match pow_n {
                2 => *instr = Instruction::Square { dest, src },
                3 => *instr = Instruction::Cube { dest, src },
                4 => *instr = Instruction::Pow4 { dest, src },
                -1 => *instr = Instruction::Recip { dest, src },
                -2 => *instr = Instruction::InvSquare { dest, src },
                -3 => *instr = Instruction::InvCube { dest, src },
                _ => {}
            },
            Instruction::End {}
            | Instruction::SinCos { .. }
            | Instruction::AsinAcos { .. }
            | Instruction::Add { .. }
            | Instruction::Sub { .. }
            | Instruction::Div { .. }
            | Instruction::Pow { .. }
            | Instruction::Neg { .. }
            | Instruction::Copy { .. }
            | Instruction::Sin { .. }
            | Instruction::Cos { .. }
            | Instruction::Exp { .. }
            | Instruction::Ln { .. }
            | Instruction::Sqrt { .. }
            | Instruction::Square { .. }
            | Instruction::Cube { .. }
            | Instruction::Pow4 { .. }
            | Instruction::Pow3_2 { .. }
            | Instruction::InvPow3_2 { .. }
            | Instruction::InvSqrt { .. }
            | Instruction::Recip { .. }
            | Instruction::InvSquare { .. }
            | Instruction::InvCube { .. }
            | Instruction::ExpSqr { .. }
            | Instruction::ExpSqrNeg { .. }
            | Instruction::RecipExpm1 { .. }
            | Instruction::Add3 { .. }
            | Instruction::Add4 { .. }
            | Instruction::AddN { .. }
            | Instruction::Mul3 { .. }
            | Instruction::Mul4 { .. }
            | Instruction::MulN { .. }
            | Instruction::Builtin1 { .. }
            | Instruction::Builtin2 { .. }
            | Instruction::Builtin3 { .. }
            | Instruction::Builtin4 { .. }
            | Instruction::MulAdd { .. }
            | Instruction::MulSub { .. }
            | Instruction::NegMul { .. }
            | Instruction::NegMulAdd { .. }
            | Instruction::NegMulSub { .. } => {}
        }
    }
}

// ── Power-chain optimization ──────────────────────────────────────────────

/// Optimization pass that rewires independent power instructions sharing the same base into chains.
///
/// For example:
/// ```text
///   t1 = Square(R0)     // x^2
///   t2 = Cube(R0)       // x^3
/// ```
/// is transformed into:
/// ```text
///   t1 = Square(R0)     // x^2
///   t2 = Mul(t1, R0)    // x^3 = x^2 * x
/// ```
///
/// # Key invariants
///
/// ## `available_by_base` map
///
/// Contains `base_register → [(exponent, dest_register)]` entries that satisfy:
///
/// 1. **Destination still valid**: The `dest_register` has NOT been overwritten by
///    any instruction after the power was computed. If it has, the power is stale.
/// 2. **Base still valid**: The `base_register` has NOT been redefined since the
///    power was computed. If the base is overwritten, every power rooted at that
///    base refers to an old value and must be evicted.
/// 3. **Linear order**: Powers are only available *after* their defining instruction.
///    We never reorder instructions — only rewrite the RHS of a power to use an
///    already-computed result from an earlier instruction.
///
/// ## `kill_written_reg` function
///
/// Called whenever a register is defined (written to). It evicts:
///
/// - All powers whose **base** equals the written register (the base value changed).
/// - All powers whose **destination** equals the written register (the cached result
///   is now stale because the register holds a different value).
///
/// ## `dest != base` guard
///
/// After recording a power, we skip the entry if `dest == base`. A self-referencing
/// entry would create a cycle: the power instruction rewrites itself to use its own
/// result before it has been computed.
pub(super) fn optimize_power_chains(instructions: &mut [Instruction]) {
    // Only use powers that have already been computed earlier in the instruction stream.
    // Reordering by exponent can create use-before-def bugs when, for example, `x^4`
    // is emitted before `x^2` and then rewritten to depend on that later `x^2`.
    let mut available_by_base: FxHashMap<u32, Vec<(i32, u32)>> = FxHashMap::default();
    let mut dest_to_base: FxHashMap<u32, u32> = FxHashMap::default();

    for instr in instructions.iter_mut() {
        let power_info = get_power_info(instr, &available_by_base, &dest_to_base);

        if let Some((base, exp, dest)) = power_info {
            if let Some(replacement) = available_by_base
                .get(&base)
                .and_then(|available| find_cheap_combo(base, exp, dest, available))
            {
                *instr = replacement;
            }

            kill_written_reg(&mut available_by_base, &mut dest_to_base, dest);
            if dest != base {
                available_by_base.entry(base).or_default().push((exp, dest));
                dest_to_base.insert(dest, base);
            }
        } else {
            instr.for_each_write(|dest| {
                kill_written_reg(&mut available_by_base, &mut dest_to_base, dest);
            });
        }
    }
}

/// Evicts cached powers invalidated by `written_reg` being overwritten.
///
/// Two cases require eviction:
///
/// 1. **Base overwritten**: If `written_reg` was used as the base of cached powers,
///    all those powers now refer to an old value. E.g. if `R0 = x` and we cached
///    `(R0, [(2, R5)])` meaning `R5 = x^2`, then `R0 = y` invalidates `R5 = x^2`.
///
/// 2. **Destination overwritten**: If `written_reg` was the destination of a cached
///    power (e.g. `R5`), that register no longer holds the power value.
fn kill_written_reg(
    available_by_base: &mut FxHashMap<u32, Vec<(i32, u32)>>,
    dest_to_base: &mut FxHashMap<u32, u32>,
    written_reg: u32,
) {
    // Fast path: most temps are not tracked
    if !dest_to_base.contains_key(&written_reg) && !available_by_base.contains_key(&written_reg) {
        return;
    }

    // If the base register itself is overwritten, every cached power rooted at that base
    // becomes stale because future instructions see a different value in that register.
    if let Some(removed) = available_by_base.remove(&written_reg) {
        for (_, reg) in removed {
            dest_to_base.remove(&reg);
        }
    }

    if let Some(base) = dest_to_base.remove(&written_reg)
        && let Some(cached) = available_by_base.get_mut(&base)
    {
        cached.retain(|&(_, reg)| reg != written_reg);
        if cached.is_empty() {
            available_by_base.remove(&base);
        }
    }
}

fn find_cheap_combo(
    base: u32,
    target_exp: i32,
    dest: u32,
    available: &[(i32, u32)],
) -> Option<Instruction> {
    // x^3 from x^2: Mul(sq_reg, base)
    if target_exp == 3 {
        return available
            .iter()
            .find(|&&(e, _)| e == 2)
            .map(|&(_, sq)| Instruction::Mul {
                dest,
                a: sq,
                b: base,
            });
    }
    // x^4 from x^2: Square(sq_reg)
    // x^4 from x^3: Mul(cube_reg, base)
    if target_exp == 4 {
        if let Some(&(_, sq)) = available.iter().find(|&&(e, _)| e == 2) {
            return Some(Instruction::Square { dest, src: sq });
        }
        if let Some(&(_, cu)) = available.iter().find(|&&(e, _)| e == 3) {
            return Some(Instruction::Mul {
                dest,
                a: cu,
                b: base,
            });
        }
    }
    // x^5 from x^4: Mul(pow4_reg, base)
    if target_exp == 5 {
        if let Some(&(_, p4)) = available.iter().find(|&&(e, _)| e == 4) {
            return Some(Instruction::Mul {
                dest,
                a: p4,
                b: base,
            });
        }
        // or x^3 * x^2
        let cu = available.iter().find(|&&(e, _)| e == 3).map(|&(_, r)| r);
        let sq = available.iter().find(|&&(e, _)| e == 2).map(|&(_, r)| r);
        if let (Some(cu), Some(sq)) = (cu, sq) {
            return Some(Instruction::Mul { dest, a: cu, b: sq });
        }
    }

    // General: target = a + b where both a and b are available
    // Only positive exponents for now to keep it simple and safe.
    if target_exp > 1 {
        for &(ea, ra) in available {
            // target = a * 2 (squaring an available power)
            if ea * 2 == target_exp {
                return Some(Instruction::Square { dest, src: ra });
            }

            let eb = target_exp - ea;
            if eb <= 0 || eb == ea {
                continue;
            }
            if let Some(&(_, rb)) = available.iter().find(|&&(e, _)| e == eb) {
                return Some(Instruction::Mul { dest, a: ra, b: rb });
            }
        }
    }

    if target_exp < -1 {
        for &(ea, ra) in available {
            if ea >= 0 {
                continue;
            }
            // target = a * 2 (squaring an available negative power, e.g. x^-4 from x^-2)
            if ea * 2 == target_exp {
                return Some(Instruction::Square { dest, src: ra });
            }

            let eb = target_exp - ea;
            if eb >= 0 || eb == ea {
                continue;
            }
            if let Some(&(_, rb)) = available.iter().find(|&&(e, _)| e == eb) {
                return Some(Instruction::Mul { dest, a: ra, b: rb });
            }
        }
    }

    None
}

const fn instruction_multiplier(instr: &Instruction) -> i32 {
    match *instr {
        Instruction::Square { .. } => 2,
        Instruction::Cube { .. } => 3,
        Instruction::Pow4 { .. } => 4,
        Instruction::Powi { n, .. } => n,
        Instruction::Recip { .. } => -1,
        Instruction::InvSquare { .. } => -2,
        Instruction::InvCube { .. } => -3,
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => 1,
    }
}

fn get_power_info(
    instr: &Instruction,
    available_by_base: &FxHashMap<u32, Vec<(i32, u32)>>,
    dest_to_base: &FxHashMap<u32, u32>,
) -> Option<(u32, i32, u32)> {
    match *instr {
        Instruction::Square { src, dest }
        | Instruction::Cube { src, dest }
        | Instruction::Pow4 { src, dest }
        | Instruction::Powi { src, dest, .. }
        | Instruction::Recip { src, dest }
        | Instruction::InvSquare { src, dest }
        | Instruction::InvCube { src, dest } => {
            let base_src = dest_to_base.get(&src).copied();
            let exp_src = base_src.map_or(1, |b| {
                available_by_base
                    .get(&b)
                    .and_then(|v| v.iter().find(|&&(_, r)| r == src))
                    .map_or(1, |&(e, _)| e)
            });

            Some((
                base_src.unwrap_or(src),
                exp_src.saturating_mul(instruction_multiplier(instr)),
                dest,
            ))
        }
        Instruction::Mul { a, b, dest } => {
            let base_a = dest_to_base.get(&a).copied();
            let base_b = dest_to_base.get(&b).copied();

            let exp_a = base_a.map_or(1, |base_reg| {
                available_by_base
                    .get(&base_reg)
                    .and_then(|v| v.iter().find(|&&(_, r)| r == a))
                    .map_or(1, |&(e, _)| e)
            });
            let exp_b = base_b.map_or(1, |base_reg| {
                available_by_base
                    .get(&base_reg)
                    .and_then(|v| v.iter().find(|&&(_, r)| r == b))
                    .map_or(1, |&(e, _)| e)
            });

            let effective_base_a = base_a.unwrap_or(a);
            let effective_base_b = base_b.unwrap_or(b);

            (effective_base_a == effective_base_b).then(|| (effective_base_a, exp_a + exp_b, dest))
        }
        Instruction::End { .. }
        | Instruction::Copy { .. }
        | Instruction::Neg { .. }
        | Instruction::SinCos { .. }
        | Instruction::AsinAcos { .. }
        | Instruction::Add { .. }
        | Instruction::Add3 { .. }
        | Instruction::Add4 { .. }
        | Instruction::AddN { .. }
        | Instruction::Mul3 { .. }
        | Instruction::Mul4 { .. }
        | Instruction::MulN { .. }
        | Instruction::Sub { .. }
        | Instruction::Div { .. }
        | Instruction::Pow { .. }
        | Instruction::MulAdd { .. }
        | Instruction::MulSub { .. }
        | Instruction::NegMul { .. }
        | Instruction::NegMulAdd { .. }
        | Instruction::NegMulSub { .. }
        | Instruction::Pow3_2 { .. }
        | Instruction::InvPow3_2 { .. }
        | Instruction::InvSqrt { .. }
        | Instruction::Sin { .. }
        | Instruction::Cos { .. }
        | Instruction::Exp { .. }
        | Instruction::Ln { .. }
        | Instruction::Sqrt { .. }
        | Instruction::RecipExpm1 { .. }
        | Instruction::ExpSqr { .. }
        | Instruction::ExpSqrNeg { .. }
        | Instruction::Builtin1 { .. }
        | Instruction::Builtin2 { .. }
        | Instruction::Builtin3 { .. }
        | Instruction::Builtin4 { .. } => None,
    }
}
