//! Principal logarithm identities with explicit realness and base conditions.

use core::{
    iter::once,
    ops::{Div, Mul, Neg, Sub},
};

use alloc::vec::Vec;

use crate::core::{BuiltinFunction, Expr, ExprView, FunctionDefinition, Property};

use super::{DomainSafety, PreparedStep};

impl PreparedStep<'_> {
    /// Use \(\log_{b}(p^r)=r\log_{b}(p)\) for positive \(p\) and real \(r\).
    /// Even integer powers of real inputs retain their absolute value and pole.
    pub fn logarithm(
        builtin: BuiltinFunction,
        args: &[Expr],
        safety: DomainSafety,
    ) -> Option<Expr> {
        let (base, content) = match (builtin, args) {
            (BuiltinFunction::Ln | BuiltinFunction::Log2 | BuiltinFunction::Log10, [content]) => {
                (None, content)
            }
            (BuiltinFunction::Log, [base, content]) => {
                if !Self::permitted(base, Property::Positive, safety)
                    || !Self::permitted(&Sub::sub(base, 1), Property::Nonzero, safety)
                {
                    return None;
                }
                (Some(base), content)
            }
            _ => return None,
        };
        let logarithm = |argument: Expr| {
            let arguments: Vec<_> = base
                .iter()
                .copied()
                .cloned()
                .chain(once(argument))
                .collect();
            // Dispatch established the builtin identity and its complete argument shape.
            FunctionDefinition::construct(builtin.function(), arguments)
        };
        if content.is_one_num() {
            return Some(Expr::number(0));
        }
        if base == Some(content) {
            return Some(Expr::number(1));
        }
        match content.view() {
            ExprView::Pow(inner, exponent) => {
                if Self::exact_integer(exponent)
                    .is_some_and(|count| count != 0 && count.checked_rem(2) == Some(0))
                    && Self::permitted(inner, Property::Real, safety)
                {
                    return Some(Mul::mul(exponent, logarithm(inner.abs())));
                }
                (Self::permitted(inner, Property::Positive, safety)
                    && Self::permitted(exponent, Property::Real, safety))
                .then(|| Mul::mul(exponent, logarithm(inner.clone())))
            }
            ExprView::Div(top, bottom)
                if top.is_one_num() && Self::permitted(bottom, Property::Positive, safety) =>
            {
                Some(Neg::neg(logarithm(bottom.clone())))
            }
            ExprView::Function {
                name,
                args: inner_args,
            } => {
                let [inner] = inner_args else {
                    return None;
                };
                if *name == BuiltinFunction::Sqrt.function()
                    && Self::proved(inner, Property::Finite)
                {
                    Some(Div::div(logarithm(inner.clone()), 2))
                } else if *name == BuiltinFunction::Cbrt.function()
                    && Self::permitted(inner, Property::Positive, safety)
                {
                    Some(Div::div(logarithm(inner.clone()), 3))
                } else if *name == BuiltinFunction::Exp.function()
                    && Self::permitted(inner, Property::Real, safety)
                {
                    Some(Mul::mul(inner, logarithm(Expr::e())))
                } else {
                    None
                }
            }
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Sum(_)
            | ExprView::Product(_)
            | ExprView::Div(..)
            | ExprView::Derivative { .. } => None,
        }
    }
}
