//! Finite approximate folding through shared numerical function kernels.

use num_anafis::Number;

use super::{BuiltinFunction, Kernel, NumericValue, Scalar};

impl NumericValue {
    /// Evaluate finite literal arguments when at least one component is approximate.
    /// Nonfinite results retain their symbolic function call.
    ///
    /// # Panics
    /// Propagates numerical backend capacity panics.
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
            (Kernel::Quaternary(kernel), [first, second, third, fourth]) => {
                kernel(&first.0, &second.0, &third.0, &fourth.0)
            }
            _ => return None,
        };
        value.is_finite().then_some(value)
    }
}
