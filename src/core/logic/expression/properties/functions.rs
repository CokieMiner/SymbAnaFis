//! Function-domain proofs dispatched by static builtin identities.

use super::{BuiltinFunction, Facts, Property, Truth};

#[derive(Clone, Copy, PartialEq, Eq)]
enum RoundingKind {
    Floor,
    Ceil,
    Round,
    Trunc,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FunctionDomain {
    Absolute,
    Exponential,
    SquareRoot,
    CubeRoot,
    Logarithm,
    Ln1p,
    Gamma,
    Signum,
    RealPart,
    ImaginaryPart,
    Conjugate,
    Heaviside,
    Rounding(RoundingKind),
    MonotonicZero { finite: bool },
    FiniteImage(Property),
    RealImage(Property),
    Unknown,
}

impl Facts {
    /// Facts of a builtin call from the facts of its arguments. Other calls
    /// have no proved facts.
    pub fn function(identity: Option<BuiltinFunction>, arguments: &[Self]) -> Self {
        let Some(builtin) = identity else {
            return Self::default();
        };
        if arguments
            .iter()
            .any(|argument| argument.get(Property::Finite) != Truth::True)
        {
            return Self::default();
        }
        match *arguments {
            [input] => Self::unary_function(builtin, input),
            [first, second] => Self::binary_function(builtin, first, second),
            [first, second, third] if builtin == BuiltinFunction::Clamp => {
                let mut inner = Self::default();
                extremum_facts(&mut inner, true, first, second);
                inner.close();
                let mut clamped = Self::default();
                extremum_facts(&mut clamped, false, inner, third);
                clamped
            }
            _ => Self::default(),
        }
    }

    fn unary_function(builtin: BuiltinFunction, input: Self) -> Self {
        let mut facts = Self::default();
        let domain = FunctionDomain::of(builtin);
        match domain {
            FunctionDomain::Absolute => {
                facts.prove(Property::Nonnegative);
                propagate(&mut facts, input, &[Property::Zero, Property::Nonzero]);
                if input.get(Property::Real) == Truth::True {
                    propagate(&mut facts, input, &[Property::Integer, Property::Rational]);
                }
            }
            FunctionDomain::Exponential => {
                facts.prove(Property::Nonzero);
                if input.get(Property::Real) == Truth::True {
                    facts.prove(Property::Positive);
                }
            }
            FunctionDomain::SquareRoot | FunctionDomain::CubeRoot => {
                root_facts(&mut facts, input, domain == FunctionDomain::SquareRoot);
            }
            FunctionDomain::Logarithm => logarithm_facts(&mut facts, input),
            FunctionDomain::Ln1p => {
                if input.get(Property::Nonnegative) == Truth::True {
                    facts.prove(Property::Nonnegative);
                    propagate(&mut facts, input, &[Property::Zero, Property::Positive]);
                }
            }
            FunctionDomain::Gamma => {
                if input.get(Property::Positive) == Truth::True {
                    facts.prove(Property::Positive);
                    if input.get(Property::Integer) == Truth::True {
                        facts.prove(Property::Integer);
                    }
                }
            }
            FunctionDomain::Signum => signum_facts(&mut facts, input),
            FunctionDomain::RealPart => {
                if input.get(Property::Real) == Truth::True {
                    return input;
                }
                facts.prove(Property::Real);
                propagate(&mut facts, input, &[Property::Zero]);
            }
            FunctionDomain::ImaginaryPart => {
                if input.get(Property::Real) == Truth::True {
                    facts.prove(Property::Zero);
                } else {
                    facts.prove(Property::Real);
                    if input.get(Property::Real) == Truth::False {
                        facts.prove(Property::Nonzero);
                    }
                }
            }
            FunctionDomain::Conjugate => {
                if input.get(Property::Real) == Truth::True {
                    return input;
                }
                facts.prove(Property::Finite);
                propagate(&mut facts, input, &[Property::Zero, Property::Nonzero]);
                if input.get(Property::Real) == Truth::False {
                    facts.refute(Property::Real);
                }
            }
            FunctionDomain::Heaviside => heaviside_facts(&mut facts, input),
            FunctionDomain::Rounding(kind) => rounding_facts(&mut facts, kind, input),
            FunctionDomain::MonotonicZero { finite } => {
                if finite {
                    facts.prove(Property::Finite);
                }
                if input.get(Property::Real) == Truth::True {
                    facts.prove(Property::Real);
                    propagate(
                        &mut facts,
                        input,
                        &[
                            Property::Zero,
                            Property::Nonzero,
                            Property::Positive,
                            Property::Negative,
                            Property::Nonnegative,
                            Property::Nonpositive,
                        ],
                    );
                }
            }
            FunctionDomain::FiniteImage(image) => {
                facts.prove(Property::Finite);
                if input.get(Property::Real) == Truth::True {
                    facts.prove(image);
                }
            }
            FunctionDomain::RealImage(image) => {
                if input.get(Property::Real) == Truth::True {
                    facts.prove(image);
                }
            }
            FunctionDomain::Unknown => {}
        }
        facts
    }

    fn binary_function(builtin: BuiltinFunction, first: Self, second: Self) -> Self {
        let mut facts = Self::default();
        if builtin == BuiltinFunction::Atan2 {
            atan2_facts(&mut facts, first, second);
        } else if builtin == BuiltinFunction::Beta
            && first.get(Property::Positive) == Truth::True
            && second.get(Property::Positive) == Truth::True
        {
            facts.prove(Property::Positive);
            if first.get(Property::Integer) == Truth::True
                && second.get(Property::Integer) == Truth::True
            {
                facts.prove(Property::Rational);
            }
        } else if builtin == BuiltinFunction::Hypot {
            hypot_facts(&mut facts, first, second);
        } else if matches!(builtin, BuiltinFunction::Max | BuiltinFunction::Min) {
            extremum_facts(&mut facts, builtin == BuiltinFunction::Max, first, second);
        } else if matches!(builtin, BuiltinFunction::Modulo | BuiltinFunction::Rem) {
            remainder_facts(
                &mut facts,
                builtin == BuiltinFunction::Modulo,
                first,
                second,
            );
        }
        facts
    }
}

fn propagate(target: &mut Facts, source: Facts, properties: &[Property]) {
    for &property in properties {
        if source.get(property) == Truth::True {
            target.prove(property);
        }
    }
}

fn root_facts(facts: &mut Facts, input: Facts, is_square_root: bool) {
    facts.prove(Property::Finite);
    propagate(
        facts,
        input,
        &[
            Property::Zero,
            Property::Nonzero,
            Property::Positive,
            Property::Nonnegative,
        ],
    );
    if is_square_root && input.get(Property::Negative) == Truth::True {
        facts.refute(Property::Real);
    }
    if !is_square_root && input.get(Property::Real) == Truth::True {
        facts.prove(Property::Real);
        propagate(facts, input, &[Property::Negative, Property::Nonpositive]);
    }
}

fn logarithm_facts(facts: &mut Facts, input: Facts) {
    if input.get(Property::Zero) == Truth::True {
        facts.refute(Property::Finite);
    } else if input.get(Property::Nonzero) == Truth::True {
        facts.prove(Property::Finite);
        if input.get(Property::Positive) == Truth::True {
            facts.prove(Property::Real);
        } else if input.get(Property::Negative) == Truth::True {
            facts.refute(Property::Real);
        }
    }
}

fn signum_facts(facts: &mut Facts, input: Facts) {
    facts.prove(Property::Finite);
    propagate(facts, input, &[Property::Nonzero]);
    if input.get(Property::Real) == Truth::True {
        facts.prove(Property::Integer);
        propagate(
            facts,
            input,
            &[
                Property::Zero,
                Property::Positive,
                Property::Negative,
                Property::Nonnegative,
                Property::Nonpositive,
            ],
        );
    }
}

fn rounding_facts(facts: &mut Facts, kind: RoundingKind, input: Facts) {
    if input.get(Property::Real) != Truth::True {
        return;
    }
    facts.prove(Property::Integer);
    propagate(
        facts,
        input,
        &[Property::Zero, Property::Nonnegative, Property::Nonpositive],
    );
    if kind == RoundingKind::Ceil && input.get(Property::Positive) == Truth::True {
        facts.prove(Property::Positive);
    }
    if kind == RoundingKind::Floor && input.get(Property::Negative) == Truth::True {
        facts.prove(Property::Negative);
    }
    if input.get(Property::Integer) == Truth::True {
        propagate(
            facts,
            input,
            &[Property::Nonzero, Property::Positive, Property::Negative],
        );
    }
}

fn atan2_facts(facts: &mut Facts, y: Facts, x: Facts) {
    if y.get(Property::Real) != Truth::True || x.get(Property::Real) != Truth::True {
        return;
    }
    if y.get(Property::Nonzero) == Truth::True || x.get(Property::Nonzero) == Truth::True {
        facts.prove(Property::Real);
    }
    if y.get(Property::Positive) == Truth::True {
        facts.prove(Property::Positive);
    } else if y.get(Property::Negative) == Truth::True {
        facts.prove(Property::Negative);
    } else if y.get(Property::Zero) == Truth::True {
        if x.get(Property::Positive) == Truth::True {
            facts.prove(Property::Zero);
        } else if x.get(Property::Negative) == Truth::True {
            // IEEE 754 distinguishes $\operatorname{atan2}(+0.0, x) = +\pi$ and
            // $\operatorname{atan2}(-0.0, x) = -\pi$, so only nonzero realness
            // holds across both signed zeros.
            facts.prove(Property::Nonzero);
        }
    } else if x.get(Property::Positive) == Truth::True {
        if y.get(Property::Nonnegative) == Truth::True {
            facts.prove(Property::Nonnegative);
        } else if y.get(Property::Nonpositive) == Truth::True {
            facts.prove(Property::Nonpositive);
        }
    } else if x.get(Property::Negative) == Truth::True {
        // For $x < 0$ and any real $y$, $\operatorname{atan2}(y, x) \in [-\pi, 0) \cup (0, \pi]$.
        facts.prove(Property::Nonzero);
    }
}

fn heaviside_facts(facts: &mut Facts, input: Facts) {
    if input.get(Property::Real) == Truth::False {
        facts.refute(Property::Finite);
        return;
    }
    if input.get(Property::Real) != Truth::True {
        return;
    }
    facts.prove(Property::Rational);
    facts.prove(Property::Nonnegative);
    if input.get(Property::Positive) == Truth::True {
        facts.prove(Property::Positive);
        facts.prove(Property::Integer);
    } else if input.get(Property::Nonnegative) == Truth::True {
        // Half-maximum convention: H(0) = 1/2 > 0, so u >= 0 implies H(u) > 0.
        facts.prove(Property::Positive);
    } else if input.get(Property::Negative) == Truth::True {
        facts.prove(Property::Zero);
    }
    if input.get(Property::Zero) == Truth::True {
        facts.refute(Property::Integer);
    } else if input.get(Property::Nonzero) == Truth::True {
        facts.prove(Property::Integer);
    }
}

fn hypot_facts(facts: &mut Facts, first: Facts, second: Facts) {
    facts.prove(Property::Finite);
    if first.get(Property::Real) != Truth::True || second.get(Property::Real) != Truth::True {
        return;
    }
    facts.prove(Property::Nonnegative);
    if first.get(Property::Nonzero) == Truth::True || second.get(Property::Nonzero) == Truth::True {
        facts.prove(Property::Positive);
    } else if first.get(Property::Zero) == Truth::True && second.get(Property::Zero) == Truth::True
    {
        facts.prove(Property::Zero);
    }
}

fn prove_real_tier(facts: &mut Facts, first: Facts, second: Facts) {
    facts.prove(Property::Real);
    if first.get(Property::Integer) == Truth::True && second.get(Property::Integer) == Truth::True {
        facts.prove(Property::Integer);
    } else if first.get(Property::Rational) == Truth::True
        && second.get(Property::Rational) == Truth::True
    {
        facts.prove(Property::Rational);
    }
}

fn extremum_facts(facts: &mut Facts, is_max: bool, first: Facts, second: Facts) {
    if first.get(Property::Real) == Truth::False || second.get(Property::Real) == Truth::False {
        facts.refute(Property::Finite);
        return;
    }
    if first.get(Property::Real) != Truth::True || second.get(Property::Real) != Truth::True {
        return;
    }
    prove_real_tier(facts, first, second);
    let (any_strict, any_weak, all_strict, all_weak) = if is_max {
        (
            Property::Positive,
            Property::Nonnegative,
            Property::Negative,
            Property::Nonpositive,
        )
    } else {
        (
            Property::Negative,
            Property::Nonpositive,
            Property::Positive,
            Property::Nonnegative,
        )
    };
    if first.get(any_strict) == Truth::True || second.get(any_strict) == Truth::True {
        facts.prove(any_strict);
    } else if first.get(any_weak) == Truth::True || second.get(any_weak) == Truth::True {
        facts.prove(any_weak);
    }
    if first.get(all_strict) == Truth::True && second.get(all_strict) == Truth::True {
        facts.prove(all_strict);
    } else if first.get(all_weak) == Truth::True && second.get(all_weak) == Truth::True {
        facts.prove(all_weak);
    }
    if first.get(Property::Zero) == Truth::True && second.get(Property::Zero) == Truth::True {
        facts.prove(Property::Zero);
    } else if first.get(Property::Nonzero) == Truth::True
        && second.get(Property::Nonzero) == Truth::True
    {
        facts.prove(Property::Nonzero);
    }
}

fn remainder_facts(facts: &mut Facts, is_modulo: bool, dividend: Facts, divisor: Facts) {
    if divisor.get(Property::Zero) == Truth::True
        || dividend.get(Property::Real) == Truth::False
        || divisor.get(Property::Real) == Truth::False
    {
        facts.refute(Property::Finite);
        return;
    }
    if dividend.get(Property::Real) != Truth::True
        || divisor.get(Property::Real) != Truth::True
        || divisor.get(Property::Nonzero) != Truth::True
    {
        return;
    }
    prove_real_tier(facts, dividend, divisor);
    if dividend.get(Property::Zero) == Truth::True {
        facts.prove(Property::Zero);
    } else if is_modulo {
        if divisor.get(Property::Positive) == Truth::True {
            facts.prove(Property::Nonnegative);
        } else if divisor.get(Property::Negative) == Truth::True {
            facts.prove(Property::Nonpositive);
        }
    } else if dividend.get(Property::Nonnegative) == Truth::True {
        facts.prove(Property::Nonnegative);
    } else if dividend.get(Property::Nonpositive) == Truth::True {
        facts.prove(Property::Nonpositive);
    }
}

impl FunctionDomain {
    const fn of(builtin: BuiltinFunction) -> Self {
        match builtin {
            BuiltinFunction::Abs => Self::Absolute,
            BuiltinFunction::Exp => Self::Exponential,
            BuiltinFunction::Sqrt => Self::SquareRoot,
            BuiltinFunction::Cbrt => Self::CubeRoot,
            BuiltinFunction::Ln | BuiltinFunction::Log2 | BuiltinFunction::Log10 => Self::Logarithm,
            BuiltinFunction::Ln1p => Self::Ln1p,
            BuiltinFunction::Gamma => Self::Gamma,
            BuiltinFunction::Signum => Self::Signum,
            BuiltinFunction::Re => Self::RealPart,
            BuiltinFunction::Im => Self::ImaginaryPart,
            BuiltinFunction::Conj => Self::Conjugate,
            BuiltinFunction::Heaviside => Self::Heaviside,
            BuiltinFunction::Floor => Self::Rounding(RoundingKind::Floor),
            BuiltinFunction::Ceil => Self::Rounding(RoundingKind::Ceil),
            BuiltinFunction::Round => Self::Rounding(RoundingKind::Round),
            BuiltinFunction::Trunc => Self::Rounding(RoundingKind::Trunc),
            BuiltinFunction::Sinh
            | BuiltinFunction::Asinh
            | BuiltinFunction::Erf
            | BuiltinFunction::ExpM1 => Self::MonotonicZero { finite: true },
            BuiltinFunction::Tanh | BuiltinFunction::Atan => Self::MonotonicZero { finite: false },
            BuiltinFunction::Cosh | BuiltinFunction::Erfc => Self::FiniteImage(Property::Positive),
            BuiltinFunction::Sin | BuiltinFunction::Cos | BuiltinFunction::Sinc => {
                Self::FiniteImage(Property::Real)
            }
            BuiltinFunction::Fract => Self::RealImage(Property::Real),
            BuiltinFunction::Sech => Self::RealImage(Property::Positive),
            BuiltinFunction::Tan
            | BuiltinFunction::Cot
            | BuiltinFunction::Sec
            | BuiltinFunction::Csc
            | BuiltinFunction::Asin
            | BuiltinFunction::Acos
            | BuiltinFunction::Acot
            | BuiltinFunction::Asec
            | BuiltinFunction::Acsc
            | BuiltinFunction::Coth
            | BuiltinFunction::Csch
            | BuiltinFunction::Acosh
            | BuiltinFunction::Atanh
            | BuiltinFunction::Acoth
            | BuiltinFunction::Asech
            | BuiltinFunction::Acsch
            | BuiltinFunction::Lgamma
            | BuiltinFunction::Digamma
            | BuiltinFunction::Trigamma
            | BuiltinFunction::Tetragamma
            | BuiltinFunction::Zeta
            | BuiltinFunction::EllipticK
            | BuiltinFunction::EllipticE
            | BuiltinFunction::Polygamma
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
            | BuiltinFunction::Clamp => Self::Unknown,
        }
    }
}
