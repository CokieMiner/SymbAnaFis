use super::helper::ConstantPool;
use crate::evaluator::logic::instruction::Instruction;

/// Performs strength reduction on instructions, such as converting `Mul { a, a }` to `Square { a }`,
/// and `DivConst` to `MulConst` by computing the reciprocal.
///
/// This pass may add new constants to the pool (e.g. reciprocals).
#[allow(
    clippy::float_cmp,
    reason = "The pass matches exact identity constants like 0.0, 1.0, and -1.0."
)]
pub(super) fn reduce_strength(instructions: &mut [Instruction], pool: &mut ConstantPool<'_>) {
    for instr in instructions {
        match *instr {
            Instruction::Mul { dest, a, b } if a == b => {
                *instr = Instruction::Square { dest, src: a };
            }
            Instruction::DivConst {
                dest,
                src,
                const_idx,
            } => {
                let divisor = pool.get(const_idx);
                if divisor == 1.0 {
                    *instr = Instruction::Copy { dest, src };
                } else if divisor != 0.0 && divisor.is_finite() {
                    let recip = 1.0 / divisor;
                    let new_idx = pool.get_or_insert(recip);

                    *instr = Instruction::MulConst {
                        dest,
                        src,
                        const_idx: new_idx,
                    };
                }
            }
            Instruction::ConstDiv {
                dest,
                src,
                const_idx,
            } => {
                let c_val = pool.get(const_idx);
                if c_val == 1.0 {
                    *instr = Instruction::Recip { dest, src };
                }
            }
            Instruction::AddConst {
                dest,
                src,
                const_idx,
            } => {
                let c_val = pool.get(const_idx);
                if c_val == 0.0 {
                    *instr = Instruction::Copy { dest, src };
                }
            }
            Instruction::MulConst {
                dest,
                src,
                const_idx,
            } => {
                let c_val = pool.get(const_idx);
                if c_val == 1.0 {
                    *instr = Instruction::Copy { dest, src };
                } else if c_val == -1.0 {
                    *instr = Instruction::Neg { dest, src };
                }
            }
            Instruction::SubConst {
                dest,
                src,
                const_idx,
            } => {
                let c_val = pool.get(const_idx);
                if c_val == 0.0 {
                    *instr = Instruction::Copy { dest, src };
                } else {
                    let neg_c = -c_val;
                    let new_idx = pool.get_or_insert(neg_c);
                    *instr = Instruction::AddConst {
                        dest,
                        src,
                        const_idx: new_idx,
                    };
                }
            }
            Instruction::ConstSub {
                dest,
                src,
                const_idx,
            } => {
                let c_val = pool.get(const_idx);
                if c_val == 0.0 {
                    *instr = Instruction::Neg { dest, src };
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
            _ => {}
        }
    }
}
