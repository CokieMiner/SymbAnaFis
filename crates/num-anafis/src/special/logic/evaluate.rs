use crate::{CliffordNumber, Number, Signature};

fn coeff_limit(n: u8) -> usize {
    1_usize
        .checked_shl(u32::from(n))
        .expect("active generator count exceeds addressable bit width")
}

/// Trait for transcendental and special evaluation on numeric types.
pub trait Evaluate: Sized {
    /// Sine function.
    #[must_use]
    fn sin(&self) -> Self;

    /// Cosine function.
    #[must_use]
    fn cos(&self) -> Self;

    /// Natural exponential function (e^x).
    #[must_use]
    fn exp(&self) -> Self;

    /// Natural logarithm (ln(x)).
    #[must_use]
    fn ln(&self) -> Self;

    /// Square root (√x).
    #[must_use]
    fn sqrt(&self) -> Self;
}

impl Evaluate for Number {
    fn sin(&self) -> Self {
        self.sin()
    }

    fn cos(&self) -> Self {
        self.cos()
    }

    fn exp(&self) -> Self {
        self.exp()
    }

    fn ln(&self) -> Self {
        self.ln()
    }

    fn sqrt(&self) -> Self {
        self.sqrt()
    }
}

fn nan_mv(sig: Signature, n: u8) -> CliffordNumber {
    CliffordNumber::scalar_unchecked(sig, n, Number::float(f64::NAN))
}

fn scalar_mv(sig: Signature, n: u8, value: Number) -> CliffordNumber {
    CliffordNumber::scalar_unchecked(sig, n, value)
}

fn scalar_only(mv: &CliffordNumber) -> bool {
    let limit = coeff_limit(mv.active_generators());
    for blade in 1..limit {
        if !mv.coeff(blade).is_zero() {
            return false;
        }
    }
    true
}

fn scalar_and_e1(mv: &CliffordNumber) -> Option<(Number, Number)> {
    if mv.active_generators() == 0 {
        return Some((mv.coeff(0).clone(), Number::from(0_i64)));
    }

    let limit = coeff_limit(mv.active_generators());
    for blade in 0..limit {
        if blade != 0 && blade != 1 && !mv.coeff(blade).is_zero() {
            return None;
        }
    }

    Some((mv.coeff(0).clone(), mv.coeff(1).clone()))
}

fn sinh_num(x: &Number) -> Number {
    let ex = x.exp();
    let enx = (-x).exp();
    (ex - enx) / Number::from(2_i64)
}

fn cosh_num(x: &Number) -> Number {
    let ex = x.exp();
    let enx = (-x).exp();
    (ex + enx) / Number::from(2_i64)
}

fn sign_num(x: &Number) -> Number {
    if x.is_negative() {
        Number::from(-1_i64)
    } else if x.is_positive() {
        Number::from(1_i64)
    } else {
        Number::from(0_i64)
    }
}

impl Evaluate for CliffordNumber {
    fn sin(&self) -> Self {
        const SPLIT_SIG: Signature = Signature { p: 1, q: 0, r: 0 };
        const COMPLEX_SIG: Signature = Signature { p: 0, q: 1, r: 0 };
        const DUAL_SIG: Signature = Signature { p: 0, q: 0, r: 1 };

        let sig = self.signature();
        let n = self.active_generators();

        if scalar_only(self) {
            return scalar_mv(sig, n, self.coeff(0).sin());
        }

        if let Some((a, b)) = scalar_and_e1(self) {
            if sig == SPLIT_SIG && n == 1 {
                // sin(a+bj) = sin(a)cosh(b) + j*cos(a)sinh(b)
                let scalar = a.sin() * cosh_num(&b);
                let e1 = a.cos() * sinh_num(&b);
                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, scalar);
                out.set_coeff(1, e1);
                return out;
            }

            if sig == COMPLEX_SIG && n == 1 {
                // sin(a+bi) = sin(a)cosh(b) + i*cos(a)sinh(b)
                let scalar = a.sin() * cosh_num(&b);
                let e1 = a.cos() * sinh_num(&b);
                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, scalar);
                out.set_coeff(1, e1);
                return out;
            }

            if sig == DUAL_SIG && n == 1 {
                // sin(a+bε) = sin(a) + b*cos(a) ε
                let scalar = a.sin();
                let e1 = b * a.cos();
                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, scalar);
                out.set_coeff(1, e1);
                return out;
            }
        }

        nan_mv(sig, n)
    }

    fn cos(&self) -> Self {
        const SPLIT_SIG: Signature = Signature { p: 1, q: 0, r: 0 };
        const COMPLEX_SIG: Signature = Signature { p: 0, q: 1, r: 0 };
        const DUAL_SIG: Signature = Signature { p: 0, q: 0, r: 1 };

        let sig = self.signature();
        let n = self.active_generators();

        if scalar_only(self) {
            return scalar_mv(sig, n, self.coeff(0).cos());
        }

        if let Some((a, b)) = scalar_and_e1(self) {
            if sig == SPLIT_SIG && n == 1 {
                // cos(a+bj) = cos(a)cosh(b) - j*sin(a)sinh(b)
                let scalar = a.cos() * cosh_num(&b);
                let e1 = -(a.sin() * sinh_num(&b));
                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, scalar);
                out.set_coeff(1, e1);
                return out;
            }

            if sig == COMPLEX_SIG && n == 1 {
                // cos(a+bi) = cos(a)cosh(b) - i*sin(a)sinh(b)
                let scalar = a.cos() * cosh_num(&b);
                let e1 = -(a.sin() * sinh_num(&b));
                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, scalar);
                out.set_coeff(1, e1);
                return out;
            }

            if sig == DUAL_SIG && n == 1 {
                // cos(a+bε) = cos(a) - b*sin(a) ε
                let scalar = a.cos();
                let e1 = -(b * a.sin());
                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, scalar);
                out.set_coeff(1, e1);
                return out;
            }
        }

        nan_mv(sig, n)
    }

    fn exp(&self) -> Self {
        const SPLIT_SIG: Signature = Signature { p: 1, q: 0, r: 0 };
        const COMPLEX_SIG: Signature = Signature { p: 0, q: 1, r: 0 };
        const DUAL_SIG: Signature = Signature { p: 0, q: 0, r: 1 };

        let sig = self.signature();
        let n = self.active_generators();

        if scalar_only(self) {
            return scalar_mv(sig, n, self.coeff(0).exp());
        }

        if let Some((a, b)) = scalar_and_e1(self) {
            if sig == SPLIT_SIG && n == 1 {
                // exp(a+bj) = exp(a)*cosh(b) + j*exp(a)*sinh(b)
                let ea = a.exp();
                let scalar = ea.clone() * cosh_num(&b);
                let e1 = ea * sinh_num(&b);
                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, scalar);
                out.set_coeff(1, e1);
                return out;
            }

            if sig == COMPLEX_SIG && n == 1 {
                // exp(a+bi) = exp(a)(cos(b) + i sin(b))
                let ea = a.exp();
                let scalar = ea.clone() * b.cos();
                let e1 = ea * b.sin();
                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, scalar);
                out.set_coeff(1, e1);
                return out;
            }

            if sig == DUAL_SIG && n == 1 {
                // exp(a+bε) = exp(a) + b*exp(a) ε
                let ea = a.exp();
                let scalar = ea.clone();
                let e1 = b * ea;
                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, scalar);
                out.set_coeff(1, e1);
                return out;
            }
        }

        nan_mv(sig, n)
    }

    fn ln(&self) -> Self {
        const SPLIT_SIG: Signature = Signature { p: 1, q: 0, r: 0 };
        const COMPLEX_SIG: Signature = Signature { p: 0, q: 1, r: 0 };
        const DUAL_SIG: Signature = Signature { p: 0, q: 0, r: 1 };

        let sig = self.signature();
        let n = self.active_generators();

        if scalar_only(self) {
            return scalar_mv(sig, n, self.coeff(0).ln());
        }

        if let Some((a, b)) = scalar_and_e1(self) {
            if sig == SPLIT_SIG && n == 1 {
                // ln(a+bj)
                if a.abs() <= b.abs() {
                    return nan_mv(sig, n); // Domain error for split-complex ln
                }
                let r = (&a * &a - &b * &b).sqrt();
                let theta_val = ((&a + &b) / (&a - &b)).ln() / Number::from(2_i64);

                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, r.ln());
                out.set_coeff(1, theta_val);
                return out;
            }

            if sig == COMPLEX_SIG && n == 1 {
                // ln(a+bi) = ln(r) + i*arg(z)
                let r = (&a * &a + &b * &b).sqrt();
                let theta = b.atan2(&a);

                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, r.ln());
                out.set_coeff(1, theta);
                return out;
            }

            if sig == DUAL_SIG && n == 1 {
                // ln(a+bε) = ln(a) + (b/a) ε
                if a.is_zero() {
                    return nan_mv(sig, n);
                }
                let scalar = a.ln();
                let e1 = b / a;
                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, scalar);
                out.set_coeff(1, e1);
                return out;
            }
        }

        nan_mv(sig, n)
    }

    fn sqrt(&self) -> Self {
        const SPLIT_SIG: Signature = Signature { p: 1, q: 0, r: 0 };
        const COMPLEX_SIG: Signature = Signature { p: 0, q: 1, r: 0 };
        const DUAL_SIG: Signature = Signature { p: 0, q: 0, r: 1 };

        let sig = self.signature();
        let n = self.active_generators();

        if scalar_only(self) {
            return scalar_mv(sig, n, self.coeff(0).sqrt());
        }

        if let Some((a, b)) = scalar_and_e1(self) {
            if sig == SPLIT_SIG && n == 1 {
                // sqrt(a+bj)
                let r = (&a * &a - &b * &b).sqrt();
                // Avoid using full formulas if they drop to nan for |a| < |b| unless needed
                // u = sqrt((a + r) / 2), v = sign(b) * sqrt((a - r) / 2)
                let two = Number::from(2_i64);
                let u = ((&a + &r) / two.clone()).sqrt();
                let v = ((&a - &r) / two).sqrt();
                let v_signed = sign_num(&b) * v;

                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, u);
                out.set_coeff(1, v_signed);
                return out;
            }

            if sig == COMPLEX_SIG && n == 1 {
                // Principal complex square root.
                let r = (&a * &a + &b * &b).sqrt();
                let two = Number::from(2_i64);
                let u = ((&r + &a) / two.clone()).sqrt();
                let v = ((&r - &a) / two).sqrt();
                let v_signed = sign_num(&b) * v;

                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, u);
                out.set_coeff(1, v_signed);
                return out;
            }

            if sig == DUAL_SIG && n == 1 {
                // sqrt(a+bε) = sqrt(a) + b/(2 sqrt(a)) ε
                let root = a.sqrt();
                if root.is_zero() {
                    return nan_mv(sig, n);
                }
                let scalar = root.clone();
                let e1 = b / (Number::from(2_i64) * root);
                let mut out = Self::zero_unchecked(sig, n);
                out.set_coeff(0, scalar);
                out.set_coeff(1, e1);
                return out;
            }
        }

        nan_mv(sig, n)
    }
}
