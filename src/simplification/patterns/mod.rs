use crate::Expr;

/// Common pattern matching utilities for simplification rules
pub mod common {
    use super::*;

    /// Extract coefficient and base from a multiplication term
    /// Returns (coefficient, base) where base is normalized
    pub fn extract_coefficient(expr: &Expr) -> (f64, Expr) {
        match expr {
            Expr::Number(n) => (*n, Expr::Number(1.0)),
            Expr::Mul(coeff, base) => {
                if let Expr::Number(n) = **coeff {
                    (n, *base.clone())
                } else {
                    (1.0, expr.clone())
                }
            }
            _ => (1.0, expr.clone()),
        }
    }
}

/// Trigonometric pattern matching utilities
pub mod trigonometric {
    use super::*;

    /// Extract function name and argument if expression is a trig function
    pub fn get_trig_function(expr: &Expr) -> Option<(&str, Expr)> {
        if let Expr::FunctionCall { name, args } = expr {
            if args.len() == 1 {
                match name.as_str() {
                    "sin" | "cos" | "tan" | "cot" | "sec" | "csc" => {
                        Some((name.as_str(), args[0].clone()))
                    }
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}