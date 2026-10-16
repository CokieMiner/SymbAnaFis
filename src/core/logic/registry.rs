//! Immutable builtin arities and representation-aware construction rules.

use num_anafis::Number;

use super::{BuiltinFunction, NumericValue};

/// A builtin's identity, arity, and exact constructor behavior.
#[derive(Clone, Copy, Debug)]
pub struct FunctionDefinition {
    builtin: BuiltinFunction,
    arity: usize,
    unary: Option<UnaryRule>,
    binary: Option<BinaryRule>,
}

/// Exact unary cells; this is an algebraic rule, not a numerical approximation.
#[derive(Clone, Copy, Debug)]
pub enum UnaryRule {
    Identity,
    Zero,
    Negate,
    Abs,
    Sign,
    Sqrt,
    Cbrt,
    AtZero(i64),
    AtOneZero,
    Lgamma,
    Gamma,
    Log(i64),
}

/// Exact binary cells in the registered argument order.
#[derive(Clone, Copy, Debug)]
pub enum BinaryRule {
    Log,
    LogBase,
    Atan2,
    Beta,
    Bessel,
    Hermite,
    LambertW,
}

impl FunctionDefinition {
    /// The builtin's fixed argument count.
    pub const fn arity(&self) -> usize {
        self.arity
    }

    /// Preserve proved exact cells, or evaluate finite approximate literals.
    /// Sealed constructors supply the builtin's validated argument count.
    ///
    /// # Panics
    /// Numeric evaluation can propagate the backend's rational capacity panics.
    pub fn fold(&self, arguments: &[&NumericValue]) -> Option<Number> {
        let exact = match arguments {
            [argument] => self.unary.and_then(|rule| argument.evaluate(rule)),
            [left, right] => self
                .binary
                .and_then(|rule| left.evaluate_binary(rule, right)),
            _ => None,
        };
        exact.or_else(|| NumericValue::evaluate_approximate(self.builtin, arguments))
    }

    /// Define a builtin identity, arity, and optional exact arithmetic cells.
    pub const fn new(
        builtin: BuiltinFunction,
        arity: usize,
        unary: Option<UnaryRule>,
        binary: Option<BinaryRule>,
    ) -> Self {
        Self {
            builtin,
            arity,
            unary,
            binary,
        }
    }
}
