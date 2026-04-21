use super::Instruction;
use rustc_hash::FxHashMap;

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
        let (base, exp, dest) = match *instr {
            Instruction::Square { src, dest } => (src, 2_i32, dest),
            Instruction::Cube { src, dest } => (src, 3, dest),
            Instruction::Pow4 { src, dest } => (src, 4, dest),
            Instruction::Powi { src, n, dest } => (src, n, dest),
            Instruction::Recip { src, dest } => (src, -1, dest),
            Instruction::InvSquare { src, dest } => (src, -2, dest),
            Instruction::InvCube { src, dest } => (src, -3, dest),
            _ => {
                kill_written_reg(&mut available_by_base, &mut dest_to_base, instr.dest_reg());
                continue;
            }
        };

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
