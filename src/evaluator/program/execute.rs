//! Scalar execution using compile-time validated register indices.

use crate::{
    core::{Scalar, Symbol},
    errors::EvalError,
};

use super::{Evaluator, Operation};

impl<Value: Scalar> Evaluator<Value> {
    /// Evaluate one complete parameter row without allocating a workspace.
    /// Parameters follow the order supplied to [`Self::compile`].
    /// NaNs, infinities, exact overflow, and complex continuation follow the scalar.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`] for missing or extra inputs,
    /// before modifying the execution workspace.
    ///
    /// # Panics
    /// Propagates numerical backend panics.
    #[expect(
        unsafe_code,
        reason = "the checked input length equals the independently validated parameter prefix of the fixed workspace"
    )]
    pub fn evaluate(&mut self, parameters: &[Value]) -> Result<Value, EvalError> {
        if parameters.len() != self.program.parameters.len() {
            return Err(EvalError::ParameterCount {
                expected: self.program.parameters.len(),
                actual: parameters.len(),
            });
        }
        // SAFETY: The checked length equals the program's parameter count.
        // Validation places that entire prefix inside the fixed workspace.
        let inputs = unsafe { self.registers.get_unchecked_mut(..parameters.len()) };
        inputs.clone_from_slice(parameters);
        Ok(self.execute())
    }

    /// Evaluate parameter columns into caller-owned output using scalar execution.
    /// All columns must have exactly `output.len()` entries, including unused
    /// parameters. Zero-parameter programs repeat their scalar result per row.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`] or [`EvalError::BatchLength`]
    /// before writing any output when column shapes disagree.
    ///
    /// # Panics
    /// Propagates numerical backend panics; previously written rows remain valid.
    #[expect(
        unsafe_code,
        reason = "all column lengths equal the output length before bounded output enumeration reads any row"
    )]
    pub fn evaluate_batch(
        &mut self,
        columns: &[&[Value]],
        output: &mut [Value],
    ) -> Result<(), EvalError> {
        if columns.len() != self.program.parameters.len() {
            return Err(EvalError::ParameterCount {
                expected: self.program.parameters.len(),
                actual: columns.len(),
            });
        }
        for (parameter, column) in columns.iter().enumerate() {
            if column.len() != output.len() {
                return Err(EvalError::BatchLength {
                    parameter,
                    expected: output.len(),
                    actual: column.len(),
                });
            }
        }
        for (row, destination) in output.iter_mut().enumerate() {
            for (slot, column) in self.registers.iter_mut().zip(columns) {
                // SAFETY: Every column has output.len() entries, and row comes
                // from enumerating that same output slice without resizing it.
                slot.clone_from(unsafe { column.get_unchecked(row) });
            }
            *destination = self.execute();
        }
        Ok(())
    }

    /// Inspect retained symbol identities in their numerical input order.
    /// The returned symbols borrow this program's metadata.
    pub fn parameters(&self) -> impl ExactSizeIterator<Item = Symbol<'_>> + '_ {
        self.program.parameters.iter().map(|data| data.symbol())
    }

    /// The number of initialized scalar slots in this evaluator's workspace.
    #[must_use]
    pub const fn workspace_size(&self) -> usize {
        self.registers.len()
    }

    /// The number of reachable instructions after shared-node reuse.
    #[must_use]
    pub fn instruction_count(&self) -> usize {
        self.program.instructions.len()
    }

    #[expect(
        unsafe_code,
        clippy::too_many_lines,
        reason = "Program::validate proves every immutable instruction's bounds and initialization before the evaluator is constructed"
    )]
    fn execute(&mut self) -> Value {
        // SAFETY: Program::validate checks every pooled range, nonempty arithmetic
        // group, source, destination, result, and read-before-write dependency.
        // Construction initializes exactly the validated workspace length. Private
        // fields keep that length and the shared program immutable during execution.
        // Joint outputs are distinct. Every kernel finishes reading its input
        // before either output is written, including reuse of a dying input slot.
        unsafe {
            for instruction in &self.program.instructions {
                let value = match &instruction.operation {
                    Operation::Add([left, right]) => self
                        .registers
                        .get_unchecked(*left)
                        .clone()
                        .add(self.registers.get_unchecked(*right)),
                    Operation::Subtract([left, right]) => self
                        .registers
                        .get_unchecked(*left)
                        .clone()
                        .sub(self.registers.get_unchecked(*right)),
                    Operation::Negate([input]) => {
                        self.registers.get_unchecked(*input).clone().neg()
                    }
                    Operation::Multiply([left, right]) => self
                        .registers
                        .get_unchecked(*left)
                        .clone()
                        .mul(self.registers.get_unchecked(*right)),
                    Operation::NegatedMultiply([left, right]) => self
                        .registers
                        .get_unchecked(*left)
                        .clone()
                        .mul(self.registers.get_unchecked(*right))
                        .neg(),
                    Operation::Divide([left, right]) => self
                        .registers
                        .get_unchecked(*left)
                        .clone()
                        .div(self.registers.get_unchecked(*right)),
                    Operation::AddMany(range)
                    | Operation::MultiplyMany(range)
                    | Operation::NegatedMultiplyMany(range) => {
                        let operands = self.program.operands.get_unchecked(range.clone());
                        let first = *operands.get_unchecked(0);
                        let initial = self.registers.get_unchecked(first).clone();
                        if matches!(instruction.operation, Operation::AddMany(_)) {
                            operands.iter().skip(1).fold(initial, |sum, source| {
                                sum.add(self.registers.get_unchecked(*source))
                            })
                        } else {
                            let product =
                                operands.iter().skip(1).fold(initial, |product, source| {
                                    product.mul(self.registers.get_unchecked(*source))
                                });
                            if matches!(instruction.operation, Operation::NegatedMultiplyMany(_)) {
                                product.neg()
                            } else {
                                product
                            }
                        }
                    }
                    Operation::Power([base, exponent]) => Value::power(
                        self.registers.get_unchecked(*base),
                        self.registers.get_unchecked(*exponent),
                    ),
                    Operation::Unary(_, kernel, [value]) => {
                        kernel(self.registers.get_unchecked(*value))
                    }
                    Operation::Pair(_, kernel, [value], second) => {
                        let (sine, cosine) = kernel(self.registers.get_unchecked(*value));
                        *self.registers.get_unchecked_mut(*second) = cosine;
                        sine
                    }
                    Operation::Fused([left, right, addend]) => Value::fused(
                        self.registers.get_unchecked(*left),
                        self.registers.get_unchecked(*right),
                        self.registers.get_unchecked(*addend),
                    ),
                    Operation::FusedSubtract([left, right, subtrahend]) => Value::fused(
                        self.registers.get_unchecked(*left),
                        self.registers.get_unchecked(*right),
                        &self.registers.get_unchecked(*subtrahend).clone().neg(),
                    ),
                    Operation::NegatedFusedAdd([left, right, addend]) => Value::fused(
                        &self.registers.get_unchecked(*left).clone().neg(),
                        self.registers.get_unchecked(*right),
                        self.registers.get_unchecked(*addend),
                    ),
                    Operation::NegatedFusedSubtract([left, right, subtrahend]) => Value::fused(
                        &self.registers.get_unchecked(*left).clone().neg(),
                        self.registers.get_unchecked(*right),
                        &self.registers.get_unchecked(*subtrahend).clone().neg(),
                    ),
                    Operation::Binary(_, kernel, [left, right]) => kernel(
                        self.registers.get_unchecked(*left),
                        self.registers.get_unchecked(*right),
                    ),
                    Operation::Ternary(kernel, [first, second, third]) => kernel(
                        self.registers.get_unchecked(*first),
                        self.registers.get_unchecked(*second),
                        self.registers.get_unchecked(*third),
                    ),
                    Operation::Quaternary(kernel, [first, second, third, fourth]) => kernel(
                        self.registers.get_unchecked(*first),
                        self.registers.get_unchecked(*second),
                        self.registers.get_unchecked(*third),
                        self.registers.get_unchecked(*fourth),
                    ),
                };
                // Every source is read before the output reuses a dying source slot.
                *self.registers.get_unchecked_mut(instruction.output) = value;
            }
            self.registers.get_unchecked(self.program.result).clone()
        }
    }
}
