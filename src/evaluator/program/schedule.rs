//! Sethi-Ullman pressure labels and dependency-preserving instruction order.

use core::cmp::Reverse;

use alloc::{vec, vec::Vec};

use super::{Program, Source};

impl<Value> Program<Value> {
    /// Schedule larger programs by descending temporary-register demand.
    /// Small programs retain their existing postorder without another graph pass.
    #[expect(
        clippy::indexing_slicing,
        reason = "dependencies reference earlier SSA producers, adjacency ranges are built from their vector bounds, and every state table has one entry per source"
    )]
    pub fn schedule(
        &self,
        sources: &[Source<Value>],
        instruction_count: usize,
    ) -> Option<Vec<usize>> {
        if instruction_count < 64 {
            return None;
        }
        let mut demands = vec![0_usize; sources.len()];
        let mut edges = Vec::new();
        let mut ranges = Vec::with_capacity(sources.len());
        for (index, source) in sources.iter().enumerate() {
            let start = edges.len();
            if let Source::Computed(operation) = source {
                edges.extend(
                    operation
                        .inputs(&self.operands)
                        .iter()
                        .copied()
                        .filter(|input| {
                            matches!(sources[*input], Source::Computed(_) | Source::Projection(_))
                        }),
                );
                edges[start..].sort_unstable_by_key(|input| (Reverse(demands[*input]), *input));
                let demand = edges[start..]
                    .iter()
                    .enumerate()
                    .map(|(offset, input)| demands[*input].saturating_add(offset))
                    .max()
                    .unwrap_or(1);
                demands[index] = demand.max(1);
            } else if let Source::Projection(producer) = source {
                edges.push(*producer);
                demands[index] = demands[*producer];
            }
            ranges.push(start..edges.len());
        }
        let mut pending = vec![(self.result, false)];
        let mut visited = vec![false; sources.len()];
        let mut order = Vec::new();
        while let Some((index, complete)) = pending.pop() {
            if complete {
                order.push(index);
            } else if !visited[index] {
                visited[index] = true;
                pending.push((index, true));
                pending.extend(
                    edges[ranges[index].clone()]
                        .iter()
                        .rev()
                        .map(|input| (*input, false)),
                );
            }
        }
        Some(order)
    }
}
