#![allow(
    clippy::doc_markdown,
    reason = "LaTeX math notation in $...$ / $$...$$ is not recognized by clippy"
)]
//! Bessel functions (J, Y, I, K) and their asymptotic expansions.
use super::super::{BackingFloat, get_precision, nan};
use crate::number::logic::int_math::IntType;
use rug::{Float, Integer};

pub fn besselj(n: &IntType, value: &BackingFloat) -> BackingFloat {
    let Some(order) = n.to_i32() else {
        return nan();
    };
    value.clone().jn(order)
}

pub fn bessely(n: &IntType, value: &BackingFloat) -> BackingFloat {
    let Some(order) = n.to_i32() else {
        return nan();
    };
    value.clone().yn(order)
}

/// Computes the modified Bessel function of the first kind $I_n(x)$.
///
/// **Algorithm**:
/// - For $n=0,1$: power series or asymptotic series via `besseli{0,1}_with_prec`.
/// - For $n \ge 2, x \ge n$: Miller's forward recurrence (DLMF §10.29.1):
///   $$ I_{k-1}(x) = I_{k+1}(x) - \frac{2k}{x} I_k(x). $$
/// - For $n \ge 2, x < n$: Miller's backward recurrence (DLMF §10.74),
///   starting from a computed starting index $N$ where $I_N(x) \approx 0$,
///   then normalizing against a separately computed $I_0(x)$.
///
/// **Magic constants**:
/// - `work_prec = prec + n * clamp(50, 300)`: each forward recurrence step can
///   lose up to ~1 bit of relative accuracy; with $n$ up to 300, 300 guard bits
///   safely absorb all accumulated error.  The `n_abs * 1` scaling is a rough
///   model of the worst-case error growth in the forward recurrence.
///
/// **References**:
/// - DLMF §10.29.1 (forward recurrence), §10.74 (Miller's algorithm)
/// - Olver, F.W.J. (1997). *Asymptotics and Special Functions*, Ch. 10.
pub fn besseli(n: &IntType, v: &BackingFloat) -> BackingFloat {
    let prec = get_precision();
    let n_abs = Integer::from(n.clone().abs_ref());
    let is_neg = v.is_sign_negative();
    let v_abs = v.clone().abs();

    if n_abs.is_zero() {
        let res = besseli0_with_prec(&v_abs, prec + 20);
        return Float::with_val(prec, res);
    }
    if n_abs == 1 {
        let res = besseli1_with_prec(&v_abs, prec + 20);
        let final_res = if is_neg { -res } else { res };
        return Float::with_val(prec, final_res);
    }

    // Guard bits scale with n: running forward recurrence in the direction of the
    // decreasing solution (from I0, I1 to In) causes catastrophic cancellation.
    // The precision loss is proportional to log2(I0(x)/In(x)), which is bounded by
    // 1.5 * n for x > n. We allocate 2 * n + 80 guard bits to absorb all lost precision.
    let work_prec = prec
        .saturating_add(n_abs.to_u32().unwrap_or(80).saturating_mul(2))
        .saturating_add(80);
    let v_w = Float::with_val(work_prec, &v_abs);
    let tol = precision_threshold_with(work_prec);
    if v_w < tol {
        return Float::with_val(prec, 0);
    }

    let two = BackingFloat::with_val(work_prec, 2);

    let res = if v_w > Float::with_val(work_prec, &n_abs) {
        let i0 = besseli0_with_prec(&v_w, work_prec);
        let i1 = besseli1_with_prec(&v_w, work_prec);
        let mut i_prev = i0;
        let mut i_curr = i1;
        let mut k = Integer::from(1);
        let mut k_t = BackingFloat::with_val(work_prec, 1);

        while k < n_abs {
            let term = BackingFloat::with_val(
                work_prec,
                BackingFloat::with_val(work_prec, &two * &k_t) / &v_w * &i_curr,
            );
            let ik_plus_1 = BackingFloat::with_val(work_prec, &i_prev - term);
            i_prev = i_curr;
            i_curr = ik_plus_1;
            k_t = BackingFloat::with_val(work_prec, &k_t + 1);
            k += 1;
        }
        i_curr
    } else {
        let n_start = compute_i_start(&n_abs, &v_w, work_prec);

        let mut i_next = BackingFloat::with_val(work_prec, 0);
        let mut i_curr = precision_threshold_with(work_prec);
        let mut result = BackingFloat::with_val(work_prec, 0);
        let mut i0_unnorm = BackingFloat::with_val(work_prec, 0);

        let mut k = n_start;
        let mut k_t = BackingFloat::with_val(work_prec, &k);

        loop {
            let i_prev = BackingFloat::with_val(
                work_prec,
                BackingFloat::with_val(work_prec, &two * &k_t) / &v_w * &i_curr,
            ) + &i_next;

            if k == n_abs {
                result.clone_from(&i_curr);
            }

            if k.is_zero() {
                i0_unnorm.clone_from(&i_curr);
            }

            i_next = i_curr;
            i_curr = i_prev;
            if k.is_zero() {
                break;
            }
            k -= 1;
            k_t = BackingFloat::with_val(work_prec, &k_t - 1);
        }

        let i0_actual = besseli0_with_prec(&v_w, work_prec);
        let scale = BackingFloat::with_val(work_prec, &i0_actual / &i0_unnorm);
        Float::with_val(work_prec, result * scale)
    };

    if is_neg && n_abs.is_odd() {
        Float::with_val(prec, -res)
    } else {
        Float::with_val(prec, res)
    }
}

/// Computes the modified Bessel function of the second kind $K_n(x)$.
///
/// **Algorithm**: uses the forward recurrence (DLMF §10.29.1):
/// $$ K_{k+1}(x) = K_{k-1}(x) + \frac{2k}{x} K_k(x), $$
/// starting from $K_0$ and $K_1$ computed by `besselk0` / `besselk1`.
/// The forward recurrence for $K_n$ is numerically stable (no cancellation
/// because $K_n(x) > 0$ for $x > 0$).
///
/// **Magic constants**:
/// - `work_prec = prec + 20`: 20 guard bits absorb the recurrence
///   accumulation (each of the $n$ steps contributes < 1 ulp).
///
/// **Reference**: DLMF §10.29.1 (recurrence), §10.31 (series).
pub fn besselk(n: &IntType, v: &BackingFloat) -> BackingFloat {
    if v.is_sign_negative() || v.is_zero() {
        return nan();
    }
    let n_abs = Integer::from(n.clone().abs_ref());
    let prec = get_precision();
    let work_prec = prec + 20;

    let v_w = Float::with_val(work_prec, v);
    let k0 = besselk0(&v_w);
    if n_abs.is_zero() {
        return Float::with_val(prec, k0);
    }
    let k1 = besselk1(&v_w);
    if n_abs == 1 {
        return Float::with_val(prec, k1);
    }

    let two = BackingFloat::with_val(work_prec, 2);
    let (mut k_prev, mut k_curr) = (k0, k1);
    let mut k = Integer::from(1);
    let mut k_t = BackingFloat::with_val(work_prec, 1);

    while k < n_abs {
        let kn = BackingFloat::with_val(
            work_prec,
            &k_prev
                + BackingFloat::with_val(
                    work_prec,
                    BackingFloat::with_val(work_prec, &two * &k_t) / &v_w * &k_curr,
                ),
        );
        k_prev = k_curr;
        k_curr = kn;
        k_t = BackingFloat::with_val(work_prec, &k_t + 1);
        k += 1;
    }
    Float::with_val(prec, k_curr)
}

/// Asymptotic expansion of $I_0(x)$ for large $|x|$ (DLMF §10.40.1):
/// $$ I_0(x) \sim \frac{e^x}{\sqrt{2\pi x}} \Bigl[ 1 + \frac{1}{8x}
///    + \frac{9}{128x^2} + \cdots \Bigr]. $$
///
/// The series coefficients are $(2k-1)^2 / (8k)^k$.  Returns `None` if the series
/// starts to diverge (asymptotic series must be truncated at the optimal term).
fn besseli0_asymp(x: &Float, prec: u32) -> Option<Float> {
    let pi = Float::with_val(prec, rug::float::Constant::Pi);
    let two = Float::with_val(prec, 2);
    let prefactor_inner = Float::with_val(prec, &two * &pi);
    let prefactor_sqrt = Float::with_val(prec, Float::with_val(prec, &prefactor_inner * x).sqrt());
    let prefactor = Float::with_val(prec, x.clone().exp() / prefactor_sqrt);

    let mut sum = Float::with_val(prec, 1);
    let mut term = Float::with_val(prec, 1);
    let mut k: usize = 1;
    let tol = precision_threshold_with(prec);
    let mut prev_abs = Float::with_val(prec, 1);

    loop {
        let mut num = Integer::from(2 * k - 1);
        num = Integer::from(&num * &num);
        let denom = Float::with_val(prec, 8 * k) * x;
        let factor = Float::with_val(prec, num / denom);
        term = Float::with_val(prec, &term * factor);

        let term_abs = term.clone().abs();
        if term_abs < tol {
            sum = Float::with_val(prec, &sum + &term);
            break;
        }
        if term_abs >= prev_abs {
            return None;
        }
        sum = Float::with_val(prec, &sum + &term);
        prev_abs = term_abs;
        k += 1;
    }
    Some(Float::with_val(prec, sum * prefactor))
}

/// Asymptotic expansion of $I_1(x)$ for large $|x|$ (DLMF §10.40.1):
/// $$ I_1(x) \sim \frac{e^x}{\sqrt{2\pi x}} \Bigl[ 1 - \frac{3}{8x}
///    - \frac{15}{128x^2} - \cdots \Bigr]. $$
///
/// Coefficients: $\bigl((2k-1)^2 - 4\bigr) / (8k)^k$.
fn besseli1_asymp(x: &Float, prec: u32) -> Option<Float> {
    let pi = Float::with_val(prec, rug::float::Constant::Pi);
    let two = Float::with_val(prec, 2);
    let prefactor_inner = Float::with_val(prec, &two * &pi);
    let prefactor_sqrt = Float::with_val(prec, Float::with_val(prec, &prefactor_inner * x).sqrt());
    let prefactor = Float::with_val(prec, x.clone().exp() / prefactor_sqrt);

    let mut sum = Float::with_val(prec, 1);
    let mut term = Float::with_val(prec, 1);
    let mut k: usize = 1;
    let tol = precision_threshold_with(prec);
    let mut prev_abs = Float::with_val(prec, 1);

    loop {
        let mut num = Integer::from(2 * k - 1);
        num = Integer::from(&num * &num) - 4;
        let denom = Float::with_val(prec, 8 * k) * x;
        let factor = Float::with_val(prec, num / denom);
        term = Float::with_val(prec, &term * factor);

        let term_abs = term.clone().abs();
        if term_abs < tol {
            sum = Float::with_val(prec, &sum + &term);
            break;
        }
        if term_abs >= prev_abs {
            return None;
        }
        sum = Float::with_val(prec, &sum + &term);
        prev_abs = term_abs;
        k += 1;
    }
    Some(Float::with_val(prec, sum * prefactor))
}

/// Asymptotic expansion of $K_0(x)$ for large $|x|$ (DLMF §10.40.2):
/// $$ K_0(x) \sim \sqrt{\frac{\pi}{2x}} e^{-x} \Bigl[ 1 - \frac{1}{8x}
///    + \frac{9}{128x^2} - \cdots \Bigr]. $$
///
/// The series alternates in sign; coefficients are $(-1)^k (2k-1)^2 / (8k)^k$.
fn besselk0_asymp(x: &Float, prec: u32) -> Option<Float> {
    let pi = Float::with_val(prec, rug::float::Constant::Pi);
    let two = Float::with_val(prec, 2);
    let two_x = Float::with_val(prec, &two * x);
    let prefactor_inner = Float::with_val(prec, &pi / &two_x);
    let prefactor_sqrt = Float::with_val(prec, prefactor_inner.sqrt());
    let prefactor = Float::with_val(
        prec,
        prefactor_sqrt * Float::with_val(prec, -x.clone()).exp(),
    );

    let mut sum = Float::with_val(prec, 1);
    let mut term = Float::with_val(prec, 1);
    let mut k: usize = 1;
    let tol = precision_threshold_with(prec);
    let mut prev_abs = Float::with_val(prec, 1);

    loop {
        let mut num = Integer::from(2 * k - 1);
        num = Integer::from(&num * &num);
        let denom = Float::with_val(prec, 8 * k) * x;
        let factor = Float::with_val(prec, num / denom);
        let t_mul = Float::with_val(prec, &term * factor);
        term = Float::with_val(prec, -t_mul);

        let term_abs = term.clone().abs();
        if term_abs < tol {
            sum = Float::with_val(prec, &sum + &term);
            break;
        }
        if term_abs >= prev_abs {
            return None;
        }
        sum = Float::with_val(prec, &sum + &term);
        prev_abs = term_abs;
        k += 1;
    }
    Some(Float::with_val(prec, sum * prefactor))
}

/// Asymptotic expansion of $K_1(x)$ for large $|x|$ (DLMF §10.40.2):
/// $$ K_1(x) \sim \sqrt{\frac{\pi}{2x}} e^{-x} \Bigl[ 1 + \frac{3}{8x}
///    - \frac{15}{128x^2} + \cdots \Bigr]. $$
///
/// Coefficients: $\bigl(4 - (2k-1)^2\bigr) / (8k)^k$.
fn besselk1_asymp(x: &Float, prec: u32) -> Option<Float> {
    let pi = Float::with_val(prec, rug::float::Constant::Pi);
    let two = Float::with_val(prec, 2);
    let two_x = Float::with_val(prec, &two * x);
    let prefactor_inner = Float::with_val(prec, &pi / &two_x);
    let prefactor_sqrt = Float::with_val(prec, prefactor_inner.sqrt());
    let prefactor = Float::with_val(
        prec,
        prefactor_sqrt * Float::with_val(prec, -x.clone()).exp(),
    );

    let mut sum = Float::with_val(prec, 1);
    let mut term = Float::with_val(prec, 1);
    let mut k: usize = 1;
    let tol = precision_threshold_with(prec);
    let mut prev_abs = Float::with_val(prec, 1);

    loop {
        let mut num = Integer::from(2 * k - 1);
        num = Integer::from(4) - Integer::from(&num * &num);
        let denom = Float::with_val(prec, 8 * k) * x;
        let factor = Float::with_val(prec, num / denom);
        term = Float::with_val(prec, &term * factor);

        let term_abs = term.clone().abs();
        if term_abs < tol {
            sum = Float::with_val(prec, &sum + &term);
            break;
        }
        if term_abs >= prev_abs {
            return None;
        }
        sum = Float::with_val(prec, &sum + &term);
        prev_abs = term_abs;
        k += 1;
    }
    Some(Float::with_val(prec, sum * prefactor))
}

/// Same as `precision_threshold()` but accepts an explicit precision parameter.
pub(super) fn precision_threshold_with(prec: u32) -> BackingFloat {
    Float::with_val(prec, 1) >> (prec.saturating_sub(10))
}

/// Power series for $I_0(x)$ (DLMF §10.25.2):
/// $$ I_0(x) = \sum_{k=0}^\infty \frac{(x/2)^{2k}}{(k!)^2}. $$
/// Falls back to the asymptotic expansion for large $|x|$ when the series
/// converges too slowly.
fn besseli0_with_prec(x: &Float, prec: u32) -> Float {
    let work_prec = prec + 40;
    let x_abs = Float::with_val(work_prec, x.clone().abs());
    if let Some(res) = besseli0_asymp(&x_abs, work_prec) {
        return Float::with_val(prec, res);
    }
    let mut sum = Float::with_val(work_prec, 1);
    let mut term = Float::with_val(work_prec, 1);
    let x_half_sq = Float::with_val(work_prec, &x_abs * &x_abs) / 4;
    let tol = precision_threshold_with(work_prec);
    for k in 1..=3000 {
        term *= &x_half_sq;
        term /= Float::with_val(work_prec, k * k);
        sum += &term;
        if term.clone().abs() < tol {
            break;
        }
    }
    Float::with_val(prec, sum)
}

/// Power series for $I_1(x)$ (DLMF §10.25.2):
/// $$ I_1(x) = \frac{x}{2} \sum_{k=0}^\infty \frac{(x/2)^{2k}}{k!(k+1)!}. $$
fn besseli1_with_prec(x: &Float, prec: u32) -> Float {
    let work_prec = prec + 40;
    let x_abs = Float::with_val(work_prec, x.clone().abs());
    if let Some(res) = besseli1_asymp(&x_abs, work_prec) {
        let final_res = Float::with_val(prec, res);
        return if x.is_sign_negative() {
            -final_res
        } else {
            final_res
        };
    }
    let mut sum = Float::with_val(work_prec, 1);
    let mut term = Float::with_val(work_prec, 1);
    let x_half_sq = Float::with_val(work_prec, &x_abs * &x_abs) / 4;
    let tol = precision_threshold_with(work_prec);
    for k in 1..=3000 {
        term *= &x_half_sq;
        term /= Float::with_val(work_prec, k * (k + 1));
        sum += &term;
        if term.clone().abs() < tol {
            break;
        }
    }
    let x_half = Float::with_val(work_prec, x) / 2;
    Float::with_val(prec, x_half * sum)
}

/// Computes the starting index $N$ for Miller's backward recurrence of $I_n(x)$.
///
/// **Heuristic Justification (Single-Pass bounds)**:
/// Miller's backward recurrence (DLMF §10.74) assumes $I_N(x) = 0$ for some
/// sufficiently large $N$. The error introduced by this initialization is roughly
/// $$ I_N(x) \sim \frac{(x/2)^N}{N!}. $$
/// To guarantee `prec` bits of accuracy without a Ziv retry loop, we need $N$ such
/// that
/// $$ \log_2(N!) - N \log_2(x/2) > prec + 50. $$
/// Using Stirling's approximation $\log_2(N!) \approx N \log_2 N - N \log_2 e$
/// (Abramowitz & Stegun §6.1.34, DLMF §5.11.1), we solve this inequality with
/// native `f64` arithmetic (53-bit mantissa).  `f64` precision is mathematically
/// sufficient for the bound because:
/// - The error in Stirling's approximation is $O(1/N)$, so asymptotically negligible.
/// - We always overshoot to be safe: the `prec + 50` target and `+20` initial offset
///   guarantee a conservative overestimate.
///
/// **Magic constants**:
/// - `n_start += 20`: initial safety margin ensuring $N > n$ always.
/// - `target = prec + 50`: the 50-bit safety margin compensates for Stirling error
///   and any `f64` rounding (max ~0.5 ulp ≪ 1 bit).
/// - `chunk = max(10, prec / 20)`: adaptive step size.  At high prec, the loop
///   requires fewer passes (larger chunk); at low prec, we use a minimal step of 10.
///
/// **References**:
/// - DLMF §5.11.1 (Stirling's approximation)
/// - DLMF §10.74 (Miller's algorithm for Bessel functions)
/// - Abramowitz & Stegun §9.7.1 (Bessel function asymptotics)
/// - Olver, F.W.J. (1997). *Asymptotics and Special Functions*.
fn compute_i_start(n: &Integer, v: &Float, prec: u32) -> Integer {
    let mut n_start = n.clone().abs();
    let v_abs = v.clone().abs();
    let v_ceil = v_abs
        .clone()
        .ceil()
        .to_integer()
        .unwrap_or_else(|| Integer::from(0));

    if n_start < v_ceil {
        n_start = v_ceil;
    }
    n_start += 20;

    let x_val = v_abs.to_f64();
    let x_bits = if x_val > 0.0 {
        (x_val / 2.0).log2()
    } else {
        0.0
    };
    let log2_e = core::f64::consts::LOG2_E;
    let target = f64::from(prec + 50);

    // Fast approximation of log2(N!) - N * log2(x/2) using Stirling
    loop {
        let n_f = n_start.to_f64();
        let log2_fact = n_f * n_f.log2() - n_f * log2_e;
        if log2_fact - n_f * x_bits > target {
            break;
        }
        let chunk = 10.max(prec.div_euclid(20));
        n_start += chunk;
    }

    n_start
}

/// Evaluates the Modified Bessel Function of the Second Kind $K_0(x)$.
///
/// **Heuristic Justification (Single-Pass bounds)**:
/// Uses the standard identity (DLMF §10.31.1, Watson Ch. 3):
/// $$ K_0(x) = -\ln(x/2) I_0(x) + \sum_{k=0}^\infty \psi(k+1) \frac{(x/2)^{2k}}{(k!)^2}, $$
/// where $\psi$ is the digamma function.  Since $I_0(x) \sim e^x / \sqrt{2\pi x}$ and
/// $K_0(x) \sim e^{-x} \sqrt{\pi / 2x}$, the sum subtracts terms of magnitude $O(e^x)$
/// to produce a result of magnitude $O(e^{-x})$.  This catastrophic cancellation
/// destroys exactly
/// $$ \log_2(e^x / e^{-x}) = 2x \log_2(e) = 2x / \ln 2 $$
/// bits (Watson, Ch. 3, §3.1).  By allocating
/// $$ \text{work\_prec} = \text{prec} + x \cdot (2 / \ln 2) + 50 $$
/// bits upfront, we perfectly absorb the cancellation in a single pass — no
/// Ziv retry loop is needed.  The `+ 50` guard covers the series summation error
/// (sum of `k` terms, each contributing < 1 ulp, bounded by ~50 ulp for typical `k`).
///
/// **References**:
/// - DLMF §10.31.1 (series for $K_0$)
/// - Watson, G.N. (1944). *A Treatise on the Theory of Bessel Functions*, Ch. 3.
/// - DLMF §10.40.2 (asymptotic expansion, used for large $x$ fallback).
fn besselk0(x: &Float) -> Float {
    let orig_prec = x.prec();
    let work_prec = if *x > 0 {
        let mut ln2 = Float::with_val(orig_prec, 2);
        ln2 = ln2.ln();
        let factor = Float::with_val(orig_prec, Float::with_val(orig_prec, 2) / &ln2);
        let extra = Float::with_val(orig_prec, x) * factor;
        let extra_int = extra
            .ceil()
            .to_integer()
            .unwrap_or_else(|| Integer::from(0));
        let extra_u32 = extra_int.to_u32().unwrap_or(u32::MAX - orig_prec - 50);
        orig_prec.saturating_add(extra_u32).saturating_add(50)
    } else {
        orig_prec + 50
    };

    let x_w = Float::with_val(work_prec, x);

    if let Some(res) = besselk0_asymp(&x_w, work_prec) {
        return Float::with_val(orig_prec, res);
    }
    let two_w = Float::with_val(work_prec, 2);

    let i0 = besseli0_with_prec(&x_w, work_prec);
    let gamma = Float::with_val(work_prec, rug::float::Constant::Euler);
    let x_half = BackingFloat::with_val(work_prec, &x_w / &two_w);
    let ln_term = BackingFloat::with_val(
        work_prec,
        -(BackingFloat::with_val(work_prec, x_half.clone().ln()) + &gamma),
    ) * &i0;

    let t = BackingFloat::with_val(work_prec, &x_half * &x_half);
    let mut sum = BackingFloat::with_val(work_prec, 0);
    let mut term = BackingFloat::with_val(work_prec, 1);
    let mut k = Integer::from(1);
    let mut h = BackingFloat::with_val(work_prec, 0);
    let tol = precision_threshold_with(work_prec);

    loop {
        let kf = BackingFloat::with_val(work_prec, &k);
        h = BackingFloat::with_val(
            work_prec,
            &h + BackingFloat::with_val(work_prec, BackingFloat::with_val(work_prec, 1) / &kf),
        );
        term = BackingFloat::with_val(work_prec, &term * &t)
            / BackingFloat::with_val(work_prec, &kf * &kf);
        let delta = BackingFloat::with_val(work_prec, &h * &term);
        let prev = sum.clone();
        sum = BackingFloat::with_val(work_prec, &sum + &delta);
        if BackingFloat::with_val(work_prec, &sum - &prev).abs() < tol {
            break;
        }
        k += 1;
    }
    let res = BackingFloat::with_val(work_prec, ln_term + sum);
    Float::with_val(orig_prec, res)
}

/// Modified Bessel function of the second kind, order 1 (DLMF §10.31.1):
/// $$ K_1(x) = \ln(x/2) I_1(x) + \frac{1}{x}
///    - \sum_{k=0}^\infty \bigl(\psi(k+1) + \psi(k+2)\bigr) \frac{(x/2)^{2k}}{k!(k+1)!}. $$
///
/// Same cancellation analysis as $K_0$ applies: $2x/\ln 2$ bits destroyed.
fn besselk1(x: &Float) -> Float {
    let orig_prec = x.prec();
    let work_prec = if *x > 0 {
        let mut ln2 = Float::with_val(orig_prec, 2);
        ln2 = ln2.ln();
        let factor = Float::with_val(orig_prec, Float::with_val(orig_prec, 2) / &ln2);
        let extra = Float::with_val(orig_prec, x) * factor;
        let extra_int = extra
            .ceil()
            .to_integer()
            .unwrap_or_else(|| Integer::from(0));
        let extra_u32 = extra_int.to_u32().unwrap_or(u32::MAX - orig_prec - 50);
        orig_prec.saturating_add(extra_u32).saturating_add(50)
    } else {
        orig_prec + 50
    };

    let x_w = Float::with_val(work_prec, x);
    if let Some(res) = besselk1_asymp(&x_w, work_prec) {
        return Float::with_val(orig_prec, res);
    }
    let two_w = Float::with_val(work_prec, 2);

    let i1 = besseli1_with_prec(&x_w, work_prec);
    let gamma = Float::with_val(work_prec, rug::float::Constant::Euler);
    let x_half = BackingFloat::with_val(work_prec, &x_w / &two_w);
    let ln_term = BackingFloat::with_val(
        work_prec,
        BackingFloat::with_val(work_prec, x_half.clone().ln()) + &gamma,
    ) * &i1;

    let t = BackingFloat::with_val(work_prec, &x_half * &x_half);
    let mut sum = BackingFloat::with_val(work_prec, 0);
    let mut term = BackingFloat::with_val(work_prec, 1);
    let mut k = Integer::from(1);
    let mut h_k = BackingFloat::with_val(work_prec, 0);
    let tol = precision_threshold_with(work_prec);

    loop {
        let kf = BackingFloat::with_val(work_prec, &k);
        let kp1 = BackingFloat::with_val(work_prec, Integer::from(&k + 1));

        h_k = BackingFloat::with_val(work_prec, &h_k + BackingFloat::with_val(work_prec, 1) / &kf);
        let h_kp1 = BackingFloat::with_val(
            work_prec,
            &h_k + BackingFloat::with_val(work_prec, 1) / &kp1,
        );
        let h_sum = BackingFloat::with_val(work_prec, &h_k + &h_kp1);

        term = BackingFloat::with_val(work_prec, &term * &t)
            / BackingFloat::with_val(work_prec, &kf * &kp1);
        let delta = BackingFloat::with_val(work_prec, &h_sum * &term);
        let prev = sum.clone();
        sum = BackingFloat::with_val(work_prec, &sum + &delta);
        if BackingFloat::with_val(work_prec, &sum - &prev).abs() < tol {
            break;
        }
        k += 1;
    }

    let x_fourth = BackingFloat::with_val(work_prec, &x_half / 2);
    let res = BackingFloat::with_val(work_prec, 1) / &x_w + &ln_term
        - &x_fourth
        - BackingFloat::with_val(work_prec, x_fourth * sum);
    Float::with_val(orig_prec, res)
}
