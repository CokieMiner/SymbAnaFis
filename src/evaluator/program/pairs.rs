//! Joint function lowering for identical completed scalar arguments.

use core::mem::take;

use alloc::{vec, vec::Vec};

use crate::{
    collections::HashMap,
    core::{BuiltinFunction, Scalar},
};

use super::{Compilation, Operation, PairedFunction, PairedKernel, Source};

impl<Value: Scalar> Compilation<Value> {
    #[expect(
        clippy::indexing_slicing,
        reason = "each table has one slot per original SSA source, and operands reference completed earlier sources"
    )]
    pub fn pair_functions(&mut self) {
        let mut groups = HashMap::new();
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
        for ((_, hyperbolic), (sine_slot, cosine_slot)) in groups {
            if let (Some(sine), Some(cosine)) = (sine_slot, cosine_slot) {
                pairs[sine.min(cosine)] = Some((sine, cosine, hyperbolic));
            }
        }
        let previous = take(&mut self.sources);
        self.sources = Vec::with_capacity(previous.len());
        let mut remapped = vec![0; previous.len()];
        let mut consumed = vec![false; previous.len()];
        for (index, mut source) in previous.into_iter().enumerate() {
            if consumed[index] {
                continue;
            }
            if let (
                Some((sine, cosine, hyperbolic)),
                Source::Computed(Operation::Unary(_, _, [input])),
            ) = (pairs[index], &source)
            {
                let producer = self.sources.len();
                // Two original unary sources are replaced by a producer and
                // its projection, so the original source capacity suffices.
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "the producer and projection replace two existing SSA sources within the original vector capacity"
                )]
                let projection = producer + 1;
                let (function, kernel): (_, PairedKernel<Value>) = if hyperbolic {
                    (PairedFunction::SinhCosh, Value::sinh_cosh)
                } else {
                    (PairedFunction::SinCos, Value::sin_cos)
                };
                self.sources.push(Source::Computed(Operation::Pair(
                    function,
                    kernel,
                    [remapped[*input]],
                    projection,
                )));
                self.sources.push(Source::Projection(producer));
                remapped[sine] = producer;
                remapped[cosine] = projection;
                consumed[sine] = true;
                consumed[cosine] = true;
                continue;
            }
            if let Source::Computed(operation) = &mut source {
                for input in operation.inputs_mut(&mut self.program.operands) {
                    *input = remapped[*input];
                }
            }
            remapped[index] = self.sources.len();
            self.sources.push(source);
        }
        self.program.result = remapped[self.program.result];
    }
}
