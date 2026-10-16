//! Capacity proofs for coefficient division without oversized power intermediates.

use num_anafis::{Complex, Number, Real};

use super::NumericValue;

impl NumericValue {
    /// Divide by repeated scalar factors while the exact backend has capacity.
    /// Return the remaining exponent for approximate, compressed reconstruction.
    /// Callers prove finite operands, a nonzero divisor, and a positive count.
    pub fn divide_power(&self, divisor: &Self, count: i64) -> (Number, i64) {
        let mut remaining = match divisor.gaussian() {
            Some((1, 0)) => 0,
            Some((-1, 0)) => count.rem_euclid(2),
            Some((0, 1 | -1)) => count.rem_euclid(4),
            _ => count,
        };
        if remaining == 0 {
            return (self.0.clone(), 0);
        }
        let mut current = self.clone();
        let divisor_parts = divisor.parts();
        let approximate = self.has_float()
            || divisor_parts
                .as_ref()
                .is_some_and(|parts| parts.re.is_float() || parts.im.is_float());
        // A native scalar has four components of at most 64 bits across its
        // numerator/denominator magnitudes. Capacity checks stop growing ratios;
        // this bound also keeps evaluation independent of a symbolic exponent.
        for _ in 0..u64::BITS.saturating_mul(4) {
            if remaining == 0
                || !(approximate
                    || divisor_parts
                        .as_ref()
                        .is_some_and(|parts| current.division_fits(parts)))
            {
                break;
            }
            let Some(value) = current.div(divisor) else {
                break;
            };
            current = Self::new(value);
            remaining = remaining.saturating_sub(1);
            if approximate {
                break;
            }
        }
        (current.0, remaining)
    }

    fn division_fits(&self, right: &Complex) -> bool {
        let Some(left) = self.parts() else {
            return false;
        };
        if left.im.is_zero() && right.im.is_zero() {
            let (Some((a, b, negative)), Some((c, d, other_negative))) = (
                Self::fraction_capacity(&left.re),
                Self::fraction_capacity(&right.re),
            ) else {
                return false;
            };
            let numerator = u128::from(a).saturating_mul(u128::from(d));
            let denominator = u128::from(b).saturating_mul(u128::from(c));
            if denominator == 0 {
                return false;
            }
            let mut first = numerator;
            let mut second = denominator;
            while second != 0 {
                let remainder = first.checked_rem(second).unwrap_or(0);
                first = second;
                second = remainder;
            }
            let limit = if negative == other_negative {
                (1_u128 << 63).saturating_sub(1)
            } else {
                1_u128 << 63
            };
            return numerator
                .checked_div(first)
                .is_some_and(|value| value <= limit)
                && denominator
                    .checked_div(first)
                    .is_some_and(|value| value <= u128::from(u64::MAX));
        }
        let mut bounds = [(0_u32, 0_u32); 2];
        for (parts, bound) in [&left, right].into_iter().zip(&mut bounds) {
            for component in [&parts.re, &parts.im] {
                let Some((numerator, denominator, _)) = Self::fraction_capacity(component) else {
                    return false;
                };
                bound.0 = bound
                    .0
                    .max(u64::BITS.saturating_sub(numerator.leading_zeros()));
                bound.1 = bound.1.max(if denominator == 1 {
                    0
                } else {
                    u64::BITS.saturating_sub(denominator.leading_zeros())
                });
            }
        }
        let [(ln, ld), (rn, rd)] = bounds;
        // Complex division forms ac+bd, bc-ad, and c²+d² before division.
        // Cross-denominator bounds include those intermediate products and sums.
        let numerator = ln
            .saturating_add(rn)
            .saturating_add(ld)
            .saturating_add(rd.saturating_mul(5))
            .saturating_add(1);
        let denominator = ld
            .saturating_mul(2)
            .saturating_add(rn.saturating_mul(2))
            .saturating_add(rd.saturating_mul(4))
            .saturating_add(1);
        numerator <= 63 && denominator <= 63
    }

    /// Inspect representation capacity; Number performs the actual arithmetic.
    const fn fraction_capacity(value: &Real) -> Option<(u64, u64, bool)> {
        match value {
            Real::Int(integer) => Some((integer.unsigned_abs(), 1, *integer < 0)),
            Real::Rational(ratio) => Some((
                ratio.numer().unsigned_abs(),
                *ratio.denom(),
                *ratio.numer() < 0,
            )),
            Real::Float(_) | _ => None,
        }
    }
}
