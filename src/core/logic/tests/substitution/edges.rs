//! Deep-DAG substitution on a bounded call stack.

use std::thread;

use alloc::vec;

use crate::core::{Context, Expr, Substitution, UserFunction};

#[test]
fn deep_shared_polynomial_substitution_uses_an_iterative_walk() {
    thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let context = Context::new();
            let x = context.symb("x").expect("symbol allocation").to_expr();
            let y = context.symb("y").expect("symbol allocation").to_expr();
            let mut original = x.clone();
            let mut expected = y.clone();
            let function = context
                .register_function("deep_substitution", UserFunction::new(1..=1))
                .expect("the custom name is unbound");
            let pair = context
                .register_function("pair", UserFunction::new(2..=2))
                .expect("the custom name is unbound");
            for _ in 0..10_000 {
                original = function
                    .call(vec![original])
                    .expect("the function is unary");
                expected = function
                    .call(vec![expected])
                    .expect("the function is unary");
            }
            let polynomial = Expr::sum(vec![original.pow(2), original.pow(3)]);
            let expected_polynomial = Expr::sum(vec![expected.pow(2), expected.pow(3)]);
            let input = pair
                .call(vec![polynomial.clone(), polynomial])
                .expect("pair accepts two arguments");
            let result = Substitution::new(x, y).apply_all(&input);
            assert_eq!(
                result,
                pair.call(vec![expected_polynomial.clone(), expected_polynomial])
                    .expect("pair accepts two arguments"),
                "all shared polynomial bases receive the replacement on a small stack"
            );
        })
        .expect("the small-stack substitution worker starts")
        .join()
        .expect("substitution and destruction use bounded call stacks");
}
