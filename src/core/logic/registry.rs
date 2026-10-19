//! Immutable builtin arities and representation-aware construction rules.

use num_anafis::{Number, Real};

use super::{BuiltinFunction, Expr, ExprView, NumericValue};

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
    Floor,
    Ceil,
    Round,
    Trunc,
    Fract,
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
    Atan2,
    Beta,
    Bessel,
    Hermite,
    LambertW,
}

impl FunctionDefinition {
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

    /// Preserve proved exact cells, or evaluate finite approximate literals.
    /// Sealed constructors supply the builtin's validated argument count.
    /// Exact inputs accept only integer or rational result components; floating
    /// results remain symbolic even when their rounded value is integral.
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
        exact
            .filter(|value| {
                value.extract_complex().is_ok_and(|parts| {
                    matches!(parts.re, Real::Int(_) | Real::Rational(_))
                        && matches!(parts.im, Real::Int(_) | Real::Rational(_))
                })
            })
            .or_else(|| NumericValue::evaluate_approximate(self.builtin, arguments))
    }

    /// Select the canonical logarithm for a manifest exact base.
    pub fn logarithm_alias(base: &Expr) -> Option<BuiltinFunction> {
        if matches!(base.view(), ExprView::Euler) {
            return Some(BuiltinFunction::Ln);
        }
        let parts = base.as_number()?.extract_complex().ok()?;
        if parts.im.is_float() || !parts.im.is_zero() {
            return None;
        }
        match parts.re {
            Real::Int(2) => Some(BuiltinFunction::Log2),
            Real::Int(10) => Some(BuiltinFunction::Log10),
            Real::Int(_) | Real::Rational(_) | Real::Float(_) | _ => None,
        }
    }

    /// The builtin's fixed argument count.
    pub const fn arity(&self) -> usize {
        self.arity
    }

    /// Read the static identity used by sealed domain analysis.
    pub const fn builtin(&self) -> BuiltinFunction {
        self.builtin
    }
}
