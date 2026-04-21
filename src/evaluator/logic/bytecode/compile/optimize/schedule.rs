use super::{VInstruction, VReg};
use rustc_hash::FxHashSet;
use std::collections::BinaryHeap;

/// Schedules `VInstruction`s using a greedy topological sort to minimize peak register pressure.
///
/// It builds a dependency graph (DAG) of the instructions and prioritizes instructions that
/// consume the most inputs whose last use is this instruction (Sethi-Ullman heuristic),
/// reducing active live ranges.
#[allow(
    clippy::too_many_lines,
    reason = "CSR construction and greedy scheduling involve several O(n) passes"
)]
pub fn greedy_schedule(vinstrs: Vec<VInstruction>, next_vreg: u32) -> Vec<VInstruction> {
    if vinstrs.is_empty() {
        return vinstrs;
    }

    let n = vinstrs.len();
    assert!(
        u32::try_from(n).is_ok(),
        "too many instructions for u32 index"
    );

    // Temp IDs are allocated sequentially but some may be deleted during GVN.
    // We use next_vreg as the safe upper bound for temporary indices.
    let max_t = next_vreg as usize;

    // Pass 1: populate producer_map (temp -> instr index) and last_use (temp -> last instr index)
    let mut producer_map = vec![usize::MAX; max_t];
    let mut last_use = vec![usize::MAX; max_t];
    for (i, instr) in vinstrs.iter().enumerate() {
        if let VReg::Temp(t) = instr.dest() {
            debug_assert!((t as usize) < max_t, "Temp ID {t} exceeds max_vreg {max_t}");
            producer_map[t as usize] = i;
        }
        instr.for_each_read(|r| {
            if let VReg::Temp(t) = r {
                debug_assert!((t as usize) < max_t, "Temp ID {t} exceeds max_vreg {max_t}");
                last_use[t as usize] = i;
            }
        });
    }

    // Pass 2: count outgoing degree per node to allocate CSR arrays and compute heuristic weights
    let mut degree = vec![0_usize; n];
    let mut weights = vec![0_i32; n];
    let mut seen = FxHashSet::default();
    for (i, instr) in vinstrs.iter().enumerate() {
        seen.clear();
        let mut dying = 0_i32;
        instr.for_each_read(|r| {
            if let VReg::Temp(t) = r {
                let p = producer_map[t as usize];
                if p != usize::MAX && seen.insert(p) {
                    degree[p] += 1;
                    if last_use[t as usize] == i {
                        dying += 1;
                    }
                }
            }
        });
        weights[i] = dying - 1;
    }

    // Pass 3: prefix sum -> offsets
    let mut offsets = vec![0_usize; n + 1];
    for i in 0..n {
        offsets[i + 1] = offsets[i] + degree[i];
    }

    // Pass 4: fill CSR edges
    let total_edges = offsets[n];
    let mut edges = vec![0_u32; total_edges];
    // Reuse degree as cursor to avoid an extra allocation/clone
    let mut cursor = degree;
    cursor[..n].copy_from_slice(&offsets[..n]);
    let mut in_degree = vec![0_u32; n];
    for (i, instr) in vinstrs.iter().enumerate() {
        seen.clear();
        instr.for_each_read(|r| {
            if let VReg::Temp(t) = r {
                let p = producer_map[t as usize];
                if p != usize::MAX && seen.insert(p) {
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "n is checked to fit in u32 upfront"
                    )]
                    let v_idx = i as u32;
                    edges[cursor[p]] = v_idx;
                    cursor[p] += 1;
                    in_degree[i] += 1;
                }
            }
        });
    }

    let mut pq = BinaryHeap::new();
    for i in 0..n {
        if in_degree[i] == 0 {
            #[allow(
                clippy::cast_possible_wrap,
                reason = "Indices fit well within i64 bounds"
            )]
            let priority = -(i as i64);
            pq.push((weights[i], priority, i));
        }
    }

    let mut vinstrs_opt: Vec<Option<VInstruction>> = vinstrs.into_iter().map(Some).collect();
    let mut scheduled = Vec::with_capacity(n);

    while let Some((_, _, u)) = pq.pop() {
        scheduled.push(vinstrs_opt[u].take().expect("double-scheduled"));

        let start = offsets[u];
        let end = offsets[u + 1];
        for &v_u32 in &edges[start..end] {
            let v = v_u32 as usize;
            in_degree[v] -= 1;
            if in_degree[v] == 0 {
                #[allow(
                    clippy::cast_possible_wrap,
                    reason = "Indices fit well within i64 bounds"
                )]
                let priority = -(v as i64);
                pq.push((weights[v], priority, v));
            }
        }
    }

    debug_assert!(
        scheduled.len() == n,
        "cycle detected in VIR: scheduled {}, expected {}",
        scheduled.len(),
        n
    );
    scheduled
}
