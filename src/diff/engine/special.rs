//! Special-function derivatives using registered identities and exact constants.

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::{vec, vec::Vec};

use num_anafis::Real;

use crate::core::{BuiltinFunction, Expr, FunctionDefinition, Property, Truth};

use super::Differentiation;

impl Differentiation<'_> {
    /// Construct special-function derivatives on their regular domains.
    /// Bessel recurrences use adjacent orders without division by the argument.
    /// The Lambert slope \(e^{-W}/(1+W)\) retains the regular principal value at zero.
    pub fn special(function: BuiltinFunction, args: &[Expr], primes: &[Expr]) -> Option<Expr> {
        let result = match (function, args, primes) {
            (BuiltinFunction::Erf | BuiltinFunction::Erfc, [u], [du]) => Expr::product(vec![
                Div::div(
                    if function == BuiltinFunction::Erf {
                        2
                    } else {
                        -2
                    },
                    Expr::pi().sqrt(),
                ),
                Neg::neg(u.pow(2)).exp(),
                du.clone(),
            ]),
            (BuiltinFunction::Gamma, [u], [du]) => {
                Expr::product(vec![u.gamma(), u.digamma(), du.clone()])
            }
            (BuiltinFunction::Lgamma, [u], [du]) => Mul::mul(u.digamma(), du),
            (BuiltinFunction::Digamma, [u], [du]) => Mul::mul(u.trigamma(), du),
            (BuiltinFunction::Trigamma, [u], [du]) => Mul::mul(u.tetragamma(), du),
            (BuiltinFunction::Tetragamma, [u], [du]) => Mul::mul(u.polygamma(3), du),
            (BuiltinFunction::Zeta, [u], [du]) => Mul::mul(u.zeta_deriv(1), du),
            (BuiltinFunction::Log2 | BuiltinFunction::Log10, [u], [du]) => {
                let base = if function == BuiltinFunction::Log2 {
                    2
                } else {
                    10
                };
                Div::div(du, Mul::mul(u, Expr::number(base).ln()))
            }
            (BuiltinFunction::Abs, [u], [du]) => {
                let facts = u.properties();
                if facts.get(Property::Nonnegative) == Truth::True {
                    du.clone()
                } else if facts.get(Property::Negative) == Truth::True {
                    Neg::neg(du)
                } else if facts.get(Property::Real) == Truth::True
                    && facts.get(Property::Nonzero) == Truth::True
                {
                    Mul::mul(u.signum(), du)
                } else {
                    return None;
                }
            }
            (BuiltinFunction::Sinc, [u], [du])
                if u.properties().get(Property::Nonzero) == Truth::True =>
            {
                Mul::mul(
                    Div::div(Sub::sub(Mul::mul(u, u.cos()), u.sin()), u.pow(2)),
                    du,
                )
            }
            (BuiltinFunction::EllipticK | BuiltinFunction::EllipticE, [u], [du])
                if u.properties().get(Property::Nonzero) == Truth::True =>
            {
                let slope = if function == BuiltinFunction::EllipticK {
                    Div::div(
                        Sub::sub(Div::div(u.elliptic_e(), Sub::sub(1, u)), u.elliptic_k()),
                        Mul::mul(2, u),
                    )
                } else {
                    Div::div(Sub::sub(u.elliptic_e(), u.elliptic_k()), Mul::mul(2, u))
                };
                Mul::mul(slope, du)
            }
            (BuiltinFunction::SphericalHarmonic, [degree, order, theta, phi], [dd, dn, dt, dp])
                if dd.is_zero_num() && dn.is_zero_num() && dt.is_zero_num() =>
            {
                Expr::product(vec![
                    Expr::i(),
                    order.clone(),
                    theta.spherical_harmonic(degree, order, phi),
                    dp.clone(),
                ])
            }
            (_, [first, second], [first_prime, second_prime]) => {
                return Self::binary(function, [first, second], [first_prime, second_prime]);
            }
            _ => return None,
        };
        Some(result)
    }

    fn binary(function: BuiltinFunction, args: [&Expr; 2], primes: [&Expr; 2]) -> Option<Expr> {
        let result = match (function, args, primes) {
            (
                BuiltinFunction::Polygamma | BuiltinFunction::ZetaDeriv | BuiltinFunction::Hermite,
                [n, u],
                [dn, du],
            ) if dn.is_zero_num() => return Self::discrete(function, n, u, du),
            (
                BuiltinFunction::BesselJ
                | BuiltinFunction::BesselY
                | BuiltinFunction::BesselI
                | BuiltinFunction::BesselK,
                [n, u],
                [dn, du],
            ) if dn.is_zero_num() => return Self::bessel(function, n, u, du),
            (BuiltinFunction::LambertW, [branch, u], [db, du]) if db.is_zero_num() => {
                let value = u.lambertw(branch);
                Mul::mul(Div::div(Neg::neg(&value).exp(), Add::add(value, 1)), du)
            }
            (BuiltinFunction::Beta, [a, b], [da, db]) => {
                let common = Add::add(a, b).digamma();
                let mut terms = Vec::new();
                if !da.is_zero_num() {
                    terms.push(Mul::mul(Sub::sub(a.digamma(), &common), da));
                }
                if !db.is_zero_num() {
                    terms.push(Mul::mul(Sub::sub(b.digamma(), common), db));
                }
                Mul::mul(a.beta(b), Expr::sum(terms))
            }
            (BuiltinFunction::Log, [base, u], [db, du]) => {
                let mut terms = Vec::new();
                if !du.is_zero_num() {
                    terms.push(Div::div(du, Mul::mul(u, base.ln())));
                }
                if !db.is_zero_num() {
                    terms.push(Neg::neg(Div::div(
                        Mul::mul(u.ln(), db),
                        Mul::mul(base, base.ln().pow(2)),
                    )));
                }
                Expr::sum(terms)
            }
            (BuiltinFunction::Atan2, [y, x], [dy, dx]) => {
                let mut terms = Vec::new();
                if !dy.is_zero_num() {
                    terms.push(Mul::mul(x, dy));
                }
                if !dx.is_zero_num() {
                    terms.push(Neg::neg(Mul::mul(y, dx)));
                }
                Div::div(Expr::sum(terms), Add::add(x.pow(2), y.pow(2)))
            }
            _ => return None,
        };
        Some(result)
    }

    fn discrete(
        function: BuiltinFunction,
        index: &Expr,
        argument: &Expr,
        prime: &Expr,
    ) -> Option<Expr> {
        let components = index.as_number()?.extract_complex().ok()?;
        let Real::Int(order) = components.re else {
            return None;
        };
        if !components.im.is_zero() || order < 0 {
            return None;
        }
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
        if let Some(value) = order.as_number()
            && let Ok(parts) = value.extract_complex()
            && let Real::Int(integer) = parts.re
            && (integer.checked_add(1).is_none() || integer.checked_sub(1).is_none())
        {
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
