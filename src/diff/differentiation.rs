//! Identity-bound differentiation over numbered logical expression DAGs and
//! factored arithmetic derivative rules.

#![expect(
    unsafe_code,
    reason = "quotient, power, and derivative nodes read their operands' partials by edge ordinal, which their edge counts bound"
)]

use core::{
    convert::Infallible,
    num::NonZeroU32,
    ops::{Div, Mul, Neg, Sub},
};

use alloc::{vec, vec::Vec};

use crate::core::{
    CompactStorage, ExactIdentity, Expr, ExprView, Forest, Mapped, Postorder, Symbol,
};

use super::Functions;

/// Differentiation by one borrowed symbol identity.
///
/// A numbering is evaluated once per variable, so every distinct node,
/// including shared logical polynomial monomials, is differentiated once.
pub struct Differentiation<'variable> {
    variable: Symbol<'variable>,
    /// The variable as an expression, compared by identity with stored bases.
    variable_expr: Expr,
    zero: Expr,
    one: Expr,
    /// The partials of a call's arguments.
    scratch: Vec<Expr>,
}

impl<'variable> Differentiation<'variable> {
    pub fn new(variable: Symbol<'variable>) -> Self {
        Self {
            variable,
            variable_expr: variable.to_expr(),
            zero: Expr::number(0),
            one: Expr::number(1),
            scratch: Vec::new(),
        }
    }

    /// The derivative of the root of `order`, or `None` when no rule applies
    /// to the root itself. A descendant without a rule becomes its first-order
    /// derivative node.
    pub fn evaluate(&mut self, order: &Postorder<'_>) -> Option<Expr> {
        let mut resolved = false;
        let Ok(root) = order.evaluate(|node| {
            let partial = self.partial(node);
            // The root is evaluated last, so `resolved` ends describing it.
            resolved = partial.is_some();
            Ok::<_, Infallible>(partial.unwrap_or_else(|| {
                self.variable
                    .metadata()
                    .derivative_of(node.expression().clone(), NonZeroU32::MIN)
            }))
        });
        resolved.then_some(root)
    }

    /// The derivatives of the roots of `forest`, in root order. A node without
    /// a rule, a root included, becomes its first-order derivative node.
    pub fn evaluate_forest(&mut self, forest: &Forest<'_>) -> Vec<Expr> {
        let Ok(roots) = forest.evaluate(|node| {
            Ok::<_, Infallible>(self.partial(node).unwrap_or_else(|| {
                self.variable
                    .metadata()
                    .derivative_of(node.expression().clone(), NonZeroU32::MIN)
            }))
        });
        roots
    }

    /// The partial of one node from the partials of its logical children, or
    /// `None` when no rule applies to the node.
    fn partial(&mut self, node: &Mapped<'_, '_, Expr>) -> Option<Expr> {
        let expression = node.expression();
        // Compact storage in symbols, \(\pi\), and \(e\) is differentiated from
        // its stored terms, whether it is the node or the group that the
        // numbering keeps whole in a sum.
        if let Some(derivative) = expression.polynomial_derivative(&self.variable_expr) {
            return Some(derivative);
        }
        let group = node
            .group()
            .and_then(|storage| storage.polynomial_derivative(&self.variable_expr));
        match expression.view() {
            ExprView::Number(value) => value.is_finite().then(|| self.zero.clone()),
            ExprView::ImaginaryUnit | ExprView::Pi | ExprView::Euler => Some(self.zero.clone()),
            ExprView::Symbol(symbol) => Some(if symbol == self.variable {
                self.one.clone()
            } else {
                self.zero.clone()
            }),
            // A node whose children, and group, are all constant is constant.
            _ if group.as_ref().is_none_or(ExactIdentity::is_exact_zero)
                && node.values().all(|child| *child == self.zero) =>
            {
                Some(self.zero.clone())
            }
            ExprView::Sum(_) => Some(Expr::sum(
                node.values()
                    .chain(&group)
                    .filter(|term| !term.is_exact_zero())
                    .cloned()
                    .collect(),
            )),
            ExprView::Product(factors) => Some(Self::product(factors, node.values())),
            ExprView::Div(numerator, denominator) => {
                // SAFETY: a quotient has two edges.
                let (top, bottom) = unsafe { (node.child(0), node.child(1)) };
                Some(Self::quotient(
                    numerator,
                    denominator,
                    top.clone(),
                    bottom.clone(),
                ))
            }
            ExprView::Pow(base, exponent) => {
                // SAFETY: a power has two edges.
                let (base_prime, exponent_prime) = unsafe { (node.child(0), node.child(1)) };
                Self::power(base, exponent, base_prime.clone(), exponent_prime.clone())
            }
            ExprView::Function { name, args } => {
                self.scratch.clear();
                self.scratch.extend(node.values().cloned());
                Functions::function(name, args, &self.scratch)
            }
            ExprView::Derivative { .. } => {
                // SAFETY: a derivative has one edge, its inner expression.
                let inner = unsafe { node.child(0) };
                // \(\partial_x\partial_y f = \partial_y(\partial_x f)\), and the derivative
                // of any literal, including an approximate zero, is exact zero.
                inner.is_zero_num().then(|| self.zero.clone())
            }
        }
    }

    /// Apply the product rule without dividing by any input factor.
    /// Constant factors occur once outside a balanced derivative expression.
    /// For active groups, \((AB)'=A'B+AB'\) bounds intermediate storage by
    /// \(O(n\log n)\), rather than materializing \(n\) flat products of length \(n\).
    ///
    /// Active factors merge as in a binary counter: a group merges with the
    /// group before it while both span equally many factors, and the last
    /// factor folds every remaining group from the right. This is the tree of
    /// pairing neighbours level by level, and the product of all active
    /// factors, which the derivative does not use, is never formed.
    fn product<'prime>(factors: &[Expr], primes: impl Iterator<Item = &'prime Expr>) -> Expr {
        let mut constants = Vec::new();
        // Each group's product, derivative, and merge level.
        let mut groups: Vec<(Expr, Expr, u32)> = Vec::new();
        let mut last: Option<(Expr, Expr)> = None;
        for (factor, prime) in factors.iter().zip(primes) {
            if prime.is_exact_zero() {
                constants.push(factor.clone());
                continue;
            }
            let Some((mut product, mut derivative)) = last.replace((factor.clone(), prime.clone()))
            else {
                continue;
            };
            let mut level = 0_u32;
            while let Some((left, left_prime, _)) = groups.pop_if(|(_, _, merged)| *merged == level)
            {
                derivative = Expr::sum(vec![
                    Mul::mul(left_prime, &product),
                    Mul::mul(&left, derivative),
                ]);
                product = Mul::mul(left, product);
                level = level.saturating_add(1);
            }
            groups.push((product, derivative, level));
        }
        let Some((mut product, mut derivative)) = last else {
            return Expr::number(0);
        };
        while let Some((left, left_prime, _)) = groups.pop() {
            derivative = Expr::sum(vec![
                Mul::mul(left_prime, &product),
                Mul::mul(&left, derivative),
            ]);
            if groups.is_empty() {
                break;
            }
            product = Mul::mul(left, product);
        }
        // A product of one factor is that factor.
        if constants.is_empty() {
            return derivative;
        }
        constants.push(derivative);
        Expr::product(constants)
    }

    /// Apply \((u/v)'=(u'v-uv')/v^2\) on the quotient's regular domain.
    /// A constant denominator uses \(u'/v\), retaining its original pole.
    fn quotient(
        numerator: &Expr,
        denominator: &Expr,
        numerator_prime: Expr,
        denominator_prime: Expr,
    ) -> Expr {
        if numerator_prime.is_exact_zero() && denominator_prime.is_exact_zero() {
            return Expr::number(0);
        }
        if denominator_prime.is_exact_zero() {
            return Div::div(numerator_prime, denominator);
        }
        let negative = Neg::neg(Mul::mul(numerator, denominator_prime));
        let top = if numerator_prime.is_exact_zero() {
            negative
        } else {
            Expr::sum(vec![Mul::mul(numerator_prime, denominator), negative])
        };
        Div::div(top, denominator.pow(2))
    }

    /// Apply \((u^v)'=v u^{v-1}u'+u^v\log(u)v'\) on regular principal branches.
    /// Constant exponents omit the logarithm; exponent one omits the zero power.
    fn power(base: &Expr, exponent: &Expr, base_prime: Expr, exponent_prime: Expr) -> Option<Expr> {
        if base_prime.is_exact_zero() && exponent_prime.is_exact_zero() {
            return Some(Expr::number(0));
        }
        if base.is_zero_num() && !exponent_prime.is_exact_zero() {
            return None;
        }
        let mut terms = Vec::new();
        if !base_prime.is_exact_zero() && !exponent.is_zero_num() {
            terms.push(if exponent.is_exact_one() {
                base_prime
            } else if exponent.is_one_num() {
                // An approximate unit exponent keeps its tier: \(1.0\,u^{0.0}u' = 1.0\,u'\).
                Expr::product(vec![exponent.clone(), base_prime])
            } else {
                Expr::product(vec![
                    exponent.clone(),
                    base.pow(Sub::sub(exponent, 1)),
                    base_prime,
                ])
            });
        }
        if !exponent_prime.is_exact_zero() {
            terms.push(Expr::product(vec![
                base.pow(exponent),
                base.ln(),
                exponent_prime,
            ]));
        }
        Some(Expr::sum(terms))
    }
}
