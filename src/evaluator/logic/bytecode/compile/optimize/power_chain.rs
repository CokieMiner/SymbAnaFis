use crate::evaluator::logic::bytecode::instruction::Instruction;
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
pub(super) fn optimize_power_chains(instructions: &mut [Instruction]) {
    // Only use powers that have already been computed earlier in the instruction stream.
    // Reordering by exponent can create use-before-def bugs when, for example, `x^4`
    // is emitted before `x^2` and then rewritten to depend on that later `x^2`.
    let mut available_by_base: FxHashMap<u32, Vec<(i32, u32)>> = FxHashMap::default();

    for instr in instructions.iter_mut() {
        let (base, exp, dest) = match *instr {
            Instruction::Square { src, dest } => (src, 2_i32, dest),
            Instruction::Cube { src, dest } => (src, 3, dest),
            Instruction::Pow4 { src, dest } => (src, 4, dest),
            Instruction::Powi { src, n, dest } => (src, n, dest),
            Instruction::InvSquare { src, dest } => (src, -2, dest),
            Instruction::InvCube { src, dest } => (src, -3, dest),
            _ => {
                kill_written_reg(&mut available_by_base, instr.dest_reg());
                continue;
            }
        };

        if let Some(replacement) = available_by_base
            .get(&base)
            .and_then(|available| find_cheap_combo(base, exp, dest, available))
        {
            *instr = replacement;
        }
        kill_written_reg(&mut available_by_base, dest);
        if dest != base {
            available_by_base.entry(base).or_default().push((exp, dest));
        }
    }
}

fn kill_written_reg(available_by_base: &mut FxHashMap<u32, Vec<(i32, u32)>>, written_reg: u32) {
    // If the base register itself is overwritten, every cached power rooted at that base
    // becomes stale because future instructions see a different value in that register.
    available_by_base.remove(&written_reg);
    available_by_base.retain(|_, cached| {
        cached.retain(|&(_, reg)| reg != written_reg);
        !cached.is_empty()
    });
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
            let eb = target_exp - ea;
            if eb <= 0 || eb == ea {
                continue;
            }
            if let Some(&(_, rb)) = available.iter().find(|&&(e, _)| e == eb) {
                return Some(Instruction::Mul { dest, a: ra, b: rb });
            }
            // target = a * 2 (squaring an available power)
            if ea * 2 == target_exp {
                return Some(Instruction::Square { dest, src: ra });
            }
        }
    }

    None
}
