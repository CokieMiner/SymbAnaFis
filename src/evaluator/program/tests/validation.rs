//! Independent register validation and generated SSA allocation checks.

use alloc::{sync::Arc, vec, vec::Vec};

use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::core::BuiltinFunction;

use super::super::{Evaluator, Instruction, Operation, Program, Source};

proptest! {
    #[test]
    #[expect(clippy::indexing_slicing, reason = "the generator reduces each operand modulo the nonempty completed SSA value table")]
    #[expect(clippy::arithmetic_side_effects, reason = "SSA tables start with two constants and only grow; modulo divisors are nonzero and floating arithmetic has IEEE semantics")]
    fn register_allocation_preserves_generated_acyclic_programs(
        first in -0.5_f64..=0.5,
        second in -0.5_f64..=0.5,
        operations in proptest::collection::vec((0_u16..u16::MAX, 0_u16..u16::MAX, 0_u8..5), 1..150),
    ) {
        let mut values = vec![first, second];
        let mut constants = values.clone();
        let mut sources = vec![Source::Constant(0), Source::Constant(1)];
        let mut program = Program { instructions: Vec::new(), operands: Vec::new(), parameters: Vec::new(), result: 0 };
        for (left, right, kind) in operations {
            let a = usize::from(left) % values.len();
            let b = usize::from(right) % values.len();
            let (operation, expected) = match kind {
                0 => (Operation::Add([a, b]), values[a] + values[b]),
                1 => (Operation::Multiply([a, b]), values[a] * values[b]),
                2 => (Operation::Divide([a, b]), values[a] / values[b]),
                3 => {
                    let start = program.operands.len();
                    program.operands.extend([a, b, a]);
                    (Operation::AddMany(start..program.operands.len()), (values[a] + values[b]) + values[a])
                }
                _ => {
                    let start = program.operands.len();
                    program.operands.extend([a, b, b]);
                    (Operation::MultiplyMany(start..program.operands.len()), (values[a] * values[b]) * values[b])
                }
            };
            program.result = sources.len();
            sources.push(Source::Computed(operation));
            values.push(expected);
        }
        let expected = *values.last().expect("two constants and at least one instruction");
        let width = program.allocate_registers(&sources, &mut constants);
        prop_assert!(program.validate(width, constants.len()));
        constants.resize(width, 0.0);
        let mut evaluator = Evaluator { program: Arc::new(program), registers: constants };
        let actual = evaluator.evaluate(&[]).expect("constant program");
        if expected.is_nan() {
            prop_assert!(actual.is_nan());
        } else {
            prop_assert_eq!(actual.to_bits(), expected.to_bits());
        }
    }
}

#[test]
fn validator_rejects_uninitialized_reads_invalid_ranges_and_prefix_writes() {
    let mut program = Program::<f64> {
        instructions: vec![Instruction {
            output: 2,
            operation: Operation::Add([0, 1]),
        }],
        operands: vec![0, 1, 0],
        parameters: Vec::new(),
        result: 2,
    };
    assert!(program.validate(3, 2));
    assert!(!program.validate(3, 4));
    assert!(!program.validate(2, 2));
    for operation in [
        Operation::Add([0, 3]),
        Operation::Multiply([2, 0]),
        Operation::Divide([3, 0]),
        Operation::Power([0, 3]),
        Operation::Fused([0, 1, 2]),
        Operation::Unary(BuiltinFunction::Abs, |value: &f64| value.abs(), [3]),
        Operation::Binary(BuiltinFunction::Beta, |left, right| left + right, [0, 2]),
        Operation::Ternary(|a, b, c| a + b + c, [0, 1, 3]),
        Operation::Quaternary(|a, b, c, d| a + b + c + d, [0, 1, 0, 3]),
        Operation::AddMany(0..0),
        Operation::AddMany(0..2),
        Operation::MultiplyMany(1..4),
    ] {
        program.instructions = vec![Instruction {
            output: 2,
            operation,
        }];
        assert!(!program.validate(3, 2));
    }
    program.instructions = vec![Instruction {
        output: 0,
        operation: Operation::Add([0, 1]),
    }];
    assert!(!program.validate(3, 2));
    program.instructions.clear();
    assert!(!program.validate(3, 2));
    program.result = 0;
    assert!(program.validate(3, 2));
    program.result = 3;
    assert!(!program.validate(3, 2));
}

#[test]
fn pressure_scheduling_finishes_the_larger_subtree_before_a_simple_sibling() {
    let mut sources = vec![Source::Constant(0), Source::Constant(1)];
    let simple = sources.len();
    sources.push(Source::Computed(Operation::Divide([0, 1])));
    let mut frontier: Vec<_> = (0..64)
        .map(|_| {
            let index = sources.len();
            sources.push(Source::Computed(Operation::Multiply([0, 1])));
            index
        })
        .collect();
    while frontier.len() > 1 {
        frontier = frontier
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[left, right]| {
                let index = sources.len();
                sources.push(Source::Computed(Operation::Add([*left, *right])));
                index
            })
            .collect();
    }
    let result = sources.len();
    let high = *frontier
        .first()
        .expect("balanced reduction retains one root");
    sources.push(Source::Computed(Operation::Add([simple, high])));
    let mut program = Program {
        instructions: Vec::new(),
        operands: Vec::new(),
        parameters: Vec::new(),
        result,
    };
    let mut constants = vec![0.25, 0.5];
    let width = program.allocate_registers(&sources, &mut constants);
    assert!(program.validate(width, constants.len()));
    let division = program
        .instructions
        .iter()
        .position(|instruction| matches!(instruction.operation, Operation::Divide(_)));
    assert_eq!(division, Some(program.instructions.len().saturating_sub(2)));
    assert!(width <= 10);
    constants.resize(width, 0.0);
    let mut evaluator = Evaluator {
        program: Arc::new(program),
        registers: constants,
    };
    assert_eq!(evaluator.evaluate(&[]).expect("constant program"), 8.5);
}
