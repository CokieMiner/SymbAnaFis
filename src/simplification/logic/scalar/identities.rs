//! The direct identities of the Roots, Exponential, Trigonometric, and
//! Hyperbolic steps, applied to one node at a time: roots of powers and
//! products, principal logarithms of powers, quotients, roots, and
//! exponentials, exact values at multiples of \(\pi/12\), quarter-turn
//! translations, and parity.

use core::{
    iter::once,
    num::NonZeroU64,
    ops::{Add, Div, Mul, Neg, Sub},
};

use alloc::{vec, vec::Vec};

use num_anafis::NonZeroU64Ext;

use crate::core::{
    BuiltinFunction, ExactIdentity, Expr, ExprView, FunctionDefinition, Property, Truth,
};

use super::{Algebraic, Domain, DomainSafety};

/// Namespace for direct root, logarithm, trigonometric angle, and parity identities.
pub struct Identities;

impl Identities {
    /// Roots of exact integer and rational powers, and square roots of
    /// products.
    pub fn root(expression: &Expr, safety: DomainSafety) -> Option<Expr> {
        let ExprView::Function {
            name,
            args: [argument],
        } = expression.view()
        else {
            return None;
        };
        let builtin = BuiltinFunction::of(name)?;
        if !matches!(builtin, BuiltinFunction::Sqrt | BuiltinFunction::Cbrt) {
            return None;
        }
        if builtin == BuiltinFunction::Sqrt
            && let ExprView::Product(factors) = argument.view()
        {
            return Self::extract_squares(factors, safety);
        }
        let ExprView::Pow(base, exponent) = argument.view() else {
            return None;
        };
        let facts = base.properties();
        if !Domain::admits(facts.get(Property::Real), safety) {
            return None;
        }
        let Some((count, 1)) = exponent.exact_ratio().filter(|(count, _)| *count > 0) else {
            if (exponent.as_number().is_none() || exponent.exact_ratio().is_some())
                && Domain::admits(facts.get(Property::Positive), safety)
                && Domain::admits(exponent.properties().get(Property::Real), safety)
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
        let magnitude = || {
            if facts.get(Property::Nonnegative) == Truth::True {
                base.clone()
            } else {
                base.abs()
            }
        };
        if builtin == BuiltinFunction::Sqrt && count % 2 == 0 {
            Some(magnitude().pow(count.checked_div(2)?))
        } else if builtin == BuiltinFunction::Cbrt && count % 3 == 0 {
            Some(base.pow(count.checked_div(3)?))
        } else if builtin == BuiltinFunction::Sqrt && count > 1 {
            Some(Mul::mul(magnitude().pow(count >> 1), base.sqrt()))
        } else if builtin == BuiltinFunction::Cbrt && count > 1 {
            Some(base.cbrt().pow(count))
        } else {
            None
        }
    }

    /// \(\sqrt{b^{2n+r}z}=\lvert b\rvert^{n}\sqrt{b^{r}z}\) for every real
    /// base \(b\) with a stored integer power above one in the product, and
    /// \(\sqrt{\pm(p^2/q^2)z}=(p/q)\sqrt{\pm z}\) for an exact coefficient
    /// whose numerator and denominator are perfect squares; the remainder
    /// must be finite. The extracted factors are nonnegative, so their phase
    /// is zero and the root of the product splits without a sign change.
    fn extract_squares(factors: &[Expr], safety: DomainSafety) -> Option<Expr> {
        let mut outside = Vec::new();
        let mut inside = Vec::new();
        for factor in factors {
            if let ExprView::Pow(base, exponent) = factor.view()
                && let Some((count, 1)) = exponent.exact_ratio()
                && count > 1
                && Domain::admits(base.properties().get(Property::Real), safety)
            {
                let magnitude = if base.properties().get(Property::Nonnegative) == Truth::True {
                    base.clone()
                } else {
                    base.abs()
                };
                outside.push(magnitude.pow(count >> 1));
                if count & 1 == 1 {
                    inside.push(base.clone());
                }
            } else if let Some((numerator, denominator)) = factor.exact_ratio()
                && let Some(top_root) = NonZeroU64::new(numerator.unsigned_abs())
                    .and_then(NonZeroU64Ext::perfect_square)
                && let Some(bottom_root) =
                    NonZeroU64::new(denominator).and_then(NonZeroU64Ext::perfect_square)
                && let (Ok(top), Ok(bottom)) = (
                    i64::try_from(top_root.get()),
                    i64::try_from(bottom_root.get()),
                )
                && (top, bottom) != (1, 1)
            {
                outside.push(Div::div(Expr::number(top), Expr::number(bottom)));
                if numerator < 0 {
                    inside.push(Expr::number(-1));
                }
            } else {
                inside.push(factor.clone());
            }
        }
        if outside.is_empty() {
            return None;
        }
        let remainder = Expr::product(inside);
        if !Domain::admits(remainder.properties().get(Property::Finite), safety) {
            return None;
        }
        outside.push(remainder.sqrt());
        Some(Expr::product(outside))
    }

    /// Use \(\log_{b}(p^r)=r\log_{b}(p)\) for positive \(p\) and real \(r\).
    /// Even integer powers of real inputs retain their absolute value and pole.
    pub fn logarithm(expression: &Expr, safety: DomainSafety) -> Option<Expr> {
        let ExprView::Function { name, args } = expression.view() else {
            return None;
        };
        let builtin = BuiltinFunction::of(name)?;
        let (base, content) = match (builtin, args) {
            (BuiltinFunction::Ln | BuiltinFunction::Log2 | BuiltinFunction::Log10, [content]) => {
                (None, content)
            }
            (BuiltinFunction::Log, [base, content]) => {
                if !Domain::admits(base.properties().get(Property::Positive), safety)
                    || !Domain::admits(
                        Sub::sub(base, 1).properties().get(Property::Nonzero),
                        safety,
                    )
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
                let facts = inner.properties();
                if matches!(
                    exponent.exact_ratio(),
                    Some((count, 1)) if count != 0 && count.checked_rem(2) == Some(0)
                ) && Domain::admits(facts.get(Property::Real), safety)
                {
                    return Some(Mul::mul(exponent, logarithm(inner.abs())));
                }
                (Domain::admits(facts.get(Property::Positive), safety)
                    && Domain::admits(exponent.properties().get(Property::Real), safety))
                .then(|| Mul::mul(exponent, logarithm(inner.clone())))
            }
            ExprView::Div(top, bottom)
                if top.is_one_num()
                    && Domain::admits(bottom.properties().get(Property::Positive), safety) =>
            {
                Some(Neg::neg(logarithm(bottom.clone())))
            }
            ExprView::Function {
                name: inner_name,
                args: inner_args,
            } => {
                let [inner] = inner_args else {
                    return None;
                };
                let facts = inner.properties();
                if *inner_name == BuiltinFunction::Sqrt.function()
                    && facts.get(Property::Finite) == Truth::True
                {
                    Some(Div::div(logarithm(inner.clone()), 2))
                } else if *inner_name == BuiltinFunction::Cbrt.function()
                    && Domain::admits(facts.get(Property::Positive), safety)
                {
                    Some(Div::div(logarithm(inner.clone()), 3))
                } else if *inner_name == BuiltinFunction::Exp.function()
                    && Domain::admits(facts.get(Property::Real), safety)
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

    /// Exact values at multiples of \(\pi/12\) and quarter-turn translations
    /// of the six circular functions. Exact identities apply to symbolic pi,
    /// never approximately nearby floats.
    pub fn angle(expression: &Expr) -> Option<Expr> {
        let ExprView::Function {
            name,
            args: [argument],
        } = expression.view()
        else {
            return None;
        };
        let builtin = BuiltinFunction::of(name)?;
        if !matches!(
            builtin,
            BuiltinFunction::Sin
                | BuiltinFunction::Cos
                | BuiltinFunction::Tan
                | BuiltinFunction::Cot
                | BuiltinFunction::Sec
                | BuiltinFunction::Csc
        ) {
            return None;
        }
        if let Some(phase_index) = Self::phase(argument) {
            return Self::value(builtin, phase_index);
        }
        let ExprView::Sum(children) = argument.view() else {
            return None;
        };
        let terms = children.collect::<Vec<_>>();
        let (index, shift) = terms.iter().enumerate().find_map(|(index, term)| {
            Self::phase(term)
                .filter(|shift| shift.checked_rem(6) == Some(0))
                .map(|shift| (index, shift))
        })?;
        let remainder = Expr::sum(
            terms
                .into_iter()
                .enumerate()
                .filter(|(position, _)| *position != index)
                .map(|(_, term)| term.clone())
                .collect(),
        );
        if remainder.properties().get(Property::Finite) != Truth::True {
            return None;
        }
        let quarter = shift.checked_div(6)?;
        let (destination, negative) = match (builtin, quarter) {
            (_, 0) | (BuiltinFunction::Tan | BuiltinFunction::Cot, 2) => (builtin, false),
            (BuiltinFunction::Sin, 1) => (BuiltinFunction::Cos, false),
            (BuiltinFunction::Sin, 2) | (BuiltinFunction::Cos, 1) => (BuiltinFunction::Sin, true),
            (BuiltinFunction::Sin, 3) | (BuiltinFunction::Cos, 2) => (BuiltinFunction::Cos, true),
            (BuiltinFunction::Cos, 3) => (BuiltinFunction::Sin, false),
            (BuiltinFunction::Tan, 1 | 3) => (BuiltinFunction::Cot, true),
            (BuiltinFunction::Cot, 1 | 3) => (BuiltinFunction::Tan, true),
            (BuiltinFunction::Sec, 1) | (BuiltinFunction::Csc, 2) => (BuiltinFunction::Csc, true),
            (BuiltinFunction::Sec, 2) | (BuiltinFunction::Csc, 3) => (BuiltinFunction::Sec, true),
            (BuiltinFunction::Sec, 3) => (BuiltinFunction::Csc, false),
            (BuiltinFunction::Csc, 1) => (BuiltinFunction::Sec, false),
            _ => return None,
        };
        // Destination identities are selected only from the six unary trig builtins.
        let result = FunctionDefinition::construct(destination.function(), vec![remainder]);
        Some(if negative { Neg::neg(result) } else { result })
    }

    /// The exact value at \(\mathrm{phase}\cdot\pi/12\); poles return `None`.
    fn value(builtin: BuiltinFunction, phase_index: i64) -> Option<Expr> {
        let sine_value = Self::sine(phase_index);
        let cosine = Self::sine(phase_index.saturating_add(6).checked_rem(24)?);
        if builtin == BuiltinFunction::Sin {
            Some(sine_value)
        } else if builtin == BuiltinFunction::Cos {
            Some(cosine)
        } else if builtin == BuiltinFunction::Tan && !cosine.is_zero_num() {
            Some(Div::div(sine_value, cosine))
        } else if builtin == BuiltinFunction::Cot && !sine_value.is_zero_num() {
            Some(Div::div(cosine, sine_value))
        } else if builtin == BuiltinFunction::Sec && !cosine.is_zero_num() {
            Some(Div::div(Expr::number(1), cosine))
        } else if builtin == BuiltinFunction::Csc && !sine_value.is_zero_num() {
            Some(Div::div(Expr::number(1), sine_value))
        } else {
            None
        }
    }

    /// \(\sin(\mathrm{phase}\cdot\pi/12)\) for a phase in \([0,24)\).
    fn sine(mut phase_index: i64) -> Expr {
        let negative = phase_index > 12;
        if negative {
            phase_index = phase_index.saturating_sub(12);
        }
        if phase_index > 6 {
            phase_index = 12_i64.saturating_sub(phase_index);
        }
        let result = match phase_index {
            0 => Expr::number(0),
            1 => Div::div(Sub::sub(Expr::number(6).sqrt(), Expr::number(2).sqrt()), 4),
            2 => Div::div(Expr::number(1), 2),
            3 => Div::div(Expr::number(2).sqrt(), 2),
            4 => Div::div(Expr::number(3).sqrt(), 2),
            5 => Div::div(Add::add(Expr::number(6).sqrt(), Expr::number(2).sqrt()), 4),
            // phase starts in 0..24; the two reflections establish 0..=6.
            _ => Expr::number(1),
        };
        if negative { Neg::neg(result) } else { result }
    }

    /// An exact rational multiple of \(\pi\) whose denominator divides 12, as
    /// the multiple of \(\pi/12\) reduced to \([0,24)\).
    fn phase(expression: &Expr) -> Option<i64> {
        let (numerator, denominator) = if matches!(expression.view(), ExprView::Pi) {
            (1, 1)
        } else if let ExprView::Product(factors) = expression.view()
            && let [coefficient, pi] = factors
            && matches!(pi.view(), ExprView::Pi)
        {
            coefficient.exact_ratio()?
        } else {
            return None;
        };
        if 12_u64.checked_rem(denominator) != Some(0) {
            return None;
        }
        // The accepted denominator divides 12, so period <=24 and reduced*scale <=132.
        let divisor = i64::try_from(denominator).ok()?;
        let period = divisor.checked_mul(2)?;
        let reduced = numerator.rem_euclid(period);
        reduced.checked_mul(12_i64.checked_div(divisor)?)
    }

    /// \(f(-a)=-f(a)\) for the odd and \(f(-a)=f(a)\) for the even circular
    /// and hyperbolic functions, for finite \(a\) with a manifest negative
    /// coefficient.
    pub fn parity(expression: &Expr) -> Option<Expr> {
        let ExprView::Function {
            name,
            args: [argument],
        } = expression.view()
        else {
            return None;
        };
        let builtin = BuiltinFunction::of(name)?;
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
        if !(odd || even) {
            return None;
        }
        let positive = Algebraic::positive_argument(argument)?;
        if argument.properties().get(Property::Finite) != Truth::True {
            return None;
        }
        // The parity tables establish a unary builtin identity before construction.
        let result = FunctionDefinition::construct(builtin.function(), vec![positive]);
        Some(if odd { Neg::neg(result) } else { result })
    }
}
