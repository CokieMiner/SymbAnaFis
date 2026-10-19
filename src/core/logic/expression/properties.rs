//! Conservative finite-scalar proofs over the stored DAG.

use core::iter::once;

use alloc::vec;

use num_anafis::Number;

use crate::collections::HashMap;

use super::{BuiltinFunction, Expr, FunctionDefinition, Node};

/// The result of a conservative mathematical property query.
/// Unknown facts do not authorize transformations requiring a proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Truth {
    /// The property holds for every admissible assignment.
    True,
    /// The property fails for every admissible assignment.
    False,
    /// The analysis cannot establish either result.
    Unknown,
}

/// Membership predicates for finite real and complex scalars.
///
/// Sign predicates require a finite real value; nonreal and undefined values
/// do not satisfy them. Nonzero means a finite complex value unequal to zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Property {
    /// A defined finite complex value.
    Finite,
    /// A finite real value.
    Real,
    /// A real integer value, independently of its literal representation.
    Integer,
    /// A real rational value.
    Rational,
    /// Exactly zero.
    Zero,
    /// A finite complex value unequal to zero.
    Nonzero,
    /// A strictly positive real value.
    Positive,
    /// A strictly negative real value.
    Negative,
    /// A nonnegative real value.
    Nonnegative,
    /// A nonpositive real value.
    Nonpositive,
}

/// Proven and refuted facts for one expression, computed in one DAG traversal.
///
/// Symbols range over finite complex scalars. No realness, sign, integrality,
/// or nonzero assumptions are attached to them. Facts describe mathematical
/// values; they do not predict overflow or rounding in numerical evaluation.
/// Unknown function domains remain unknown. Results carry no global cache and
/// cannot change expression identity or canonical construction.
/// Nonfinite inputs do not refute a composition's finite value without a domain
/// proof: extended values or limits may require additional rules.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Properties {
    facts: Facts,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Facts {
    proven: u16,
    refuted: u16,
}

enum PropertyFrame<'expr> {
    Visit(&'expr Expr),
    Evaluate(&'expr Expr),
}

impl Expr {
    /// Prove scalar properties conservatively in children-before-parent order.
    /// Shared nodes are analyzed once; compact polynomials are inspected without
    /// materializing their logical monomials. Unknown facts remain unknown.
    ///
    /// ```
    /// use symb_anafis::{Expr, Property, Truth};
    /// let x = Expr::symbol("properties_example_x");
    /// assert_eq!(x.properties().get(Property::Real), Truth::Unknown);
    /// assert_eq!(x.abs().properties().get(Property::Nonnegative), Truth::True);
    /// assert_eq!(x.exp().properties().get(Property::Positive), Truth::Unknown);
    /// ```
    #[must_use]
    pub fn properties(&self) -> Properties {
        let mut memo = HashMap::new();
        let mut pending = vec![PropertyFrame::Visit(self)];
        while let Some(frame) = pending.pop() {
            let expression = match frame {
                PropertyFrame::Visit(node) => {
                    if memo.contains_key(node) {
                        continue;
                    }
                    pending.push(PropertyFrame::Evaluate(node));
                    pending.extend(Node::children(node).map(PropertyFrame::Visit));
                    continue;
                }
                PropertyFrame::Evaluate(node) => node,
            };
            let get = |child: &Self| memo.get(child).copied().unwrap_or_default();
            let facts = match expression.node() {
                Node::Number(value) => Facts::literal(value.value()),
                Node::ImaginaryUnit => Facts::literal(&Number::i()),
                Node::Pi | Node::Euler => {
                    let mut facts = Facts::default();
                    facts.prove(Property::Positive);
                    facts.refute(Property::Rational);
                    facts
                }
                Node::Symbol(_) => {
                    let mut facts = Facts::default();
                    facts.prove(Property::Finite);
                    facts
                }
                Node::Sum(terms) => Facts::sum(terms.iter().map(get)),
                Node::Product(factors) => Facts::product(factors.iter().map(get)),
                Node::Div(numerator, denominator) => {
                    Facts::quotient(get(numerator), get(denominator))
                }
                Node::Pow(base, exponent) => {
                    Facts::power(get(base), get(exponent), exponent.as_number())
                }
                Node::Function { name, args } => Facts::function(
                    FunctionDefinition::get(name).map(FunctionDefinition::builtin),
                    args,
                    get,
                ),
                Node::Poly(polynomial) => Facts::sum(polynomial.terms.iter().map(|term| {
                    Facts::product(once(get(&term.coefficient)).chain(term.powers.iter().map(
                        |(base, exponent)| Facts::integer_power(get(base), i64::from(*exponent)),
                    )))
                })),
                Node::Derivative { .. } => Facts::default(),
            };
            memo.insert(expression, facts);
        }
        Properties {
            facts: memo.get(self).copied().unwrap_or_default(),
        }
    }
}

impl Properties {
    /// Read one proven, refuted, or unknown property without traversing the DAG.
    #[must_use]
    pub const fn get(self, property: Property) -> Truth {
        self.facts.get(property)
    }
}

impl Facts {
    const fn get(self, property: Property) -> Truth {
        let bit = property.bit();
        if self.proven & bit != 0 {
            Truth::True
        } else if self.refuted & bit != 0 {
            Truth::False
        } else {
            Truth::Unknown
        }
    }

    fn literal(value: &Number) -> Self {
        let mut facts = Self::default();
        if !value.is_finite() {
            facts.refute(Property::Finite);
            return facts;
        }
        facts.prove(Property::Finite);
        if !value.is_real() {
            facts.refute(Property::Real);
            facts.prove(Property::Nonzero);
            return facts;
        }
        facts.prove(Property::Real);
        facts.prove(Property::Rational);
        if value.is_int() {
            facts.prove(Property::Integer);
        } else {
            facts.refute(Property::Integer);
        }
        if value.is_zero() {
            facts.prove(Property::Zero);
        } else if value.is_positive() {
            facts.prove(Property::Positive);
        } else {
            facts.prove(Property::Negative);
        }
        facts
    }

    fn prove(&mut self, property: Property) {
        let implied = match property {
            Property::Positive => [
                Property::Finite,
                Property::Real,
                Property::Nonzero,
                Property::Nonnegative,
            ]
            .as_slice(),
            Property::Negative => [
                Property::Finite,
                Property::Real,
                Property::Nonzero,
                Property::Nonpositive,
            ]
            .as_slice(),
            Property::Zero => [
                Property::Finite,
                Property::Real,
                Property::Rational,
                Property::Integer,
                Property::Nonnegative,
                Property::Nonpositive,
            ]
            .as_slice(),
            Property::Integer => [Property::Finite, Property::Real, Property::Rational].as_slice(),
            Property::Rational | Property::Nonnegative | Property::Nonpositive => {
                [Property::Finite, Property::Real].as_slice()
            }
            Property::Real | Property::Nonzero => [Property::Finite].as_slice(),
            Property::Finite => [].as_slice(),
        };
        for fact in once(property).chain(implied.iter().copied()) {
            self.proven |= fact.bit();
        }
        let excluded = match property {
            Property::Positive => {
                [Property::Zero, Property::Negative, Property::Nonpositive].as_slice()
            }
            Property::Negative => {
                [Property::Zero, Property::Positive, Property::Nonnegative].as_slice()
            }
            Property::Zero => {
                [Property::Nonzero, Property::Positive, Property::Negative].as_slice()
            }
            Property::Nonzero => [Property::Zero].as_slice(),
            Property::Nonnegative => [Property::Negative].as_slice(),
            Property::Nonpositive => [Property::Positive].as_slice(),
            Property::Finite | Property::Real | Property::Integer | Property::Rational => {
                [].as_slice()
            }
        };
        for fact in excluded {
            self.refuted |= fact.bit();
        }
    }

    fn refute(&mut self, property: Property) {
        let excluded = match property {
            Property::Finite => [
                Property::Real,
                Property::Integer,
                Property::Rational,
                Property::Zero,
                Property::Nonzero,
                Property::Positive,
                Property::Negative,
                Property::Nonnegative,
                Property::Nonpositive,
            ]
            .as_slice(),
            Property::Real => [
                Property::Integer,
                Property::Rational,
                Property::Zero,
                Property::Positive,
                Property::Negative,
                Property::Nonnegative,
                Property::Nonpositive,
            ]
            .as_slice(),
            Property::Rational => [Property::Integer, Property::Zero].as_slice(),
            Property::Integer => [Property::Zero].as_slice(),
            Property::Zero
            | Property::Nonzero
            | Property::Positive
            | Property::Negative
            | Property::Nonnegative
            | Property::Nonpositive => [].as_slice(),
        };
        for fact in once(property).chain(excluded.iter().copied()) {
            self.refuted |= fact.bit();
        }
    }
}

impl Property {
    const fn bit(self) -> u16 {
        match self {
            Self::Finite => 1,
            Self::Real => 2,
            Self::Integer => 4,
            Self::Rational => 8,
            Self::Zero => 16,
            Self::Nonzero => 32,
            Self::Positive => 64,
            Self::Negative => 128,
            Self::Nonnegative => 256,
            Self::Nonpositive => 512,
        }
    }
}

mod arithmetic;
mod functions;
