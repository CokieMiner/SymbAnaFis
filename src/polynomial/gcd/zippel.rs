//! Multivariate GCDs modulo one prime by dense and sparse interpolation.
//!
//! Terms are kept in strictly descending lexicographic order. A polynomial of
//! level $k$ uses the first $k$ positions; later positions are zero. Level one
//! is a dense Euclidean GCD. A higher level eliminates its last position
//! $x_{k}$ by evaluation and Newton interpolation (Brown 1971). With both inputs
//! made primitive over $\mathbb{F}_{p}[x_{k}]$, each image is scaled so that its
//! leading coefficient is $\gamma(\alpha)$ for $\gamma = \gcd(\operatorname{lc}
//! a, \operatorname{lc} b)$; the images are then values of $\gamma g /
//! \operatorname{lc}(g)$, whose primitive part is the GCD $g$. Interpolation
//! stops at the degree bound or when a further image agrees with the
//! interpolant.
//!
//! The first image of a level fixes the support of the inner GCD, and later
//! images are found by [`Shape::interpolate`]. If a sparse attempt fails (for
//! instance because free image scales are rank-deficient in the presence of
//! inner content), the level falls back to dense recursion; an image whose
//! leading monomial contradicts earlier ones ends the attempt.

use core::{iter::successors, mem::swap};

use alloc::{vec, vec::Vec};

use ntt_anafis::{PrimeField, Residue};

use crate::collections::FixedSet;

use super::{
    Dense, ModularTerm, Monomial, PolynomialLimitExceeded, PolynomialWork, Sampler, Shape,
    SparsePolynomial,
};

/// A polynomial by its last position: each distinct prefix monomial, in
/// descending order, with its dense coefficient polynomial in that position.
type Rows = Vec<(Monomial, Vec<Residue>)>;

/// Random points tried before an evaluation is abandoned.
const ATTEMPTS: usize = 8;

/// GCD state for one prime.
pub struct Zippel<'state> {
    field: PrimeField,
    dimensions: usize,
    /// Degree bounds of the GCD, by position.
    bounds: &'state [u32],
    sampler: &'state mut Sampler,
    work: &'state mut PolynomialWork,
}

/// Reusable ping-pong state for Newton interpolation in one variable.
struct Newton {
    rows: Rows,
    scratch: Rows,
    basis: Vec<Residue>,
    basis_scratch: Vec<Residue>,
    needed: usize,
}

impl<'state> Zippel<'state> {
    /// Creates the interpolation state for one catalog prime.
    pub const fn new(
        field: PrimeField,
        dimensions: usize,
        bounds: &'state [u32],
        sampler: &'state mut Sampler,
        work: &'state mut PolynomialWork,
    ) -> Self {
        Self {
            field,
            dimensions,
            bounds,
            sampler,
            work,
        }
    }

    /// Reduce `polynomial` modulo `self.field` and specialize positions from
    /// `level` on at independent random points in one pass, precomputing power
    /// tables for the specialized variables and coalescing equal prefixes before
    /// constructing prefix `Monomial`s.
    pub fn specialize(
        &mut self,
        polynomial: &SparsePolynomial,
        degrees: &[u32],
        level: usize,
    ) -> Result<Vec<ModularTerm>, PolynomialLimitExceeded> {
        let terms = polynomial.terms();
        self.work.spend(terms.len())?;
        let field = self.field;
        if level >= self.dimensions {
            let mut reduced = Vec::with_capacity(terms.len());
            for (powers, value) in terms {
                let residue = field.reduce(*value);
                if residue != Residue::ZERO {
                    reduced.push((powers.clone(), residue));
                }
            }
            return Ok(reduced);
        }
        let tables: Vec<Vec<Residue>> = (level..self.dimensions)
            .map(|variable| {
                let base = self.sampler.nonzero(field);
                let max_deg = degrees.get(variable).copied().unwrap_or(0);
                let len = usize::try_from(max_deg).map_or(1, |d| d.saturating_add(1));
                successors(Some(Residue::ONE), move |&p| Some(field.mul(p, base)))
                    .take(len)
                    .collect()
            })
            .collect();
        let mut result: Vec<ModularTerm> = Vec::with_capacity(terms.len());
        for (powers, value) in terms {
            let mut scaled = field.reduce(*value);
            if scaled == Residue::ZERO {
                continue;
            }
            for (&exponent, table) in powers.as_slice().iter().skip(level).zip(&tables) {
                if exponent != 0
                    && let Ok(idx) = usize::try_from(exponent)
                    && let Some(&pow) = table.get(idx)
                {
                    scaled = field.mul(scaled, pow);
                }
            }
            // Lexicographic order keeps equal prefixes in `0..level` adjacent.
            if let Some((last, sum)) = result.last_mut()
                && last.as_slice().get(..level) == powers.as_slice().get(..level)
            {
                *sum = field.add(*sum, scaled);
            } else {
                let prefix = Monomial::from_fn(self.dimensions, |index| {
                    if index < level {
                        powers.power(index)
                    } else {
                        0
                    }
                });
                result.push((prefix, scaled));
            }
        }
        result.retain(|(_, value)| *value != Residue::ZERO);
        Ok(result)
    }

    /// The GCD of two nonzero polynomials of `level`, normalized arbitrarily,
    /// or `None` when an unlucky point or prime makes the images inconsistent.
    pub fn gcd(
        &mut self,
        left: &[ModularTerm],
        right: &[ModularTerm],
        level: usize,
    ) -> Result<Option<Vec<ModularTerm>>, PolynomialLimitExceeded> {
        let Some(variable) = level.checked_sub(1) else {
            return Ok(None);
        };
        let field = self.field;
        let (Some(mut left_rows), Some(mut right_rows)) =
            (self.rows(left, variable)?, self.rows(right, variable)?)
        else {
            return Ok(None);
        };
        if variable == 0 {
            let (Some((prefix, first)), Some((_, second))) = (left_rows.pop(), right_rows.pop())
            else {
                return Ok(None);
            };
            let common = Dense::gcd(field, first, second, self.work)?;
            return self.terms(&[(prefix, common)], 0);
        }
        let (Some(left_content), Some(right_content)) = (
            self.primitive(&mut left_rows)?,
            self.primitive(&mut right_rows)?,
        ) else {
            return Ok(None);
        };
        let content = Dense::gcd(field, left_content, right_content, self.work)?;
        let Some(mut rows) = self.interpolate(&left_rows, &right_rows, variable, &content)? else {
            return Ok(None);
        };
        if self.primitive(&mut rows)?.is_none() {
            return Ok(None);
        }
        if content.len() > 1 {
            for (_, dense) in &mut rows {
                if !dense.is_empty() {
                    let Some(len) = dense
                        .len()
                        .checked_add(content.len())
                        .and_then(|n| n.checked_sub(1))
                    else {
                        return Ok(None);
                    };
                    let mut prod = vec![Residue::ZERO; len];
                    if !field.convolve(&mut prod, dense, &content) {
                        return Ok(None);
                    }
                    *dense = prod;
                }
            }
        }
        self.terms(&rows, variable)
    }

    /// Newton interpolation of the primitive multivariate GCD in `variable`
    /// from evaluated images at `variable = alpha`.
    fn interpolate(
        &mut self,
        left_rows: &Rows,
        right_rows: &Rows,
        variable: usize,
        content: &[Residue],
    ) -> Result<Option<Rows>, PolynomialLimitExceeded> {
        let field = self.field;
        let (Some((_, left_lead)), Some((_, right_lead))) = (left_rows.first(), right_rows.first())
        else {
            return Ok(None);
        };
        let gamma = Dense::gcd(field, left_lead.clone(), right_lead.clone(), self.work)?;
        // $\gamma g / \operatorname{lc}(g)$ has degree at most the primitive
        // bound plus $\deg \gamma$.
        let max_len = |rows: &Rows| rows.iter().map(|(_, d)| d.len()).max().unwrap_or(0);
        let content_deg = content.len().saturating_sub(1);
        let bound = self
            .bounds
            .get(variable)
            .and_then(|&limit| usize::try_from(limit).ok())
            .map_or(usize::MAX, |n| n.saturating_sub(content_deg))
            .min(max_len(left_rows).saturating_sub(1))
            .min(max_len(right_rows).saturating_sub(1));
        let needed = bound.saturating_add(gamma.len());
        let mut points: FixedSet<Residue> = FixedSet::default();
        points.reserve(needed);
        let (mut leading, mut shape, mut sparse_enabled) = (None, None::<Shape>, variable > 1);
        let mut newton = Newton {
            rows: Vec::new(),
            scratch: Vec::new(),
            basis: vec![Residue::ONE],
            basis_scratch: Vec::with_capacity(needed.saturating_add(1)),
            needed,
        };
        while points.len() < needed {
            let Some(point) = (0..ATTEMPTS)
                .map(|_| self.sampler.nonzero(field))
                .find(|&p| {
                    !points.contains(&p)
                        && field.evaluate(left_lead, p) != Residue::ZERO
                        && field.evaluate(right_lead, p) != Residue::ZERO
                })
            else {
                return Ok(None);
            };
            self.work
                .spend(left_rows.len().saturating_add(right_rows.len()))?;
            let [first, second] = [left_rows, right_rows].map(|rows| {
                rows.iter()
                    .filter_map(|(prefix, dense)| {
                        let value = field.evaluate(dense, point);
                        (value != Residue::ZERO).then(|| (prefix.clone(), value))
                    })
                    .collect::<Vec<_>>()
            });
            let mut image = if let Some(support) = &shape
                && let Some(got) = support.interpolate(
                    field,
                    &first,
                    &second,
                    variable,
                    self.sampler,
                    self.work,
                )? {
                got
            } else {
                if shape.is_some() {
                    shape = None;
                    sparse_enabled = false;
                }
                let Some(got) = self.gcd(&first, &second, variable)? else {
                    return Ok(None);
                };
                got
            };
            let Some((powers, inv)) = image.first().and_then(|(p, l)| Some((p, field.inv(*l)?)))
            else {
                return Ok(None);
            };
            if leading.get_or_insert_with(|| powers.clone()) != powers {
                return Ok(None);
            }
            let scale = field.mul(field.evaluate(&gamma, point), inv);
            if scale != Residue::ONE {
                for (_, value) in &mut image {
                    *value = field.mul(*value, scale);
                }
            }
            if shape.is_none() && sparse_enabled {
                shape = Some(Shape::new(&image));
            }
            points.insert(point);
            match newton.update(field, &image, point, self.work)? {
                Some(true) => {}
                Some(false) => break,
                None => return Ok(None),
            }
        }
        newton.rows.retain_mut(|(_, dense)| {
            while dense.last() == Some(&Residue::ZERO) {
                dense.pop();
            }
            !dense.is_empty()
        });
        Ok(Some(newton.rows))
    }

    /// Group terms of a level `variable + 1` polynomial into rows.
    fn rows(
        &mut self,
        terms: &[ModularTerm],
        variable: usize,
    ) -> Result<Option<Rows>, PolynomialLimitExceeded> {
        self.work.spend(terms.len())?;
        let mut rows: Rows = Vec::new();
        for (powers, value) in terms {
            let Some(index) = usize::try_from(powers.power(variable)).ok() else {
                return Ok(None);
            };
            if let Some((last, dense)) = rows.last_mut()
                && powers.matches_prefix(last, variable)
            {
                if let Some(slot) = dense.get_mut(index) {
                    *slot = *value;
                }
            } else {
                let (_, prefix) = powers.separate(variable);
                let mut dense = vec![Residue::ZERO; index.saturating_add(1)];
                if let Some(slot) = dense.last_mut() {
                    *slot = *value;
                }
                rows.push((prefix, dense));
            }
        }
        Ok(Some(rows))
    }

    /// Expand rows back into descending terms.
    fn terms(
        &self,
        rows: &[(Monomial, Vec<Residue>)],
        variable: usize,
    ) -> Result<Option<Vec<ModularTerm>>, PolynomialLimitExceeded> {
        let mut terms = Vec::new();
        for (prefix, dense) in rows {
            for (index, &value) in dense.iter().enumerate().rev() {
                if value == Residue::ZERO {
                    continue;
                }
                let mut powers = prefix.clone();
                let Some(degree) = u32::try_from(index).ok() else {
                    return Ok(None);
                };
                powers.shift(variable, degree, self.work)?;
                terms.push((powers, value));
            }
        }
        Ok(Some(terms))
    }

    /// Divide the rows by their monic GCD in the last position and return it,
    /// or `None` when a quotient is inexact.
    fn primitive(
        &mut self,
        rows: &mut Rows,
    ) -> Result<Option<Vec<Residue>>, PolynomialLimitExceeded> {
        let Some((shortest_index, (_, shortest))) = rows
            .iter()
            .enumerate()
            .min_by_key(|(_, (_, dense))| dense.len())
        else {
            return Ok(Some(Vec::new()));
        };
        if shortest.len() <= 1 {
            return Ok(Some(vec![Residue::ONE]));
        }
        let mut content = Dense::gcd(self.field, Vec::new(), shortest.clone(), self.work)?;
        for (index, (_, dense)) in rows.iter().enumerate() {
            if index != shortest_index {
                content = Dense::gcd(self.field, content, dense.clone(), self.work)?;
                if content.len() <= 1 {
                    return Ok(Some(content));
                }
            }
        }
        for (_, dense) in rows.iter_mut() {
            let Some(quotient) = Dense::quotient(self.field, dense, &content) else {
                return Ok(None);
            };
            *dense = quotient;
        }
        Ok(Some(content))
    }
}

impl Newton {
    /// Add one image to the Newton form $h + c q$, where $q$ is the product of
    /// $x - \alpha$ over the earlier points, and report whether it changed.
    fn update(
        &mut self,
        field: PrimeField,
        image: &[ModularTerm],
        point: Residue,
        work: &mut PolynomialWork,
    ) -> Result<Option<bool>, PolynomialLimitExceeded> {
        let eval = field.evaluate(&self.basis, point);
        let Some(scale) = (if eval == Residue::ONE {
            Some(Residue::ONE)
        } else {
            field.inv(eval)
        }) else {
            return Ok(None);
        };
        self.scratch.clear();
        self.scratch.reserve(self.rows.len().max(image.len()));
        let mut previous = self.rows.drain(..).peekable();
        let mut values = image.iter().peekable();
        let mut changed = false;
        loop {
            let next = values.peek().copied();
            let (powers, mut dense) =
                match previous.next_if(|(row, _)| next.is_none_or(|(key, _)| row >= key)) {
                    Some(row) => row,
                    None => match next {
                        Some((key, _)) => (key.clone(), Vec::with_capacity(self.needed)),
                        None => break,
                    },
                };
            work.spend(self.basis.len())?;
            let value = values
                .next_if(|(key, _)| *key == powers)
                .map_or(Residue::ZERO, |(_, residue)| *residue);
            let difference = field.sub(value, field.evaluate(&dense, point));
            if difference != Residue::ZERO {
                changed = true;
                if dense.len() < self.basis.len() {
                    dense.resize(self.basis.len(), Residue::ZERO);
                }
                field.mul_sub(
                    &mut dense,
                    &self.basis,
                    field.neg(field.mul(difference, scale)),
                );
            }
            self.scratch.push((powers, dense));
        }
        drop(previous);
        swap(&mut self.rows, &mut self.scratch);
        self.basis_scratch.clear();
        self.basis_scratch.push(Residue::ZERO);
        self.basis_scratch.extend_from_slice(&self.basis);
        field.mul_sub(&mut self.basis_scratch, &self.basis, point);
        swap(&mut self.basis, &mut self.basis_scratch);
        Ok(Some(changed))
    }
}
