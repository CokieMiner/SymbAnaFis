//! Direct scalar identities and explicit negative-argument normalization.

use core::{
    iter::once,
    ops::{Div, Mul, Neg},
};

use alloc::vec;

use num_anafis::Number;

use crate::core::{BuiltinFunction, Expr, ExprView, FunctionDefinition, Property};

use super::{DomainSafety, PreparedStep, SimplificationStep};

impl PreparedStep<'_> {
    pub fn identities(
        expression: &Expr,
        step: &SimplificationStep,
        safety: DomainSafety,
    ) -> Option<Expr> {
        if matches!(step, SimplificationStep::Compilation) {
            // \(x^{1/3}=\operatorname{cbrt}(x)\) for positive x. The exact
            // exponent must be stored; inferred power groups introduce new roots.
            if let ExprView::Pow(base, exponent) = expression.view()
                && Self::exact_ratio(exponent) == Some((1, 3))
                && Self::proved(base, Property::Positive)
            {
                return Some(base.cbrt());
            }
            return None;
        }
        if matches!(step, SimplificationStep::Algebraic)
            && let Some(result) = Self::cancel(expression, safety)
                .or_else(|| Self::combine_powers(expression, safety))
                .or_else(|| Self::combine_fractions(expression))
        {
            return Some(result);
        }
        let ExprView::Function { name, args } = expression.view() else {
            return None;
        };
        let builtin = FunctionDefinition::get(name)?.builtin();
        if matches!(step, SimplificationStep::Exponential) {
            return Self::logarithm(builtin, args, safety);
        }
        let [argument] = args else {
            return None;
        };
        match step {
            SimplificationStep::Trigonometric => {
                Self::angle(builtin, argument).or_else(|| Self::parity(builtin, argument))
            }
            SimplificationStep::Hyperbolic => Self::parity(builtin, argument),
            SimplificationStep::Roots => Self::root(builtin, argument, safety),
            SimplificationStep::Algebraic => {
                if builtin == BuiltinFunction::Abs && Self::proved(argument, Property::Finite) {
                    if Self::proved(argument, Property::Nonnegative) {
                        return Some(argument.clone());
                    }
                    if Self::proved(argument, Property::Nonpositive) {
                        return Some(Neg::neg(argument));
                    }
                    if let Some(positive) = Self::positive_argument(argument) {
                        return Some(positive.abs());
                    }
                    if let ExprView::Pow(base, exponent) = argument.view()
                        && Self::exact_integer(exponent).is_some_and(|count| count > 0)
                        && Self::proved(base, Property::Finite)
                    {
                        return Some(base.abs().pow(exponent));
                    }
                }
                None
            }
            SimplificationStep::Exponential
            | SimplificationStep::Expand
            | SimplificationStep::Factor
            | SimplificationStep::Compilation
            | SimplificationStep::Rewrite(_)
            | SimplificationStep::Map(_) => None,
        }
    }

    fn parity(builtin: BuiltinFunction, argument: &Expr) -> Option<Expr> {
        let odd = matches!(
            builtin,
            BuiltinFunction::Sin
                | BuiltinFunction::Tan
                | BuiltinFunction::Cot
                | BuiltinFunction::Csc
                | BuiltinFunction::Sinh
                | BuiltinFunction::Tanh
                | BuiltinFunction::Coth
                | BuiltinFunction::Csch
        );
        let even = matches!(
            builtin,
            BuiltinFunction::Cos
                | BuiltinFunction::Sec
                | BuiltinFunction::Cosh
                | BuiltinFunction::Sech
        );
        if !(odd || even) || !Self::proved(argument, Property::Finite) {
            return None;
        }
        let positive = Self::positive_argument(argument)?;
        // The parity tables establish a unary builtin identity before construction.
        let result = FunctionDefinition::construct(builtin.function(), vec![positive]);
        Some(if odd { Neg::neg(result) } else { result })
    }

    fn root(builtin: BuiltinFunction, argument: &Expr, safety: DomainSafety) -> Option<Expr> {
        if !matches!(builtin, BuiltinFunction::Sqrt | BuiltinFunction::Cbrt) {
            return None;
        }
        let ExprView::Pow(base, exponent) = argument.view() else {
            return None;
        };
        if !Self::permitted(base, Property::Real, safety) {
            return None;
        }
        let Some(count) = Self::exact_integer(exponent).filter(|count| *count > 0) else {
            if Self::permitted(base, Property::Positive, safety)
                && Self::permitted(exponent, Property::Real, safety)
                && (exponent.as_number().is_none() || Self::exact_ratio(exponent).is_some())
            {
                let degree = if builtin == BuiltinFunction::Sqrt {
                    2
                } else {
                    3
                };
                return Some(base.pow(Div::div(exponent, degree)));
            }
            return None;
        };
        if builtin == BuiltinFunction::Sqrt && count % 2 == 0 {
            Some(base.abs().pow(count.checked_div(2)?))
        } else if builtin == BuiltinFunction::Cbrt && count % 3 == 0 {
            Some(base.pow(count.checked_div(3)?))
        } else if builtin == BuiltinFunction::Sqrt && count > 1 {
            Some(Mul::mul(base.abs().pow(count >> 1), base.sqrt()))
        } else if builtin == BuiltinFunction::Cbrt && count > 1 {
            Some(base.cbrt().pow(count))
        } else {
            None
        }
    }

    fn positive_argument(argument: &Expr) -> Option<Expr> {
        let ExprView::Product(factors) = argument.view() else {
            return None;
        };
        let first = factors.first()?;
        if !first.as_number().is_some_and(Number::is_negative) {
            return None;
        }
        let positive = Neg::neg(first);
        // Native minimum-integer negation wraps; it cannot establish a positive prefix.
        if !positive.as_number().is_some_and(Number::is_positive) {
            return None;
        }
        Some(Expr::product(
            once(positive)
                .chain(factors.iter().skip(1).cloned())
                .collect(),
        ))
    }
}
