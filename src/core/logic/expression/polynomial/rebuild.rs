//! Fast sparse polynomial reconstruction and partial term merging without
//! materializing intermediate product nodes for unchanged or monomial terms.

#![expect(
    unsafe_code,
    reason = "base indices, power spans, and direct-mapped cache slots are bounded by the tables and slices they index"
)]

use core::cmp::Ordering;

use alloc::{boxed::Box, vec, vec::Vec};

use num_anafis::Number;

use crate::collections::{FixedMap, FixedState};

use super::{BuiltinFunction, ExactIdentity, Expr, Node, NumericValue, Polynomial, Projection};

/// Direct-mapped stack cache slots for raised base powers `(position, exponent)`.
const POWER_CACHE_SLOTS: usize = 64;

/// Bitmask for indexing [`PowerCache`].
const POWER_CACHE_MASK: usize = POWER_CACHE_SLOTS - 1;

/// Stack cache of materialized `Expr` powers keyed by `(position, exponent)`.
type PowerCache = [(u32, u32, Option<Expr>); POWER_CACHE_SLOTS];

/// Stack cache of folded literal powers keyed by `(position, exponent)`.
type LiteralPowCache = [(u32, u32, Option<Number>); POWER_CACHE_SLOTS];

/// Ranked distinct base table, pointer-to-rank map, and optional Euler base rank.
type BaseTable = (Vec<Expr>, FixedMap<*const Node, u32>, Option<u32>);

/// Decomposed item referencing a slice `[start..start + len]` in a flat symbolic factor buffer.
struct DecomposedSpan<'expr> {
    literal: Option<&'expr NumericValue>,
    start: usize,
    len: usize,
}

/// Ranked mapped base referencing `[start..start + len]` in a flat `(rank, exp)` buffer.
struct RankedSpan {
    literal: Option<NumericValue>,
    start: usize,
    len: usize,
}

/// Prepared mapped bases and their ranked symbolic factor arena.
struct PreparedBases {
    table: Vec<Expr>,
    bases: Vec<RankedSpan>,
    sym: Vec<(u32, u32)>,
    euler_rank: Option<u32>,
}

/// Decomposed replacement terms and merged distinct base table.
struct ParsedReplacements<'expr> {
    table: BaseTable,
    items: Vec<(&'expr Expr, DecomposedSpan<'expr>)>,
    sym: Vec<(&'expr Expr, u32)>,
    constants: Vec<Expr>,
}

/// Lowered coefficient: either an untouched interned `Expr` or a newly folded `Number`.
enum LoweredCoeff<'expr> {
    Kept(&'expr Expr),
    Scaled(Option<Number>),
}

/// One lowered symbolic monomial stored in the flat power arena.
struct LoweredEntry<'expr> {
    first_key: u64,
    start: usize,
    len: usize,
    coefficient: LoweredCoeff<'expr>,
}

/// Surviving merged monomial entry with its materialized nonzero/unit coefficient handle.
struct MergedTerm<'arena> {
    powers: &'arena [(u32, u32)],
    coefficient: Expr,
}

impl Node {
    /// Rebuild `polynomial` from `mapped_bases` and optional `mapped_coeffs`
    /// (`None` when literal coefficients are unchanged), staying in flat sparse
    /// form whenever mapped bases and coefficients remain monomials.
    pub fn rebuild_polynomial(
        polynomial: &Polynomial,
        mapped_bases: &[&Expr],
        mapped_coeffs: Option<&[&Expr]>,
    ) -> Expr {
        if let Some(rebuilt) = try_monomial_rebuild(polynomial, mapped_bases, mapped_coeffs) {
            return rebuilt;
        }
        let owned_bases: Vec<Expr> = mapped_bases.iter().map(|&base| base.clone()).collect();
        let mut power_cache: PowerCache = [const { (0, 0, None) }; POWER_CACHE_SLOTS];
        let mut terms = Vec::with_capacity(polynomial.coefficients.len());
        for (index, (stored_coeff, term_powers)) in polynomial.terms().enumerate() {
            let Some(coeff) = mapped_coeffs.map_or(Some(stored_coeff), |c| c.get(index).copied())
            else {
                break;
            };
            let mut factors = Vec::with_capacity(term_powers.len().saturating_add(1));
            if !coeff.is_exact_one() {
                factors.push(coeff.clone());
            }
            for &(position, exponent) in term_powers {
                factors.push(cached_expr_power(
                    &owned_bases,
                    position,
                    exponent,
                    &mut power_cache,
                ));
            }
            terms.push(Expr::product(factors));
        }
        Expr::sum(terms)
    }
}

impl Polynomial {
    /// Materialize the canonical product expression for term `term_powers` using `cache`.
    pub fn materialize_term(
        &self,
        coefficient: &Expr,
        term_powers: &[(u32, u32)],
        cache: &mut [(u32, u32, Option<Expr>); 64],
    ) -> Expr {
        let mut factors = Vec::with_capacity(term_powers.len().saturating_add(1));
        if !coefficient.is_exact_one() {
            factors.push(coefficient.clone());
        }
        if self.simple_bases {
            for &(position, exponent) in term_powers {
                if exponent == 1 {
                    factors.push(cached_expr_power(&self.bases, position, 1, cache));
                }
            }
            for &(position, exponent) in term_powers {
                if exponent > 1 {
                    factors.push(cached_expr_power(&self.bases, position, exponent, cache));
                }
            }
            return Node::finish(factors, false);
        }
        for &(position, exponent) in term_powers {
            factors.push(cached_expr_power(&self.bases, position, exponent, cache));
        }
        Expr::product(factors)
    }

    /// Combine the untouched terms of `self` (those not in sorted `affected`)
    /// with the substituted `replacement_terms` without materializing untouched
    /// monomials into intermediate `Product` nodes when `replacement_terms`
    /// decompose into monomials.
    pub fn merge_substituted_terms(
        &self,
        affected: &[usize],
        replacement_terms: Vec<Expr>,
    ) -> Expr {
        if let Some(merged) = try_merge_substituted_flat(self, affected, &replacement_terms) {
            return merged;
        }
        let mut power_cache: PowerCache = [const { (0, 0, None) }; POWER_CACHE_SLOTS];
        let mut all_terms = Vec::with_capacity(
            self.coefficients
                .len()
                .saturating_add(replacement_terms.len()),
        );
        let mut skip = affected.iter().copied().peekable();
        for (index, (coefficient, term_powers)) in self.terms().enumerate() {
            if skip.next_if_eq(&index).is_some() {
                continue;
            }
            all_terms.push(self.materialize_term(coefficient, term_powers, &mut power_cache));
        }
        all_terms.extend(replacement_terms);
        Expr::sum(all_terms)
    }
}

fn prepare_ranked_bases(mapped_bases: &[&Expr]) -> Option<PreparedBases> {
    let mut spans = Vec::with_capacity(mapped_bases.len());
    let mut raw_sym: Vec<(&Expr, u32)> = Vec::with_capacity(mapped_bases.len());
    let mut distinct: Vec<&Expr> = Vec::with_capacity(mapped_bases.len());
    let mut rank_by_ptr: FixedMap<*const Node, u32> =
        FixedMap::with_capacity_and_hasher(mapped_bases.len(), FixedState::default());
    for &base in mapped_bases {
        let item = decompose_into(base, &mut raw_sym)?;
        for &(sym_base, _) in raw_sym.get(item.start..item.start.saturating_add(item.len))? {
            let ptr = sym_base.pointer();
            if !rank_by_ptr.contains_key(&ptr) {
                rank_by_ptr.insert(ptr, 0);
                distinct.push(sym_base);
            }
        }
        spans.push(item);
    }
    let (table, euler_rank) = assign_base_ranks(distinct, &mut rank_by_ptr)?;
    let mut ranked_sym: Vec<(u32, u32)> = Vec::with_capacity(raw_sym.len());
    let mut ranked_bases = Vec::with_capacity(spans.len());
    let mut scratch = Vec::new();
    for item in spans {
        scratch.clear();
        for &(base, exp) in raw_sym.get(item.start..item.start.saturating_add(item.len))? {
            scratch.push((*rank_by_ptr.get(&base.pointer())?, exp));
        }
        scratch.sort_unstable_by_key(|&(rank, _)| rank);
        merge_power_pairs(&mut scratch)?;
        let start = ranked_sym.len();
        ranked_sym.extend_from_slice(&scratch);
        ranked_bases.push(RankedSpan {
            literal: item.literal.cloned(),
            start,
            len: scratch.len(),
        });
    }
    Some(PreparedBases {
        table,
        bases: ranked_bases,
        sym: ranked_sym,
        euler_rank,
    })
}

#[expect(
    clippy::as_conversions,
    reason = "stored base positions are `u32` indices into `ranked_bases`"
)]
fn try_monomial_rebuild(
    polynomial: &Polynomial,
    mapped_bases: &[&Expr],
    mapped_coeffs: Option<&[&Expr]>,
) -> Option<Expr> {
    if let Some(coeffs) = mapped_coeffs
        && !coeffs.iter().all(|coeff| {
            coeff.is_exact_one()
                || coeff
                    .literal()
                    .is_some_and(|lit| lit.value().is_finite() && !lit.value().is_zero())
        })
    {
        return None;
    }
    let prepared = prepare_ranked_bases(mapped_bases)?;
    let mut lit_pow_cache: LiteralPowCache = [const { (0, 0, None) }; POWER_CACHE_SLOTS];
    let mut flat_powers: Vec<(u32, u32)> = Vec::with_capacity(polynomial.powers.len());
    let mut entries: Vec<LoweredEntry<'_>> = Vec::with_capacity(polynomial.coefficients.len());
    let mut constant_terms: Vec<Expr> = Vec::new();
    let mut scratch: Vec<(u32, u32)> = Vec::with_capacity(12);

    for (index, (stored_coeff, term_powers)) in polynomial.terms().enumerate() {
        let coeff_expr = mapped_coeffs.map_or(Some(stored_coeff), |c| c.get(index).copied())?;
        let mut scaled_num: Option<Number> = None;
        let mut scaled = false;
        scratch.clear();
        let mut ordered = true;
        let mut prev_rank = 0_u32;
        for &(old_pos, exp) in term_powers {
            // SAFETY: `old_pos` indexes `polynomial.bases`, which `prepared.bases` mirrors.
            let mapped_base = unsafe { prepared.bases.get_unchecked(old_pos as usize) };
            if let Some(lit) = &mapped_base.literal {
                if !scaled {
                    scaled = true;
                    if !coeff_expr.is_exact_one() {
                        scaled_num = Some(coeff_expr.literal()?.value().clone());
                    }
                }
                let powered = cached_literal_power(old_pos, exp, lit, &mut lit_pow_cache)?;
                scaled_num = Some(match scaled_num {
                    None => powered,
                    Some(current) => {
                        let prod = NumericValue::new(current).mul(&NumericValue::new(powered))?;
                        if !prod.is_finite() || prod.is_zero() {
                            return None;
                        }
                        prod
                    }
                });
            }
            // SAFETY: `[start..start + len]` was recorded when building `prepared.sym`.
            let sym_slice = unsafe {
                prepared.sym.get_unchecked(
                    mapped_base.start..mapped_base.start.saturating_add(mapped_base.len),
                )
            };
            for &(rank, base_exp) in sym_slice {
                let scaled_exp = if base_exp == 1 {
                    exp
                } else {
                    base_exp.checked_mul(exp)?
                };
                if !scratch.is_empty() && rank <= prev_rank {
                    ordered = false;
                }
                prev_rank = rank;
                scratch.push((rank, scaled_exp));
            }
        }
        if scratch.is_empty() {
            constant_terms.push(if scaled {
                scaled_num.map_or_else(|| Expr::from(1_i64), Expr::number)
            } else {
                coeff_expr.clone()
            });
            continue;
        }
        if !ordered {
            scratch.sort_unstable_by_key(|&(rank, _)| rank);
            merge_power_pairs(&mut scratch)?;
        }
        if prepared
            .euler_rank
            .is_some_and(|e_rank| scratch.iter().any(|&(r, e)| r == e_rank && e > 1))
        {
            return None;
        }
        let &(first_rank, first_exp) = scratch.first()?;
        let start = flat_powers.len();
        flat_powers.extend_from_slice(&scratch);
        entries.push(LoweredEntry {
            first_key: u64::from(first_rank).wrapping_shl(32) | u64::from(first_exp),
            start,
            len: scratch.len(),
            coefficient: if scaled {
                LoweredCoeff::Scaled(scaled_num)
            } else {
                LoweredCoeff::Kept(coeff_expr)
            },
        });
    }

    finish_lowered_entries(&prepared.table, &flat_powers, entries, constant_terms)
}

fn finish_lowered_entries(
    table: &[Expr],
    flat_powers: &[(u32, u32)],
    mut entries: Vec<LoweredEntry<'_>>,
    mut extra_terms: Vec<Expr>,
) -> Option<Expr> {
    let mut power_cache: PowerCache = [const { (0, 0, None) }; POWER_CACHE_SLOTS];
    entries.sort_unstable_by(|left, right| {
        left.first_key.cmp(&right.first_key).then_with(|| {
            // SAFETY: each entry's `[start..start + len]` lies inside `flat_powers` with `len >= 1`.
            let (left_tail, right_tail) = unsafe {
                (
                    flat_powers.get_unchecked(
                        left.start.saturating_add(1)..left.start.saturating_add(left.len),
                    ),
                    flat_powers.get_unchecked(
                        right.start.saturating_add(1)..right.start.saturating_add(right.len),
                    ),
                )
            };
            left_tail.cmp(right_tail)
        })
    });

    let one = Expr::from(1_i64);
    let unit_val = NumericValue::new(Number::one());
    let mut merged_terms: Vec<MergedTerm<'_>> = Vec::with_capacity(entries.len());
    let mut cursor = 0_usize;
    while let Some(first_entry) = entries.get(cursor) {
        // SAFETY: `[start..start + len]` was recorded when pushing into `flat_powers`.
        let powers = unsafe {
            flat_powers
                .get_unchecked(first_entry.start..first_entry.start.saturating_add(first_entry.len))
        };
        let mut run_end = cursor.saturating_add(1);
        while let Some(candidate) = entries.get(run_end) {
            // SAFETY: as above.
            let cand_powers = unsafe {
                flat_powers
                    .get_unchecked(candidate.start..candidate.start.saturating_add(candidate.len))
            };
            if cand_powers != powers {
                break;
            }
            run_end = run_end.saturating_add(1);
        }
        if run_end == cursor.saturating_add(1) {
            let coefficient = match &first_entry.coefficient {
                LoweredCoeff::Kept(expr) => (*expr).clone(),
                LoweredCoeff::Scaled(num) => num.clone().map_or_else(|| one.clone(), Expr::number),
            };
            merged_terms.push(MergedTerm {
                powers,
                coefficient,
            });
        } else {
            let group = entries.get(cursor..run_end)?;
            let mut numeric_storage: Vec<NumericValue> = Vec::with_capacity(group.len());
            for item in group {
                let value = match &item.coefficient {
                    LoweredCoeff::Kept(expr) => {
                        if expr.is_exact_one() {
                            unit_val.clone()
                        } else {
                            expr.literal()?.clone()
                        }
                    }
                    LoweredCoeff::Scaled(num) => num
                        .clone()
                        .map_or_else(|| unit_val.clone(), NumericValue::new),
                };
                numeric_storage.push(value);
            }
            numeric_storage.sort_unstable_by(NumericValue::compare);
            let combined = Node::finite_fold(numeric_storage.iter(), true)?;
            if !combined.is_finite() {
                return None;
            }
            if combined.is_zero() {
                let mut factors = Vec::with_capacity(powers.len().saturating_add(1));
                factors.push(Expr::number(combined));
                for &(pos, exp) in powers {
                    factors.push(cached_expr_power(table, pos, exp, &mut power_cache));
                }
                extra_terms.push(Node::normalize_product(factors));
            } else {
                merged_terms.push(MergedTerm {
                    powers,
                    coefficient: Expr::number(combined),
                });
            }
        }
        cursor = run_end;
    }
    Some(pack_merged_monomials(
        table,
        merged_terms,
        extra_terms,
        &mut power_cache,
    ))
}

#[expect(
    clippy::as_conversions,
    reason = "stored base positions are valid `u32` indices into `table`, `exponents`, and `renumbering`"
)]
fn pack_merged_monomials(
    table: &[Expr],
    merged_terms: Vec<MergedTerm<'_>>,
    mut extra_terms: Vec<Expr>,
    power_cache: &mut PowerCache,
) -> Expr {
    let mut exponents = vec![(0_u32, false); table.len()];
    for term in &merged_terms {
        for &(pos, exp) in term.powers {
            // SAFETY: `pos` is a rank below `table.len()`.
            let slot = unsafe { exponents.get_unchecked_mut(pos as usize) };
            if slot.0 == 0 {
                slot.0 = exp;
            } else {
                slot.1 |= slot.0 != exp;
            }
        }
    }
    let all_distinct = exponents
        .iter()
        .all(|&(first, distinct)| first == 0 || distinct);
    let mut poly_terms: Vec<MergedTerm<'_>> = Vec::with_capacity(merged_terms.len());
    for term in merged_terms {
        let keep_in_poly = all_distinct
            || term.powers.iter().any(|&(pos, _)| {
                // SAFETY: `pos` is a rank below `exponents.len()`.
                unsafe { exponents.get_unchecked(pos as usize) }.1
            });
        if keep_in_poly {
            poly_terms.push(term);
        } else {
            let mut factors = Vec::with_capacity(term.powers.len().saturating_add(1));
            if !term.coefficient.is_exact_one() {
                factors.push(term.coefficient);
            }
            for &(pos, exp) in term.powers {
                factors.push(cached_expr_power(table, pos, exp, power_cache));
            }
            extra_terms.push(Expr::product(factors));
        }
    }
    if poly_terms.is_empty() {
        return Expr::sum(extra_terms);
    }
    let mut used = vec![false; table.len()];
    let mut power_count = 0_usize;
    for term in &poly_terms {
        power_count = power_count.saturating_add(term.powers.len());
        for &(pos, _) in term.powers {
            // SAFETY: `pos` is below `table.len()`.
            *unsafe { used.get_unchecked_mut(pos as usize) } = true;
        }
    }
    let mut final_bases = Vec::with_capacity(table.len());
    let mut renumbering = Vec::with_capacity(table.len());
    let mut next_rank = 0_u32;
    for (base, &in_use) in table.iter().zip(&used) {
        renumbering.push(next_rank);
        if in_use {
            final_bases.push(base.clone());
            next_rank = next_rank.saturating_add(1);
        }
    }
    let all_used = final_bases.len() == table.len();
    let simple_bases = final_bases
        .iter()
        .all(|base| matches!(base.node(), Node::Symbol(_) | Node::Pi | Node::Euler));
    let mut coefficients = Vec::with_capacity(poly_terms.len());
    let mut lengths = Vec::with_capacity(poly_terms.len());
    let mut powers = Vec::with_capacity(power_count);
    for term in poly_terms {
        coefficients.push(term.coefficient);
        lengths.push(term.powers.len());
        if all_used {
            powers.extend_from_slice(term.powers);
        } else {
            powers.extend(term.powers.iter().map(|&(pos, exp)| {
                (
                    // SAFETY: `renumbering` has length `table.len()`.
                    *unsafe { renumbering.get_unchecked(pos as usize) },
                    exp,
                )
            }));
        }
    }
    let built = Expr::intern(Node::Poly(Box::new(Polynomial {
        bases: final_bases,
        coefficients,
        lengths,
        powers,
        simple_bases,
        projection: Projection::default(),
    })));
    if extra_terms.is_empty() {
        built
    } else {
        extra_terms.push(built);
        Expr::sum(extra_terms)
    }
}

fn parse_replacement_monomials<'expr>(
    polynomial: &'expr Polynomial,
    replacement_terms: &'expr [Expr],
) -> Option<ParsedReplacements<'expr>> {
    let mut expanded: Vec<&'expr Expr> = Vec::with_capacity(replacement_terms.len());
    for expr in replacement_terms {
        match expr.node() {
            Node::Sum(children) => expanded.extend(children.iter()),
            Node::Poly(_) => return None,
            Node::Number(_)
            | Node::ImaginaryUnit
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Product(_)
            | Node::Div(..)
            | Node::Pow(..)
            | Node::Derivative { .. } => expanded.push(expr),
        }
    }
    let mut parsed = Vec::with_capacity(expanded.len());
    let mut sym_buf: Vec<(&'expr Expr, u32)> = Vec::with_capacity(expanded.len().saturating_mul(2));
    let mut constants: Vec<Expr> = Vec::new();
    let mut distinct: Vec<&'expr Expr> = polynomial.bases.iter().collect();
    let mut rank_by_ptr: FixedMap<*const Node, u32> =
        FixedMap::with_capacity_and_hasher(distinct.len().saturating_add(8), FixedState::default());
    for base in &polynomial.bases {
        rank_by_ptr.insert(base.pointer(), 0);
    }
    for &term in &expanded {
        if let Some(lit) = term.literal() {
            if !lit.value().is_finite() {
                return None;
            }
            constants.push(term.clone());
            continue;
        }
        let span = decompose_into(term, &mut sym_buf)?;
        if span.len == 0 {
            constants.push(term.clone());
            continue;
        }
        for &(sym_base, _) in sym_buf.get(span.start..span.start.saturating_add(span.len))? {
            let ptr = sym_base.pointer();
            if !rank_by_ptr.contains_key(&ptr) {
                rank_by_ptr.insert(ptr, 0);
                distinct.push(sym_base);
            }
        }
        parsed.push((term, span));
    }
    let (table, euler_rank) = assign_base_ranks(distinct, &mut rank_by_ptr)?;
    Some(ParsedReplacements {
        table: (table, rank_by_ptr, euler_rank),
        items: parsed,
        sym: sym_buf,
        constants,
    })
}

#[expect(
    clippy::as_conversions,
    reason = "stored base positions are valid `u32` indices into `polynomial.bases` and `old_to_new`"
)]
fn try_merge_substituted_flat(
    polynomial: &Polynomial,
    affected: &[usize],
    replacement_terms: &[Expr],
) -> Option<Expr> {
    let ParsedReplacements {
        table: (table, rank_by_ptr, euler_rank),
        items: parsed_replacements,
        sym: sym_buf,
        constants: constant_terms,
    } = parse_replacement_monomials(polynomial, replacement_terms)?;
    let mut old_to_new: Vec<u32> = Vec::with_capacity(polynomial.bases.len());
    let mut identity_ranks = table.len() == polynomial.bases.len();
    for (old_idx, base) in polynomial.bases.iter().enumerate() {
        let new_rank = *rank_by_ptr.get(&base.pointer())?;
        if new_rank as usize != old_idx {
            identity_ranks = false;
        }
        old_to_new.push(new_rank);
    }

    let total_terms = polynomial
        .coefficients
        .len()
        .saturating_sub(affected.len())
        .saturating_add(parsed_replacements.len());
    let mut flat_powers: Vec<(u32, u32)> =
        Vec::with_capacity(polynomial.powers.len().saturating_add(sym_buf.len()));
    let mut entries: Vec<LoweredEntry<'_>> = Vec::with_capacity(total_terms);

    let mut skip = affected.iter().copied().peekable();
    for (index, (coeff, term_powers)) in polynomial.terms().enumerate() {
        if skip.next_if_eq(&index).is_some() {
            continue;
        }
        let start = flat_powers.len();
        if identity_ranks {
            flat_powers.extend_from_slice(term_powers);
        } else {
            for &(old_pos, exp) in term_powers {
                // SAFETY: `old_pos` indexes `polynomial.bases`, which `old_to_new` mirrors.
                let mapped_rank = *unsafe { old_to_new.get_unchecked(old_pos as usize) };
                flat_powers.push((mapped_rank, exp));
            }
        }
        // Old bases were in strict `Node::compare` order, so `old_to_new` is strictly ascending.
        // SAFETY: `term_powers` in a `Polynomial` is non-empty, so `start < flat_powers.len()`.
        let &(first_rank, first_exp) = unsafe { flat_powers.get_unchecked(start) };
        entries.push(LoweredEntry {
            first_key: u64::from(first_rank).wrapping_shl(32) | u64::from(first_exp),
            start,
            len: term_powers.len(),
            coefficient: LoweredCoeff::Kept(coeff),
        });
    }

    let mut scratch: Vec<(u32, u32)> = Vec::with_capacity(12);
    for (term, span) in parsed_replacements {
        scratch.clear();
        for &(sym_base, exp) in sym_buf.get(span.start..span.start.saturating_add(span.len))? {
            scratch.push((*rank_by_ptr.get(&sym_base.pointer())?, exp));
        }
        scratch.sort_unstable_by_key(|&(rank, _)| rank);
        merge_power_pairs(&mut scratch)?;
        if euler_rank.is_some_and(|e_rank| scratch.iter().any(|&(r, e)| r == e_rank && e > 1)) {
            return None;
        }
        let &(first_rank, first_exp) = scratch.first()?;
        let start = flat_powers.len();
        flat_powers.extend_from_slice(&scratch);
        let coefficient = if span.literal.is_some()
            && let Node::Product(factors) = term.node()
        {
            LoweredCoeff::Kept(factors.first()?)
        } else {
            LoweredCoeff::Scaled(None)
        };
        entries.push(LoweredEntry {
            first_key: u64::from(first_rank).wrapping_shl(32) | u64::from(first_exp),
            start,
            len: scratch.len(),
            coefficient,
        });
    }

    finish_lowered_entries(&table, &flat_powers, entries, constant_terms)
}

fn decompose_factor(factor: &Expr) -> Option<(&Expr, u32)> {
    let atomic_base = |expr: &Expr| match expr.node() {
        Node::Symbol(_) | Node::Pi | Node::Sum(_) | Node::Poly(_) | Node::Derivative { .. } => true,
        Node::Function { name, .. } => *name != BuiltinFunction::Sqrt.function(),
        Node::Number(_)
        | Node::ImaginaryUnit
        | Node::Euler
        | Node::Product(_)
        | Node::Div(..)
        | Node::Pow(..) => false,
    };
    if atomic_base(factor) || matches!(factor.node(), Node::Euler) {
        return Some((factor, 1));
    }
    if matches!(factor.node(), Node::Pow(..))
        && let Some((inner, power)) = factor.positive_factor()
        && atomic_base(inner)
        && let Ok(exp_u32) = u32::try_from(power)
    {
        return Some((inner, exp_u32));
    }
    None
}

fn decompose_into<'expr>(
    expression: &'expr Expr,
    symbolic_out: &mut Vec<(&'expr Expr, u32)>,
) -> Option<DecomposedSpan<'expr>> {
    let start = symbolic_out.len();
    if let Some(lit) = expression.literal() {
        if lit.value().is_finite() && !lit.value().is_zero() {
            return Some(DecomposedSpan {
                literal: Some(lit),
                start,
                len: 0,
            });
        }
        return None;
    }
    if let Some(pair) = decompose_factor(expression) {
        symbolic_out.push(pair);
        return Some(DecomposedSpan {
            literal: None,
            start,
            len: 1,
        });
    }
    if let Node::Product(factors) = expression.node() {
        let (first, rest) = factors.split_first()?;
        let (literal, sym_slice) = if let Some(lit) = first.literal() {
            if !lit.value().is_finite() || lit.value().is_zero() {
                return None;
            }
            (Some(lit), rest)
        } else {
            (None, factors.as_slice())
        };
        if sym_slice.is_empty() {
            return None;
        }
        for factor in sym_slice {
            let Some(pair) = decompose_factor(factor) else {
                symbolic_out.truncate(start);
                return None;
            };
            symbolic_out.push(pair);
        }
        return Some(DecomposedSpan {
            literal,
            start,
            len: sym_slice.len(),
        });
    }
    None
}

fn assign_base_ranks(
    mut distinct: Vec<&Expr>,
    rank_by_ptr: &mut FixedMap<*const Node, u32>,
) -> Option<(Vec<Expr>, Option<u32>)> {
    distinct.sort_unstable_by(|left, right| Node::compare(left, right));
    if distinct
        .iter()
        .zip(distinct.iter().skip(1))
        .any(|(left, right)| Node::compare(left, right) == Ordering::Equal)
    {
        return None;
    }
    let mut euler_rank = None;
    for (index, &base) in distinct.iter().enumerate() {
        let rank_u32 = u32::try_from(index).ok()?;
        if matches!(base.node(), Node::Euler) {
            euler_rank = Some(rank_u32);
        }
        if let Some(slot) = rank_by_ptr.get_mut(&base.pointer()) {
            *slot = rank_u32;
        }
    }
    let table = distinct.into_iter().cloned().collect();
    Some((table, euler_rank))
}

fn merge_power_pairs(pairs: &mut Vec<(u32, u32)>) -> Option<()> {
    let mut write = 0_usize;
    for read in 0..pairs.len() {
        let &(rank, exp) = pairs.get(read)?;
        if write > 0
            && let Some(prev) = pairs.get_mut(write.saturating_sub(1))
            && prev.0 == rank
        {
            prev.1 = prev.1.checked_add(exp)?;
        } else {
            if let Some(slot) = pairs.get_mut(write) {
                *slot = (rank, exp);
            }
            write = write.saturating_add(1);
        }
    }
    pairs.truncate(write);
    Some(())
}

#[expect(
    clippy::as_conversions,
    reason = "base positions widen losslessly to slice and cache indices"
)]
fn cached_expr_power(bases: &[Expr], position: u32, exponent: u32, cache: &mut PowerCache) -> Expr {
    if exponent == 1 {
        // SAFETY: callers pass valid base positions of `bases`.
        return unsafe { bases.get_unchecked(position as usize) }.clone();
    }
    let slot = ((position as usize).wrapping_mul(31) ^ (exponent as usize).wrapping_mul(13))
        & POWER_CACHE_MASK;
    // SAFETY: `slot` is masked below `POWER_CACHE_SLOTS`.
    let entry = unsafe { cache.get_unchecked_mut(slot) };
    if entry.0 == position
        && entry.1 == exponent
        && let Some(cached) = &entry.2
    {
        return cached.clone();
    }
    // SAFETY: callers pass valid base positions of `bases`.
    let base = unsafe { bases.get_unchecked(position as usize) }.clone();
    let built = Node::normalize_power(base, Expr::from(i64::from(exponent)));
    *entry = (position, exponent, Some(built.clone()));
    built
}

#[expect(
    clippy::as_conversions,
    reason = "base positions widen losslessly to cache indices"
)]
fn cached_literal_power(
    position: u32,
    exponent: u32,
    literal: &NumericValue,
    cache: &mut LiteralPowCache,
) -> Option<Number> {
    if exponent == 1 {
        return Some(literal.value().clone());
    }
    let slot = ((position as usize).wrapping_mul(31) ^ (exponent as usize).wrapping_mul(13))
        & POWER_CACHE_MASK;
    // SAFETY: `slot` is masked below `POWER_CACHE_SLOTS`.
    let entry = unsafe { cache.get_unchecked_mut(slot) };
    if entry.0 == position
        && entry.1 == exponent
        && let Some(cached) = &entry.2
    {
        return Some(cached.clone());
    }
    let powered = literal.pow(&NumericValue::new(Number::from_int(i64::from(exponent))))?;
    if !powered.is_finite() || powered.is_zero() {
        return None;
    }
    *entry = (position, exponent, Some(powered.clone()));
    Some(powered)
}
