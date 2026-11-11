//! Borrowed inspection of allocated instructions and immutable literal registers.

#![expect(
    unsafe_code,
    reason = "a program exists only after validation, which bounds every pooled range"
)]

use core::fmt::{Debug, Display, Formatter, Result as FmtResult};

use super::{Fold, Operation, PairedFunction, Program, ProgramData};

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

impl<Value: Debug> Display for Disassembly<'_, Value> {
    /// Symbol names and literals are written by their `Debug` implementations,
    /// which escape names and keep the backend's literal representation.
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
            if let Some(name) = symbol.symbol().name() {
                Debug::fmt(&*name, formatter)?;
            } else {
                formatter.write_str("unnamed")?;
            }
            writeln!(formatter, ")")?;
        }
        // The allocator's first temporary follows all protected input and literal
        // slots. An instruction-free program contains only those protected slots.
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
                Operation::Add(_) | Operation::Fold(Fold::Sum, _) => "add",
                Operation::Subtract(_) => "sub",
                Operation::Negate(_) => "neg",
                Operation::Multiply(_) | Operation::Fold(Fold::Product, _) => "mul",
                Operation::NegatedMultiply(_) => "neg_mul",
                Operation::Fold(Fold::Dot, _) => "dot",
                Operation::Divide(_) => "div",
                Operation::Power(_) => "pow",
                Operation::Fused(_) => "fma",
                Operation::FusedSubtract(_) => "fms",
                Operation::NegatedFusedAdd(_) => "fnma",
                Operation::NegatedFusedSubtract(_) => "fnms",
                Operation::Unary(builtin, ..)
                | Operation::Binary(builtin, ..)
                | Operation::Ternary(builtin, ..) => builtin.as_str(),
                Operation::Pair(PairedFunction::SinCos, ..) => "sin_cos",
                Operation::Pair(PairedFunction::SinhCosh, ..) => "sinh_cosh",
            };
            write!(formatter, "{index:04} r{}", instruction.output)?;
            if let Operation::Pair(_, _, _, second) = instruction.operation {
                write!(formatter, ", r{second}")?;
            }
            write!(formatter, " = {name}")?;
            // SAFETY: a program is constructed only after `validate`, which
            // proves every pooled range in its operand pool.
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
        writeln!(formatter, "return r{}", program.result)
    }
}
