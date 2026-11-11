//! Scalar and blocked execution using compile-time validated register indices.

#![expect(
    unsafe_code,
    reason = "validated programs and equal column lengths prove every register and row access before execution"
)]

use core::{
    array::from_fn,
    mem::ManuallyDrop,
    ops::{Add, Div, Mul, Neg, Sub},
    ptr::read,
};

use crate::{
    core::{Scalar, Symbol},
    errors::EvalError,
};

use super::{BLOCK, Fold, Instruction, Operation, Program, ProgramData, Workspace};

impl<Value: Scalar> Program<Value> {
    /// Evaluate one complete parameter row into a caller-owned workspace.
    /// Parameters follow the order supplied to [`Self::compile`].
    /// A workspace bound to another program is reinitialized before execution.
    /// NaNs, infinities, exact overflow, and complex continuation follow the scalar.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`] for missing or extra inputs. The
    /// workspace is not modified on this error.
    ///
    /// # Panics
    /// Propagates numerical backend panics.
    pub fn evaluate(
        &self,
        workspace: &mut Workspace<Value>,
        parameters: &[Value],
    ) -> Result<Value, EvalError> {
        if parameters.len() != self.data.parameters.len() {
            return Err(EvalError::ParameterCount {
                expected: self.data.parameters.len(),
                actual: parameters.len(),
            });
        }
        let registers = workspace.prepare::<1>(self);
        // The parameter slots open the register file.
        for ([slot], value) in registers.iter_mut().zip(parameters) {
            slot.clone_from(value);
        }
        let [result] = self.data.execute(registers);
        Ok(result.clone())
    }

    /// Evaluate one complete parameter row using a transient workspace.
    ///
    /// For repeated evaluations, create a persistent [`Workspace`] with
    /// [`Self::workspace`] and pass it to [`Self::evaluate`] to avoid
    /// allocating and preparing registers on each call.
    ///
    /// # Errors
    /// Returns [`EvalError`] on parameter count mismatches or evaluation errors.
    ///
    /// # Panics
    /// Propagates numerical backend panics.
    ///
    /// ```
    /// use symb_anafis::{Context, Program};
    /// let context = Context::new()?;
    /// let x = context.symb("eval_x")?;
    /// let program = Program::<f64>::compile(&(x + 10), &[x])?;
    /// assert_eq!(program.eval(&[5.0])?, 15.0);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn eval(&self, parameters: &[Value]) -> Result<Value, EvalError> {
        // `evaluate` binds the fresh workspace to this program.
        self.evaluate(&mut Workspace::new(), parameters)
    }

    /// Evaluate parameter columns into caller-owned output using a workspace.
    /// All columns must have exactly `output.len()` entries, including unused
    /// parameters. Zero-parameter programs repeat their scalar result per row.
    /// Native floating programs execute each instruction on a block of sixteen
    /// rows; the final block repeats the last row in its unused lanes.
    ///
    /// # Errors
    /// Returns [`EvalError::ParameterCount`] or [`EvalError::BatchLength`]
    /// before writing any output when column shapes disagree.
    ///
    /// # Panics
    /// Propagates numerical backend panics; previously written rows remain valid.
    pub fn evaluate_batch(
        &self,
        workspace: &mut Workspace<Value>,
        columns: &[&[Value]],
        output: &mut [Value],
    ) -> Result<(), EvalError> {
        if columns.len() != self.data.parameters.len() {
            return Err(EvalError::ParameterCount {
                expected: self.data.parameters.len(),
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
        if Value::OPTIMIZE_FLOATS {
            self.batch::<BLOCK>(workspace, columns, output);
        } else {
            self.batch::<1>(workspace, columns, output);
        }
        Ok(())
    }

    /// Inspect retained symbol identities in their numerical input order.
    /// The returned symbols borrow this program's metadata.
    pub fn parameters(&self) -> impl ExactSizeIterator<Item = Symbol<'_>> + '_ {
        self.data.parameters.iter().map(|data| data.symbol())
    }

    /// The number of initialized scalar slots required by this program.
    #[must_use]
    pub fn workspace_size(&self) -> usize {
        self.data.register_count
    }

    /// The number of reachable instructions after shared-node reuse.
    #[must_use]
    pub fn instruction_count(&self) -> usize {
        self.data.instructions.len()
    }

    /// Evaluate validated columns in blocks of `LANES` rows. Lanes past the
    /// last row repeat it, and only the rows of `output` are written.
    fn batch<const LANES: usize>(
        &self,
        workspace: &mut Workspace<Value>,
        columns: &[&[Value]],
        output: &mut [Value],
    ) {
        let last = output.len().saturating_sub(1);
        let registers = workspace.prepare::<LANES>(self);
        for (start, destination) in (0_usize..).step_by(LANES).zip(output.chunks_mut(LANES)) {
            for (slots, column) in registers.iter_mut().zip(columns) {
                for (lane, slot) in slots.iter_mut().enumerate() {
                    // SAFETY: every column has `output.len()` entries, and the
                    // row is clamped to the last of them.
                    slot.clone_from(unsafe {
                        column.get_unchecked(start.saturating_add(lane).min(last))
                    });
                }
            }
            for (row, value) in destination.iter_mut().zip(self.data.execute(registers)) {
                row.clone_from(value);
            }
        }
    }
}

/// Apply a function to the lanes of the named register blocks. With `owned`,
/// the operator receives a clone of the first lane by value, the form of the
/// arithmetic operator traits, and borrows the others.
macro_rules! lanes {
    ($registers:ident, [$first:ident $(, $input:ident)*] => owned $operator:path) => {{
        let $first = $registers.get_unchecked(*$first);
        $(let $input = $registers.get_unchecked(*$input);)*
        from_fn(|lane| $operator($first.get_unchecked(lane).clone() $(, $input.get_unchecked(lane))*))
    }};
    ($registers:ident, [$($input:ident),+] => $kernel:expr) => {{
        $(let $input = $registers.get_unchecked(*$input);)+
        from_fn(|lane| $kernel($($input.get_unchecked(lane)),+))
    }};
}

/// Fold the pooled operands of `$range` from left to right, one register
/// block at a time. Each step moves every accumulated lane out exactly once;
/// a panicking operation leaks the unread lanes.
macro_rules! fold {
    ($registers:ident, $operands:expr, $range:ident, $operation:path) => {{
        let operands = $operands.get_unchecked($range.clone());
        let mut folded =
            ManuallyDrop::new($registers.get_unchecked(*operands.get_unchecked(0)).clone());
        for source in operands.get_unchecked(1..) {
            let values = $registers.get_unchecked(*source);
            folded = ManuallyDrop::new(from_fn(|lane| {
                $operation(read(folded.get_unchecked(lane)), values.get_unchecked(lane))
            }));
        }
        ManuallyDrop::into_inner(folded)
    }};
}

/// Sum the products of the pooled operand pairs of `$range` from left to
/// right, moving the accumulated lanes as the sum fold does.
macro_rules! dot {
    ($registers:ident, $operands:expr, $range:ident) => {{
        let (pairs, _) = $operands.get_unchecked($range.clone()).as_chunks::<2>();
        let [left, right] = pairs.get_unchecked(0);
        let mut total = ManuallyDrop::new(lanes!($registers, [left, right] => owned Mul::mul));
        for [left, right] in pairs.get_unchecked(1..) {
            let left = $registers.get_unchecked(*left);
            let right = $registers.get_unchecked(*right);
            total = ManuallyDrop::new(from_fn(|lane| {
                Add::add(
                    read(total.get_unchecked(lane)),
                    Mul::mul(left.get_unchecked(lane).clone(), right.get_unchecked(lane)),
                )
            }));
        }
        ManuallyDrop::into_inner(total)
    }};
}

impl<Value: Scalar> ProgramData<Value> {
    /// Execute every instruction on `LANES` independent rows, one register
    /// block per slot, and return the result block.
    ///
    /// Products; sums, products, and dot products of pooled operands; and the
    /// binary step of a polynomial (a fused multiply-add when
    /// [`Scalar::FUSE`] holds, a sum otherwise) run inline; every other
    /// operation runs in [`Self::operate`]. Three operation arms and a
    /// separate branch on the fold kind lower to short chains of conditional
    /// branches, which the predictor resolves from the preceding opcodes far
    /// more often than one indirect jump over every operation.
    fn execute<'registers, const LANES: usize>(
        &self,
        registers: &'registers mut [[Value; LANES]],
    ) -> &'registers [Value; LANES] {
        // SAFETY: `validate` proves every pooled range in bounds with at
        // least three operands, or at least two whole pairs for a dot
        // product, and every input and output below `register_count`;
        // `Workspace::prepare` returns exactly `register_count` blocks, and
        // instructions run in program order, so every input of `operate` is
        // written. `from_fn` passes lanes below `LANES`. Every lane reads its
        // inputs before the output block is stored, including a reused dying
        // input. A fold reads each lane of its `ManuallyDrop` accumulator once
        // and then replaces the accumulator without dropping it. Each arm
        // stores its own output block, so a block held in vector registers is
        // not first assembled in a shared stack slot.
        // The dispatch is a chain of discriminant tests, not a `match` over
        // every operation, which LLVM would lower to one jump table.
        unsafe {
            for instruction in &self.instructions {
                let output = instruction.output;
                let operation = &instruction.operation;
                if let Operation::Multiply([left, right]) = operation {
                    *registers.get_unchecked_mut(output) =
                        lanes!(registers, [left, right] => owned Mul::mul);
                } else if let Operation::Fold(kind, range) = operation {
                    match kind {
                        Fold::Sum => {
                            *registers.get_unchecked_mut(output) =
                                fold!(registers, self.operands, range, Add::add);
                        }
                        Fold::Product => {
                            *registers.get_unchecked_mut(output) =
                                fold!(registers, self.operands, range, Mul::mul);
                        }
                        Fold::Dot => {
                            *registers.get_unchecked_mut(output) =
                                dot!(registers, self.operands, range);
                        }
                    }
                } else if Value::FUSE
                    && let Operation::Fused([left, right, addend]) = operation
                {
                    *registers.get_unchecked_mut(output) =
                        lanes!(registers, [left, right, addend] => Value::fused);
                } else if !Value::FUSE
                    && let Operation::Add([left, right]) = operation
                {
                    *registers.get_unchecked_mut(output) =
                        lanes!(registers, [left, right] => owned Add::add);
                } else {
                    self.operate(instruction, registers);
                }
            }
            registers.get_unchecked(self.result)
        }
    }

    /// Execute one instruction of this program on `LANES` rows and store its
    /// output block; a paired function also stores its second output.
    ///
    /// # Safety
    /// `instruction` belongs to this validated program, `registers` holds its
    /// `register_count` blocks, and every input of `instruction` has been
    /// written.
    #[inline(never)]
    unsafe fn operate<const LANES: usize>(
        &self,
        instruction: &Instruction<Value>,
        registers: &mut [[Value; LANES]],
    ) {
        // SAFETY: by the caller's contract and `validate`, every pooled range
        // lies in the operand pool with at least three operands, or at least
        // two whole pairs for a dot product, every index is below
        // `register_count`, and a joint output differs from the output and the
        // input. `from_fn` passes lanes below `LANES`.
        unsafe {
            *registers.get_unchecked_mut(instruction.output) = match &instruction.operation {
                Operation::Add([left, right]) => lanes!(registers, [left, right] => owned Add::add),
                Operation::Subtract([left, right]) => {
                    lanes!(registers, [left, right] => owned Sub::sub)
                }
                Operation::Negate([input]) => lanes!(registers, [input] => owned Neg::neg),
                Operation::Multiply([left, right]) => {
                    lanes!(registers, [left, right] => owned Mul::mul)
                }
                Operation::NegatedMultiply([left, right]) => {
                    lanes!(registers, [left, right] => owned Mul::mul).map(Neg::neg)
                }
                Operation::Divide([left, right]) => {
                    lanes!(registers, [left, right] => owned Div::div)
                }
                Operation::Fold(Fold::Sum, range) => {
                    fold!(registers, self.operands, range, Add::add)
                }
                Operation::Fold(Fold::Product, range) => {
                    fold!(registers, self.operands, range, Mul::mul)
                }
                Operation::Fold(Fold::Dot, range) => dot!(registers, self.operands, range),
                Operation::Power([base, exponent]) => {
                    lanes!(registers, [base, exponent] => Value::power)
                }
                Operation::Unary(_, kernel, [input]) => lanes!(registers, [input] => kernel),
                Operation::Pair(_, kernel, [input], second) => {
                    let [arguments, cosines] =
                        registers.get_disjoint_unchecked_mut([*input, *second]);
                    from_fn(|lane| {
                        let (sine, cosine) = kernel(arguments.get_unchecked(lane));
                        *cosines.get_unchecked_mut(lane) = cosine;
                        sine
                    })
                }
                Operation::Fused([left, right, addend]) => {
                    lanes!(registers, [left, right, addend] => Value::fused)
                }
                Operation::FusedSubtract([left, right, subtrahend]) => {
                    lanes!(registers, [left, right, subtrahend] => |x, y, z: &Value| {
                        Value::fused(x, y, &z.clone().neg())
                    })
                }
                Operation::NegatedFusedAdd([left, right, addend]) => {
                    lanes!(registers, [left, right, addend] => |x: &Value, y, z| {
                        Value::fused(&x.clone().neg(), y, z)
                    })
                }
                Operation::NegatedFusedSubtract([left, right, subtrahend]) => {
                    lanes!(registers, [left, right, subtrahend] => |x: &Value, y, z: &Value| {
                        Value::fused(&x.clone().neg(), y, &z.clone().neg())
                    })
                }
                Operation::Binary(_, kernel, [left, right]) => {
                    lanes!(registers, [left, right] => kernel)
                }
                Operation::Ternary(_, kernel, [first, second, third]) => {
                    lanes!(registers, [first, second, third] => kernel)
                }
            };
        }
    }
}
