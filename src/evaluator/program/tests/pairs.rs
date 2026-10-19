//! Joint outputs, projection liveness, pressure scheduling, and backend domains.

use core::ops::Add;

use alloc::{string::ToString, sync::Arc, vec, vec::Vec};

use num_anafis::{F64Ext, Number};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{Context, Evaluator};

use super::super::{Instruction, Operation, PairedFunction, Program, Source};

proptest! {
    #[test]
    fn paired_argument_reads_finish_before_output_slots_are_reused(input in -20.0_f64..=20.0) {
        let context = Context::new();
        let x = context.symb("x").expect("available identity");
        for (expression, expected, name) in [
            (Add::add(x.sin(), x.cos()), F64Ext::sin(input) + F64Ext::cos(input), "sin_cos"),
            (Add::add(x.sinh(), x.cosh()), F64Ext::sinh(input) + F64Ext::cosh(input), "sinh_cosh"),
        ] {
            let mut evaluator = Evaluator::<f64>::compile(&expression, &[x]).expect("real program");
            prop_assert_eq!(evaluator.instruction_count(), 2);
            prop_assert_eq!(evaluator.workspace_size(), 3);
            prop_assert!(evaluator.disassemble().to_string().contains(name));
            prop_assert_eq!(evaluator.evaluate(&[input]).expect("complete row").to_bits(), expected.to_bits());
        }
    }

    #[test]
    fn nested_pairs_and_shared_arguments_preserve_numeric_results(input in -2.0_f64..=2.0) {
        let context = Context::new();
        let x = context.symb("x").expect("available identity");
        let expression = context.parse("sin(sin(x)) + cos(sin(x)) + sin(cos(x)) + cos(cos(x))").expect("known functions");
        let mut compiled = Evaluator::<Number>::compile(&expression, &[x]).expect("scalar program");
        let number = Number::from_float(input);
        let expected = expression.evaluate([(x, number.clone())]).expect("bound input");
        let actual = compiled.evaluate(&[number]).expect("complete row");
        prop_assert_eq!(actual, expected);
        prop_assert_eq!(compiled.program.instructions.iter().filter(|instruction| matches!(instruction.operation, Operation::Pair(..))).count(), 3);
    }
}

#[test]
fn unrelated_arguments_and_single_outputs_keep_separate_kernels() {
    let context = Context::new();
    let x = context.symb("x").expect("available identity");
    let y = context.symb("y").expect("available identity");
    for expression in [
        Add::add(x.sin(), y.cos()),
        Add::add(x.sinh(), y.cosh()),
        x.sin(),
    ] {
        let evaluator = Evaluator::<f64>::compile(&expression, &[x, y]).expect("real program");
        assert!(
            evaluator
                .program
                .instructions
                .iter()
                .all(|instruction| !matches!(instruction.operation, Operation::Pair(..)))
        );
    }
}

#[test]
fn projection_liveness_preserves_either_output_under_pressure_scheduling() {
    for choose_cosine in [false, true] {
        let mut sources = vec![
            Source::Constant(0),
            Source::Computed(Operation::Pair(
                PairedFunction::SinCos,
                |value: &f64| (value + 1.0, value + 2.0),
                [0],
                2,
            )),
            Source::Projection(1),
        ];
        let mut result = if choose_cosine { 2 } else { 1 };
        for _ in 0..100 {
            let index = sources.len();
            sources.push(Source::Computed(Operation::Add([result, 0])));
            result = index;
        }
        let mut program = Program {
            instructions: Vec::new(),
            operands: Vec::new(),
            parameters: Vec::new(),
            result,
        };
        let mut constants = vec![0.25];
        let width = program.allocate_registers(&sources, &mut constants);
        assert!(program.validate(width, constants.len()));
        assert_eq!(program.instructions.len(), 101);
        constants.resize(width, 0.0);
        let mut evaluator = Evaluator {
            program: Arc::new(program),
            registers: constants,
        };
        assert_eq!(
            evaluator.evaluate(&[]).expect("constant program"),
            if choose_cosine { 27.25 } else { 26.25 }
        );
    }
}

#[test]
fn joint_destinations_are_distinct_initialized_and_outside_the_prefix() {
    for (first, second, valid) in [
        (1, 2, true),
        (1, 1, false),
        (1, 0, false),
        (1, 3, false),
        (0, 2, false),
    ] {
        let program = Program::<f64> {
            instructions: vec![Instruction {
                output: first,
                operation: Operation::Pair(
                    PairedFunction::SinCos,
                    |value| (*value, *value),
                    [0],
                    second,
                ),
            }],
            operands: Vec::new(),
            parameters: Vec::new(),
            result: 2,
        };
        assert_eq!(program.validate(3, 1), valid);
    }
}
