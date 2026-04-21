use super::{VInstruction, VReg};
use rustc_hash::FxHashMap;
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::mem::swap;

#[derive(Clone)]
struct Extracted {
    min_idx: usize,
    step: usize,
    instr: VInstruction,
}

#[derive(Clone)]
struct Candidate {
    subset: Vec<VReg>,
    affected_instrs: Vec<usize>,
    min_idx: usize,
    savings: isize,
}

/// Extracts common subsets of operands from N-ary Add and Mul instructions.
///
/// Instead of just extracting pairs greedily, this pass maps all pairs of variables
/// to the instructions that contain them. It then intersects those instructions
/// to find the *maximal* subset of variables they all share, evaluating the global
/// arg pool savings: `(|S| - 1) * (K - 1) - 1`.
///
/// By extracting the most profitable subset at each step, it optimally compresses
/// the instruction stream and massively reduces the argument pool size for large expressions.
pub(in crate::evaluator::logic::bytecode::compile) fn optimize_nary_extraction(
    vinstrs: &mut Vec<VInstruction>,
    next_vreg: &mut u32,
) {
    let mut step = 0;
    let mut extracted = Vec::new();

    extract_optimal_subsets(vinstrs.as_mut_slice(), next_vreg, true, &mut extracted, &mut step);
    extract_optimal_subsets(vinstrs.as_mut_slice(), next_vreg, false, &mut extracted, &mut step);

    if extracted.is_empty() {
        return;
    }

    extracted.sort_unstable_by(|left, right| {
        left.min_idx
            .cmp(&right.min_idx)
            .then(left.step.cmp(&right.step))
    });

    let mut new_vinstrs = Vec::with_capacity(vinstrs.len() + extracted.len());
    let mut extracted_idx = 0;

    for (instr_idx, instr) in vinstrs.drain(..).enumerate() {
        while extracted_idx < extracted.len() && extracted[extracted_idx].min_idx <= instr_idx {
            new_vinstrs.push(extracted[extracted_idx].instr.clone());
            extracted_idx += 1;
        }
        new_vinstrs.push(instr);
    }

    while extracted_idx < extracted.len() {
        new_vinstrs.push(extracted[extracted_idx].instr.clone());
        extracted_idx += 1;
    }

    *vinstrs = new_vinstrs;
}

fn extract_optimal_subsets(
    vinstrs: &mut [VInstruction],
    next_vreg: &mut u32,
    is_mul: bool,
    extracted: &mut Vec<Extracted>,
    step: &mut usize,
) {
    let mut intersection = Vec::new();
    let mut scratch = Vec::new();
    let mut candidates = Vec::new();
    let mut selected = Vec::new();

    loop {
        let pair_to_instrs = build_pair_index(vinstrs, is_mul);
        collect_candidates(
            vinstrs,
            is_mul,
            pair_to_instrs,
            &mut intersection,
            &mut scratch,
            &mut candidates,
        );
        select_batch_candidates(&mut candidates, vinstrs.len(), &mut selected);

        if selected.is_empty() {
            break;
        }

        apply_candidate_batch(vinstrs, is_mul, next_vreg, extracted, step, &selected);
    }
}

fn build_pair_index(
    vinstrs: &[VInstruction],
    is_mul: bool,
) -> FxHashMap<(VReg, VReg), Vec<usize>> {
    // We rebuild the pair index each iteration to preserve the exact greedy behavior,
    // but use ordered Vecs instead of per-pair HashSets to avoid allocator churn.
    let mut pair_to_instrs: FxHashMap<(VReg, VReg), Vec<usize>> = FxHashMap::default();

    for (instr_idx, instr) in vinstrs.iter().enumerate() {
        let Some(srcs) = nary_srcs(instr, is_mul) else {
            continue;
        };

        if !is_extraction_candidate(srcs) {
            continue;
        }

        for_each_unique_pair(srcs, |left, right| {
            pair_to_instrs
                .entry((left, right))
                .or_default()
                .push(instr_idx);
        });
    }

    pair_to_instrs
}

fn collect_candidates(
    vinstrs: &[VInstruction],
    is_mul: bool,
    pair_to_instrs: FxHashMap<(VReg, VReg), Vec<usize>>,
    intersection: &mut Vec<VReg>,
    scratch: &mut Vec<VReg>,
    candidates: &mut Vec<Candidate>,
) {
    let mut candidate_by_subset: FxHashMap<Vec<VReg>, Candidate> = FxHashMap::default();
    candidates.clear();

    for instr_indices in pair_to_instrs.into_values() {
        if instr_indices.len() < 2 {
            continue;
        }

        let first_srcs = nary_srcs(&vinstrs[instr_indices[0]], is_mul)
            .expect("pair index only stores n-ary instruction indices");
        fill_unique_sorted(first_srcs, intersection);

        for &instr_idx in &instr_indices[1..] {
            let srcs = nary_srcs(&vinstrs[instr_idx], is_mul)
                .expect("pair index only stores n-ary instruction indices");
            intersect_unique_sorted_with_sorted_srcs(intersection, srcs, scratch);
            if intersection.len() < 2 {
                break;
            }
        }

        if intersection.len() < 2 {
            continue;
        }

        let subset_len = usize_to_isize(intersection.len());
        let mut affected_instrs = Vec::new();
        for &instr_idx in &instr_indices {
            let srcs = nary_srcs(&vinstrs[instr_idx], is_mul)
                .expect("pair index only stores n-ary instruction indices");
            if srcs.len() > intersection.len()
                && is_unique_sorted_subset_of_sorted_srcs(intersection, srcs)
            {
                affected_instrs.push(instr_idx);
            }
        }

        if affected_instrs.is_empty() {
            continue;
        }

        let min_idx = affected_instrs[0];
        let strict_superset_count = usize_to_isize(affected_instrs.len());
        let savings = (subset_len - 1) * (strict_superset_count - 1) - 1;
        
        let candidate = Candidate {
            subset: intersection.clone(),
            affected_instrs,
            min_idx,
            savings,
        };

        match candidate_by_subset.entry(intersection.clone()) {
            Entry::Occupied(mut occupied) => {
                let existing = occupied.get_mut();
                for &idx in &candidate.affected_instrs {
                    if !existing.affected_instrs.contains(&idx) {
                        existing.affected_instrs.push(idx);
                    }
                }
                existing.affected_instrs.sort_unstable();
                existing.min_idx = existing.affected_instrs[0];
                let count = usize_to_isize(existing.affected_instrs.len());
                existing.savings = (subset_len - 1) * (count - 1) - 1;
            }
            Entry::Vacant(vacant) => {
                if candidate.savings > 0 {
                    vacant.insert(candidate);
                }
            }
        }
    }

    // Keep only profitable candidates
    candidates.extend(candidate_by_subset.into_values().filter(|c| c.savings > 0));
}

fn select_batch_candidates(
    candidates: &mut [Candidate],
    instr_count: usize,
    selected: &mut Vec<Candidate>,
) {
    selected.clear();
    if candidates.is_empty() {
        return;
    }

    candidates.sort_unstable_by(candidate_cmp);

    let mut claimed_instrs = vec![false; instr_count];
    for candidate in candidates.iter() {
        if candidate
            .affected_instrs
            .iter()
            .any(|&instr_idx| claimed_instrs[instr_idx])
        {
            continue;
        }

        for &instr_idx in &candidate.affected_instrs {
            claimed_instrs[instr_idx] = true;
        }
        selected.push(candidate.clone());
    }
}

fn apply_candidate_batch(
    vinstrs: &mut [VInstruction],
    is_mul: bool,
    next_vreg: &mut u32,
    extracted: &mut Vec<Extracted>,
    step: &mut usize,
    selected: &[Candidate],
) {
    for candidate in selected {
        let new_reg = VReg::Temp(*next_vreg);
        *next_vreg += 1;

        for &instr_idx in &candidate.affected_instrs {
            let Some(srcs) = nary_srcs_mut(&mut vinstrs[instr_idx], is_mul) else {
                continue;
            };
            remove_unique_sorted_subset_once(srcs, &candidate.subset, new_reg);
        }

        extracted.push(Extracted {
            min_idx: candidate.min_idx,
            step: *step,
            instr: build_extracted_instr(&candidate.subset, is_mul, new_reg),
        });
        *step += 1;
    }
}

fn candidate_cmp(left: &Candidate, right: &Candidate) -> Ordering {
    right
        .savings
        .cmp(&left.savings)
        .then(right.affected_instrs.len().cmp(&left.affected_instrs.len()))
        .then(right.subset.len().cmp(&left.subset.len()))
        .then(left.min_idx.cmp(&right.min_idx))
        .then(left.subset.cmp(&right.subset))
}

fn build_extracted_instr(best_subset: &[VReg], is_mul: bool, new_reg: VReg) -> VInstruction {
    if is_mul {
        if best_subset.len() == 2 {
            VInstruction::Mul2 {
                dest: new_reg,
                a: best_subset[0],
                b: best_subset[1],
            }
        } else {
            VInstruction::Mul {
                dest: new_reg,
                srcs: best_subset.to_vec(),
            }
        }
    } else if best_subset.len() == 2 {
        VInstruction::Add2 {
            dest: new_reg,
            a: best_subset[0],
            b: best_subset[1],
        }
    } else {
        VInstruction::Add {
            dest: new_reg,
            srcs: best_subset.to_vec(),
        }
    }
}

fn nary_srcs(instr: &VInstruction, is_mul: bool) -> Option<&[VReg]> {
    match instr {
        VInstruction::Mul { srcs, .. } if is_mul => Some(srcs),
        VInstruction::Add { srcs, .. } if !is_mul => Some(srcs),
        _ => None,
    }
}

const fn nary_srcs_mut(instr: &mut VInstruction, is_mul: bool) -> Option<&mut Vec<VReg>> {
    match instr {
        VInstruction::Mul { srcs, .. } if is_mul => Some(srcs),
        VInstruction::Add { srcs, .. } if !is_mul => Some(srcs),
        _ => None,
    }
}

fn usize_to_isize(value: usize) -> isize {
    value.try_into().expect("optimizer sizes must fit in isize")
}

const fn is_extraction_candidate(srcs: &[VReg]) -> bool {
    let len = srcs.len();
    len > 2 && len <= 256
}

fn for_each_unique_pair(srcs: &[VReg], mut callback: impl FnMut(VReg, VReg)) {
    let mut left_idx = 0;
    while left_idx < srcs.len() {
        let left_reg = srcs[left_idx];
        let mut right_idx = left_idx + 1;
        while right_idx < srcs.len() {
            let right_reg = srcs[right_idx];
            callback(left_reg, right_reg);
            right_idx += 1;
            while right_idx < srcs.len() && srcs[right_idx] == right_reg {
                right_idx += 1;
            }
        }
        left_idx += 1;
        while left_idx < srcs.len() && srcs[left_idx] == left_reg {
            left_idx += 1;
        }
    }
}

fn fill_unique_sorted(srcs: &[VReg], out: &mut Vec<VReg>) {
    out.clear();
    out.reserve(srcs.len());

    let mut read_idx = 0;
    while read_idx < srcs.len() {
        let reg = srcs[read_idx];
        out.push(reg);
        read_idx += 1;
        while read_idx < srcs.len() && srcs[read_idx] == reg {
            read_idx += 1;
        }
    }
}

fn intersect_unique_sorted_with_sorted_srcs(
    intersection: &mut Vec<VReg>,
    srcs: &[VReg],
    scratch: &mut Vec<VReg>,
) {
    scratch.clear();

    let mut intersection_idx = 0;
    let mut src_idx = 0;
    while intersection_idx < intersection.len() && src_idx < srcs.len() {
        let intersection_reg = intersection[intersection_idx];
        let src_reg = srcs[src_idx];

        match intersection_reg.cmp(&src_reg) {
            Ordering::Less => {
                intersection_idx += 1;
            }
            Ordering::Greater => {
                src_idx += 1;
                while src_idx < srcs.len() && srcs[src_idx] == src_reg {
                    src_idx += 1;
                }
            }
            Ordering::Equal => {
                scratch.push(intersection_reg);
                intersection_idx += 1;
                src_idx += 1;
                while src_idx < srcs.len() && srcs[src_idx] == src_reg {
                    src_idx += 1;
                }
            }
        }
    }

    swap(intersection, scratch);
}

fn is_unique_sorted_subset_of_sorted_srcs(subset: &[VReg], srcs: &[VReg]) -> bool {
    let mut subset_idx = 0;
    let mut src_idx = 0;

    while subset_idx < subset.len() && src_idx < srcs.len() {
        let subset_reg = subset[subset_idx];
        let src_reg = srcs[src_idx];

        if subset_reg < src_reg {
            return false;
        }

        if subset_reg > src_reg {
            src_idx += 1;
            while src_idx < srcs.len() && srcs[src_idx] == src_reg {
                src_idx += 1;
            }
            continue;
        }

        subset_idx += 1;
        src_idx += 1;
        while src_idx < srcs.len() && srcs[src_idx] == src_reg {
            src_idx += 1;
        }
    }

    subset_idx == subset.len()
}

fn remove_unique_sorted_subset_once(srcs: &mut Vec<VReg>, subset: &[VReg], new_reg: VReg) {
    let mut subset_idx = 0;
    let mut write_idx = 0;

    for read_idx in 0..srcs.len() {
        let src_reg = srcs[read_idx];

        if subset_idx < subset.len() {
            let subset_reg = subset[subset_idx];
            if src_reg == subset_reg {
                subset_idx += 1;
                continue;
            }
        }

        srcs[write_idx] = src_reg;
        write_idx += 1;
    }

    debug_assert_eq!(
        subset_idx,
        subset.len(),
        "subset removal requires the caller to check containment first"
    );

    srcs.truncate(write_idx);
    let insert_idx = srcs.binary_search(&new_reg).unwrap_or_else(|idx| idx);
    srcs.insert(insert_idx, new_reg);
}
