//! Shared reciprocals, native signs, and single-use arithmetic fusion.

use core::mem::replace;

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use crate::{collections::HashMap, core::Scalar, errors::EvalError};

use super::{Compilation, Operation, Source};

impl<Value: Scalar> Compilation<Value> {
    /// Replace repeated \(a/d\) operations by \(a(1/d)\) in native floating plans.
    /// Floating rounding and intermediate overflow follow the transformed plan.
    #[expect(
        clippy::indexing_slicing,
        reason = "SSA inputs refer to completed earlier producers and remapping appends one entry for every original source"
    )]
    pub fn reuse_reciprocals(&mut self) -> Result<(), EvalError> {
        if self
            .sources
            .iter()
            .filter(|source| matches!(source, Source::Computed(Operation::Divide(_))))
            .take(2)
            .count()
            < 2
        {
            return Ok(());
        }
        let mut denominators = HashMap::new();
        for source in &self.sources {
            if let Source::Computed(Operation::Divide([_, denominator])) = source {
                denominators
                    .entry(*denominator)
                    .and_modify(|shared| *shared = true)
                    .or_insert(false);
            }
        }
        let shared = denominators.values().filter(|multiple| **multiple).count();
        if shared == 0 {
            return Ok(());
        }
        let capacity = self
            .sources
            .len()
            .checked_add(shared)
            .and_then(|count| count.checked_add(1))
            .ok_or(EvalError::ProgramTooLarge)?;
        let one = Number::one();
        let unit = Value::literal(&one).ok_or(EvalError::UnsupportedLiteral { value: one })?;
        let previous = replace(&mut self.sources, Vec::with_capacity(capacity));
        let mut remapped = Vec::with_capacity(previous.len());
        let mut reciprocals = HashMap::new();
        let constant = self.constant(unit);
        self.sources.push(constant);
        for mut source in previous {
            let shared_denominator = match &source {
                Source::Computed(Operation::Divide([_, denominator]))
                    if denominators.get(denominator) == Some(&true) =>
                {
                    Some(*denominator)
                }
                Source::Parameter(_)
                | Source::Constant(_)
                | Source::Computed(_)
                | Source::Projection(_) => None,
            };
            if let Source::Computed(operation) = &mut source {
                for input in operation.inputs_mut(&mut self.program.operands) {
                    *input = remapped[*input];
                }
                if let (Some(original), Operation::Divide([numerator, denominator])) =
                    (shared_denominator, &*operation)
                {
                    let reciprocal = *reciprocals.entry(original).or_insert_with(|| {
                        let result = self.sources.len();
                        self.sources
                            .push(Source::Computed(Operation::Divide([0, *denominator])));
                        result
                    });
                    *operation = Operation::Multiply([*numerator, reciprocal]);
                }
            }
            remapped.push(self.sources.len());
            self.sources.push(source);
        }
        self.program.result = remapped[self.program.result];
        Ok(())
    }

    #[expect(
        clippy::indexing_slicing,
        clippy::too_many_lines,
        reason = "SSA operands index completed producers, constants index their literal table, and use counts have one entry per source"
    )]
    /// Lower \(-x\), \(a-b\), and signed multiply-add forms in one SSA pass.
    /// Fusion consumes only single-use producers; shared products remain shared.
    pub fn optimize_arithmetic(&mut self) {
        let mut uses = vec![0_usize; self.sources.len()];
        for source in &self.sources {
            if let Source::Computed(operation) = source {
                for input in operation.inputs(&self.program.operands) {
                    uses[*input] = uses[*input].saturating_add(1);
                }
            }
        }
        uses[self.program.result] = uses[self.program.result].saturating_add(1);
        for index in 0..self.sources.len() {
            let negative_one = |source: usize| {
                matches!(self.sources[source], Source::Constant(constant)
                    if Value::is_negative_one(&self.constants[constant]))
            };
            let replacement = match &self.sources[index] {
                Source::Computed(Operation::Multiply([left, right])) => {
                    if negative_one(*left) {
                        Some(self.negated(*right, &uses))
                    } else if negative_one(*right) {
                        Some(self.negated(*left, &uses))
                    } else {
                        None
                    }
                }
                Source::Computed(Operation::MultiplyMany(range)) => {
                    let inputs = &self.program.operands[range.clone()];
                    if let [first, rest @ ..] = inputs
                        && negative_one(*first)
                    {
                        Some(if let [left, right] = rest {
                            Operation::NegatedMultiply([*left, *right])
                        } else {
                            // A valid pooled product has at least three operands.
                            // Removing its first operand leaves this bounded suffix.
                            Operation::NegatedMultiplyMany(range.start.saturating_add(1)..range.end)
                        })
                    } else {
                        None
                    }
                }
                Source::Computed(Operation::Add([left, right])) => {
                    let fused = Value::FUSE
                        .then(|| {
                            [(*left, *right), (*right, *left)].into_iter().find_map(
                                |(product, addend)| {
                                    if uses[product] != 1 {
                                        return None;
                                    }
                                    let (inputs, negative) = match &self.sources[product] {
                                        Source::Computed(Operation::Multiply(inputs)) => {
                                            (*inputs, false)
                                        }
                                        Source::Computed(Operation::NegatedMultiply(inputs)) => {
                                            (*inputs, true)
                                        }
                                        Source::Parameter(_)
                                        | Source::Constant(_)
                                        | Source::Computed(_)
                                        | Source::Projection(_) => return None,
                                    };
                                    let (third, subtract) = match self.sources[addend] {
                                        Source::Computed(Operation::Negate([input]))
                                            if uses[addend] == 1 =>
                                        {
                                            (input, true)
                                        }
                                        Source::Parameter(_)
                                        | Source::Constant(_)
                                        | Source::Computed(_)
                                        | Source::Projection(_) => (addend, false),
                                    };
                                    let [first, second] = inputs;
                                    let arguments = [first, second, third];
                                    Some(match (negative, subtract) {
                                        (false, false) => Operation::Fused(arguments),
                                        (false, true) => Operation::FusedSubtract(arguments),
                                        (true, false) => Operation::NegatedFusedAdd(arguments),
                                        (true, true) => Operation::NegatedFusedSubtract(arguments),
                                    })
                                },
                            )
                        })
                        .flatten();
                    fused.or_else(|| match (&self.sources[*left], &self.sources[*right]) {
                        (_, Source::Computed(Operation::Negate([input]))) if uses[*right] == 1 => {
                            Some(Operation::Subtract([*left, *input]))
                        }
                        (Source::Computed(Operation::Negate([input])), _) if uses[*left] == 1 => {
                            Some(Operation::Subtract([*right, *input]))
                        }
                        _ => None,
                    })
                }
                Source::Computed(Operation::Fused([left, right, addend])) => {
                    if negative_one(*left) {
                        Some(Operation::Subtract([*addend, *right]))
                    } else if negative_one(*right) {
                        Some(Operation::Subtract([*addend, *left]))
                    } else {
                        None
                    }
                }
                Source::Parameter(_)
                | Source::Constant(_)
                | Source::Computed(_)
                | Source::Projection(_) => None,
            };
            if let Some(operation) = replacement {
                self.sources[index] = Source::Computed(operation);
            }
        }
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "the input is a completed SSA producer and use counts cover the same source table"
    )]
    fn negated(&self, input: usize, uses: &[usize]) -> Operation<Value> {
        if uses[input] == 1 {
            match &self.sources[input] {
                Source::Computed(Operation::Multiply(inputs)) => {
                    return Operation::NegatedMultiply(*inputs);
                }
                Source::Computed(Operation::NegatedMultiply(inputs)) => {
                    return Operation::Multiply(*inputs);
                }
                Source::Computed(Operation::Fused(inputs)) => {
                    return Operation::NegatedFusedSubtract(*inputs);
                }
                Source::Computed(Operation::FusedSubtract(inputs)) => {
                    return Operation::NegatedFusedAdd(*inputs);
                }
                Source::Computed(Operation::NegatedFusedAdd(inputs)) => {
                    return Operation::FusedSubtract(*inputs);
                }
                Source::Computed(Operation::NegatedFusedSubtract(inputs)) => {
                    return Operation::Fused(*inputs);
                }
                Source::Parameter(_)
                | Source::Constant(_)
                | Source::Computed(_)
                | Source::Projection(_) => {}
            }
        }
        Operation::Negate([input])
    }
}
