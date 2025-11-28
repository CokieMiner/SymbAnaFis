// Display formatting for AST
use crate::Expr;
use std::fmt;

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Number(n) => {
                if n.is_nan() {
                    write!(f, "NaN")
                } else if n.is_infinite() {
                    if *n > 0.0 {
                        write!(f, "Infinity")
                    } else {
                        write!(f, "-Infinity")
                    }
                } else if n.fract() == 0.0 && n.abs() < 1e10 {
                    // Display as integer if no fractional part
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }

            Expr::Symbol(s) => write!(f, "{}", s),

            Expr::FunctionCall { name, args } => {
                if args.is_empty() {
                    write!(f, "{}()", name)
                } else {
                    let args_str: Vec<String> = args.iter().map(|arg| format!("{}", arg)).collect();
                    write!(f, "{}({})", name, args_str.join(", "))
                }
            }

            Expr::Add(u, v) => {
                write!(f, "({} + {})", u, v)
            }

            Expr::Sub(u, v) => {
                write!(f, "({} - {})", u, v)
            }

            Expr::Mul(u, v) => {
                if let Expr::Number(n) = **u {
                    if n == -1.0 {
                        write!(f, "-{}", format_mul_operand(v))
                    } else {
                        write!(f, "{} * {}", format_mul_operand(u), format_mul_operand(v))
                    }
                } else {
                    write!(f, "{} * {}", format_mul_operand(u), format_mul_operand(v))
                }
            }

            Expr::Div(u, v) => {
                write!(f, "{} / {}", format_mul_operand(u), format_mul_operand(v))
            }

            Expr::Pow(u, v) => {
                write!(f, "({})^{}", u, v)
            }
        }
    }
}

/// Format operand for multiplication to minimize parentheses
fn format_mul_operand(expr: &Expr) -> String {
    match expr {
        Expr::Add(_, _) => format!("({})", expr),
        _ => format!("{}", expr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_number() {
        let expr = Expr::Number(3.0);
        assert_eq!(format!("{}", expr), "3");

        let expr = Expr::Number(3.14);
        assert_eq!(format!("{}", expr), "3.14");
    }

    #[test]
    fn test_display_symbol() {
        let expr = Expr::Symbol("x".to_string());
        assert_eq!(format!("{}", expr), "x");
    }

    #[test]
    fn test_display_addition() {
        let expr = Expr::Add(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(1.0)),
        );
        assert_eq!(format!("{}", expr), "(x + 1)");
    }

    #[test]
    fn test_display_multiplication() {
        let expr = Expr::Mul(
            Box::new(Expr::Symbol("x".to_string())),
            Box::new(Expr::Number(2.0)),
        );
        assert_eq!(format!("{}", expr), "x * 2");
    }

    #[test]
    fn test_display_function() {
        let expr = Expr::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::Symbol("x".to_string())],
        };
        assert_eq!(format!("{}", expr), "sin(x)");
    }

    #[test]
    fn test_display_negative_term() {
        let expr = Expr::Mul(
            Box::new(Expr::Number(-1.0)),
            Box::new(Expr::Symbol("x".to_string())),
        );
        assert_eq!(format!("{}", expr), "-x");

        let expr2 = Expr::Mul(
            Box::new(Expr::Number(-1.0)),
            Box::new(Expr::FunctionCall {
                name: "sin".to_string(),
                args: vec![Expr::Symbol("x".to_string())],
            }),
        );
        assert_eq!(format!("{}", expr2), "-sin(x)");
    }
}
