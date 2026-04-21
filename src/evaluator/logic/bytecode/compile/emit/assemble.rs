use super::Instruction;

/// Internal assembler: translates the rich IR `Instruction` enum into a dense
/// flat array of `u32` for L1-cache optimized execution.
#[allow(
    clippy::cast_sign_loss,
    reason = "Two's complement bit pattern for i32 is safely preserved in u32 flat bytecode"
)]
pub fn assemble_flat_bytecode(instructions: &[Instruction]) -> Vec<u32> {
    let mut bc = Vec::with_capacity(instructions.len() * 4 + 1);
    for instr in instructions {
        let op = instr.opcode();
        match *instr {
            Instruction::End {} => bc.push(op),
            Instruction::Copy { dest, src }
            | Instruction::Neg { dest, src }
            | Instruction::Square { dest, src }
            | Instruction::Cube { dest, src }
            | Instruction::Pow4 { dest, src }
            | Instruction::Pow3_2 { dest, src }
            | Instruction::InvPow3_2 { dest, src }
            | Instruction::InvSqrt { dest, src }
            | Instruction::InvSquare { dest, src }
            | Instruction::InvCube { dest, src }
            | Instruction::Recip { dest, src }
            | Instruction::RecipExpm1 { dest, src }
            | Instruction::ExpSqr { dest, src }
            | Instruction::ExpSqrNeg { dest, src } => bc.extend_from_slice(&[op, dest, src]),

            Instruction::Sin { dest, arg }
            | Instruction::Cos { dest, arg }
            | Instruction::Exp { dest, arg }
            | Instruction::Ln { dest, arg }
            | Instruction::Sqrt { dest, arg } => bc.extend_from_slice(&[op, dest, arg]),

            Instruction::Add { dest, a, b }
            | Instruction::Mul { dest, a, b }
            | Instruction::Sub { dest, a, b }
            | Instruction::NegMul { dest, a, b } => bc.extend_from_slice(&[op, dest, a, b]),

            Instruction::Add3 { dest, a, b, c }
            | Instruction::Mul3 { dest, a, b, c }
            | Instruction::MulAdd { dest, a, b, c }
            | Instruction::MulSub { dest, a, b, c }
            | Instruction::NegMulAdd { dest, a, b, c }
            | Instruction::NegMulSub { dest, a, b, c } => {
                bc.extend_from_slice(&[op, dest, a, b, c]);
            }

            Instruction::Add4 { dest, a, b, c, d } | Instruction::Mul4 { dest, a, b, c, d } => {
                bc.extend_from_slice(&[op, dest, a, b, c, d]);
            }

            Instruction::AddN {
                dest,
                start_idx,
                count,
            }
            | Instruction::MulN {
                dest,
                start_idx,
                count,
            } => bc.extend_from_slice(&[op, dest, start_idx, count]),

            Instruction::SinCos {
                sin_dest,
                cos_dest,
                arg,
            } => bc.extend_from_slice(&[op, sin_dest, cos_dest, arg]),
            Instruction::Div { dest, num, den } => bc.extend_from_slice(&[op, dest, num, den]),
            Instruction::Pow { dest, base, exp } => bc.extend_from_slice(&[op, dest, base, exp]),
            Instruction::Powi { dest, src, n } => bc.extend_from_slice(&[op, dest, src, n as u32]),
            Instruction::Builtin1 {
                dest,
                op: func_op,
                arg,
            } => bc.extend_from_slice(&[op, dest, func_op as u32, arg]),
            Instruction::Builtin2 {
                dest,
                op: func_op,
                arg1,
                arg2,
            } => bc.extend_from_slice(&[op, dest, func_op as u32, arg1, arg2]),
            Instruction::Builtin3 {
                dest,
                op: func_op,
                arg1,
                arg2,
                arg3,
            } => bc.extend_from_slice(&[op, dest, func_op as u32, arg1, arg2, arg3]),
            Instruction::Builtin4 {
                dest,
                op: func_op,
                arg1,
                arg2,
                arg3,
                arg4,
            } => bc.extend_from_slice(&[op, dest, func_op as u32, arg1, arg2, arg3, arg4]),
        }
    }
    bc.push(0); // End opcode to terminate execution loop without pointer length checks
    bc
}
