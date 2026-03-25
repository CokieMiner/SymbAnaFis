use super::super::super::instruction::FnOp;
use crate::core::known_symbols::KS;
use rustc_hash::FxHashMap;
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
    m.insert(ks.abs, f64::abs as ConstFoldFn);
    m.insert(ks.floor, f64::floor as ConstFoldFn);
    m.insert(ks.ceil, f64::ceil as ConstFoldFn);
    m.insert(ks.round, f64::round as ConstFoldFn);
    m
});
