use crate::core::traits::MathScalar;

/// Here to remenber to implement properly when adding proper imaginary support
/// Exponential function for polar representation
///
/// Currently just wraps `exp()`. This function exists as a placeholder
/// for potential future polar-form exponential implementations.
pub fn eval_exp_polar<T: MathScalar>(x: T) -> T {
    x.exp()
}
