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
//! images are found by sparse interpolation on that [`Shape`]. An image
//! that contradicts the support, degree, or leading monomial of an earlier one
//! ends the attempt, and the caller retries with other points or another prime.

use core::{iter::once, mem::take};

use alloc::{vec, vec::Vec};

use ntt_anafis::{PrimeField, Residue};

use crate::errors::SimplifyError;

use super::{Dense, Monomial, PolynomialWork, Sampler, Shape};

/// A monomial with its nonzero coefficient modulo one prime.
pub type ModularTerm = (Monomial, Residue);

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

impl<'state> Zippel<'state> {
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

    /// Evaluate the positions from `level` on at independent random points.
    ///
    /// A GCD free of those variables divides the image, and for random
    /// points the images of two inputs share no further factor.
    pub fn specialize(
        &mut self,
        terms: &[ModularTerm],
        level: usize,
    ) -> Result<Vec<ModularTerm>, SimplifyError> {
        if level >= self.dimensions {
            return Ok(terms.to_vec());
        }
        self.work.spend(terms.len())?;
        let field = self.field;
        let point: Vec<_> = (level..self.dimensions)
            .map(|_| self.sampler.nonzero(field))
            .collect();
        let mut result: Vec<ModularTerm> = Vec::with_capacity(terms.len());
        for (powers, value) in terms {
            let scaled = powers.as_slice().iter().skip(level).zip(&point).fold(
                *value,
                |product, (&exponent, &base)| {
                    field.mul(product, field.pow(base, u64::from(exponent)))
                },
            );
            let prefix = Monomial::from_fn(self.dimensions, |index| {
                if index < level {
                    powers.power(index)
                } else {
                    0
                }
            });
            // Lexicographic order keeps equal prefixes adjacent.
            match result.last_mut() {
                Some((last, sum)) if *last == prefix => *sum = field.add(*sum, scaled),
                _ => result.push((prefix, scaled)),
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
    ) -> Result<Option<Vec<ModularTerm>>, SimplifyError> {
        let Some(variable) = level.checked_sub(1) else {
            return Ok(None);
        };
        let field = self.field;
        let mut left_rows = self.rows(left, variable)?;
        let mut right_rows = self.rows(right, variable)?;
        if variable == 0 {
            // A univariate input is one row with the zero prefix.
            let (Some((prefix, first)), Some((_, second))) = (left_rows.pop(), right_rows.pop())
            else {
                return Ok(None);
            };
            let common = Dense::gcd(field, first, second, self.work)?;
            return self.terms(&[(prefix, common)], 0).map(Some);
        }
        let (Some(left_content), Some(right_content)) = (
            self.primitive(&mut left_rows)?,
            self.primitive(&mut right_rows)?,
        ) else {
            return Ok(None);
        };
        let content = Dense::gcd(field, left_content, right_content, self.work)?;
        let (Some(left_lead), Some(right_lead)) = (
            left_rows.first().map(|(_, dense)| dense.clone()),
            right_rows.first().map(|(_, dense)| dense.clone()),
        ) else {
            return Ok(None);
        };
        let gamma = Dense::gcd(field, left_lead.clone(), right_lead.clone(), self.work)?;
        // $\gamma g / \operatorname{lc}(g)$ has degree at most the bound plus
        // $\deg \gamma$.
        let length = |rows: &Rows| rows.iter().map(|(_, dense)| dense.len()).max().unwrap_or(0);
        let bound = self
            .bounds
            .get(variable)
            .and_then(|&limit| usize::try_from(limit).ok())
            .unwrap_or(usize::MAX)
            .min(length(&left_rows).saturating_sub(1))
            .min(length(&right_rows).saturating_sub(1));
        let needed = bound.saturating_add(gamma.len());
        let mut points: Vec<Residue> = Vec::new();
        let mut leading: Option<Monomial> = None;
        let mut shape: Option<Shape> = None;
        let mut interpolant: Rows = Vec::new();
        let mut basis = vec![Residue::ONE];
        while points.len() < needed {
            let Some(point) = self.point(&left_lead, &right_lead, &points) else {
                return Ok(None);
            };
            let first = self.evaluate(&left_rows, point)?;
            let second = self.evaluate(&right_rows, point)?;
            let candidate = match &shape {
                Some(support) => self.image(&first, &second, support, variable)?,
                None => self.gcd(&first, &second, variable)?,
            };
            let Some(mut image) = candidate else {
                return Ok(None);
            };
            let Some((powers, lead)) = image.first() else {
                return Ok(None);
            };
            // An unlucky point changes the leading monomial of its image.
            if leading.as_ref().is_some_and(|expected| expected != powers) {
                return Ok(None);
            }
            let Some(inverse) = field.inv(*lead) else {
                return Ok(None);
            };
            if leading.is_none() {
                leading = Some(powers.clone());
            }
            let scale = field.mul(field.evaluate(&gamma, point), inverse);
            for (_, value) in &mut image {
                *value = field.mul(*value, scale);
            }
            if shape.is_none() && variable > 1 {
                shape = Some(Shape::new(&image));
            }
            let changed = self.newton(&mut interpolant, &mut basis, &image, point)?;
            points.push(point);
            if !changed {
                break;
            }
        }
        let mut rows: Rows = interpolant
            .into_iter()
            .filter_map(|(prefix, mut dense)| {
                while dense.last() == Some(&Residue::ZERO) {
                    dense.pop();
                }
                (!dense.is_empty()).then_some((prefix, dense))
            })
            .collect();
        if self.primitive(&mut rows)?.is_none() {
            return Ok(None);
        }
        if content.len() > 1 {
            for (_, dense) in &mut rows {
                *dense = Dense::product(field, dense, &content);
            }
        }
        self.terms(&rows, variable).map(Some)
    }

    /// The GCD of two polynomials of `level` with the support of `shape`, up
    /// to a scalar, by sparse interpolation in positions `1..level`.
    pub fn image(
        &mut self,
        left: &[ModularTerm],
        right: &[ModularTerm],
        shape: &Shape,
        level: usize,
    ) -> Result<Option<Vec<ModularTerm>>, SimplifyError> {
        let field = self.field;
        let groups = &shape.groups;
        let (Some(count), Some(length)) = (shape.count, shape.length) else {
            return Ok(None);
        };
        let main = |powers: &Monomial| usize::try_from(powers.power(0)).unwrap_or(0);
        let lengths = [left, right].map(|terms| {
            terms
                .first()
                .map_or(0, |(powers, _)| main(powers).saturating_add(1))
        });
        'attempt: for _ in 0..ATTEMPTS {
            let point: Vec<_> = (1..level).map(|_| self.sampler.nonzero(field)).collect();
            let weight = |powers: &Monomial| {
                powers.as_slice().iter().skip(1).zip(&point).fold(
                    Residue::ONE,
                    |product, (&exponent, &base)| {
                        field.mul(product, field.pow(base, u64::from(exponent)))
                    },
                )
            };
            let nodes: Vec<Vec<Residue>> = groups
                .iter()
                .map(|(_, monomials)| monomials.iter().map(weight).collect())
                .collect();
            // Vandermonde systems need distinct nodes in each block.
            for block in &nodes {
                let mut sorted = block.clone();
                sorted.sort_unstable();
                if sorted.windows(2).any(|pair| pair.first() == pair.last()) {
                    continue 'attempt;
                }
            }
            // Each term's main exponent, weight, and value at `beta^l`.
            let mut sources = [left, right].map(|terms| {
                terms
                    .iter()
                    .map(|(powers, value)| {
                        let factor = weight(powers);
                        (main(powers), factor, field.mul(*value, factor))
                    })
                    .collect::<Vec<_>>()
            });
            let mut rows: Vec<Vec<Residue>> = Vec::with_capacity(count);
            for _ in 0..count {
                let mut univariate = lengths.map(|size| vec![Residue::ZERO; size]);
                for (dense, terms) in univariate.iter_mut().zip(&mut sources) {
                    self.work.spend(terms.len())?;
                    for (index, factor, value) in terms.iter_mut() {
                        if let Some(slot) = dense.get_mut(*index) {
                            *slot = field.add(*slot, *value);
                        }
                        *value = field.mul(*value, *factor);
                    }
                }
                let [first, second] = univariate;
                // A vanished leading coefficient lowers an input degree.
                if first.last() == Some(&Residue::ZERO) || second.last() == Some(&Residue::ZERO) {
                    continue 'attempt;
                }
                let common = Dense::gcd(field, first, second, self.work)?;
                if common.len() != length {
                    return Ok(None);
                }
                // Coefficients outside the support refute it.
                let mut expected = groups.iter().map(|(exponent, _)| *exponent).peekable();
                let mut row = Vec::with_capacity(groups.len());
                for (index, &value) in common.iter().enumerate().rev() {
                    let exponent = u32::try_from(index).ok();
                    if expected.next_if(|next| Some(*next) == exponent).is_some() {
                        row.push(value);
                    } else if value != Residue::ZERO {
                        return Ok(None);
                    }
                }
                rows.push(row);
            }
            return Ok(shape.solve(field, &nodes, &rows));
        }
        Ok(None)
    }

    /// Group terms of a level `variable + 1` polynomial into rows.
    fn rows(&mut self, terms: &[ModularTerm], variable: usize) -> Result<Rows, SimplifyError> {
        self.work.spend(terms.len())?;
        let mut rows: Rows = Vec::new();
        for (powers, value) in terms {
            let (degree, prefix) = powers.separate(variable);
            let index = usize::try_from(degree)
                .ok()
                .ok_or(SimplifyError::InvalidPolynomial)?;
            match rows.last_mut() {
                // The first term of a row has its highest degree.
                Some((last, dense)) if *last == prefix => {
                    if let Some(slot) = dense.get_mut(index) {
                        *slot = *value;
                    }
                }
                _ => {
                    let mut dense = vec![Residue::ZERO; index.saturating_add(1)];
                    if let Some(slot) = dense.last_mut() {
                        *slot = *value;
                    }
                    rows.push((prefix, dense));
                }
            }
        }
        Ok(rows)
    }

    /// Expand rows back into descending terms.
    fn terms(
        &self,
        rows: &[(Monomial, Vec<Residue>)],
        variable: usize,
    ) -> Result<Vec<ModularTerm>, SimplifyError> {
        let mut terms = Vec::new();
        for (prefix, dense) in rows {
            for (index, &value) in dense.iter().enumerate().rev() {
                if value == Residue::ZERO {
                    continue;
                }
                let mut powers = prefix.clone();
                let degree = u32::try_from(index)
                    .ok()
                    .ok_or(SimplifyError::InvalidPolynomial)?;
                powers.shift(variable, degree, self.work)?;
                terms.push((powers, value));
            }
        }
        Ok(terms)
    }

    /// Divide the rows by their monic GCD in the last position and return it,
    /// or `None` when a quotient is inexact.
    fn primitive(&mut self, rows: &mut Rows) -> Result<Option<Vec<Residue>>, SimplifyError> {
        let mut content = Vec::new();
        for (_, dense) in rows.iter() {
            content = Dense::gcd(self.field, content, dense.clone(), self.work)?;
            if content.len() <= 1 {
                return Ok(Some(content));
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

    /// The rows at `point` in the last position, without zero values.
    fn evaluate(&mut self, rows: &Rows, point: Residue) -> Result<Vec<ModularTerm>, SimplifyError> {
        self.work.spend(rows.len())?;
        let field = self.field;
        Ok(rows
            .iter()
            .filter_map(|(prefix, dense)| {
                let value = field.evaluate(dense, point);
                (value != Residue::ZERO).then(|| (prefix.clone(), value))
            })
            .collect())
    }

    /// A fresh point at which neither leading coefficient vanishes.
    fn point(&mut self, left: &[Residue], right: &[Residue], used: &[Residue]) -> Option<Residue> {
        let field = self.field;
        (0..ATTEMPTS)
            .map(|_| self.sampler.nonzero(field))
            .find(|&point| {
                !used.contains(&point)
                    && field.evaluate(left, point) != Residue::ZERO
                    && field.evaluate(right, point) != Residue::ZERO
            })
    }

    /// Add one image to the Newton form `h + c*q`, where `q` is the product of
    /// `(x - alpha)` over the earlier points, and report whether it changed.
    ///
    /// Rows and image terms both descend, so one merge visits every prefix
    /// of either; a prefix new to the interpolant starts from zero.
    fn newton(
        &mut self,
        interpolant: &mut Rows,
        basis: &mut Vec<Residue>,
        image: &[ModularTerm],
        point: Residue,
    ) -> Result<bool, SimplifyError> {
        let field = self.field;
        let scale = field
            .inv(field.evaluate(basis, point))
            .ok_or(SimplifyError::InvalidPolynomial)?;
        let previous = take(interpolant);
        interpolant.reserve(previous.len().max(image.len()));
        let mut rows = previous.into_iter().peekable();
        let mut values = image.iter().peekable();
        let mut changed = false;
        loop {
            let next = values.peek().copied();
            let (powers, mut dense) =
                match rows.next_if(|(row, _)| next.is_none_or(|(key, _)| row >= key)) {
                    Some(row) => row,
                    None => match next {
                        Some((key, _)) => (key.clone(), Vec::new()),
                        None => break,
                    },
                };
            self.work.spend(basis.len())?;
            let value = values
                .next_if(|(key, _)| *key == powers)
                .map_or(Residue::ZERO, |(_, value)| *value);
            let difference = field.sub(value, field.evaluate(&dense, point));
            if difference != Residue::ZERO {
                changed = true;
                if dense.len() < basis.len() {
                    dense.resize(basis.len(), Residue::ZERO);
                }
                field.mul_sub(&mut dense, basis, field.neg(field.mul(difference, scale)));
            }
            interpolant.push((powers, dense));
        }
        let mut next: Vec<_> = once(Residue::ZERO).chain(basis.iter().copied()).collect();
        field.mul_sub(&mut next, basis, point);
        *basis = next;
        Ok(changed)
    }
}
