// Test to verify that hyperbolic conversion patterns handle different term orderings
use crate::Expr;
use crate::simplification::simplify;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosh_reversed_order() {
        // (exp(-x) + exp(x)) / 2 -> cosh(x)
        // Testing REVERSED order: e^(-x) first, e^x second
        let expr = Expr::Div(
            Box::new(Expr::Add(
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::Symbol("x".to_string())),
                    )],
                }),
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Symbol("x".to_string())],
                }),
            )),
            Box::new(Expr::Number(2.0)),
        );

        let simplified = simplify(expr);
        if let Expr::FunctionCall { name, args } = simplified {
            assert_eq!(name, "cosh");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected cosh(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_cosh_normal_order() {
        // (exp(x) + exp(-x)) / 2 -> cosh(x)
        // Testing NORMAL order: e^x first, e^(-x) second
        let expr = Expr::Div(
            Box::new(Expr::Add(
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Symbol("x".to_string())],
                }),
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::Symbol("x".to_string())),
                    )],
                }),
            )),
            Box::new(Expr::Number(2.0)),
        );

        let simplified = simplify(expr);
        if let Expr::FunctionCall { name, args } = simplified {
            assert_eq!(name, "cosh");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected cosh(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_coth_reversed_numerator() {
        // (exp(-x) + exp(x)) / (exp(x) - exp(-x)) -> coth(x)
        // Testing REVERSED order in numerator
        let expr = Expr::Div(
            Box::new(Expr::Add(
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::Symbol("x".to_string())),
                    )],
                }),
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Symbol("x".to_string())],
                }),
            )),
            Box::new(Expr::Sub(
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Symbol("x".to_string())],
                }),
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::Symbol("x".to_string())),
                    )],
                }),
            )),
        );

        let simplified = simplify(expr);
        if let Expr::FunctionCall { name, args } = simplified {
            assert_eq!(name, "coth");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected coth(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_tanh_reversed_denominator() {
        // (exp(x) - exp(-x)) / (exp(-x) + exp(x)) -> tanh(x)
        // Testing REVERSED order in denominator
        let expr = Expr::Div(
            Box::new(Expr::Sub(
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Symbol("x".to_string())],
                }),
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::Symbol("x".to_string())),
                    )],
                }),
            )),
            Box::new(Expr::Add(
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::Symbol("x".to_string())),
                    )],
                }),
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Symbol("x".to_string())],
                }),
            )),
        );

        let simplified = simplify(expr);
        if let Expr::FunctionCall { name, args } = simplified {
            assert_eq!(name, "tanh");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected tanh(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_sech_reversed_denominator() {
        // 2 / (exp(-x) + exp(x)) -> sech(x)
        // Testing REVERSED order in denominator
        let expr = Expr::Div(
            Box::new(Expr::Number(2.0)),
            Box::new(Expr::Add(
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::Symbol("x".to_string())),
                    )],
                }),
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Symbol("x".to_string())],
                }),
            )),
        );

        let simplified = simplify(expr);
        if let Expr::FunctionCall { name, args } = simplified {
            assert_eq!(name, "sech");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected sech(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_sinh_reversed_numerator() {
        // (exp(-x) - exp(x)) / 2 should give us -(exp(x) - exp(-x))/2 = -sinh(x)
        // But let's test the canonical form: (exp(x) - exp(-x)) / 2 reversed in subtraction
        // Actually for sinh we need to test: (exp(x) - exp(-x))/2 in different forms

        // The pattern matcher in SinhFromExpRule handles both Add and Sub
        // Let's verify it handles reversed subtraction: since Sub is not commutative,
        // we test that the match_sinh_pattern handles both u and v positions

        // This is already tested in the original tests, but let's add a variant
        // where the terms appear in a different order due to Add with negation
        let expr = Expr::Div(
            Box::new(Expr::Add(
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Symbol("x".to_string())],
                }),
                Box::new(Expr::Mul(
                    Box::new(Expr::Number(-1.0)),
                    Box::new(Expr::FunctionCall {
                        name: "exp".to_string(),
                        args: vec![Expr::Mul(
                            Box::new(Expr::Number(-1.0)),
                            Box::new(Expr::Symbol("x".to_string())),
                        )],
                    }),
                )),
            )),
            Box::new(Expr::Number(2.0)),
        );

        let simplified = simplify(expr);
        if let Expr::FunctionCall { name, args } = simplified {
            assert_eq!(name, "sinh");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected sinh(x), got {:?}", simplified);
        }
    }

    #[test]
    fn test_sinh_reversed_add_pattern() {
        // (-1*exp(-x)) + exp(x) -> sinh(x)
        // Testing REVERSED order in Add pattern with negation first
        let expr = Expr::Div(
            Box::new(Expr::Add(
                Box::new(Expr::Mul(
                    Box::new(Expr::Number(-1.0)),
                    Box::new(Expr::FunctionCall {
                        name: "exp".to_string(),
                        args: vec![Expr::Mul(
                            Box::new(Expr::Number(-1.0)),
                            Box::new(Expr::Symbol("x".to_string())),
                        )],
                    }),
                )),
                Box::new(Expr::FunctionCall {
                    name: "exp".to_string(),
                    args: vec![Expr::Symbol("x".to_string())],
                }),
            )),
            Box::new(Expr::Number(2.0)),
        );

        let simplified = simplify(expr);
        if let Expr::FunctionCall { name, args } = simplified {
            assert_eq!(name, "sinh");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected sinh(x), got {:?}", simplified);
        }
    }
}
