//! Independent register validation and generated SSA allocation checks.

use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};

use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{collections::FixedMap, core::BuiltinFunction};

use super::super::{
    Combine, Compilation, Fold, Fused, Instruction, NativeSlot, OperandSpan, Operation, Program,
    ProgramData, Source, Workspace,
};

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
        let constants = values.clone();
        let mut sources = vec![Source::Constant(0), Source::Constant(1)];
        let mut operands = Vec::new();
        let mut result_source = 0_u32;
        for (left, right, kind) in operations {
            let a_idx = usize::from(left) % values.len();
            let b_idx = usize::from(right) % values.len();
            let a = u32::try_from(a_idx).expect("small SSA index");
            let b = u32::try_from(b_idx).expect("small SSA index");
            let (operation, expected) = match kind {
                0 => (Operation::Combine(Combine::Add, [a, b]), values[a_idx] + values[b_idx]),
                1 => (Operation::Combine(Combine::Multiply, [a, b]), values[a_idx] * values[b_idx]),
                2 => (Operation::Divide([a, b]), values[a_idx] / values[b_idx]),
                3 => {
                    let start = u32::try_from(operands.len()).expect("small pool");
                    operands.extend([a, b, a]);
                    let end = u32::try_from(operands.len()).expect("small pool");
                    (Operation::Fold(Fold::Sum, OperandSpan { start, end }), (values[a_idx] + values[b_idx]) + values[a_idx])
                }
                _ => {
                    let start = u32::try_from(operands.len()).expect("small pool");
                    operands.extend([a, b, b]);
                    let end = u32::try_from(operands.len()).expect("small pool");
                    (Operation::Fold(Fold::Product, OperandSpan { start, end }), (values[a_idx] * values[b_idx]) * values[b_idx])
                }
            };
            result_source = u32::try_from(sources.len()).expect("small source count");
            sources.push(Source::Computed(operation));
            values.push(expected);
        }
        let expected = *values.last().expect("two constants and at least one instruction");
        let compilation = Compilation {
            operands,
            parameters: Box::new([]),
            results: vec![result_source],
            sources,
            constants,
            parameter_map: FixedMap::default(),
            powers: FixedMap::default(),
            constant_reciprocals: FixedMap::default(),
            constant_bits: FixedMap::default(),
            exponent_constants: FixedMap::default(),
            denominators: FixedMap::default(),
            binary: FixedMap::default(),
            fused: FixedMap::default(),
            folds: FixedMap::default(),
            unit: None,
            initial: 0.0,
            trig_mask: 0,
        };
        let program = compilation.finish().expect("register allocation");
        prop_assert!(program.validate(program.register_count, program.constants.len()));
        let compiled = Program {
            data: Arc::new(program),
            native: NativeSlot::none(),
        };
        let mut workspace = Workspace::new();
        let actual = compiled
            .eval_with(&mut workspace, &[])
            .expect("constant program");
        if expected.is_nan() {
            prop_assert!(actual.is_nan());
        } else {
            prop_assert_eq!(actual.to_bits(), expected.to_bits());
        }
    }
}

#[test]
fn validator_rejects_uninitialized_reads_invalid_ranges_and_prefix_writes() {
    let mut program = ProgramData::<f64> {
        instructions: vec![Instruction {
            output: 2,
            operation: Operation::Combine(Combine::Add, [0, 1]),
        }]
        .into_boxed_slice(),
        operands: vec![0, 1, 0].into_boxed_slice(),
        parameters: Box::new([]),
        active_parameters: Box::new([]),
        results: vec![2].into_boxed_slice(),
        constants: Box::new([]),
        register_count: 0,
        initial: 0.0,
    };
    assert!(program.validate(3, 2));
    assert!(!program.validate(3, 4));
    assert!(!program.validate(2, 2));
    for operation in [
        Operation::Combine(Combine::Add, [0, 3]),
        Operation::Combine(Combine::Multiply, [2, 0]),
        Operation::Divide([3, 0]),
        Operation::Power([0, 3]),
        Operation::Fused(
            Fused {
                negate_product: false,
                negate_addend: false,
            },
            [0, 1, 2],
        ),
        Operation::Unary(BuiltinFunction::Abs, [3]),
        Operation::Unary(BuiltinFunction::Beta, [0]),
        Operation::Binary(BuiltinFunction::Beta, [0, 2]),
        Operation::Binary(BuiltinFunction::Abs, [0, 1]),
        Operation::Ternary(BuiltinFunction::AssocLegendre, [0, 1, 3]),
        Operation::Fold(Fold::Sum, OperandSpan { start: 0, end: 0 }),
        Operation::Fold(Fold::Sum, OperandSpan { start: 0, end: 2 }),
        Operation::Fold(Fold::Product, OperandSpan { start: 1, end: 4 }),
    ] {
        program.instructions = vec![Instruction {
            output: 2,
            operation,
        }]
        .into_boxed_slice();
        assert!(!program.validate(3, 2));
    }
    program.instructions = vec![Instruction {
        output: 0,
        operation: Operation::Combine(Combine::Add, [0, 1]),
    }]
    .into_boxed_slice();
    assert!(!program.validate(3, 2));
    program.instructions = Box::new([]);
    assert!(!program.validate(3, 2));
    program.results = vec![0].into_boxed_slice();
    assert!(program.validate(3, 2));
    program.results = vec![3].into_boxed_slice();
    assert!(!program.validate(3, 2));
}

#[test]
fn pressure_scheduling_finishes_the_larger_subtree_before_a_simple_sibling() {
    let mut sources = vec![Source::Constant(0), Source::Constant(1)];
    let simple = u32::try_from(sources.len()).expect("small index");
    sources.push(Source::Computed(Operation::Divide([0, 1])));
    let mut frontier: Vec<u32> = (0..64)
        .map(|_| {
            let index = u32::try_from(sources.len()).expect("small index");
            sources.push(Source::Computed(Operation::Combine(
                Combine::Multiply,
                [0, 1],
            )));
            index
        })
        .collect();
    while frontier.len() > 1 {
        frontier = frontier
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[left, right]| {
                let index = u32::try_from(sources.len()).expect("small index");
                sources.push(Source::Computed(Operation::Combine(
                    Combine::Add,
                    [*left, *right],
                )));
                index
            })
            .collect();
    }
    let result = u32::try_from(sources.len()).expect("small index");
    let high = *frontier
        .first()
        .expect("balanced reduction retains one root");
    sources.push(Source::Computed(Operation::Combine(
        Combine::Add,
        [simple, high],
    )));
    let compilation = Compilation {
        operands: Vec::new(),
        parameters: Box::new([]),
        results: vec![result],
        sources,
        constants: vec![0.25, 0.5],
        parameter_map: FixedMap::default(),
        powers: FixedMap::default(),
        constant_reciprocals: FixedMap::default(),
        constant_bits: FixedMap::default(),
        exponent_constants: FixedMap::default(),
        denominators: FixedMap::default(),
        binary: FixedMap::default(),
        fused: FixedMap::default(),
        folds: FixedMap::default(),
        unit: None,
        initial: 0.0,
        trig_mask: 0,
    };
    let program = compilation.finish().expect("register allocation");
    assert!(program.validate(program.register_count, program.constants.len()));
    let division = program
        .instructions
        .iter()
        .position(|instruction| matches!(instruction.operation, Operation::Divide(_)));
    assert_eq!(division, Some(program.instructions.len().saturating_sub(2)));
    assert!(program.register_count <= 10);
    let compiled = Program {
        data: Arc::new(program),
        native: NativeSlot::none(),
    };
    let mut workspace = Workspace::new();
    assert_eq!(
        compiled
            .eval_with(&mut workspace, &[])
            .expect("constant program"),
        8.5
    );
}
