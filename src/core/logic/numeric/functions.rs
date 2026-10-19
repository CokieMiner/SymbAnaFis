//! Function cells on stored exact real and complex inputs.
//!
//! These rules reuse backend evaluation and prove additional cells using identities
//! and checked arithmetic. The constructor accepts only stored exact components;
//! a rounded floating value is never evidence of exactness.
//! Rational roots retain identities such as \(\sqrt{p^2/q^2}=|p|/q\) for \(q>0\).

use num_anafis::{Number, Numeric, Real};

use super::{BinaryRule, NumericValue, UnaryRule};

impl NumericValue {
    /// Compute unary candidates for exact inputs; construction rejects approximate results.
    ///
    /// # Panics
    /// Numeric evaluation can propagate the backend's rational capacity panics.
    pub fn evaluate(&self, rule: UnaryRule) -> Option<Number> {
        let parts = self.parts()?;
        if !matches!(parts.re, Real::Int(_) | Real::Rational(_))
            || !matches!(parts.im, Real::Int(_) | Real::Rational(_))
        {
            return None;
        }
        match rule {
            UnaryRule::Floor => Some(Numeric::floor(&self.0)),
            UnaryRule::Ceil => Some(Numeric::ceil(&self.0)),
            UnaryRule::Round => Some(Numeric::round(&self.0)),
            UnaryRule::Trunc => Some(Numeric::trunc(&self.0)),
            UnaryRule::Fract => Some(Numeric::fract(&self.0)),
            UnaryRule::Abs => Some(Numeric::abs(&self.0)),
            UnaryRule::Sign => Some(Numeric::signum(&self.0)),
            UnaryRule::Sqrt => Some(Numeric::sqrt(&self.0)),
            UnaryRule::Cbrt if parts.im.is_zero() => Some(Numeric::cbrt(&self.0)),
            UnaryRule::AtZero(value) if self.0.is_zero() => Some(Number::from_int(value)),
            UnaryRule::AtOneZero if self.0.is_one() => Some(Number::zero()),
            UnaryRule::Lgamma if parts.im.is_zero() => Some(Numeric::lgamma(&self.0)),
            UnaryRule::Gamma if parts.im.is_zero() => Some(Numeric::gamma(&self.0)),
            UnaryRule::Log(base) if parts.im.is_zero() => Some(match base {
                2 => Numeric::log2(&self.0),
                10 => Numeric::log10(&self.0),
                _ => Numeric::log(&self.0, &Number::from_int(base)),
            }),
            UnaryRule::Cbrt
            | UnaryRule::AtZero(_)
            | UnaryRule::AtOneZero
            | UnaryRule::Lgamma
            | UnaryRule::Gamma
            | UnaryRule::Log(_) => None,
        }
    }

    /// Evaluate exact integer cells of supported two-argument functions.
    pub fn evaluate_binary(&self, rule: BinaryRule, other: &Self) -> Option<Number> {
        let (left, left_im) = self.gaussian()?;
        let (right, right_im) = other.gaussian()?;
        match rule {
            // The symbolic log constructor stores (base, argument).
            BinaryRule::Log if left_im == 0 && right_im == 0 => {
                Some(Numeric::log(&other.0, &self.0))
            }
            BinaryRule::Atan2 if left == 0 && left_im == 0 && right > 0 && right_im == 0 => {
                Some(Number::zero())
            }
            BinaryRule::Beta if (left, left_im, right, right_im) == (1, 0, 1, 0) => {
                Some(Number::one())
            }
            // W_0(0) = 0; other branches at zero do not share this exact cell.
            BinaryRule::LambertW if (left, left_im, right, right_im) == (0, 0, 0, 0) => {
                Some(Number::zero())
            }
            BinaryRule::Bessel if left_im == 0 && right == 0 && right_im == 0 => {
                Some(if left == 0 {
                    Number::one()
                } else {
                    Number::zero()
                })
            }
            BinaryRule::Hermite if left_im == 0 && left == 0 => Some(Number::one()),
            BinaryRule::Hermite if left_im == 0 && left == 1 => Some(Number::from_complex(
                Number::from_int(right.checked_mul(2)?),
                Number::from_int(right_im.checked_mul(2)?),
            )),
            BinaryRule::Hermite
                if left_im == 0 && left > 0 && left & 1 != 0 && right == 0 && right_im == 0 =>
            {
                Some(Number::zero())
            }
            BinaryRule::Log
            | BinaryRule::Atan2
            | BinaryRule::Beta
            | BinaryRule::Bessel
            | BinaryRule::Hermite
            | BinaryRule::LambertW => None,
        }
    }
}
