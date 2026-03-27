//! API surface for the `math` module.

// Public to external library users
pub use super::logic::Dual;

// Crate-internal numerical entry points used by sibling modules.
pub use super::logic::{
    bessel_i, bessel_j, bessel_k, bessel_y, eval_assoc_legendre, eval_beta, eval_digamma,
    eval_elliptic_e, eval_elliptic_k, eval_erf, eval_erfc, eval_exp_polar, eval_gamma,
    eval_hermite, eval_lambert_w, eval_lgamma, eval_polygamma, eval_spherical_harmonic,
    eval_tetragamma, eval_trigamma, eval_zeta, eval_zeta_deriv,
};
