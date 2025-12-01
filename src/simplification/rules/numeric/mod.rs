use crate::Expr;
use crate::simplification::rules::{Rule, RuleCategory, RuleContext};
use std::rc::Rc;

/// Numeric simplification rules
pub mod rules {
    use super::*;

    /// Rule for adding zero: x + 0 = x, 0 + x = x
    pub struct AddZeroRule;

    impl Rule for AddZeroRule {
        fn name(&self) -> &'static str {
            "add_zero"
        }

        fn priority(&self) -> i32 {
            100
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Add(u, v) = expr {
                if matches!(**u, Expr::Number(n) if n == 0.0) {
                    return Some((**v).clone());
                }
                if matches!(**v, Expr::Number(n) if n == 0.0) {
                    return Some((**u).clone());
                }
            }
            None
        }
    }

    /// Rule for subtracting zero: x - 0 = x
    pub struct SubZeroRule;

    impl Rule for SubZeroRule {
        fn name(&self) -> &'static str {
            "sub_zero"
        }

        fn priority(&self) -> i32 {
            100
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Sub(u, v) = expr
                && matches!(**v, Expr::Number(n) if n == 0.0)
            {
                return Some((**u).clone());
            }
            None
        }
    }

    /// Rule for multiplying by zero: 0 * x = 0, x * 0 = 0
    pub struct MulZeroRule;

    impl Rule for MulZeroRule {
        fn name(&self) -> &'static str {
            "mul_zero"
        }

        fn priority(&self) -> i32 {
            100
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Mul(u, v) = expr {
                if matches!(**u, Expr::Number(n) if n == 0.0) {
                    return Some(Expr::Number(0.0));
                }
                if matches!(**v, Expr::Number(n) if n == 0.0) {
                    return Some(Expr::Number(0.0));
                }
            }
            None
        }
    }

    /// Rule for multiplying by one: 1 * x = x, x * 1 = x
    pub struct MulOneRule;

    impl Rule for MulOneRule {
        fn name(&self) -> &'static str {
            "mul_one"
        }

        fn priority(&self) -> i32 {
            100
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Mul(u, v) = expr {
                if matches!(**u, Expr::Number(n) if n == 1.0) {
                    return Some((**v).clone());
                }
                if matches!(**v, Expr::Number(n) if n == 1.0) {
                    return Some((**u).clone());
                }
            }
            None
        }
    }

    /// Rule for dividing by one: x / 1 = x
    pub struct DivOneRule;

    impl Rule for DivOneRule {
        fn name(&self) -> &'static str {
            "div_one"
        }

        fn priority(&self) -> i32 {
            100
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Div(u, v) = expr
                && matches!(**v, Expr::Number(n) if n == 1.0)
            {
                return Some((**u).clone());
            }
            None
        }
    }

    /// Rule for zero divided by something: 0 / x = 0 (when x != 0)
    pub struct ZeroDivRule;

    impl Rule for ZeroDivRule {
        fn name(&self) -> &'static str {
            "zero_div"
        }

        fn priority(&self) -> i32 {
            100
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Div(_u, _v) = expr
                && matches!(**_u, Expr::Number(n) if n == 0.0)
            {
                return Some(Expr::Number(0.0));
            }
            None
        }
    }

    /// Rule for normalizing signs in division: x / -y -> -x / y
    /// Moves negative signs from denominator to numerator
    pub struct NormalizeSignDivRule;

    impl Rule for NormalizeSignDivRule {
        fn name(&self) -> &'static str {
            "normalize_sign_div"
        }

        fn priority(&self) -> i32 {
            95 // High priority to normalize early
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Div(num, den) = expr {
                // Check if denominator is negative number
                if let Expr::Number(d) = **den
                    && d < 0.0
                {
                    // x / -y -> -x / y
                    return Some(Expr::Div(
                        Box::new(Expr::Mul(Box::new(Expr::Number(-1.0)), num.clone())),
                        Box::new(Expr::Number(-d)),
                    ));
                }

                // Check if denominator is (-1 * something)
                if let Expr::Mul(c, rest) = &**den
                    && matches!(**c, Expr::Number(n) if n == -1.0)
                {
                    // x / (-1 * y) -> -x / y
                    return Some(Expr::Div(
                        Box::new(Expr::Mul(Box::new(Expr::Number(-1.0)), num.clone())),
                        rest.clone(),
                    ));
                }

                // Check if denominator is (something * -1)
                if let Expr::Mul(rest, c) = &**den
                    && matches!(**c, Expr::Number(n) if n == -1.0)
                {
                    // x / (y * -1) -> -x / y
                    return Some(Expr::Div(
                        Box::new(Expr::Mul(Box::new(Expr::Number(-1.0)), num.clone())),
                        rest.clone(),
                    ));
                }
            }
            None
        }
    }

    /// Rule for power of zero: x^0 = 1 (when x != 0)
    pub struct PowZeroRule;

    impl Rule for PowZeroRule {
        fn name(&self) -> &'static str {
            "pow_zero"
        }

        fn priority(&self) -> i32 {
            100
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(_u, v) = expr
                && matches!(**v, Expr::Number(n) if n == 0.0)
            {
                return Some(Expr::Number(1.0));
            }
            None
        }
    }

    /// Rule for power of one: x^1 = x
    pub struct PowOneRule;

    impl Rule for PowOneRule {
        fn name(&self) -> &'static str {
            "pow_one"
        }

        fn priority(&self) -> i32 {
            100
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(u, v) = expr
                && matches!(**v, Expr::Number(n) if n == 1.0)
            {
                return Some((**u).clone());
            }
            None
        }
    }

    /// Rule for zero to a power: 0^x = 0 (for x > 0)
    pub struct ZeroPowRule;

    impl Rule for ZeroPowRule {
        fn name(&self) -> &'static str {
            "zero_pow"
        }

        fn priority(&self) -> i32 {
            100
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(_u, _v) = expr
                && matches!(**_u, Expr::Number(n) if n == 0.0)
            {
                return Some(Expr::Number(0.0));
            }
            None
        }
    }

    /// Rule for one to a power: 1^x = 1
    pub struct OnePowRule;

    impl Rule for OnePowRule {
        fn name(&self) -> &'static str {
            "one_pow"
        }

        fn priority(&self) -> i32 {
            100
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(_u, _v) = expr
                && matches!(**_u, Expr::Number(n) if n == 1.0)
            {
                return Some(Expr::Number(1.0));
            }
            None
        }
    }

    /// Rule for constant folding arithmetic operations
    pub struct ConstantFoldRule;

    impl Rule for ConstantFoldRule {
        fn name(&self) -> &'static str {
            "constant_fold"
        }

        fn priority(&self) -> i32 {
            90 // Lower priority than identity rules
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            match expr {
                Expr::Add(u, v) => {
                    if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v) {
                        let result = a + b;
                        if !result.is_nan() && !result.is_infinite() {
                            return Some(Expr::Number(result));
                        }
                    }
                }
                Expr::Sub(u, v) => {
                    if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v) {
                        let result = a - b;
                        if !result.is_nan() && !result.is_infinite() {
                            return Some(Expr::Number(result));
                        }
                    }
                }
                Expr::Mul(u, v) => {
                    if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v) {
                        let result = a * b;
                        if !result.is_nan() && !result.is_infinite() {
                            return Some(Expr::Number(result));
                        }
                    }
                    // Flatten the multiplication and look for multiple numbers to combine
                    // This handles cases like 3 * (2 * x) -> 6 * x, or (x * 3) * 2 -> 6 * x
                    // or even deeply nested: 3 * (y * (2 * z)) -> 6 * y * z
                    let factors = crate::simplification::helpers::flatten_mul(expr);
                    let mut numbers: Vec<f64> = Vec::new();
                    let mut non_numbers: Vec<Expr> = Vec::new();

                    for factor in &factors {
                        if let Expr::Number(n) = factor {
                            numbers.push(*n);
                        } else {
                            non_numbers.push(factor.clone());
                        }
                    }

                    // If we have 2+ numbers, combine them
                    if numbers.len() >= 2 {
                        let combined: f64 = numbers.iter().product();
                        if !combined.is_nan() && !combined.is_infinite() {
                            // Rebuild with the combined number and the non-numbers
                            let mut result_factors = vec![Expr::Number(combined)];
                            result_factors.extend(non_numbers);
                            return Some(crate::simplification::helpers::rebuild_mul(
                                result_factors,
                            ));
                        }
                    }

                    // Transform Mul(Number, Div(Number, Number)) to final result
                    if let (Expr::Number(a), Expr::Div(b, c)) = (&**u, &**v)
                        && let (Expr::Number(b_val), Expr::Number(c_val)) = (&**b, &**c)
                        && *c_val != 0.0
                    {
                        let result = (a * b_val) / c_val;
                        // If result is integer, fold completely
                        if (result - result.round()).abs() < 1e-10 {
                            return Some(Expr::Number(result.round()));
                        } else {
                            // Return as fraction
                            return Some(Expr::Div(
                                Box::new(Expr::Number(a * b_val)),
                                Box::new(Expr::Number(*c_val)),
                            ));
                        }
                    }
                    // Transform Mul(Div(Number, Number), Number) to final result
                    if let (Expr::Div(b, c), Expr::Number(a)) = (&**u, &**v)
                        && let (Expr::Number(b_val), Expr::Number(c_val)) = (&**b, &**c)
                        && *c_val != 0.0
                    {
                        let result = (a * b_val) / c_val;
                        // If result is integer, fold completely
                        if (result - result.round()).abs() < 1e-10 {
                            return Some(Expr::Number(result.round()));
                        } else {
                            // Return as fraction
                            return Some(Expr::Div(
                                Box::new(Expr::Number(a * b_val)),
                                Box::new(Expr::Number(*c_val)),
                            ));
                        }
                    }
                }
                Expr::Div(u, v) => {
                    if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v)
                        && *b != 0.0
                    {
                        let result = a / b;
                        // Conservative: only fold if result is an integer
                        // This preserves symbolic fractions like 7/2
                        if (result - result.round()).abs() < 1e-10 {
                            return Some(Expr::Number(result.round()));
                        }
                    }
                }
                Expr::Pow(u, v) => {
                    if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v) {
                        let result = a.powf(*b);
                        if !result.is_nan() && !result.is_infinite() {
                            return Some(Expr::Number(result));
                        }
                    }
                }
                _ => {}
            }
            None
        }
    }

    /// Rule for simplifying fractions with integer coefficients
    pub struct FractionSimplifyRule;

    impl Rule for FractionSimplifyRule {
        fn name(&self) -> &'static str {
            "fraction_simplify"
        }

        fn priority(&self) -> i32 {
            80
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Numeric
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Div(u, v) = expr
                && let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v)
                && *b != 0.0
            {
                let is_int_a = a.fract() == 0.0;
                let is_int_b = b.fract() == 0.0;

                if is_int_a && is_int_b {
                    if a % b == 0.0 {
                        // Exact integer division
                        return Some(Expr::Number(a / b));
                    } else {
                        // Simplify fraction: 2/4 -> 1/2
                        let a_int = *a as i64;
                        let b_int = *b as i64;
                        let common = gcd(a_int.abs(), b_int.abs());

                        if common > 1 {
                            return Some(Expr::Div(
                                Box::new(Expr::Number((a_int / common) as f64)),
                                Box::new(Expr::Number((b_int / common) as f64)),
                            ));
                        }
                    }
                }
            }
            None
        }
    }
}

/// Helper: Greatest Common Divisor
fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

/// Get all numeric rules in priority order
pub fn get_numeric_rules() -> Vec<Rc<dyn Rule>> {
    vec![
        Rc::new(rules::AddZeroRule),
        Rc::new(rules::SubZeroRule),
        Rc::new(rules::MulZeroRule),
        Rc::new(rules::MulOneRule),
        Rc::new(rules::DivOneRule),
        Rc::new(rules::ZeroDivRule),
        Rc::new(rules::NormalizeSignDivRule),
        Rc::new(rules::PowZeroRule),
        Rc::new(rules::PowOneRule),
        Rc::new(rules::ZeroPowRule),
        Rc::new(rules::OnePowRule),
        Rc::new(rules::ConstantFoldRule),
        Rc::new(rules::FractionSimplifyRule),
    ]
}
