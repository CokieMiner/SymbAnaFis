use std::f64;
use std::ops::{Add, Div, Mul, Sub};

use super::scalar::Number;

/// Closed numeric interval [lo, hi].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Interval {
    /// The lower bound of the interval.
    pub lo: Number,
    /// The upper bound of the interval.
    pub hi: Number,
}

impl Interval {
    /// Create a normalized interval (swaps endpoints when needed).
    #[must_use]
    pub fn new(a: Number, b: Number) -> Self {
        if a <= b {
            Self { lo: a, hi: b }
        } else {
            Self { lo: b, hi: a }
        }
    }

    /// Width `hi - lo`.
    #[must_use]
    pub fn width(&self) -> Number {
        &self.hi - &self.lo
    }

    /// Midpoint `(lo + hi)/2`.
    ///
    /// # Note
    /// Division by 2 for `Int` results in `Rational`. The denominator is always 2,
    /// so this is safe from division-by-zero panics.
    #[must_use]
    pub fn midpoint(&self) -> Number {
        (&self.lo + &self.hi) / Number::from(2_i64)
    }

    /// Membership test.
    #[must_use]
    pub fn contains(&self, value: &Number) -> bool {
        value >= &self.lo && value <= &self.hi
    }

    /// Interval addition.
    #[must_use]
    pub fn add_interval(&self, other: &Self) -> Self {
        Self::new(&self.lo + &other.lo, &self.hi + &other.hi)
    }

    /// Interval subtraction.
    #[must_use]
    pub fn sub_interval(&self, other: &Self) -> Self {
        Self::new(&self.lo - &other.hi, &self.hi - &other.lo)
    }

    /// Interval multiplication with endpoint enclosure.
    #[must_use]
    pub fn mul_interval(&self, other: &Self) -> Self {
        let p1 = &self.lo * &other.lo;
        let p2 = &self.lo * &other.hi;
        let p3 = &self.hi * &other.lo;
        let p4 = &self.hi * &other.hi;

        let mut lo = p1.clone();
        let mut hi = p1;

        for p in [p2, p3, p4] {
            if p < lo {
                lo = p.clone();
            }
            if p > hi {
                hi = p;
            }
        }

        Self { lo, hi }
    }

    /// Interval division.
    ///
    /// # Note
    /// This is a basic implementation of interval division. If the divisor interval
    /// contains zero, the result will have `(-∞, +∞)` endpoints as the standard
    /// interval-arithmetic convention.
    #[must_use]
    pub fn div_interval(&self, other: &Self) -> Self {
        if other.contains(&Number::from(0_i64)) {
            return Self {
                lo: Number::float(f64::NEG_INFINITY),
                hi: Number::float(f64::INFINITY),
            };
        }

        let one = Number::from(1_i64);
        let inv_other = Self::new(&one / &other.hi, &one / &other.lo);
        self.mul_interval(&inv_other)
    }
}

impl Add for Interval {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::add_interval(&self, &rhs)
    }
}

impl Add<&Self> for Interval {
    type Output = Self;

    fn add(self, rhs: &Self) -> Self::Output {
        Self::add_interval(&self, rhs)
    }
}

impl Add<Interval> for &Interval {
    type Output = Interval;

    fn add(self, rhs: Interval) -> Self::Output {
        Interval::add_interval(self, &rhs)
    }
}

impl Add<&Interval> for &Interval {
    type Output = Interval;

    fn add(self, rhs: &Interval) -> Self::Output {
        Interval::add_interval(self, rhs)
    }
}

impl Sub for Interval {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::sub_interval(&self, &rhs)
    }
}

impl Sub<&Self> for Interval {
    type Output = Self;

    fn sub(self, rhs: &Self) -> Self::Output {
        Self::sub_interval(&self, rhs)
    }
}

impl Sub<Interval> for &Interval {
    type Output = Interval;

    fn sub(self, rhs: Interval) -> Self::Output {
        Interval::sub_interval(self, &rhs)
    }
}

impl Sub<&Interval> for &Interval {
    type Output = Interval;

    fn sub(self, rhs: &Interval) -> Self::Output {
        Interval::sub_interval(self, rhs)
    }
}

impl Mul for Interval {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::mul_interval(&self, &rhs)
    }
}

impl Mul<&Self> for Interval {
    type Output = Self;

    fn mul(self, rhs: &Self) -> Self::Output {
        Self::mul_interval(&self, rhs)
    }
}

impl Mul<Interval> for &Interval {
    type Output = Interval;

    fn mul(self, rhs: Interval) -> Self::Output {
        Interval::mul_interval(self, &rhs)
    }
}

impl Mul<&Interval> for &Interval {
    type Output = Interval;

    fn mul(self, rhs: &Interval) -> Self::Output {
        Interval::mul_interval(self, rhs)
    }
}

impl Div for Interval {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::div_interval(&self, &rhs)
    }
}

impl Div<&Self> for Interval {
    type Output = Self;

    fn div(self, rhs: &Self) -> Self::Output {
        Self::div_interval(&self, rhs)
    }
}

impl Div<Interval> for &Interval {
    type Output = Interval;

    fn div(self, rhs: Interval) -> Self::Output {
        Interval::div_interval(self, &rhs)
    }
}

impl Div<&Interval> for &Interval {
    type Output = Interval;

    fn div(self, rhs: &Interval) -> Self::Output {
        Interval::div_interval(self, rhs)
    }
}
