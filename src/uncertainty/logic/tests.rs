use super::super::api::{CovEntry, CovarianceMatrix, uncertainty_propagation};
use crate::symb;

#[test]
fn test_simple_sum_uncorrelated() {
    let x = symb("x");
    let y = symb("y");
    let expr = x + y;

    let result =
        uncertainty_propagation(&expr, &["x", "y"], None::<&CovarianceMatrix>).expect("failed sum");
    let latex = result.to_latex();

    #[allow(
        clippy::literal_string_with_formatting_args,
        reason = "LaTeX braces are not formatting args"
    )]
    let contains_sigma = latex.contains(r"\sigma_{x}") && latex.contains(r"\sigma_{y}");
    assert!(contains_sigma);
}

#[test]
fn test_simple_product_uncorrelated() {
    let x = symb("x");
    let y = symb("y");
    let expr = x * y;

    let result = uncertainty_propagation(&expr, &["x", "y"], None::<&CovarianceMatrix>)
        .expect("failed product");
    assert!(!result.is_zero_num());
}

#[test]
fn test_numeric_covariance() {
    let x = symb("x");
    let y = symb("y");
    let expr = x + y;

    let cov = CovarianceMatrix::diagonal(vec![CovEntry::Num(1.0), CovEntry::Num(4.0)]);

    let result = uncertainty_propagation(&expr, &["x", "y"], Some(&cov))
        .expect("Uncertainty propagation failed");

    if let Some(n) = result.as_number() {
        assert!(
            (n - 5.0_f64.sqrt()).abs() < 1e-10,
            "Expected {}, got {}",
            5.0_f64.sqrt(),
            n
        );
    }
}

#[test]
fn test_single_variable() {
    let x = symb("test_unc_x");
    let expr = x.pow(2.0);

    let result = uncertainty_propagation(&expr, &["test_unc_x"], None::<&CovarianceMatrix>)
        .expect("Uncertainty propagation failed");

    let display = format!("{result}");
    assert!(!display.is_empty());
}
