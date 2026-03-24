use crate::convenience::{evaluate_str, gradient_str, hessian_str, jacobian_str};

#[allow(clippy::unwrap_used, reason = "Standard test relaxations")]
#[test]
fn test_gradient() {
    let grad = gradient_str("x^2 + y^2", &["x", "y"]).unwrap();
    assert_eq!(grad.len(), 2);
    assert_eq!(grad[0], "2*x");
    assert_eq!(grad[1], "2*y");
}

#[allow(clippy::unwrap_used, reason = "Standard test relaxations")]
#[test]
fn test_hessian() {
    let hess = hessian_str("x^2 + y^2", &["x", "y"]).unwrap();
    assert_eq!(hess.len(), 2);
    assert_eq!(hess[0].len(), 2);
    assert_eq!(hess[0][0], "2");
    assert_eq!(hess[1][1], "2");
}

#[allow(clippy::unwrap_used, reason = "Standard test relaxations")]
#[test]
fn test_jacobian() {
    let jac = jacobian_str(&["x^2", "x * y"], &["x", "y"]).unwrap();
    assert_eq!(jac.len(), 2);
    assert_eq!(jac[0][0], "2*x");
    assert_eq!(jac[1][0], "y");
}

#[allow(clippy::unwrap_used, reason = "Standard test relaxations")]
#[test]
fn test_evaluate_str_partial() {
    let result = evaluate_str("x * y", &[("x", 3.0)]).unwrap();
    assert!(result.contains('3') && result.contains('y'));
}

#[allow(clippy::unwrap_used, reason = "Standard test relaxations")]
#[test]
fn test_evaluate_str_full() {
    let result = evaluate_str("x * y", &[("x", 3.0), ("y", 2.0)]).unwrap();
    assert_eq!(result, "6");
}
