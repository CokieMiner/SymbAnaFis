//! Joint outputs, projection liveness, pressure scheduling, and backend domains.

use core::ops::Add;

use alloc::{boxed::Box, string::ToString, sync::Arc, vec, vec::Vec};

use num_anafis::{Number, Numeric};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{collections::FixedMap, core::Context};

use super::{
    super::{
        Combine, Compilation, Instruction, NativeSlot, Operation, PairedFunction, Program,
        ProgramData, Source, Workspace,
    },
    support::Compiled,
};

proptest! {
    #[test]
    fn paired_argument_reads_finish_before_output_slots_are_reused(input in -20.0_f64..=20.0) {
        let context = Context::new()?;
        let x = context.symb("x").expect("available identity");
        for (expression, expected, name) in [
            (Add::add(x.sin(), x.cos()), Numeric::sin(&input) + Numeric::cos(&input), "sin_cos"),
            (Add::add(x.sinh(), x.cosh()), Numeric::sinh(&input) + Numeric::cosh(&input), "sinh_cosh"),
        ] {
            let mut evaluator = Compiled::<f64>::compile(&expression, &[x]).expect("real program");
            prop_assert_eq!(evaluator.instruction_count(), 2);
            prop_assert_eq!(evaluator.workspace_size(), 3);
            prop_assert!(evaluator.disassemble().to_string().contains(name));
            prop_assert_eq!(evaluator.evaluate(&[input]).expect("complete row").to_bits(), expected.to_bits());
        }
    }

    #[test]
    fn nested_pairs_and_shared_arguments_preserve_numeric_results(input in -2.0_f64..=2.0) {
        let context = Context::new()?;
        let x = context.symb("x").expect("available identity");
        let expression = context.parse("sin(sin(x)) + cos(sin(x)) + sin(cos(x)) + cos(cos(x))").expect("known functions");
        let mut compiled = Compiled::<Number>::compile(&expression, &[x]).expect("scalar program");
        let number = Number::from_float(input);
        let expected = expression.evaluate([(x, number.clone())]).expect("bound input");
        let actual = compiled.evaluate(&[number]).expect("complete row");
        prop_assert_eq!(actual, expected);
        prop_assert_eq!(compiled.program().data.instructions.iter().filter(|instruction| matches!(instruction.operation, Operation::Pair(..))).count(), 3);
    }
}

#[test]
fn unrelated_arguments_and_single_outputs_keep_separate_kernels() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let y = context.symb("y").expect("available identity");
    for expression in [
        Add::add(x.sin(), y.cos()),
        Add::add(x.sinh(), y.cosh()),
        x.sin(),
    ] {
        let evaluator = Compiled::<f64>::compile(&expression, &[x, y]).expect("real program");
        assert!(
            evaluator
                .program()
                .data
                .instructions
                .iter()
                .all(|instruction| !matches!(instruction.operation, Operation::Pair(..)))
        );
    }
}

#[test]
fn projection_liveness_preserves_either_output_under_pressure_scheduling() {
    for choose_cosine in [false, true] {
        // `sin_cos(0.0) == (0.0, 1.0)`, followed by 100 additions of `0.25`.
        let mut sources = vec![
            Source::Constant(0),
            Source::Constant(1),
            Source::Computed(Operation::Pair(PairedFunction::SinCos, [0], 3)),
            Source::Projection(2),
        ];
        let mut result = if choose_cosine { 3_u32 } else { 2_u32 };
        for _ in 0..100 {
            let index = u32::try_from(sources.len()).expect("small index");
            sources.push(Source::Computed(Operation::Combine(
                Combine::Add,
                [result, 1],
            )));
            result = index;
        }
        let compilation = Compilation {
            operands: Vec::new(),
            parameters: Box::new([]),
            results: vec![result],
            sources,
            constants: vec![0.0_f64, 0.25],
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
        assert_eq!(program.instructions.len(), 101);
        let compiled = Program {
            data: Arc::new(program),
            native: NativeSlot::none(),
        };
        let mut workspace = Workspace::new();
        assert_eq!(
            compiled
                .eval_with(&mut workspace, &[])
                .expect("constant program"),
            if choose_cosine { 26.0 } else { 25.0 }
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
        let program = ProgramData::<f64> {
            instructions: vec![Instruction {
                output: first,
                operation: Operation::Pair(PairedFunction::SinCos, [0], second),
            }]
            .into_boxed_slice(),
            operands: Box::new([]),
            parameters: Box::new([]),
            active_parameters: Box::new([]),
            results: vec![2].into_boxed_slice(),
            constants: Box::new([]),
            register_count: 0,
            initial: 0.0,
        };
        assert_eq!(program.validate(3, 1), valid);
    }
}
