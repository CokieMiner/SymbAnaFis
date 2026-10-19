//! Closure and sign proofs for scalar arithmetic.

use core::iter::once;

use num_anafis::{Number, Real};

use super::{Facts, Property, Truth};

impl Facts {
    pub fn sum(terms: impl Iterator<Item = Self>) -> Self {
        let mut all = u16::MAX;
        let mut some = 0;
        let mut facts = Self::default();
        for value in terms {
            if value.get(Property::Finite) == Truth::False {
                return Self::default();
            }
            all &= value.proven;
            some |= value.proven;
        }
        for property in [
            Property::Finite,
            Property::Real,
            Property::Integer,
            Property::Rational,
            Property::Zero,
        ] {
            if all & property.bit() != 0 {
                facts.prove(property);
            }
        }
        for (weak, strict) in [
            (Property::Nonnegative, Property::Positive),
            (Property::Nonpositive, Property::Negative),
        ] {
            if all & weak.bit() != 0 {
                facts.prove(weak);
                if some & strict.bit() != 0 {
                    facts.prove(strict);
                }
            }
        }
        facts
    }

    pub fn quotient(numerator: Self, denominator: Self) -> Self {
        let mut facts = Self::default();
        if denominator.get(Property::Zero) == Truth::True {
            facts.refute(Property::Finite);
            return facts;
        }
        if denominator.get(Property::Nonzero) != Truth::True {
            return facts;
        }
        facts = Self::product(once(numerator).chain(once(denominator)));
        // Integral quotients need divisibility, not just integral operands.
        facts.proven &= !Property::Integer.bit();
        if numerator.get(Property::Zero) == Truth::True
            && facts.get(Property::Finite) == Truth::True
        {
            facts.prove(Property::Integer);
        }
        facts
    }

    pub fn product(factors: impl Iterator<Item = Self>) -> Self {
        let mut all = u16::MAX;
        let mut some = 0;
        let mut negative = false;
        let mut signed = true;
        let mut facts = Self::default();
        for value in factors {
            if value.get(Property::Finite) == Truth::False {
                return Self::default();
            }
            all &= value.proven;
            some |= value.proven;
            if value.get(Property::Nonnegative) != Truth::True {
                if value.get(Property::Nonpositive) == Truth::True {
                    negative = !negative;
                } else {
                    signed = false;
                }
            }
        }
        if all & Property::Finite.bit() == 0 {
            return facts;
        }
        facts.prove(Property::Finite);
        if some & Property::Zero.bit() != 0 {
            facts.prove(Property::Zero);
            return facts;
        }
        for property in [
            Property::Real,
            Property::Integer,
            Property::Rational,
            Property::Nonzero,
        ] {
            if all & property.bit() != 0 {
                facts.prove(property);
            }
        }
        if !signed {
            return facts;
        }
        facts.prove(if negative {
            Property::Nonpositive
        } else {
            Property::Nonnegative
        });
        if facts.get(Property::Nonzero) == Truth::True {
            facts.prove(if negative {
                Property::Negative
            } else {
                Property::Positive
            });
        }
        facts
    }

    pub fn power(base: Self, exponent: Self, literal: Option<&Number>) -> Self {
        if let Some(value) = literal
            && let Ok(parts) = value.extract_complex()
            && parts.im.is_zero()
            && let Real::Int(count) = parts.re
        {
            return Self::integer_power(base, count);
        }
        let mut facts = Self::default();
        if base.get(Property::Finite) != Truth::True
            || exponent.get(Property::Finite) != Truth::True
        {
            return facts;
        }
        if base.get(Property::Zero) == Truth::True {
            if exponent.get(Property::Positive) == Truth::True {
                facts.prove(Property::Zero);
            } else if exponent.get(Property::Nonpositive) == Truth::True {
                facts.refute(Property::Finite);
            }
            return facts;
        }
        if base.get(Property::Nonzero) != Truth::True
            && exponent.get(Property::Positive) != Truth::True
        {
            return facts;
        }
        facts.prove(Property::Finite);
        if base.get(Property::Nonzero) == Truth::True {
            facts.prove(Property::Nonzero);
        }
        if base.get(Property::Positive) == Truth::True
            && exponent.get(Property::Real) == Truth::True
        {
            facts.prove(Property::Positive);
        }
        facts
    }

    pub fn integer_power(base: Self, exponent: i64) -> Self {
        let mut facts = Self::default();
        if exponent <= 0 && base.get(Property::Zero) == Truth::True {
            facts.refute(Property::Finite);
            return facts;
        }
        if base.get(Property::Finite) != Truth::True
            || exponent <= 0 && base.get(Property::Nonzero) != Truth::True
        {
            return facts;
        }
        if exponent == 0 {
            return Self::literal(&Number::one());
        }
        facts.prove(Property::Finite);
        if base.get(Property::Nonzero) == Truth::True {
            facts.prove(Property::Nonzero);
        }
        if exponent > 0 {
            for property in [Property::Integer, Property::Rational, Property::Zero] {
                if base.get(property) == Truth::True {
                    facts.prove(property);
                }
            }
        } else if base.get(Property::Rational) == Truth::True {
            facts.prove(Property::Rational);
        }
        if base.get(Property::Real) == Truth::True {
            facts.prove(Property::Real);
            if exponent & 1 == 0 {
                facts.prove(Property::Nonnegative);
                if base.get(Property::Nonzero) == Truth::True {
                    facts.prove(Property::Positive);
                }
            } else {
                for property in [
                    Property::Positive,
                    Property::Negative,
                    Property::Nonnegative,
                    Property::Nonpositive,
                ] {
                    if base.get(property) == Truth::True {
                        facts.prove(property);
                    }
                }
            }
        }
        facts
    }
}
