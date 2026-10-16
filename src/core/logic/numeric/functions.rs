//! Exact function cells on stored integer and Gaussian-integer inputs.
//!
//! These rules reuse the backend's stored integer results where supported, and
//! prove the remaining cells using identities and checked arithmetic.
//! A rounded floating result that happens to be integral is never evidence of
//! exactness. All other calls remain symbolic for explicit evaluation later.

use num_anafis::{Number, Numeric};

use super::{BinaryRule, NumericValue, UnaryRule};

impl NumericValue {
    /// Evaluate only exact integer cells of supported unary functions.
    pub fn evaluate(&self, rule: UnaryRule) -> Option<Number> {
        let (re, im) = self.gaussian()?;
        match rule {
            UnaryRule::Identity => Some(self.0.clone()),
            UnaryRule::Zero => Some(Number::zero()),
            // This exact-function boundary retains its representable-result
            // contract; ordinary arithmetic still follows native backend overflow.
            UnaryRule::Negate if re != i64::MIN && im != i64::MIN => {
                Self::integer_result(Numeric::negate(&self.0))
            }
            UnaryRule::Abs => Self::integer_result(Numeric::abs(&self.0)),
            UnaryRule::Sign => Self::integer_result(Numeric::signum(&self.0)),
            UnaryRule::Sqrt => Self::integer_result(Numeric::sqrt(&self.0)),
            UnaryRule::Cbrt if im == 0 => Self::integer_result(Numeric::cbrt(&self.0)),
            UnaryRule::AtZero(value) if re == 0 && im == 0 => Some(Number::from_int(value)),
            UnaryRule::AtOneZero if re == 1 && im == 0 => Some(Number::zero()),
            UnaryRule::Lgamma if im == 0 => Self::integer_result(Numeric::lgamma(&self.0)),
            UnaryRule::Gamma if im == 0 => Self::integer_result(Numeric::gamma(&self.0)),
            UnaryRule::Log(base) if im == 0 => Self::integer_result(match base {
                2 => Numeric::log2(&self.0),
                10 => Numeric::log10(&self.0),
                _ => Numeric::log(&self.0, &Number::from_int(base)),
            }),
            UnaryRule::Negate
            | UnaryRule::Cbrt
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
                Self::integer_result(Numeric::log(&other.0, &self.0))
            }
            BinaryRule::LogBase if left_im == 0 && right_im == 0 => {
                Self::integer_result(Numeric::log(&self.0, &other.0))
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
            | BinaryRule::LogBase
            | BinaryRule::Atan2
            | BinaryRule::Beta
            | BinaryRule::Bessel
            | BinaryRule::Hermite
            | BinaryRule::LambertW => None,
        }
    }

    fn integer_result(value: Number) -> Option<Number> {
        // Number::is_int also accepts integral floats. Only the stored component
        // variants prove that evaluation retained the exact integer tier.
        let result = Self(value);
        result.gaussian().map(|_| result.0)
    }
}
