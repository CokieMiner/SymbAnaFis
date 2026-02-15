#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::float_cmp,
    reason = "Fuzz-style numeric tests use tolerant comparisons and controlled casts"
)]

use crate::Dual;
use crate::math::{
    bessel_i, bessel_j, bessel_k, bessel_y, eval_assoc_legendre, eval_elliptic_e, eval_elliptic_k,
    eval_gamma, eval_hermite, eval_lambert_w, eval_spherical_harmonic,
};
use num_traits::Float;
use rand::{RngExt, SeedableRng, rngs::StdRng};

fn random_std_rng() -> StdRng {
    StdRng::seed_from_u64(rand::random())
}

fn approx_eq(a: f64, b: f64, rel: f64, abs: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_infinite() || b.is_infinite() {
        return a == b;
    }
    let diff = (a - b).abs();
    diff <= abs || diff <= rel * a.abs().max(b.abs()).max(1.0)
}

fn finite_diff_opt<F>(f: F, x: f64, h: f64) -> Option<f64>
where
    F: Fn(f64) -> Option<f64>,
{
    let f_plus = f(x + h)?;
    let f_minus = f(x - h)?;
    Some((f_plus - f_minus) / (2.0 * h))
}

#[test]
fn fuzz_bessel_j_parity_and_recurrence() {
    let mut rng = random_std_rng();
    for _ in 0..400 {
        let n: i32 = rng.random_range(0..=8);
        let mut x: f64 = rng.random_range(-20.0..20.0);
        if x.abs() < 0.25 {
            x = if x.is_sign_negative() { -0.25 } else { 0.25 };
        }

        let jn = bessel_j(n, x);
        let jneg = bessel_j(-n, x);
        if let (Some(jn_val), Some(jneg_val)) = (jn, jneg) {
            let sign = if n % 2 == 0 { 1.0 } else { -1.0 };
            assert!(
                approx_eq(jneg_val, sign * jn_val, 5e-7, 1e-10),
                "J parity failed: n={n}, x={x}, Jn={jn_val}, Jneg={jneg_val}"
            );
        }

        if n > 0 {
            let jm1 = bessel_j(n - 1, x);
            let jp1 = bessel_j(n + 1, x);
            if let (Some(jm1_val), Some(jn_val), Some(jp1_val)) = (jm1, jn, jp1) {
                let lhs = jp1_val - ((2.0 * f64::from(n) / x) * jn_val - jm1_val);
                assert!(
                    approx_eq(lhs, 0.0, 2e-6, 1e-8),
                    "J recurrence failed: n={n}, x={x}, residual={lhs}"
                );
            }
        }
    }
}

#[test]
fn fuzz_bessel_y_i_k_recurrence_and_domain() {
    let mut rng = random_std_rng();
    for _ in 0..300 {
        let n: i32 = rng.random_range(1..=6);
        let x: f64 = rng.random_range(0.35..20.0);

        let ym1 = bessel_y(n - 1, x);
        let yn = bessel_y(n, x);
        let yp1 = bessel_y(n + 1, x);
        if let (Some(ym1_val), Some(yn_val), Some(yp1_val)) = (ym1, yn, yp1) {
            let lhs = yp1_val - ((2.0 * f64::from(n) / x) * yn_val - ym1_val);
            assert!(
                approx_eq(lhs, 0.0, 1e-6, 1e-8),
                "Y recurrence failed: n={n}, x={x}, residual={lhs}"
            );
        }

        let i_n = bessel_i(n, x);
        let i_neg_n = bessel_i(-n, x);
        assert!(
            i_n.is_finite() && i_n >= 0.0,
            "I_n should be finite and non-negative for x>0: n={n}, x={x}, value={i_n}"
        );
        assert!(
            approx_eq(i_n, i_neg_n, 5e-7, 1e-10),
            "I parity failed: n={n}, x={x}, I_n={i_n}, I_-n={i_neg_n}"
        );

        let km1 = bessel_k(n - 1, x);
        let k_n = bessel_k(n, x);
        let kp1 = bessel_k(n + 1, x);
        if let (Some(km1_val), Some(kn_val), Some(kp1_val)) = (km1, k_n, kp1) {
            let lhs = kp1_val - (km1_val + (2.0 * f64::from(n) / x) * kn_val);
            assert!(
                approx_eq(lhs, 0.0, 2e-5, 1e-8),
                "K recurrence failed: n={n}, x={x}, residual={lhs}"
            );
        }

        let x_bad: f64 = -rng.random_range(0.1..10.0);
        assert!(
            bessel_y(n, x_bad).is_none(),
            "Y domain check failed for x={x_bad}"
        );
        assert!(
            bessel_k(n, x_bad).is_none(),
            "K domain check failed for x={x_bad}"
        );
    }
}

#[test]
fn fuzz_elliptic_symmetry_and_domain_edges() {
    let mut rng = random_std_rng();
    let half_pi = std::f64::consts::FRAC_PI_2;

    for _ in 0..400 {
        let k: f64 = rng.random_range(-0.95..0.95);
        let k_val = eval_elliptic_k(k);
        let k_neg_val = eval_elliptic_k(-k);
        let e_val = eval_elliptic_e(k);
        let e_neg_val = eval_elliptic_e(-k);

        if let (Some(k1), Some(k2), Some(e1), Some(e2)) = (k_val, k_neg_val, e_val, e_neg_val) {
            assert!(
                approx_eq(k1, k2, 1e-9, 1e-12),
                "K evenness failed for k={k}"
            );
            assert!(
                approx_eq(e1, e2, 1e-9, 1e-12),
                "E evenness failed for k={k}"
            );
            assert!(k1 >= half_pi - 1e-9, "K bound failed for k={k}: K={k1}");
            assert!(e1 <= half_pi + 1e-9, "E bound failed for k={k}: E={e1}");
            assert!(e1 > 0.0, "E positivity failed for k={k}: E={e1}");
        }
    }

    let k_at_one = eval_elliptic_k(1.0);
    assert!(
        k_at_one.is_some_and(f64::is_infinite),
        "Expected K(1) to be infinite"
    );
    let k_outside = eval_elliptic_k(1.2);
    assert!(
        k_outside.is_some_and(f64::is_infinite),
        "Expected K(|k|>1) to be infinite"
    );
    let e_outside = eval_elliptic_e(1.2);
    assert!(
        e_outside.is_some_and(f64::is_nan),
        "Expected E(|k|>1) to be NaN"
    );
}

#[test]
fn fuzz_polynomial_recurrence_and_domain_checks() {
    let mut rng = random_std_rng();

    for _ in 0..300 {
        let n: i32 = rng.random_range(1..15);
        let x: f64 = rng.random_range(-3.0..3.0);

        let hm1 = eval_hermite(n - 1, x);
        let h_n = eval_hermite(n, x);
        let hp1 = eval_hermite(n + 1, x);

        if let (Some(hm1_val), Some(hn_val), Some(hp1_val)) = (hm1, h_n, hp1) {
            let lhs = hp1_val - (2.0 * x * hn_val - 2.0 * f64::from(n) * hm1_val);
            assert!(
                approx_eq(lhs, 0.0, 1e-8, 1e-8),
                "Hermite recurrence failed: n={n}, x={x}, residual={lhs}"
            );
        }

        let l: i32 = rng.random_range(1..10);
        let x_leg: f64 = rng.random_range(-0.95..0.95);
        let plm1 = eval_assoc_legendre(l - 1, 0, x_leg);
        let pl = eval_assoc_legendre(l, 0, x_leg);
        let plp1 = eval_assoc_legendre(l + 1, 0, x_leg);

        if let (Some(plm1_val), Some(pl_val), Some(plp1_val)) = (plm1, pl, plp1) {
            let lhs = f64::from(l + 1) * plp1_val;
            let rhs = f64::from(2 * l + 1) * x_leg * pl_val - f64::from(l) * plm1_val;
            assert!(
                approx_eq(lhs, rhs, 2e-6, 2e-8),
                "Legendre recurrence failed: l={l}, x={x_leg}, lhs={lhs}, rhs={rhs}"
            );
        }

        let m_bad = l + 1;
        assert!(
            eval_assoc_legendre(l, m_bad, x_leg).is_none(),
            "Assoc Legendre should fail for |m|>l"
        );
        assert!(
            eval_assoc_legendre(l, 0, 1.1).is_none(),
            "Assoc Legendre should fail for |x|>1"
        );
    }

    for _ in 0..200 {
        let l: i32 = rng.random_range(0..=6);
        let m: i32 = rng.random_range(-l..=l);
        let theta: f64 = rng.random_range(0.0..std::f64::consts::PI);
        let phi: f64 = rng.random_range(-std::f64::consts::PI..std::f64::consts::PI);

        let ylm = eval_spherical_harmonic(l, m, theta, phi);
        assert!(
            ylm.is_some_and(f64::is_finite),
            "Spherical harmonic should be finite: l={l}, m={m}, theta={theta}, phi={phi}"
        );

        assert!(
            eval_spherical_harmonic(l, l + 1, theta, phi).is_none(),
            "Spherical harmonic should reject |m|>l"
        );
    }
}

#[test]
fn fuzz_dual_composed_derivative_vs_finite_difference() {
    let mut rng = random_std_rng();
    let h = 1e-6;

    let f = |x: f64| -> f64 { x.sin() * x.exp() + x * x * x + (x + 2.5).ln() };

    for _ in 0..300 {
        let x: f64 = rng.random_range(-2.0..3.0);
        let dx = Dual::new(x, 1.0);
        let y = dx.sin() * dx.exp() + dx * dx * dx + (dx + Dual::constant(2.5)).ln();

        let fd = (f(x + h) - f(x - h)) / (2.0 * h);
        assert!(
            approx_eq(y.eps, fd, 8e-5, 1e-6),
            "Dual derivative mismatch: x={x}, dual={}, fd={fd}",
            y.eps
        );
    }
}

#[test]
fn fuzz_dual_special_derivatives() {
    let mut rng = random_std_rng();

    for _ in 0..150 {
        let n: i32 = rng.random_range(0..=6);
        let x_b: f64 = rng.random_range(0.3..12.0);
        let dual_bessel = Dual::new(x_b, 1.0).bessel_j(n);
        let fd_bessel = finite_diff_opt(|t| bessel_j(n, t), x_b, 1e-6);
        if let (Some(d), Some(fd)) = (dual_bessel, fd_bessel) {
            assert!(
                approx_eq(d.eps, fd, 2e-4, 1e-6),
                "Dual Bessel derivative mismatch: n={n}, x={x_b}, dual={}, fd={fd}",
                d.eps
            );
        }

        let k: f64 = rng.random_range(-0.85..0.85);
        let dual_k = Dual::new(k, 1.0).elliptic_k();
        let fd_k = finite_diff_opt(eval_elliptic_k, k, 1e-6);
        if let (Some(dk), Some(fd)) = (dual_k, fd_k) {
            assert!(
                approx_eq(dk.eps, fd, 2e-4, 1e-6),
                "Dual elliptic_k derivative mismatch: k={k}, dual={}, fd={fd}",
                dk.eps
            );
        }

        let dual_e = Dual::new(k, 1.0).elliptic_e();
        let fd_e = finite_diff_opt(eval_elliptic_e, k, 1e-6);
        if let (Some(de), Some(fd)) = (dual_e, fd_e) {
            assert!(
                approx_eq(de.eps, fd, 2e-4, 1e-6),
                "Dual elliptic_e derivative mismatch: k={k}, dual={}, fd={fd}",
                de.eps
            );
        }

        let x_w: f64 = rng.random_range(-0.35..8.0);
        let dual_w = Dual::new(x_w, 1.0).lambert_w();
        let fd_w = finite_diff_opt(eval_lambert_w, x_w, 1e-6);
        if let (Some(dw), Some(fd)) = (dual_w, fd_w) {
            assert!(
                approx_eq(dw.eps, fd, 3e-4, 1e-6),
                "Dual lambert_w derivative mismatch: x={x_w}, dual={}, fd={fd}",
                dw.eps
            );
        }

        let x_g: f64 = rng.random_range(0.5..8.0);
        let dual_g = Dual::new(x_g, 1.0).gamma();
        let fd_g = finite_diff_opt(eval_gamma, x_g, 1e-6);
        if let (Some(dg), Some(fd)) = (dual_g, fd_g) {
            assert!(
                approx_eq(dg.eps, fd, 5e-4, 1e-6),
                "Dual gamma derivative mismatch: x={x_g}, dual={}, fd={fd}",
                dg.eps
            );
        }
    }
}
