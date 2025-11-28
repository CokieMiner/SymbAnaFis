use crate::Expr;

pub fn apply_numeric_rules(expr: Expr) -> Expr {
    match &expr {
        // Addition rules
        Expr::Add(u, v) => {
            // 0 + x = x
            if matches!(**u, Expr::Number(n) if n == 0.0) {
                return (**v).clone();
            }
            // x + 0 = x
            if matches!(**v, Expr::Number(n) if n == 0.0) {
                return (**u).clone();
            }
            // Constant folding: a + b = c
            if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v) {
                let result = a + b;
                if !result.is_nan() && !result.is_infinite() {
                    return Expr::Number(result);
                }
            }
            expr
        }

        // Subtraction rules
        Expr::Sub(u, v) => {
            // x - 0 = x
            if matches!(**v, Expr::Number(n) if n == 0.0) {
                return (**u).clone();
            }
            // Constant folding: a - b = c
            if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v) {
                let result = a - b;
                if !result.is_nan() && !result.is_infinite() {
                    return Expr::Number(result);
                }
            }
            expr
        }

        // Multiplication rules
        Expr::Mul(u, v) => {
            // 0 * x = 0
            if matches!(**u, Expr::Number(n) if n == 0.0) {
                return Expr::Number(0.0);
            }
            // x * 0 = 0
            if matches!(**v, Expr::Number(n) if n == 0.0) {
                return Expr::Number(0.0);
            }
            // 1 * x = x
            if matches!(**u, Expr::Number(n) if n == 1.0) {
                return (**v).clone();
            }
            // x * 1 = x
            if matches!(**v, Expr::Number(n) if n == 1.0) {
                return (**u).clone();
            }
            // Constant folding: a * b = c
            if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v) {
                let result = a * b;
                if !result.is_nan() && !result.is_infinite() {
                    return Expr::Number(result);
                }
            }
            expr
        }

        // Division rules
        Expr::Div(u, v) => {
            // x / 1 = x
            if matches!(**v, Expr::Number(n) if n == 1.0) {
                return (**u).clone();
            }
            // 0 / x = 0 (if x != 0)
            if matches!(**u, Expr::Number(n) if n == 0.0) {
                return Expr::Number(0.0);
            }
            // Constant folding: a / b = c
            if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v)
                && *b != 0.0 {
                // Check if we should preserve as fraction
                // Only preserve if both are effectively integers and result is not an integer
                let is_int_a = a.fract() == 0.0;
                let is_int_b = b.fract() == 0.0;

                if is_int_a && is_int_b {
                    if a % b == 0.0 {
                        // Exact integer division
                        return Expr::Number(a / b);
                    } else {
                        // Non-integer result, keep as fraction
                        // We can try to simplify the fraction (e.g. 2/4 -> 1/2) here if we want,
                        // but for now just keeping it is enough to satisfy the requirement.
                        // To be safe against infinite recursion if we return expr, we should return a new Div
                        // But since we are in a match on &expr, returning expr.clone() is fine.
                        // Actually, returning expr.clone() might cause infinite loop in simplify() if it expects change?
                        // No, simplify() loop breaks if current == original.
                        // If we return expr.clone(), it means no change, so it stabilizes.
                        return expr.clone();
                    }
                }

                let result = a / b;
                if !result.is_nan() && !result.is_infinite() {
                    return Expr::Number(result);
                }
            }
            expr
        }

        // Power rules
        Expr::Pow(u, v) => {
            // x^0 = 1
            if matches!(**v, Expr::Number(n) if n == 0.0) {
                return Expr::Number(1.0);
            }
            // x^1 = x
            if matches!(**v, Expr::Number(n) if n == 1.0) {
                return (**u).clone();
            }
            // 0^x = 0 (for x > 0)
            if matches!(**u, Expr::Number(n) if n == 0.0) {
                return Expr::Number(0.0);
            }
            // 1^x = 1
            if matches!(**u, Expr::Number(n) if n == 1.0) {
                return Expr::Number(1.0);
            }
            // Constant folding: a^b = c
            if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v) {
                let result = a.powf(*b);
                if !result.is_nan() && !result.is_infinite() {
                    return Expr::Number(result);
                }
            }
            expr
        }

        _ => expr,
    }
}
