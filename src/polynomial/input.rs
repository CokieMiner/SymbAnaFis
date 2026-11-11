//! Validate exact polynomial inputs once and lower a memoized logical DAG.

use core::{num::NonZeroU32, slice::from_ref};

use alloc::{vec, vec::Vec};

use num_anafis::{RationalType, Real};

use crate::{
    collections::{FixedMap, FixedSet},
    core::{ExactIdentity, Expr, ExprView, StoredForm},
    errors::SimplifyError,
};

use super::{Monomial, PolynomialWork, SparsePolynomial};

pub struct PolynomialInput {
    // Each expression is classified once. Every operand of a non-atomic form
    // precedes it; both roots are included. `lower` only reads completed values.
    order: Vec<(Expr, Form)>,
    bases: Vec<Expr>,
}

enum Form {
    Scalar(RationalType),
    Variable(usize),
    Sum,
    Product,
    Power {
        base: Expr,
        count: NonZeroU32,
    },
    Scale {
        numerator: Expr,
        // `classify` establishes exact representation and a nonzero numerator.
        // The backend guarantees a positive, reduced rational denominator.
        divisor: RationalType,
    },
}

impl PolynomialInput {
    pub fn pair(
        left: &Expr,
        right: &Expr,
        work: &mut PolynomialWork,
    ) -> Result<Option<(SparsePolynomial, SparsePolynomial, Vec<Expr>)>, SimplifyError> {
        if let Some(result) = Self::compact_pair(left, right, work)? {
            return Ok(Some(result));
        }
        let mut input = Self {
            order: Vec::new(),
            bases: Vec::new(),
        };
        let mut pending = vec![(left.clone(), None), (right.clone(), None)];
        let mut visited = FixedSet::default();
        while let Some((node, ready)) = pending.pop() {
            if let Some(form) = ready {
                input.order.push((node, form));
                continue;
            }
            if !visited.insert(node.clone()) {
                continue;
            }
            work.spend(1)?;
            let Some(form) = input.classify(&node) else {
                return Ok(None);
            };
            match &form {
                Form::Sum | Form::Product => {
                    pending.push((node.clone(), Some(form)));
                    pending.extend(node.children().map(|child| (child.clone(), None)));
                }
                Form::Power { base, .. } => {
                    let child = base.clone();
                    pending.push((node, Some(form)));
                    pending.push((child, None));
                }
                Form::Scale { numerator, .. } => {
                    let child = numerator.clone();
                    pending.push((node, Some(form)));
                    pending.push((child, None));
                }
                Form::Scalar(_) | Form::Variable(_) => input.order.push((node, form)),
            }
        }
        input.lower(left, right, work)
    }

    /// Lower `factors` over one shared base table when every term is an exact
    /// rational multiple of a product of positive powers of symbols and $\pi$.
    /// Construction collects products of these bases only by adding
    /// exponents, so polynomial arithmetic on them matches canonical
    /// expression arithmetic. Powers of $e$ become `exp` calls, which products
    /// do not collect, so $e$ is not a base here. `None` for any other term or
    /// an exact value beyond native rational capacity.
    pub fn scalar_leaves(factors: &[Expr]) -> Option<(Vec<SparsePolynomial>, Vec<Expr>)> {
        let mut bases = Vec::new();
        let mut indices = FixedMap::default();
        let mut lowered = Vec::with_capacity(factors.len());
        for factor in factors {
            let operands = factor.stored_terms().unwrap_or_else(|| from_ref(factor));
            let mut terms = Vec::with_capacity(operands.len());
            for operand in operands {
                if let Some(monomials) = operand.monomials() {
                    for (coefficient, powers) in monomials {
                        let mut indexed = Vec::with_capacity(powers.len());
                        for (base, exponent) in powers {
                            if !matches!(base.view(), ExprView::Symbol(_) | ExprView::Pi) {
                                return None;
                            }
                            let index = Self::register_base(base, &mut indices, &mut bases);
                            indexed.push((index, exponent));
                        }
                        terms.push((indexed, Self::rational(coefficient)?));
                    }
                    continue;
                }
                let parts = if let ExprView::Product(parts) = operand.view() {
                    parts
                } else {
                    from_ref(operand)
                };
                let mut coefficient = RationalType::from_integer(1);
                let mut indexed = Vec::with_capacity(parts.len());
                for part in parts {
                    if let Some(value) = Self::rational(part) {
                        coefficient = coefficient.checked_mul(&value)?;
                        continue;
                    }
                    let (base, count) = part.positive_factor().unwrap_or((part, 1));
                    if !matches!(base.view(), ExprView::Symbol(_) | ExprView::Pi) {
                        return None;
                    }
                    let index = Self::register_base(base, &mut indices, &mut bases);
                    indexed.push((index, u32::try_from(count).ok()?));
                }
                terms.push((indexed, coefficient));
            }
            lowered.push(terms);
        }
        let dimensions = bases.len();
        // Construction bounds the inputs; the caller charges the products.
        let mut work = PolynomialWork::new(usize::MAX);
        let mut polynomials = Vec::with_capacity(lowered.len());
        for terms in lowered {
            let mut monomials = Vec::with_capacity(terms.len());
            for (indexed, coefficient) in terms {
                monomials.push((Monomial::from_indexed(dimensions, &indexed)?, coefficient));
            }
            polynomials.push(
                SparsePolynomial::from_terms(dimensions, monomials, &mut work)
                    .ok()
                    .flatten()?,
            );
        }
        Some((polynomials, bases))
    }

    fn compact_pair(
        left: &Expr,
        right: &Expr,
        work: &mut PolynomialWork,
    ) -> Result<Option<(SparsePolynomial, SparsePolynomial, Vec<Expr>)>, SimplifyError> {
        if left.monomials().is_none() || right.monomials().is_none() {
            return Ok(None);
        }
        let mut bases = Vec::new();
        let mut indices = FixedMap::default();
        for expression in [left, right] {
            let Some(monomials) = expression.monomials() else {
                return Ok(None);
            };
            for (coefficient, powers) in monomials {
                if Self::rational(coefficient).is_none() {
                    return Ok(None);
                }
                for (base, _) in powers {
                    if !Self::compact_base(base) {
                        return Ok(None);
                    }
                    Self::register_base(base, &mut indices, &mut bases);
                }
            }
        }
        let dimensions = bases.len();
        let Some(left_terms) = Self::compact_terms(left, &indices, dimensions) else {
            return Ok(None);
        };
        let Some(right_terms) = Self::compact_terms(right, &indices, dimensions) else {
            return Ok(None);
        };
        let Some(left_poly) = SparsePolynomial::from_terms(dimensions, left_terms, work)? else {
            return Ok(None);
        };
        let Some(right_poly) = SparsePolynomial::from_terms(dimensions, right_terms, work)? else {
            return Ok(None);
        };
        Ok(Some((left_poly, right_poly, bases)))
    }

    fn compact_terms(
        expression: &Expr,
        indices: &FixedMap<Expr, usize>,
        dimensions: usize,
    ) -> Option<Vec<(Monomial, RationalType)>> {
        expression
            .monomials()?
            .map(|(coefficient, powers)| {
                let indexed = powers
                    .map(|(base, exponent)| Some((indices.get(base).copied()?, exponent)))
                    .collect::<Option<Vec<_>>>()?;
                Some((
                    Monomial::from_indexed(dimensions, &indexed)?,
                    Self::rational(coefficient)?,
                ))
            })
            .collect()
    }

    /// The index of `base` in the shared table, registering it when new.
    fn register_base(
        base: &Expr,
        indices: &mut FixedMap<Expr, usize>,
        bases: &mut Vec<Expr>,
    ) -> usize {
        if let Some(&index) = indices.get(base) {
            return index;
        }
        let index = bases.len();
        indices.insert(base.clone(), index);
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

    fn compact_base(base: &Expr) -> bool {
        match base.view() {
            ExprView::Number(_) | ExprView::Sum(_) | ExprView::Product(_) => false,
            ExprView::Pow(_, exponent) => !matches!(
                exponent.exact_ratio(),
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
        }
    }

    fn classify(&mut self, node: &Expr) -> Option<Form> {
        match node.view() {
            ExprView::Number(_) => Self::rational(node).map(Form::Scalar),
            ExprView::Sum(_) => Some(Form::Sum),
            ExprView::Product(_) => Some(Form::Product),
            ExprView::Pow(base, exponent) => Some(
                exponent
                    .exact_ratio()
                    .filter(|(_, denominator)| *denominator == 1)
                    .and_then(|(count, _)| u32::try_from(count).ok())
                    .and_then(NonZeroU32::new)
                    .map_or_else(
                        || self.variable(node),
                        |count| Form::Power {
                            base: base.clone(),
                            count,
                        },
                    ),
            ),
            ExprView::Div(numerator, denominator) => Some(
                Self::rational(denominator)
                    .filter(|value| *value.numer() != 0)
                    .map_or_else(
                        || self.variable(node),
                        |divisor| Form::Scale {
                            numerator: numerator.clone(),
                            divisor,
                        },
                    ),
            ),
            ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Function { .. }
            | ExprView::Derivative { .. } => Some(self.variable(node)),
        }
    }

    fn variable(&mut self, node: &Expr) -> Form {
        let index = self.bases.len();
        self.bases.push(node.clone());
        Form::Variable(index)
    }

    /// Lower the classified DAG, or return `None` when a coefficient leaves
    /// native rational capacity so the caller keeps the original expressions.
    fn lower(
        self,
        left: &Expr,
        right: &Expr,
        work: &mut PolynomialWork,
    ) -> Result<Option<(SparsePolynomial, SparsePolynomial, Vec<Expr>)>, SimplifyError> {
        let dimensions = self.bases.len();
        let mut values = FixedMap::<Expr, SparsePolynomial>::default();
        for (node, form) in self.order {
            let value = match form {
                Form::Scalar(value) => SparsePolynomial::constant(value, dimensions, work)?,
                Form::Variable(index) => SparsePolynomial::variable(dimensions, index, work)?,
                Form::Sum => {
                    let mut addends = Vec::new();
                    for term in node.children() {
                        addends.push(values.get(term).ok_or(SimplifyError::InvalidPolynomial)?);
                    }
                    let Some(sum) = SparsePolynomial::sum(dimensions, &addends, work)? else {
                        return Ok(None);
                    };
                    sum
                }
                Form::Product => {
                    let mut product = SparsePolynomial::one(dimensions);
                    for factor in node.children() {
                        let multiplier =
                            values.get(factor).ok_or(SimplifyError::InvalidPolynomial)?;
                        let Some(next) = product.checked_mul(multiplier, work)? else {
                            return Ok(None);
                        };
                        product = next;
                    }
                    product
                }
                Form::Power { base, count } => {
                    let lowered = values.get(&base).ok_or(SimplifyError::InvalidPolynomial)?;
                    let Some(power) = lowered.checked_pow(count.get(), work)? else {
                        return Ok(None);
                    };
                    power
                }
                Form::Scale { numerator, divisor } => {
                    let lowered = values
                        .get(&numerator)
                        .ok_or(SimplifyError::InvalidPolynomial)?
                        .clone();
                    let Some(quotient) = RationalType::from_integer(1)
                        .checked_div(&divisor)
                        .and_then(|factor| lowered.checked_scale(&factor))
                    else {
                        return Ok(None);
                    };
                    quotient
                }
            };
            values.insert(node, value);
        }
        let second = values
            .remove(right)
            .ok_or(SimplifyError::InvalidPolynomial)?;
        // Distinct roots transfer their polynomial storage without cloning.
        // Identical roots share one memo entry and require one owning copy.
        let first = if left == right {
            second.clone()
        } else {
            values
                .remove(left)
                .ok_or(SimplifyError::InvalidPolynomial)?
        };
        Ok(Some((first, second, self.bases)))
    }
}
