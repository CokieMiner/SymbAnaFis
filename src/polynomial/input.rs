//! Expression-to-polynomial lowering, canonical expression reconstruction, and
//! simplification-budget error conversion.

use core::{num::NonZeroU32, slice::from_ref};

use alloc::{vec, vec::Vec};

use num_anafis::{Number, RationalType, Real};

use crate::{
    collections::FixedMap,
    core::{CompactStorage, ExactIdentity, Expr, ExprView, StoredForm},
    errors::SimplifyError,
};

use super::{Monomial, PolynomialLimitExceeded, PolynomialWork, SparsePolynomial};

/// Namespace for lowering symbolic expressions to sparse polynomials.
pub struct PolynomialInput;

struct Builder<'input> {
    // Each expression is classified once. Every operand of a non-atomic form
    // precedes it; both roots are included. `lower` reads completed values by
    // slot rather than by expression.
    order: Vec<(usize, Form)>,
    bases: Vec<Expr>,
    base_indices: FixedMap<&'input Expr, usize>,
}

enum Form {
    Scalar(RationalType),
    Variable(usize),
    /// A compact `Node::Poly` lowered straight from its stored monomials
    /// without projecting intermediate `Product` or `Pow` expressions.
    Compact {
        terms: Vec<(usize, RationalType)>,
        powers: Vec<(usize, u32)>,
    },
    Sum(Vec<usize>),
    Product(Vec<usize>),
    Power {
        base: usize,
        count: NonZeroU32,
    },
    Scale {
        numerator: usize,
        divisor: RationalType,
    },
}

/// One slot per distinct expression, assigned at first visit, with use counts
/// so `lower` moves single-use operands instead of cloning them.
#[derive(Default)]
struct Slots<'input> {
    positions: FixedMap<&'input Expr, usize>,
    visited: Vec<bool>,
    uses: Vec<usize>,
}

enum Frame<'input> {
    Visit(usize, &'input Expr),
    Ready(usize, Form),
}

impl From<PolynomialLimitExceeded> for SimplifyError {
    fn from(error: PolynomialLimitExceeded) -> Self {
        Self::PolynomialLimitExceeded { limit: error.limit }
    }
}

impl PolynomialInput {
    /// Lower `left` and `right` over one shared base table, returning `None`
    /// when a coefficient or exponent leaves native rational capacity.
    pub fn pair(
        left: &Expr,
        right: &Expr,
        work: &mut PolynomialWork,
    ) -> Result<Option<(SparsePolynomial, SparsePolynomial, Vec<Expr>)>, SimplifyError> {
        let mut builder = Builder {
            order: Vec::new(),
            bases: Vec::new(),
            base_indices: FixedMap::default(),
        };
        let mut slots = Slots::default();
        let (left_slot, right_slot) = (slots.slot(left), slots.slot(right));
        let mut pending = vec![
            Frame::Visit(left_slot, left),
            Frame::Visit(right_slot, right),
        ];
        let mut children = Vec::new();
        while let Some(frame) = pending.pop() {
            let (slot, node) = match frame {
                Frame::Ready(ready_slot, form) => {
                    builder.order.push((ready_slot, form));
                    continue;
                }
                Frame::Visit(visit_slot, node) => (visit_slot, node),
            };
            let Some(seen) = slots.visited.get_mut(slot) else {
                continue;
            };
            if *seen {
                continue;
            }
            *seen = true;
            work.spend(1)?;
            children.clear();
            let Some(form) = builder.classify(node, &mut slots, &mut children, work)? else {
                return Ok(None);
            };
            if children.is_empty() {
                builder.order.push((slot, form));
            } else {
                pending.push(Frame::Ready(slot, form));
                pending.extend(
                    children
                        .iter()
                        .copied()
                        .map(|(child_slot, child)| Frame::Visit(child_slot, child)),
                );
            }
        }
        builder.lower(left_slot, right_slot, slots.uses, work)
    }

    /// Lower `factors` over one shared base table when every term is an exact
    /// rational multiple of a product of positive powers of symbols and $\pi$.
    /// Powers of $e$ become `exp` calls, which products do not collect, so $e$
    /// is not a base here. `None` for any other term or overflow beyond native
    /// rational capacity.
    pub fn scalar_leaves(factors: &[Expr]) -> Option<(Vec<SparsePolynomial>, Vec<Expr>)> {
        let mut bases = Vec::new();
        let mut indices: FixedMap<&Expr, usize> = FixedMap::default();
        let mut all_powers: Vec<(usize, u32)> = Vec::new();
        let mut term_meta: Vec<(usize, RationalType)> = Vec::new();
        let mut factor_counts: Vec<usize> = Vec::with_capacity(factors.len());
        for factor in factors {
            let operands = factor.stored_terms().unwrap_or_else(|| from_ref(factor));
            let term_start = term_meta.len();
            for operand in operands {
                if let Some(monomials) = operand.monomials() {
                    term_meta.reserve(monomials.len());
                    for (coefficient, powers) in monomials {
                        let start = all_powers.len();
                        for (base, exponent) in powers {
                            if !matches!(base.view(), ExprView::Symbol(_) | ExprView::Pi) {
                                return None;
                            }
                            all_powers.push((
                                Self::register_base(base, &mut indices, &mut bases),
                                exponent,
                            ));
                        }
                        term_meta.push((
                            all_powers.len().saturating_sub(start),
                            Self::rational(coefficient)?,
                        ));
                    }
                    continue;
                }
                let parts = if let ExprView::Product(parts) = operand.view() {
                    parts
                } else {
                    from_ref(operand)
                };
                let mut coefficient = RationalType::from_integer(1);
                let start = all_powers.len();
                for part in parts {
                    if let Some(value) = Self::rational(part) {
                        coefficient = coefficient.checked_mul(&value)?;
                        continue;
                    }
                    let (base, count) = part.positive_factor().unwrap_or((part, 1));
                    if !matches!(base.view(), ExprView::Symbol(_) | ExprView::Pi) {
                        return None;
                    }
                    all_powers.push((
                        Self::register_base(base, &mut indices, &mut bases),
                        u32::try_from(count).ok()?,
                    ));
                }
                term_meta.push((all_powers.len().saturating_sub(start), coefficient));
            }
            factor_counts.push(term_meta.len().saturating_sub(term_start));
        }
        let dimensions = bases.len();
        let mut work = PolynomialWork::new(usize::MAX);
        let mut polynomials = Vec::with_capacity(factor_counts.len());
        let mut meta = term_meta.into_iter();
        let mut rest = all_powers.as_slice();
        for count in factor_counts {
            let poly =
                Self::from_flat_terms(dimensions, meta.by_ref().take(count), &mut rest, &mut work)
                    .ok()
                    .flatten()?;
            polynomials.push(poly);
        }
        Some((polynomials, bases))
    }
}

impl SparsePolynomial {
    /// The canonical sum of the terms over `bases`, or `None` when a reduced
    /// coefficient leaves native rational capacity.
    pub fn to_expr(&self, bases: &[Expr]) -> Option<Expr> {
        let terms = self.terms();
        if terms.is_empty() {
            return Some(Expr::number(0));
        }
        let content = self.content();
        let integer_content = content.is_int().then_some(*content.numer());
        let scalar = |value: i64| -> Option<Expr> {
            if let Some(unit) = integer_content {
                return Some(Expr::number(unit.checked_mul(value)?));
            }
            let ratio = content.checked_mul(&RationalType::from_integer(value))?;
            if ratio.is_int() {
                Some(Expr::number(*ratio.numer()))
            } else {
                Some(Expr::number(Number::from_rational(ratio)))
            }
        };
        // In strictly descending lexicographic order, a constant term can only
        // occupy the final slot; separating it keeps `sum_of_indexed_monomials`
        // on the direct compact-storage path.
        let (constant, monomial_terms) = match terms.split_last() {
            Some(((powers, value), rest)) if powers.is_zero() => (Some(scalar(*value)?), rest),
            _ => (None, terms),
        };
        if monomial_terms.is_empty() {
            return constant;
        }
        let mut coefficients = Vec::with_capacity(monomial_terms.len());
        let mut lengths = Vec::with_capacity(monomial_terms.len());
        let mut powers = Vec::with_capacity(
            monomial_terms
                .len()
                .saturating_mul(self.dimensions().min(4)),
        );
        // The canonical construction sorts ascending input fastest.
        for (monomial, value) in monomial_terms.iter().rev() {
            coefficients.push(scalar(*value)?);
            let start = powers.len();
            for (position, &count) in monomial.as_slice().iter().enumerate() {
                if count != 0 {
                    powers.push((u32::try_from(position).ok()?, count));
                }
            }
            lengths.push(powers.len().saturating_sub(start));
        }
        let body = Expr::sum_of_indexed_monomials(bases, coefficients, lengths, powers);
        Some(match constant {
            Some(term) => Expr::sum(vec![term, body]),
            None => body,
        })
    }
}

impl<'input> Builder<'input> {
    /// Classify `node`, appending operand `(slot, expr)` pairs to `children`.
    ///
    /// `monomials()` and `stored_terms()` precede `view()` so compact groups
    /// inside sums lower straight from storage without projecting intermediate
    /// `Product` or `Pow` nodes.
    fn classify(
        &mut self,
        node: &'input Expr,
        slots: &mut Slots<'input>,
        children: &mut Vec<(usize, &'input Expr)>,
        work: &mut PolynomialWork,
    ) -> Result<Option<Form>, SimplifyError> {
        if let Some(monomials) = node.monomials() {
            work.spend(monomials.len())?;
            'compact: {
                let mut terms = Vec::with_capacity(monomials.len());
                let mut raw_powers: Vec<(&'input Expr, u32)> = Vec::with_capacity(monomials.len());
                for (coefficient, powers) in monomials {
                    let Some(ratio) = PolynomialInput::rational(coefficient) else {
                        return Ok(None);
                    };
                    let start = raw_powers.len();
                    for (base, exponent) in powers {
                        let atomic = match base.view() {
                            ExprView::Number(_) | ExprView::Sum(_) | ExprView::Product(_) => false,
                            ExprView::Pow(_, power) => !matches!(
                                power.exact_ratio(),
                                Some((count, 1)) if count > 0 && u32::try_from(count).is_ok()
                            ),
                            ExprView::Div(_, denominator) => denominator
                                .exact_ratio()
                                .is_none_or(|(numerator, _)| numerator == 0),
                            ExprView::ImaginaryUnit
                            | ExprView::Pi
                            | ExprView::Euler
                            | ExprView::Symbol(_)
                            | ExprView::Function { .. }
                            | ExprView::Derivative { .. } => true,
                        };
                        if !atomic {
                            break 'compact;
                        }
                        raw_powers.push((base, exponent));
                    }
                    terms.push((raw_powers.len().saturating_sub(start), ratio));
                }
                let powers = raw_powers
                    .into_iter()
                    .map(|(base, exponent)| {
                        (
                            PolynomialInput::register_base(
                                base,
                                &mut self.base_indices,
                                &mut self.bases,
                            ),
                            exponent,
                        )
                    })
                    .collect();
                return Ok(Some(Form::Compact { terms, powers }));
            }
        }
        let mut push = |child: &'input Expr| {
            let child_slot = slots.slot(child);
            children.push((child_slot, child));
            child_slot
        };
        if let Some(operands) = node.stored_terms() {
            return Ok(Some(Form::Sum(operands.iter().map(push).collect())));
        }
        let form = match node.view() {
            ExprView::Number(_) => match PolynomialInput::rational(node) {
                Some(value) => Some(Form::Scalar(value)),
                None => return Ok(None),
            },
            ExprView::Sum(terms) => Some(Form::Sum(terms.map(push).collect())),
            ExprView::Product(factors) => Some(Form::Product(factors.iter().map(push).collect())),
            ExprView::Pow(base, exponent) => exponent
                .exact_ratio()
                .filter(|(_, denominator)| *denominator == 1)
                .and_then(|(count, _)| u32::try_from(count).ok())
                .and_then(NonZeroU32::new)
                .map(|count| Form::Power {
                    base: push(base),
                    count,
                }),
            ExprView::Div(numerator, denominator) => PolynomialInput::rational(denominator)
                .filter(|value| *value.numer() != 0)
                .map(|divisor| Form::Scale {
                    numerator: push(numerator),
                    divisor,
                }),
            ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Function { .. }
            | ExprView::Derivative { .. } => None,
        };
        Ok(Some(form.unwrap_or_else(|| {
            Form::Variable(PolynomialInput::register_base(
                node,
                &mut self.base_indices,
                &mut self.bases,
            ))
        })))
    }

    /// Lower the classified DAG, folding scalar product factors upfront and
    /// multiplying non-constant factors in ascending term-count order.
    fn lower(
        self,
        left: usize,
        right: usize,
        uses: Vec<usize>,
        work: &mut PolynomialWork,
    ) -> Result<Option<(SparsePolynomial, SparsePolynomial, Vec<Expr>)>, SimplifyError> {
        let dimensions = self.bases.len();
        let mut entries: Vec<(Option<SparsePolynomial>, usize)> =
            uses.into_iter().map(|count| (None, count)).collect();
        for (slot, form) in self.order {
            let value = match form {
                Form::Scalar(scalar) => Some(SparsePolynomial::constant(scalar, dimensions, work)?),
                Form::Variable(index) => Some(SparsePolynomial::variable(dimensions, index, work)?),
                Form::Compact { terms, powers } => {
                    let mut slice = powers.as_slice();
                    PolynomialInput::from_flat_terms(
                        dimensions,
                        terms.into_iter(),
                        &mut slice,
                        work,
                    )?
                }
                Form::Sum(children) => {
                    let Some(addends) = children
                        .iter()
                        .map(|&child| entries.get(child).and_then(|(poly, _)| poly.as_ref()))
                        .collect::<Option<Vec<_>>>()
                    else {
                        return Ok(None);
                    };
                    let sum = SparsePolynomial::sum(dimensions, &addends, work)?;
                    for child in children {
                        Self::release(&mut entries, child);
                    }
                    sum
                }
                Form::Product(children) => {
                    Self::lower_product(dimensions, children, &mut entries, work)?
                }
                Form::Power { base, count } => {
                    let Some(lowered) = entries.get(base).and_then(|(poly, _)| poly.as_ref())
                    else {
                        return Ok(None);
                    };
                    let powered = lowered.checked_pow(count.get(), work)?;
                    Self::release(&mut entries, base);
                    powered
                }
                Form::Scale { numerator, divisor } => {
                    Self::take(&mut entries, numerator).and_then(|p| p.checked_div_scalar(&divisor))
                }
            };
            let (Some(poly), Some((target, _))) = (value, entries.get_mut(slot)) else {
                return Ok(None);
            };
            *target = Some(poly);
        }
        let (Some(second), Some(first)) = (
            Self::take(&mut entries, right),
            Self::take(&mut entries, left),
        ) else {
            return Ok(None);
        };
        Ok(Some((first, second, self.bases)))
    }

    /// Fold scalar product factors upfront and multiply non-constant factors
    /// in ascending term-count order.
    fn lower_product(
        dimensions: usize,
        children: Vec<usize>,
        entries: &mut [(Option<SparsePolynomial>, usize)],
        work: &mut PolynomialWork,
    ) -> Result<Option<SparsePolynomial>, PolynomialLimitExceeded> {
        let mut scale = RationalType::from_integer(1);
        let mut non_constants = Vec::with_capacity(children.len());
        let mut zeroed = false;
        for child in children {
            let Some(factor) = entries.get(child).and_then(|(poly, _)| poly.as_ref()) else {
                return Ok(None);
            };
            if factor.is_zero() {
                zeroed = true;
                Self::release(entries, child);
            } else if factor.is_constant() {
                let Some(next_scale) = scale.checked_mul(factor.content()) else {
                    return Ok(None);
                };
                scale = next_scale;
                Self::release(entries, child);
            } else {
                non_constants.push((factor.terms().len(), child));
            }
        }
        if zeroed {
            for (_, child) in non_constants {
                Self::release(entries, child);
            }
            return Ok(Some(SparsePolynomial::zero(dimensions)));
        }
        if non_constants.is_empty() {
            return Ok(Some(SparsePolynomial::constant(scale, dimensions, work)?));
        }
        non_constants.sort_unstable_by_key(|&(len, _)| len);
        let mut factors = non_constants.into_iter().map(|(_, child)| child);
        let Some(first_slot) = factors.next() else {
            return Ok(None);
        };
        let Some(mut product) = Self::take(entries, first_slot) else {
            return Ok(None);
        };
        for child in factors {
            let Some(factor) = entries.get(child).and_then(|(poly, _)| poly.as_ref()) else {
                return Ok(None);
            };
            let Some(next) = product.checked_mul(factor, work)? else {
                return Ok(None);
            };
            Self::release(entries, child);
            product = next;
        }
        Ok(if scale == RationalType::from_integer(1) {
            Some(product)
        } else {
            product.checked_scale(&scale)
        })
    }

    fn release(table: &mut [(Option<SparsePolynomial>, usize)], slot: usize) {
        if let Some((entry, remaining)) = table.get_mut(slot) {
            *remaining = remaining.saturating_sub(1);
            if *remaining == 0 {
                *entry = None;
            }
        }
    }

    fn take(
        table: &mut [(Option<SparsePolynomial>, usize)],
        slot: usize,
    ) -> Option<SparsePolynomial> {
        let (entry, remaining) = table.get_mut(slot)?;
        *remaining = remaining.saturating_sub(1);
        if *remaining == 0 {
            entry.take()
        } else {
            entry.clone()
        }
    }
}

impl<'input> Slots<'input> {
    fn slot(&mut self, node: &'input Expr) -> usize {
        if let Some(&index) = self.positions.get(node) {
            if let Some(count) = self.uses.get_mut(index) {
                *count = count.saturating_add(1);
            }
            return index;
        }
        let index = self.visited.len();
        self.positions.insert(node, index);
        self.visited.push(false);
        self.uses.push(1);
        index
    }
}

impl PolynomialInput {
    /// Build one `SparsePolynomial` from `(power_len, coeff)` descriptors,
    /// advancing `powers` past the consumed entries.
    fn from_flat_terms(
        dimensions: usize,
        descriptors: impl ExactSizeIterator<Item = (usize, RationalType)>,
        powers: &mut &[(usize, u32)],
        work: &mut PolynomialWork,
    ) -> Result<Option<SparsePolynomial>, PolynomialLimitExceeded> {
        let mut monomials = Vec::with_capacity(descriptors.len());
        for (len, coefficient) in descriptors {
            let Some((slice, tail)) = powers.split_at_checked(len) else {
                return Ok(None);
            };
            *powers = tail;
            let Some(monomial) = Monomial::from_indexed(dimensions, slice.iter().copied()) else {
                return Ok(None);
            };
            monomials.push((monomial, coefficient));
        }
        SparsePolynomial::from_terms(dimensions, monomials, work)
    }

    fn register_base<'expr>(
        base: &'expr Expr,
        indices: &mut FixedMap<&'expr Expr, usize>,
        bases: &mut Vec<Expr>,
    ) -> usize {
        if let Some(&index) = indices.get(base) {
            return index;
        }
        let index = bases.len();
        indices.insert(base, index);
        bases.push(base.clone());
        index
    }

    /// An exact real literal as a backend rational.
    fn rational(expression: &Expr) -> Option<RationalType> {
        match expression.as_number()?.components() {
            (Real::Int(numerator), Real::Int(0)) => Some(RationalType::from_integer(*numerator)),
            (Real::Rational(ratio), Real::Int(0)) => Some(ratio.clone()),
            _ => None,
        }
    }
}
