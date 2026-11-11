//! The chain rule for calls: sparse custom partials, elementary slopes on
//! regular principal branches, and special-function identities.

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::{vec, vec::Vec};

use num_anafis::{Number, Real};

use crate::core::{
    BuiltinFunction, ExactIdentity, Expr, Function, FunctionDefinition, Property, Truth,
};

/// Namespace for function chain-rule and special-function derivatives.
pub struct Functions;

impl Functions {
    /// Apply $D f(u_{0},\ldots,u_{n})=\sum_{i} \partial_{i} f(u) D u_{i}$.
    /// Missing active custom partials preserve the whole derivative. Validation
    /// precedes callback execution, and a zero argument derivative needs no partial.
    pub fn function(
        expression: &Expr,
        function: &Function,
        args: &[Expr],
        primes: &[Expr],
    ) -> Option<Expr> {
        if primes.iter().all(Expr::is_exact_zero) {
            return Some(Expr::number(0));
        }
        if let Some(custom) = function.definition() {
            for (index, prime) in primes.iter().enumerate() {
                if !prime.is_exact_zero() && custom.partial_fn(index).is_none() {
                    return None;
                }
            }
            let mut iter = primes
                .iter()
                .enumerate()
                .filter(|(_, prime)| !prime.is_exact_zero())
                .filter_map(|(index, prime)| {
                    let value = (custom.partial_fn(index)?)(args);
                    (!value.is_exact_zero()).then(|| Mul::mul(value, prime))
                });
            let Some(first) = iter.next() else {
                return Some(Expr::number(0));
            };
            let Some(second) = iter.next() else {
                return Some(first);
            };
            let mut terms = Vec::with_capacity(iter.size_hint().0.saturating_add(2));
            terms.push(first);
            terms.push(second);
            terms.extend(iter);
            return Some(Expr::sum(terms));
        }
        let builtin = BuiltinFunction::of(function)?;
        match (args, primes) {
            ([argument], [prime]) => Self::unary(builtin, expression, argument, prime),
            ([first, second], [first_prime, second_prime]) => Self::binary(
                builtin,
                expression,
                [first, second],
                [first_prime, second_prime],
            ),
            ([degree, order, value], [degree_prime, order_prime, value_prime])
                if builtin == BuiltinFunction::AssocLegendre =>
            {
                Self::assoc_legendre(
                    expression,
                    [degree, order, value],
                    [degree_prime, order_prime, value_prime],
                )
            }
            ([value, lower, upper], [value_prime, lower_prime, upper_prime])
                if builtin == BuiltinFunction::Clamp =>
            {
                Self::clamp(
                    [value, lower, upper],
                    [value_prime, lower_prime, upper_prime],
                )
            }
            _ => None,
        }
    }

    /// Construct the derivative of a unary builtin call on its regular
    /// principal branch or real domain, without rebuilding the call itself.
    ///
    /// Reciprocal inverse functions retain their composed square-root branches.
    /// A slope is emitted whenever its singularity is an isolated point:
    /// $\lvert u\rvert' = \operatorname{sgn}(u)$ excludes $u = 0$, and
    /// $\operatorname{sinc}'$ and the complete elliptic slopes exclude $u = 0$.
    /// Step functions and $\operatorname{sgn}$ are locally constant between
    /// real discontinuities (and $\operatorname{fract}' = 1$ almost everywhere
    /// on $\mathbb{R}$), while arguments proved non-real defer because these
    /// functions are not complex-differentiable.
    #[expect(
        clippy::too_many_lines,
        reason = "exhaustive unary builtin derivative dispatch table"
    )]
    fn unary(function: BuiltinFunction, expression: &Expr, u: &Expr, du: &Expr) -> Option<Expr> {
        let slope = match function {
            BuiltinFunction::Sin => u.cos(),
            BuiltinFunction::Cos => Neg::neg(u.sin()),
            BuiltinFunction::Tan => u.sec().pow(2),
            BuiltinFunction::Cot => Neg::neg(u.csc().pow(2)),
            BuiltinFunction::Sec => Mul::mul(u.sec(), u.tan()),
            BuiltinFunction::Csc => Neg::neg(Mul::mul(u.csc(), u.cot())),
            BuiltinFunction::Asin => Div::div(1, Sub::sub(1, u.pow(2)).sqrt()),
            BuiltinFunction::Acos => Div::div(-1, Sub::sub(1, u.pow(2)).sqrt()),
            BuiltinFunction::Atan => Div::div(1, Add::add(1, u.pow(2))),
            BuiltinFunction::Acot => Div::div(-1, Add::add(1, u.pow(2))),
            BuiltinFunction::Asec | BuiltinFunction::Acsc => {
                let square = u.pow(2);
                let inverse_square = Div::div(1, &square);
                let denominator = Mul::mul(square, Sub::sub(1, inverse_square).sqrt());
                Div::div(
                    if function == BuiltinFunction::Asec {
                        1
                    } else {
                        -1
                    },
                    denominator,
                )
            }
            BuiltinFunction::Sinh => u.cosh(),
            BuiltinFunction::Cosh => u.sinh(),
            BuiltinFunction::Tanh => u.sech().pow(2),
            BuiltinFunction::Coth => Neg::neg(u.csch().pow(2)),
            BuiltinFunction::Sech => Neg::neg(Mul::mul(u.sech(), u.tanh())),
            BuiltinFunction::Csch => Neg::neg(Mul::mul(u.csch(), u.coth())),
            BuiltinFunction::Asinh => Div::div(1, Add::add(1, u.pow(2)).sqrt()),
            BuiltinFunction::Acosh => {
                Div::div(1, Mul::mul(Sub::sub(u, 1).sqrt(), Add::add(u, 1).sqrt()))
            }
            BuiltinFunction::Atanh | BuiltinFunction::Acoth => Div::div(1, Sub::sub(1, u.pow(2))),
            BuiltinFunction::Asech => {
                let reciprocal = Div::div(1, u);
                Div::div(
                    -1,
                    Expr::product(vec![
                        u.pow(2),
                        Sub::sub(&reciprocal, 1).sqrt(),
                        Add::add(reciprocal, 1).sqrt(),
                    ]),
                )
            }
            BuiltinFunction::Acsch => {
                let square = u.pow(2);
                Div::div(
                    -1,
                    Mul::mul(&square, Add::add(1, Div::div(1, &square)).sqrt()),
                )
            }
            BuiltinFunction::Exp => expression.clone(),
            BuiltinFunction::ExpM1 => u.exp(),
            BuiltinFunction::Ln => Div::div(1, u),
            BuiltinFunction::Ln1p => Div::div(1, Add::add(u, 1)),
            BuiltinFunction::Log2 => Div::div(1, Mul::mul(u, Expr::number(2).ln())),
            BuiltinFunction::Log10 => Div::div(1, Mul::mul(u, Expr::number(10).ln())),
            BuiltinFunction::Sqrt => Div::div(1, Mul::mul(2, expression)),
            BuiltinFunction::Cbrt => Div::div(1, Mul::mul(3, expression.pow(2))),
            BuiltinFunction::Erf | BuiltinFunction::Erfc => Mul::mul(
                Div::div(
                    if function == BuiltinFunction::Erf {
                        2
                    } else {
                        -2
                    },
                    Expr::pi().sqrt(),
                ),
                Neg::neg(u.pow(2)).exp(),
            ),
            BuiltinFunction::Gamma => Mul::mul(expression, u.digamma()),
            BuiltinFunction::Lgamma => u.digamma(),
            BuiltinFunction::Digamma => u.trigamma(),
            BuiltinFunction::Trigamma => u.tetragamma(),
            BuiltinFunction::Tetragamma => u.polygamma(3),
            BuiltinFunction::Zeta => u.zeta_deriv(1),
            BuiltinFunction::Abs => {
                let facts = u.properties();
                if facts.get(Property::Nonnegative) == Truth::True {
                    return Some(du.clone());
                }
                if facts.get(Property::Nonpositive) == Truth::True {
                    return Some(Neg::neg(du));
                }
                if facts.get(Property::Real) == Truth::False {
                    return None;
                }
                u.signum()
            }
            BuiltinFunction::Signum
            | BuiltinFunction::Heaviside
            | BuiltinFunction::Floor
            | BuiltinFunction::Ceil
            | BuiltinFunction::Round
            | BuiltinFunction::Trunc => {
                if u.properties().get(Property::Real) == Truth::False {
                    return None;
                }
                return Some(Expr::number(0));
            }
            BuiltinFunction::Re => {
                return Some(if u.properties().get(Property::Real) == Truth::True {
                    du.clone()
                } else {
                    du.re()
                });
            }
            BuiltinFunction::Im => {
                return Some(if u.properties().get(Property::Real) == Truth::True {
                    Expr::number(0)
                } else {
                    du.im()
                });
            }
            BuiltinFunction::Conj => {
                return Some(if u.properties().get(Property::Real) == Truth::True {
                    du.clone()
                } else {
                    du.conj()
                });
            }
            BuiltinFunction::Fract => {
                if u.properties().get(Property::Real) == Truth::False {
                    return None;
                }
                return Some(du.clone());
            }
            BuiltinFunction::Sinc => Div::div(Sub::sub(Mul::mul(u, u.cos()), u.sin()), u.pow(2)),
            BuiltinFunction::EllipticK => Div::div(
                Sub::sub(Div::div(u.elliptic_e(), Sub::sub(1, u)), expression),
                Mul::mul(2, u),
            ),
            BuiltinFunction::EllipticE => {
                Div::div(Sub::sub(expression, u.elliptic_k()), Mul::mul(2, u))
            }
            BuiltinFunction::Polygamma
            | BuiltinFunction::BesselJ
            | BuiltinFunction::BesselY
            | BuiltinFunction::BesselI
            | BuiltinFunction::BesselK
            | BuiltinFunction::ZetaDeriv
            | BuiltinFunction::Log
            | BuiltinFunction::Hermite
            | BuiltinFunction::LambertW
            | BuiltinFunction::Beta
            | BuiltinFunction::Atan2
            | BuiltinFunction::Hypot
            | BuiltinFunction::Max
            | BuiltinFunction::Min
            | BuiltinFunction::Modulo
            | BuiltinFunction::Rem
            | BuiltinFunction::AssocLegendre
            | BuiltinFunction::Clamp => return None,
        };
        Some(Mul::mul(slope, du))
    }

    /// Construct binary special-function derivatives on their regular domains.
    /// The Lambert slope $e^{-W}/(1+W)$ retains the regular principal value at zero.
    fn binary(
        function: BuiltinFunction,
        expression: &Expr,
        args: [&Expr; 2],
        primes: [&Expr; 2],
    ) -> Option<Expr> {
        let result = match (function, args, primes) {
            (
                BuiltinFunction::Polygamma | BuiltinFunction::ZetaDeriv | BuiltinFunction::Hermite,
                [n, u],
                [dn, du],
            ) if dn.is_exact_zero() => return Self::discrete(function, n, u, du),
            (
                BuiltinFunction::BesselJ
                | BuiltinFunction::BesselY
                | BuiltinFunction::BesselI
                | BuiltinFunction::BesselK,
                [n, u],
                [dn, du],
            ) if dn.is_exact_zero() => return Self::bessel(function, n, u, du),
            (BuiltinFunction::LambertW, [_, _], [db, du]) if db.is_exact_zero() => Mul::mul(
                Div::div(Neg::neg(expression).exp(), Add::add(expression, 1)),
                du,
            ),
            (BuiltinFunction::Beta, [a, b], [da, db]) => {
                let common = Add::add(a, b).digamma();
                let delta = match (!da.is_exact_zero(), !db.is_exact_zero()) {
                    (true, false) => Mul::mul(Sub::sub(a.digamma(), common), da),
                    (false, true) => Mul::mul(Sub::sub(b.digamma(), common), db),
                    (true, true) => Add::add(
                        Mul::mul(Sub::sub(a.digamma(), &common), da),
                        Mul::mul(Sub::sub(b.digamma(), common), db),
                    ),
                    (false, false) => return Some(Expr::number(0)),
                };
                Mul::mul(expression, delta)
            }
            (BuiltinFunction::Log, [base, u], [db, du]) => {
                match (!du.is_exact_zero(), !db.is_exact_zero()) {
                    (true, false) => Div::div(du, Mul::mul(u, base.ln())),
                    (false, true) => Neg::neg(Div::div(
                        Mul::mul(u.ln(), db),
                        Mul::mul(base, base.ln().pow(2)),
                    )),
                    (true, true) => Sub::sub(
                        Div::div(du, Mul::mul(u, base.ln())),
                        Div::div(Mul::mul(u.ln(), db), Mul::mul(base, base.ln().pow(2))),
                    ),
                    (false, false) => Expr::number(0),
                }
            }
            (BuiltinFunction::Atan2, [y, x], [dy, dx]) => {
                let numerator = match (!dy.is_exact_zero(), !dx.is_exact_zero()) {
                    (true, false) => Mul::mul(x, dy),
                    (false, true) => Neg::neg(Mul::mul(y, dx)),
                    (true, true) => Sub::sub(Mul::mul(x, dy), Mul::mul(y, dx)),
                    (false, false) => return Some(Expr::number(0)),
                };
                Div::div(numerator, Add::add(x.pow(2), y.pow(2)))
            }
            (BuiltinFunction::Hypot, [u, v], [du, dv]) => {
                let numerator = match (!du.is_exact_zero(), !dv.is_exact_zero()) {
                    (true, false) => Mul::mul(u, du),
                    (false, true) => Mul::mul(v, dv),
                    (true, true) => Add::add(Mul::mul(u, du), Mul::mul(v, dv)),
                    (false, false) => return Some(Expr::number(0)),
                };
                Div::div(numerator, expression)
            }
            (BuiltinFunction::Max | BuiltinFunction::Min, [u, v], [du, dv]) => {
                return Self::extremum(function == BuiltinFunction::Max, [u, v], [du, dv]);
            }
            (BuiltinFunction::Modulo | BuiltinFunction::Rem, [u, v], [du, dv]) => {
                return Self::remainder(function == BuiltinFunction::Modulo, [u, v], [du, dv]);
            }
            _ => return None,
        };
        Some(result)
    }

    /// Almost-everywhere chain rule of `max(u, v)` and `min(u, v)` on real domains.
    fn extremum(is_max: bool, [u, v]: [&Expr; 2], [du, dv]: [&Expr; 2]) -> Option<Expr> {
        if u.properties().get(Property::Real) == Truth::False
            || v.properties().get(Property::Real) == Truth::False
        {
            return None;
        }
        let (hi, lo) = if is_max { (u, v) } else { (v, u) };
        let first = || Sub::sub(hi, lo).heaviside();
        let second = || Sub::sub(lo, hi).heaviside();
        Some(match (!du.is_exact_zero(), !dv.is_exact_zero()) {
            (true, false) => Mul::mul(first(), du),
            (false, true) => Mul::mul(second(), dv),
            (true, true) => Add::add(Mul::mul(first(), du), Mul::mul(second(), dv)),
            (false, false) => Expr::number(0),
        })
    }

    /// Almost-everywhere chain rule of `mod(u, v) = u - v * floor(u / v)` and
    /// `rem(u, v) = u - v * trunc(u / v)` on real domains.
    fn remainder(is_modulo: bool, [u, v]: [&Expr; 2], [du, dv]: [&Expr; 2]) -> Option<Expr> {
        if u.properties().get(Property::Real) == Truth::False
            || v.properties().get(Property::Real) == Truth::False
        {
            return None;
        }
        let quotient = || {
            let ratio = Div::div(u, v);
            if is_modulo {
                ratio.floor()
            } else {
                ratio.trunc()
            }
        };
        Some(match (!du.is_exact_zero(), !dv.is_exact_zero()) {
            (true, false) => du.clone(),
            (false, true) => Neg::neg(Mul::mul(quotient(), dv)),
            (true, true) => Sub::sub(du, Mul::mul(quotient(), dv)),
            (false, false) => Expr::number(0),
        })
    }

    /// Almost-everywhere chain rule of `clamp(u, lo, hi) = min(max(u, lo), hi)`
    /// on real domains.
    fn clamp(
        [value, lower, upper]: [&Expr; 3],
        [value_prime, lower_prime, upper_prime]: [&Expr; 3],
    ) -> Option<Expr> {
        if value.properties().get(Property::Real) == Truth::False
            || lower.properties().get(Property::Real) == Truth::False
            || upper.properties().get(Property::Real) == Truth::False
        {
            return None;
        }
        if lower_prime.is_exact_zero()
            && upper_prime.is_exact_zero()
            && let (Some((lower_re, lower_im)), Some((upper_re, upper_im))) = (
                lower.as_number().map(Number::components),
                upper.as_number().map(Number::components),
            )
            && lower_im.is_zero()
            && upper_im.is_zero()
            && Self::is_finite_real(lower_re)
            && Self::is_finite_real(upper_re)
            && let Some(order) = lower_re.partial_cmp(upper_re)
        {
            if order.is_ge() {
                return Some(Expr::number(0));
            }
            return Some(Mul::mul(
                Sub::sub(upper, value).heaviside(),
                Mul::mul(Sub::sub(value, lower).heaviside(), value_prime),
            ));
        }
        let floored = value.max(lower.clone());
        let inner = match (!value_prime.is_exact_zero(), !lower_prime.is_exact_zero()) {
            (true, false) => Some(Mul::mul(Sub::sub(value, lower).heaviside(), value_prime)),
            (false, true) => Some(Mul::mul(Sub::sub(lower, value).heaviside(), lower_prime)),
            (true, true) => Some(Add::add(
                Mul::mul(Sub::sub(value, lower).heaviside(), value_prime),
                Mul::mul(Sub::sub(lower, value).heaviside(), lower_prime),
            )),
            (false, false) => None,
        };
        let lower_part =
            inner.map(|derivative| Mul::mul(Sub::sub(upper, &floored).heaviside(), derivative));
        let upper_part = (!upper_prime.is_exact_zero())
            .then(|| Mul::mul(Sub::sub(&floored, upper).heaviside(), upper_prime));
        Some(match (lower_part, upper_part) {
            (Some(left), Some(right)) => Add::add(left, right),
            (Some(only), None) | (None, Some(only)) => only,
            (None, None) => Expr::number(0),
        })
    }

    const fn is_finite_real(value: &Real) -> bool {
        matches!(value, Real::Int(_) | Real::Rational(_))
            || matches!(value, Real::Float(native) if native.is_finite())
    }

    /// DLMF 14.10.5 for associated Legendre functions $P_{\nu}^{\mu}(u)$ at
    /// constant degree and order, excluding $u = \pm 1$.
    fn assoc_legendre(
        expression: &Expr,
        [degree, order, value]: [&Expr; 3],
        [degree_prime, order_prime, value_prime]: [&Expr; 3],
    ) -> Option<Expr> {
        if !degree_prime.is_exact_zero()
            || !order_prime.is_exact_zero()
            || matches!(degree.exact_ratio(), Some((i64::MIN, 1)))
        {
            return None;
        }
        if degree.is_exact_zero() && order.is_exact_zero() {
            return Some(Expr::number(0));
        }
        let leading = Mul::mul(Mul::mul(degree, value), expression);
        // For exact integer $n \ge 0$ and $\lvert m \rvert = n$, the lowered-degree
        // term $(n + m) P_{n-1}^{m}(u)$ vanishes identically without entering
        // out-of-range $\lvert m \rvert > n - 1$.
        let numerator = if let (Some((n, 1)), Some((m, 1))) =
            (degree.exact_ratio(), order.exact_ratio())
            && n >= 0
            && m.checked_abs() == Some(n)
        {
            leading
        } else {
            let previous = value.assoc_legendre(Sub::sub(degree, 1), order.clone());
            Sub::sub(leading, Mul::mul(Add::add(degree, order), previous))
        };
        Some(Mul::mul(
            Div::div(numerator, Sub::sub(value.pow(2), 1)),
            value_prime,
        ))
    }

    /// Integer-indexed families, raising the index by one:
    /// $H_{n}' = 2n H_{n-1}$, and the derivatives of `polygamma(n, u)` and
    /// `zeta_deriv(n, u)` are `polygamma(n + 1, u)` and `zeta_deriv(n + 1, u)`.
    fn discrete(
        function: BuiltinFunction,
        index: &Expr,
        argument: &Expr,
        prime: &Expr,
    ) -> Option<Expr> {
        let Some((order, 1)) = index.exact_ratio().filter(|&(order, _)| order >= 0) else {
            return None;
        };
        let slope = if function == BuiltinFunction::Hermite {
            if order == 0 {
                return Some(Expr::number(0));
            }
            Mul::mul(
                Expr::number(order.checked_mul(2)?),
                argument.hermite(Expr::number(order.checked_sub(1)?)),
            )
        } else {
            let next = Expr::number(order.checked_add(1)?);
            if function == BuiltinFunction::Polygamma {
                argument.polygamma(next)
            } else {
                argument.zeta_deriv(next)
            }
        };
        Some(Mul::mul(slope, prime))
    }

    /// DLMF 10.6.1 and 10.29.1 give adjacent-order derivatives with no argument pole.
    fn bessel(
        function: BuiltinFunction,
        order: &Expr,
        argument: &Expr,
        prime: &Expr,
    ) -> Option<Expr> {
        if matches!(
            order.as_number().map(Number::components),
            Some((Real::Int(i64::MIN | i64::MAX), _))
        ) {
            return None;
        }
        let lower = FunctionDefinition::construct(
            function.function(),
            vec![Sub::sub(order, 1), argument.clone()],
        );
        let upper = FunctionDefinition::construct(
            function.function(),
            vec![Add::add(order, 1), argument.clone()],
        );
        let difference = matches!(
            function,
            BuiltinFunction::BesselJ | BuiltinFunction::BesselY
        );
        let combined = if difference {
            Sub::sub(lower, upper)
        } else {
            Add::add(lower, upper)
        };
        let signed = if function == BuiltinFunction::BesselK {
            Neg::neg(combined)
        } else {
            combined
        };
        Some(Mul::mul(Div::div(signed, 2), prime))
    }
}
