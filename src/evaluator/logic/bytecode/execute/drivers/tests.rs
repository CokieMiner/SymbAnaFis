use crate::{Expr, core::ExprKind, eval_parallel, symb};

const fn get_num(expr: &Expr) -> f64 {
    match &expr.kind {
        ExprKind::Number(n) => *n,
        _ => f64::NAN,
    }
}

#[test]
fn test_string_expr_single_var() {
    let results = eval_parallel!(
        exprs: ["x^2"],
        vars: [["x"]],
        values: [[[1.0, 2.0, 3.0]]]
    )
    .expect("Should pass");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].len(), 3);
    assert!(results[0][0].is_string());
    assert_eq!(results[0][0].to_string(), "1");
    assert_eq!(results[0][1].to_string(), "4");
    assert_eq!(results[0][2].to_string(), "9");
}

#[test]
fn test_expr_input_single_var() {
    let x = symb("x");
    let expr = x.pow(2.0);

    let results = eval_parallel!(
        exprs: [expr],
        vars: [["x"]],
        values: [[[1.0, 2.0, 3.0]]]
    )
    .expect("Should pass");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].len(), 3);
    assert!(results[0][0].is_expr());
    assert!((get_num(&results[0][0].clone().unwrap_expr()) - 1.0).abs() < 1e-10);
    assert!((get_num(&results[0][1].clone().unwrap_expr()) - 4.0).abs() < 1e-10);
    assert!((get_num(&results[0][2].clone().unwrap_expr()) - 9.0).abs() < 1e-10);
}

#[test]
fn test_mixed_exprs() {
    let t = symb("t");
    let expr = &t + 1.0;

    let results = eval_parallel!(
        exprs: ["x^2", expr],
        vars: [["x"], ["t"]],
        values: [
            [[2.0, 3.0]],
            [[10.0, 20.0]]
        ]
    )
    .expect("Should pass");

    assert_eq!(results.len(), 2);

    // First was string
    assert!(results[0][0].is_string());
    assert_eq!(results[0][0].to_string(), "4");
    assert_eq!(results[0][1].to_string(), "9");

    // Second was Expr
    assert!(results[1][0].is_expr());
    assert!((get_num(&results[1][0].clone().unwrap_expr()) - 11.0).abs() < 1e-10);
    assert!((get_num(&results[1][1].clone().unwrap_expr()) - 21.0).abs() < 1e-10);
}

#[test]
fn test_two_vars() {
    let results = eval_parallel!(
        exprs: ["x + y"],
        vars: [["x", "y"]],
        values: [[[1.0, 2.0], [10.0, 20.0]]]
    )
    .expect("Should pass");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].len(), 2);
    assert_eq!(results[0][0].to_string(), "11");
    assert_eq!(results[0][1].to_string(), "22");
}

#[test]
fn test_skip_value() {
    use super::parallel::SKIP;
    let eval_results = eval_parallel!(
        exprs: ["x * y"],
        vars: [["x", "y"]],
        values: [[[2.0, SKIP], [3.0, 5.0]]]
    )
    .expect("Should pass");

    assert_eq!(eval_results.len(), 1);
    assert_eq!(eval_results[0].len(), 2);

    // Point 0: x=2, y=3 → 6
    assert_eq!(eval_results[0][0].to_string(), "6");

    // Point 1: x=skip, y=5 → symbolic
    let result_str = eval_results[0][1].to_string();
    assert!(result_str.contains('x'));
    assert!(result_str.contains('5'));
}
