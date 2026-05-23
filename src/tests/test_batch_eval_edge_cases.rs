use crate::evaluator::VmEvaluator;
use crate::parser::parse;
use std::collections::HashSet;

#[test]
fn test_batch_eval_empty_column() {
    let expr = parse("x + y", &HashSet::new(), &HashSet::new(), None).unwrap();
    let compiled = VmEvaluator::compile(&expr, &["x", "y"], None).unwrap();

    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y: Vec<f64> = vec![]; // Empty column

    let mut output = vec![0.0; 5];
    compiled.eval_batch(&[&x, &y], &mut output, None).unwrap();

    // Expected: x + 0.0 = x
    for (out_val, x_val) in output.iter().zip(&x) {
        assert_eq!(*out_val, *x_val);
    }
}

#[test]
fn test_batch_eval_all_empty_columns() {
    let expr = parse("x + y", &HashSet::new(), &HashSet::new(), None).unwrap();
    let compiled = VmEvaluator::compile(&expr, &["x", "y"], None).unwrap();

    let x: Vec<f64> = vec![];
    let y: Vec<f64> = vec![];

    let mut output = vec![0.0; 5];
    compiled.eval_batch(&[&x, &y], &mut output, None).unwrap();

    // Expected: 0.0 + 0.0 = 0.0
    for val in &output {
        assert_eq!(*val, 0.0);
    }
}

#[test]
fn test_batch_eval_inconsistent_lengths() {
    let expr = parse("x + y", &HashSet::new(), &HashSet::new(), None).unwrap();
    let compiled = VmEvaluator::compile(&expr, &["x", "y"], None).unwrap();

    let x = vec![1.0, 2.0];
    let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];

    let mut output = vec![0.0; 5];
    compiled.eval_batch(&[&x, &y], &mut output, None).unwrap();

    // Expected:
    // i=0: 1.0 + 10.0 = 11.0
    // i=1: 2.0 + 20.0 = 22.0
    // i=2: 2.0 (last x) + 30.0 = 32.0
    // i=3: 2.0 (last x) + 40.0 = 42.0
    // i=4: 2.0 (last x) + 50.0 = 52.0
    assert_eq!(output[0], 11.0);
    assert_eq!(output[1], 22.0);
    assert_eq!(output[2], 32.0);
    assert_eq!(output[3], 42.0);
    assert_eq!(output[4], 52.0);
}

#[test]
fn test_batch_eval_tail_cases() {
    let expr = parse("x", &HashSet::new(), &HashSet::new(), None).unwrap();
    let compiled = VmEvaluator::compile(&expr, &["x"], None).unwrap();

    // Test various tail lengths (1, 2, 3, 5)
    for n in [1, 2, 3, 5, 10] {
        let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let mut output = vec![0.0; n];
        compiled.eval_batch(&[&x], &mut output, None).unwrap();
        for (out_val, x_val) in output.iter().zip(&x) {
            assert_eq!(*out_val, *x_val);
        }
    }
}
