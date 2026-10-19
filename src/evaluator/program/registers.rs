//! Reachability, constant compaction, and counted-use register allocation.

use alloc::{vec, vec::Vec};

use super::{Instruction, Operation, Program, Source};

impl<Value> Program<Value> {
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
            let inputs: &[usize] = match &instruction.operation {
                Operation::AddMany(range)
                | Operation::MultiplyMany(range)
                | Operation::NegatedMultiplyMany(range) => {
                    let Some(sources) = self.operands.get(range.clone()) else {
                        return false;
                    };
                    if sources.len() < 3 {
                        return false;
                    }
                    sources
                }
                Operation::Add(sources)
                | Operation::Subtract(sources)
                | Operation::Multiply(sources)
                | Operation::NegatedMultiply(sources)
                | Operation::Divide(sources)
                | Operation::Power(sources)
                | Operation::Binary(_, _, sources) => sources,
                Operation::Negate(sources)
                | Operation::Unary(_, _, sources)
                | Operation::Pair(_, _, sources, _) => sources,
                Operation::Fused(sources)
                | Operation::FusedSubtract(sources)
                | Operation::NegatedFusedAdd(sources)
                | Operation::NegatedFusedSubtract(sources)
                | Operation::Ternary(_, sources) => sources,
                Operation::Quaternary(_, sources) => sources,
            };
            if !inputs
                .iter()
                .all(|index| initialized.get(*index) == Some(&true))
            {
                return false;
            }
            if instruction.output < prefix {
                return false;
            }
            if let Operation::Pair(_, _, _, second) = instruction.operation {
                if second < prefix || second == instruction.output {
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

    #[expect(
        clippy::indexing_slicing,
        clippy::too_many_lines,
        reason = "SSA sources precede their consumers and all analysis tables have one entry per source or literal"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the compiler checks the combined register bound; use counts are bounded by allocated fixed and pooled edges, and decrements consume exactly the counted edges"
    )]
    pub fn allocate_registers(
        &mut self,
        sources: &[Source<Value>],
        constants: &mut Vec<Value>,
    ) -> usize
    where
        Value: Clone,
    {
        let mut reachable = vec![false; sources.len()];
        let mut pending = vec![self.result];
        while let Some(index) = pending.pop() {
            if !reachable[index] {
                reachable[index] = true;
                if let Source::Computed(operation) = &sources[index] {
                    pending.extend(operation.inputs(&self.operands));
                } else if let Source::Projection(producer) = sources[index] {
                    pending.push(producer);
                }
            }
        }
        let mut uses = vec![0_usize; sources.len()];
        let mut retained = vec![false; constants.len()];
        let mut instruction_count = 0;
        let mut operand_count = 0;
        for (_, source) in sources
            .iter()
            .enumerate()
            .filter(|(index, _)| reachable[*index])
        {
            match source {
                Source::Computed(operation) => {
                    instruction_count += 1;
                    if let Operation::AddMany(range)
                    | Operation::MultiplyMany(range)
                    | Operation::NegatedMultiplyMany(range) = operation
                    {
                        operand_count += range.len();
                    }
                    for input in operation.inputs(&self.operands) {
                        uses[*input] += 1;
                    }
                }
                Source::Constant(literal) => retained[*literal] = true,
                Source::Parameter(_) | Source::Projection(_) => {}
            }
        }
        uses[self.result] += 1;
        let (constant_slots, prefix) = self.compact_constants(constants, &retained);
        let mut physical = vec![0; sources.len()];
        for (index, source) in sources.iter().enumerate() {
            match source {
                Source::Parameter(parameter) => physical[index] = *parameter,
                Source::Constant(constant) => physical[index] = constant_slots[*constant],
                Source::Computed(_) | Source::Projection(_) => {}
            }
        }
        let mut free = Vec::new();
        let mut width = prefix;
        self.instructions.reserve_exact(instruction_count);
        let mut compact_operands = Vec::with_capacity(operand_count);
        let order = self
            .schedule(sources, instruction_count)
            .unwrap_or_else(|| (0..sources.len()).collect());
        for virtual_output in order {
            if !reachable[virtual_output] {
                continue;
            }
            let mut operation = match &sources[virtual_output] {
                Source::Parameter(_) | Source::Constant(_) | Source::Projection(_) => continue,
                Source::Computed(operation) => operation.clone(),
            };
            for input in operation.inputs_mut(&mut self.operands) {
                let virtual_source = *input;
                *input = physical[virtual_source];
                uses[virtual_source] -= 1;
                if uses[virtual_source] == 0 && *input >= prefix {
                    free.push(*input);
                }
            }
            let output = free.pop().unwrap_or_else(|| {
                let allocated = width;
                width += 1;
                allocated
            });
            if let Operation::Pair(_, _, _, second) = &mut operation {
                let projection = *second;
                let destination = free.pop().unwrap_or_else(|| {
                    let allocated = width;
                    width += 1;
                    allocated
                });
                physical[projection] = destination;
                *second = destination;
                if uses[projection] == 0 {
                    free.push(destination);
                }
                if uses[virtual_output] == 0 {
                    free.push(output);
                }
            }
            if let Operation::AddMany(range)
            | Operation::MultiplyMany(range)
            | Operation::NegatedMultiplyMany(range) = &mut operation
            {
                let start = compact_operands.len();
                compact_operands.extend_from_slice(&self.operands[range.clone()]);
                *range = start..compact_operands.len();
            }
            physical[virtual_output] = output;
            self.instructions.push(Instruction { output, operation });
        }
        self.result = physical[self.result];
        self.operands = compact_operands;
        width
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "the reachability pass creates one retained flag per constant and retain visits each original constant exactly once"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the checked compiler bound contains the parameter count and every original constant"
    )]
    fn compact_constants(
        &self,
        constants: &mut Vec<Value>,
        retained: &[bool],
    ) -> (Vec<usize>, usize) {
        let mut slots = vec![0; constants.len()];
        let mut prefix = self.parameters.len();
        for (slot, keep) in slots.iter_mut().zip(retained) {
            if *keep {
                *slot = prefix;
                prefix += 1;
            }
        }
        let mut index = 0;
        constants.retain(|_| {
            let keep = retained[index];
            index += 1;
            keep
        });
        (slots, prefix)
    }
}

impl<Value> Operation<Value> {
    #[expect(
        clippy::indexing_slicing,
        reason = "pooled ranges are bounded by the operand vector's length immediately around appending each normalized group"
    )]
    pub fn inputs<'sources>(&'sources self, operands: &'sources [usize]) -> &'sources [usize] {
        match self {
            Self::Negate(sources) | Self::Unary(_, _, sources) | Self::Pair(_, _, sources, _) => {
                sources
            }
            Self::AddMany(range) | Self::MultiplyMany(range) | Self::NegatedMultiplyMany(range) => {
                &operands[range.clone()]
            }
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
            | Self::Ternary(_, sources) => sources,
            Self::Quaternary(_, sources) => sources,
        }
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "pooled ranges keep their proven bounds while only register indices are remapped"
    )]
    pub fn inputs_mut<'sources>(
        &'sources mut self,
        operands: &'sources mut [usize],
    ) -> &'sources mut [usize] {
        match self {
            Self::Negate(sources) | Self::Unary(_, _, sources) | Self::Pair(_, _, sources, _) => {
                sources
            }
            Self::AddMany(range) | Self::MultiplyMany(range) | Self::NegatedMultiplyMany(range) => {
                &mut operands[range.clone()]
            }
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
            | Self::Ternary(_, sources) => sources,
            Self::Quaternary(_, sources) => sources,
        }
    }
}
