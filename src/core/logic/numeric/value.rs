//! Numeric literals with representation identity and the folding policy.
//!
//! A fold succeeds only for a finite result. Exact results that exceed the
//! numeric backend's capacity, and approximate products or quotients that
//! underflow to zero, keep the operation symbolic. Exact function cells keep a
//! result only when its components are exact, so a rounded float is never taken
//! as exact; rational roots keep identities such as
//! \(\sqrt{p^2/q^2} = |p|/q\) for \(q > 0\).

use core::{
    cmp::Ordering,
    fmt::{Display, Formatter, Result as FmtResult, Write},
    hash::{Hash, Hasher},
};

use num_anafis::{Number, Numeric, Real};

use super::{BinaryRule, BuiltinFunction, Kernel, Scalar, UnaryRule};

static IMAGINARY_UNIT: NumericValue = NumericValue(Number::i());

/// A `NumAnaFis` literal whose identity is its representation.
///
/// Equality and hashing follow `Number::representation_cmp`, so `2` and `2.0`
/// are distinct literals, as are `0.0` and `-0.0`.
#[derive(Clone, Debug)]
pub struct NumericValue(Number);

/// Forwards rendered text to a writer without its leading minus sign.
struct Magnitude<'run, Output: Write + ?Sized> {
    formatter: &'run mut Output,
    started: bool,
}

impl NumericValue {
    /// Wrap a value, keeping its representation.
    pub const fn new(value: Number) -> Self {
        Self(value)
    }

    /// Borrow the stored value.
    pub const fn value(&self) -> &Number {
        &self.0
    }

    /// The exact imaginary unit stored by the dedicated DAG variant.
    pub const fn imaginary_unit() -> &'static Self {
        &IMAGINARY_UNIT
    }

    /// Read an exact Gaussian integer `(re, im)`. Integral floats are not exact.
    pub fn gaussian(&self) -> Option<(i64, i64)> {
        match self.0.components() {
            (Real::Int(re), Real::Int(im)) => Some((*re, *im)),
            _ => None,
        }
    }

    /// Whether any component is approximate.
    pub fn has_float(&self) -> bool {
        !self.0.is_exact()
    }

    /// Total order of representations: tier (integer, rational, float) and
    /// then value, component by component.
    pub fn compare(&self, other: &Self) -> Ordering {
        self.0.representation_cmp(&other.0)
    }

    /// Fold a sum.
    pub fn add(&self, other: &Self) -> Option<Number> {
        self.0.checked_add(&other.0).filter(Number::is_finite)
    }

    /// Fold a product, keeping approximate underflow symbolic.
    pub fn mul(&self, other: &Self) -> Option<Number> {
        let value = self.0.checked_mul(&other.0)?;
        if value.is_zero()
            && !self.0.is_zero()
            && !other.0.is_zero()
            && (self.has_float() || other.has_float())
        {
            return None;
        }
        value.is_finite().then_some(value)
    }

    /// Fold a quotient; a zero divisor stays symbolic.
    pub fn div(&self, other: &Self) -> Option<Number> {
        if other.0.is_zero() {
            return None;
        }
        let value = self.0.checked_div(&other.0)?;
        if value.is_zero() && !self.0.is_zero() && (self.has_float() || other.has_float()) {
            return None;
        }
        value.is_finite().then_some(value)
    }

    /// Fold a power with an exact integer exponent, or any power with an
    /// approximate base. Exact bases with other exponents stay symbolic, and
    /// zero to a nonpositive power stays undefined.
    pub fn pow(&self, exponent: &Self) -> Option<Number> {
        if self.0.is_zero() && !exponent.0.is_positive() {
            return None;
        }
        if !self.has_float() && exponent.gaussian().is_none_or(|(_, im)| im != 0) {
            return None;
        }
        let value = self.0.checked_pow(&exponent.0)?;
        if value.is_zero() && !self.0.is_zero() && self.has_float() {
            return None;
        }
        value.is_finite().then_some(value)
    }

    /// Divide by \(d^n\) for `divisor` \(d\) and `count` \(n > 0\), one factor at
    /// a time, and return the quotient with the exponent left undivided.
    ///
    /// The units \(1\), \(-1\), and \(\pm i\) first reduce \(n\) modulo their
    /// order. Exact operands divide completely or not at all: when an exact
    /// quotient exceeds the backend, the result is the dividend with \(n\)
    /// undivided. An approximate operand absorbs one factor and leaves
    /// \(n - 1\). Callers guarantee finite operands and a nonzero divisor.
    pub fn divide_power(&self, divisor: &Self, count: i64) -> (Number, i64) {
        let remaining = match divisor.gaussian() {
            Some((1, 0)) => 0,
            Some((-1, 0)) => count.rem_euclid(2),
            Some((0, 1 | -1)) => count.rem_euclid(4),
            _ => count,
        };
        if remaining == 0 {
            return (self.0.clone(), 0);
        }
        if self.has_float() || divisor.has_float() {
            return self.div(divisor).map_or_else(
                || (self.0.clone(), remaining),
                |value| (value, remaining.saturating_sub(1)),
            );
        }
        // A non-unit divisor has a nonzero exponent at some Gaussian prime, so
        // each division moves the quotient's exponent there by at least one.
        // Native components confine that exponent to fewer than 256 values.
        if remaining > i64::from(u64::BITS.saturating_mul(4)) {
            return (self.0.clone(), remaining);
        }
        let mut quotient = self.clone();
        for _ in 0..remaining {
            let Some(value) = quotient.div(divisor) else {
                return (self.0.clone(), remaining);
            };
            quotient = Self::new(value);
        }
        (quotient.0, 0)
    }

    /// Candidate value of a unary cell at an exact argument. The caller rejects
    /// a candidate with an approximate component.
    pub fn evaluate(&self, rule: UnaryRule) -> Option<Number> {
        if self.has_float() {
            return None;
        }
        let (_, imaginary) = self.0.components();
        match rule {
            UnaryRule::Floor => Some(Numeric::floor(&self.0)),
            UnaryRule::Ceil => Some(Numeric::ceil(&self.0)),
            UnaryRule::Round => Some(Numeric::round(&self.0)),
            UnaryRule::Trunc => Some(Numeric::trunc(&self.0)),
            UnaryRule::Fract => Some(Numeric::fract(&self.0)),
            UnaryRule::Abs => Some(Numeric::abs(&self.0)),
            UnaryRule::Sign => Some(Numeric::signum(&self.0)),
            UnaryRule::Sqrt => Some(Numeric::sqrt(&self.0)),
            UnaryRule::Cbrt if imaginary.is_zero() => Some(Numeric::cbrt(&self.0)),
            UnaryRule::AtZero(value) if self.0.is_zero() => Some(Number::from_int(value)),
            UnaryRule::AtOneZero if self.0.is_one() => Some(Number::zero()),
            UnaryRule::Lgamma if imaginary.is_zero() => Some(Numeric::lgamma(&self.0)),
            UnaryRule::Gamma if imaginary.is_zero() => Some(Numeric::gamma(&self.0)),
            UnaryRule::Log(base) if imaginary.is_zero() => Some(match base {
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

    /// Value of a binary cell at exact Gaussian-integer arguments.
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

    /// Evaluate finite literal arguments when at least one component is
    /// approximate, through the shared numerical kernels. A nonfinite result
    /// keeps the symbolic call.
    pub fn evaluate_approximate(builtin: BuiltinFunction, arguments: &[&Self]) -> Option<Number> {
        if !arguments.iter().any(|value| value.has_float())
            || !arguments.iter().all(|value| value.0.is_finite())
        {
            return None;
        }
        let value = match (Number::kernel(builtin), arguments) {
            (Kernel::Unary(kernel), [value]) => kernel(&value.0),
            (Kernel::Binary(kernel), [left, right]) => kernel(&left.0, &right.0),
            (Kernel::Ternary(kernel), [first, second, third]) => {
                kernel(&first.0, &second.0, &third.0)
            }
            _ => return None,
        };
        value.is_finite().then_some(value)
    }

    /// Render a negative real coefficient's magnitude without native negation,
    /// which would overflow a fixed-width minimum component.
    pub fn format_magnitude(&self, output: &mut (impl Write + ?Sized)) -> FmtResult {
        write!(
            Magnitude {
                formatter: output,
                started: false
            },
            "{self}"
        )
    }
}

impl PartialEq for NumericValue {
    fn eq(&self, other: &Self) -> bool {
        self.compare(other).is_eq()
    }
}

impl Eq for NumericValue {}

impl Hash for NumericValue {
    fn hash<HasherType: Hasher>(&self, state: &mut HasherType) {
        self.0.representation_hash(state);
    }
}

impl Display for NumericValue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        if !self.0.is_complex() {
            return Display::fmt(&self.0, formatter);
        }
        let (re, im) = self.0.components();
        if re.is_zero() {
            if im.is_one() {
                formatter.write_str("i")
            } else if im.is_neg_one() {
                formatter.write_str("-i")
            } else {
                write!(formatter, "{im}*i")
            }
        } else {
            write!(formatter, "{re}")?;
            if im.is_negative() {
                formatter.write_str(" - ")?;
                if !im.is_neg_one() {
                    write!(
                        Magnitude {
                            formatter: &mut *formatter,
                            started: false
                        },
                        "{im}"
                    )?;
                    formatter.write_str("*")?;
                }
            } else {
                formatter.write_str(" + ")?;
                if !im.is_one() {
                    write!(formatter, "{im}*")?;
                }
            }
            formatter.write_str("i")
        }
    }
}

impl<Output: Write + ?Sized> Write for Magnitude<'_, Output> {
    fn write_str(&mut self, text: &str) -> FmtResult {
        if self.started {
            return self.formatter.write_str(text);
        }
        self.started = !text.is_empty();
        self.formatter
            .write_str(text.strip_prefix('-').unwrap_or(text))
    }
}
