use super::FnOp;
use crate::EPSILON;
use crate::core::known_symbols::KS;
use crate::math::{
    eval_digamma, eval_elliptic_e, eval_elliptic_k, eval_erf, eval_erfc, eval_exp_polar,
    eval_gamma, eval_lambert_w, eval_lgamma, eval_tetragamma, eval_trigamma, eval_zeta,
};
use rustc_hash::FxHashMap;
use std::f64::consts::FRAC_PI_2;
use std::sync::LazyLock;

pub type ConstFoldFn = fn(f64) -> f64;

pub static FN_MAP: LazyLock<FxHashMap<u64, FnOp>> = LazyLock::new(|| {
    let mut m = FxHashMap::default();
    let ks = &KS;
    // Arity 1
    m.insert(ks.sin, FnOp::Sin);
    m.insert(ks.cos, FnOp::Cos);
    m.insert(ks.tan, FnOp::Tan);
    m.insert(ks.cot, FnOp::Cot);
    m.insert(ks.sec, FnOp::Sec);
    m.insert(ks.csc, FnOp::Csc);
    m.insert(ks.asin, FnOp::Asin);
    m.insert(ks.acos, FnOp::Acos);
    m.insert(ks.atan, FnOp::Atan);
    m.insert(ks.acot, FnOp::Acot);
    m.insert(ks.asec, FnOp::Asec);
    m.insert(ks.acsc, FnOp::Acsc);
    m.insert(ks.sinh, FnOp::Sinh);
    m.insert(ks.cosh, FnOp::Cosh);
    m.insert(ks.tanh, FnOp::Tanh);
    m.insert(ks.coth, FnOp::Coth);
    m.insert(ks.sech, FnOp::Sech);
    m.insert(ks.csch, FnOp::Csch);
    m.insert(ks.asinh, FnOp::Asinh);
    m.insert(ks.acosh, FnOp::Acosh);
    m.insert(ks.atanh, FnOp::Atanh);
    m.insert(ks.acoth, FnOp::Acoth);
    m.insert(ks.acsch, FnOp::Acsch);
    m.insert(ks.asech, FnOp::Asech);
    m.insert(ks.exp, FnOp::Exp);
    m.insert(ks.ln, FnOp::Ln);
    m.insert(ks.sqrt, FnOp::Sqrt);
    m.insert(ks.cbrt, FnOp::Cbrt);
    m.insert(ks.abs, FnOp::Abs);
    m.insert(ks.signum, FnOp::Signum);
    m.insert(ks.sign, FnOp::Signum);
    m.insert(ks.sgn, FnOp::Signum);
    m.insert(ks.floor, FnOp::Floor);
    m.insert(ks.ceil, FnOp::Ceil);
    m.insert(ks.round, FnOp::Round);
    m.insert(ks.erf, FnOp::Erf);
    m.insert(ks.erfc, FnOp::Erfc);
    m.insert(ks.gamma, FnOp::Gamma);
    m.insert(ks.lgamma, FnOp::Lgamma);
    m.insert(ks.digamma, FnOp::Digamma);
    m.insert(ks.trigamma, FnOp::Trigamma);
    m.insert(ks.tetragamma, FnOp::Tetragamma);
    m.insert(ks.sinc, FnOp::Sinc);
    m.insert(ks.lambertw, FnOp::LambertW);
    m.insert(ks.elliptic_k, FnOp::EllipticK);
    m.insert(ks.elliptic_e, FnOp::EllipticE);
    m.insert(ks.zeta, FnOp::Zeta);
    m.insert(ks.exp_polar, FnOp::ExpPolar);

    // Arity 2
    m.insert(ks.atan2, FnOp::Atan2);
    m.insert(ks.log, FnOp::Log);
    m.insert(ks.besselj, FnOp::BesselJ);
    m.insert(ks.bessely, FnOp::BesselY);
    m.insert(ks.besseli, FnOp::BesselI);
    m.insert(ks.besselk, FnOp::BesselK);
    m.insert(ks.polygamma, FnOp::Polygamma);
    m.insert(ks.beta, FnOp::Beta);
    m.insert(ks.zeta_deriv, FnOp::ZetaDeriv);
    m.insert(ks.hermite, FnOp::Hermite);

    // Arity 3
    m.insert(ks.assoc_legendre, FnOp::AssocLegendre);

    // Arity 4
    m.insert(ks.spherical_harmonic, FnOp::SphericalHarmonic);
    m.insert(ks.ynm, FnOp::SphericalHarmonic);

    m
});

pub static CONST_FOLD_MAP: LazyLock<FxHashMap<u64, ConstFoldFn>> = LazyLock::new(|| {
    let mut m = FxHashMap::default();
    let ks = &KS;
    m.insert(ks.sin, f64::sin as ConstFoldFn);
    m.insert(ks.cos, f64::cos as ConstFoldFn);
    m.insert(ks.tan, f64::tan as ConstFoldFn);
    m.insert(ks.asin, f64::asin as ConstFoldFn);
    m.insert(ks.acos, f64::acos as ConstFoldFn);
    m.insert(ks.atan, f64::atan as ConstFoldFn);
    m.insert(ks.sinh, f64::sinh as ConstFoldFn);
    m.insert(ks.cosh, f64::cosh as ConstFoldFn);
    m.insert(ks.tanh, f64::tanh as ConstFoldFn);
    m.insert(ks.exp, f64::exp as ConstFoldFn);
    m.insert(ks.ln, f64::ln as ConstFoldFn);
    m.insert(ks.log, f64::ln as ConstFoldFn);
    m.insert(ks.sqrt, f64::sqrt as ConstFoldFn);
    m.insert(ks.cbrt, f64::cbrt as ConstFoldFn);
    m.insert(ks.abs, f64::abs as ConstFoldFn);
    m.insert(ks.floor, f64::floor as ConstFoldFn);
    m.insert(ks.ceil, f64::ceil as ConstFoldFn);
    m.insert(ks.round, f64::round as ConstFoldFn);
    m.insert(ks.signum, f64::signum as ConstFoldFn);
    m.insert(ks.sign, f64::signum as ConstFoldFn);
    m.insert(ks.sgn, f64::signum as ConstFoldFn);
    m.insert(ks.log2, f64::log2 as ConstFoldFn);
    m.insert(ks.log10, f64::log10 as ConstFoldFn);

    // Additional secondary trig and hyperbolic functions mapped to avoid CONST_FOLD_MAP discrepancy
    m.insert(ks.cot, (|v: f64| v.tan().recip()) as ConstFoldFn);
    m.insert(ks.sec, (|v: f64| v.cos().recip()) as ConstFoldFn);
    m.insert(ks.csc, (|v: f64| v.sin().recip()) as ConstFoldFn);
    m.insert(ks.acot, (|v: f64| FRAC_PI_2 - v.atan()) as ConstFoldFn);
    m.insert(ks.asec, (|v: f64| (1.0 / v).acos()) as ConstFoldFn);
    m.insert(ks.acsc, (|v: f64| (1.0 / v).asin()) as ConstFoldFn);
    m.insert(ks.coth, (|v: f64| v.tanh().recip()) as ConstFoldFn);
    m.insert(ks.sech, (|v: f64| v.cosh().recip()) as ConstFoldFn);
    m.insert(ks.csch, (|v: f64| v.sinh().recip()) as ConstFoldFn);
    m.insert(ks.asinh, f64::asinh as ConstFoldFn);
    m.insert(ks.acosh, f64::acosh as ConstFoldFn);
    m.insert(ks.atanh, f64::atanh as ConstFoldFn);
    m.insert(
        ks.acoth,
        (|v: f64| 0.5 * ((v + 1.0) / (v - 1.0)).ln()) as ConstFoldFn,
    );
    m.insert(
        ks.acsch,
        (|v: f64| (1.0 / v + v.mul_add(v, 1.0).sqrt() / v.abs()).ln()) as ConstFoldFn,
    );
    m.insert(
        ks.asech,
        (|v: f64| ((1.0 + v.mul_add(-v, 1.0).sqrt()) / v).ln()) as ConstFoldFn,
    );
    m.insert(
        ks.sinc,
        (|v: f64| if v.abs() < EPSILON { 1.0 } else { v.sin() / v }) as ConstFoldFn,
    );

    // Special functions via math crate
    m.insert(ks.erf, eval_erf::<f64> as ConstFoldFn);
    m.insert(ks.erfc, eval_erfc::<f64> as ConstFoldFn);
    m.insert(ks.gamma, eval_gamma::<f64> as ConstFoldFn);
    m.insert(ks.lgamma, eval_lgamma::<f64> as ConstFoldFn);
    m.insert(ks.digamma, eval_digamma::<f64> as ConstFoldFn);
    m.insert(ks.trigamma, eval_trigamma::<f64> as ConstFoldFn);
    m.insert(ks.tetragamma, eval_tetragamma::<f64> as ConstFoldFn);
    m.insert(ks.lambertw, eval_lambert_w::<f64> as ConstFoldFn);
    m.insert(ks.elliptic_k, eval_elliptic_k::<f64> as ConstFoldFn);
    m.insert(ks.elliptic_e, eval_elliptic_e::<f64> as ConstFoldFn);
    m.insert(ks.zeta, eval_zeta::<f64> as ConstFoldFn);
    m.insert(ks.exp_polar, eval_exp_polar::<f64> as ConstFoldFn);
    m
});
