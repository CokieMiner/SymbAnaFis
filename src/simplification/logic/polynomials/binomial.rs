//! The binomial identities over exact rational roots of whole terms.
//!
//! $u^2 - v^2 = (u - v)(u + v)$, $u^3 + v^3 = (u + v)(u^2 - uv + v^2)$ with
//! signed cube roots, $s(u^2 \pm 2uv + v^2) = s(u \pm v)^2$ for square terms
//! $su^2$ and $sv^2$ of one sign, and $u^3 + 3u^2v + 3uv^2 + v^3 = (u + v)^3$.
//! Each identity factors the sums it builds through the memo, so a chain of
//! them reaches every distinct sub-sum once.

use core::{num::NonZeroU64, ops::Neg};

use alloc::{vec, vec::Vec};

use num_anafis::{NonZeroU64Ext, Number, RationalType};

use crate::{
    collections::FixedMap,
    core::{ExactIdentity, Expr, ExprView},
};

use super::{Factoring, Polynomials, Term};

/// The sub-sums a factoring run has already factored, with their results, and
/// whether the run keeps a grouped sum only when it stores no more nodes.
///
/// The size goal belongs to the sequence, not to the factoring: `compact()`
/// promises the smallest stored form, while `factored()` promises a factored
/// one even when the tree is larger than the compact storage it replaces. One
/// run has one goal, so the memo carries it and every nested factoring of the
/// same run inherits it.
pub struct Memo {
    pub entries: FixedMap<Expr, Option<Expr>>,
    pub minimize: bool,
}

impl Memo {
    /// An empty memo for a run that keeps a grouped sum only when `minimize`.
    pub fn new(minimize: bool) -> Self {
        Self {
            entries: FixedMap::default(),
            minimize,
        }
    }
}

impl Factoring {
    /// $u^2 - v^2 = (u - v)(u + v)$, else $u^3 + v^3 = (u + v)(u^2 - uv + v^2)$
    /// with signed cube roots.
    pub fn pair(&self, first: &Term, second: &Term, memo: &mut Memo) -> Option<Expr> {
        let mut nested = |sum: Expr| Polynomials::factor_memo(&sum, memo).unwrap_or(sum);
        if let (Some((first_negative, first_root)), Some((second_negative, second_root))) =
            (self.root(first, false), self.root(second, false))
            && first_negative != second_negative
        {
            let (positive, negative) = if first_negative {
                (second_root, first_root)
            } else {
                (first_root, second_root)
            };
            return Some(Expr::product(vec![
                nested(Expr::sum(vec![positive.clone(), Neg::neg(&negative)])),
                nested(Expr::sum(vec![positive, negative])),
            ]));
        }
        let (Some((_, left)), Some((_, right))) = (self.root(first, true), self.root(second, true))
        else {
            return None;
        };
        Some(Expr::product(vec![
            nested(Expr::sum(vec![left.clone(), right.clone()])),
            nested(Expr::sum(vec![
                left.pow(2),
                Neg::neg(Expr::product(vec![left, right.clone()])),
                right.pow(2),
            ])),
        ]))
    }

    /// $s(u^2 \pm 2uv + v^2) = s(u \pm v)^2$ for square terms $su^2$ and $sv^2$
    /// of one sign $s$.
    pub fn square(&self, terms: [&Term; 3], memo: &mut Memo) -> Option<Expr> {
        let [first, second, third] = terms;
        let [first_root, second_root, third_root] = terms.map(|term| self.root(term, false));
        for (middle, outer, inner) in [
            (first, &second_root, &third_root),
            (second, &first_root, &third_root),
            (third, &first_root, &second_root),
        ] {
            let (Some((negative, left)), Some((other_negative, right))) = (outer, inner) else {
                continue;
            };
            if negative != other_negative {
                continue;
            }
            let (sign, sum, difference) = if *negative { (-1, -2, 2) } else { (1, 2, -2) };
            let actual = self.term(middle);
            let twice =
                |scale: i64| Expr::product(vec![Expr::number(scale), left.clone(), right.clone()]);
            let base = if actual == twice(sum) {
                Expr::sum(vec![left.clone(), right.clone()])
            } else if actual == twice(difference) {
                // $(u - v)^2 = (v - u)^2$; the base keeps its last term positive.
                let forward = Expr::sum(vec![left.clone(), Neg::neg(right)]);
                // A canonical product leads with its literal coefficient.
                let negative_last = forward.children().next_back().is_some_and(|term| {
                    let coefficient = if let ExprView::Product(factors) = term.view() {
                        factors.first()
                    } else {
                        Some(term)
                    };
                    coefficient
                        .and_then(ExactIdentity::exact_ratio)
                        .is_some_and(|(numerator, _)| numerator < 0)
                });
                if negative_last {
                    Expr::sum(vec![right.clone(), Neg::neg(left)])
                } else {
                    forward
                }
            } else {
                continue;
            };
            let nested = Polynomials::factor_memo(&base, memo).unwrap_or(base);
            return Some(Expr::product(vec![Expr::number(sign), nested.pow(2)]));
        }
        None
    }

    /// $u^3 + 3u^2v + 3uv^2 + v^3 = (u + v)^3$ with signed cube roots.
    pub fn cube(&self, terms: [&Term; 4], memo: &mut Memo) -> Option<Expr> {
        let [first, second, third, fourth] = terms;
        let [first_root, second_root, third_root, fourth_root] =
            terms.map(|term| self.root(term, true));
        for (cubes, one, other) in [
            ((&first_root, &second_root), third, fourth),
            ((&first_root, &third_root), second, fourth),
            ((&first_root, &fourth_root), second, third),
            ((&second_root, &third_root), first, fourth),
            ((&second_root, &fourth_root), first, third),
            ((&third_root, &fourth_root), first, second),
        ] {
            let (Some((_, left)), Some((_, right))) = cubes else {
                continue;
            };
            let leading = Expr::product(vec![Expr::number(3), left.pow(2), right.clone()]);
            let trailing = Expr::product(vec![Expr::number(3), left.clone(), right.pow(2)]);
            let (first_middle, second_middle) = (self.term(one), self.term(other));
            if (first_middle == leading && second_middle == trailing)
                || (first_middle == trailing && second_middle == leading)
            {
                let base = Expr::sum(vec![left.clone(), right.clone()]);
                return Some(Polynomials::factor_memo(&base, memo).unwrap_or(base).pow(3));
            }
        }
        None
    }

    /// The square or cube root of a whole term over exact rationals. A square
    /// reports the sign of its coefficient separately; a cube root keeps it.
    fn root(&self, term: &Term, cube: bool) -> Option<(bool, Expr)> {
        let degree = if cube { 3 } else { 2 };
        if term
            .powers
            .iter()
            .any(|&(_, count)| count.checked_rem(degree) != Some(0))
        {
            return None;
        }
        let mut factors = Vec::with_capacity(term.powers.len().saturating_add(1));
        let mut negative = false;
        if let Some(coefficient) = &term.coefficient {
            let (numerator, denominator) = coefficient.exact_ratio()?;
            let magnitude = NonZeroU64::new(numerator.unsigned_abs())?;
            let divisor = NonZeroU64::new(denominator)?;
            let (top, bottom) = if cube {
                (magnitude.perfect_cube()?, divisor.perfect_cube()?)
            } else {
                (magnitude.perfect_square()?, divisor.perfect_square()?)
            };
            negative = numerator < 0;
            let root = i64::try_from(top.get()).ok()?;
            let signed = if negative && cube {
                root.checked_neg()?
            } else {
                root
            };
            factors.push(if bottom == NonZeroU64::MIN {
                Expr::number(signed)
            } else {
                Expr::number(Number::from_rational(
                    RationalType::from_integer(signed).checked_div(&RationalType::from_integer(
                        i64::try_from(bottom.get()).ok()?,
                    ))?,
                ))
            });
        }
        for &(base, count) in &term.powers {
            factors.push(self.power(base, count.checked_div(degree)?));
        }
        Some((negative && !cube, Expr::product(factors)))
    }
}
