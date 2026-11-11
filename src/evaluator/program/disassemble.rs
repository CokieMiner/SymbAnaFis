//! Borrowed inspection of allocated instructions and immutable literal registers.

#![expect(
    unsafe_code,
    reason = "a program exists only after validation, which bounds every pooled range"
)]

use core::fmt::{Debug, Display, Formatter, Result as FmtResult};

use super::{Combine, Fold, MultiProgram, Operation, PairedFunction, Program, ProgramData};

struct Disassembly<'program, Value>(&'program ProgramData<Value>);

impl<Value: Debug> Program<Value> {
    /// Inspect the compiled scalar program as a borrowed text display.
    ///
    /// Lists ordered parameters, stored constants, physical register operands,
    /// builtin names, and the result register. Registers can be overwritten after
    /// their last use. Symbol names and literal representations use escaped debug
    /// formatting. Current input values and temporary contents are omitted.
    /// Formatting requires no workspace and performs no numerical execution.
    /// The text layout is descriptive and is not a serialized program format.
    ///
    /// # Panics
    /// Propagates a custom scalar's `Debug` formatting panics.
    ///
    /// ```
    /// use symb_anafis::{Context, Program};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let program = Program::<f64>::compile(&(x.sin() + 1), &[x])?;
    /// let listing = format!("{}", program.disassemble());
    /// assert!(listing.contains("sin r0"));
    /// assert!(listing.contains("return r"));
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    #[must_use]
    pub fn disassemble(&self) -> impl Display + '_ {
        Disassembly(&self.data)
    }
}

impl<Value: Debug> MultiProgram<Value> {
    /// Inspect the compiled multi-output program as a borrowed text display.
    ///
    /// Lists ordered parameters, stored constants, physical register operands,
    /// builtin names, and all result registers.
    ///
    /// # Panics
    /// Propagates a custom scalar's `Debug` formatting panics.
    #[must_use]
    pub fn disassemble(&self) -> impl Display + '_ {
        Disassembly(&self.data)
    }
}

impl<Value: Debug> Display for Disassembly<'_, Value> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        let program = self.0;
        writeln!(
            formatter,
            "registers: {}, parameters: {}, instructions: {}",
            program.register_count,
            program.parameters.len(),
            program.instructions.len()
        )?;
        for (index, symbol) in program.parameters.iter().enumerate() {
            write!(formatter, "r{index} = parameter {index} (")?;
            if let Some(name) = symbol.name() {
                Debug::fmt(name, formatter)?;
            } else {
                formatter.write_str("unnamed")?;
            }
            writeln!(formatter, ")")?;
        }
        for (index, value) in program.constants.iter().enumerate() {
            write!(
                formatter,
                "r{} = constant ",
                program.parameters.len().saturating_add(index)
            )?;
            Debug::fmt(value, formatter)?;
            writeln!(formatter)?;
        }
        for (index, instruction) in program.instructions.iter().enumerate() {
            let name = match &instruction.operation {
                Operation::Combine(Combine::Add, _) | Operation::Fold(Fold::Sum, _) => "add",
                Operation::Combine(Combine::Subtract, _) => "sub",
                Operation::Negate(_) => "neg",
                Operation::Combine(Combine::Multiply, _) | Operation::Fold(Fold::Product, _) => {
                    "mul"
                }
                Operation::NegatedMultiply(_) => "neg_mul",
                Operation::Fold(Fold::Dot, _) => "dot",
                Operation::Divide(_) => "div",
                Operation::Power(_) => "pow",
                Operation::Fused(fused, _) => match (fused.negate_product, fused.negate_addend) {
                    (false, false) => "fma",
                    (false, true) => "fms",
                    (true, false) => "fnma",
                    (true, true) => "fnms",
                },
                Operation::Unary(builtin, _)
                | Operation::Binary(builtin, _)
                | Operation::Ternary(builtin, _) => builtin.as_str(),
                Operation::Pair(PairedFunction::SinCos, ..) => "sin_cos",
                Operation::Pair(PairedFunction::SinhCosh, ..) => "sinh_cosh",
            };
            write!(formatter, "{index:04} r{}", instruction.output)?;
            if let Operation::Pair(_, _, second) = instruction.operation {
                write!(formatter, ", r{second}")?;
            }
            write!(formatter, " = {name}")?;
            // SAFETY: `validate` proved every pooled span in `program.operands`.
            let inputs = unsafe { instruction.operation.inputs(&program.operands) };
            for (position, input) in inputs.iter().enumerate() {
                write!(
                    formatter,
                    "{}r{input}",
                    if position == 0 { " " } else { ", " }
                )?;
            }
            writeln!(formatter)?;
        }
        match &*program.results {
            [] => writeln!(formatter, "return ()"),
            [single] => writeln!(formatter, "return r{single}"),
            [first, rest @ ..] => {
                write!(formatter, "return r{first}")?;
                for result in rest {
                    write!(formatter, ", r{result}")?;
                }
                writeln!(formatter)
            }
        }
    }
}
