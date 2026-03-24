// Allow some relaxations for tests
#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    reason = "Standard test relaxations"
)]

#[cfg(feature = "parallel")]
use crate::evaluator::eval_f64;

#[cfg(feature = "parallel")]
use crate::Expr;
#[cfg(feature = "parallel")]
use std::collections::HashSet;

#[cfg(feature = "parallel")]
fn parse_expr(s: &str) -> Expr {
    crate::parser::parse(s, &HashSet::new(), &HashSet::new(), None).unwrap()
}

#[cfg(feature = "parallel")]
#[test]
fn test_eval_f64_simple() {
    let expr = parse_expr("x^2 + y");
    let x_data = [1.0, 2.0, 3.0];
    let y_data = [0.5, 0.5, 0.5];
    let result = eval_f64(&[&expr], &[&["x", "y"]], &[&[&x_data[..], &y_data[..]]]).unwrap();
    assert_eq!(result.len(), 1);
    assert!((result[0][0] - 1.5).abs() < 1e-10);
    assert!((result[0][1] - 4.5).abs() < 1e-10);
    assert!((result[0][2] - 9.5).abs() < 1e-10);
}

#[cfg(feature = "parallel")]
#[test]
fn test_eval_f64_trig() {
    let expr = parse_expr("sin(x) * cos(x)");
    let x_data: Vec<f64> = (0..100).map(|i| f64::from(i) * 0.01).collect();
    let result = eval_f64(&[&expr], &[&["x"]], &[&[&x_data[..]]]).unwrap();
    for (i, &res) in result[0].iter().enumerate() {
        let x = i as f64 * 0.01;
        assert!(x.sin().mul_add(-x.cos(), res).abs() < 1e-10);
    }
}

#[cfg(feature = "parallel")]
#[test]
fn test_eval_f64_multiple_exprs() {
    let expr1 = parse_expr("x + 1");
    let expr2 = parse_expr("y * 2");
    let x_data = [1.0, 2.0, 3.0];
    let y_data = [10.0, 20.0, 30.0];
    let result = eval_f64(
        &[&expr1, &expr2],
        &[&["x"], &["y"]],
        &[&[&x_data[..]], &[&y_data[..]]],
    )
    .unwrap();
    assert_eq!(result[0], vec![2.0, 3.0, 4.0]);
    assert_eq!(result[1], vec![20.0, 40.0, 60.0]);
}

#[cfg(feature = "parallel")]
#[test]
fn test_eval_f64_large() {
    let expr = parse_expr("sin(x) + cos(x)");
    let x_data: Vec<f64> = (0..100_000).map(|i| f64::from(i) * 0.0001).collect();
    let result = eval_f64(&[&expr], &[&["x"]], &[&[&x_data[..]]]).unwrap();
    assert_eq!(result[0].len(), 100_000);
    // Spot check
    for i in [0, 1000, 50000, 99999] {
        let x = i as f64 * 0.0001;
        assert!((result[0][i] - (x.sin() + x.cos())).abs() < 1e-10);
    }
}
