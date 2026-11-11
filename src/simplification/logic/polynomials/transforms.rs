//! Bounded expansion, complete common-factor extraction, and binomial identities.
//!
//! Expansion multiplies factors whose terms are exact rational multiples of
//! products of powers of symbols and $\pi$ as sparse polynomials and returns one
//! compact sum; other factors follow the same binary-exponentiation schedule as
//! expressions and charge the same generated term counts.
//!
//! Factoring reads a finite sum once into terms over a table of its distinct
//! bases: compact monomials straight from polynomial storage, other terms
//! through their manifest positive integer powers. Each set of terms is reduced
//! by its integer content, then by common bases with least multiplicities, then
//! (when enabled) by the binomial identities $u^2 - v^2 = (u - v)(u + v)$,
//! $u^3 + v^3 = (u + v)(u^2 - uv + v^2)$, $u^2 \pm 2uv + v^2 = (u \pm v)^2$, and
//! $u^3 + 3u^2v + 3uv^2 + v^3 = (u + v)^3$ over exact rational roots of whole
//! terms, and finally by greedy multivariate Horner grouping (Ceberio and
//! Kreinovich 2004).

use core::{
    num::NonZeroU64,
    ops::{Mul, Neg},
    slice::from_ref,
};

use alloc::{vec, vec::Vec};

use num_anafis::{NonZeroU64Ext, Number, RationalType};

use crate::{
    core::{ExactIdentity, Expr, ExprView, Property, StoredForm, Truth},
    errors::SimplifyError,
    polynomial::{PolynomialInput, PolynomialWork},
};

use super::{Factoring, Level, Start, Term};

/// Namespace for bounded polynomial expansion and complete factoring.
pub struct Polynomials;

impl Polynomials {
    /// Expand products of sums and positive exact integer powers of sums.
    /// The distributive identity is \((a+b)c=ac+bc\).
    /// All generated Cartesian products reserve their work before materialization.
    pub fn expand(
        expression: &Expr,
        remaining: &mut usize,
        limit: usize,
    ) -> Result<Option<Expr>, SimplifyError> {
        let is_sum = |factor: &Expr| matches!(factor.view(), ExprView::Sum(_));
        let (factors, count) = match expression.view() {
            ExprView::Product(factors) if factors.iter().any(is_sum) => (factors, 1),
            ExprView::Pow(base, exponent) if is_sum(base) => {
                let Some((count, 1)) = exponent.exact_ratio().filter(|(count, _)| *count > 1)
                else {
                    return Ok(None);
                };
                (from_ref(base), count)
            }
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Function { .. }
            | ExprView::Sum(_)
            | ExprView::Product(_)
            | ExprView::Div(..)
            | ExprView::Pow(..)
            | ExprView::Derivative { .. } => return Ok(None),
        };
        if let Some((polynomials, bases)) = PolynomialInput::scalar_leaves(factors) {
            let reserved = *remaining;
            // Construction bounds the inputs; `remaining` bounds the products.
            let mut work = PolynomialWork::new(usize::MAX);
            let product = Self::schedule(polynomials, count, |left, right| {
                let sizes = (left.terms().len(), right.terms().len());
                if sizes.0 > 1 || sizes.1 > 1 {
                    Self::charge(sizes, remaining, limit)?;
                }
                // An exponent beyond `u32` also declines to expression arithmetic.
                Ok(left.checked_mul(right, &mut work).ok().flatten())
            })?;
            if let Some(expanded) = product.and_then(|polynomial| polynomial.to_expr(&bases)) {
                return Ok(Some(expanded));
            }
            // A declined product releases its charges to the expression path.
            *remaining = reserved;
        }
        Self::schedule(factors.to_vec(), count, |left, right| {
            if !is_sum(left) && !is_sum(right) {
                return Ok(Some(Mul::mul(left, right)));
            }
            let terms = |factor: &Expr| {
                if is_sum(factor) {
                    factor.children().cloned().collect()
                } else {
                    vec![factor.clone()]
                }
            };
            let (left_terms, right_terms): (Vec<_>, Vec<_>) = (terms(left), terms(right));
            Self::charge((left_terms.len(), right_terms.len()), remaining, limit)?;
            let mut products =
                Vec::with_capacity(left_terms.len().saturating_mul(right_terms.len()));
            for first in &left_terms {
                for second in &right_terms {
                    products.push(Mul::mul(first, second));
                }
            }
            Ok(Some(Expr::sum(products)))
        })
    }

    /// The product of `factors` in order, raised to `count` by binary
    /// exponentiation that starts at the lowest set bit, so no unit is
    /// multiplied. `None` when a multiplication declines.
    fn schedule<Value: Clone>(
        factors: Vec<Value>,
        mut count: i64,
        mut multiply: impl FnMut(&Value, &Value) -> Result<Option<Value>, SimplifyError>,
    ) -> Result<Option<Value>, SimplifyError> {
        let mut values = factors.into_iter();
        let Some(mut power) = values.next() else {
            return Ok(None);
        };
        for factor in values {
            let Some(product) = multiply(&power, &factor)? else {
                return Ok(None);
            };
            power = product;
        }
        // `count` is positive, so it has a lowest set bit.
        while count & 1 == 0 {
            let Some(square) = multiply(&power, &power)? else {
                return Ok(None);
            };
            power = square;
            count >>= 1;
        }
        let mut result = power.clone();
        count >>= 1;
        while count != 0 {
            let Some(square) = multiply(&power, &power)? else {
                return Ok(None);
            };
            power = square;
            if count & 1 != 0 {
                let Some(product) = multiply(&result, &power)? else {
                    return Ok(None);
                };
                result = product;
            }
            count >>= 1;
        }
        Ok(Some(result))
    }

    /// Reserve the term count of one Cartesian product.
    fn charge(
        (left, right): (usize, usize),
        remaining: &mut usize,
        limit: usize,
    ) -> Result<(), SimplifyError> {
        let count = left
            .checked_mul(right)
            .ok_or(SimplifyError::ExpansionLimitExceeded { limit })?;
        *remaining = remaining
            .checked_sub(count)
            .ok_or(SimplifyError::ExpansionLimitExceeded { limit })?;
        Ok(())
    }

    /// Factor a finite sum completely: integer content, common manifest powers, the
    /// binomial identities when `identities` is set, and nested grouping. `None`
    /// when no factor is found.
    pub fn factor(expression: &Expr, identities: bool) -> Option<Expr> {
        if expression.stored_terms().is_none() && expression.monomials().is_none() {
            return None;
        }
        if expression.properties().get(Property::Finite) != Truth::True {
            return None;
        }
        let mut factoring = Factoring::new(identities);
        let terms = factoring.read(expression);
        factoring.run(terms)
    }
}

impl Factoring {
    /// The logical terms of a sum, without building compact monomials.
    fn read(&mut self, expression: &Expr) -> Vec<Term> {
        let operands = expression
            .stored_terms()
            .unwrap_or_else(|| from_ref(expression));
        let mut terms = Vec::with_capacity(operands.len());
        for operand in operands {
            if let Some(monomials) = operand.monomials() {
                terms.reserve(monomials.len());
                // Distinct base positions name distinct bases.
                for (coefficient, powers) in monomials {
                    terms.push(Term {
                        coefficient: (!coefficient.is_exact_one()).then(|| coefficient.clone()),
                        powers: powers
                            .map(|(base, exponent)| (self.base(base), i64::from(exponent)))
                            .collect(),
                    });
                }
                continue;
            }
            let factors = if let ExprView::Product(factors) = operand.view() {
                factors
            } else {
                from_ref(operand)
            };
            let mut term = Term::default();
            for factor in factors {
                if factor.as_number().is_some() {
                    term.coefficient = Some(term.coefficient.take().map_or_else(
                        || factor.clone(),
                        |coefficient| Expr::product(vec![coefficient, factor.clone()]),
                    ));
                } else {
                    let (base, count) = factor.positive_factor().unwrap_or((factor, 1));
                    term.powers.push((self.base(base), count));
                }
            }
            term.powers.sort_unstable_by_key(|&(base, _)| base);
            // Canonical products collect equal bases; merging keeps one power
            // per base for any other input.
            term.powers.dedup_by(|later, earlier| {
                let equal = later.0 == earlier.0;
                if equal {
                    earlier.1 = earlier.1.saturating_add(later.1);
                }
                equal
            });
            terms.push(term);
        }
        terms
    }

    /// Reduce and group the sets depth first. `None` when the outermost set
    /// is left unchanged.
    fn run(&mut self, terms: Vec<Term>) -> Option<Expr> {
        let mut stack: Vec<Level> = Vec::new();
        let mut pending = terms;
        loop {
            let mut value = match self.start(pending) {
                Start::Done(value) => value,
                Start::Grouping(mut level) => {
                    if let Some(group) = level.next() {
                        self.mark_changed(true);
                        stack.push(level);
                        pending = group;
                        continue;
                    }
                    if stack.is_empty() && !self.is_changed() {
                        return None;
                    }
                    level.finish(self)
                }
            };
            pending = loop {
                let Some(mut parent) = stack.pop() else {
                    return self.is_changed().then_some(value);
                };
                let (base, count) = parent.extracted();
                parent.push_grouped(Expr::product(vec![self.power(base, count), value]));
                if let Some(group) = parent.next() {
                    stack.push(parent);
                    break group;
                }
                value = parent.finish(self);
            };
        }
    }

    /// Remove the integer content and the common powers of a set, then apply
    /// the binomial identities or prepare the grouping.
    fn start(&mut self, mut terms: Vec<Term>) -> Start {
        if terms.len() < 2 {
            return Start::Done(
                terms
                    .pop()
                    .map_or_else(|| Expr::number(0), |term| self.term(&term)),
            );
        }
        let mut enclosing = Vec::new();
        if let Some(content) = Self::content(&mut terms) {
            enclosing.push(Expr::number(content));
        }
        self.common(&mut terms, &mut enclosing);
        self.mark_changed(!enclosing.is_empty());
        let identity = if self.has_identities() {
            match terms.as_slice() {
                [first, second] => self.pair(first, second),
                [first, second, third] => self.square([first, second, third]),
                [first, second, third, fourth] => self.cube([first, second, third, fourth]),
                _ => None,
            }
        } else {
            None
        };
        if let Some(product) = identity {
            self.mark_changed(true);
            enclosing.push(product);
            return Start::Done(Expr::product(enclosing));
        }
        Start::Grouping(self.level(terms, enclosing))
    }

    /// Divide out the GCD $g > 1$ of exact integer coefficients and return
    /// it. The scan stops at the first unit or other coefficient.
    fn content(terms: &mut [Term]) -> Option<i64> {
        let value = |term: &Term| {
            term.coefficient
                .as_ref()
                .map_or(Some((1, 1)), ExactIdentity::exact_ratio)
                .and_then(|(numerator, denominator)| (denominator == 1).then_some(numerator))
        };
        let mut magnitude = None::<NonZeroU64>;
        for term in terms.iter() {
            if let Some(coefficient) = NonZeroU64::new(value(term)?.unsigned_abs()) {
                let divisor =
                    magnitude.map_or(coefficient, |current| current.gcd(coefficient.get()));
                if divisor == NonZeroU64::MIN {
                    return None;
                }
                magnitude = Some(divisor);
            }
        }
        let common = i64::try_from(magnitude?.get()).ok()?;
        // `common > 1` divides every coefficient, so no quotient fails.
        let quotients = terms
            .iter()
            .map(|term| value(term)?.checked_div(common))
            .collect::<Option<Vec<_>>>()?;
        for (term, quotient) in terms.iter_mut().zip(quotients) {
            term.coefficient = (quotient != 1).then(|| Expr::number(quotient));
        }
        Some(common)
    }

    /// Remove each base of the first term that every term contains, with its
    /// least multiplicity, into `enclosing`.
    fn common(&self, terms: &mut [Term], enclosing: &mut Vec<Expr>) {
        let Some((first, rest)) = terms.split_first() else {
            return;
        };
        let shared: Vec<(usize, i64)> = first
            .powers
            .iter()
            .filter_map(|&(base, count)| {
                rest.iter()
                    .try_fold(count, |least, term| {
                        let &(_, other) = term.powers.iter().find(|(other, _)| *other == base)?;
                        Some(least.min(other))
                    })
                    .map(|least| (base, least))
            })
            .collect();
        if shared.is_empty() {
            return;
        }
        for term in terms.iter_mut() {
            term.powers.retain_mut(|(base, count)| {
                if let Some(&(_, least)) = shared.iter().find(|(other, _)| other == base) {
                    *count = count.saturating_sub(least);
                }
                *count != 0
            });
        }
        enclosing.extend(shared.iter().map(|&(base, least)| self.power(base, least)));
    }

    /// $u^2 - v^2 = (u - v)(u + v)$, else $u^3 + v^3 = (u + v)(u^2 - uv + v^2)$
    /// with signed cube roots.
    fn pair(&self, first: &Term, second: &Term) -> Option<Expr> {
        let nested = |sum: Expr| Polynomials::factor(&sum, true).unwrap_or(sum);
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
    fn square(&self, terms: [&Term; 3]) -> Option<Expr> {
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
                let negative_last = forward.children().last().is_some_and(|term| {
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
            let nested = Polynomials::factor(&base, true).unwrap_or(base);
            return Some(Expr::product(vec![Expr::number(sign), nested.pow(2)]));
        }
        None
    }

    /// $u^3 + 3u^2v + 3uv^2 + v^3 = (u + v)^3$ with signed cube roots.
    fn cube(&self, terms: [&Term; 4]) -> Option<Expr> {
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
                return Some(Polynomials::factor(&base, true).unwrap_or(base).pow(3));
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
