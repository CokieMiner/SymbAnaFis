//! Sparse interpolation of GCD images on a known support.
//!
//! For a support grouped by the exponent of $x_{1}$, the monic univariate GCDs
//! at $\beta^{l}$, $l = 1, 2, \ldots$, give for each group the values
//! $y_{l} = s_{l} g_{l} = \sum_{i} c_{i} m_{i}^{l}$ with nodes $m_{i}$, the
//! monomials at $\beta$, and image scales $s_{l}$ (Zippel 1979). The coefficients solve a
//! transposed Vandermonde system (Zippel 1990). The scales follow from a
//! single leading monomial, or else from the linear recurrence that each group
//! satisfies with its node polynomial (de Kleine, Monagan, and Wittkopf 2005).

use core::iter::{once, successors};

use alloc::{vec, vec::Vec};

use ntt_anafis::{PrimeField, Residue};

use super::{ModularTerm, Monomial};

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
        // Each block needs one equation per unknown. Free scales add one
        // unknown per image beyond the first, and a block of `n` unknowns
        // yields `count - n` recurrence equations.
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

    /// The image with this support from the sampled rows: scales from the
    /// single leading monomial or from the block recurrences, then one
    /// transposed Vandermonde solve per block.
    pub fn solve(
        &self,
        field: PrimeField,
        nodes: &[Vec<Residue>],
        rows: &[Vec<Residue>],
    ) -> Option<Vec<ModularTerm>> {
        // A single leading monomial with coefficient one has the value
        // `m^l` at `beta^l`, which is the scale of the monic image.
        let scales: Vec<Residue> = if self.single {
            let &lead = nodes.first()?.first()?;
            successors(Some(lead), |&scale| Some(field.mul(scale, lead)))
                .take(rows.len())
                .collect()
        } else {
            Self::scales(field, rows, nodes)?
        };
        let mut terms = Vec::with_capacity(self.total);
        for (index, ((_, monomials), block)) in self.groups.iter().zip(nodes).enumerate() {
            let values = rows
                .iter()
                .zip(&scales)
                .take(block.len())
                .map(|(row, &scale)| row.get(index).map(|&value| field.mul(value, scale)))
                .collect::<Option<Vec<_>>>()?;
            let coefficients = Self::vandermonde(field, block, &values)?;
            for (powers, coefficient) in monomials.iter().zip(coefficients) {
                if coefficient != Residue::ZERO {
                    terms.push((powers.clone(), coefficient));
                }
            }
        }
        Some(terms)
    }

    /// The image scales `s_l`, with `s_1 = 1`, for which every block
    /// `y_l = s_l * row_l` satisfies the recurrence of its node polynomial
    /// `P(z) = prod (z - m_i)`: `sum_q P_q y_(l+q) = 0`.
    fn scales(
        field: PrimeField,
        rows: &[Vec<Residue>],
        nodes: &[Vec<Residue>],
    ) -> Option<Vec<Residue>> {
        let count = rows.len();
        // Each equation lists the factors of `s_1, ..., s_T`.
        let mut system: Vec<Vec<Residue>> = Vec::new();
        for (index, block) in nodes.iter().enumerate() {
            let master = Self::master(field, block);
            for shift in 0..count.saturating_sub(block.len()) {
                let mut equation = vec![Residue::ZERO; count];
                for ((slot, row), &coefficient) in equation
                    .iter_mut()
                    .skip(shift)
                    .zip(rows.iter().skip(shift))
                    .zip(&master)
                {
                    *slot = field.mul(coefficient, *row.get(index)?);
                }
                system.push(equation);
            }
        }
        // Gauss-Jordan elimination of `s_2, ..., s_T`; `s_1 = 1` stays in
        // column zero as the constant.
        for column in 1..count {
            let pivot = column.checked_sub(1)?;
            let found = system
                .iter()
                .enumerate()
                .skip(pivot)
                .find(|(_, equation)| {
                    equation
                        .get(column)
                        .is_some_and(|value| *value != Residue::ZERO)
                })
                .map(|(index, _)| index)?;
            system.swap(pivot, found);
            let mut reference = system.get(pivot)?.clone();
            let inverse = field.inv(*reference.get(column)?)?;
            field.scale(&mut reference, inverse);
            for (index, equation) in system.iter_mut().enumerate() {
                if index == pivot {
                    equation.clone_from(&reference);
                    continue;
                }
                let factor = *equation.get(column)?;
                if factor != Residue::ZERO {
                    field.mul_sub(equation, &reference, factor);
                }
            }
        }
        // Equations beyond the pivots must hold for the solved scales.
        let solved = count.saturating_sub(1);
        if system.iter().skip(solved).any(|equation| {
            equation
                .first()
                .is_some_and(|value| *value != Residue::ZERO)
        }) {
            return None;
        }
        Some(
            once(Residue::ONE)
                .chain(
                    system.iter().take(solved).map(|equation| {
                        field.neg(equation.first().copied().unwrap_or(Residue::ZERO))
                    }),
                )
                .collect(),
        )
    }

    /// The coefficients `c_i` of `y_l = sum_i c_i m_i^l`, `l = 1..=n`, for
    /// distinct nonzero nodes `m_i`. With `P(z) = prod (z - m_j)` and
    /// `Q_i = P/(z - m_i)`, `sum_q Q_iq y_(q+1) = c_i m_i Q_i(m_i)` (Zippel 1990).
    fn vandermonde(
        field: PrimeField,
        nodes: &[Residue],
        values: &[Residue],
    ) -> Option<Vec<Residue>> {
        let master = Self::master(field, nodes);
        nodes
            .iter()
            .map(|&node| {
                // Synthetic division from the top gives `Q_i` coefficient by
                // coefficient, together with `Q_i(m_i)` by Horner's rule.
                let mut carry = Residue::ONE;
                let mut numerator = Residue::ZERO;
                let mut derivative = Residue::ZERO;
                for (&coefficient, &value) in master.iter().rev().skip(1).zip(values.iter().rev()) {
                    numerator = field.add(numerator, field.mul(carry, value));
                    derivative = field.add(field.mul(derivative, node), carry);
                    carry = field.add(coefficient, field.mul(node, carry));
                }
                let inverse = field.inv(field.mul(node, derivative))?;
                Some(field.mul(numerator, inverse))
            })
            .collect()
    }

    /// The monic node polynomial `prod (z - m_i)` by ascending degree.
    fn master(field: PrimeField, nodes: &[Residue]) -> Vec<Residue> {
        let mut master = vec![Residue::ONE];
        for &node in nodes {
            let mut next: Vec<_> = once(Residue::ZERO).chain(master.iter().copied()).collect();
            field.mul_sub(&mut next, &master, node);
            master = next;
        }
        master
    }
}
