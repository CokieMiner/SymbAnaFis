//! Validate exact polynomial inputs once and lower a memoized logical DAG.

use core::num::NonZeroU32;

use alloc::{vec, vec::Vec};

use num_anafis::RationalType;

use crate::{
    collections::{HashMap, HashSet},
    core::{Expr, ExprView},
    errors::SimplifyError,
};

use super::{PolynomialWork, PreparedStep, SparsePolynomial};

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
        let mut input = Self {
            order: Vec::new(),
            bases: Vec::new(),
        };
        let mut pending = vec![(left.clone(), None), (right.clone(), None)];
        let mut visited = HashSet::new();
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
                    pending.extend(node.children().map(|child| (child, None)));
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
        input.lower(left, right, work).map(Some)
    }

    fn classify(&mut self, node: &Expr) -> Option<Form> {
        match node.view() {
            ExprView::Number(_) => PreparedStep::exact_ratio(node)
                .map(|(top, bottom)| Form::Scalar(RationalType::from_parts(top, bottom))),
            ExprView::Sum(_) => Some(Form::Sum),
            ExprView::Product(_) => Some(Form::Product),
            ExprView::Pow(base, exponent) => Some(
                PreparedStep::exact_integer(exponent)
                    .and_then(|count| u32::try_from(count).ok())
                    .and_then(NonZeroU32::new)
                    .map_or_else(
                        || self.variable(node),
                        |count| Form::Power {
                            base: base.clone(),
                            count,
                        },
                    ),
            ),
            ExprView::Div(numerator, denominator) => {
                if let Some((top, bottom)) =
                    PreparedStep::exact_ratio(denominator).filter(|(top, _)| *top != 0)
                {
                    Some(Form::Scale {
                        numerator: numerator.clone(),
                        divisor: RationalType::from_parts(top, bottom),
                    })
                } else {
                    Some(self.variable(node))
                }
            }
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

    fn lower(
        self,
        left: &Expr,
        right: &Expr,
        work: &mut PolynomialWork,
    ) -> Result<(SparsePolynomial, SparsePolynomial, Vec<Expr>), SimplifyError> {
        let dimensions = self.bases.len();
        let mut values = HashMap::<Expr, SparsePolynomial>::new();
        for (node, form) in self.order {
            let value = match form {
                Form::Scalar(value) => SparsePolynomial::constant(value, dimensions, work)?,
                Form::Variable(index) => SparsePolynomial::variable(dimensions, index, work)?,
                Form::Sum => {
                    let mut sum = SparsePolynomial::zero(dimensions);
                    for term in node.children() {
                        sum.add_assign(
                            values.get(&term).expect("postorder visits logical addends"),
                            false,
                            work,
                        )?;
                    }
                    sum
                }
                Form::Product => {
                    let mut product = SparsePolynomial::one(dimensions, work)?;
                    for factor in node.children() {
                        product = product
                            .mul(values.get(&factor).expect("postorder visits factors"), work)?;
                    }
                    product
                }
                Form::Power { base, count } => values
                    .get(&base)
                    .expect("postorder visits power bases")
                    .pow(count, work)?,
                Form::Scale { numerator, divisor } => values
                    .get(&numerator)
                    .expect("postorder visits numerators")
                    .clone()
                    .divide_scalar(&divisor, work)?,
            };
            values.insert(node, value);
        }
        let second = values
            .remove(right)
            .expect("the traversal includes its second root");
        // Distinct roots transfer their polynomial storage without cloning.
        // Identical roots share one memo entry and require one owning copy.
        let first = if left == right {
            second.clone()
        } else {
            values
                .remove(left)
                .expect("the traversal includes its first root")
        };
        Ok((first, second, self.bases))
    }
}
