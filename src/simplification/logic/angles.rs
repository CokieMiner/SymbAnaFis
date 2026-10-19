//! Exact rational multiples of pi and quarter-turn translations.

use core::ops::{Add, Div, Neg, Sub};

use alloc::{vec, vec::Vec};

use crate::core::{BuiltinFunction, Expr, ExprView, FunctionDefinition, Property};

use super::PreparedStep;

impl PreparedStep<'_> {
    /// Exact identities apply to symbolic pi, never approximately nearby floats.
    pub fn angle(builtin: BuiltinFunction, argument: &Expr) -> Option<Expr> {
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
        if let Some(phase) = Self::phase(argument) {
            return Self::value(builtin, phase);
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
                .filter_map(|(position, term)| (position != index).then_some(term))
                .collect(),
        );
        if !Self::proved(&remainder, Property::Finite) {
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

    fn value(builtin: BuiltinFunction, phase: i64) -> Option<Expr> {
        let sine_value = Self::sine(phase);
        let cosine = Self::sine(phase.saturating_add(6).checked_rem(24)?);
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

    fn sine(mut phase: i64) -> Expr {
        let negative = phase > 12;
        if negative {
            phase = phase.saturating_sub(12);
        }
        if phase > 6 {
            phase = 12_i64.saturating_sub(phase);
        }
        let result = match phase {
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

    fn phase(expression: &Expr) -> Option<i64> {
        let (numerator, denominator) = if matches!(expression.view(), ExprView::Pi) {
            (1, 1)
        } else if let ExprView::Product(factors) = expression.view()
            && let [coefficient, pi] = factors
            && matches!(pi.view(), ExprView::Pi)
        {
            Self::exact_ratio(coefficient)?
        } else {
            return None;
        };
        if denominator == 0 || 12_u64.checked_rem(denominator) != Some(0) {
            return None;
        }
        // The accepted denominator divides 12, so period <=24 and reduced*scale <=132.
        let divisor = i64::try_from(denominator).ok()?;
        let period = divisor.checked_mul(2)?;
        let reduced = numerator.rem_euclid(period);
        reduced.checked_mul(12_i64.checked_div(divisor)?)
    }
}
