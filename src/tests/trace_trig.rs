#[cfg(test)]
mod tests {
    use crate::{Expr, simplification::simplify};
    use std::f64::consts::PI;

    #[test]
    fn test_trig_reflection_shifts() {
        // sin(pi - x) = sin(x)
        let expr = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Sub(
                Box::new(Expr::Number(PI)),
                Box::new(Expr::Symbol("x".to_string())),
            )],
        };
        let simplified = simplify(expr);
        println!("sin(pi - x) -> {}", simplified);
        if let Expr::FunctionCall { name, args } = &simplified {
            assert_eq!(name, "sin");
            assert_eq!(args[0], Expr::Symbol("x".to_string()));
        } else {
            panic!("Expected sin(x), got {:?}", simplified);
        }

        // cos(pi + x) = -cos(x)
        let expr = Expr::FunctionCall {
            name: "cos".to_string(),
            args: vec![Expr::Add(
                Box::new(Expr::Number(PI)),
                Box::new(Expr::Symbol("x".to_string())),
            )],
        };
        let simplified = simplify(expr);
        println!("cos(pi + x) -> {}", simplified);
        if let Expr::Mul(a, b) = &simplified {
            assert_eq!(**a, Expr::Number(-1.0));
            if let Expr::FunctionCall { name, args } = &**b {
                assert_eq!(name, "cos");
                assert_eq!(args[0], Expr::Symbol("x".to_string()));
            } else {
                panic!("Expected cos(x), got {:?}", simplified);
            }
        } else {
            panic!("Expected -cos(x), got {:?}", simplified);
        }

        // sin(3pi/2 - x) = -cos(x)
        let expr = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Sub(
                Box::new(Expr::Number(3.0 * PI / 2.0)),
                Box::new(Expr::Symbol("x".to_string())),
            )],
        };
        let simplified = simplify(expr);
        println!("sin(3pi/2 - x) -> {}", simplified);
        if let Expr::Mul(a, b) = &simplified {
            assert_eq!(**a, Expr::Number(-1.0));
            if let Expr::FunctionCall { name, args } = &**b {
                assert_eq!(name, "cos");
                assert_eq!(args[0], Expr::Symbol("x".to_string()));
            } else {
                panic!("Expected cos(x), got {:?}", simplified);
            }
        } else {
            panic!("Expected -cos(x), got {:?}", simplified);
        }
    }
}
