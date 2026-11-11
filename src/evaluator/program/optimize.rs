//! SSA rewrites between lowering and register allocation: shared
//! reciprocals, native signs, single-use arithmetic fusion, and dot products
//! in native floating plans, and joint sine and cosine evaluation in every
//! plan.
//!
//! Every pass reads sources, constants, pooled operands, and use counts by
//! index without bounds checks; the source invariants of [`Compilation`]
//! bound every such index.

#![expect(
    unsafe_code,
    reason = "SSA source indices and pooled ranges are bounded by construction; see the Compilation invariants"
)]

use core::mem::{replace, take};

use alloc::{vec, vec::Vec};

use crate::{
    collections::FixedMap,
    core::{BuiltinFunction, Scalar},
    errors::EvalError,
};

use super::{Compilation, Fold, Operation, PairedFunction, PairedKernel, Source};

/// The fewest operands of a sum lowered to a dot product. A dot product saves
/// one dispatch per absorbed product, but its loop exit is no longer preceded
/// by those dispatches, from which the branch predictor infers the length of
/// a sum; shorter sums save less than the mispredicted exit costs.
const DOT_LENGTH: usize = 8;

impl<Value: Scalar> Compilation<Value> {
    /// Replace repeated \(a/d\) operations by \(a(1/d)\) in native floating plans.
    /// Floating rounding and intermediate overflow follow the transformed plan.
    /// Lowering records the denominators, so a plan without a shared one is
    /// not scanned.
    ///
    /// # Errors
    /// Returns [`EvalError::ProgramTooLarge`] if the sources outgrow memory
    /// and [`EvalError::UnsupportedLiteral`] if the scalar cannot represent one.
    pub fn reuse_reciprocals(&mut self) -> Result<(), EvalError> {
        // The pass renumbers the sources, so it consumes the record.
        let denominators = take(&mut self.denominators);
        let shared = denominators.values().filter(|multiple| **multiple).count();
        if shared == 0 {
            return Ok(());
        }
        let capacity = self
            .sources
            .len()
            .checked_add(shared)
            .ok_or(EvalError::ProgramTooLarge)?;
        let unit = self.coefficient(None)?;
        let previous = replace(&mut self.sources, Vec::with_capacity(capacity));
        let mut remapped = Vec::with_capacity(previous.len());
        let mut reciprocals = FixedMap::default();
        // SAFETY: `remapped` gains one entry per original source in order, and
        // every input of a source, the unit created before lowering, and the
        // result are earlier sources, so each lookup reads an entry already
        // pushed; every pooled range lies in `self.program.operands`.
        unsafe {
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
                        *input = *remapped.get_unchecked(*input);
                    }
                    if let (Some(original), Operation::Divide([numerator, denominator])) =
                        (shared_denominator, &*operation)
                    {
                        let one = *remapped.get_unchecked(unit);
                        let reciprocal = *reciprocals.entry(original).or_insert_with(|| {
                            let result = self.sources.len();
                            self.sources
                                .push(Source::Computed(Operation::Divide([one, *denominator])));
                            result
                        });
                        *operation = Operation::Multiply([*numerator, reciprocal]);
                    }
                }
                remapped.push(self.sources.len());
                self.sources.push(source);
            }
            self.program.result = *remapped.get_unchecked(self.program.result);
            self.unit = Some(*remapped.get_unchecked(unit));
        }
        Ok(())
    }

    /// Lower \(-x\), \(a-b\), signed multiply-add forms, and sums of products
    /// in one SSA pass. Fusion and dot products consume only single-use
    /// producers; shared products remain shared.
    ///
    /// Only reads by sources that reach the result count, so the sources that
    /// lowering leaves unread, such as the terms of a sum evaluated in Horner
    /// or Estrin form, neither keep a producer shared nor are rewritten. A
    /// rewrite moves the reads of a consumed producer to its consumer or drops
    /// a read of the negative unit, so the counts remain upper bounds of the
    /// live reads throughout the pass.
    ///
    /// # Errors
    /// Returns [`EvalError::UnsupportedLiteral`] if the scalar cannot represent
    /// the unit that pairs plain terms of a dot product.
    pub fn optimize_arithmetic(&mut self) -> Result<(), EvalError> {
        let unit = self.coefficient(None)?;
        let mut pairs = Vec::new();
        let mut uses = vec![0_usize; self.sources.len()];
        // SAFETY: every input of a source, every constant source, every pooled
        // range, the unit, and the result index their tables by the source
        // invariants; `uses` holds one count per source, and sources are only
        // replaced in place, keeping their inputs earlier sources.
        unsafe {
            // Every source follows its inputs, so a reverse sweep meets all
            // readers of a source before the source, and a positive count
            // marks exactly the sources that reach the result. Projections
            // appear only after this pass.
            *uses.get_unchecked_mut(self.program.result) = 1;
            for (index, source) in self.sources.iter().enumerate().rev() {
                if let Source::Computed(operation) = source
                    && *uses.get_unchecked(index) != 0
                {
                    for input in operation.inputs(&self.program.operands) {
                        let count = uses.get_unchecked_mut(*input);
                        *count = count.saturating_add(1);
                    }
                }
            }
            for index in 0..self.sources.len() {
                if *uses.get_unchecked(index) == 0 {
                    continue;
                }
                let negative_one = |source: usize| {
                    matches!(self.sources.get_unchecked(source), Source::Constant(constant)
                        if Value::is_negative_one(self.constants.get_unchecked(*constant)))
                };
                let replacement = match self.sources.get_unchecked(index) {
                    Source::Computed(Operation::Multiply([left, right])) => {
                        if negative_one(*left) {
                            Some(self.negated(*right, &uses))
                        } else if negative_one(*right) {
                            Some(self.negated(*left, &uses))
                        } else {
                            None
                        }
                    }
                    // A longer product keeps its leading negative unit: the
                    // multiplication by it is an exact negation inside the fold.
                    Source::Computed(Operation::Fold(Fold::Product, range)) => {
                        match self.program.operands.get_unchecked(range.clone()) {
                            [first, left, right] if negative_one(*first) => {
                                Some(Operation::NegatedMultiply([*left, *right]))
                            }
                            _ => None,
                        }
                    }
                    Source::Computed(Operation::Add([left, right])) => {
                        self.binary_sum(*left, *right, &uses)
                    }
                    Source::Computed(Operation::Fold(Fold::Sum, range)) => {
                        let dot = self.dot(
                            self.program.operands.get_unchecked(range.clone()),
                            &uses,
                            unit,
                            &mut pairs,
                        );
                        self.program.operands.append(&mut pairs);
                        dot
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
                    *self.sources.get_unchecked_mut(index) = Source::Computed(operation);
                }
            }
        }
        Ok(())
    }

    /// Replace a sine and a cosine, or a hyperbolic sine and cosine, of the
    /// same argument by one paired evaluation at the earlier of the two
    /// sources and a projection of its second value.
    pub fn pair_functions(&mut self) {
        let mut groups = FixedMap::default();
        for (index, source) in self.sources.iter().enumerate() {
            if let Source::Computed(Operation::Unary(function, _, [input])) = source {
                let hyperbolic = matches!(function, BuiltinFunction::Sinh | BuiltinFunction::Cosh);
                if !hyperbolic && !matches!(function, BuiltinFunction::Sin | BuiltinFunction::Cos) {
                    continue;
                }
                let cosine = matches!(function, BuiltinFunction::Cos | BuiltinFunction::Cosh);
                let (sine_slot, cosine_slot) =
                    groups.entry((*input, hyperbolic)).or_insert((None, None));
                if cosine {
                    cosine_slot.get_or_insert(index);
                } else {
                    sine_slot.get_or_insert(index);
                }
            }
        }
        if !groups
            .values()
            .any(|(sine, cosine)| sine.is_some() && cosine.is_some())
        {
            return;
        }
        let mut pairs = vec![None; self.sources.len()];
        let previous = take(&mut self.sources);
        self.sources = Vec::with_capacity(previous.len());
        let mut remapped = vec![0; previous.len()];
        let mut consumed = vec![false; previous.len()];
        // SAFETY: the sine and cosine of a group are source indices, the
        // length of `pairs`, `remapped`, and `consumed`. Every input of a
        // source is an earlier source, already remapped: a consumed source is
        // remapped when the earlier member of its pair is emitted, and a
        // paired argument precedes both members. The result and the unit are
        // sources.
        unsafe {
            for ((_, hyperbolic), (sine_slot, cosine_slot)) in groups {
                if let (Some(sine), Some(cosine)) = (sine_slot, cosine_slot) {
                    *pairs.get_unchecked_mut(sine.min(cosine)) = Some((sine, cosine, hyperbolic));
                }
            }
            for ((index, mut source), pair) in previous.into_iter().enumerate().zip(pairs) {
                if *consumed.get_unchecked(index) {
                    continue;
                }
                if let (
                    Some((sine, cosine, hyperbolic)),
                    Source::Computed(Operation::Unary(_, _, [input])),
                ) = (pair, &source)
                {
                    let producer = self.sources.len();
                    // The producer and its projection replace two sources, so
                    // the projection index is below the original source count.
                    let projection = producer.saturating_add(1);
                    let (function, kernel): (_, PairedKernel<Value>) = if hyperbolic {
                        (PairedFunction::SinhCosh, Value::sinh_cosh)
                    } else {
                        (PairedFunction::SinCos, Value::sin_cos)
                    };
                    self.sources.push(Source::Computed(Operation::Pair(
                        function,
                        kernel,
                        [*remapped.get_unchecked(*input)],
                        projection,
                    )));
                    self.sources.push(Source::Projection(producer));
                    *remapped.get_unchecked_mut(sine) = producer;
                    *remapped.get_unchecked_mut(cosine) = projection;
                    *consumed.get_unchecked_mut(sine) = true;
                    *consumed.get_unchecked_mut(cosine) = true;
                    continue;
                }
                if let Source::Computed(operation) = &mut source {
                    for input in operation.inputs_mut(&mut self.program.operands) {
                        *input = *remapped.get_unchecked(*input);
                    }
                }
                *remapped.get_unchecked_mut(index) = self.sources.len();
                self.sources.push(source);
            }
            self.program.result = *remapped.get_unchecked(self.program.result);
            self.unit = self.unit.map(|unit| *remapped.get_unchecked(unit));
        }
    }

    /// The fused or subtracting form of \(l + r\): a single-use product
    /// operand fuses with the other operand when [`Scalar::FUSE`] holds, and a
    /// single-use negation operand turns the sum into a difference.
    ///
    /// # Safety
    /// `left` and `right` are sources, and `uses` holds one count per source.
    unsafe fn binary_sum(
        &self,
        left: usize,
        right: usize,
        uses: &[usize],
    ) -> Option<Operation<Value>> {
        // SAFETY: the caller's contract; the inputs of a source are earlier
        // sources.
        unsafe {
            let single = |source: usize| *uses.get_unchecked(source) == 1;
            let fused = Value::FUSE
                .then(|| {
                    [(left, right), (right, left)]
                        .into_iter()
                        .find_map(|(product, addend)| {
                            if !single(product) {
                                return None;
                            }
                            let ([first, second], negative) = match self
                                .sources
                                .get_unchecked(product)
                            {
                                Source::Computed(Operation::Multiply(inputs)) => (*inputs, false),
                                Source::Computed(Operation::NegatedMultiply(inputs)) => {
                                    (*inputs, true)
                                }
                                Source::Parameter(_)
                                | Source::Constant(_)
                                | Source::Computed(_)
                                | Source::Projection(_) => return None,
                            };
                            let (third, subtract) = match self.sources.get_unchecked(addend) {
                                Source::Computed(Operation::Negate([input])) if single(addend) => {
                                    (*input, true)
                                }
                                Source::Parameter(_)
                                | Source::Constant(_)
                                | Source::Computed(_)
                                | Source::Projection(_) => (addend, false),
                            };
                            let arguments = [first, second, third];
                            Some(match (negative, subtract) {
                                (false, false) => Operation::Fused(arguments),
                                (false, true) => Operation::FusedSubtract(arguments),
                                (true, false) => Operation::NegatedFusedAdd(arguments),
                                (true, true) => Operation::NegatedFusedSubtract(arguments),
                            })
                        })
                })
                .flatten();
            fused.or_else(|| {
                match (
                    self.sources.get_unchecked(left),
                    self.sources.get_unchecked(right),
                ) {
                    (_, Source::Computed(Operation::Negate([input]))) if single(right) => {
                        Some(Operation::Subtract([left, *input]))
                    }
                    (Source::Computed(Operation::Negate([input])), _) if single(left) => {
                        Some(Operation::Subtract([right, *input]))
                    }
                    _ => None,
                }
            })
        }
    }

    /// The dot product equal to the left-to-right sum of `operands`: every
    /// single-use binary product contributes its factor pair, and every other
    /// operand is paired with `unit`, whose product it equals exactly. It is
    /// returned for sums of at least [`DOT_LENGTH`] operands of which such
    /// products are at least half, with its pairs left in `pairs` for the
    /// caller to append to the pool.
    ///
    /// # Safety
    /// Every operand is a source, and `uses` holds one count per source.
    unsafe fn dot(
        &self,
        operands: &[usize],
        uses: &[usize],
        unit: usize,
        pairs: &mut Vec<usize>,
    ) -> Option<Operation<Value>> {
        if operands.len() < DOT_LENGTH {
            return None;
        }
        let mut products = 0_usize;
        for &operand in operands {
            // SAFETY: the caller's contract.
            let (source, count) = unsafe {
                (
                    self.sources.get_unchecked(operand),
                    *uses.get_unchecked(operand),
                )
            };
            if let Source::Computed(Operation::Multiply([left, right])) = source
                && count == 1
            {
                pairs.extend([*left, *right]);
                products = products.saturating_add(1);
            } else {
                pairs.extend([operand, unit]);
            }
        }
        if products.saturating_mul(2) < operands.len() {
            pairs.clear();
            return None;
        }
        let start = self.program.operands.len();
        Some(Operation::Fold(
            Fold::Dot,
            start..start.saturating_add(pairs.len()),
        ))
    }

    /// The negation of `input`, folded into a single-use product or fused
    /// operation when one produces it.
    ///
    /// # Safety
    /// `input` is a source, and `uses` holds one count per source.
    unsafe fn negated(&self, input: usize, uses: &[usize]) -> Operation<Value> {
        // SAFETY: the caller's contract.
        let (source, count) = unsafe {
            (
                self.sources.get_unchecked(input),
                *uses.get_unchecked(input),
            )
        };
        if count == 1 {
            match source {
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
