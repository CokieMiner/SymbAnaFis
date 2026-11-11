//! Reachability, Sethi-Ullman pressure labels, constant compaction, and
//! counted-use register allocation in a pressure-ordered postorder.
//!
//! Allocation reads its tables by source index without bounds checks; the
//! source invariants of [`super::Compilation`] bound every such index.

#![expect(
    unsafe_code,
    reason = "SSA source indices, pooled ranges, and adjacency spans are bounded by construction; see the Compilation invariants"
)]

use core::cmp::Reverse;

use alloc::{boxed::Box, vec, vec::Vec};

use num_anafis::Number;

use crate::{core::Kernel, errors::EvalError};

use super::{Compilation, Fold, Instruction, OperandSpan, Operation, ProgramData, Source};

struct Labels {
    reached: Vec<bool>,
    uses: Vec<u32>,
    offsets: Vec<usize>,
    edges: Vec<u32>,
    retained: Vec<bool>,
    root_order: Vec<u32>,
    instructions: usize,
    operands: usize,
}

impl<Value> ProgramData<Value> {
    /// Verify that every pooled span lies in `operands` with valid arity, every
    /// builtin arity matches its `Operation` variant, every input is initialized,
    /// outputs lie outside the fixed prefix, paired outputs are distinct from
    /// primary outputs and inputs, and all results and active parameters are valid.
    pub fn validate(&self, register_count: usize, prefix: usize) -> bool {
        if self.parameters.len() > prefix || u32::try_from(register_count).is_err() {
            return false;
        }
        if !self
            .active_parameters
            .iter()
            .all(|&param| usize::try_from(param).is_ok_and(|idx| idx < self.parameters.len()))
        {
            return false;
        }
        let mut initialized = vec![false; register_count];
        let Some((fixed, _)) = initialized.split_at_mut_checked(prefix) else {
            return false;
        };
        fixed.fill(true);
        for instruction in &self.instructions {
            match &instruction.operation {
                Operation::Fold(fold, span) => {
                    let Some(range) = span.range() else {
                        return false;
                    };
                    if range.end > self.operands.len()
                        || match fold {
                            Fold::Dot => range.len() < 4 || range.len() % 2 != 0,
                            Fold::Sum | Fold::Product => range.len() < 3,
                        }
                    {
                        return false;
                    }
                }
                Operation::Unary(builtin, _) => {
                    if !matches!(Kernel::<Number>::of(*builtin), Kernel::Unary(_)) {
                        return false;
                    }
                }
                Operation::Binary(builtin, _) => {
                    if !matches!(Kernel::<Number>::of(*builtin), Kernel::Binary(_)) {
                        return false;
                    }
                }
                Operation::Ternary(builtin, _) => {
                    if !matches!(Kernel::<Number>::of(*builtin), Kernel::Ternary(_)) {
                        return false;
                    }
                }
                Operation::Combine(..)
                | Operation::Negate(_)
                | Operation::NegatedMultiply(_)
                | Operation::Divide(_)
                | Operation::Power(_)
                | Operation::Fused(..)
                | Operation::Pair(..) => {}
            }
            // SAFETY: `Fold` spans outside `self.operands` were rejected above.
            let inputs = unsafe { instruction.operation.inputs(&self.operands) };
            if !inputs.iter().all(|&index| {
                usize::try_from(index)
                    .ok()
                    .and_then(|slot| initialized.get(slot))
                    == Some(&true)
            }) {
                return false;
            }
            let (Some(out_idx), true) = (
                usize::try_from(instruction.output).ok(),
                usize::try_from(instruction.output).is_ok_and(|idx| idx >= prefix),
            ) else {
                return false;
            };
            if let Operation::Pair(_, [input], second) = instruction.operation {
                let Some(sec_idx) = usize::try_from(second).ok().filter(|&idx| {
                    idx >= prefix && second != instruction.output && second != input
                }) else {
                    return false;
                };
                let Some(output) = initialized.get_mut(sec_idx) else {
                    return false;
                };
                *output = true;
            }
            let Some(output) = initialized.get_mut(out_idx) else {
                return false;
            };
            *output = true;
        }
        self.results.iter().all(|&result| {
            usize::try_from(result)
                .ok()
                .and_then(|slot| initialized.get(slot))
                == Some(&true)
        })
    }
}

impl<Value: Clone> Compilation<Value> {
    /// Allocate physical registers for reachable sources and freeze into an
    /// immutable [`ProgramData`].
    pub fn finish(mut self) -> Result<ProgramData<Value>, EvalError> {
        let mut labels = Labels::build(&self);
        let mut physical = vec![0_u32; self.sources.len()];
        let (prefix, active_parameters) =
            self.assign_fixed_registers(&labels.reached, &labels.retained, &mut physical)?;
        let prefix_u32 = u32::try_from(prefix).map_err(|_overflow| EvalError::ProgramTooLarge)?;
        let mut free: Vec<u32> = Vec::new();
        let mut register_count = prefix;
        let allocate = |released: &mut Vec<u32>, end: &mut usize| -> Result<u32, EvalError> {
            if let Some(reused) = released.pop() {
                return Ok(reused);
            }
            let allocated = u32::try_from(*end).map_err(|_overflow| EvalError::ProgramTooLarge)?;
            *end = end.saturating_add(1);
            Ok(allocated)
        };
        let mut instructions = Vec::with_capacity(labels.instructions);
        let mut compact_operands = Vec::with_capacity(labels.operands);
        let mut pending: Vec<(u32, bool)> = labels
            .root_order
            .into_iter()
            .rev()
            .map(|r| (r, false))
            .collect();
        // SAFETY: results, inputs, projection producers, and edges index `self.sources`;
        // pooled spans lie in `self.operands`.
        unsafe {
            while let Some((virtual_output, finished)) = pending.pop() {
                let v_idx = usize::try_from(virtual_output).unwrap_or(0);
                if !finished {
                    let unentered = labels.reached.get_unchecked_mut(v_idx);
                    if *unentered {
                        *unentered = false;
                        pending.push((virtual_output, true));
                        let edge_range = *labels.offsets.get_unchecked(v_idx)
                            ..*labels.offsets.get_unchecked(v_idx.saturating_add(1));
                        pending.extend(
                            labels
                                .edges
                                .get_unchecked(edge_range)
                                .iter()
                                .rev()
                                .map(|&i| (i, false)),
                        );
                    }
                    continue;
                }
                let Source::Computed(mut operation) = *self.sources.get_unchecked(v_idx) else {
                    continue;
                };
                for input in operation.inputs_mut(&mut self.operands) {
                    let src_idx = usize::try_from(*input).unwrap_or(0);
                    *input = *physical.get_unchecked(src_idx);
                    let remaining = labels.uses.get_unchecked_mut(src_idx);
                    *remaining = remaining.saturating_sub(1);
                    if *remaining == 0 && *input >= prefix_u32 {
                        free.push(*input);
                    }
                }
                let output = allocate(&mut free, &mut register_count)?;
                if let Operation::Pair(_, _, second) = &mut operation {
                    let proj_idx = usize::try_from(*second).unwrap_or(0);
                    let destination = allocate(&mut free, &mut register_count)?;
                    *physical.get_unchecked_mut(proj_idx) = destination;
                    *second = destination;
                    if *labels.uses.get_unchecked(proj_idx) == 0 {
                        free.push(destination);
                    }
                    if *labels.uses.get_unchecked(v_idx) == 0 {
                        free.push(output);
                    }
                }
                if let Operation::Fold(_, span) = &mut operation {
                    let old_range = usize::try_from(span.start).unwrap_or(0)
                        ..usize::try_from(span.end).unwrap_or(0);
                    let start = u32::try_from(compact_operands.len())
                        .map_err(|_overflow| EvalError::ProgramTooLarge)?;
                    compact_operands.extend_from_slice(self.operands.get_unchecked(old_range));
                    let end = u32::try_from(compact_operands.len())
                        .map_err(|_overflow| EvalError::ProgramTooLarge)?;
                    *span = OperandSpan { start, end };
                }
                *physical.get_unchecked_mut(v_idx) = output;
                instructions.push(Instruction { output, operation });
            }
            for result in &mut self.results {
                *result = *physical.get_unchecked(usize::try_from(*result).unwrap_or(0));
            }
        }
        Ok(ProgramData {
            instructions: instructions.into_boxed_slice(),
            operands: compact_operands.into_boxed_slice(),
            parameters: self.parameters,
            active_parameters,
            results: self.results.into_boxed_slice(),
            constants: self.constants.into_boxed_slice(),
            register_count,
            initial: self.initial,
        })
    }

    fn assign_fixed_registers(
        &mut self,
        reached: &[bool],
        retained: &[bool],
        physical: &mut [u32],
    ) -> Result<(usize, Box<[u32]>), EvalError> {
        let parameter_count = self.parameters.len();
        let mut slots = vec![0_u32; self.constants.len()];
        let mut prefix = parameter_count;
        for (slot, keep) in slots.iter_mut().zip(retained) {
            if *keep {
                *slot = u32::try_from(prefix).map_err(|_overflow| EvalError::ProgramTooLarge)?;
                prefix = prefix.saturating_add(1);
            }
        }
        let mut flags = retained.iter();
        self.constants.retain(|_| flags.next() == Some(&true));
        let mut param_used = vec![false; parameter_count];
        for ((register, source), &is_live) in physical.iter_mut().zip(&self.sources).zip(reached) {
            match source {
                Source::Parameter(parameter) => {
                    *register = *parameter;
                    if is_live
                        && let Some(flag) =
                            param_used.get_mut(usize::try_from(*parameter).unwrap_or(usize::MAX))
                    {
                        *flag = true;
                    }
                }
                Source::Constant(constant) => {
                    let idx = usize::try_from(*constant).unwrap_or(0);
                    // SAFETY: `Compilation` invariants bound `*constant` inside `slots`.
                    *register = unsafe { *slots.get_unchecked(idx) };
                }
                Source::Computed(_) | Source::Projection(_) => {}
            }
        }
        let active_parameters: Box<[u32]> = param_used
            .into_iter()
            .enumerate()
            .filter(|&(_, used)| used)
            .map(|(idx, _)| u32::try_from(idx).unwrap_or(0))
            .collect();
        Ok((prefix, active_parameters))
    }
}

impl Labels {
    fn build<Value>(compilation: &Compilation<Value>) -> Self {
        let mut labels = Self {
            reached: vec![false; compilation.sources.len()],
            uses: vec![0; compilation.sources.len()],
            offsets: vec![0; compilation.sources.len().saturating_add(1)],
            edges: Vec::new(),
            retained: vec![false; compilation.constants.len()],
            root_order: compilation.results.clone(),
            instructions: 0,
            operands: 0,
        };
        labels.mark_reachable(
            &compilation.sources,
            &compilation.operands,
            &compilation.results,
        );
        labels.order_by_demand(&compilation.sources, &compilation.operands);
        labels
    }

    fn mark_reachable(&mut self, sources: &[Source], operands_pool: &[u32], results: &[u32]) {
        // SAFETY: results, inputs, and projection producers index `sources`;
        // constant sources index `self.retained`; pooled spans lie in `operands_pool`.
        unsafe {
            for &result in results {
                let r_idx = usize::try_from(result).unwrap_or(0);
                *self.reached.get_unchecked_mut(r_idx) = true;
                let count = self.uses.get_unchecked_mut(r_idx);
                *count = count.saturating_add(1);
            }
            for (index, source) in sources.iter().enumerate().rev() {
                if !*self.reached.get_unchecked(index) {
                    continue;
                }
                match source {
                    Source::Computed(operation) => {
                        self.instructions = self.instructions.saturating_add(1);
                        if let Operation::Fold(_, span) = operation {
                            let len =
                                usize::try_from(span.end.saturating_sub(span.start)).unwrap_or(0);
                            self.operands = self.operands.saturating_add(len);
                        }
                        for &input in operation.inputs(operands_pool) {
                            let in_idx = usize::try_from(input).unwrap_or(0);
                            *self.reached.get_unchecked_mut(in_idx) = true;
                            let count = self.uses.get_unchecked_mut(in_idx);
                            *count = count.saturating_add(1);
                        }
                    }
                    Source::Projection(producer) => {
                        let prod_idx = usize::try_from(*producer).unwrap_or(0);
                        *self.reached.get_unchecked_mut(prod_idx) = true;
                    }
                    Source::Constant(literal) => {
                        let lit_idx = usize::try_from(*literal).unwrap_or(0);
                        *self.retained.get_unchecked_mut(lit_idx) = true;
                    }
                    Source::Parameter(_) => {}
                }
            }
        }
    }

    fn order_by_demand(&mut self, sources: &[Source], operands_pool: &[u32]) {
        let mut demands = vec![0_usize; sources.len()];
        self.edges.reserve(
            self.instructions
                .saturating_mul(2)
                .saturating_add(self.operands),
        );
        // SAFETY: `sources`, `demands`, and `self.reached` share length `sources.len()`;
        // `self.offsets` has length `sources.len() + 1`; pooled spans lie in `operands_pool`.
        unsafe {
            for (index, source) in sources.iter().enumerate() {
                let start = self.edges.len();
                *self.offsets.get_unchecked_mut(index) = start;
                if !*self.reached.get_unchecked(index) {
                    continue;
                }
                match source {
                    Source::Computed(operation) => {
                        self.edges
                            .extend(operation.inputs(operands_pool).iter().copied().filter(
                                |&input| {
                                    *demands.get_unchecked(usize::try_from(input).unwrap_or(0)) != 0
                                },
                            ));
                        let inputs = self.edges.get_unchecked_mut(start..);
                        match inputs {
                            [] | [_] => {}
                            [first, second] => {
                                let d0 =
                                    *demands.get_unchecked(usize::try_from(*first).unwrap_or(0));
                                let d1 =
                                    *demands.get_unchecked(usize::try_from(*second).unwrap_or(0));
                                if (Reverse(d1), *second) < (Reverse(d0), *first) {
                                    inputs.swap(0, 1);
                                }
                            }
                            _ => {
                                inputs.sort_unstable_by_key(|&input| {
                                    (
                                        Reverse(
                                            *demands
                                                .get_unchecked(usize::try_from(input).unwrap_or(0)),
                                        ),
                                        input,
                                    )
                                });
                            }
                        }
                        let demand = inputs
                            .iter()
                            .enumerate()
                            .map(|(offset, &input)| {
                                demands
                                    .get_unchecked(usize::try_from(input).unwrap_or(0))
                                    .saturating_add(offset)
                            })
                            .max()
                            .unwrap_or(1);
                        *demands.get_unchecked_mut(index) = demand.max(1);
                    }
                    Source::Projection(producer) => {
                        self.edges.push(*producer);
                        *demands.get_unchecked_mut(index) =
                            *demands.get_unchecked(usize::try_from(*producer).unwrap_or(0));
                    }
                    Source::Parameter(_) | Source::Constant(_) => {}
                }
            }
            *self.offsets.get_unchecked_mut(sources.len()) = self.edges.len();
            if self.root_order.len() > 1 {
                self.root_order.sort_by_key(|&root| {
                    Reverse(*demands.get_unchecked(usize::try_from(root).unwrap_or(0)))
                });
            }
        }
    }
}

impl Operation {
    /// The input sources or registers of this operation.
    ///
    /// # Safety
    /// Every pooled span of this operation lies in `operands`.
    pub unsafe fn inputs<'sources>(&'sources self, operands: &'sources [u32]) -> &'sources [u32] {
        match self {
            Self::Negate(sources) | Self::Unary(_, sources) | Self::Pair(_, sources, _) => sources,
            Self::Fold(_, span) => {
                let (s, e) = (
                    usize::try_from(span.start).unwrap_or(0),
                    usize::try_from(span.end).unwrap_or(0),
                );
                // SAFETY: the caller's contract.
                unsafe { operands.get_unchecked(s..e) }
            }
            Self::Combine(_, sources)
            | Self::NegatedMultiply(sources)
            | Self::Divide(sources)
            | Self::Power(sources)
            | Self::Binary(_, sources) => sources,
            Self::Fused(_, sources) | Self::Ternary(_, sources) => sources,
        }
    }

    /// The mutable input sources or registers of this operation.
    ///
    /// # Safety
    /// Every pooled span of this operation lies in `operands`.
    pub unsafe fn inputs_mut<'sources>(
        &'sources mut self,
        operands: &'sources mut [u32],
    ) -> &'sources mut [u32] {
        match self {
            Self::Negate(sources) | Self::Unary(_, sources) | Self::Pair(_, sources, _) => sources,
            Self::Fold(_, span) => {
                let (s, e) = (
                    usize::try_from(span.start).unwrap_or(0),
                    usize::try_from(span.end).unwrap_or(0),
                );
                // SAFETY: the caller's contract.
                unsafe { operands.get_unchecked_mut(s..e) }
            }
            Self::Combine(_, sources)
            | Self::NegatedMultiply(sources)
            | Self::Divide(sources)
            | Self::Power(sources)
            | Self::Binary(_, sources) => sources,
            Self::Fused(_, sources) | Self::Ternary(_, sources) => sources,
        }
    }
}
