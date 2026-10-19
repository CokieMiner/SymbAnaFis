//! Borrowed inspection of allocated instructions and immutable literal registers.

use core::fmt::{Debug, Display, Formatter, Result as FmtResult};

use super::{Evaluator, Operation, PairedFunction};

struct Disassembly<'program, Value>(&'program Evaluator<Value>);

impl<Value: Debug> Evaluator<Value> {
    /// Inspect the compiled scalar program as a borrowed text display.
    ///
    /// Lists ordered parameters, stored constants, physical register operands,
    /// builtin names, and the result register. Registers can be overwritten after
    /// their last use. Symbol names and literal representations use escaped debug
    /// formatting. Current input values and temporary contents are omitted.
    /// Formatting requires no evaluator mutation or numerical execution.
    /// The text layout is descriptive and is not a serialized program format.
    ///
    /// # Panics
    /// Propagates a custom scalar's `Debug` formatting panics.
    ///
    /// ```
    /// use symb_anafis::{Context, Evaluator};
    /// let context = Context::new();
    /// let x = context.symb("x")?;
    /// let evaluator = Evaluator::<f64>::compile(&(x.sin() + 1), &[x])?;
    /// let listing = format!("{}", evaluator.disassemble());
    /// assert!(listing.contains("sin r0"));
    /// assert!(listing.contains("return r"));
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    #[must_use]
    pub fn disassemble(&self) -> impl Display + '_ {
        Disassembly(self)
    }
}

impl<Value: Debug> Display for Disassembly<'_, Value> {
    #[expect(
        clippy::use_debug,
        reason = "instruction inspection escapes symbol names and preserves backend literal representations"
    )]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        let evaluator = self.0;
        let program = &evaluator.program;
        writeln!(
            formatter,
            "registers: {}, parameters: {}, instructions: {}",
            evaluator.registers.len(),
            program.parameters.len(),
            program.instructions.len()
        )?;
        for (index, symbol) in program.parameters.iter().enumerate() {
            write!(formatter, "r{index} = parameter {index} (")?;
            if let Some(name) = symbol.symbol().name() {
                write!(formatter, "{name:?}")?;
            } else {
                formatter.write_str("unnamed")?;
            }
            writeln!(formatter, ")")?;
        }
        // The allocator's first temporary follows all protected input and literal
        // slots. An instruction-free program contains only those protected slots.
        let prefix = program
            .instructions
            .first()
            .map_or(evaluator.registers.len(), |instruction| instruction.output);
        for (index, value) in evaluator
            .registers
            .iter()
            .enumerate()
            .take(prefix)
            .skip(program.parameters.len())
        {
            writeln!(formatter, "r{index} = constant {value:?}")?;
        }
        for (index, instruction) in program.instructions.iter().enumerate() {
            let (name, inputs): (&str, &[usize]) = match &instruction.operation {
                Operation::Add(inputs) => ("add", inputs),
                Operation::AddMany(_) => ("add", instruction.operation.inputs(&program.operands)),
                Operation::Subtract(inputs) => ("sub", inputs),
                Operation::Negate(inputs) => ("neg", inputs),
                Operation::Multiply(inputs) => ("mul", inputs),
                Operation::NegatedMultiply(inputs) => ("neg_mul", inputs),
                Operation::MultiplyMany(_) => {
                    ("mul", instruction.operation.inputs(&program.operands))
                }
                Operation::NegatedMultiplyMany(_) => {
                    ("neg_mul", instruction.operation.inputs(&program.operands))
                }
                Operation::Divide(inputs) => ("div", inputs),
                Operation::Power(inputs) => ("pow", inputs),
                Operation::Fused(inputs) => ("fma", inputs),
                Operation::FusedSubtract(inputs) => ("fms", inputs),
                Operation::NegatedFusedAdd(inputs) => ("fnma", inputs),
                Operation::NegatedFusedSubtract(inputs) => ("fnms", inputs),
                Operation::Unary(builtin, _, inputs) => (builtin.as_str(), inputs),
                Operation::Binary(builtin, _, inputs) => (builtin.as_str(), inputs),
                Operation::Ternary(_, inputs) => ("assoc_legendre", inputs),
                Operation::Quaternary(_, inputs) => ("spherical_harmonic", inputs),
                Operation::Pair(function, _, inputs, _) => (
                    match function {
                        PairedFunction::SinCos => "sin_cos",
                        PairedFunction::SinhCosh => "sinh_cosh",
                    },
                    inputs,
                ),
            };
            write!(formatter, "{index:04} r{}", instruction.output)?;
            if let Operation::Pair(_, _, _, second) = instruction.operation {
                write!(formatter, ", r{second}")?;
            }
            write!(formatter, " = {name}")?;
            for (position, input) in inputs.iter().enumerate() {
                write!(
                    formatter,
                    "{}r{input}",
                    if position == 0 { " " } else { ", " }
                )?;
            }
            writeln!(formatter)?;
        }
        writeln!(formatter, "return r{}", program.result)
    }
}
