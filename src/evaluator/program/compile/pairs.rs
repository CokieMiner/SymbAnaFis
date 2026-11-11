//! Source-renumbering SSA passes: shared denominator reciprocals in native
//! floating plans and joint `sin`/`cos` and `sinh`/`cosh` pairing in all plans.

#![expect(
    unsafe_code,
    reason = "SSA source indices and pooled ranges are bounded by construction; see the Compilation invariants"
)]

use core::mem::{replace, take};

use alloc::{vec, vec::Vec};

use crate::{collections::FixedMap, core::BuiltinFunction, errors::EvalError};

use super::{Combine, Compilation, EvaluationValue, Operation, PairedFunction, Source};

impl<Value: EvaluationValue> Compilation<Value> {
    /// Mark all sources reachable from `self.results` in one reverse sweep.
    pub fn reachable_sources(&self) -> Vec<bool> {
        let mut reached = vec![false; self.sources.len()];
        // SAFETY: every result, every input of a computed source, every projection
        // producer, and every pooled range satisfy `Compilation`'s source invariants.
        unsafe {
            for &result in &self.results {
                let idx = usize::try_from(result).unwrap_or(0);
                *reached.get_unchecked_mut(idx) = true;
            }
            for (index, source) in self.sources.iter().enumerate().rev() {
                if !*reached.get_unchecked(index) {
                    continue;
                }
                match source {
                    Source::Computed(operation) => {
                        for &input in operation.inputs(&self.operands) {
                            let in_idx = usize::try_from(input).unwrap_or(0);
                            *reached.get_unchecked_mut(in_idx) = true;
                        }
                    }
                    Source::Projection(producer) => {
                        let prod_idx = usize::try_from(*producer).unwrap_or(0);
                        *reached.get_unchecked_mut(prod_idx) = true;
                    }
                    Source::Parameter(_) | Source::Constant(_) => {}
                }
            }
        }
        reached
    }

    /// Replace repeated reachable $a/d$ operations by $a(1/d)$ in native floating plans.
    pub fn reuse_reciprocals(&mut self) -> Result<(), EvalError> {
        let lowered_denominators = take(&mut self.denominators);
        if !lowered_denominators.values().any(|multiple| *multiple) {
            return Ok(());
        }
        let reached = self.reachable_sources();
        let mut live_denominators: FixedMap<u32, bool> = FixedMap::default();
        // SAFETY: `reached` has one flag per source.
        unsafe {
            for (index, source) in self.sources.iter().enumerate() {
                if *reached.get_unchecked(index)
                    && let Source::Computed(Operation::Divide([_, denominator])) = source
                    && lowered_denominators.get(denominator) == Some(&true)
                {
                    live_denominators
                        .entry(*denominator)
                        .and_modify(|shared| *shared = true)
                        .or_insert(false);
                }
            }
        }
        let shared = live_denominators
            .values()
            .filter(|multiple| **multiple)
            .count();
        if shared == 0 {
            return Ok(());
        }
        let capacity = self
            .sources
            .len()
            .checked_add(shared)
            .and_then(|total| u32::try_from(total).ok().map(|_| total))
            .ok_or(EvalError::ProgramTooLarge)?;
        let unit = self.coefficient(None)?;
        let previous = replace(&mut self.sources, Vec::with_capacity(capacity));
        let mut remapped: Vec<u32> = Vec::with_capacity(previous.len());
        let mut reciprocals: FixedMap<u32, u32> = FixedMap::default();
        // SAFETY: `remapped` gains one entry per original source in order,
        // `capacity <= u32::MAX` bounds every pushed source ID in `u32`, and
        // every input, unit, and result is an earlier source.
        unsafe {
            for (index, mut source) in previous.into_iter().enumerate() {
                let shared_denominator = match &source {
                    Source::Computed(Operation::Divide([_, denominator]))
                        if *reached.get_unchecked(index)
                            && live_denominators.get(denominator) == Some(&true) =>
                    {
                        Some(*denominator)
                    }
                    Source::Parameter(_)
                    | Source::Constant(_)
                    | Source::Computed(_)
                    | Source::Projection(_) => None,
                };
                if let Source::Computed(operation) = &mut source {
                    for input in operation.inputs_mut(&mut self.operands) {
                        *input = *remapped.get_unchecked(usize::try_from(*input).unwrap_or(0));
                    }
                    if let (Some(original), Operation::Divide([numerator, denominator])) =
                        (shared_denominator, &*operation)
                    {
                        let one = *remapped.get_unchecked(usize::try_from(unit).unwrap_or(0));
                        let reciprocal = if let Some(&existing) = reciprocals.get(&original) {
                            existing
                        } else {
                            let created = u32::try_from(self.sources.len()).unwrap_or(u32::MAX);
                            self.sources
                                .push(Source::Computed(Operation::Divide([one, *denominator])));
                            reciprocals.insert(original, created);
                            created
                        };
                        if *numerator == one {
                            remapped.push(reciprocal);
                            continue;
                        }
                        *operation =
                            Operation::Combine(Combine::Multiply, [*numerator, reciprocal]);
                    }
                }
                let next_id = u32::try_from(self.sources.len()).unwrap_or(u32::MAX);
                remapped.push(next_id);
                self.sources.push(source);
            }
            for result in &mut self.results {
                *result = *remapped.get_unchecked(usize::try_from(*result).unwrap_or(0));
            }
            self.unit = Some(*remapped.get_unchecked(usize::try_from(unit).unwrap_or(0)));
        }
        Ok(())
    }

    /// Replace reachable `sin`/`cos` and `sinh`/`cosh` of the same argument by
    /// one paired evaluation and a projection of its second output.
    pub fn pair_functions(&mut self) -> Result<(), EvalError> {
        if (self.trig_mask & 0b0011) != 0b0011 && (self.trig_mask & 0b1100) != 0b1100 {
            return Ok(());
        }
        let reached = self.reachable_sources();
        let mut groups: FixedMap<(u32, bool), (Option<usize>, Option<usize>)> = FixedMap::default();
        let mut duplicates: Vec<Option<usize>> = vec![None; self.sources.len()];
        // SAFETY: `reached` and `duplicates` have one entry per source.
        unsafe {
            for (index, source) in self.sources.iter().enumerate() {
                if !*reached.get_unchecked(index) {
                    continue;
                }
                if let Source::Computed(Operation::Unary(function, [input])) = source {
                    let hyperbolic =
                        matches!(function, BuiltinFunction::Sinh | BuiltinFunction::Cosh);
                    if !hyperbolic
                        && !matches!(function, BuiltinFunction::Sin | BuiltinFunction::Cos)
                    {
                        continue;
                    }
                    let cosine = matches!(function, BuiltinFunction::Cos | BuiltinFunction::Cosh);
                    let (sine_slot, cosine_slot) =
                        groups.entry((*input, hyperbolic)).or_insert((None, None));
                    let slot = if cosine { cosine_slot } else { sine_slot };
                    if let Some(canonical) = *slot {
                        *duplicates.get_unchecked_mut(index) = Some(canonical);
                    } else {
                        *slot = Some(index);
                    }
                }
            }
        }
        if !groups.values().any(|(s, c)| s.is_some() && c.is_some()) {
            return Ok(());
        }
        let mut pairs = vec![None; self.sources.len()];
        let previous = take(&mut self.sources);
        self.sources = Vec::with_capacity(previous.len());
        let mut remapped = vec![0_u32; previous.len()];
        let mut consumed = vec![false; previous.len()];
        // SAFETY: all group indices, inputs, results, and `unit` are bounded source indices,
        // and the output source count never exceeds `previous.len() <= u32::MAX`.
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
                if let Some(canonical) = *duplicates.get_unchecked(index) {
                    *remapped.get_unchecked_mut(index) = *remapped.get_unchecked(canonical);
                    continue;
                }
                if let (Some((sine, cosine, hyp)), Source::Computed(Operation::Unary(_, [input]))) =
                    (pair, &source)
                {
                    let producer = u32::try_from(self.sources.len())
                        .map_err(|_overflow| EvalError::ProgramTooLarge)?;
                    let projection = producer.checked_add(1).ok_or(EvalError::ProgramTooLarge)?;
                    let func = if hyp {
                        PairedFunction::SinhCosh
                    } else {
                        PairedFunction::SinCos
                    };
                    let mapped_in = *remapped.get_unchecked(usize::try_from(*input).unwrap_or(0));
                    self.sources.push(Source::Computed(Operation::Pair(
                        func,
                        [mapped_in],
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
                    for input in operation.inputs_mut(&mut self.operands) {
                        *input = *remapped.get_unchecked(usize::try_from(*input).unwrap_or(0));
                    }
                }
                let mapped_id = u32::try_from(self.sources.len())
                    .map_err(|_overflow| EvalError::ProgramTooLarge)?;
                *remapped.get_unchecked_mut(index) = mapped_id;
                self.sources.push(source);
            }
            for result in &mut self.results {
                *result = *remapped.get_unchecked(usize::try_from(*result).unwrap_or(0));
            }
            self.unit = self
                .unit
                .map(|unit| *remapped.get_unchecked(usize::try_from(unit).unwrap_or(0)));
        }
        Ok(())
    }
}
