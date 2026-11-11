//! Sparse interpolation of GCD images on a known support.
//!
//! For a support grouped by the exponent of $x_{1}$, the monic univariate GCDs
//! at $\beta^{l}$, $l = 1, 2, \ldots$, give for each group the values
//! $y_{l} = s_{l} g_{l} = \sum_{i} c_{i} m_{i}^{l}$ with nodes $m_{i}$, the
//! monomials at $\beta$, and image scales $s_{l}$ (Zippel 1979). The
//! coefficients solve a transposed Vandermonde system (Zippel 1990). The
//! scales follow from a single leading monomial, or else from the linear
//! recurrence that each group satisfies with its node polynomial (de Kleine,
//! Monagan, and Wittkopf 2005).

use core::{iter::successors, mem::swap};

use alloc::{vec, vec::Vec};

use ntt_anafis::{PrimeField, Residue};

use super::{Dense, ModularTerm, Monomial, PolynomialLimitExceeded, PolynomialWork, Sampler};

/// Random evaluation bases tried before abandoning a sparse image.
const ATTEMPTS: usize = 8;

/// Per-variable power tables or per-group node/master blocks in $\mathbb{F}_p$.
type ResidueBlocks = Vec<Vec<Residue>>;

/// The support of an image, grouped by descending exponent of the first
/// position, each group in descending order.
pub struct Shape {
    pub groups: Vec<(u32, Vec<Monomial>)>,
    /// Whether the leading group has one monomial, which fixes the scales.
    pub single: bool,
    /// The number of monomials.
    pub total: usize,
    /// The images per sparse interpolation, or `None` when free scales
    /// cannot be determined because only one group exists.
    pub count: Option<usize>,
    /// The leading exponent of the first position plus one.
    pub length: Option<usize>,
}

impl Shape {
    /// The support of `image`, whose terms are in descending order.
    pub fn new(image: &[ModularTerm]) -> Self {
        let mut groups: Vec<(u32, Vec<Monomial>)> = Vec::new();
        for (powers, _) in image {
            let exponent = powers.power(0);
            match groups.last_mut() {
                Some((last, monomials)) if *last == exponent => monomials.push(powers.clone()),
                _ => groups.push((exponent, vec![powers.clone()])),
            }
        }
        let largest = groups
            .iter()
            .map(|(_, monomials)| monomials.len())
            .max()
            .unwrap_or(0);
        let single = groups
            .first()
            .is_some_and(|(_, monomials)| monomials.len() == 1);
        let count = if single {
            Some(largest)
        } else {
            groups
                .len()
                .checked_sub(1)
                .filter(|others| *others != 0)
                .map(|others| image.len().saturating_sub(1).div_ceil(others).max(largest))
        };
        let length = groups
            .first()
            .and_then(|(exponent, _)| usize::try_from(*exponent).ok())
            .map(|exponent| exponent.saturating_add(1));
        Self {
            groups,
            single,
            total: image.len(),
            count,
            length,
        }
    }

    /// The GCD of two polynomials of `level` on this support, up to a scalar,
    /// by sparse interpolation in positions `1..level`.
    pub fn interpolate(
        &self,
        field: PrimeField,
        left: &[ModularTerm],
        right: &[ModularTerm],
        level: usize,
        sampler: &mut Sampler,
        work: &mut PolynomialWork,
    ) -> Result<Option<Vec<ModularTerm>>, PolynomialLimitExceeded> {
        let (Some(count), Some(length)) = (self.count, self.length) else {
            return Ok(None);
        };
        let max_exponents = self.max_inner_exponents(left, right, level);
        let main = |powers: &Monomial| usize::try_from(powers.power(0)).unwrap_or(0);
        let lengths = [left, right].map(|terms| {
            terms
                .first()
                .map_or(0, |(powers, _)| main(powers).saturating_add(1))
        });
        'attempt: for _ in 0..ATTEMPTS {
            let Some((tables, nodes)) = self.sample_nodes(field, &max_exponents, sampler) else {
                continue 'attempt;
            };
            let mut sources = [left, right].map(|terms| {
                terms
                    .iter()
                    .map(|(powers, value)| {
                        let factor = Self::eval_inner(field, powers, &tables);
                        (main(powers), factor, field.mul(*value, factor))
                    })
                    .collect::<Vec<_>>()
            });
            let mut group_rows = vec![Vec::with_capacity(count); self.groups.len()];
            for step in 0..count {
                let advance = step.saturating_add(1) < count;
                let mut univariate = lengths.map(|size| vec![Residue::ZERO; size]);
                for (dense, terms) in univariate.iter_mut().zip(&mut sources) {
                    work.spend(terms.len())?;
                    for (index, factor, value) in terms.iter_mut() {
                        if let Some(slot) = dense.get_mut(*index) {
                            *slot = field.add(*slot, *value);
                        }
                        if advance {
                            *value = field.mul(*value, *factor);
                        }
                    }
                }
                let [first, second] = univariate;
                if first.last() == Some(&Residue::ZERO) || second.last() == Some(&Residue::ZERO) {
                    continue 'attempt;
                }
                let common = Dense::gcd(field, first, second, work)?;
                if common.len() > length {
                    continue 'attempt;
                }
                if common.len() < length {
                    return Ok(None);
                }
                let mut expected = self
                    .groups
                    .iter()
                    .enumerate()
                    .map(|(group_index, (exponent, _))| (group_index, *exponent))
                    .peekable();
                for (index, &value) in common.iter().enumerate().rev() {
                    let exponent = u32::try_from(index).ok();
                    if let Some((group_index, _)) =
                        expected.next_if(|(_, next)| Some(*next) == exponent)
                    {
                        if let Some(series) = group_rows.get_mut(group_index) {
                            series.push(value);
                        }
                    } else if value != Residue::ZERO {
                        return Ok(None);
                    }
                }
            }
            return Ok(self.solve(field, &nodes, &group_rows));
        }
        Ok(None)
    }

    /// The image with this support from the sampled group series: `group_rows[g]`
    /// holds the `count` sampled values of group `g` at $\beta^{1}, \ldots, \beta^{\text{count}}$.
    pub fn solve(
        &self,
        field: PrimeField,
        nodes: &[Vec<Residue>],
        group_rows: &[Vec<Residue>],
    ) -> Option<Vec<ModularTerm>> {
        let count = group_rows.first()?.len();
        let (scales, masters): (Vec<Residue>, Option<ResidueBlocks>) = if self.single {
            let &lead = nodes.first()?.first()?;
            let fixed = successors(Some(lead), |&scale| Some(field.mul(scale, lead)))
                .take(count)
                .collect();
            (fixed, None)
        } else {
            let built: ResidueBlocks = nodes
                .iter()
                .map(|block| Self::master(field, block))
                .collect();
            let solved = Self::scales(field, count, group_rows, nodes, &built)?;
            (solved, Some(built))
        };
        let mut terms = Vec::with_capacity(self.total);
        let mut values = Vec::new();
        for (index, (((_, monomials), block), series)) in
            self.groups.iter().zip(nodes).zip(group_rows).enumerate()
        {
            values.clear();
            for (&sample, &scale) in series.iter().zip(&scales).take(block.len()) {
                values.push(field.mul(sample, scale));
            }
            if values.len() != block.len() {
                return None;
            }
            let coefficients = if let [node] = block.as_slice() {
                let &entry = values.first()?;
                vec![field.mul(entry, field.inv(*node)?)]
            } else if let Some(cached) = masters.as_ref().and_then(|list| list.get(index)) {
                Self::vandermonde(field, block, cached, &values)?
            } else {
                let master = Self::master(field, block);
                Self::vandermonde(field, block, &master, &values)?
            };
            for (powers, coefficient) in monomials.iter().zip(coefficients) {
                if coefficient != Residue::ZERO {
                    terms.push((powers.clone(), coefficient));
                }
            }
        }
        Some(terms)
    }

    fn max_inner_exponents(
        &self,
        left: &[ModularTerm],
        right: &[ModularTerm],
        level: usize,
    ) -> Vec<u32> {
        let mut max_exponents = vec![0_u32; level.saturating_sub(1)];
        for (powers, _) in left.iter().chain(right) {
            for (max, &exp) in max_exponents
                .iter_mut()
                .zip(powers.as_slice().iter().skip(1))
            {
                *max = (*max).max(exp);
            }
        }
        for (_, monomials) in &self.groups {
            for powers in monomials {
                for (max, &exp) in max_exponents
                    .iter_mut()
                    .zip(powers.as_slice().iter().skip(1))
                {
                    *max = (*max).max(exp);
                }
            }
        }
        max_exponents
    }

    /// Samples random base power tables for `1..level` and evaluates every
    /// group's monomial nodes, returning `None` if any group has colliding nodes.
    fn sample_nodes(
        &self,
        field: PrimeField,
        max_exponents: &[u32],
        sampler: &mut Sampler,
    ) -> Option<(ResidueBlocks, ResidueBlocks)> {
        let tables: ResidueBlocks = max_exponents
            .iter()
            .map(|&max_exp| {
                let base = sampler.nonzero(field);
                let len = usize::try_from(max_exp).map_or(1, |d| d.saturating_add(1));
                successors(Some(Residue::ONE), move |&p| Some(field.mul(p, base)))
                    .take(len)
                    .collect()
            })
            .collect();
        let mut nodes = Vec::with_capacity(self.groups.len());
        for (_, monomials) in &self.groups {
            let block: Vec<Residue> = monomials
                .iter()
                .map(|powers| Self::eval_inner(field, powers, &tables))
                .collect();
            let collided = match block.as_slice() {
                [] | [_] => false,
                [first, second] => first == second,
                _ => {
                    let mut sorted = block.clone();
                    sorted.sort_unstable();
                    sorted.windows(2).any(|pair| pair.first() == pair.last())
                }
            };
            if collided {
                return None;
            }
            nodes.push(block);
        }
        Some((tables, nodes))
    }

    #[inline]
    fn eval_inner(field: PrimeField, powers: &Monomial, tables: &[Vec<Residue>]) -> Residue {
        let mut product = Residue::ONE;
        for (&exponent, table) in powers.as_slice().iter().skip(1).zip(tables) {
            if exponent != 0
                && let Ok(idx) = usize::try_from(exponent)
                && let Some(&pow) = table.get(idx)
            {
                product = field.mul(product, pow);
            }
        }
        product
    }

    /// The image scales $s_{l}$, with $s_{1} = 1$, for which every block
    /// $y_{l} = s_{l} \operatorname{series}_{l}$ satisfies the recurrence of its
    /// node polynomial $P(z) = \prod (z - m_{i})$: $\sum_{q} P_{q} y_{l+q} = 0$,
    /// solved in one contiguous row-major matrix of width `count`.
    fn scales(
        field: PrimeField,
        count: usize,
        group_rows: &[Vec<Residue>],
        nodes: &[Vec<Residue>],
        masters: &[Vec<Residue>],
    ) -> Option<Vec<Residue>> {
        let mut matrix = Self::scale_matrix(field, count, group_rows, nodes, masters)?;
        for column in 1..count {
            let pivot = column.checked_sub(1)?;
            let found = matrix
                .chunks_exact(count)
                .enumerate()
                .skip(pivot)
                .find(|(_, eq)| eq.get(column).is_some_and(|v| *v != Residue::ZERO))
                .map(|(index, _)| index)?;
            let (_, tail_flat) = matrix.split_at_mut(pivot.checked_mul(count)?);
            if found > pivot {
                let offset = found.saturating_sub(pivot).checked_mul(count)?;
                let (first_part, second_part) = tail_flat.split_at_mut(offset);
                first_part
                    .get_mut(..count)?
                    .swap_with_slice(second_part.get_mut(..count)?);
            }
            let (pivot_row, below_flat) = tail_flat.split_at_mut(count);
            let pivot_entry = *pivot_row.get(column)?;
            if pivot_entry != Residue::ONE {
                let inverse = field.inv(pivot_entry)?;
                if let Some(first) = pivot_row.first_mut() {
                    *first = field.mul(*first, inverse);
                }
                field.scale(pivot_row.get_mut(column..)?, inverse);
            }
            let constant = *pivot_row.first()?;
            let tail = pivot_row.get(column..)?;
            for equation in below_flat.chunks_exact_mut(count) {
                let factor = *equation.get(column)?;
                if factor != Residue::ZERO {
                    let slot = equation.first_mut()?;
                    *slot = field.sub(*slot, field.mul(constant, factor));
                    field.mul_sub(equation.get_mut(column..)?, tail, factor);
                }
            }
        }
        Self::back_substitute(field, count, &matrix)
    }

    fn scale_matrix(
        field: PrimeField,
        count: usize,
        group_rows: &[Vec<Residue>],
        nodes: &[Vec<Residue>],
        masters: &[Vec<Residue>],
    ) -> Option<Vec<Residue>> {
        if count == 0 {
            return None;
        }
        let equations: usize = nodes
            .iter()
            .map(|block| count.saturating_sub(block.len()))
            .sum();
        let mut matrix = vec![Residue::ZERO; equations.checked_mul(count)?];
        let mut cursor = matrix.chunks_exact_mut(count);
        for ((block, master), series) in nodes.iter().zip(masters).zip(group_rows) {
            for shift in 0..count.saturating_sub(block.len()) {
                let equation = cursor.next()?;
                for ((slot, &sample), &coefficient) in equation
                    .iter_mut()
                    .skip(shift)
                    .zip(series.iter().skip(shift))
                    .zip(master)
                {
                    *slot = field.mul(coefficient, sample);
                }
            }
        }
        Some(matrix)
    }

    fn back_substitute(
        field: PrimeField,
        count: usize,
        matrix: &[Residue],
    ) -> Option<Vec<Residue>> {
        let solved = count.saturating_sub(1);
        if matrix
            .chunks_exact(count)
            .skip(solved)
            .any(|eq| eq.first().is_some_and(|v| *v != Residue::ZERO))
        {
            return None;
        }
        let mut scales = vec![Residue::ZERO; count];
        if let Some(first) = scales.first_mut() {
            *first = Residue::ONE;
        }
        for pivot in (0..solved).rev() {
            let column = pivot.saturating_add(1);
            let row_start = pivot.checked_mul(count)?;
            let equation = matrix.get(row_start..row_start.checked_add(count)?)?;
            let mut sum = *equation.first()?;
            for (&coefficient, &scale) in equation
                .get(column.saturating_add(1)..)
                .unwrap_or_default()
                .iter()
                .zip(scales.get(column.saturating_add(1)..).unwrap_or_default())
            {
                if coefficient != Residue::ZERO {
                    sum = field.add(sum, field.mul(coefficient, scale));
                }
            }
            if let Some(slot) = scales.get_mut(column) {
                *slot = field.neg(sum);
            }
        }
        Some(scales)
    }

    /// The coefficients $c_{i}$ of $y_{l} = \sum_{i} c_{i} m_{i}^{l}$,
    /// $l = 1, \ldots, n$, for distinct nonzero nodes $m_{i}$ and their master
    /// polynomial $P(z) = \prod (z - m_{j})$. With $Q_{i} = P / (z - m_{i})$,
    /// $\sum_{q} Q_{iq} y_{q+1} = c_{i} m_{i} Q_{i}(m_{i})$ (Zippel 1990).
    ///
    /// Montgomery's batch inversion computes all $n$ inverses with one field
    /// inversion and $3(n - 1)$ multiplications.
    fn vandermonde(
        field: PrimeField,
        nodes: &[Residue],
        master: &[Residue],
        values: &[Residue],
    ) -> Option<Vec<Residue>> {
        let len = nodes.len();
        if len == 0 {
            return Some(Vec::new());
        }
        let mut numerators = Vec::with_capacity(len);
        let mut denominators = Vec::with_capacity(len);
        let mut prefixes = Vec::with_capacity(len);
        let mut running = Residue::ONE;
        for &node in nodes {
            let mut carry = Residue::ONE;
            let mut numerator = Residue::ZERO;
            let mut derivative = Residue::ZERO;
            for (&coefficient, &value) in master.iter().rev().skip(1).zip(values.iter().rev()) {
                numerator = field.add(numerator, field.mul(carry, value));
                derivative = field.add(field.mul(derivative, node), carry);
                carry = field.add(coefficient, field.mul(node, carry));
            }
            let denominator = field.mul(node, derivative);
            numerators.push(numerator);
            denominators.push(denominator);
            running = field.mul(running, denominator);
            prefixes.push(running);
        }
        let mut inverse_all = field.inv(running)?;
        let mut coefficients = vec![Residue::ZERO; len];
        for index in (1..len).rev() {
            let prefix = *prefixes.get(index.saturating_sub(1))?;
            let denominator = *denominators.get(index)?;
            let numerator = *numerators.get(index)?;
            let inverse = field.mul(inverse_all, prefix);
            *coefficients.get_mut(index)? = field.mul(numerator, inverse);
            inverse_all = field.mul(inverse_all, denominator);
        }
        *coefficients.first_mut()? = field.mul(*numerators.first()?, inverse_all);
        Some(coefficients)
    }

    /// The monic node polynomial $\prod (z - m_{i})$ by ascending degree.
    fn master(field: PrimeField, nodes: &[Residue]) -> Vec<Residue> {
        let capacity = nodes.len().saturating_add(1);
        let mut master = Vec::with_capacity(capacity);
        master.push(Residue::ONE);
        let mut next = Vec::with_capacity(capacity);
        for &node in nodes {
            next.clear();
            next.push(Residue::ZERO);
            next.extend_from_slice(&master);
            field.mul_sub(&mut next, &master, node);
            swap(&mut master, &mut next);
        }
        master
    }
}
