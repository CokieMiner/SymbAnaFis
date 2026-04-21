use super::Instruction;
use super::vir::FnOp;

/// Internal assembler: translates the rich IR `Instruction` enum into a dense
/// flat array of `u32` for L1-cache optimized execution.
#[allow(
    clippy::cast_sign_loss,
    reason = "Two's complement bit pattern for i32 is safely preserved in u32 flat bytecode"
)]
pub fn assemble_flat_bytecode(instructions: &[Instruction]) -> Vec<u32> {
    let mut bc = Vec::with_capacity(instructions.len() * 4 + 1);
    for instr in instructions {
        match *instr {
            Instruction::Copy { dest, src } => bc.extend_from_slice(&[1, dest, src]),
            Instruction::Add { dest, a, b } => bc.extend_from_slice(&[4, dest, a, b]),
            Instruction::Add3 { dest, a, b, c } => bc.extend_from_slice(&[5, dest, a, b, c]),
            Instruction::Add4 { dest, a, b, c, d } => {
                bc.extend_from_slice(&[6, dest, a, b, c, d]);
            }
            Instruction::Mul { dest, a, b } => bc.extend_from_slice(&[8, dest, a, b]),
            Instruction::Mul3 { dest, a, b, c } => bc.extend_from_slice(&[9, dest, a, b, c]),
            Instruction::Mul4 { dest, a, b, c, d } => {
                bc.extend_from_slice(&[10, dest, a, b, c, d]);
            }
            Instruction::AddN {
                dest,
                start_idx,
                count,
            } => bc.extend_from_slice(&[7, dest, start_idx, count]),
            Instruction::MulN {
                dest,
                start_idx,
                count,
            } => bc.extend_from_slice(&[11, dest, start_idx, count]),
            Instruction::Sub { dest, a, b } => bc.extend_from_slice(&[12, dest, a, b]),
            Instruction::Div { dest, num, den } => bc.extend_from_slice(&[13, dest, num, den]),
            Instruction::Pow { dest, base, exp } => bc.extend_from_slice(&[14, dest, base, exp]),
            Instruction::Neg { dest, src } => bc.extend_from_slice(&[2, dest, src]),
            Instruction::Builtin1 { dest, op, arg } => match op {
                FnOp::Sin => bc.extend_from_slice(&[30, dest, arg]),
                FnOp::Cos => bc.extend_from_slice(&[31, dest, arg]),
                FnOp::Exp => bc.extend_from_slice(&[32, dest, arg]),
                FnOp::Ln => bc.extend_from_slice(&[33, dest, arg]),
                FnOp::Sqrt => bc.extend_from_slice(&[34, dest, arg]),
                _ => {
                    let op_u32 = op as u32;
                    bc.extend_from_slice(&[38, dest, op_u32, arg]);
                }
            },
            Instruction::Builtin2 {
                dest,
                op,
                arg1,
                arg2,
            } => {
                let op_u32 = op as u32;
                bc.extend_from_slice(&[39, dest, op_u32, arg1, arg2]);
            }
            Instruction::Builtin3 {
                dest,
                op,
                start_idx,
            } => {
                let op_u32 = op as u32;
                bc.extend_from_slice(&[40, dest, op_u32, start_idx]);
            }
            Instruction::Builtin4 {
                dest,
                op,
                start_idx,
            } => {
                let op_u32 = op as u32;
                bc.extend_from_slice(&[41, dest, op_u32, start_idx]);
            }
            Instruction::Square { dest, src } => bc.extend_from_slice(&[20, dest, src]),
            Instruction::Cube { dest, src } => bc.extend_from_slice(&[21, dest, src]),
            Instruction::Pow4 { dest, src } => bc.extend_from_slice(&[22, dest, src]),
            Instruction::Pow3_2 { dest, src } => bc.extend_from_slice(&[23, dest, src]),
            Instruction::InvPow3_2 { dest, src } => bc.extend_from_slice(&[24, dest, src]),
            Instruction::InvSqrt { dest, src } => bc.extend_from_slice(&[25, dest, src]),
            Instruction::InvSquare { dest, src } => bc.extend_from_slice(&[26, dest, src]),
            Instruction::InvCube { dest, src } => bc.extend_from_slice(&[27, dest, src]),
            Instruction::Recip { dest, src } => bc.extend_from_slice(&[28, dest, src]),
            Instruction::Powi { dest, src, n } => bc.extend_from_slice(&[29, dest, src, n as u32]),
            Instruction::MulAdd { dest, a, b, c } => bc.extend_from_slice(&[15, dest, a, b, c]),
            Instruction::MulSub { dest, a, b, c } => bc.extend_from_slice(&[16, dest, a, b, c]),
            Instruction::NegMul { dest, a, b } => bc.extend_from_slice(&[17, dest, a, b]),
            Instruction::NegMulAdd { dest, a, b, c } => bc.extend_from_slice(&[18, dest, a, b, c]),
            Instruction::NegMulSub { dest, a, b, c } => bc.extend_from_slice(&[19, dest, a, b, c]),
            Instruction::RecipExpm1 { dest, src } => bc.extend_from_slice(&[35, dest, src]),
            Instruction::ExpSqr { dest, src } => bc.extend_from_slice(&[36, dest, src]),
            Instruction::ExpSqrNeg { dest, src } => bc.extend_from_slice(&[37, dest, src]),
            Instruction::SinCos {
                sin_dest,
                cos_dest,
                arg,
            } => bc.extend_from_slice(&[3, sin_dest, cos_dest, arg]),
        }
    }
    bc.push(0); // End opcode to terminate execution loop without pointer length checks
    bc
}
