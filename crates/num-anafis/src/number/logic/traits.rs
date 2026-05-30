//! The `Number` trait — single source of truth for all mathematical operations
//! that a scalar type must implement.
//!
//! The `define_number_trait!` macro generates the trait definition from a list
//! of unary and binary function names. Any implementing type that forgets a
//! method gets an immediate compile error.

use core::cmp::Ordering;
use core::fmt::{Debug, Display};
use core::hash::Hash;
use core::ops::{Add, Div, Mul, Neg, Sub};

/// Generates the `Number` trait with all required mathematical operations.
///
/// Unary functions are listed as `method_name,` and the macro generates
/// a `fn method_name(&self) -> Self` for each.
///
/// Binary and multi-arg functions are defined manually in the trait body
/// because they have varying signatures.
macro_rules! define_number_trait {
    (
        unary { $( $method:ident, )* }
    ) => {
        /// Trait that all number types must implement.
        ///
        /// This is the compile-time contract for every backend. If a backend
        /// is selected and the implementing type is missing any method, the
        /// build fails immediately.
        #[allow(clippy::module_name_repetitions, reason = "Trait name is the canonical identifier")]
        pub trait Number:
            Sized
            + Clone
            + Debug
            + Display
            + PartialEq
            + PartialOrd
            + Hash
            + Add<Output = Self>
            + for<'num> Add<&'num Self, Output = Self>
            + Sub<Output = Self>
            + for<'num> Sub<&'num Self, Output = Self>
            + Mul<Output = Self>
            + for<'num> Mul<&'num Self, Output = Self>
            + Div<Output = Self>
            + for<'num> Div<&'num Self, Output = Self>
            + Neg<Output = Self>
        {
            // =================================================================
            // Unary functions (macro-generated)
            // =================================================================
            $(
                /// Unary mathematical function.
                #[must_use]
                fn $method(&self) -> Self;
            )*

            // =================================================================
            // Binary / multi-arg functions (manually defined)
            // =================================================================

            /// Two-argument arctangent: `atan2(y, x)` where `self` is `y`.
            #[must_use]
            fn atan2(&self, x: &Self) -> Self;

            /// Logarithm with explicit base: `log_base(self, base)`.
            #[must_use]
            fn log_base(&self, base: &Self) -> Self;

            /// Power: `self^exp`.
            #[must_use]
            fn pow(&self, exp: &Self) -> Self;

            /// Bessel function of the first kind: `J_order(self)`.
            #[must_use]
            fn besselj(&self, order: &Self) -> Self;

            /// Bessel function of the second kind: `Y_order(self)`.
            #[must_use]
            fn bessely(&self, order: &Self) -> Self;

            /// Modified Bessel function of the first kind: `I_order(self)`.
            #[must_use]
            fn besseli(&self, order: &Self) -> Self;

            /// Modified Bessel function of the second kind: `K_order(self)`.
            #[must_use]
            fn besselk(&self, order: &Self) -> Self;

            /// Polygamma function: `ψ^(order)(self)`.
            #[must_use]
            fn polygamma(&self, order: &Self) -> Self;

            /// Beta function: `B(self, other)`.
            #[must_use]
            fn beta(&self, other: &Self) -> Self;

            /// Derivative of the Riemann zeta function.
            #[must_use]
            fn zeta_deriv(&self, order: &Self) -> Self;

            /// Lambert W function on branch n: `W_n(self)`.
            #[must_use]
            fn lambertw(&self, n: &Self) -> Self;

            /// Hermite polynomial: `H_n(self)`.
            #[must_use]
            fn hermite(&self, n: &Self) -> Self;

            /// Associated Legendre polynomial: `P_l^m(self)`.
            #[must_use]
            fn assoc_legendre(&self, l: &Self, m: &Self) -> Self;

            /// Spherical harmonic: `Y_l^m(theta, phi)`.
            #[must_use]
            fn spherical_harmonic(&self, l: &Self, m: &Self, phi: &Self) -> Self;

            // =================================================================
            // Core properties
            // =================================================================

            /// Check whether this value is numerically zero.
            #[must_use]
            fn is_zero(&self) -> bool;

            /// Check whether this value is numerically one.
            #[must_use]
            fn is_one(&self) -> bool;

            /// Check whether this value is numerically negative one.
            #[must_use]
            fn is_neg_one(&self) -> bool;

            /// Check whether this value is an exact integer (no fractional part).
            #[must_use]
            fn is_integer(&self) -> bool;

            /// Check whether this value is negative.
            #[must_use]
            fn is_negative(&self) -> bool;

            /// Check whether this value is positive.
            #[must_use]
            fn is_positive(&self) -> bool;

            /// Check whether this value is finite.
            #[must_use]
            fn is_finite(&self) -> bool;

            /// Force conversion to the backend's float representation.
            #[must_use]
            fn to_float(&self) -> Self;

            /// Approximate comparison with tolerance.
            #[must_use]
            fn approx_eq_number(&self, other: &Self, tolerance: &Self) -> bool;

            /// Compare with total ordering (NaN compares greater than all finite values).
            #[must_use]
            fn total_cmp(&self, other: &Self) -> Ordering;

            /// IEEE 754 maxNum: NaN propagation — if either is NaN, returns NaN.
            #[must_use]
            fn num_max(&self, other: &Self) -> Self;

            /// IEEE 754 minNum: NaN propagation — if either is NaN, returns NaN.
            #[must_use]
            fn num_min(&self, other: &Self) -> Self;
        }
    };
}

define_number_trait! {
    unary {
        // --- Trigonometric ---
        sin,
        cos,
        tan,
        cot,
        sec,
        csc,

        // --- Inverse Trigonometric ---
        asin,
        acos,
        atan,
        acot,
        asec,
        acsc,

        // --- Hyperbolic ---
        sinh,
        cosh,
        tanh,
        coth,
        sech,
        csch,

        // --- Inverse Hyperbolic ---
        asinh,
        acosh,
        atanh,
        acoth,
        acsch,
        asech,

        // --- Exponential & Logarithmic ---
        exp,
        expm1,
        exp_neg,
        ln,
        log1p,

        // --- Powers & Roots ---
        sqrt,
        cbrt,

        // --- Basic Math ---
        abs,
        signum,
        floor,
        ceil,
        round,
        fract,
        negate,

        // --- Special Functions (Unary) ---
        erf,
        erfc,
        gamma,
        lgamma,
        digamma,
        trigamma,
        tetragamma,
        sinc,
        elliptic_k,
        elliptic_e,
        zeta,
        exp_polar,
    }
}
