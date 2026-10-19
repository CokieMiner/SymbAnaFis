//! Common manifest factors without division by symbolic values.

use core::ops::Mul;

use alloc::vec::Vec;

use crate::{
    collections::HashMap,
    core::{Expr, ExprView, Property},
};

use super::PreparedStep;

impl PreparedStep<'_> {
    /// Extract \(fg+fh=f(g+h)\), including integral power multiplicities.
    /// Numeric GCD extraction uses checked native integer division exclusively.
    pub fn factor(expression: &Expr) -> Option<Expr> {
        let ExprView::Sum(children) = expression.view() else {
            return None;
        };
        if !Self::proved(expression, Property::Finite) {
            return None;
        }
        let terms = children.collect::<Vec<_>>();
        if let Some(result) = Self::numeric_gcd(&terms) {
            return Some(result);
        }
        let decomposed = terms.iter().map(Self::factors).collect::<Vec<_>>();
        // Preserve first-occurrence order; hash iteration never determines a factor.
        let mut locations: HashMap<Expr, Vec<(usize, i64)>> = HashMap::new();
        let mut order = Vec::new();
        for (index, operands) in decomposed.iter().enumerate() {
            for (base, count) in operands {
                if base.as_number().is_some() {
                    continue;
                }
                let occurrences = locations.entry(base.clone()).or_insert_with(|| {
                    order.push(base.clone());
                    Vec::new()
                });
                occurrences.push((index, *count));
            }
        }
        let base = order
            .iter()
            .filter(|base| {
                locations
                    .get(*base)
                    .is_some_and(|indices| indices.len() > 1)
            })
            .max_by_key(|base| locations.get(*base).map_or(0, Vec::len))?;
        let indices = locations.get(base)?;
        let common = indices.iter().map(|(_, count)| *count).min()?;
        let mut remaining = Vec::with_capacity(terms.len());
        let mut grouped = Vec::with_capacity(indices.len());
        for ((index, term), operands) in terms.into_iter().enumerate().zip(decomposed) {
            if indices.iter().any(|(selected, _)| *selected == index) {
                let reduced = operands
                    .into_iter()
                    .map(|(candidate, count)| {
                        let exponent = if &candidate == base {
                            count.saturating_sub(common)
                        } else {
                            count
                        };
                        (candidate, exponent)
                    })
                    .collect();
                grouped.push(Self::reconstruct(reduced));
            } else {
                remaining.push(term);
            }
        }
        remaining.push(Mul::mul(base.pow(common), Expr::sum(grouped)));
        Some(Expr::sum(remaining))
    }

    fn numeric_gcd(terms: &[Expr]) -> Option<Expr> {
        let coefficients = terms
            .iter()
            .map(|term| {
                if let ExprView::Product(factors) = term.view()
                    && let Some(first) = factors.first()
                    && first.as_number().is_some()
                {
                    Self::exact_integer(first).map(|value| {
                        (
                            value,
                            Expr::product(factors.iter().skip(1).cloned().collect()),
                        )
                    })
                } else if term.as_number().is_some() {
                    Self::exact_integer(term).map(|value| (value, Expr::number(1)))
                } else {
                    Some((1, term.clone()))
                }
            })
            .collect::<Option<Vec<_>>>()?;
        let magnitude = coefficients.iter().fold(0_u64, |current, (value, _)| {
            Self::gcd(current, value.unsigned_abs())
        });
        let common = i64::try_from(magnitude).ok().filter(|value| *value > 1)?;
        let mut reduced = Vec::with_capacity(coefficients.len());
        for (value, base) in coefficients {
            // `common > 1`: division cannot be zero or the MIN / -1 case.
            let coefficient = value.checked_div(common)?;
            reduced.push(Mul::mul(Expr::number(coefficient), base));
        }
        Some(Mul::mul(Expr::number(common), Expr::sum(reduced)))
    }

    const fn gcd(mut left: u64, mut right: u64) -> u64 {
        while let Some(remainder) = left.checked_rem(right) {
            left = right;
            right = remainder;
        }
        left
    }
}
