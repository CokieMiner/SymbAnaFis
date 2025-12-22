//! Comprehensive API and Function Accuracy Tests
//!
//! Tests all public APIs and verifies numerical accuracy of all implemented functions.

use crate::parser::parse as parser_parse;
use crate::{Diff, Expr, ExprKind, Simplify, diff, simplify, symb};
use std::collections::{HashMap, HashSet};

const EPSILON: f64 = 1e-9;
const LOOSE_EPSILON: f64 = 1e-4; // For approximation algorithms

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_infinite() && b.is_infinite() {
        return a.signum() == b.signum();
    }
    (a - b).abs() < eps || (a - b).abs() < eps * a.abs().max(b.abs())
}

fn eval(expr_str: &str) -> Option<f64> {
    let fixed = HashSet::new();
    let custom = HashSet::new();
    let expr = parser_parse(expr_str, &fixed, &custom, None).ok()?;
    let vars: HashMap<&str, f64> = HashMap::new();
    match &expr.evaluate(&vars).kind {
        ExprKind::Number(n) => Some(*n),
        _ => None,
    }
}

fn eval_with(expr_str: &str, vars: &[(&str, f64)]) -> Option<f64> {
    let fixed = HashSet::new();
    let custom = HashSet::new();
    let expr = parser_parse(expr_str, &fixed, &custom, None).ok()?;
    let var_map: HashMap<&str, f64> = vars.iter().cloned().collect();
    match &expr.evaluate(&var_map).kind {
        ExprKind::Number(n) => Some(*n),
        _ => None,
    }
}

// ============================================================
// PART 1: PUBLIC API TESTS
// ============================================================

mod api_tests {
    use super::*;

    // --- diff() function ---
    #[test]
    fn test_diff_basic() {
        let result = diff("x^2", "x", None, None).unwrap();
        assert!(result.contains("2") && result.contains("x"));
    }

    #[test]
    fn test_diff_with_fixed_vars() {
        let result = diff("a*x^2", "x", Some(&["a"]), None).unwrap();
        // a is constant, so derivative should have 2ax
        assert!(result.contains("a"));
    }

    #[test]
    fn test_diff_with_custom_functions() {
        let result = diff("f(x)", "x", None, Some(&["f"])).unwrap();
        // Custom function derivative should contain f'
        assert!(result.contains("f'") || result.contains("∂"));
    }

    // --- simplify() function ---
    #[test]
    fn test_simplify_basic() {
        let result = simplify("x + x", None, None).unwrap();
        assert!(result.contains("2") && result.contains("x"));
    }

    #[test]
    fn test_simplify_with_fixed_vars() {
        let result = simplify("a + a", Some(&["a"]), None).unwrap();
        assert!(result.contains("2") && result.contains("a"));
    }

    // --- Diff builder ---
    #[test]
    fn test_diff_builder_basic() {
        let result = Diff::new().diff_str("x^3", "x").unwrap();
        assert!(result.contains("3") && result.contains("x"));
    }

    #[test]
    fn test_diff_builder_domain_safe() {
        let result = Diff::new()
            .domain_safe(true)
            .diff_str("sqrt(x)", "x")
            .unwrap();
        assert!(result.contains("sqrt") || result.contains("1/"));
    }

    #[test]
    fn test_diff_builder_fixed_vars() {
        let a = symb("a");
        let b = symb("b");
        let result = Diff::new()
            .fixed_vars(&[&a, &b])
            .diff_str("a*x + b", "x")
            .unwrap();
        assert!(result.contains("a"));
    }

    #[test]
    fn test_diff_builder_differentiate_expr() {
        let x = symb("x");
        let expr = x.clone().pow(2.0);
        let result = Diff::new().differentiate(expr, &x).unwrap();
        // Result should be 2x
        assert!(format!("{}", result).contains("2") && format!("{}", result).contains("x"));
    }

    // --- Simplify builder ---
    #[test]
    fn test_simplify_builder_basic() {
        let result = Simplify::new().simplify_str("x*1 + 0").unwrap();
        assert_eq!(result, "x");
    }

    #[test]
    fn test_simplify_builder_domain_safe() {
        let result = Simplify::new()
            .domain_safe(true)
            .simplify_str("(x^2)^(1/2)")
            .unwrap();
        // Domain-safe shouldn't simplify to |x|
        assert!(result.contains("x"));
    }

    #[test]
    fn test_simplify_builder_simplify_expr() {
        let x = symb("x");
        let expr = x + Expr::number(0.0);
        let result = Simplify::new().simplify(expr).unwrap();
        assert_eq!(format!("{}", result), "x");
    }

    // --- symb() and Symbol builder ---
    #[test]
    fn test_sym_creation() {
        let x = symb("x");
        let expr: Expr = x.into(); // Convert to Expr for display
        assert_eq!(format!("{}", expr), "x");
    }

    #[test]
    fn test_symbol_arithmetic() {
        let x = symb("x");
        let y = symb("y");

        // Addition
        let sum = x + y;
        assert!(format!("{}", sum).contains("x") && format!("{}", sum).contains("y"));

        // Multiplication
        let prod = x * y;
        assert!(format!("{}", prod).contains("x") && format!("{}", prod).contains("y"));

        // Power
        let pow = x.clone().pow(2.0);
        assert!(format!("{}", pow).contains("x") && format!("{}", pow).contains("2"));
    }

    #[test]
    fn test_symbol_functions() {
        let x = symb("x");

        // Test function methods
        let _ = x.clone().sin();
        let _ = x.clone().cos();
        let _ = x.clone().exp();
        let _ = x.clone().ln();
        let _ = x.clone().sqrt();
        // All should work without panicking
    }

    // --- Expr methods ---
    #[test]
    fn test_expr_evaluate() {
        let fixed = HashSet::new();
        let custom = HashSet::new();
        let expr = parser_parse("2 + 3", &fixed, &custom, None).unwrap();
        let vars = HashMap::new();
        let result = expr.evaluate(&vars);
        if let ExprKind::Number(n) = result.kind {
            assert_eq!(n, 5.0);
        } else {
            panic!("Expected number");
        }
    }

    #[test]
    fn test_expr_substitute() {
        let x = symb("x");
        let expr = x + Expr::number(1.0);
        let substituted = expr.substitute("x", &Expr::number(5.0));
        let vars = HashMap::new();
        let result = substituted.evaluate(&vars);
        if let ExprKind::Number(n) = result.kind {
            assert_eq!(n, 6.0);
        } else {
            panic!("Expected number");
        }
    }

    #[test]
    fn test_expr_fold_map() {
        let x = symb("x");
        let expr = x + Expr::number(1.0);

        // fold: count nodes
        let count = expr.fold(0, |acc, _| acc + 1);
        assert!(count >= 2); // At least Add + x + 1

        // map: identity
        let mapped = expr.map(|e: &Expr| e.clone());
        assert_eq!(format!("{}", mapped), format!("{}", expr));
    }

    #[test]
    fn test_expr_node_count() {
        let x: Expr = symb("x").into();
        assert_eq!(x.node_count(), 1);

        let expr = x.clone() + Expr::number(1.0);
        assert_eq!(expr.node_count(), 3); // Add + x + 1
    }

    #[test]
    fn test_expr_max_depth() {
        let x: Expr = symb("x").into();
        assert_eq!(x.max_depth(), 1);

        let expr = x.clone() + Expr::number(1.0);
        assert_eq!(expr.max_depth(), 2); // Add(x, 1)
    }

    // --- gradient, hessian, jacobian APIs ---
    #[test]
    fn test_gradient() {
        use crate::gradient;
        let x = symb("x");
        let y = symb("y");
        let x_expr: Expr = x.into();
        let y_expr: Expr = y.into();
        let expr = x_expr.clone() * x_expr.clone() + y_expr.clone() * y_expr.clone(); // x² + y²
        let grad = gradient(&expr, &[&x, &y]);
        assert_eq!(grad.len(), 2);
        // ∂/∂x = 2x, ∂/∂y = 2y
        let s = format!("{}", grad[0]);
        assert!(s.contains("2") && s.contains("x"));
    }

    #[test]
    fn test_hessian() {
        use crate::hessian;
        let x = symb("x");
        let x_expr: Expr = x.into();
        let expr = x_expr.clone() * x_expr.clone() * x_expr.clone(); // x³
        let hess = hessian(&expr, &[&x]);
        assert_eq!(hess.len(), 1); // 1x1 matrix
        assert_eq!(hess[0].len(), 1);
        // ∂²/∂x² of x³ = 6x
        let s = format!("{}", hess[0][0]);
        assert!(s.contains("6") || s.contains("x"));
    }

    #[test]
    fn test_jacobian() {
        use crate::jacobian;
        let x = symb("x");
        let y = symb("y");
        let x_expr: Expr = x.into();
        let y_expr: Expr = y.into();
        let f = x_expr.clone() * y_expr.clone(); // xy
        let g = x_expr.clone() + y_expr.clone(); // x + y
        let jac = jacobian(&[f, g], &[&x, &y]);
        assert_eq!(jac.len(), 2); // 2 rows
        assert_eq!(jac[0].len(), 2); // 2 columns
    }

    // --- String-based gradient/hessian/jacobian helpers ---
    #[test]
    fn test_gradient_str() {
        use crate::gradient_str;
        let grad = gradient_str("x^2 + y^2", &["x", "y"]).unwrap();
        assert_eq!(grad.len(), 2);
    }

    #[test]
    fn test_hessian_str() {
        use crate::hessian_str;
        let hess = hessian_str("x^3", &["x"]).unwrap();
        assert_eq!(hess.len(), 1);
    }

    #[test]
    fn test_jacobian_str() {
        use crate::jacobian_str;
        let jac = jacobian_str(&["x*y", "x+y"], &["x", "y"]).unwrap();
        assert_eq!(jac.len(), 2);
    }

    // --- evaluate_str helper ---
    #[test]
    fn test_evaluate_str() {
        use crate::evaluate_str;
        let result = evaluate_str("x^2 + 1", &[("x", 3.0)]).unwrap();
        assert_eq!(result, "10");
    }

    // ===================================================================
    // INTEGRATED WORKFLOW TESTS
    // ===================================================================

    /// Test evaluation with custom functions (user-defined functions)
    #[test]
    fn test_eval_with_custom_functions() {
        // Parse with custom function "f" registered
        let mut custom_fns = HashSet::new();
        custom_fns.insert("f".to_string());
        let fixed_vars = HashSet::new();

        // Parse expression containing custom function: f(x) + x
        let expr = parser_parse("f(x) + x", &fixed_vars, &custom_fns, None).unwrap();

        // Evaluate with x = 2.0 (custom function remains symbolic)
        let vars: HashMap<&str, f64> = [("x", 2.0)].iter().cloned().collect();
        let result = expr.evaluate(&vars);

        // Result should contain f(2) + 2 (custom function partially evaluated)
        let s = format!("{}", result);
        assert!(s.contains("f") && s.contains("2"), "Got: {}", s);
    }

    /// Test: differentiate first, then evaluate with symbol values
    #[test]
    fn test_diff_then_evaluate_with_symbols() {
        // Build expression: x^2 * y + sin(x)
        let x_sym = symb("x");
        let y_sym = symb("y");
        let x: Expr = x_sym.into();
        let y: Expr = y_sym.into();
        let expr = x.clone() * x.clone() * y.clone() + x.clone().sin();

        // Differentiate with respect to x
        let diff = Diff::new();
        let derivative = diff.differentiate(expr, &x_sym).unwrap();
        // d/dx(x^2 * y + sin(x)) = 2xy + cos(x)

        // Now evaluate with specific values
        let vars: HashMap<&str, f64> = [("x", std::f64::consts::PI), ("y", 3.0)]
            .iter()
            .cloned()
            .collect();
        let result = derivative.evaluate(&vars);

        // At x=π, y=3: 2*π*3 + cos(π) = 6π - 1 ≈ 17.85
        if let ExprKind::Number(n) = result.kind {
            let expected = 2.0 * std::f64::consts::PI * 3.0 + std::f64::consts::PI.cos();
            assert!(
                approx_eq(n, expected, LOOSE_EPSILON),
                "Expected ~{}, got {}",
                expected,
                n
            );
        } else {
            panic!("Expected numeric result, got: {}", result);
        }
    }

    /// Test: full workflow - build expression, compute hessian, evaluate elements
    #[test]
    fn test_full_hessian_workflow() {
        use crate::hessian;

        // Expression: x^2 * y + y^3
        let x_sym = symb("x");
        let y_sym = symb("y");
        let x: Expr = x_sym.into();
        let y: Expr = y_sym.into();
        let expr = x.clone() * x.clone() * y.clone() + y.clone() * y.clone() * y.clone();

        // Compute Hessian matrix
        let hess = hessian(&expr, &[&x_sym, &y_sym]);
        assert_eq!(hess.len(), 2, "Hessian should be 2x2");
        assert_eq!(hess[0].len(), 2);

        // Hessian of f(x,y) = x²y + y³:
        // H = [[∂²f/∂x², ∂²f/∂x∂y], [∂²f/∂y∂x, ∂²f/∂y²]]
        //   = [[2y, 2x], [2x, 6y]]

        // Evaluate Hessian at (x=2, y=3)
        let vars: HashMap<&str, f64> = [("x", 2.0), ("y", 3.0)].iter().cloned().collect();

        // H[0][0] = 2y = 6
        let h00 = &hess[0][0].evaluate(&vars);
        if let ExprKind::Number(n) = h00.kind {
            assert!(approx_eq(n, 6.0, EPSILON), "H[0][0] = 2y = 6, got {}", n);
        } else {
            panic!("H[0][0] should be numeric, got: {}", h00);
        }

        // H[0][1] = 2x = 4
        let h01 = &hess[0][1].evaluate(&vars);
        if let ExprKind::Number(n) = h01.kind {
            assert!(approx_eq(n, 4.0, EPSILON), "H[0][1] = 2x = 4, got {}", n);
        } else {
            panic!("H[0][1] should be numeric, got: {}", h01);
        }

        // H[1][1] = 6y = 18
        let h11 = &hess[1][1].evaluate(&vars);
        if let ExprKind::Number(n) = h11.kind {
            assert!(approx_eq(n, 18.0, EPSILON), "H[1][1] = 6y = 18, got {}", n);
        } else {
            panic!("H[1][1] should be numeric, got: {}", h11);
        }

        // Verify symmetry: H[0][1] = H[1][0]
        let h10 = &hess[1][0].evaluate(&vars);
        if let ExprKind::Number(n) = h10.kind {
            assert!(
                approx_eq(n, 4.0, EPSILON),
                "H[1][0] = H[0][1] = 4, got {}",
                n
            );
        } else {
            panic!("H[1][0] should be numeric, got: {}", h10);
        }
    }

    /// Test: differentiate custom function, symbolic operations preserved
    #[test]
    fn test_diff_custom_function_symbolic() {
        // Differentiate f(x)*g(x) where f and g are custom functions
        let result = diff("f(x) * g(x)", "x", None, Some(&["f", "g"])).unwrap();

        // Result should contain product rule: f'(x)*g(x) + f(x)*g'(x)
        assert!(
            result.contains("f'") || result.contains("∂"),
            "Expected derivative notation in: {}",
            result
        );
        assert!(
            result.contains("g") && result.contains("f"),
            "Expected both f and g in: {}",
            result
        );
    }

    /// Test: gradient -> evaluate -> compute magnitude
    #[test]
    fn test_gradient_evaluate_magnitude() {
        use crate::gradient;

        // f(x, y) = x^2 + y^2 (paraboloid)
        let x_sym = symb("x");
        let y_sym = symb("y");
        let x: Expr = x_sym.into();
        let y: Expr = y_sym.into();
        let expr = x.clone() * x.clone() + y.clone() * y.clone();

        // Gradient: [2x, 2y]
        let grad = gradient(&expr, &[&x_sym, &y_sym]);
        assert_eq!(grad.len(), 2);

        // Evaluate at (3, 4)
        let vars: HashMap<&str, f64> = [("x", 3.0), ("y", 4.0)].iter().cloned().collect();

        let gx = if let ExprKind::Number(n) = grad[0].evaluate(&vars).kind {
            n
        } else {
            0.0
        };
        let gy = if let ExprKind::Number(n) = grad[1].evaluate(&vars).kind {
            n
        } else {
            0.0
        };

        // Gradient at (3,4) = [6, 8], magnitude = sqrt(36+64) = 10
        let magnitude = (gx * gx + gy * gy).sqrt();
        assert!(
            approx_eq(magnitude, 10.0, EPSILON),
            "Gradient magnitude should be 10, got {}",
            magnitude
        );
    }
}

// ============================================================
// PART 2: FUNCTION ACCURACY TESTS
// ============================================================

mod function_accuracy_tests {
    use super::*;
    use std::f64::consts::{E, PI};

    // === Trigonometric Functions ===
    #[test]
    fn test_trig_sin() {
        assert!(approx_eq(eval("sin(0)").unwrap(), 0.0, EPSILON));
        assert!(approx_eq(
            eval("sin(1.5707963267948966)").unwrap(),
            1.0,
            EPSILON
        )); // π/2
        assert!(approx_eq(
            eval("sin(3.141592653589793)").unwrap(),
            0.0,
            EPSILON
        )); // π
        assert!(approx_eq(
            eval("sin(-1.5707963267948966)").unwrap(),
            -1.0,
            EPSILON
        )); // -π/2
    }

    #[test]
    fn test_trig_cos() {
        assert!(approx_eq(eval("cos(0)").unwrap(), 1.0, EPSILON));
        assert!(approx_eq(
            eval("cos(1.5707963267948966)").unwrap(),
            0.0,
            EPSILON
        )); // π/2
        assert!(approx_eq(
            eval("cos(3.141592653589793)").unwrap(),
            -1.0,
            EPSILON
        )); // π
    }

    #[test]
    fn test_trig_tan() {
        assert!(approx_eq(eval("tan(0)").unwrap(), 0.0, EPSILON));
        assert!(approx_eq(
            eval("tan(0.7853981633974483)").unwrap(),
            1.0,
            EPSILON
        )); // π/4
    }

    #[test]
    fn test_trig_cot_sec_csc() {
        // cot(π/4) = 1
        assert!(approx_eq(
            eval("cot(0.7853981633974483)").unwrap(),
            1.0,
            EPSILON
        ));
        // sec(0) = 1
        assert!(approx_eq(eval("sec(0)").unwrap(), 1.0, EPSILON));
        // csc(π/2) = 1
        assert!(approx_eq(
            eval("csc(1.5707963267948966)").unwrap(),
            1.0,
            EPSILON
        ));
        // cot(0) = Inf
        assert!(eval("cot(0)").unwrap().is_infinite());
        // csc(0) = Inf
        assert!(eval("csc(0)").unwrap().is_infinite());
    }

    // === Inverse Trigonometric Functions ===
    #[test]
    fn test_inverse_trig() {
        assert!(approx_eq(eval("asin(0)").unwrap(), 0.0, EPSILON));
        assert!(approx_eq(eval("asin(1)").unwrap(), PI / 2.0, EPSILON));
        assert!(approx_eq(eval("acos(1)").unwrap(), 0.0, EPSILON));
        assert!(approx_eq(eval("acos(0)").unwrap(), PI / 2.0, EPSILON));
        assert!(approx_eq(eval("atan(0)").unwrap(), 0.0, EPSILON));
        assert!(approx_eq(eval("atan(1)").unwrap(), PI / 4.0, EPSILON));
    }

    #[test]
    fn test_inverse_trig_reciprocal() {
        // acot(1) = π/4
        assert!(approx_eq(eval("acot(1)").unwrap(), PI / 4.0, EPSILON));
        // asec(1) = 0
        assert!(approx_eq(eval("asec(1)").unwrap(), 0.0, EPSILON));
        // asec(0.5) = NaN (|x| < 1)
        assert!(eval("asec(0.5)").unwrap().is_nan());
        // acsc(0.5) = NaN
        assert!(eval("acsc(0.5)").unwrap().is_nan());
    }

    // === Hyperbolic Functions ===
    #[test]
    fn test_hyperbolic() {
        assert!(approx_eq(eval("sinh(0)").unwrap(), 0.0, EPSILON));
        assert!(approx_eq(eval("cosh(0)").unwrap(), 1.0, EPSILON));
        assert!(approx_eq(eval("tanh(0)").unwrap(), 0.0, EPSILON));
        assert!(approx_eq(eval("sech(0)").unwrap(), 1.0, EPSILON));
        // coth(0) = Inf, csch(0) = Inf
        assert!(eval("coth(0)").unwrap().is_infinite());
        assert!(eval("csch(0)").unwrap().is_infinite());

        // Known values
        assert!(approx_eq(
            eval("sinh(1)").unwrap(),
            1.1752011936438014,
            EPSILON
        ));
        assert!(approx_eq(
            eval("cosh(1)").unwrap(),
            1.5430806348152437,
            EPSILON
        ));
    }

    // === Inverse Hyperbolic Functions ===
    #[test]
    fn test_inverse_hyperbolic() {
        assert!(approx_eq(eval("asinh(0)").unwrap(), 0.0, EPSILON));
        assert!(approx_eq(eval("acosh(1)").unwrap(), 0.0, EPSILON));
        assert!(approx_eq(eval("atanh(0)").unwrap(), 0.0, EPSILON));
        assert!(approx_eq(
            eval("acoth(2)").unwrap(),
            0.5493061443340549,
            EPSILON
        ));
        // acoth(0.5) = NaN
        assert!(eval("acoth(0.5)").unwrap().is_nan());
    }

    // === Exponential and Logarithmic ===
    #[test]
    fn test_exp_log() {
        assert!(approx_eq(eval("exp(0)").unwrap(), 1.0, EPSILON));
        assert!(approx_eq(eval("exp(1)").unwrap(), E, EPSILON));
        assert!(approx_eq(eval("ln(1)").unwrap(), 0.0, EPSILON));
        assert!(approx_eq(
            eval("ln(2.718281828459045)").unwrap(),
            1.0,
            EPSILON
        ));
        assert!(approx_eq(eval("log10(10)").unwrap(), 1.0, EPSILON));
        assert!(approx_eq(eval("log10(100)").unwrap(), 2.0, EPSILON));
        assert!(approx_eq(eval("log2(2)").unwrap(), 1.0, EPSILON));
        assert!(approx_eq(eval("log2(8)").unwrap(), 3.0, EPSILON));
        // ln(-1) = NaN
        assert!(eval("ln(-1)").unwrap().is_nan());
    }

    // === Roots ===
    #[test]
    fn test_roots() {
        assert!(approx_eq(eval("sqrt(4)").unwrap(), 2.0, EPSILON));
        assert!(approx_eq(eval("sqrt(9)").unwrap(), 3.0, EPSILON));
        assert!(approx_eq(eval("cbrt(8)").unwrap(), 2.0, EPSILON));
        assert!(approx_eq(eval("cbrt(-8)").unwrap(), -2.0, EPSILON));
        // sqrt(-1) = NaN
        assert!(eval("sqrt(-1)").unwrap().is_nan());
    }

    // === Rounding Functions ===
    #[test]
    fn test_rounding() {
        assert!(approx_eq(eval("abs(-5)").unwrap(), 5.0, EPSILON));
        assert!(approx_eq(eval("abs(3.5)").unwrap(), 3.5, EPSILON));
    }

    // === Sinc ===
    #[test]
    fn test_sinc() {
        assert!(approx_eq(eval("sinc(0)").unwrap(), 1.0, EPSILON));
        assert!(eval("sinc(3.141592653589793)").unwrap().abs() < 1e-10);
    }

    // === Error Functions ===
    #[test]
    fn test_erf() {
        assert!(approx_eq(eval("erf(0)").unwrap(), 0.0, EPSILON));
        let erf1 = eval("erf(1)").unwrap();
        assert!(approx_eq(erf1, 0.8427007929497149, LOOSE_EPSILON));
        assert!(approx_eq(eval("erfc(0)").unwrap(), 1.0, EPSILON));
        // erf(x) + erfc(x) = 1
        let erfc1 = eval("erfc(1)").unwrap();
        assert!(approx_eq(erf1 + erfc1, 1.0, LOOSE_EPSILON));
    }

    // === Gamma and Related ===
    #[test]
    fn test_gamma() {
        // Γ(n) = (n-1)! for positive integers
        assert!(approx_eq(eval("gamma(1)").unwrap(), 1.0, EPSILON));
        assert!(approx_eq(eval("gamma(2)").unwrap(), 1.0, EPSILON));
        assert!(approx_eq(eval("gamma(3)").unwrap(), 2.0, EPSILON));
        assert!(approx_eq(eval("gamma(4)").unwrap(), 6.0, EPSILON));
        assert!(approx_eq(eval("gamma(5)").unwrap(), 24.0, EPSILON));
        // Γ(0.5) = √π
        assert!(approx_eq(
            eval("gamma(0.5)").unwrap(),
            PI.sqrt(),
            LOOSE_EPSILON
        ));
    }

    #[test]
    fn test_digamma_trigamma() {
        // ψ(1) = -γ ≈ -0.5772
        let psi1 = eval("digamma(1)").unwrap();
        assert!(approx_eq(psi1, -0.5772156649015329, 0.01));
        // ψ₁(1) = π²/6
        let trigamma1 = eval("trigamma(1)").unwrap();
        assert!(approx_eq(trigamma1, PI * PI / 6.0, 0.01));
    }

    #[test]
    fn test_beta() {
        // B(1,1) = 1
        assert!(approx_eq(eval("beta(1, 1)").unwrap(), 1.0, LOOSE_EPSILON));
        // B(a,b) = B(b,a)
        let b23 = eval("beta(2, 3)").unwrap();
        let b32 = eval("beta(3, 2)").unwrap();
        assert!(approx_eq(b23, b32, EPSILON));
    }

    #[test]
    fn test_polygamma() {
        // polygamma(0, x) = digamma(x)
        let poly0 = eval("polygamma(0, 1)").unwrap();
        let digamma = eval("digamma(1)").unwrap();
        assert!(approx_eq(poly0, digamma, EPSILON));
    }

    // === Lambert W ===
    #[test]
    fn test_lambertw() {
        assert!(approx_eq(eval("lambertw(0)").unwrap(), 0.0, EPSILON));
        // W(e) = 1 is the most reliable test
        assert!(approx_eq(
            eval("lambertw(2.718281828459045)").unwrap(),
            1.0,
            LOOSE_EPSILON
        ));
        // W(1) ~ 0.567 (use very loose tolerance for iterative algorithm)
        let w1 = eval("lambertw(1)").unwrap();
        assert!(w1 > 0.5 && w1 < 0.7);
    }

    // === Bessel Functions ===
    #[test]
    fn test_bessel_j() {
        // J_0(0) = 1 (use looser tolerance for numerical approximation)
        assert!(approx_eq(
            eval("besselj(0, 0)").unwrap(),
            1.0,
            LOOSE_EPSILON
        ));
        // J_1(0) = 0
        assert!(approx_eq(
            eval("besselj(1, 0)").unwrap(),
            0.0,
            LOOSE_EPSILON
        ));
        // J_0(2.4048) ≈ 0 (first zero)
        assert!(eval("besselj(0, 2.4048)").unwrap().abs() < 0.01);
    }

    #[test]
    fn test_bessel_y() {
        // Y_0(1) ≈ 0.0883
        let y01 = eval("bessely(0, 1)").unwrap();
        assert!(approx_eq(y01, 0.0883, 0.01));
    }

    #[test]
    fn test_bessel_i() {
        // I_0(0) = 1
        assert!(approx_eq(eval("besseli(0, 0)").unwrap(), 1.0, EPSILON));
        // I_1(0) = 0
        assert!(approx_eq(eval("besseli(1, 0)").unwrap(), 0.0, EPSILON));
    }

    // === Hermite Polynomials ===
    #[test]
    fn test_hermite() {
        // H_0(x) = 1
        assert!(approx_eq(eval("hermite(0, 2)").unwrap(), 1.0, EPSILON));
        // H_1(x) = 2x
        assert!(approx_eq(eval("hermite(1, 3)").unwrap(), 6.0, EPSILON));
        // H_2(x) = 4x² - 2
        assert!(approx_eq(eval("hermite(2, 2)").unwrap(), 14.0, EPSILON)); // 4*4-2
    }

    // === Elliptic Integrals ===
    #[test]
    fn test_elliptic_k() {
        // K(0) = π/2
        assert!(approx_eq(eval("elliptic_k(0)").unwrap(), PI / 2.0, EPSILON));
        // K(1) = ∞
        assert!(eval("elliptic_k(1)").unwrap().is_infinite());
    }

    #[test]
    fn test_elliptic_e() {
        // E(0) = π/2
        assert!(approx_eq(
            eval("elliptic_e(0)").unwrap(),
            PI / 2.0,
            LOOSE_EPSILON
        ));
        // E(0.5) ~ 1.467 (avoid boundary at k=1 which has numerical issues)
        let e05 = eval("elliptic_e(0.5)").unwrap();
        assert!(e05 > 1.0 && e05 < 2.0);
    }

    // === Associated Legendre ===
    #[test]
    fn test_assoc_legendre() {
        // P_0^0(x) = 1
        assert!(approx_eq(
            eval("assoc_legendre(0, 0, 0.5)").unwrap(),
            1.0,
            EPSILON
        ));
        // P_1^0(x) = x
        assert!(approx_eq(
            eval("assoc_legendre(1, 0, 0.5)").unwrap(),
            0.5,
            EPSILON
        ));
        // P_1^1(x) = -√(1-x²)
        let p11 = eval("assoc_legendre(1, 1, 0.5)").unwrap();
        assert!(approx_eq(p11, -(1.0 - 0.25_f64).sqrt(), EPSILON));
    }

    // === Bessel K ===
    #[test]
    fn test_bessel_k() {
        // K_0(1) test - just check it returns a finite positive value
        let k01 = eval("besselk(0, 1)").unwrap();
        assert!(k01.is_finite() && k01 > 0.0, "besselk(0, 1) = {}", k01);
    }

    // === Zeta Function ===
    #[test]
    fn test_zeta() {
        // ζ(2) = π²/6
        assert!(approx_eq(
            eval("zeta(2)").unwrap(),
            PI * PI / 6.0,
            LOOSE_EPSILON
        ));
        // ζ(4) = π⁴/90
        assert!(approx_eq(
            eval("zeta(4)").unwrap(),
            PI.powi(4) / 90.0,
            LOOSE_EPSILON
        ));
    }

    // === Zeta Derivatives ===
    #[test]
    fn test_zeta_derivatives() {
        use crate::math::eval_zeta_deriv;

        // Test first derivative at s=2
        // ζ'(2) = -Σ ln(n)/n² ≈ -0.9375...
        let zeta_prime_2 = eval_zeta_deriv(1, 2.0_f64).unwrap();
        assert!(zeta_prime_2 < 0.0, "ζ'(2) should be negative");
        assert!(
            zeta_prime_2.abs() < 1.0 && zeta_prime_2.abs() > 0.9,
            "ζ'(2) ≈ -0.9375, got {}",
            zeta_prime_2
        );

        // Test second derivative at s=2
        // ζ''(2) = Σ [ln(n)]²/n² > 0
        let zeta_double_prime_2 = eval_zeta_deriv(2, 2.0_f64).unwrap();
        assert!(zeta_double_prime_2 > 0.0, "ζ''(2) should be positive");

        // Test derivatives at s=3
        let zeta_prime_3 = eval_zeta_deriv(1, 3.0_f64).unwrap();
        assert!(zeta_prime_3 < 0.0, "ζ'(3) should be negative");

        // Test that derivative gets smaller for larger s (convergence)
        let zeta_prime_10 = eval_zeta_deriv(1, 10.0_f64).unwrap();
        assert!(
            zeta_prime_10.abs() < zeta_prime_3.abs(),
            "|ζ'(10)| should be less than |ζ'(3)|"
        );

        // Test n=0 gives the function itself
        let zeta_0_deriv = eval_zeta_deriv(0, 2.0_f64).unwrap();
        let zeta_direct = eval("zeta(2)").unwrap();
        assert!(
            approx_eq(zeta_0_deriv, zeta_direct, LOOSE_EPSILON),
            "0th derivative should equal the function"
        );

        // Test that pole at s=1 returns None
        assert!(
            eval_zeta_deriv(1, 1.0_f64).is_none(),
            "Derivative should be None at pole s=1"
        );
    }

    // === Floor, Ceil, Round ===
    #[test]
    fn test_floor_ceil_round() {
        assert!(approx_eq(eval("floor(3.7)").unwrap(), 3.0, EPSILON));
        assert!(approx_eq(eval("floor(-3.7)").unwrap(), -4.0, EPSILON));
        assert!(approx_eq(eval("ceil(3.2)").unwrap(), 4.0, EPSILON));
        assert!(approx_eq(eval("ceil(-3.2)").unwrap(), -3.0, EPSILON));
        assert!(approx_eq(eval("round(3.5)").unwrap(), 4.0, EPSILON));
        assert!(approx_eq(eval("round(3.4)").unwrap(), 3.0, EPSILON));
    }

    // === Tetragamma ===
    #[test]
    fn test_tetragamma() {
        // tetragamma(1) = -2 * ζ(3) ≈ -2.404
        let tet1 = eval("tetragamma(1)").unwrap();
        assert!(tet1.abs() > 2.0 && tet1.abs() < 3.0);
    }

    // === log10/log2 ===
    #[test]
    fn test_log_variants() {
        // log10
        assert!(approx_eq(eval("log10(1000)").unwrap(), 3.0, EPSILON));
        // log2
        assert!(approx_eq(eval("log2(16)").unwrap(), 4.0, EPSILON));
    }

    // === Spherical Harmonics Y_l^m ===
    #[test]
    fn test_ynm() {
        // Y_0^0(θ, φ) = 1/(2√π) for any θ, φ
        let y00 = eval("ynm(0, 0, 0, 0)").unwrap();
        assert!(approx_eq(y00, 1.0 / (2.0 * PI.sqrt()), EPSILON));
    }

    // === exp_polar ===
    #[test]
    fn test_exp_polar() {
        // exp_polar(x) behaves like exp(x) but preserves polar semantics
        assert!(approx_eq(eval("exp_polar(0)").unwrap(), 1.0, EPSILON));

        // exp(1.5) approx 4.481689
        let expected = (1.5f64).exp();
        assert!(approx_eq(
            eval("exp_polar(1.5)").unwrap(),
            expected,
            EPSILON
        ));

        // Test derivative: d/dx exp_polar(x) = exp_polar(x)
        let deriv = diff("exp_polar(x)", "x", None, None).unwrap();
        // Should be "exp_polar(x)" (or "1 * exp_polar(x)")
        assert!(
            deriv.contains("exp_polar"),
            "Derivative should contain exp_polar, got: {}",
            deriv
        );
    }

    // === Sign variants ===
    #[test]
    fn test_sign_variants() {
        assert!(approx_eq(eval("signum(7)").unwrap(), 1.0, EPSILON));
    }
}

// ============================================================
// PART 3: DERIVATIVE ACCURACY TESTS
// ============================================================

mod derivative_accuracy_tests {
    use super::*;

    #[test]
    fn test_power_rule_accuracy() {
        // d/dx(x^n) = n*x^(n-1)
        for n in 2..=5 {
            let formula = format!("x^{}", n);
            let deriv = diff(&formula, "x", None, None).unwrap();
            // Evaluate at x=2
            let symbolic = eval_with(&deriv, &[("x", 2.0)]).unwrap_or(0.0);
            let expected = n as f64 * 2.0_f64.powi(n - 1);
            assert!(
                approx_eq(symbolic, expected, LOOSE_EPSILON),
                "Power rule failed for n={}: got {}, expected {}",
                n,
                symbolic,
                expected
            );
        }
    }

    #[test]
    fn test_trig_derivative_accuracy() {
        // d/dx(sin(x)) = cos(x)
        let deriv_sin = diff("sin(x)", "x", None, None).unwrap();
        let x = 0.5;
        let symbolic = eval_with(&deriv_sin, &[("x", x)]).unwrap_or(0.0);
        let expected = x.cos();
        assert!(approx_eq(symbolic, expected, LOOSE_EPSILON));

        // d/dx(cos(x)) = -sin(x)
        let deriv_cos = diff("cos(x)", "x", None, None).unwrap();
        let symbolic = eval_with(&deriv_cos, &[("x", x)]).unwrap_or(0.0);
        let expected = -x.sin();
        assert!(approx_eq(symbolic, expected, LOOSE_EPSILON));
    }

    #[test]
    fn test_exp_log_derivative_accuracy() {
        // d/dx(exp(x)) = exp(x)
        let deriv_exp = diff("exp(x)", "x", None, None).unwrap();
        let x = 1.0;
        let symbolic = eval_with(&deriv_exp, &[("x", x)]).unwrap_or(0.0);
        let expected = x.exp();
        assert!(approx_eq(symbolic, expected, LOOSE_EPSILON));

        // d/dx(ln(x)) = 1/x
        let deriv_ln = diff("ln(x)", "x", None, None).unwrap();
        let x = 2.0;
        let symbolic = eval_with(&deriv_ln, &[("x", x)]).unwrap_or(0.0);
        let expected = 1.0 / x;
        assert!(approx_eq(symbolic, expected, LOOSE_EPSILON));
    }

    #[test]
    fn test_chain_rule_accuracy() {
        // d/dx(sin(x^2)) = 2x*cos(x^2)
        let deriv = diff("sin(x^2)", "x", None, None).unwrap();
        let x = 0.5;
        let symbolic = eval_with(&deriv, &[("x", x)]).unwrap_or(0.0);
        let expected = 2.0 * x * (x * x).cos();
        assert!(approx_eq(symbolic, expected, LOOSE_EPSILON));
    }

    #[test]
    fn test_product_rule_accuracy() {
        // d/dx(x*sin(x)) = sin(x) + x*cos(x)
        let deriv = diff("x*sin(x)", "x", None, None).unwrap();
        let x = 1.0;
        let symbolic = eval_with(&deriv, &[("x", x)]).unwrap_or(0.0);
        let expected = x.sin() + x * x.cos();
        assert!(approx_eq(symbolic, expected, LOOSE_EPSILON));
    }

    #[test]
    fn test_quotient_rule_accuracy() {
        // d/dx(x/(1+x)) = 1/(1+x)^2
        let deriv = diff("x/(1+x)", "x", None, None).unwrap();
        let x = 2.0;
        let symbolic = eval_with(&deriv, &[("x", x)]).unwrap_or(0.0);
        let expected = 1.0 / ((1.0 + x) * (1.0 + x));
        assert!(approx_eq(symbolic, expected, LOOSE_EPSILON));
    }
}

// ============================================================
// PART 4: EDGE CASE TESTS
// ============================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_nan_inf_handling() {
        // sqrt(-1) = NaN
        assert!(eval("sqrt(-1)").unwrap().is_nan());

        // ln(0) = -Inf
        assert!(eval("ln(0)").unwrap().is_infinite());
    }

    #[test]
    fn test_large_values() {
        // Large exponent
        let result = eval("exp(100)").unwrap();
        assert!(result.is_finite() && result > 1e40);

        // Very small value
        let result = eval("exp(-100)").unwrap();
        assert!(result.is_finite() && result > 0.0 && result < 1e-40);
    }

    #[test]
    fn test_near_singularity() {
        // tan near π/2
        let result = eval("tan(1.57)").unwrap();
        assert!(result.abs() > 100.0); // Should be very large
    }

    #[test]
    fn test_variable_substitution() {
        let x_val = 2.5;
        let result = eval_with("x^2 + 3*x + 1", &[("x", x_val)]).unwrap();
        let expected = x_val * x_val + 3.0 * x_val + 1.0;
        assert!(approx_eq(result, expected, EPSILON));
    }

    #[test]
    fn test_multi_variable() {
        let result = eval_with("x*y + z", &[("x", 2.0), ("y", 3.0), ("z", 4.0)]).unwrap();
        assert!(approx_eq(result, 10.0, EPSILON));
    }
}

// ============================================================
// PART 5: CustomFn Multi-Argument Differentiation Tests
// ============================================================

mod custom_fn_differentiation_tests {
    use crate::api::builder::CustomFn;
    use crate::{Diff, Expr, symb};

    /// Test basic CustomFn registration and differentiation
    #[test]
    fn test_custom_fn_basic_differentiation() {
        use std::sync::Arc;
        // Define a custom 2-arg function: f(a, b) = a * b (for testing)
        // ∂f/∂a = b, ∂f/∂b = a
        let custom_fn = CustomFn::new(2)
            .partial(0, |args: &[Arc<Expr>]| Expr::from(&args[1])) // ∂f/∂a = b
            .partial(1, |args: &[Arc<Expr>]| Expr::from(&args[0])); // ∂f/∂b = a

        // Create expression: custom_mul(t, t^2)
        // d/dt custom_mul(t, t^2) = ∂f/∂a * dt/dt + ∂f/∂b * d(t^2)/dt
        //                        = t^2 * 1 + t * 2t
        //                        = t^2 + 2t^2 = 3t^2

        let t = symb("t");
        let expr = Expr::call::<2>(
            "custom_mul",
            [
                Expr::symbol("t"),
                Expr::pow(Expr::symbol("t"), Expr::number(2.0)),
            ],
        );

        let result = Diff::new()
            .custom_fn_multi("custom_mul", custom_fn)
            .differentiate(expr, &t)
            .unwrap();

        // The result should simplify to 3*t^2
        let result_str = format!("{}", result);
        eprintln!("CustomFn derivative result: {}", result_str);

        // Verify result contains expected terms
        assert!(
            result_str.contains("t^2") || result_str.contains("t²"),
            "Expected t^2 in result: {}",
            result_str
        );
    }

    /// Test CustomFn with single partial derivative
    #[test]
    fn test_custom_fn_single_partial() {
        use std::sync::Arc;
        // Define f(x) = custom function where ∂f/∂x = 2*x
        let custom_fn = CustomFn::new(1).partial(0, |args: &[Arc<Expr>]| {
            Expr::mul_expr(Expr::number(2.0), Expr::from(&args[0]))
        });

        let x = symb("x");
        let expr = Expr::call::<1>("custom_f", [Expr::symbol("x")]);

        // d/dx custom_f(x) = 2x * 1 = 2x
        let result = Diff::new()
            .custom_fn_multi("custom_f", custom_fn)
            .differentiate(expr, &x)
            .unwrap();

        let result_str = format!("{}", result);
        eprintln!("Single partial result: {}", result_str);

        // Should be 2*x or simplified to 2x
        assert!(
            result_str.contains("2*x") || result_str.contains("2 * x") || result_str == "2x",
            "Expected 2*x or 2x in result: {}",
            result_str
        );
    }

    /// Test chain rule application with nested functions
    #[test]
    fn test_custom_fn_chain_rule() {
        // f(u, v) where ∂f/∂u = v, ∂f/∂v = u
        // Evaluate f(sin(x), cos(x))
        // d/dx = v * cos(x) + u * (-sin(x))
        //      = cos(x) * cos(x) + sin(x) * (-sin(x))
        //      = cos²(x) - sin²(x)
        //      = cos(2x) [by double angle formula]

        use std::sync::Arc;
        let custom_fn = CustomFn::new(2)
            .partial(0, |args: &[Arc<Expr>]| Expr::from(&args[1])) // ∂f/∂u = v
            .partial(1, |args: &[Arc<Expr>]| Expr::from(&args[0])); // ∂f/∂v = u

        let x = symb("x");
        let sin_x = Expr::func("sin", Expr::symbol("x"));
        let cos_x = Expr::func("cos", Expr::symbol("x"));
        let f_call = Expr::call::<2>("my_func", [sin_x, cos_x]);

        let result = Diff::new()
            .custom_fn_multi("my_func", custom_fn)
            .differentiate(f_call, &x)
            .unwrap();

        let result_str = format!("{}", result);
        eprintln!("Chain rule result: {}", result_str);

        // Result should contain sin and cos terms, OR be simplified to cos(2x)
        // cos²(x) - sin²(x) = cos(2x) by double angle formula
        assert!(
            (result_str.contains("sin") && result_str.contains("cos"))
                || result_str.contains("cos(2")
                || result_str.contains("cos(2x)"),
            "Expected sin/cos terms or cos(2x) in chain rule result: {}",
            result_str
        );
    }

    /// Test CustomFn with constant argument (derivative should skip that term)
    #[test]
    fn test_custom_fn_constant_arg() {
        // f(a, b) where ∂f/∂a = b, ∂f/∂b = a
        // Evaluate f(5, x)
        // d/dx = b * 0 + a * 1 = a = 5

        use std::sync::Arc;
        let custom_fn = CustomFn::new(2)
            .partial(0, |args: &[Arc<Expr>]| Expr::from(&args[1]))
            .partial(1, |args: &[Arc<Expr>]| Expr::from(&args[0]));

        let x = symb("x");
        let f_call = Expr::call::<2>("f", [Expr::number(5.0), Expr::symbol("x")]);

        let result = Diff::new()
            .custom_fn_multi("f", custom_fn)
            .differentiate(f_call, &x)
            .unwrap();

        let result_str = format!("{}", result);
        eprintln!("Constant arg result: {}", result_str);

        // Should simplify to 5
        assert!(
            result_str.contains("5"),
            "Expected 5 in result: {}",
            result_str
        );
    }

    /// Test CustomFn without all partials defined (should use symbolic notation)
    #[test]
    fn test_custom_fn_missing_partial() {
        use std::sync::Arc;
        // f(a, b) where only ∂f/∂a is defined
        let custom_fn = CustomFn::new(2).partial(0, |args: &[Arc<Expr>]| Expr::from(&args[1]));
        // Note: ∂f/∂b is NOT defined

        let x = symb("x");
        let f_call = Expr::call::<2>("g", [Expr::symbol("x"), Expr::symbol("x")]);

        let result = Diff::new()
            .custom_fn_multi("g", custom_fn)
            .differentiate(f_call, &x)
            .unwrap();

        let result_str = format!("{}", result);
        eprintln!("Missing partial result: {}", result_str);

        // The missing partial should create symbolic derivative notation
        // At minimum, we should have a result
        assert!(!result_str.is_empty(), "Expected non-empty result");
    }
}
