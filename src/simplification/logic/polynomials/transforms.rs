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

use core::{num::NonZeroU64, ops::Mul, slice::from_ref};

use alloc::{vec, vec::Vec};

use num_anafis::NonZeroU64Ext;

use crate::{
    core::{ExactIdentity, Expr, ExprView, Property, StoredForm, Truth},
    errors::SimplifyError,
    polynomial::{PolynomialInput, PolynomialWork},
};

use super::{Factoring, Level, Memo, Start, Term};

/// Namespace for bounded polynomial expansion and complete factoring.
pub struct Polynomials;

impl Polynomials {
    /// Expand products of sums and positive exact integer powers of sums.
    /// The distributive identity is $(a+b)c=ac+bc$.
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
            // Counting a compact group does not build its terms, so the budget
            // is charged before either operand list is materialized.
            let count_of = |factor: &Expr| {
                if is_sum(factor) {
                    factor.children().len()
                } else {
                    1
                }
            };
            Self::charge((count_of(left), count_of(right)), remaining, limit)?;
            let terms = |factor: &Expr| {
                if is_sum(factor) {
                    factor.children().cloned().collect()
                } else {
                    vec![factor.clone()]
                }
            };
            let (left_terms, right_terms): (Vec<_>, Vec<_>) = (terms(left), terms(right));
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
    /// binomial identities, and nested grouping. `None` when no factor is found.
    ///
    /// `minimize` keeps a grouped sum only when it stores no more nodes than the
    /// input, which is the `compact()` goal; `factored()` and `compilation()` pass
    /// `false` so an explicit `Factor` pass keeps its nested factorization.
    pub fn factor(expression: &Expr, minimize: bool) -> Option<Expr> {
        if (expression.stored_terms().is_none() && expression.monomials().is_none())
            || expression.properties().get(Property::Finite) != Truth::True
        {
            return None;
        }
        Self::factor_uncached(expression, &mut Memo::new(minimize))
    }

    /// Factor a sub-sum the binomial identities built, reusing the memo.
    ///
    /// The identities re-enter factoring on the sums they build, and the
    /// grouping of those sums re-enters the identities, so a chain such as
    /// $x^{2^k}-y^{2^k}$ reaches the same sub-sums along many paths. The memo
    /// makes each distinct sub-sum cost one factoring.
    pub fn factor_memo(expression: &Expr, memo: &mut Memo) -> Option<Expr> {
        if (expression.stored_terms().is_none() && expression.monomials().is_none())
            || expression.properties().get(Property::Finite) != Truth::True
        {
            return None;
        }
        if let Some(cached) = memo.entries.get(expression) {
            return cached.clone();
        }
        let result = Self::factor_uncached(expression, memo);
        memo.entries.insert(expression.clone(), result.clone());
        result
    }

    /// Factor a sum whose guards and memo entry the caller has already read.
    fn factor_uncached(expression: &Expr, memo: &mut Memo) -> Option<Expr> {
        let mut factoring = Factoring::new();
        let terms = factoring.read(expression);
        let result = factoring.run(terms, memo)?;
        // A grouped sum that only re-associates the terms is kept only when it
        // stores no more nodes than the compact storage it replaces, which is
        // smaller than any tree of the same monomials. A grouped product
        // carries an extracted common factor, so it is kept regardless of
        // stored size. Only the `compact()` goal asks for the smaller form;
        // `factored()` keeps the factorization.
        let reassociated = memo.minimize
            && matches!(result.view(), ExprView::Sum(_))
            && result.node_count() > expression.node_count();
        // A declined grouping returns the input rather than `None`, so the
        // caller does not settle the node: the size goal is the sequence's, and
        // a later `factored()` run over the same node must still factor it.
        if reassociated {
            return Some(expression.clone());
        }
        Some(result)
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
    fn run(&mut self, terms: Vec<Term>, memo: &mut Memo) -> Option<Expr> {
        let mut stack: Vec<Level> = Vec::new();
        let mut pending = terms;
        loop {
            let mut value = match self.start(pending, memo) {
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
    fn start(&mut self, mut terms: Vec<Term>, memo: &mut Memo) -> Start {
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
        let identity = match terms.as_slice() {
            [first, second] => self.pair(first, second, memo),
            [first, second, third] => self.square([first, second, third], memo),
            [first, second, third, fourth] => self.cube([first, second, third, fourth], memo),
            _ => None,
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
        // The scan above proved every coefficient an exact integer, and `common`
        // is positive and divides each of them, so no quotient fails and no
        // coefficient is left divided.
        for term in terms.iter_mut() {
            let quotient = value(term)?.checked_div(common)?;
            term.coefficient = (quotient != 1).then(|| Expr::number(quotient));
        }
        Some(common)
    }

    /// Remove each base of the first term that every term contains, with its
    /// least multiplicity, into `enclosing`.
    ///
    /// One slot per base of the factoring holds the least multiplicity seen so
    /// far and the last term that carried it, so every term's powers are read
    /// once: $O(N P + P D)$ for $N$ terms of $P$ powers over $D$ bases,
    /// against $O(N P^2)$ for a scan per candidate base. The first term is
    /// generation one and the $i$-th later term generation $i+2$, so a base is
    /// common exactly when the last term carried it, which is the mark the
    /// final term leaves; the extraction reads that slot instead of searching
    /// the common list.
    fn common(&self, terms: &mut [Term], enclosing: &mut Vec<Expr>) {
        let Some((first, rest)) = terms.split_first() else {
            return;
        };
        let mut least = vec![0_i64; self.base_count()];
        let mut seen = vec![0_usize; self.base_count()];
        let mut shared: Vec<usize> = Vec::with_capacity(first.powers.len());
        for &(base, count) in &first.powers {
            if let (Some(slot), Some(mark)) = (least.get_mut(base), seen.get_mut(base)) {
                *slot = count;
                *mark = 1;
                shared.push(base);
            }
        }
        for (index, term) in rest.iter().enumerate() {
            let generation = index.saturating_add(2);
            let previous = generation.saturating_sub(1);
            for &(base, count) in &term.powers {
                // Only a base the previous term carried is still a candidate;
                // one a term dropped keeps its old mark and must not revive.
                if let (Some(slot), Some(mark)) = (least.get_mut(base), seen.get_mut(base))
                    && *mark == previous
                {
                    *slot = (*slot).min(count);
                    *mark = generation;
                }
            }
            // A base this term lacks keeps an older mark and is not common.
            shared.retain(|&base| seen.get(base) == Some(&generation));
            if shared.is_empty() {
                return;
            }
        }
        // Only a base the last term carried is common; a base of the first term
        // that a later one dropped keeps its mark and must not be removed.
        let common = rest.len().saturating_add(1);
        for term in terms.iter_mut() {
            term.powers.retain_mut(|(base, count)| {
                if seen.get(*base) == Some(&common)
                    && let Some(&slot) = least.get(*base)
                {
                    *count = count.saturating_sub(slot);
                }
                *count != 0
            });
        }
        enclosing.extend(
            shared
                .iter()
                .map(|&base| self.power(base, least.get(base).copied().unwrap_or(0))),
        );
    }
}
