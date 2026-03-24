use crate::evaluator::logic::instruction::Instruction;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct BaseVersion {
    reg: u32,
    version: u32,
}

#[derive(Clone, Copy)]
struct PolyInfo {
    idx: usize,
    dest: u32,
    const_idx: u32,
    degree: u32,
}

/// Optimization pass that detects multiple `PolyEval` instructions sharing the same base register
/// and selectively expands them into a shared forward power chain.
///
/// `PolyEval` is already a compact Horner evaluator, so this pass stays intentionally simple:
/// - find same-base polynomials,
/// - collect the non-zero exponents they actually need,
/// - if the group is small enough / dense enough, build `x^2, x^3, ...` once,
/// - rewrite each polynomial as a sparse sum over those shared powers.
#[allow(
    clippy::too_many_lines,
    clippy::float_cmp,
    reason = "Optimization pass logic"
)]
pub(super) fn optimize_shared_poly_bases(
    instructions: Vec<Instruction>,
    constants: &[f64],
    next_reg: &mut u32,
) -> Vec<Instruction> {
    const ALWAYS_EXPAND_MAX_DEGREE: u32 = 8;
    const MAX_FORWARD_CHAIN_DEGREE: u32 = 24;

    let mut groups: FxHashMap<BaseVersion, Vec<PolyInfo>> = FxHashMap::default();
    let mut current_versions: FxHashMap<u32, u32> = FxHashMap::default();
    for (idx, instr) in instructions.iter().enumerate() {
        if let Instruction::PolyEval {
            dest,
            x,
            const_idx,
            degree,
        } = *instr
        {
            let version = *current_versions.get(&x).unwrap_or(&0);
            groups
                .entry(BaseVersion { reg: x, version })
                .or_default()
                .push(PolyInfo {
                    idx,
                    dest,
                    const_idx,
                    degree,
                });
        }
        *current_versions.entry(instr.dest_reg()).or_default() += 1;
    }

    let mut expanded_results: FxHashMap<usize, Vec<Instruction>> = FxHashMap::default();
    let mut power_chains: FxHashMap<usize, Vec<Instruction>> = FxHashMap::default();

    for (base_version, polys) in groups {
        if polys.len() < 2 {
            continue;
        }

        let mut eligible = Vec::new();
        let mut needed_exponents = FxHashSet::default();
        let mut exponent_use_count: FxHashMap<u32, usize> = FxHashMap::default();
        let mut max_degree = 0_u32;

        for poly in polys {
            if poly.degree < 2 {
                continue;
            }

            let mut uses_high_power = false;
            for exp in 2..=poly.degree {
                let coeff = coeff_for(constants, poly.const_idx, poly.degree, exp);
                if coeff != 0.0 {
                    uses_high_power = true;
                    max_degree = max_degree.max(exp);
                    needed_exponents.insert(exp);
                    *exponent_use_count.entry(exp).or_default() += 1;
                }
            }

            if uses_high_power {
                eligible.push(poly);
            }
        }

        if eligible.len() < 2 || needed_exponents.is_empty() {
            continue;
        }

        let shared_exponent_count = exponent_use_count
            .values()
            .filter(|&&count| count > 1)
            .count();
        let exponent_span = max_degree.saturating_sub(1) as usize;
        let dense_exponents =
            exponent_span > 0 && needed_exponents.len().saturating_mul(2) >= exponent_span;

        let should_expand = max_degree <= ALWAYS_EXPAND_MAX_DEGREE
            || (max_degree <= MAX_FORWARD_CHAIN_DEGREE
                && (shared_exponent_count > 0 || dense_exponents || eligible.len() >= 3));
        if !should_expand {
            continue;
        }

        let (power_instrs, power_regs) =
            build_forward_chain(base_version.reg, max_degree, &needed_exponents, next_reg);

        for poly in &eligible {
            let expanded = expand_poly(*poly, base_version.reg, constants, &power_regs, next_reg);
            expanded_results.insert(poly.idx, expanded);
        }

        let first_idx = eligible
            .iter()
            .map(|poly| poly.idx)
            .min()
            .expect("eligible polynomial list should not be empty");
        power_chains.insert(first_idx, power_instrs);
    }

    if expanded_results.is_empty() {
        return instructions;
    }

    let mut out = Vec::with_capacity(instructions.len() + expanded_results.len() * 4);
    for (idx, instr) in instructions.into_iter().enumerate() {
        if let Some(chain) = power_chains.remove(&idx) {
            out.extend(chain);
        }
        if let Some(expanded) = expanded_results.remove(&idx) {
            out.extend(expanded);
        } else {
            out.push(instr);
        }
    }
    out
}

fn coeff_for(constants: &[f64], const_idx: u32, degree: u32, exp: u32) -> f64 {
    constants[(const_idx + degree - exp) as usize]
}

fn build_forward_chain(
    base_reg: u32,
    max_degree: u32,
    needed_exponents: &FxHashSet<u32>,
    next_reg: &mut u32,
) -> (Vec<Instruction>, FxHashMap<u32, u32>) {
    let mut out = Vec::with_capacity(max_degree.saturating_sub(1) as usize);
    let mut power_regs = FxHashMap::default();

    let r_sq = *next_reg;
    *next_reg += 1;
    out.push(Instruction::Square {
        dest: r_sq,
        src: base_reg,
    });
    if needed_exponents.contains(&2) {
        power_regs.insert(2, r_sq);
    }

    let mut prev = r_sq;
    for exp in 3..=max_degree {
        let reg = *next_reg;
        *next_reg += 1;
        out.push(Instruction::Mul {
            dest: reg,
            a: prev,
            b: base_reg,
        });
        if needed_exponents.contains(&exp) {
            power_regs.insert(exp, reg);
        }
        prev = reg;
    }

    (out, power_regs)
}

fn expand_poly(
    poly: PolyInfo,
    base_reg: u32,
    constants: &[f64],
    power_regs: &FxHashMap<u32, u32>,
    next_reg: &mut u32,
) -> Vec<Instruction> {
    let mut out = Vec::new();
    let dest = poly.dest;
    let mut initialized = false;

    let const_term_idx = poly.const_idx + poly.degree;
    let const_term = coeff_for(constants, poly.const_idx, poly.degree, 0);
    if const_term != 0.0 {
        out.push(Instruction::LoadConst {
            dest,
            const_idx: const_term_idx,
        });
        initialized = true;
    }

    if poly.degree >= 1 {
        let linear_idx = poly.const_idx + poly.degree - 1;
        let linear_term = coeff_for(constants, poly.const_idx, poly.degree, 1);
        if linear_term != 0.0 {
            if initialized {
                let tmp = *next_reg;
                *next_reg += 1;
                out.push(Instruction::MulConst {
                    dest: tmp,
                    src: base_reg,
                    const_idx: linear_idx,
                });
                out.push(Instruction::Add {
                    dest,
                    a: dest,
                    b: tmp,
                });
            } else {
                out.push(Instruction::MulConst {
                    dest,
                    src: base_reg,
                    const_idx: linear_idx,
                });
                initialized = true;
            }
        }
    }

    for exp in 2..=poly.degree {
        let coeff = coeff_for(constants, poly.const_idx, poly.degree, exp);
        if coeff == 0.0 {
            continue;
        }

        let power_reg = power_regs[&exp];
        let coeff_idx = poly.const_idx + poly.degree - exp;
        if initialized {
            let tmp = *next_reg;
            *next_reg += 1;
            out.push(Instruction::MulConst {
                dest: tmp,
                src: power_reg,
                const_idx: coeff_idx,
            });
            out.push(Instruction::Add {
                dest,
                a: dest,
                b: tmp,
            });
        } else {
            out.push(Instruction::MulConst {
                dest,
                src: power_reg,
                const_idx: coeff_idx,
            });
            initialized = true;
        }
    }

    if !initialized {
        out.push(Instruction::LoadConst {
            dest,
            const_idx: const_term_idx,
        });
    }

    out
}
