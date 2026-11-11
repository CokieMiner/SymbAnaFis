//! Identity-bound differentiation over numbered logical expression DAGs and
//! factored arithmetic derivative rules.

use core::{
    convert::Infallible,
    num::NonZeroU32,
    ops::{Add, Div, Mul, Neg, Sub},
    slice::from_ref,
};

use alloc::{vec, vec::Vec};

use crate::{
    collections::{Entry, FixedMap},
    core::{
        CompactStorage, ExactIdentity, Expr, ExprView, Forest, Mapped, Postorder, Symbol,
        SymbolData,
    },
};

use super::Functions;

/// Differentiation by one borrowed symbol identity or along a linear
/// combination of symbol directions (forward-mode Jacobian-vector product).
///
/// A numbering is evaluated once per pass, so every distinct node, including
/// shared logical polynomial monomials, is differentiated once.
pub struct Differentiation<'variable> {
    target: Target<'variable>,
    zero: Expr,
    one: Expr,
    /// The partials of a multi-argument call's arguments.
    scratch: Vec<Expr>,
}

enum Target<'variable> {
    Single {
        variable: Symbol<'variable>,
        /// The variable as an expression, compared by identity with stored bases.
        variable_expr: Expr,
    },
    Directional {
        /// Active `(variable, variable_expr, direction)` tuples with distinct
        /// identities and non-exact-zero direction components.
        entries: Vec<(Symbol<'variable>, Expr, Expr)>,
        /// Maps symbol `id` to its slot in `entries`.
        lookup: FixedMap<usize, usize>,
    },
}

impl<'variable> Differentiation<'variable> {
    pub fn new(variable: Symbol<'variable>) -> Self {
        Self {
            target: Target::Single {
                variable,
                variable_expr: variable.to_expr(),
            },
            zero: Expr::number(0),
            one: Expr::number(1),
            scratch: Vec::new(),
        }
    }

    /// Build a forward-mode directional engine for `direction`. Duplicate
    /// symbols sum their direction components; exact-zero entries are dropped.
    /// Returns `None` when every direction component vanishes.
    pub fn directional(direction: &[(Symbol<'variable>, Expr)]) -> Option<Self> {
        let mut combined: Vec<(Symbol<'variable>, Expr)> = Vec::with_capacity(direction.len());
        let mut positions: FixedMap<usize, usize> = FixedMap::default();
        for (symbol, component) in direction {
            if component.is_exact_zero() {
                continue;
            }
            match positions.entry(symbol.id()) {
                Entry::Occupied(occupied) => {
                    if let Some((_, existing)) = combined.get_mut(*occupied.get()) {
                        *existing = Add::add(&*existing, component);
                    }
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(combined.len());
                    combined.push((*symbol, component.clone()));
                }
            }
        }
        let mut entries = Vec::with_capacity(combined.len());
        let mut lookup = FixedMap::default();
        for (symbol, component) in combined {
            if !component.is_exact_zero() {
                lookup.insert(symbol.id(), entries.len());
                entries.push((symbol, symbol.to_expr(), component));
            }
        }
        if entries.is_empty() {
            return None;
        }
        Some(Self {
            target: Target::Directional { entries, lookup },
            zero: Expr::number(0),
            one: Expr::number(1),
            scratch: Vec::new(),
        })
    }

    /// Retarget this single-variable engine at `variable`, keeping the constant
    /// handles and the argument scratch capacity. Retargeting to the current
    /// variable is free.
    pub fn rebind(&mut self, variable: Symbol<'variable>) {
        if let Target::Single {
            variable: current,
            variable_expr,
        } = &mut self.target
        {
            if *current == variable {
                return;
            }
            *current = variable;
            *variable_expr = variable.to_expr();
        } else {
            self.target = Target::Single {
                variable,
                variable_expr: variable.to_expr(),
            };
        }
    }

    /// The derivative of the root of `order`, paired with whether a rule
    /// resolved the root directly (`false` when the root became a deferred
    /// first-order derivative node).
    pub fn evaluate(&mut self, order: &Postorder<'_>) -> (Expr, bool) {
        let mut resolved = false;
        let Ok(root) = order.evaluate(|node| {
            let partial = self.partial(node);
            // The root is evaluated last, so `resolved` ends describing it.
            resolved = partial.is_some();
            Ok::<_, Infallible>(partial.unwrap_or_else(|| self.defer(node.expression())))
        });
        (root, resolved)
    }

    /// The derivatives of the roots of `forest`, in root order. A node without
    /// a rule, a root included, becomes its first-order derivative node.
    pub fn evaluate_forest(&mut self, forest: &Forest<'_>) -> Vec<Expr> {
        let Ok(roots) = forest.evaluate(|node| {
            Ok::<_, Infallible>(
                self.partial(node)
                    .unwrap_or_else(|| self.defer(node.expression())),
            )
        });
        roots
    }

    /// The partial or directional derivative of one node from the derivatives
    /// of its logical children, or `None` when no rule applies to the node.
    fn partial(&mut self, node: &Mapped<'_, '_, Expr>) -> Option<Expr> {
        let expression = node.expression();
        match expression.view() {
            // A literal is a constant, finite or not, so its derivative is
            // exact zero. Keeping a non-finite literal symbolic would leak it
            // into the chain rule and make the whole derivative unusable.
            ExprView::Number(_) | ExprView::ImaginaryUnit | ExprView::Pi | ExprView::Euler => {
                Some(self.zero.clone())
            }
            ExprView::Symbol(symbol) => Some(self.symbol_partial(symbol)),
            // Compact storage in symbols, $\pi$, and $e$ is differentiated from
            // its stored terms through `node.group()`.
            ExprView::Sum(_) => {
                let group = node.group().and_then(|storage| self.group_partial(storage));
                if node.values().next().is_none() {
                    return group;
                }
                Some(
                    self.sum_terms(
                        node.values()
                            .chain(&group)
                            .filter(|term| !term.is_exact_zero())
                            .cloned(),
                    ),
                )
            }
            ExprView::Product(factors) => {
                node.values().next()?;
                if node.values().all(|child| *child == self.zero) {
                    return Some(self.zero.clone());
                }
                Some(Self::product(factors, node.values()))
            }
            // A numbering that keeps a node whole gives it no links, so the
            // operands are read through the total iterator: a node whose
            // children this numbering does not expand has no rule here and
            // becomes a derivative node instead of an out-of-bounds read.
            ExprView::Div(numerator, denominator) => {
                let mut values = node.values();
                let (Some(top), Some(bottom)) = (values.next(), values.next()) else {
                    return None;
                };
                if *top == self.zero && *bottom == self.zero {
                    return Some(self.zero.clone());
                }
                Some(Self::quotient(
                    numerator,
                    denominator,
                    top.clone(),
                    bottom.clone(),
                ))
            }
            ExprView::Pow(base, exponent) => {
                let mut values = node.values();
                let (Some(base_prime), Some(exponent_prime)) = (values.next(), values.next())
                else {
                    return None;
                };
                if *base_prime == self.zero && *exponent_prime == self.zero {
                    return Some(self.zero.clone());
                }
                Self::power(
                    expression,
                    base,
                    exponent,
                    base_prime.clone(),
                    exponent_prime.clone(),
                )
            }
            ExprView::Function { name, args } => {
                if args.len() == 1 {
                    let prime = node.values().next()?;
                    if *prime == self.zero {
                        return Some(self.zero.clone());
                    }
                    return Functions::function(expression, name, args, from_ref(prime));
                }
                node.values().next()?;
                if node.values().all(|child| *child == self.zero) {
                    return Some(self.zero.clone());
                }
                self.scratch.clear();
                self.scratch.extend(node.values().cloned());
                Functions::function(expression, name, args, &self.scratch)
            }
            ExprView::Derivative { .. } => {
                let inner = node.values().next()?;
                // $\partial_x\partial_y f = \partial_y(\partial_x f)$, and the derivative
                // of any literal, including an approximate zero, is exact zero.
                inner.is_zero_num().then(|| self.zero.clone())
            }
        }
    }

    fn symbol_partial(&self, symbol: Symbol<'_>) -> Expr {
        match &self.target {
            Target::Single { variable, .. } => {
                if symbol == *variable {
                    self.one.clone()
                } else {
                    self.zero.clone()
                }
            }
            Target::Directional { entries, lookup } => lookup
                .get(&symbol.id())
                .and_then(|&slot| entries.get(slot))
                .map_or_else(|| self.zero.clone(), |(_, _, direction)| direction.clone()),
        }
    }

    fn group_partial(&self, storage: &Expr) -> Option<Expr> {
        match &self.target {
            Target::Single { variable_expr, .. } => storage.polynomial_derivative(variable_expr),
            Target::Directional { entries, .. } => {
                let mut terms = Vec::new();
                for (_, variable_expr, direction) in entries {
                    let partial = storage.polynomial_derivative(variable_expr)?;
                    if !partial.is_exact_zero() {
                        terms.push(Mul::mul(partial, direction));
                    }
                }
                Some(self.sum_terms(terms))
            }
        }
    }

    fn defer(&self, expression: &Expr) -> Expr {
        match &self.target {
            Target::Single { variable, .. } => {
                SymbolData::of(*variable).derivative_of(expression.clone(), NonZeroU32::MIN)
            }
            Target::Directional { entries, .. } => self.sum_terms(
                entries
                    .iter()
                    .filter(|(variable, _, _)| expression.contains_symbol(*variable))
                    .map(|(variable, _, direction)| {
                        let deferred = SymbolData::of(*variable)
                            .derivative_of(expression.clone(), NonZeroU32::MIN);
                        Mul::mul(deferred, direction)
                    }),
            ),
        }
    }

    /// Sum active terms without allocating a vector for zero or one term.
    fn sum_terms(&self, terms: impl IntoIterator<Item = Expr>) -> Expr {
        let mut iter = terms.into_iter();
        let Some(first) = iter.next() else {
            return self.zero.clone();
        };
        let Some(second) = iter.next() else {
            return first;
        };
        let mut collected = Vec::with_capacity(iter.size_hint().0.saturating_add(2));
        collected.push(first);
        collected.push(second);
        collected.extend(iter);
        Expr::sum(collected)
    }

    /// Apply the product rule without dividing by any input factor.
    /// Constant factors occur once outside a balanced derivative expression.
    /// For active groups, $(AB)'=A'B+AB'$ bounds intermediate storage by
    /// $O(n\log n)$, rather than materializing $n$ flat products of length $n$.
    ///
    /// Active factors merge as in a binary counter: a group merges with the
    /// group before it while both span equally many factors, and the last
    /// factor folds every remaining group from the right. This is the tree of
    /// pairing neighbours level by level, and the product of all active
    /// factors, which the derivative does not use, is never formed.
    fn product<'prime>(factors: &[Expr], primes: impl Iterator<Item = &'prime Expr>) -> Expr {
        let mut first_constant = None;
        let mut constants = Vec::new();
        // Each group's product, derivative, and merge level.
        let mut groups: Vec<(Expr, Expr, u32)> = Vec::new();
        let mut last: Option<(Expr, Expr)> = None;
        for (factor, prime) in factors.iter().zip(primes) {
            if prime.is_exact_zero() {
                if let Some(previous) = first_constant.take() {
                    constants.push(previous);
                    constants.push(factor.clone());
                } else if constants.is_empty() {
                    first_constant = Some(factor.clone());
                } else {
                    constants.push(factor.clone());
                }
                continue;
            }
            let Some((mut product, mut derivative)) = last.replace((factor.clone(), prime.clone()))
            else {
                continue;
            };
            let mut level = 0_u32;
            while let Some((left, left_prime, _)) = groups.pop_if(|(_, _, merged)| *merged == level)
            {
                derivative = Add::add(Mul::mul(left_prime, &product), Mul::mul(&left, derivative));
                product = Mul::mul(left, product);
                level = level.saturating_add(1);
            }
            groups.push((product, derivative, level));
        }
        let Some((mut product, mut derivative)) = last else {
            return Expr::number(0);
        };
        while let Some((left, left_prime, _)) = groups.pop() {
            derivative = Add::add(Mul::mul(left_prime, &product), Mul::mul(&left, derivative));
            if groups.is_empty() {
                break;
            }
            product = Mul::mul(left, product);
        }
        if let Some(constant) = first_constant {
            return Mul::mul(constant, derivative);
        }
        if constants.is_empty() {
            return derivative;
        }
        constants.push(derivative);
        Expr::product(constants)
    }

    /// Apply $(u/v)'=(u'v-uv')/v^2$ on the quotient's regular domain.
    /// A constant denominator uses $u'/v$, retaining its original pole.
    /// Callers guarantee at least one operand derivative is non-exact-zero.
    fn quotient(
        numerator: &Expr,
        denominator: &Expr,
        numerator_prime: Expr,
        denominator_prime: Expr,
    ) -> Expr {
        if denominator_prime.is_exact_zero() {
            return Div::div(numerator_prime, denominator);
        }
        let top = if numerator_prime.is_exact_zero() {
            Neg::neg(Mul::mul(numerator, denominator_prime))
        } else {
            Sub::sub(
                Mul::mul(numerator_prime, denominator),
                Mul::mul(numerator, denominator_prime),
            )
        };
        Div::div(top, denominator.pow(2))
    }

    /// Apply $(u^v)'=v u^{v-1}u'+u^v\log(u)v'$ on regular principal branches.
    /// Constant exponents omit the logarithm; exponent one omits the zero power.
    /// Callers guarantee at least one operand derivative is non-exact-zero.
    fn power(
        expression: &Expr,
        base: &Expr,
        exponent: &Expr,
        base_prime: Expr,
        exponent_prime: Expr,
    ) -> Option<Expr> {
        if base.is_zero_num() && !exponent_prime.is_exact_zero() {
            return None;
        }
        let base_term = (!base_prime.is_exact_zero() && !exponent.is_zero_num()).then(|| {
            if exponent.is_exact_one() {
                base_prime
            } else if exponent.is_one_num() {
                // An approximate unit exponent keeps its tier: $1.0 u^{0.0}u' = 1.0 u'$.
                Mul::mul(exponent, base_prime)
            } else {
                let lowered = match exponent.exact_ratio() {
                    Some((2, 1)) => base.clone(),
                    Some((n, 1)) if n != i64::MIN => base.pow(n.saturating_sub(1)),
                    _ => base.pow(Sub::sub(exponent, 1)),
                };
                if base_prime.is_exact_one() {
                    Mul::mul(exponent, lowered)
                } else {
                    Expr::product(vec![exponent.clone(), lowered, base_prime])
                }
            }
        });
        let exponent_term = (!exponent_prime.is_exact_zero()).then(|| {
            if exponent_prime.is_exact_one() {
                Mul::mul(expression, base.ln())
            } else {
                Expr::product(vec![expression.clone(), base.ln(), exponent_prime])
            }
        });
        Some(match (base_term, exponent_term) {
            (Some(left), Some(right)) => Add::add(left, right),
            (Some(term), None) | (None, Some(term)) => term,
            (None, None) => Expr::number(0),
        })
    }
}
