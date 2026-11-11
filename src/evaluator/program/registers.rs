//! Reachability, Sethi-Ullman pressure labels, constant compaction, and
//! counted-use register allocation in a pressure-ordered postorder.
//!
//! Allocation reads its tables by source index without bounds checks; the
//! source invariants of [`super::Compilation`] bound every such index.

#![expect(
    unsafe_code,
    reason = "SSA source indices, pooled ranges, and adjacency spans are bounded by construction; see the Compilation invariants"
)]

use core::{cmp::Reverse, ops::Range};

use alloc::{vec, vec::Vec};

use super::{Fold, Instruction, Operation, ProgramData, Source};

/// The sources reachable from a program's result, with one entry per source
/// in each table.
struct Labels {
    /// Whether the source is reachable.
    reached: Vec<bool>,
    /// The reads of the source by reachable computed sources, and one more
    /// for the result.
    uses: Vec<usize>,
    /// The computed inputs, or the producer of a projection, by descending
    /// demand, as a span of `edges`.
    spans: Vec<Range<usize>>,
    edges: Vec<usize>,
    /// Whether a reachable source reads the constant, one flag per constant.
    retained: Vec<bool>,
    /// The reachable computed sources and the operands they pool.
    instructions: usize,
    operands: usize,
}

impl<Value> ProgramData<Value> {
    /// Check the contract that execution relies on: every pooled range lies in
    /// the pool and holds at least three operands, or an even number of at
    /// least four for a dot product, every input is a fixed slot or an
    /// earlier output, outputs lie outside the fixed prefix, a joint output
    /// differs from the output and the input, and the result is initialized.
    pub fn validate(&self, register_count: usize, prefix: usize) -> bool {
        if self.parameters.len() > prefix {
            return false;
        }
        let mut initialized = vec![false; register_count];
        let Some((fixed, _)) = initialized.split_at_mut_checked(prefix) else {
            return false;
        };
        fixed.fill(true);
        for instruction in &self.instructions {
            // `Range::len` is zero for a reversed range.
            if let Operation::Fold(fold, range) = &instruction.operation
                && (range.end > self.operands.len()
                    || match fold {
                        Fold::Dot => range.len() < 4 || range.len() % 2 != 0,
                        Fold::Sum | Fold::Product => range.len() < 3,
                    })
            {
                return false;
            }
            // SAFETY: a pooled range that does not lie in `operands` was
            // rejected just above.
            let inputs = unsafe { instruction.operation.inputs(&self.operands) };
            if !inputs
                .iter()
                .all(|index| initialized.get(*index) == Some(&true))
            {
                return false;
            }
            if instruction.output < prefix {
                return false;
            }
            if let Operation::Pair(_, _, [input], second) = instruction.operation {
                if second < prefix || second == instruction.output || second == input {
                    return false;
                }
                let Some(output) = initialized.get_mut(second) else {
                    return false;
                };
                *output = true;
            }
            let Some(output) = initialized.get_mut(instruction.output) else {
                return false;
            };
            *output = true;
        }
        initialized.get(self.result) == Some(&true)
    }

    /// Emit the sources reachable from the result and assign each a register:
    /// parameters and kept constants occupy the fixed prefix, and a temporary
    /// register is reused after the last read of its value. Returns the number
    /// of registers.
    ///
    /// Emission is a postorder from the result that enters the computed inputs
    /// of each source by descending demand (see [`Self::label`]), which bounds
    /// the temporaries held across an input by the demand of the inputs before
    /// it.
    ///
    /// `sources`, `constants`, `self.operands`, and `self.result` satisfy the
    /// source invariants of [`super::Compilation`].
    ///
    /// Counts are bounded by the allocated sources and pooled operands, and
    /// every decrement consumes an edge counted before, so the saturating
    /// operations below never saturate.
    pub fn allocate_registers(
        &mut self,
        sources: &[Source<Value>],
        constants: &mut Vec<Value>,
    ) -> usize
    where
        Value: Clone,
    {
        let Labels {
            mut reached,
            mut uses,
            spans,
            edges,
            retained,
            instructions,
            operands,
        } = self.label(sources, constants.len());
        let mut physical = vec![0; sources.len()];
        // SAFETY: the result, every input of a computed source, every
        // projection producer, and every edge index `sources`, the length of
        // `reached`, `uses`, `spans`, and `physical`; every span lies in
        // `edges`; every constant source indexes `constants`, the length of
        // `retained`; every pooled range lies in `self.operands`. Before
        // emission the joint output of a pair names its projection source.
        unsafe {
            let prefix = self.fixed_registers(sources, constants, &retained, &mut physical);
            let mut free = Vec::new();
            let mut width = prefix;
            // A freed temporary register, or a new one past the current width.
            let allocate = |released: &mut Vec<usize>, end: &mut usize| {
                released.pop().unwrap_or_else(|| {
                    let allocated = *end;
                    *end = end.saturating_add(1);
                    allocated
                })
            };
            self.instructions.reserve_exact(instructions);
            let mut compact_operands = Vec::with_capacity(operands);
            // Emission clears the flag of each source it enters, so it enters
            // every reachable source once.
            let mut pending = vec![(self.result, false)];
            while let Some((virtual_output, finished)) = pending.pop() {
                if !finished {
                    let unentered = reached.get_unchecked_mut(virtual_output);
                    if *unentered {
                        *unentered = false;
                        pending.push((virtual_output, true));
                        pending.extend(
                            edges
                                .get_unchecked(spans.get_unchecked(virtual_output).clone())
                                .iter()
                                .rev()
                                .map(|input| (*input, false)),
                        );
                    }
                    continue;
                }
                let mut operation = match sources.get_unchecked(virtual_output) {
                    Source::Parameter(_) | Source::Constant(_) | Source::Projection(_) => continue,
                    Source::Computed(operation) => operation.clone(),
                };
                for input in operation.inputs_mut(&mut self.operands) {
                    let virtual_source = *input;
                    *input = *physical.get_unchecked(virtual_source);
                    let remaining = uses.get_unchecked_mut(virtual_source);
                    *remaining = remaining.saturating_sub(1);
                    if *remaining == 0 && *input >= prefix {
                        free.push(*input);
                    }
                }
                let output = allocate(&mut free, &mut width);
                if let Operation::Pair(_, _, _, second) = &mut operation {
                    let projection = *second;
                    let destination = allocate(&mut free, &mut width);
                    *physical.get_unchecked_mut(projection) = destination;
                    *second = destination;
                    if *uses.get_unchecked(projection) == 0 {
                        free.push(destination);
                    }
                    if *uses.get_unchecked(virtual_output) == 0 {
                        free.push(output);
                    }
                }
                if let Operation::Fold(_, range) = &mut operation {
                    let start = compact_operands.len();
                    compact_operands.extend_from_slice(self.operands.get_unchecked(range.clone()));
                    *range = start..compact_operands.len();
                }
                *physical.get_unchecked_mut(virtual_output) = output;
                self.instructions.push(Instruction { output, operation });
            }
            self.result = *physical.get_unchecked(self.result);
            self.operands = compact_operands;
            width
        }
    }

    /// Find the sources reachable from the result, count their reads, and
    /// label each with its Sethi-Ullman demand: leaves need no temporary
    /// register, and a computed source needs at least one, and the largest
    /// demand of its computed inputs taken by descending demand, each
    /// increased by the number of inputs taken before it. A projection has
    /// the demand of its producer.
    ///
    /// Every input of a source precedes it, so a sweep in reverse order meets
    /// every reader of a source before the source, and a sweep in order meets
    /// every input before its readers; both read the sources sequentially.
    ///
    /// Reference: R. Sethi and J. D. Ullman, "The generation of optimal code
    /// for arithmetic expressions", J. ACM 17 (1970), 715-728.
    ///
    /// `sources`, `self.operands`, and `self.result` satisfy the source
    /// invariants of [`super::Compilation`], and every constant source
    /// indexes the `constant_count` constants.
    fn label(&self, sources: &[Source<Value>], constant_count: usize) -> Labels {
        let mut labels = Labels {
            reached: vec![false; sources.len()],
            uses: vec![0; sources.len()],
            spans: vec![0..0; sources.len()],
            edges: Vec::new(),
            retained: vec![false; constant_count],
            instructions: 0,
            operands: 0,
        };
        let mut demands = vec![0_usize; sources.len()];
        // SAFETY: the result, every input of a computed source, and every
        // projection producer index `sources`, the length of every per-source
        // table; every constant source indexes `retained`; every pooled range
        // lies in `self.operands`; each labeled source records the span of
        // `edges` it appended, and `edges` only grows.
        unsafe {
            *labels.reached.get_unchecked_mut(self.result) = true;
            *labels.uses.get_unchecked_mut(self.result) = 1;
            for (index, source) in sources.iter().enumerate().rev() {
                if !*labels.reached.get_unchecked(index) {
                    continue;
                }
                match source {
                    Source::Computed(operation) => {
                        labels.instructions = labels.instructions.saturating_add(1);
                        if let Operation::Fold(_, range) = operation {
                            labels.operands = labels.operands.saturating_add(range.len());
                        }
                        for input in operation.inputs(&self.operands) {
                            *labels.reached.get_unchecked_mut(*input) = true;
                            let count = labels.uses.get_unchecked_mut(*input);
                            *count = count.saturating_add(1);
                        }
                    }
                    Source::Projection(producer) => {
                        *labels.reached.get_unchecked_mut(*producer) = true;
                    }
                    Source::Constant(literal) => {
                        *labels.retained.get_unchecked_mut(*literal) = true;
                    }
                    Source::Parameter(_) => {}
                }
            }
            for (index, source) in sources.iter().enumerate() {
                if !*labels.reached.get_unchecked(index) {
                    continue;
                }
                // Every input of a reachable source is reachable and labeled,
                // and exactly the computed inputs and projections carry a
                // nonzero demand.
                let start = labels.edges.len();
                match source {
                    Source::Computed(operation) => {
                        labels.edges.extend(
                            operation
                                .inputs(&self.operands)
                                .iter()
                                .filter(|input| *demands.get_unchecked(**input) != 0),
                        );
                        let inputs = labels.edges.get_unchecked_mut(start..);
                        inputs.sort_unstable_by_key(|input| {
                            (Reverse(*demands.get_unchecked(*input)), *input)
                        });
                        let demand = inputs
                            .iter()
                            .enumerate()
                            .map(|(offset, input)| {
                                demands.get_unchecked(*input).saturating_add(offset)
                            })
                            .max()
                            .unwrap_or(1);
                        *demands.get_unchecked_mut(index) = demand.max(1);
                    }
                    Source::Projection(producer) => {
                        labels.edges.push(*producer);
                        *demands.get_unchecked_mut(index) = *demands.get_unchecked(*producer);
                    }
                    Source::Parameter(_) | Source::Constant(_) => {}
                }
                *labels.spans.get_unchecked_mut(index) = start..labels.edges.len();
            }
        }
        labels
    }

    /// Lay out the fixed prefix: keep the constants flagged in `retained`, one
    /// flag per constant, and give every parameter source its parameter
    /// register and every constant source its kept constant's register,
    /// directly after the parameters. Returns the length of the prefix.
    ///
    /// # Safety
    /// Every constant source of `sources` indexes `constants`, and `physical`
    /// holds one register per source.
    unsafe fn fixed_registers(
        &self,
        sources: &[Source<Value>],
        constants: &mut Vec<Value>,
        retained: &[bool],
        physical: &mut [usize],
    ) -> usize {
        let mut slots = vec![0; constants.len()];
        let mut prefix = self.parameters.len();
        for (slot, keep) in slots.iter_mut().zip(retained) {
            if *keep {
                *slot = prefix;
                // The compiler bounds the parameters and constants together.
                prefix = prefix.saturating_add(1);
            }
        }
        let mut flags = retained.iter();
        constants.retain(|_| flags.next() == Some(&true));
        for (register, source) in physical.iter_mut().zip(sources) {
            match source {
                Source::Parameter(parameter) => *register = *parameter,
                Source::Constant(constant) => {
                    // SAFETY: the caller's contract; `slots` has one entry per
                    // original constant.
                    *register = unsafe { *slots.get_unchecked(*constant) };
                }
                Source::Computed(_) | Source::Projection(_) => {}
            }
        }
        prefix
    }
}

impl<Value> Operation<Value> {
    /// The input sources or registers of this operation; a pooled range is
    /// read from `operands`.
    ///
    /// # Safety
    /// Every pooled range of this operation lies in `operands`.
    pub unsafe fn inputs<'sources>(
        &'sources self,
        operands: &'sources [usize],
    ) -> &'sources [usize] {
        match self {
            Self::Negate(sources) | Self::Unary(_, _, sources) | Self::Pair(_, _, sources, _) => {
                sources
            }
            // SAFETY: the caller's contract.
            Self::Fold(_, range) => unsafe { operands.get_unchecked(range.clone()) },
            Self::Add(sources)
            | Self::Subtract(sources)
            | Self::Multiply(sources)
            | Self::NegatedMultiply(sources)
            | Self::Divide(sources)
            | Self::Power(sources)
            | Self::Binary(_, _, sources) => sources,
            Self::Fused(sources)
            | Self::FusedSubtract(sources)
            | Self::NegatedFusedAdd(sources)
            | Self::NegatedFusedSubtract(sources)
            | Self::Ternary(_, _, sources) => sources,
        }
    }

    /// The input sources or registers of this operation, for remapping; a
    /// pooled range is read from `operands`.
    ///
    /// # Safety
    /// Every pooled range of this operation lies in `operands`.
    pub unsafe fn inputs_mut<'sources>(
        &'sources mut self,
        operands: &'sources mut [usize],
    ) -> &'sources mut [usize] {
        match self {
            Self::Negate(sources) | Self::Unary(_, _, sources) | Self::Pair(_, _, sources, _) => {
                sources
            }
            // SAFETY: the caller's contract.
            Self::Fold(_, range) => unsafe { operands.get_unchecked_mut(range.clone()) },
            Self::Add(sources)
            | Self::Subtract(sources)
            | Self::Multiply(sources)
            | Self::NegatedMultiply(sources)
            | Self::Divide(sources)
            | Self::Power(sources)
            | Self::Binary(_, _, sources) => sources,
            Self::Fused(sources)
            | Self::FusedSubtract(sources)
            | Self::NegatedFusedAdd(sources)
            | Self::NegatedFusedSubtract(sources)
            | Self::Ternary(_, _, sources) => sources,
        }
    }
}
