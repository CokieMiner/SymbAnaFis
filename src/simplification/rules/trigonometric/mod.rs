use crate::ast::Expr;
use crate::simplification::helpers;
use crate::simplification::patterns::common::extract_coefficient;
use crate::simplification::patterns::trigonometric::get_trig_function;
use crate::simplification::rules::{Rule, RuleCategory, RuleContext};
use std::f64::consts::PI;
use std::rc::Rc;

/// Rule for sin(0) = 0
pub struct SinZeroRule;

impl Rule for SinZeroRule {
    fn name(&self) -> &'static str {
        "sin_zero"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && name == "sin"
            && args.len() == 1
            && matches!(args[0], Expr::Number(n) if n == 0.0)
        {
            return Some(Expr::Number(0.0));
        }
        None
    }
}

/// Rule for cos(0) = 1
pub struct CosZeroRule;

impl Rule for CosZeroRule {
    fn name(&self) -> &'static str {
        "cos_zero"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && name == "cos"
            && args.len() == 1
            && matches!(args[0], Expr::Number(n) if n == 0.0)
        {
            return Some(Expr::Number(1.0));
        }
        None
    }
}

/// Rule for tan(0) = 0
pub struct TanZeroRule;

impl Rule for TanZeroRule {
    fn name(&self) -> &'static str {
        "tan_zero"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && name == "tan"
            && args.len() == 1
            && matches!(args[0], Expr::Number(n) if n == 0.0)
        {
            return Some(Expr::Number(0.0));
        }
        None
    }
}

/// Rule for sin^2(x) + cos^2(x) = 1
pub struct PythagoreanIdentityRule;

impl Rule for PythagoreanIdentityRule {
    fn name(&self) -> &'static str {
        "pythagorean_identity"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Add(u, v) = expr {
            // Check for sin^2(x) + cos^2(x)
            if let (Expr::Pow(sin_base, sin_exp), Expr::Pow(cos_base, cos_exp)) = (&**u, &**v)
                && matches!(**sin_exp, Expr::Number(n) if n == 2.0)
                && matches!(**cos_exp, Expr::Number(n) if n == 2.0)
                && let (
                    Expr::FunctionCall {
                        name: sin_name,
                        args: sin_args,
                    },
                    Expr::FunctionCall {
                        name: cos_name,
                        args: cos_args,
                    },
                ) = (&**sin_base, &**cos_base)
                && sin_name == "sin"
                && cos_name == "cos"
                && sin_args.len() == 1
                && cos_args.len() == 1
                && sin_args[0] == cos_args[0]
            {
                return Some(Expr::Number(1.0));
            }
            // Check for cos^2(x) + sin^2(x) (reverse order)
            if let (Expr::Pow(cos_base, cos_exp), Expr::Pow(sin_base, sin_exp)) = (&**u, &**v)
                && matches!(**cos_exp, Expr::Number(n) if n == 2.0)
                && matches!(**sin_exp, Expr::Number(n) if n == 2.0)
                && let (
                    Expr::FunctionCall {
                        name: cos_name,
                        args: cos_args,
                    },
                    Expr::FunctionCall {
                        name: sin_name,
                        args: sin_args,
                    },
                ) = (&**cos_base, &**sin_base)
                && cos_name == "cos"
                && sin_name == "sin"
                && cos_args.len() == 1
                && sin_args.len() == 1
                && cos_args[0] == sin_args[0]
            {
                return Some(Expr::Number(1.0));
            }
        }
        None
    }
}

/// Rule for sin(π/2 - x) = cos(x) and cos(π/2 - x) = sin(x)
pub struct CofunctionIdentityRule;

impl Rule for CofunctionIdentityRule {
    fn name(&self) -> &'static str {
        "cofunction_identity"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && args.len() == 1
        {
            if let Expr::Sub(lhs, rhs) = &args[0] {
                // Check for pi/2 - x (symbolic or direct number)
                if (if let Expr::Div(num, den) = &**lhs {
                    helpers::is_pi(num) && matches!(**den, Expr::Number(n) if n == 2.0)
                } else {
                    false
                }) || helpers::approx_eq(
                    helpers::get_numeric_value(lhs),
                    std::f64::consts::PI / 2.0,
                ) {
                    match name.as_str() {
                        "sin" => {
                            return Some(Expr::FunctionCall {
                                name: "cos".to_string(),
                                args: vec![*rhs.clone()],
                            });
                        }
                        "cos" => {
                            return Some(Expr::FunctionCall {
                                name: "sin".to_string(),
                                args: vec![*rhs.clone()],
                            });
                        }
                        "tan" => {
                            return Some(Expr::FunctionCall {
                                name: "cot".to_string(),
                                args: vec![*rhs.clone()],
                            });
                        }
                        "cot" => {
                            return Some(Expr::FunctionCall {
                                name: "tan".to_string(),
                                args: vec![*rhs.clone()],
                            });
                        }
                        "sec" => {
                            return Some(Expr::FunctionCall {
                                name: "csc".to_string(),
                                args: vec![*rhs.clone()],
                            });
                        }
                        "csc" => {
                            return Some(Expr::FunctionCall {
                                name: "sec".to_string(),
                                args: vec![*rhs.clone()],
                            });
                        }
                        _ => {}
                    }
                }
            }

            // Check for Add(pi/2, -x)
            if let Expr::Add(u, v) = &args[0] {
                let (_angle, other) = if helpers::approx_eq(
                    helpers::get_numeric_value(u),
                    std::f64::consts::PI / 2.0,
                ) {
                    (u, v)
                } else if helpers::approx_eq(
                    helpers::get_numeric_value(v),
                    std::f64::consts::PI / 2.0,
                ) {
                    (v, u)
                } else {
                    // Check for symbolic pi/2
                    let is_pi_div_2 = |e: &Expr| {
                        if let Expr::Div(num, den) = e {
                            helpers::is_pi(num) && matches!(**den, Expr::Number(n) if n == 2.0)
                        } else {
                            false
                        }
                    };

                    if is_pi_div_2(u) {
                        (u, v)
                    } else if is_pi_div_2(v) {
                        (v, u)
                    } else {
                        return None;
                    }
                };

                // Check if other is -x
                if let Expr::Mul(c, x) = &**other
                    && matches!(**c, Expr::Number(n) if n == -1.0)
                {
                    match name.as_str() {
                        "sin" => {
                            return Some(Expr::FunctionCall {
                                name: "cos".to_string(),
                                args: vec![*x.clone()],
                            });
                        }
                        "cos" => {
                            return Some(Expr::FunctionCall {
                                name: "sin".to_string(),
                                args: vec![*x.clone()],
                            });
                        }
                        "tan" => {
                            return Some(Expr::FunctionCall {
                                name: "cot".to_string(),
                                args: vec![*x.clone()],
                            });
                        }
                        "cot" => {
                            return Some(Expr::FunctionCall {
                                name: "tan".to_string(),
                                args: vec![*x.clone()],
                            });
                        }
                        "sec" => {
                            return Some(Expr::FunctionCall {
                                name: "csc".to_string(),
                                args: vec![*x.clone()],
                            });
                        }
                        "csc" => {
                            return Some(Expr::FunctionCall {
                                name: "sec".to_string(),
                                args: vec![*x.clone()],
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
        None
    }
}

/// Rule for sin(asin(x)) = x and cos(acos(x)) = x
pub struct InverseTrigIdentityRule;

impl Rule for InverseTrigIdentityRule {
    fn name(&self) -> &'static str {
        "inverse_trig_identity"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn alters_domain(&self) -> bool {
        true
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && args.len() == 1
            && let Expr::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0]
            && inner_args.len() == 1
        {
            let inner_arg = &inner_args[0];
            match (name.as_str(), inner_name.as_str()) {
                ("sin", "asin") | ("cos", "acos") | ("tan", "atan") => {
                    return Some(inner_arg.clone());
                }
                _ => {}
            }
        }
        None
    }
}

/// Rule for asin(sin(x)) = x and acos(cos(x)) = x
pub struct InverseTrigCompositionRule;

impl Rule for InverseTrigCompositionRule {
    fn name(&self) -> &'static str {
        "inverse_trig_composition"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn alters_domain(&self) -> bool {
        true
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && args.len() == 1
            && let Expr::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0]
            && inner_args.len() == 1
        {
            let inner_arg = &inner_args[0];
            match (name.as_str(), inner_name.as_str()) {
                ("asin", "sin") | ("acos", "cos") | ("atan", "tan") => {
                    return Some(inner_arg.clone());
                }
                _ => {}
            }
        }
        None
    }
}

/// Rule for sin(π) = 0
pub struct SinPiRule;

impl Rule for SinPiRule {
    fn name(&self) -> &'static str {
        "sin_pi"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && name == "sin"
            && args.len() == 1
            && helpers::is_pi(&args[0])
        {
            return Some(Expr::Number(0.0));
        }
        None
    }
}

/// Rule for cos(π) = -1
pub struct CosPiRule;

impl Rule for CosPiRule {
    fn name(&self) -> &'static str {
        "cos_pi"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && name == "cos"
            && args.len() == 1
            && helpers::is_pi(&args[0])
        {
            return Some(Expr::Number(-1.0));
        }
        None
    }
}

/// Rule for sin(π/2) = 1
pub struct SinPiOverTwoRule;

impl Rule for SinPiOverTwoRule {
    fn name(&self) -> &'static str {
        "sin_pi_over_two"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && name == "sin"
            && args.len() == 1
            && let Expr::Div(num, den) = &args[0]
            && helpers::is_pi(num)
            && matches!(**den, Expr::Number(n) if n == 2.0)
        {
            return Some(Expr::Number(1.0));
        }
        None
    }
}

/// Rule for cos(π/2) = 0
pub struct CosPiOverTwoRule;

impl Rule for CosPiOverTwoRule {
    fn name(&self) -> &'static str {
        "cos_pi_over_two"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && name == "cos"
            && args.len() == 1
            && let Expr::Div(num, den) = &args[0]
            && helpers::is_pi(num)
            && matches!(**den, Expr::Number(n) if n == 2.0)
        {
            return Some(Expr::Number(0.0));
        }
        None
    }
}

/// Rule for periodicity: sin(x + 2kπ) = sin(x), cos(x + 2kπ) = cos(x)
pub struct TrigPeriodicityRule;

impl Rule for TrigPeriodicityRule {
    fn name(&self) -> &'static str {
        "trig_periodicity"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && (name == "sin" || name == "cos")
            && args.len() == 1
        {
            if let Expr::Add(lhs, rhs) = &args[0] {
                // Check x + 2kπ
                if helpers::is_multiple_of_two_pi(rhs) {
                    return Some(Expr::FunctionCall {
                        name: name.clone(),
                        args: vec![*lhs.clone()],
                    });
                }
                // Check 2kπ + x
                if helpers::is_multiple_of_two_pi(lhs) {
                    return Some(Expr::FunctionCall {
                        name: name.clone(),
                        args: vec![*rhs.clone()],
                    });
                }
            }
            if let Expr::Sub(lhs, rhs) = &args[0] {
                // Check x - 2kπ
                if helpers::is_multiple_of_two_pi(rhs) {
                    return Some(Expr::FunctionCall {
                        name: name.clone(),
                        args: vec![*lhs.clone()],
                    });
                }
                // Check 2kπ - x
                if helpers::is_multiple_of_two_pi(lhs) {
                    // sin(2kπ - x) = sin(-x) = -sin(x)
                    // cos(2kπ - x) = cos(-x) = cos(x)
                    match name.as_str() {
                        "sin" => {
                            return Some(Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "sin".to_string(),
                                    args: vec![*rhs.clone()],
                                }),
                            ));
                        }
                        "cos" => {
                            return Some(Expr::FunctionCall {
                                name: "cos".to_string(),
                                args: vec![*rhs.clone()],
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
        None
    }
}

/// Rule for reflection: sin(π - x) = sin(x), cos(π - x) = -cos(x)
pub struct TrigReflectionRule;

impl Rule for TrigReflectionRule {
    fn name(&self) -> &'static str {
        "trig_reflection"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && (name == "sin" || name == "cos")
            && args.len() == 1
        {
            // Check for Sub(pi, x)
            if let Expr::Sub(lhs, rhs) = &args[0]
                && helpers::is_pi(lhs)
            {
                match name.as_str() {
                    "sin" => {
                        return Some(Expr::FunctionCall {
                            name: "sin".to_string(),
                            args: vec![*rhs.clone()],
                        });
                    }
                    "cos" => {
                        return Some(Expr::Mul(
                            Box::new(Expr::Number(-1.0)),
                            Box::new(Expr::FunctionCall {
                                name: "cos".to_string(),
                                args: vec![*rhs.clone()],
                            }),
                        ));
                    }
                    _ => {}
                }
            }
            // Check for Add(pi, -x) or Add(-x, pi)
            if let Expr::Add(u, v) = &args[0] {
                let (_, other_term) = if helpers::is_pi(u) {
                    (u, v)
                } else if helpers::is_pi(v) {
                    (v, u)
                } else {
                    return None;
                };

                // Check if other_term is -x
                let mut is_neg_x = false;
                if let Expr::Mul(c, x) = &**other_term
                    && matches!(**c, Expr::Number(n) if n == -1.0)
                {
                    is_neg_x = true;
                    match name.as_str() {
                        "sin" => {
                            return Some(Expr::FunctionCall {
                                name: "sin".to_string(),
                                args: vec![*x.clone()],
                            });
                        }
                        "cos" => {
                            return Some(Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "cos".to_string(),
                                    args: vec![*x.clone()],
                                }),
                            ));
                        }
                        _ => {}
                    }
                }

                if !is_neg_x {
                    // pi + x
                    // sin(pi + x) = -sin(x)
                    // cos(pi + x) = -cos(x)
                    match name.as_str() {
                        "sin" | "cos" => {
                            return Some(Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: name.clone(),
                                    args: vec![*other_term.clone()],
                                }),
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
        None
    }
}

/// Rule for sin(3π/2 - x) = -cos(x), cos(3π/2 - x) = -sin(x)
pub struct TrigThreePiOverTwoRule;

impl Rule for TrigThreePiOverTwoRule {
    fn name(&self) -> &'static str {
        "trig_three_pi_over_two"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && (name == "sin" || name == "cos")
            && args.len() == 1
        {
            if let Expr::Sub(lhs, rhs) = &args[0]
                && helpers::is_three_pi_over_two(lhs)
            {
                match name.as_str() {
                    "sin" => {
                        return Some(Expr::Mul(
                            Box::new(Expr::Number(-1.0)),
                            Box::new(Expr::FunctionCall {
                                name: "cos".to_string(),
                                args: vec![*rhs.clone()],
                            }),
                        ));
                    }
                    "cos" => {
                        return Some(Expr::Mul(
                            Box::new(Expr::Number(-1.0)),
                            Box::new(Expr::FunctionCall {
                                name: "sin".to_string(),
                                args: vec![*rhs.clone()],
                            }),
                        ));
                    }
                    _ => {}
                }
            }

            // Check for Add(3pi/2, -x)
            if let Expr::Add(u, v) = &args[0] {
                let (_angle, other) = if helpers::is_three_pi_over_two(u) {
                    (u, v)
                } else if helpers::is_three_pi_over_two(v) {
                    (v, u)
                } else {
                    return None;
                };

                // Check if other is -x
                if let Expr::Mul(c, x) = &**other
                    && matches!(**c, Expr::Number(n) if n == -1.0)
                {
                    // sin(3pi/2 - x) = -cos(x)
                    // cos(3pi/2 - x) = -sin(x)
                    match name.as_str() {
                        "sin" => {
                            return Some(Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "cos".to_string(),
                                    args: vec![*x.clone()],
                                }),
                            ));
                        }
                        "cos" => {
                            return Some(Expr::Mul(
                                Box::new(Expr::Number(-1.0)),
                                Box::new(Expr::FunctionCall {
                                    name: "sin".to_string(),
                                    args: vec![*x.clone()],
                                }),
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
        None
    }
}

/// Rule for Pythagorean identities: 1 - cos²(x) = sin²(x), 1 - sin²(x) = cos²(x)
/// Also handles canonicalized forms like -cos²(x) + 1
pub struct PythagoreanComplementsRule;

impl Rule for PythagoreanComplementsRule {
    fn name(&self) -> &'static str {
        "pythagorean_complements"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        // Handle 1 - cos²(x) or 1 - sin²(x) (direct subtraction form)
        if let Expr::Sub(lhs, rhs) = expr
            && matches!(**lhs, Expr::Number(n) if n == 1.0)
        {
            // 1 - cos²(x) = sin²(x)
            if let Some(("cos", arg)) = helpers::get_fn_pow_named(rhs, 2.0) {
                return Some(Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "sin".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                ));
            }
            // 1 - sin²(x) = cos²(x)
            if let Some(("sin", arg)) = helpers::get_fn_pow_named(rhs, 2.0) {
                return Some(Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "cos".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                ));
            }
        }

        // Handle canonicalized form: -cos²(x) + 1 or -sin²(x) + 1
        // This is Add(-1 * trig²(x), 1) or Add(1, -1 * trig²(x))
        if let Expr::Add(lhs, rhs) = expr {
            // Case: -cos²(x) + 1 or -sin²(x) + 1
            if matches!(**rhs, Expr::Number(n) if n == 1.0)
                && let Expr::Mul(coef, rest) = &**lhs
                && matches!(**coef, Expr::Number(n) if n == -1.0)
            {
                if let Some(("cos", arg)) = helpers::get_fn_pow_named(rest, 2.0) {
                    return Some(Expr::Pow(
                        Box::new(Expr::FunctionCall {
                            name: "sin".to_string(),
                            args: vec![arg],
                        }),
                        Box::new(Expr::Number(2.0)),
                    ));
                }
                if let Some(("sin", arg)) = helpers::get_fn_pow_named(rest, 2.0) {
                    return Some(Expr::Pow(
                        Box::new(Expr::FunctionCall {
                            name: "cos".to_string(),
                            args: vec![arg],
                        }),
                        Box::new(Expr::Number(2.0)),
                    ));
                }
            }
            // Case: 1 + (-cos²(x)) or 1 + (-sin²(x))
            if matches!(**lhs, Expr::Number(n) if n == 1.0)
                && let Expr::Mul(coef, rest) = &**rhs
                && matches!(**coef, Expr::Number(n) if n == -1.0)
            {
                if let Some(("cos", arg)) = helpers::get_fn_pow_named(rest, 2.0) {
                    return Some(Expr::Pow(
                        Box::new(Expr::FunctionCall {
                            name: "sin".to_string(),
                            args: vec![arg],
                        }),
                        Box::new(Expr::Number(2.0)),
                    ));
                }
                if let Some(("sin", arg)) = helpers::get_fn_pow_named(rest, 2.0) {
                    return Some(Expr::Pow(
                        Box::new(Expr::FunctionCall {
                            name: "cos".to_string(),
                            args: vec![arg],
                        }),
                        Box::new(Expr::Number(2.0)),
                    ));
                }
            }
        }

        None
    }
}

/// Rule for tan^2(x) + 1 = sec^2(x) and cot^2(x) + 1 = csc^2(x)
pub struct PythagoreanTangentRule;

impl Rule for PythagoreanTangentRule {
    fn name(&self) -> &'static str {
        "pythagorean_tangent"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Add(lhs, rhs) = expr {
            // Check for tan^2(x) + 1 = sec^2(x)
            if let Some(("tan", arg)) = helpers::get_fn_pow_named(lhs, 2.0)
                && matches!(**rhs, Expr::Number(n) if n == 1.0)
            {
                return Some(Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "sec".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                ));
            }
            // Check for 1 + tan^2(x) = sec^2(x)
            if matches!(**lhs, Expr::Number(n) if n == 1.0)
                && let Some(("tan", arg)) = helpers::get_fn_pow_named(rhs, 2.0)
            {
                return Some(Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "sec".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                ));
            }
            // Check for cot^2(x) + 1 = csc^2(x)
            if let Some(("cot", arg)) = helpers::get_fn_pow_named(lhs, 2.0)
                && matches!(**rhs, Expr::Number(n) if n == 1.0)
            {
                return Some(Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "csc".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                ));
            }
            // Check for 1 + cot^2(x) = csc^2(x)
            if matches!(**lhs, Expr::Number(n) if n == 1.0)
                && let Some(("cot", arg)) = helpers::get_fn_pow_named(rhs, 2.0)
            {
                return Some(Expr::Pow(
                    Box::new(Expr::FunctionCall {
                        name: "csc".to_string(),
                        args: vec![arg],
                    }),
                    Box::new(Expr::Number(2.0)),
                ));
            }
        }
        None
    }
}

/// Rule for sin(π/6) = 1/2, cos(π/3) = 1/2, etc. (handles both Div and direct Number forms)
pub struct TrigExactValuesRule;

impl Rule for TrigExactValuesRule {
    fn name(&self) -> &'static str {
        "trig_exact_values"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && args.len() == 1
        {
            let arg = &args[0];
            let arg_val = helpers::get_numeric_value(arg);
            let is_numeric_input = matches!(arg, Expr::Number(_));

            // Handle π/6, π/4, π/3 as direct numbers or Div expressions
            match name.as_str() {
                "sin" => {
                    // sin(π/6) = 1/2
                    if helpers::approx_eq(arg_val, PI / 6.0)
                        || (matches!(arg, Expr::Div(num, den) if helpers::is_pi(num) && matches!(**den, Expr::Number(n) if n == 6.0)))
                    {
                        return if is_numeric_input {
                            Some(Expr::Number(0.5))
                        } else {
                            Some(Expr::Div(
                                Box::new(Expr::Number(1.0)),
                                Box::new(Expr::Number(2.0)),
                            ))
                        };
                    }
                    // sin(π/4) = √2/2
                    if helpers::approx_eq(arg_val, PI / 4.0)
                        || (matches!(arg, Expr::Div(num, den) if helpers::is_pi(num) && matches!(**den, Expr::Number(n) if n == 4.0)))
                    {
                        return if is_numeric_input {
                            Some(Expr::Number((2.0f64).sqrt() / 2.0))
                        } else {
                            Some(Expr::Div(
                                Box::new(Expr::FunctionCall {
                                    name: "sqrt".to_string(),
                                    args: vec![Expr::Number(2.0)],
                                }),
                                Box::new(Expr::Number(2.0)),
                            ))
                        };
                    }
                }
                "cos" => {
                    // cos(π/3) = 1/2
                    if helpers::approx_eq(arg_val, PI / 3.0)
                        || (matches!(arg, Expr::Div(num, den) if helpers::is_pi(num) && matches!(**den, Expr::Number(n) if n == 3.0)))
                    {
                        return if is_numeric_input {
                            Some(Expr::Number(0.5))
                        } else {
                            Some(Expr::Div(
                                Box::new(Expr::Number(1.0)),
                                Box::new(Expr::Number(2.0)),
                            ))
                        };
                    }
                    // cos(π/4) = √2/2
                    if helpers::approx_eq(arg_val, PI / 4.0)
                        || (matches!(arg, Expr::Div(num, den) if helpers::is_pi(num) && matches!(**den, Expr::Number(n) if n == 4.0)))
                    {
                        return if is_numeric_input {
                            Some(Expr::Number((2.0f64).sqrt() / 2.0))
                        } else {
                            Some(Expr::Div(
                                Box::new(Expr::FunctionCall {
                                    name: "sqrt".to_string(),
                                    args: vec![Expr::Number(2.0)],
                                }),
                                Box::new(Expr::Number(2.0)),
                            ))
                        };
                    }
                }
                "tan" => {
                    // tan(π/4) = 1
                    if helpers::approx_eq(arg_val, PI / 4.0)
                        || (matches!(arg, Expr::Div(num, den) if helpers::is_pi(num) && matches!(**den, Expr::Number(n) if n == 4.0)))
                    {
                        return Some(Expr::Number(1.0));
                    }
                    // tan(π/3) = √3
                    if helpers::approx_eq(arg_val, PI / 3.0)
                        || (matches!(arg, Expr::Div(num, den) if helpers::is_pi(num) && matches!(**den, Expr::Number(n) if n == 3.0)))
                    {
                        return if is_numeric_input {
                            Some(Expr::Number((3.0f64).sqrt()))
                        } else {
                            Some(Expr::FunctionCall {
                                name: "sqrt".to_string(),
                                args: vec![Expr::Number(3.0)],
                            })
                        };
                    }
                    // tan(π/6) = 1/√3 = √3/3
                    if helpers::approx_eq(arg_val, PI / 6.0)
                        || (matches!(arg, Expr::Div(num, den) if helpers::is_pi(num) && matches!(**den, Expr::Number(n) if n == 6.0)))
                    {
                        return if is_numeric_input {
                            Some(Expr::Number(1.0 / (3.0f64).sqrt()))
                        } else {
                            Some(Expr::Div(
                                Box::new(Expr::FunctionCall {
                                    name: "sqrt".to_string(),
                                    args: vec![Expr::Number(3.0)],
                                }),
                                Box::new(Expr::Number(3.0)),
                            ))
                        };
                    }
                }
                _ => {}
            }
        }
        None
    }
}

/// Rule for sin(-x) = -sin(x), cos(-x) = cos(x), etc., using get_trig_function
pub struct TrigNegArgRule;

impl Rule for TrigNegArgRule {
    fn name(&self) -> &'static str {
        "trig_neg_arg"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Some((name, arg)) = get_trig_function(expr)
            && let Expr::Mul(coeff, inner) = &arg
            && let Expr::Number(n) = **coeff
            && n == -1.0
        {
            match name {
                "sin" | "tan" => {
                    return Some(Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::FunctionCall {
                            name: name.to_string(),
                            args: vec![*inner.clone()],
                        }),
                    ));
                }
                "cos" | "sec" => {
                    return Some(Expr::FunctionCall {
                        name: name.to_string(),
                        args: vec![*inner.clone()],
                    });
                }
                _ => {}
            }
        }
        None
    }
}

/// Rule for sin(2*x) = 2*sin(x)*cos(x), using extract_coefficient
pub struct TrigDoubleAngleRule;

impl Rule for TrigDoubleAngleRule {
    fn name(&self) -> &'static str {
        "trig_double_angle"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Some(("sin", arg)) = get_trig_function(expr) {
            let (coeff, rest) = extract_coefficient(&arg);
            if coeff == 2.0 {
                return Some(Expr::Mul(
                    Box::new(Expr::Number(2.0)),
                    Box::new(Expr::Mul(
                        Box::new(Expr::FunctionCall {
                            name: "sin".to_string(),
                            args: vec![rest.clone()],
                        }),
                        Box::new(Expr::FunctionCall {
                            name: "cos".to_string(),
                            args: vec![rest],
                        }),
                    )),
                ));
            }
        }
        None
    }
}

/// Rule for sum/difference identities: sin(x+y), cos(x-y), etc.
pub struct TrigSumDifferenceRule;

impl Rule for TrigSumDifferenceRule {
    fn name(&self) -> &'static str {
        "trig_sum_difference"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        match expr {
            Expr::Add(u, v) => {
                // sin(x)cos(y) + cos(x)sin(y) = sin(x + y)
                if let Some((x, y)) = helpers::get_product_fn_args(u, "sin", "cos")
                    .and_then(|(s1, c1)| {
                        helpers::get_product_fn_args(v, "sin", "cos")
                            .map(|(s2, c2)| (s1, c1, s2, c2))
                    })
                    .and_then(|(s1, c1, s2, c2)| {
                        if s1 == c2 && c1 == s2 {
                            Some((s1, c1))
                        } else {
                            None
                        }
                    })
                {
                    return Some(Expr::FunctionCall {
                        name: "sin".to_string(),
                        args: vec![Expr::Add(Box::new(x), Box::new(y))],
                    });
                }
                // cos(x)cos(y) - sin(x)sin(y) = cos(x + y) (handled in Sub)
                // But here we might have cos(x)cos(y) + (-sin(x)sin(y))
                // This is complex to match in Add without more helpers.
                // Let's stick to the simple cases first or rely on Sub for the negative case.
            }
            Expr::Sub(u, v) => {
                // sin(x)cos(y) - cos(x)sin(y) = sin(x - y)
                if let Some((x, y)) = helpers::get_product_fn_args(u, "sin", "cos")
                    .and_then(|(s1, c1)| {
                        helpers::get_product_fn_args(v, "cos", "sin")
                            .map(|(c2, s2)| (s1, c1, c2, s2))
                    })
                    .and_then(|(s1, c1, c2, s2)| {
                        if s1 == c2 && c1 == s2 {
                            Some((s1, c1))
                        } else {
                            None
                        }
                    })
                {
                    return Some(Expr::FunctionCall {
                        name: "sin".to_string(),
                        args: vec![Expr::Sub(Box::new(x), Box::new(y))],
                    });
                }
                // cos(x)cos(y) - sin(x)sin(y) = cos(x + y)
                if let Some((cx, cy)) = helpers::get_product_fn_args(u, "cos", "cos")
                    && let Some((sx, sy)) = helpers::get_product_fn_args(v, "sin", "sin")
                    && ((cx == sx && cy == sy) || (cx == sy && cy == sx))
                {
                    return Some(Expr::FunctionCall {
                        name: "cos".to_string(),
                        args: vec![Expr::Add(Box::new(cx), Box::new(cy))],
                    });
                }

                // cos(x)cos(y) + sin(x)sin(y) = cos(x - y) (Add case)
            }
            _ => {}
        }
        None
    }
}

pub struct CosDoubleAngleDifferenceRule;

impl Rule for CosDoubleAngleDifferenceRule {
    fn name(&self) -> &'static str {
        "cos_double_angle_difference"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        let (pos, neg) = match expr {
            Expr::Sub(u, v) => (u, v),
            Expr::Add(u, v) => {
                // Check if u is negative
                if let Expr::Mul(c, inner) = &**u {
                    if matches!(**c, Expr::Number(n) if n == -1.0) {
                        (v, inner)
                    } else {
                        // Check if v is negative
                        if let Expr::Mul(c, inner) = &**v {
                            if matches!(**c, Expr::Number(n) if n == -1.0) {
                                (u, inner)
                            } else {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }
                } else {
                    // Check if v is negative
                    if let Expr::Mul(c, inner) = &**v {
                        if matches!(**c, Expr::Number(n) if n == -1.0) {
                            (u, inner)
                        } else {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
            }
            _ => return None,
        };

        // Case 1: cos^2(x) - sin^2(x) = cos(2x)
        // pos = cos^2(x), neg = sin^2(x)
        if let Some(("cos", arg1)) = helpers::get_fn_pow_named(pos, 2.0)
            && let Some(("sin", arg2)) = helpers::get_fn_pow_named(neg, 2.0)
            && arg1 == arg2
        {
            return Some(Expr::FunctionCall {
                name: "cos".to_string(),
                args: vec![Expr::Mul(Box::new(Expr::Number(2.0)), Box::new(arg1))],
            });
        }

        // Case 2: sin^2(x) - cos^2(x) = -cos(2x)
        // pos = sin^2(x), neg = cos^2(x)
        if let Some(("sin", arg1)) = helpers::get_fn_pow_named(pos, 2.0)
            && let Some(("cos", arg2)) = helpers::get_fn_pow_named(neg, 2.0)
            && arg1 == arg2
        {
            return Some(Expr::Mul(
                Box::new(Expr::Number(-1.0)),
                Box::new(Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(2.0)), Box::new(arg1))],
                }),
            ));
        }

        None
    }
}

/// Rule for (cos(x) - sin(x)) * (cos(x) + sin(x)) = cos(2x)
pub struct TrigProductToDoubleAngleRule;

impl Rule for TrigProductToDoubleAngleRule {
    fn name(&self) -> &'static str {
        "trig_product_to_double_angle"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Mul(a, b) = expr {
            // Check if it's (cos(x) - sin(x)) * (cos(x) + sin(x))
            let (cos_minus_sin, cos_plus_sin) = if is_cos_minus_sin(a) && is_cos_plus_sin(b) {
                (a, b)
            } else if is_cos_minus_sin(b) && is_cos_plus_sin(a) {
                (b, a)
            } else {
                return None;
            };

            // Extract the argument from cos
            if let Some(arg) = get_cos_arg(cos_minus_sin)
                && get_cos_arg(cos_plus_sin) == Some(arg.clone())
                && get_sin_arg(cos_minus_sin) == Some(arg.clone())
                && get_sin_arg(cos_plus_sin) == Some(arg.clone())
            {
                return Some(Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(2.0)), Box::new(arg))],
                });
            }
        }
        None
    }
}

fn is_cos_minus_sin(expr: &Expr) -> bool {
    if let Expr::Sub(a, b) = expr {
        is_cos(a) && is_sin(b)
    } else if let Expr::Add(a, b) = expr {
        if let Expr::Mul(left, right) = &**b {
            if matches!(**left, Expr::Number(n) if n == -1.0) {
                is_cos(a) && is_sin(right)
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

fn is_cos_plus_sin(expr: &Expr) -> bool {
    if let Expr::Add(a, b) = expr {
        is_cos(a) && is_sin(b)
    } else {
        false
    }
}

fn is_cos(expr: &Expr) -> bool {
    matches!(expr, Expr::FunctionCall { name, args } if name == "cos" && args.len() == 1)
}

fn is_sin(expr: &Expr) -> bool {
    matches!(expr, Expr::FunctionCall { name, args } if name == "sin" && args.len() == 1)
}

fn get_cos_arg(expr: &Expr) -> Option<Expr> {
    if let Expr::FunctionCall { name, args } = expr {
        if name == "cos" && args.len() == 1 {
            Some(args[0].clone())
        } else {
            None
        }
    } else if let Expr::Add(a, _) = expr {
        get_cos_arg(a)
    } else if let Expr::Sub(a, _) = expr {
        get_cos_arg(a)
    } else {
        None
    }
}

fn get_sin_arg(expr: &Expr) -> Option<Expr> {
    if let Expr::FunctionCall { name, args } = expr {
        if name == "sin" && args.len() == 1 {
            Some(args[0].clone())
        } else {
            None
        }
    } else if let Expr::Add(_, b) = expr {
        if let Expr::Mul(left, right) = &**b {
            if matches!(**left, Expr::Number(n) if n == -1.0) {
                get_sin_arg(right)
            } else {
                None
            }
        } else {
            get_sin_arg(b)
        }
    } else if let Expr::Sub(_, b) = expr {
        get_sin_arg(b)
    } else {
        None
    }
}

/// Rule for triple angle folding: 3sin(x) - 4sin^3(x) -> sin(3x), 4cos^3(x) - 3cos(x) -> cos(3x)
pub struct TrigTripleAngleRule;

impl Rule for TrigTripleAngleRule {
    fn name(&self) -> &'static str {
        "trig_triple_angle"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Trigonometric
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        let eps = 1e-10;
        match expr {
            Expr::Sub(u, v) => {
                // Check for 3sin(x) - 4sin^3(x)
                if let Some(result) = self.check_sin_triple(u, v, eps) {
                    return Some(result);
                }
                // Check for 4cos^3(x) - 3cos(x)
                if let Some(result) = self.check_cos_triple(u, v, eps) {
                    return Some(result);
                }
            }
            Expr::Add(u, v) => {
                // Check for Add forms where one term is negative
                // This handles cases where Sub was normalized to Add with negated term
                // Pattern: 3sin(x) + (-4)*sin^3(x) => sin(3x)
                if let Some(result) = self.check_sin_triple_add(u, v, eps) {
                    return Some(result);
                }
                // Pattern: 4cos^3(x) + (-3)*cos(x) => cos(3x)
                if let Some(result) = self.check_cos_triple_add(u, v, eps) {
                    return Some(result);
                }
            }
            _ => {}
        }
        None
    }
}

impl TrigTripleAngleRule {
    fn check_sin_triple(&self, u: &Expr, v: &Expr, eps: f64) -> Option<Expr> {
        // Check for 3sin(x) - 4sin^3(x) or permutations
        if let Expr::Mul(c1, s1) = u
            && matches!(**c1, Expr::Number(n) if n == 3.0 || (n - 3.0).abs() < eps)
            && let Expr::FunctionCall { name, args } = &**s1
            && name == "sin"
            && args.len() == 1
        {
            let x = &args[0];
            // Check v = 4sin^3(x) or -4sin^3(x)
            if let Some((coeff, _is_neg)) = self.extract_sin_cubed(v, x, eps)
                && (coeff == 4.0 || (coeff - 4.0).abs() < eps)
            {
                return Some(Expr::FunctionCall {
                    name: "sin".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(3.0)), Box::new(x.clone()))],
                });
            }
        }
        None
    }

    fn check_sin_triple_add(&self, u: &Expr, v: &Expr, eps: f64) -> Option<Expr> {
        // Check for 3sin(x) + (-4)*sin^3(x) pattern
        // u = 3*sin(x), v = (-4)*sin^3(x)
        if let Expr::Mul(c1, s1) = u
            && matches!(**c1, Expr::Number(n) if (n - 3.0).abs() < eps)
            && let Expr::FunctionCall { name, args } = &**s1
            && name == "sin"
            && args.len() == 1
        {
            let x = &args[0];
            // Check v = (-4)*sin^3(x)
            if let Some((coeff, is_neg)) = self.extract_sin_cubed(v, x, eps)
                && is_neg
                && (coeff - 4.0).abs() < eps
            {
                return Some(Expr::FunctionCall {
                    name: "sin".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(3.0)), Box::new(x.clone()))],
                });
            }
        }
        // Also check reversed: (-4)*sin^3(x) + 3*sin(x)
        if let Expr::Mul(c1, s1) = v
            && matches!(**c1, Expr::Number(n) if (n - 3.0).abs() < eps)
            && let Expr::FunctionCall { name, args } = &**s1
            && name == "sin"
            && args.len() == 1
        {
            let x = &args[0];
            // Check u = (-4)*sin^3(x)
            if let Some((coeff, is_neg)) = self.extract_sin_cubed(u, x, eps)
                && is_neg
                && (coeff - 4.0).abs() < eps
            {
                return Some(Expr::FunctionCall {
                    name: "sin".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(3.0)), Box::new(x.clone()))],
                });
            }
        }
        None
    }

    fn check_cos_triple(&self, u: &Expr, v: &Expr, eps: f64) -> Option<Expr> {
        // Check for 4cos^3(x) - 3cos(x) or permutations
        if let Expr::Mul(c1, c3) = u
            && matches!(**c1, Expr::Number(n) if n == 4.0 || (n - 4.0).abs() < eps)
            && let Expr::Pow(base, exp) = &**c3
            && matches!(**exp, Expr::Number(n) if n == 3.0)
            && let Expr::FunctionCall { name, args } = &**base
            && name == "cos"
            && args.len() == 1
        {
            let x = &args[0];
            // Check v = 3cos(x) or -3cos(x)
            if let Some((coeff, _is_neg)) = self.extract_cos(v, x, eps)
                && (coeff == 3.0 || (coeff - 3.0).abs() < eps)
            {
                return Some(Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(3.0)), Box::new(x.clone()))],
                });
            }
        }
        None
    }

    fn check_cos_triple_add(&self, u: &Expr, v: &Expr, eps: f64) -> Option<Expr> {
        // Check for 4cos^3(x) + (-3)*cos(x) pattern
        // u = 4*cos^3(x), v = (-3)*cos(x)
        if let Expr::Mul(c1, c3) = u
            && matches!(**c1, Expr::Number(n) if (n - 4.0).abs() < eps)
            && let Expr::Pow(base, exp) = &**c3
            && matches!(**exp, Expr::Number(n) if n == 3.0)
            && let Expr::FunctionCall { name, args } = &**base
            && name == "cos"
            && args.len() == 1
        {
            let x = &args[0];
            // Check v = (-3)*cos(x)
            if let Some((coeff, is_neg)) = self.extract_cos(v, x, eps)
                && is_neg
                && (coeff - 3.0).abs() < eps
            {
                return Some(Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(3.0)), Box::new(x.clone()))],
                });
            }
        }
        // Also check reversed: (-3)*cos(x) + 4*cos^3(x)
        if let Expr::Mul(c1, c3) = v
            && matches!(**c1, Expr::Number(n) if (n - 4.0).abs() < eps)
            && let Expr::Pow(base, exp) = &**c3
            && matches!(**exp, Expr::Number(n) if n == 3.0)
            && let Expr::FunctionCall { name, args } = &**base
            && name == "cos"
            && args.len() == 1
        {
            let x = &args[0];
            // Check u = (-3)*cos(x)
            if let Some((coeff, is_neg)) = self.extract_cos(u, x, eps)
                && is_neg
                && (coeff - 3.0).abs() < eps
            {
                return Some(Expr::FunctionCall {
                    name: "cos".to_string(),
                    args: vec![Expr::Mul(Box::new(Expr::Number(3.0)), Box::new(x.clone()))],
                });
            }
        }
        None
    }

    fn extract_sin_cubed(&self, expr: &Expr, x: &Expr, _eps: f64) -> Option<(f64, bool)> {
        if let Expr::Mul(c, s3) = expr
            && let Expr::Pow(base, exp) = &**s3
            && matches!(**exp, Expr::Number(n) if n == 3.0)
            && let Expr::FunctionCall { name, args } = &**base
            && name == "sin"
            && args.len() == 1
            && args[0] == *x
            && let Expr::Number(n) = **c
        {
            return Some((n.abs(), n < 0.0));
        }
        None
    }

    fn extract_cos(&self, expr: &Expr, x: &Expr, _eps: f64) -> Option<(f64, bool)> {
        if let Expr::Mul(c, c1) = expr
            && let Expr::FunctionCall { name, args } = &**c1
            && name == "cos"
            && args.len() == 1
            && args[0] == *x
            && let Expr::Number(n) = **c
        {
            return Some((n.abs(), n < 0.0));
        }
        None
    }
}

/// Get all trigonometric rules in priority order
pub fn get_trigonometric_rules() -> Vec<Rc<dyn Rule>> {
    vec![
        Rc::new(SinZeroRule),
        Rc::new(CosZeroRule),
        Rc::new(TanZeroRule),
        Rc::new(SinPiRule),
        Rc::new(CosPiRule),
        Rc::new(SinPiOverTwoRule),
        Rc::new(CosPiOverTwoRule),
        Rc::new(TrigExactValuesRule),
        Rc::new(TrigPeriodicityRule),
        Rc::new(TrigReflectionRule),
        Rc::new(TrigThreePiOverTwoRule),
        Rc::new(PythagoreanIdentityRule),
        Rc::new(PythagoreanComplementsRule),
        Rc::new(PythagoreanTangentRule),
        Rc::new(CofunctionIdentityRule),
        Rc::new(InverseTrigIdentityRule),
        Rc::new(InverseTrigCompositionRule),
        Rc::new(TrigNegArgRule),
        Rc::new(TrigDoubleAngleRule),
        Rc::new(CosDoubleAngleDifferenceRule),
        Rc::new(TrigProductToDoubleAngleRule),
        Rc::new(TrigSumDifferenceRule),
        Rc::new(TrigTripleAngleRule),
    ]
}
