//! Register listing stability, opcode names, and constant-only programs.

use core::ops::Add;

use alloc::{format, string::ToString};

use num_anafis::Number;
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{Context, Evaluator, Expr};

use super::super::Instruction;

proptest! {
    #[test]
    fn listing_preserves_literal_registers_after_execution(
        constant in 1_i32..=100,
        input in -2.0_f64..=2.0,
    ) {
        let context = Context::new();
        let x = context.symb("x").expect("available identity");
        let expression = Add::add(x.sin(), constant);
        let mut evaluator = Evaluator::<f64>::compile(&expression, &[x]).expect("real program");
        let before = evaluator.disassemble().to_string();
        let output = evaluator.evaluate(&[input]).expect("complete row");
        let after = evaluator.disassemble().to_string();
        prop_assert_eq!(before, after);
        prop_assert_eq!(output.to_bits(), evaluator.evaluate(&[input]).expect("complete row").to_bits());
        let listing = evaluator.disassemble().to_string();
        prop_assert!(listing.contains("sin r0"));
        prop_assert!(listing.contains(" = constant "));
        prop_assert!(listing.contains(" = add "));
    }
}

#[test]
fn constant_and_parameter_programs_list_their_result_without_instructions() {
    let constant = Evaluator::<Number>::compile(&Expr::number(7), &[]).expect("constant");
    let listing = constant.disassemble().to_string();
    assert!(listing.contains("instructions: 0"));
    assert!(listing.contains("r0 = constant "));
    assert!(listing.ends_with("return r0\n"));

    let context = Context::new();
    let symbol = context.symb("escaped\nname").expect("available identity");
    let parameter = Evaluator::<f64>::compile(&symbol.to_expr(), &[symbol]).expect("parameter");
    let parameter_listing = parameter.disassemble().to_string();
    assert!(parameter_listing.contains("escaped\\nname"));
    assert!(!parameter_listing.contains(" = constant "));
    assert!(parameter_listing.ends_with("return r0\n"));
}

#[test]
fn function_labels_cover_all_arities_and_generic_power() {
    let context = Context::new();
    let x = context.symb("x").expect("available identity");
    for (text, opcode) in [
        ("sin(x)", "sin"),
        ("atan2(x, 2)", "atan2"),
        ("assoc_legendre(2, 1, x)", "assoc_legendre"),
        ("spherical_harmonic(2, 1, x, 3)", "spherical_harmonic"),
        ("x^1.25", "pow"),
    ] {
        let expression = context.parse(text).expect("known function");
        let evaluator = Evaluator::<f64>::compile(&expression, &[x]).expect("real program");
        assert!(
            evaluator
                .disassemble()
                .to_string()
                .contains(&format!(" = {opcode} "))
        );
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
fn builtin_labels_fit_the_existing_instruction_layout() {
    assert_eq!(core::mem::size_of::<Instruction<f64>>(), 56);
    assert_eq!(core::mem::size_of::<Instruction<Number>>(), 56);
}
