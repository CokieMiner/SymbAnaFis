#[cfg(test)]
mod dual_tests {
    use crate::Dual;
    use num_traits::Float;
    use std::f64::consts::PI;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_dual_basic_arithmetic() {
        // x = 2, dx = 1 (variable)
        let x = Dual::new(2.0, 1.0);
        // c = 3, dc = 0 (constant)
        let c = Dual::constant(3.0);

        // x + c = 5, derivative = 1
        let sum = x + c;
        assert!(approx_eq(sum.val, 5.0));
        assert!(approx_eq(sum.eps, 1.0));

        // x - c = -1, derivative = 1
        let diff = x - c;
        assert!(approx_eq(diff.val, -1.0));
        assert!(approx_eq(diff.eps, 1.0));

        // x * c = 6, d/dx(3x) = 3
        let prod = x * c;
        assert!(approx_eq(prod.val, 6.0));
        assert!(approx_eq(prod.eps, 3.0));

        // x / c = 2/3, d/dx(x/3) = 1/3
        let quot = x / c;
        assert!(approx_eq(quot.val, 2.0 / 3.0));
        assert!(approx_eq(quot.eps, 1.0 / 3.0));
    }

    #[test]
    fn test_dual_product_rule() {
        // f(x) = x * x = x^2, f'(x) = 2x
        let x = Dual::new(3.0, 1.0);
        let x_squared = x * x;
        assert!(approx_eq(x_squared.val, 9.0));
        assert!(approx_eq(x_squared.eps, 6.0)); // 2 * 3
    }

    #[test]
    fn test_dual_quotient_rule() {
        // f(x) = x / (x + 1), f'(x) = 1/(x+1)^2
        let x = Dual::new(2.0, 1.0);
        let one = Dual::constant(1.0);
        let result = x / (x + one);

        assert!(approx_eq(result.val, 2.0 / 3.0));
        // f'(2) = 1/(3)^2 = 1/9
        assert!(approx_eq(result.eps, 1.0 / 9.0));
    }

    #[test]
    fn test_dual_sin_cos() {
        use PI;

        // At x = π/4: sin(π/4) = √2/2, cos(π/4) = √2/2
        let x = Dual::new(PI / 4.0, 1.0);

        let sin_x = x.sin();
        let cos_x = x.cos();

        let sqrt2_2 = 2.0_f64.sqrt() / 2.0;
        assert!(approx_eq(sin_x.val, sqrt2_2));
        assert!(approx_eq(sin_x.eps, sqrt2_2)); // d/dx sin(x) = cos(x)

        assert!(approx_eq(cos_x.val, sqrt2_2));
        assert!(approx_eq(cos_x.eps, -sqrt2_2)); // d/dx cos(x) = -sin(x)
    }

    #[test]
    fn test_dual_exp_ln() {
        // At x = 1: exp(1) = e, d/dx exp(x) = exp(x)
        let x = Dual::new(1.0, 1.0);
        let exp_x = x.exp();

        assert!(approx_eq(exp_x.val, std::f64::consts::E));
        assert!(approx_eq(exp_x.eps, std::f64::consts::E)); // d/dx exp(x) = exp(x)

        // At x = e: ln(e) = 1, d/dx ln(x) = 1/x
        let y = Dual::new(std::f64::consts::E, 1.0);
        let ln_y = y.ln();

        assert!(approx_eq(ln_y.val, 1.0));
        assert!(approx_eq(ln_y.eps, 1.0 / std::f64::consts::E)); // d/dx ln(x) = 1/x
    }

    #[test]
    fn test_dual_sqrt() {
        // At x = 4: sqrt(4) = 2, d/dx sqrt(x) = 1/(2*sqrt(x)) = 1/4
        let x = Dual::new(4.0, 1.0);
        let sqrt_x = x.sqrt();

        assert!(approx_eq(sqrt_x.val, 2.0));
        assert!(approx_eq(sqrt_x.eps, 0.25));
    }

    #[test]
    fn test_dual_powi() {
        // At x = 2: x^3 = 8, d/dx x^3 = 3x^2 = 12
        let x = Dual::new(2.0, 1.0);
        let x_cubed = x.powi(3);

        assert!(approx_eq(x_cubed.val, 8.0));
        assert!(approx_eq(x_cubed.eps, 12.0));
    }

    #[test]
    fn test_dual_chain_rule() {
        // f(x) = sin(x^2), f'(x) = 2x * cos(x^2)
        let x = Dual::new(2.0, 1.0);
        let x_squared = x * x;
        let result = x_squared.sin();

        assert!(approx_eq(result.val, 4.0_f64.sin()));
        // f'(2) = 2*2 * cos(4) = 4 * cos(4)
        assert!(approx_eq(result.eps, 4.0 * 4.0_f64.cos()));
    }

    #[test]
    fn test_dual_tan() {
        use PI;

        // At x = π/4: tan(π/4) = 1, d/dx tan(x) = sec^2(x) = 2
        let x = Dual::new(PI / 4.0, 1.0);
        let tan_x = x.tan();

        assert!(approx_eq(tan_x.val, 1.0));
        assert!(approx_eq(tan_x.eps, 2.0)); // sec^2(π/4) = 1/cos^2(π/4) = 2
    }

    #[test]
    fn test_dual_hyperbolic() {
        // At x = 0: sinh(0) = 0, cosh(0) = 1, tanh(0) = 0
        // d/dx sinh(x) = cosh(x) = 1 at x=0
        // d/dx cosh(x) = sinh(x) = 0 at x=0
        // d/dx tanh(x) = sech^2(x) = 1 at x=0
        let x = Dual::new(0.0, 1.0);

        let sinh_x = x.sinh();
        assert!(approx_eq(sinh_x.val, 0.0));
        assert!(approx_eq(sinh_x.eps, 1.0));

        let cosh_x = x.cosh();
        assert!(approx_eq(cosh_x.val, 1.0));
        assert!(approx_eq(cosh_x.eps, 0.0));

        let tanh_x = x.tanh();
        assert!(approx_eq(tanh_x.val, 0.0));
        assert!(approx_eq(tanh_x.eps, 1.0));
    }

    #[test]
    fn test_dual_erf() {
        // d/dx erf(x) = (2/√π) * e^(-x²)
        let x = Dual::new(0.0, 1.0);
        let result = x.erf();

        assert!(approx_eq(result.val, 0.0)); // erf(0) = 0
        // erf'(0) = 2/√π ≈ 1.1284
        let expected_deriv = 2.0 / PI.sqrt();
        assert!(approx_eq(result.eps, expected_deriv));
    }

    #[test]
    fn test_dual_gamma() {
        // Γ(1) = 1, Γ'(1) = -γ (Euler-Mascheroni constant, ≈ -0.5772)
        let x = Dual::new(2.0, 1.0);
        let result = x.gamma();

        assert!(approx_eq(result.val, 1.0)); // Γ(2) = 1! = 1
        // Γ'(2) = Γ(2) * ψ(2) = 1 * (1 - γ) ≈ 0.4228
        assert!(result.eps > 0.0 && result.eps < 1.0);
    }

    #[test]
    fn test_dual_sinc() {
        // sinc(0) = 1, sinc'(0) = 0
        let x = Dual::new(0.0, 1.0);
        let result = x.sinc();

        assert!(approx_eq(result.val, 1.0));
        assert!(approx_eq(result.eps, 0.0));

        // At x = π: sinc(π) = 0, sinc'(π) = -1/π
        let x_pi = Dual::new(PI, 1.0);
        let result_pi = x_pi.sinc();
        assert!(result_pi.val.abs() < 1e-10);
    }

    #[test]
    fn test_dual_lambert_w() {
        // W(0) = 0, W'(0) = 1
        let x = Dual::new(1.0, 1.0);
        let result = x.lambert_w();

        // W(1) ≈ 0.5671
        assert!(result.val > 0.5 && result.val < 0.6);
        // W'(1) = W(1) / (1 * (1 + W(1)))
        let expected_deriv = result.val / (1.0 * (1.0 + result.val));
        assert!((result.eps - expected_deriv).abs() < 1e-8);
    }
}
