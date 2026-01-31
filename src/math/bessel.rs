use crate::core::traits::MathScalar;

/// Bessel function of the first kind `J_n(x)`
///
/// Uses a hybrid algorithm for numerical stability:
/// - **Forward Recurrence** for $n \le |x|$: $J_{n+1} = (2n/x) `J_n` - J_{n-1}$
/// - **Miller's Algorithm (Backward Recurrence)** for $n > |x|$: Stable for decaying region.
///
/// with `J_0` and `J_1` computed via rational approximations.
///
/// # Special Values
/// - `J_0(0)` = 1
/// - `J_n(0)` = 0 for n ≠ 0
///
/// Reference: A&S §9.1.27, DLMF §10.6 <https://dlmf.nist.gov/10.6>
pub fn bessel_j<T: MathScalar>(n: i32, x: T) -> Option<T> {
    let n_abs = n.abs();
    let ax = x.abs();

    // Special case: J_n(0)
    let threshold = T::from(1e-10).expect("Failed to convert mathematical constant");
    if ax < threshold {
        return Some(if n_abs == 0 { T::one() } else { T::zero() });
    }

    // REGIME 1: Forward Recurrence is stable when n <= |x|
    if T::from_i32(n_abs).expect("Failed to convert integer argument") <= ax {
        return bessel_j_forward(n, x);
    }

    // REGIME 2: Miller's Algorithm (Backward Recurrence) when n > |x|
    bessel_j_miller(n, x)
}

/// Forward recurrence for `J_n(x)` when n <= |x|
#[allow(
    clippy::unnecessary_wraps,
    reason = "Consistent API with other Bessel functions"
)]
fn bessel_j_forward<T: MathScalar>(n: i32, x: T) -> Option<T> {
    let n_abs = n.abs();

    let j0 = bessel_j0(x);
    if n_abs == 0 {
        return Some(j0);
    }

    let j1 = bessel_j1(x);
    if n_abs == 1 {
        return Some(if n < 0 { -j1 } else { j1 });
    }

    let (mut jp, mut jc) = (j0, j1);
    let two = T::from(2.0).expect("Failed to convert mathematical constant");
    let mut k_t = T::one();

    for _ in 1..n_abs {
        // J_{k+1} = (2k/x) J_k - J_{k-1}
        let jn = (two * k_t / x) * jc - jp;
        jp = jc;
        jc = jn;
        k_t += T::one();
    }

    Some(if n < 0 && n_abs % 2 == 1 { -jc } else { jc })
}

/// Miller's backward recurrence algorithm for `J_n(x)` when n > |x|
#[allow(
    clippy::unnecessary_wraps,
    reason = "Consistent API with other Bessel functions"
)]
fn bessel_j_miller<T: MathScalar>(n: i32, x: T) -> Option<T> {
    let n_abs = n.abs();
    let two = T::from(2.0).expect("Failed to convert mathematical constant");

    // Choose starting N using a safe heuristic
    let n_start = compute_miller_start(n_abs);

    // Initialize recurrence
    let mut j_next = T::zero(); // J_{k+1}
    let mut j_curr = T::from_f64(1e-30).expect("Failed to convert seed value 1e-30"); // J_k (small seed)

    let mut result = T::zero();
    let mut sum = T::zero(); // For normalization: J_0 + 2*(J_2 + J_4 + ...)
    let mut compensation = T::zero(); // Kahan summation compensation

    let mut k_t = T::from(n_start).expect("Failed to convert start index");

    // Backward recurrence: J_{k-1} = (2k/x) J_k - J_{k+1}
    for k in (0..=n_start).rev() {
        let j_prev = (two * k_t / x) * j_curr - j_next;

        // Store J_n when we reach it
        if k == n_abs {
            result = j_curr;
        }

        // Accumulate normalization sum with Kahan summation
        // Sum = J_0 + 2 \sum_{k=1}^{\infty} J_{2k}
        if k == 0 {
            // sum += j_curr (J_0 term)
            let y = j_curr - compensation;
            let t = sum + y;
            compensation = (t - sum) - y;
            sum = t;
        } else if k % 2 == 0 {
            // sum += 2 * j_curr (even terms)
            let term = two * j_curr;
            let y = term - compensation;
            let t = sum + y;
            compensation = (t - sum) - y;
            sum = t;
        }

        j_next = j_curr;
        j_curr = j_prev;
        k_t -= T::one();
    }

    // Normalize: true J_n = (computed J_n) / (computed sum)
    // The sum should be J_0(x) of the computed sequence, which is 1
    let scale = T::one() / sum;
    let normalized_result = result * scale;

    Some(if n < 0 && n_abs % 2 == 1 {
        -normalized_result
    } else {
        normalized_result
    })
}

/// Compute starting point for Miller's algorithm
fn compute_miller_start(n: i32) -> i32 {
    // Heuristic from Numerical Recipes with extra safety margin
    // N = n + sqrt(C * n) + M
    let n_f = f64::from(n);
    let c = 50.0; // Conservative for 15+ digit accuracy
    let m = 15; // Safety margin

    #[allow(
        clippy::cast_possible_truncation,
        reason = "n is i32, sqrt result rounded and within range"
    )]
    let sqrt_round = (c * n_f).sqrt().round() as i32;
    n + sqrt_round + m
}

/// Bessel function `J_0(x)` via rational approximation
///
/// For |x| < 8: uses polynomial ratio
/// For |x| ≥ 8: uses asymptotic form `J_0(x)` ≈ √(2/πx) cos(x - π/4) P(8/x)
///
/// Reference: A&S §9.4.1-9.4.6 <https://dlmf.nist.gov/10.17>
pub fn bessel_j0<T: MathScalar>(x: T) -> T {
    const BESSEL_J0_SMALL_NUM: [f64; 6] = [
        57_568_490_574.0,
        -13_362_590_354.0,
        651_619_640.7,
        -11_214_424.18,
        77_392.330_17,
        -184.905_245_6,
    ];
    const BESSEL_J0_SMALL_DEN: [f64; 6] = [
        57_568_490_411.0,
        1_029_532_985.0,
        9_494_680.718,
        59_272.648_53,
        267.853_271_2,
        1.0,
    ];

    const BESSEL_J0_LARGE_P_COS: [f64; 5] = [
        1.0,
        -0.109_862_862_7e-2,
        0.273_451_040_7e-4,
        -0.207_337_063_9e-5,
        0.209_388_721_1e-6,
    ];
    const BESSEL_J0_LARGE_P_SIN: [f64; 5] = [
        -0.156_249_999_5e-1,
        0.143_048_876_5e-3,
        -0.691_114_765_1e-5,
        0.762_109_516_1e-6,
        0.934_935_152e-7,
    ];

    let ax = x.abs();
    let eight = T::from(8.0).expect("Failed to convert mathematical constant");

    if ax < eight {
        let y = x * x;
        eval_rational_poly(y, &BESSEL_J0_SMALL_NUM, &BESSEL_J0_SMALL_DEN)
    } else {
        let z = eight / ax;
        let y = z * z;
        let shift = T::from_f64(0.785_398_164).expect("Failed to convert phase shift constant");
        let xx = ax - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / ax).sqrt();

        // P ~ cos
        let p_cos = eval_poly_horner(y, &BESSEL_J0_LARGE_P_COS);

        // Q ~ sin
        let p_sin = eval_poly_horner(y, &BESSEL_J0_LARGE_P_SIN);

        term_sqrt * (xx.cos() * p_cos - z * xx.sin() * p_sin)
    }
}

/// Bessel function `J_1(x)` via rational approximation
///
/// Same structure as `J_0` with different coefficients.
///
/// Reference: A&S §9.4.4-9.4.6 <https://dlmf.nist.gov/10.17>
pub fn bessel_j1<T: MathScalar>(x: T) -> T {
    const BESSEL_J1_SMALL_NUM: [f64; 6] = [
        72_362_614_232.0,
        -7_895_059_235.0,
        242_396_853.1,
        -2_972_611.439,
        15_704.482_60,
        -30.160_366_06,
    ];
    const BESSEL_J1_SMALL_DEN: [f64; 6] = [
        144_725_228_442.0,
        2_300_535_178.0,
        18_583_304.74,
        99_447.433_94,
        376.999_139_7,
        1.0,
    ];

    const BESSEL_J1_LARGE_P_COS: [f64; 5] = [
        1.0,
        0.183_105e-2,
        -0.351_639_649_6e-4,
        0.245_752_017_4e-5,
        -0.240_337_019e-6,
    ];
    const BESSEL_J1_LARGE_P_SIN: [f64; 5] = [
        0.046_874_999_95,
        -0.200_269_087_3e-3,
        0.844_919_909_6e-5,
        -0.882_289_87e-6,
        0.105_787_412e-6,
    ];

    let ax = x.abs();
    let eight = T::from(8.0).expect("Failed to convert mathematical constant");

    if ax < eight {
        let y = x * x;
        let term = eval_rational_poly(y, &BESSEL_J1_SMALL_NUM, &BESSEL_J1_SMALL_DEN);
        x * term
    } else {
        let z = eight / ax;
        let y = z * z;
        let shift = T::from(2.356_194_491).expect("Failed to convert mathematical constant");
        let xx = ax - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / ax).sqrt();

        let p_cos = eval_poly_horner(y, &BESSEL_J1_LARGE_P_COS);
        let p_sin = eval_poly_horner(y, &BESSEL_J1_LARGE_P_SIN);

        let ans = term_sqrt * (xx.cos() * p_cos - z * xx.sin() * p_sin);

        if x < T::zero() { -ans } else { ans }
    }
}

/// Bessel function of the second kind `Y_n(x)`
///
/// Uses forward recurrence: Y_{n+1}(x) = (2n/x) `Y_n(x)` - Y_{n-1}(x)
/// Defined only for x > 0 (singular at origin).
///
/// Reference: A&S §9.1.27, DLMF §10.6 <https://dlmf.nist.gov/10.6>
pub fn bessel_y<T: MathScalar>(n: i32, x: T) -> Option<T> {
    if x <= T::zero() {
        return None;
    }
    let n_abs = n.abs();
    let y0 = bessel_y0(x);
    if n_abs == 0 {
        return Some(y0);
    }
    let y1 = bessel_y1(x);
    if n_abs == 1 {
        return Some(if n < 0 { -y1 } else { y1 });
    }
    let (mut yp, mut yc) = (y0, y1);
    let two = T::from(2.0).expect("Failed to convert mathematical constant");
    let mut k_t = T::one();

    for _ in 1..n_abs {
        let yn = (two * k_t / x) * yc - yp;
        yp = yc;
        yc = yn;
        k_t += T::one();
    }
    Some(if n < 0 && n_abs % 2 == 1 { -yc } else { yc })
}

/// Bessel function `Y_0(x)` via rational approximation
///
/// Reference: A&S §9.4.1-9.4.6 <https://dlmf.nist.gov/10.17>
pub fn bessel_y0<T: MathScalar>(x: T) -> T {
    const BESSEL_Y0_SMALL_NUM: [f64; 6] = [
        -2_957_821_389.0,
        7_062_834_065.0,
        -512_359_803.6,
        10_879_881.29,
        -86_327.927_57,
        228.462_273_3,
    ];
    const BESSEL_Y0_SMALL_DEN: [f64; 6] = [
        40_076_544_269.0,
        745_249_964.8,
        7_189_466.438,
        47_447.264_70,
        226.103_024_4,
        1.0,
    ];

    const BESSEL_Y0_LARGE_P_SIN: [f64; 5] = [
        1.0,
        -0.109_862_862_7e-2,
        0.273_451_040_7e-4,
        -0.207_337_063_9e-5,
        0.209_388_721_1e-6,
    ];
    const BESSEL_Y0_LARGE_P_COS: [f64; 5] = [
        -0.156_249_999_5e-1,
        0.143_048_876_5e-3,
        -0.691_114_765_1e-5,
        0.762_109_516_1e-6,
        0.934_935_152e-7,
    ];

    let eight = T::from(8.0).expect("Failed to convert mathematical constant");

    if x < eight {
        let y = x * x;
        let term = eval_rational_poly(y, &BESSEL_Y0_SMALL_NUM, &BESSEL_Y0_SMALL_DEN);
        let c = T::FRAC_2_PI();
        term + c * bessel_j0(x) * x.ln()
    } else {
        let z = eight / x;
        let y = z * z;
        let shift = T::from(0.785_398_164).expect("Failed to convert mathematical constant");
        let xx = x - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / x).sqrt();

        let p_sin = eval_poly_horner(y, &BESSEL_Y0_LARGE_P_SIN);
        let p_cos = eval_poly_horner(y, &BESSEL_Y0_LARGE_P_COS);

        term_sqrt * (xx.sin() * p_sin + z * xx.cos() * p_cos)
    }
}

/// Bessel function `Y_1(x)` via rational approximation
///
/// Reference: A&S §9.4.4-9.4.6 <https://dlmf.nist.gov/10.17>
pub fn bessel_y1<T: MathScalar>(x: T) -> T {
    const BESSEL_Y1_SMALL_NUM: [f64; 6] = [
        -0.490_060_494_3e13,
        0.127_527_439_0e13,
        -0.515_343_813_9e11,
        0.734_926_455_1e9,
        -0.423_792_272_6e7,
        0.851_193_793_5e4,
    ];
    const BESSEL_Y1_SMALL_DEN: [f64; 7] = [
        0.249_958_057_0e14,
        0.424_441_966_4e12,
        0.373_365_036_7e10,
        0.224_590_400_2e8,
        0.102_042_605_0e6,
        0.354_963_288_5e3,
        1.0,
    ];

    const BESSEL_Y1_LARGE_P_SIN: [f64; 5] = [
        1.0,
        0.183_105e-2,
        -0.351_639_649_6e-4,
        0.245_752_017_4e-5,
        -0.240_337_019e-6,
    ];
    const BESSEL_Y1_LARGE_P_COS: [f64; 5] = [
        0.046_874_999_95,
        -0.200_269_087_3e-3,
        0.844_919_909_6e-5,
        -0.882_289_87e-6,
        0.105_787_412e-6,
    ];

    let eight = T::from(8.0).expect("Failed to convert mathematical constant");

    if x < eight {
        let y = x * x;
        let term_poly = eval_rational_poly(y, &BESSEL_Y1_SMALL_NUM, &BESSEL_Y1_SMALL_DEN);
        let term = x * term_poly;
        let c = T::FRAC_2_PI();
        term + c * (bessel_j1(x) * x.ln() - T::one() / x)
    } else {
        let z = eight / x;
        let y = z * z;
        let shift = T::from(2.356_194_491).expect("Failed to convert mathematical constant");
        let xx = x - shift;

        let c_sqrt = T::FRAC_2_PI();
        let term_sqrt = (c_sqrt / x).sqrt();

        let p_sin = eval_poly_horner(y, &BESSEL_Y1_LARGE_P_SIN);
        let p_cos = eval_poly_horner(y, &BESSEL_Y1_LARGE_P_COS);

        term_sqrt * (xx.sin() * p_sin + z * xx.cos() * p_cos)
    }
}

/// Modified Bessel function of the first kind `I_n(x)`
///
/// Uses Miller's backward recurrence algorithm for numerical stability.
/// I_{k-1} = (2k/x) `I_k` + I_{k+1}, normalized using `I_0(x)`.
///
/// Reference: Numerical Recipes §6.6, A&S §9.6 <https://dlmf.nist.gov/10.25>
pub fn bessel_i<T: MathScalar>(n: i32, x: T) -> T {
    let n_abs = n.abs();
    if n_abs == 0 {
        return bessel_i0(x);
    }
    if n_abs == 1 {
        return bessel_i1(x);
    }

    let threshold = T::from(1e-10).expect("Failed to convert mathematical constant");
    if x.abs() < threshold {
        return T::zero();
    }

    // Miller's backward recurrence algorithm
    // Start from a large order N >> n, set I_N = 0, I_{N-1} = 1
    // Recur backward using: I_{k-1} = (2k/x) * I_k + I_{k+1}
    // Normalize using I_0(x) as reference

    let two = T::from(2.0).expect("Failed to convert mathematical constant");

    // Choose starting order N based on x and n
    // Empirical formula: N = n + sqrt(40*n) + 10 works well
    // The factor 40 comes from numerical stability analysis:
    // - Ensures backward recurrence converges before reaching target order
    // - Balances computational cost vs. accuracy (see NR §6.6)
    // - Tested empirically for x ∈ [0.1, 100], n ∈ [0, 100]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "n_abs is i32, sqrt result within safe range"
    )]
    let sqrt_term = f64::from(40 * n_abs).sqrt() as i32;
    let n_start = n_abs + sqrt_term + 10;
    let n_start = n_start.max(n_abs + 20);

    // Initialize backward recurrence
    let mut i_next = T::zero(); // I_{k+1}
    let mut i_curr = T::from(1e-30).expect("Failed to convert mathematical constant"); // I_k (small nonzero to avoid underflow)
    let mut result = T::zero();
    let mut sum = T::zero(); // For normalization: sum = I_0 + 2*(I_2 + I_4 + ...)
    let mut k_t = T::from(n_start).expect("Failed to convert start index");

    // Backward recurrence
    for k in (0..=n_start).rev() {
        // I_{k-1} = (2k/x) * I_k + I_{k+1}
        let i_prev = (two * k_t / x) * i_curr + i_next;

        // Save I_n when we reach it
        if k == n_abs {
            result = i_curr;
        }

        // Accumulate for normalization (using I_0 + 2*sum of even terms)
        if k == 0 {
            sum += i_curr;
        } else if k % 2 == 0 {
            sum += two * i_curr;
        }

        i_next = i_curr;
        i_curr = i_prev;
        k_t -= T::one();
    }

    // Normalize: actual I_n = result * I_0(x) / computed_I_0
    // The sum approximates I_0 when properly normalized
    let i0_actual = bessel_i0(x);
    let scale = i0_actual / sum;

    result * scale
}

/// Modified Bessel function `I_0(x)` via polynomial approximation
///
/// Reference: A&S §9.8.1-9.8.4 <https://dlmf.nist.gov/10.40>
pub fn bessel_i0<T: MathScalar>(x: T) -> T {
    const BESSEL_I0_SMALL: [f64; 7] = [
        1.0,
        3.515_622_9,
        3.089_942_4,
        1.206_749_2,
        0.265_973_2,
        0.036_076_8,
        0.004_581_3,
    ];
    const BESSEL_I0_LARGE: [f64; 9] = [
        0.398_942_28,
        0.013_285_92,
        0.002_253_19,
        -0.001_575_65,
        0.009_162_81,
        -0.020_577_06,
        0.026_355_37,
        -0.016_476_33,
        0.003_923_77,
    ];

    let ax = x.abs();
    let three_seven_five = T::from(3.75).expect("Failed to convert mathematical constant");

    if ax < three_seven_five {
        let y = (x / three_seven_five).powi(2);
        // c0=1.0 implicit? "T::one() + y * (...)"
        // Yes.
        eval_poly_horner(y, &BESSEL_I0_SMALL)
    } else {
        let y = three_seven_five / ax;
        let term = ax.exp() / ax.sqrt();

        term * eval_poly_horner(y, &BESSEL_I0_LARGE)
    }
}

/// Modified Bessel function `I_1(x)` via polynomial approximation
///
/// Reference: A&S §9.8.1-9.8.4 <https://dlmf.nist.gov/10.40>
pub fn bessel_i1<T: MathScalar>(x: T) -> T {
    const BESSEL_I1_SMALL: [f64; 7] = [
        0.5,
        0.878_905_94,
        0.514_988_69,
        0.150_849_34,
        0.026_587_33,
        0.003_015_32,
        0.000_324_11,
    ];
    const BESSEL_I1_LARGE: [f64; 9] = [
        0.398_942_28,
        -0.039_880_24,
        -0.003_620_18,
        0.001_638_01,
        -0.010_315_55,
        0.022_829_67,
        -0.028_953_12,
        0.017_876_54,
        -0.004_200_59,
    ];

    let ax = x.abs();
    let three_seven_five = T::from(3.75).expect("Failed to convert mathematical constant");

    let ans = if ax < three_seven_five {
        let y = (x / three_seven_five).powi(2);
        ax * eval_poly_horner(y, &BESSEL_I1_SMALL)
    } else {
        let y = three_seven_five / ax;
        let term = ax.exp() / ax.sqrt();

        term * eval_poly_horner(y, &BESSEL_I1_LARGE)
    };
    if x < T::zero() { -ans } else { ans }
}

/// Modified Bessel function of the second kind `K_n(x)`
///
/// Uses forward recurrence: K_{n+1} = (2n/x) `K_n` + K_{n-1}
/// Defined only for x > 0 (singular at origin).
///
/// Reference: A&S §9.6.26, DLMF §10.29 <https://dlmf.nist.gov/10.29>
pub fn bessel_k<T: MathScalar>(n: i32, x: T) -> Option<T> {
    if x <= T::zero() {
        return None;
    }
    let n_abs = n.abs();
    let k0 = bessel_k0(x);
    if n_abs == 0 {
        return Some(k0);
    }
    let k1 = bessel_k1(x);
    if n_abs == 1 {
        return Some(k1);
    }
    let (mut kp, mut kc) = (k0, k1);
    let two = T::from(2.0).expect("Failed to convert mathematical constant");
    let mut k_t = T::one();

    for _ in 1..n_abs {
        let kn = kp + (two * k_t / x) * kc;
        kp = kc;
        kc = kn;
        k_t += T::one();
    }
    Some(kc)
}

/// Modified Bessel function `K_0(x)` via polynomial approximation
///
/// Reference: A&S §9.8.5-9.8.8 <https://dlmf.nist.gov/10.40>
pub fn bessel_k0<T: MathScalar>(x: T) -> T {
    const COEFFS: [f64; 7] = [
        -0.577_215_66,
        0.422_784_20,
        0.230_697_56,
        0.034_885_90,
        0.002_626_98,
        0.000_107_50,
        0.000_007_4,
    ];

    const COEFFS_LARGE: [f64; 8] = [
        1.253_314_14,
        -0.078_323_58,
        0.021_895_68,
        -0.010_624_46,
        0.005_878_72,
        -0.002_515_40,
        0.000_532_08,
        -0.000_025_200,
    ];

    let two = T::from(2.0).expect("Failed to convert mathematical constant");
    if x <= two {
        let four = T::from(4.0).expect("Failed to convert mathematical constant");
        let y = x * x / four;
        let i0 = bessel_i0(x);
        let ln_term = -(x / two).ln() * i0;

        let poly = eval_poly_horner(y, &COEFFS);
        ln_term + poly
    } else {
        let y = two / x;
        let term = (-x).exp() / x.sqrt();

        term * eval_poly_horner(y, &COEFFS_LARGE)
    }
}

pub fn bessel_k1<T: MathScalar>(x: T) -> T {
    const COEFFS: [f64; 7] = [
        1.0,
        0.154_431_44,
        -0.672_785_79,
        -0.181_568_97,
        -0.019_194_02,
        -0.001_104_04,
        -0.000_046_86,
    ];

    const COEFFS_LARGE: [f64; 8] = [
        1.253_314_14,
        0.234_986_19,
        -0.036_556_20,
        0.015_042_68,
        -0.007_803_53,
        0.003_256_14,
        -0.000_682_45,
        0.000_031_6,
    ];

    let two = T::from(2.0).expect("Failed to convert mathematical constant");
    if x <= two {
        let four = T::from(4.0).expect("Failed to convert mathematical constant");
        let y = x * x / four;

        let term1 = x.ln() * bessel_i1(x);
        let term2 = T::one() / x;

        let poly = eval_poly_horner(y, &COEFFS);
        term1 + term2 * poly
    } else {
        let y = two / x;
        let term = (-x).exp() / x.sqrt();

        term * eval_poly_horner(y, &COEFFS_LARGE)
    }
}

/// Helper: Evaluate polynomial c\[0\] + x*c\[1\] + ... + x^n*c\[n\] using Horner's method
///
/// Note: Coefficients should be ordered from constant term c\[0\] to highest power c\[n\].
fn eval_poly_horner<T: MathScalar>(x: T, coeffs: &[f64]) -> T {
    let mut sum = T::zero();
    // Horner's method: c[0] + x(c[1] + x(c[2] + ...))
    // We iterate from highest power c[n] down to c[0]
    for &c in coeffs.iter().rev() {
        sum = sum * x + T::from(c).expect("Failed to convert mathematical constant");
    }
    sum
}

/// Helper: Evaluate rational function P(x)/Q(x)
///
/// Computes (n\[0\] + x*n\[1\] + ...) / (d\[0\] + x*d\[1\] + ...)
fn eval_rational_poly<T: MathScalar>(x: T, num: &[f64], den: &[f64]) -> T {
    let n = eval_poly_horner(x, num);
    let d = eval_poly_horner(x, den);
    n / d
}
