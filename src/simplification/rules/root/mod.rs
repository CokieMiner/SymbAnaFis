use crate::ast::Expr;
use crate::simplification::helpers::is_known_non_negative;
use crate::simplification::rules::{Rule, RuleCategory, RuleContext};
use std::rc::Rc;

/// Rule for sqrt(x^n) = x^(n/2)
/// Special case: sqrt(x^2) = abs(x) (since sqrt(x^2) = |x| for all real x)
pub struct SqrtPowerRule;

impl Rule for SqrtPowerRule {
    fn name(&self) -> &'static str {
        "sqrt_power"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Root
    }

    fn alters_domain(&self) -> bool {
        false // No longer alters domain since we use abs(x) for sqrt(x^2)
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && name == "sqrt"
            && args.len() == 1
            && let Expr::Pow(base, exp) = &args[0]
        {
            // Special case: sqrt(x^2) should always return abs(x)
            if let Expr::Number(n) = &**exp
                && *n == 2.0
            {
                // sqrt(x^2) = |x|
                return Some(Expr::FunctionCall {
                    name: "abs".to_string(),
                    args: vec![(**base).clone()],
                });
            }

            // Create new exponent: exp / 2
            let new_exp = Expr::Div(exp.clone(), Box::new(Expr::Number(2.0)));

            // Simplify the division immediately
            let simplified_exp = match &new_exp {
                Expr::Div(u, v) => {
                    if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v) {
                        if *b != 0.0 {
                            let result = a / b;
                            if (result - result.round()).abs() < 1e-10 {
                                Expr::Number(result.round())
                            } else {
                                new_exp
                            }
                        } else {
                            new_exp
                        }
                    } else {
                        new_exp
                    }
                }
                _ => new_exp,
            };

            // If exponent simplified to 1, return base directly
            if matches!(simplified_exp, Expr::Number(n) if n == 1.0) {
                return Some((**base).clone());
            }

            let result = Expr::Pow(base.clone(), Box::new(simplified_exp.clone()));

            return Some(result);
        }
        None
    }
}

/// Rule for cbrt(x^n) = x^(n/3)
pub struct CbrtPowerRule;

impl Rule for CbrtPowerRule {
    fn name(&self) -> &'static str {
        "cbrt_power"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Root
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && name == "cbrt"
            && args.len() == 1
            && let Expr::Pow(base, exp) = &args[0]
        {
            // Create new exponent: exp / 3
            let new_exp = Expr::Div(exp.clone(), Box::new(Expr::Number(3.0)));

            // Simplify the division immediately
            let simplified_exp = match &new_exp {
                Expr::Div(u, v) => {
                    if let (Expr::Number(a), Expr::Number(b)) = (&**u, &**v) {
                        if *b != 0.0 {
                            let result = a / b;
                            if (result - result.round()).abs() < 1e-10 {
                                Expr::Number(result.round())
                            } else {
                                new_exp
                            }
                        } else {
                            new_exp
                        }
                    } else {
                        new_exp
                    }
                }
                _ => new_exp,
            };

            // If exponent simplified to 1, return base directly
            if matches!(simplified_exp, Expr::Number(n) if n == 1.0) {
                return Some((**base).clone());
            }

            return Some(Expr::Pow(base.clone(), Box::new(simplified_exp)));
        }
        None
    }
}

/// Rule for sqrt(x) * sqrt(y) = sqrt(x*y)
/// Safe when both x and y are known to be non-negative
pub struct SqrtMulRule;

impl Rule for SqrtMulRule {
    fn name(&self) -> &'static str {
        "sqrt_mul"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Root
    }

    fn alters_domain(&self) -> bool {
        // Return false so the engine always calls apply()
        // We handle domain-safety logic inside apply() based on whether args are known non-negative
        false
    }

    fn apply(&self, expr: &Expr, context: &RuleContext) -> Option<Expr> {
        if let Expr::Mul(u, v) = expr {
            // Check for sqrt(a) * sqrt(b)
            if let (
                Expr::FunctionCall {
                    name: u_name,
                    args: u_args,
                },
                Expr::FunctionCall {
                    name: v_name,
                    args: v_args,
                },
            ) = (&**u, &**v)
                && u_name == "sqrt"
                && v_name == "sqrt"
                && u_args.len() == 1
                && v_args.len() == 1
            {
                // Check if both arguments are known to be non-negative
                let a_non_neg = is_known_non_negative(&u_args[0]);
                let b_non_neg = is_known_non_negative(&v_args[0]);

                // If in domain-safe mode and we can't prove both are non-negative, skip
                if context.domain_safe && !(a_non_neg && b_non_neg) {
                    return None;
                }

                return Some(Expr::FunctionCall {
                    name: "sqrt".to_string(),
                    args: vec![Expr::Mul(
                        Box::new(u_args[0].clone()),
                        Box::new(v_args[0].clone()),
                    )],
                });
            }
        }
        None
    }
}

/// Rule for sqrt(x)/sqrt(y) = sqrt(x/y)
/// Safe when both x and y are known to be non-negative (and y != 0)
pub struct SqrtDivRule;

impl Rule for SqrtDivRule {
    fn name(&self) -> &'static str {
        "sqrt_div"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Root
    }

    fn alters_domain(&self) -> bool {
        // Return false so the engine always calls apply()
        // We handle domain-safety logic inside apply() based on whether args are known non-negative
        false
    }

    fn apply(&self, expr: &Expr, context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(u, v) = expr {
            // Check for sqrt(a) / sqrt(b)
            if let (
                Expr::FunctionCall {
                    name: u_name,
                    args: u_args,
                },
                Expr::FunctionCall {
                    name: v_name,
                    args: v_args,
                },
            ) = (&**u, &**v)
                && u_name == "sqrt"
                && v_name == "sqrt"
                && u_args.len() == 1
                && v_args.len() == 1
            {
                // Check if both arguments are known to be non-negative
                let a_non_neg = is_known_non_negative(&u_args[0]);
                let b_non_neg = is_known_non_negative(&v_args[0]);

                // If in domain-safe mode and we can't prove both are non-negative, skip
                if context.domain_safe && !(a_non_neg && b_non_neg) {
                    return None;
                }

                return Some(Expr::FunctionCall {
                    name: "sqrt".to_string(),
                    args: vec![Expr::Div(
                        Box::new(u_args[0].clone()),
                        Box::new(v_args[0].clone()),
                    )],
                });
            }
        }
        None
    }
}

/// Rule for x^(1/n) = nth root of x
pub struct PowerToRootRule;

impl Rule for PowerToRootRule {
    fn name(&self) -> &'static str {
        "power_to_root"
    }

    fn priority(&self) -> i32 {
        75
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Root
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Pow(base, exp) = expr
            && let Expr::Div(num, den) = &**exp
            && matches!(**num, Expr::Number(n) if n == 1.0)
            && let Expr::Number(n) = &**den
        {
            if *n == 2.0 {
                return Some(Expr::FunctionCall {
                    name: "sqrt".to_string(),
                    args: vec![(**base).clone()],
                });
            } else if *n == 3.0 {
                return Some(Expr::FunctionCall {
                    name: "cbrt".to_string(),
                    args: vec![(**base).clone()],
                });
            }
        }
        None
    }
}

/// Rule that applies the monolithic root normalization
pub struct NormalizeRootsRule;

impl Rule for NormalizeRootsRule {
    fn name(&self) -> &'static str {
        "normalize_roots"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Root
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && args.len() == 1
        {
            match name.as_str() {
                "sqrt" => {
                    return Some(Expr::Pow(
                        Box::new(args[0].clone()),
                        Box::new(Expr::Div(
                            Box::new(Expr::Number(1.0)),
                            Box::new(Expr::Number(2.0)),
                        )),
                    ));
                }
                "cbrt" => {
                    return Some(Expr::Pow(
                        Box::new(args[0].clone()),
                        Box::new(Expr::Div(
                            Box::new(Expr::Number(1.0)),
                            Box::new(Expr::Number(3.0)),
                        )),
                    ));
                }
                _ => {}
            }
        }
        None
    }
}

/// Get all root simplification rules in priority order
pub fn get_root_rules() -> Vec<Rc<dyn Rule>> {
    vec![
        Rc::new(SqrtPowerRule),
        Rc::new(CbrtPowerRule),
        Rc::new(SqrtMulRule),
        Rc::new(SqrtDivRule),
        Rc::new(PowerToRootRule),
        Rc::new(NormalizeRootsRule),
    ]
}
