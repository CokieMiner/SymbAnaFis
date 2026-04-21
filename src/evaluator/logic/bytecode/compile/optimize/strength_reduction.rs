use super::Instruction;
use super::helper::ConstantPool;

/// Performs strength reduction on instructions, such as converting `Mul { a, a }` to `Square { a }`,
/// and `Div { x, const }` to `Mul { x, recip }`.
#[allow(
    clippy::float_cmp,
    reason = "Exact floating point comparison is necessary for identifying algebraic identities (e.g. x * 1.0)"
)]
#[allow(
    clippy::too_many_lines,
    reason = "Strength reduction pass covering many instruction variants"
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
                if pool.is_constant(a) && pool.is_constant(b) {
                    let v = pool.get(a) * pool.get(b);
                    let c = pool.get_or_insert(v);
                    *instr = Instruction::Copy { dest, src: c };
                    continue;
                }
                let (c_reg, v_reg) = if pool.is_constant(b) {
                    (b, a)
                } else if pool.is_constant(a) {
                    (a, b)
                } else {
                    continue;
                };

                // x * 1.0 -> Copy x
                // x * -1.0 -> Neg x
                // x * 2.0 -> Add x, x
                let c = pool.get(c_reg);
                if c == 1.0 {
                    *instr = Instruction::Copy { dest, src: v_reg };
                } else if c == -1.0 {
                    *instr = Instruction::Neg { dest, src: v_reg };
                } else if c == 2.0 {
                    *instr = Instruction::Add {
                        dest,
                        a: v_reg,
                        b: v_reg,
                    };
                }
            }
            Instruction::Div { dest, num, den } => {
                if pool.is_constant(num) && pool.is_constant(den) {
                    let v = pool.get(num) / pool.get(den);
                    let c = pool.get_or_insert(v);
                    *instr = Instruction::Copy { dest, src: c };
                    continue;
                }
                // x / const -> x * (1/const)
                if pool.is_constant(den) {
                    let divisor = pool.get(den);
                    if divisor == 1.0 {
                        *instr = Instruction::Copy { dest, src: num };
                    } else if divisor != 0.0 && divisor.is_finite() {
                        let recip = 1.0 / divisor;
                        let recip_reg = pool.get_or_insert(recip);
                        *instr = Instruction::Mul {
                            dest,
                            a: num,
                            b: recip_reg,
                        };
                    }
                } else if pool.is_constant(num) {
                    let c = pool.get(num);
                    if c == 1.0 {
                        *instr = Instruction::Recip { dest, src: den };
                    }
                }
            }
            Instruction::Add { dest, a, b } => {
                if pool.is_constant(a) && pool.is_constant(b) {
                    let v = pool.get(a) + pool.get(b);
                    let c = pool.get_or_insert(v);
                    *instr = Instruction::Copy { dest, src: c };
                    continue;
                }
                let (c_reg, v_reg) = if pool.is_constant(b) {
                    (b, a)
                } else if pool.is_constant(a) {
                    (a, b)
                } else {
                    continue;
                };

                // x + 0.0 -> Copy x
                if pool.get(c_reg) == 0.0 {
                    *instr = Instruction::Copy { dest, src: v_reg };
                }
            }
            Instruction::Sub { dest, a, b } => {
                if pool.is_constant(a) && pool.is_constant(b) {
                    let v = pool.get(a) - pool.get(b);
                    let c = pool.get_or_insert(v);
                    *instr = Instruction::Copy { dest, src: c };
                    continue;
                }
                // x - 0.0 -> Copy x
                // 0.0 - x -> Neg x
                if pool.is_constant(b) && pool.get(b) == 0.0 {
                    *instr = Instruction::Copy { dest, src: a };
                } else if pool.is_constant(a) && pool.get(a) == 0.0 {
                    *instr = Instruction::Neg { dest, src: b };
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
            Instruction::Pow { dest, base, exp }
                if pool.is_constant(base) && pool.is_constant(exp) =>
            {
                let v = pool.get(base).powf(pool.get(exp));
                let c = pool.get_or_insert(v);
                *instr = Instruction::Copy { dest, src: c };
            }
            _ => {}
        }
    }
}
