//! In-place SSA arithmetic rewrites for native floating plans: native signs,
//! single-use arithmetic fusion, and dot products.
//!
//! Every pass reads sources, constants, pooled operands, and use counts by
//! index without bounds checks; the source invariants of [`Compilation`]
//! bound every such index.

#![expect(
    unsafe_code,
    reason = "SSA source indices and pooled ranges are bounded by construction; see the Compilation invariants"
)]

use alloc::{vec, vec::Vec};

use crate::errors::EvalError;

use super::{
    Combine, Compilation, EvaluationValue, Fold, Fused, OperandSpan, Operation, ProgramData, Source,
};

/// The fewest operands of a sum lowered to a dot product.
const DOT_LENGTH: usize = 8;

impl<Value: EvaluationValue> Compilation<Value> {
    /// Lower $-x$, $a-b$, signed multiply-add forms, and sums of products
    /// in one SSA pass. Fusion and dot products consume only single-use
    /// producers; shared products remain shared.
    ///
    /// # Errors
    /// Returns [`EvalError::UnsupportedLiteral`] if the scalar cannot represent
    /// the unit that pairs plain terms of a dot product, or
    /// [`EvalError::ProgramTooLarge`] if pooled operands exceed `u32::MAX`.
    pub fn optimize_arithmetic(&mut self) -> Result<(), EvalError> {
        let unit = self.coefficient(None)?;
        let fuse = ProgramData::<Value>::fuse();
        let mut pairs = Vec::new();
        let mut uses = vec![0_u32; self.sources.len()];
        // SAFETY: every input of a source, every constant source, every pooled
        // span, the unit, and the results index their tables by the source
        // invariants; `uses` holds one count per source, and sources are only
        // replaced in place, keeping their inputs earlier sources.
        unsafe {
            for &result in &self.results {
                let idx = usize::try_from(result).unwrap_or(0);
                let count = uses.get_unchecked_mut(idx);
                *count = count.saturating_add(1);
            }
            for (index, source) in self.sources.iter().enumerate().rev() {
                if let Source::Computed(operation) = source
                    && *uses.get_unchecked(index) != 0
                {
                    for &input in operation.inputs(&self.operands) {
                        let input_idx = usize::try_from(input).unwrap_or(0);
                        let count = uses.get_unchecked_mut(input_idx);
                        *count = count.saturating_add(1);
                    }
                }
            }
            for index in 0..self.sources.len() {
                if *uses.get_unchecked(index) == 0 {
                    continue;
                }
                let negative_one = |source: u32| {
                    let src_idx = usize::try_from(source).unwrap_or(0);
                    matches!(*self.sources.get_unchecked(src_idx), Source::Constant(constant)
                    if Value::is_negative_one(
                        self.constants.get_unchecked(usize::try_from(constant).unwrap_or(0))
                    ))
                };
                let replacement = match *self.sources.get_unchecked(index) {
                    Source::Computed(Operation::Combine(Combine::Multiply, [left, right])) => {
                        if negative_one(left) {
                            Some(self.negated(right, &uses))
                        } else if negative_one(right) {
                            Some(self.negated(left, &uses))
                        } else {
                            None
                        }
                    }
                    Source::Computed(Operation::Fold(Fold::Product, span)) => {
                        let start = usize::try_from(span.start).unwrap_or(0);
                        let end = usize::try_from(span.end).unwrap_or(0);
                        match self.operands.get_unchecked(start..end) {
                            [first, left, right] if negative_one(*first) => {
                                Some(Operation::NegatedMultiply([*left, *right]))
                            }
                            _ => None,
                        }
                    }
                    Source::Computed(Operation::Combine(Combine::Add, [left, right])) => {
                        self.binary_sum(left, right, &uses, fuse)
                    }
                    Source::Computed(Operation::Fold(Fold::Sum, span)) => {
                        let start = usize::try_from(span.start).unwrap_or(0);
                        let end = usize::try_from(span.end).unwrap_or(0);
                        let dot = self.dot(
                            self.operands.get_unchecked(start..end),
                            &uses,
                            unit,
                            &mut pairs,
                        )?;
                        self.operands.append(&mut pairs);
                        dot
                    }
                    Source::Computed(Operation::Fused(fused, [left, right, addend]))
                        if !fused.negate_product && !fused.negate_addend =>
                    {
                        if negative_one(left) {
                            Some(Operation::Combine(Combine::Subtract, [addend, right]))
                        } else if negative_one(right) {
                            Some(Operation::Combine(Combine::Subtract, [addend, left]))
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

    /// The fused or subtracting form of $l + r$.
    ///
    /// # Safety
    /// `left` and `right` are sources, and `uses` holds one count per source.
    unsafe fn binary_sum(
        &mut self,
        left: u32,
        right: u32,
        uses: &[u32],
        fuse: bool,
    ) -> Option<Operation> {
        // SAFETY: the caller's contract; inputs of a source are earlier sources.
        unsafe {
            let single =
                |source: u32| *uses.get_unchecked(usize::try_from(source).unwrap_or(0)) == 1;
            let fused = fuse
                .then(|| {
                    [(left, right), (right, left)]
                        .into_iter()
                        .find_map(|(product, addend)| {
                            if !single(product) {
                                return None;
                            }
                            let prod_idx = usize::try_from(product).unwrap_or(0);
                            let ([first, second], negative) =
                                match *self.sources.get_unchecked(prod_idx) {
                                    Source::Computed(Operation::Combine(
                                        Combine::Multiply,
                                        inputs,
                                    )) => (inputs, false),
                                    Source::Computed(Operation::NegatedMultiply(inputs)) => {
                                        (inputs, true)
                                    }
                                    Source::Parameter(_)
                                    | Source::Constant(_)
                                    | Source::Computed(_)
                                    | Source::Projection(_) => return None,
                                };
                            let add_idx = usize::try_from(addend).unwrap_or(0);
                            let (third, subtract, unnegate) = match *self
                                .sources
                                .get_unchecked(add_idx)
                            {
                                Source::Computed(Operation::Negate([input])) if single(addend) => {
                                    (input, true, None)
                                }
                                Source::Computed(Operation::NegatedMultiply(inputs))
                                    if single(addend) =>
                                {
                                    (addend, true, Some(inputs))
                                }
                                Source::Parameter(_)
                                | Source::Constant(_)
                                | Source::Computed(_)
                                | Source::Projection(_) => (addend, false, None),
                            };
                            if let Some(factors) = unnegate {
                                *self.sources.get_unchecked_mut(add_idx) = Source::Computed(
                                    Operation::Combine(Combine::Multiply, factors),
                                );
                            }
                            Some(Operation::Fused(
                                Fused {
                                    negate_product: negative,
                                    negate_addend: subtract,
                                },
                                [first, second, third],
                            ))
                        })
                })
                .flatten();
            fused.or_else(|| {
                [(left, right), (right, left)]
                    .into_iter()
                    .find_map(|(minuend, subtrahend)| {
                        if !single(subtrahend) {
                            return None;
                        }
                        let sub_idx = usize::try_from(subtrahend).unwrap_or(0);
                        let unnegate = match *self.sources.get_unchecked(sub_idx) {
                            Source::Computed(Operation::Negate([input])) => {
                                return Some(Operation::Combine(
                                    Combine::Subtract,
                                    [minuend, input],
                                ));
                            }
                            Source::Computed(Operation::NegatedMultiply(inputs)) => inputs,
                            Source::Parameter(_)
                            | Source::Constant(_)
                            | Source::Computed(_)
                            | Source::Projection(_) => return None,
                        };
                        *self.sources.get_unchecked_mut(sub_idx) =
                            Source::Computed(Operation::Combine(Combine::Multiply, unnegate));
                        Some(Operation::Combine(Combine::Subtract, [minuend, subtrahend]))
                    })
            })
        }
    }

    /// The dot product equal to the left-to-right sum of `operands`.
    ///
    /// # Safety
    /// Every operand is a source, and `uses` holds one count per source.
    unsafe fn dot(
        &self,
        operands: &[u32],
        uses: &[u32],
        unit: u32,
        pairs: &mut Vec<u32>,
    ) -> Result<Option<Operation>, EvalError> {
        if operands.len() < DOT_LENGTH {
            return Ok(None);
        }
        let mut products = 0_usize;
        for &operand in operands {
            let op_idx = usize::try_from(operand).unwrap_or(0);
            // SAFETY: the caller's contract.
            let (source, count) = unsafe {
                (
                    *self.sources.get_unchecked(op_idx),
                    *uses.get_unchecked(op_idx),
                )
            };
            if let Source::Computed(Operation::Combine(Combine::Multiply, [left, right])) = source
                && count == 1
            {
                pairs.extend([left, right]);
                products = products.saturating_add(1);
            } else {
                pairs.extend([operand, unit]);
            }
        }
        if products.saturating_mul(2) < operands.len() {
            pairs.clear();
            return Ok(None);
        }
        let start =
            u32::try_from(self.operands.len()).map_err(|_overflow| EvalError::ProgramTooLarge)?;
        let end = self
            .operands
            .len()
            .checked_add(pairs.len())
            .and_then(|total| u32::try_from(total).ok())
            .ok_or(EvalError::ProgramTooLarge)?;
        Ok(Some(Operation::Fold(Fold::Dot, OperandSpan { start, end })))
    }

    /// The negation of `input`, folded into a single-use product or fused operation.
    ///
    /// # Safety
    /// `input` is a source, and `uses` holds one count per source.
    unsafe fn negated(&self, input: u32, uses: &[u32]) -> Operation {
        let idx = usize::try_from(input).unwrap_or(0);
        // SAFETY: the caller's contract.
        let (source, count) =
            unsafe { (*self.sources.get_unchecked(idx), *uses.get_unchecked(idx)) };
        if count == 1 {
            match source {
                Source::Computed(Operation::Combine(Combine::Multiply, inputs)) => {
                    return Operation::NegatedMultiply(inputs);
                }
                Source::Computed(Operation::Combine(Combine::Subtract, [left, right])) => {
                    return Operation::Combine(Combine::Subtract, [right, left]);
                }
                Source::Computed(Operation::NegatedMultiply(inputs)) => {
                    return Operation::Combine(Combine::Multiply, inputs);
                }
                Source::Computed(Operation::Fused(fused, inputs)) => {
                    return Operation::Fused(
                        Fused {
                            negate_product: !fused.negate_product,
                            negate_addend: !fused.negate_addend,
                        },
                        inputs,
                    );
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
