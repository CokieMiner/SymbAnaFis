use super::poly_share::optimize_shared_poly_bases;
use crate::evaluator::logic::instruction::{FnOp, Instruction};

#[test]
fn keeps_separate_base_versions_apart() {
    let instructions = vec![
        Instruction::Builtin1 {
            dest: 1,
            op: FnOp::Sin,
            arg: 0,
        },
        Instruction::PolyEval {
            dest: 2,
            x: 1,
            const_idx: 0,
            degree: 2,
        },
        Instruction::Builtin1 {
            dest: 1,
            op: FnOp::Exp,
            arg: 0,
        },
        Instruction::PolyEval {
            dest: 3,
            x: 1,
            const_idx: 3,
            degree: 2,
        },
    ];
    let constants = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let mut next_reg = 4;

    let out = optimize_shared_poly_bases(instructions, &constants, &mut next_reg);

    assert_eq!(
        out.iter()
            .filter(|instr| matches!(instr, Instruction::PolyEval { .. }))
            .count(),
        2
    );
    assert_eq!(next_reg, 4);
}

#[test]
fn expands_polys_sharing_the_same_live_base() {
    let instructions = vec![
        Instruction::Builtin1 {
            dest: 1,
            op: FnOp::Sin,
            arg: 0,
        },
        Instruction::PolyEval {
            dest: 2,
            x: 1,
            const_idx: 0,
            degree: 2,
        },
        Instruction::PolyEval {
            dest: 3,
            x: 1,
            const_idx: 3,
            degree: 2,
        },
    ];
    let constants = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let mut next_reg = 4;

    let out = optimize_shared_poly_bases(instructions, &constants, &mut next_reg);

    assert!(
        out.iter()
            .any(|instr| matches!(instr, Instruction::Square { src: 1, .. }))
    );
    assert!(
        out.iter()
            .all(|instr| !matches!(instr, Instruction::PolyEval { dest: 2 | 3, .. }))
    );
    assert!(next_reg > 4);
}
