use super::{Rule, RuleCategory, RuleContext, RuleExprKind, approx_eq, get_numeric_value, is_pi};
use crate::EPSILON;
use crate::core::known_symbols::{KS, get_symbol};
use crate::core::{Expr, ExprKind};
use std::f64::consts::PI;
use std::sync::Arc;

rule_arc!(
    SinZeroRule,
    "sin_zero",
    95,
    Trigonometric,
    &[RuleExprKind::Function],
    targets: &[KS.sin],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.sin
            && args.len() == 1
            && {
                // Exact check for sin(0.0)
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 0.0")]
                let is_zero = matches!(&args[0].kind, ExprKind::Number(n) if *n == 0.0);
                is_zero
            }
        {
            return Some(Arc::new(Expr::number(0.0)));
        }
        None
    }
);

rule_arc!(
    CosZeroRule,
    "cos_zero",
    95,
    Trigonometric,
    &[RuleExprKind::Function],
    targets: &[KS.cos],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.cos
            && args.len() == 1
            && matches!(&args[0].kind, ExprKind::Number(n) if *n == 0.0)
        {
            return Some(Arc::new(Expr::number(1.0)));
        }
        None
    }
);

rule_arc!(
    TanZeroRule,
    "tan_zero",
    95,
    Trigonometric,
    &[RuleExprKind::Function],
    targets: &[KS.tan],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.tan
            && args.len() == 1
            && matches!(&args[0].kind, ExprKind::Number(n) if *n == 0.0)
        {
            return Some(Arc::new(Expr::number(0.0)));
        }
        None
    }
);

rule_arc!(
    SinPiRule,
    "sin_pi",
    95,
    Trigonometric,
    &[RuleExprKind::Function],
    targets: &[KS.sin],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.sin
            && args.len() == 1
            && is_pi(&args[0])
        {
            return Some(Arc::new(Expr::number(0.0)));
        }
        None
    }
);

rule_arc!(
    CosPiRule,
    "cos_pi",
    95,
    Trigonometric,
    &[RuleExprKind::Function],
    targets: &[KS.cos],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.cos
            && args.len() == 1
            && is_pi(&args[0])
        {
            return Some(Arc::new(Expr::number(-1.0)));
        }
        None
    }
);

rule_arc!(
    SinPiOverTwoRule,
    "sin_pi_over_two",
    95,
    Trigonometric,
    &[RuleExprKind::Function],
    targets: &[KS.sin],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.sin
            && args.len() == 1
            && let ExprKind::Div(num, den) = &args[0].kind
            && is_pi(num)
        {
            // Exact check for pi/2 denominator
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
            let is_two = matches!(&den.kind, ExprKind::Number(n) if *n == 2.0);
            if is_two {
                return Some(Arc::new(Expr::number(1.0)));
            }
        }
        None
    }
);

rule_arc!(
    CosPiOverTwoRule,
    "cos_pi_over_two",
    95,
    Trigonometric,
    &[RuleExprKind::Function],
    targets: &[KS.cos],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.cos
            && args.len() == 1
            && let ExprKind::Div(num, den) = &args[0].kind
            && is_pi(num)
        {
            // Exact check for pi/2 denominator
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
            let is_two = matches!(&den.kind, ExprKind::Number(n) if *n == 2.0);
            if is_two {
                return Some(Arc::new(Expr::number(0.0)));
            }
        }
        None
    }
);

rule_arc!(
    TrigExactValuesRule,
    "trig_exact_values",
    95,
    Trigonometric,
    &[RuleExprKind::Function],
    targets: &[KS.sin, KS.cos, KS.tan],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::FunctionCall { name, args } = &expr.kind
            && args.len() == 1
        {
            let arg = &args[0];
            let arg_val = get_numeric_value(arg).unwrap_or(f64::NAN);
            let is_numeric_input = matches!(arg.kind, ExprKind::Number(_));

            match name {
                n if n.id() == KS.sin => {
                    let matches_pi_six = {
                        // Exact check for denominator 6.0 (PI/6)
                        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 6.0")]
                        let is_six = matches!(&arg.kind, ExprKind::Div(num, den) if is_pi(num) && matches!(&den.kind, ExprKind::Number(val) if *val == 6.0));
                        is_six
                    };
                    if approx_eq(arg_val, PI / 6.0) || matches_pi_six
                    {
                        return if is_numeric_input {
                            Some(Arc::new(Expr::number(0.5)))
                        } else {
                            Some(Arc::new(Expr::div_from_arcs(Arc::new(Expr::number(1.0)), Arc::new(Expr::number(2.0)))))
                        };
                    }
                    let matches_pi_four = {
                        // Exact check for denominator 4.0 (PI/4)
                        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 4.0")]
                        let is_four = matches!(&arg.kind, ExprKind::Div(num, den) if is_pi(num) && matches!(&den.kind, ExprKind::Number(val) if *val == 4.0));
                        is_four
                    };
                    if approx_eq(arg_val, PI / 4.0) || matches_pi_four
                    {
                        return if is_numeric_input {
                            Some(Arc::new(Expr::number((2.0_f64).sqrt() / 2.0)))
                        } else {
                            Some(Arc::new(Expr::div_from_arcs(
                                Arc::new(Expr::func_symbol(get_symbol(KS.sqrt), Expr::number(2.0))),
                                Arc::new(Expr::number(2.0)),
                            )))
                        };
                    }
                }
                n if n.id() == KS.cos => {
                    let matches_pi_three = {
                        // Exact check for denominator 3.0 (PI/3)
                        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 3.0")]
                        let is_three = matches!(&arg.kind, ExprKind::Div(num, den) if is_pi(num) && matches!(&den.kind, ExprKind::Number(val) if *val == 3.0));
                        is_three
                    };
                    if approx_eq(arg_val, PI / 3.0) || matches_pi_three
                    {
                        return if is_numeric_input {
                            Some(Arc::new(Expr::number(0.5)))
                        } else {
                            Some(Arc::new(Expr::div_from_arcs(Arc::new(Expr::number(1.0)), Arc::new(Expr::number(2.0)))))
                        };
                    }
                    let matches_pi_four = {
                        // Exact check for denominator 4.0 (PI/4)
                        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 4.0")]
                        let is_four = matches!(&arg.kind, ExprKind::Div(num, den) if is_pi(num) && matches!(&den.kind, ExprKind::Number(val) if *val == 4.0));
                        is_four
                    };
                    if approx_eq(arg_val, PI / 4.0) || matches_pi_four
                    {
                        return if is_numeric_input {
                            Some(Arc::new(Expr::number((2.0_f64).sqrt() / 2.0)))
                        } else {
                            Some(Arc::new(Expr::div_from_arcs(
                                Arc::new(Expr::func_symbol(get_symbol(KS.sqrt), Expr::number(2.0))),
                                Arc::new(Expr::number(2.0)),
                            )))
                        };
                    }
                }
                n if n.id() == KS.tan => {
                    let matches_pi_four = {
                        // Exact check for denominator 4.0 (PI/4)
                        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 4.0")]
                        let is_four = matches!(&arg.kind, ExprKind::Div(num, den) if is_pi(num) && matches!(&den.kind, ExprKind::Number(val) if *val == 4.0));
                        is_four
                    };
                    if approx_eq(arg_val, PI / 4.0) || matches_pi_four
                    {
                        return Some(Arc::new(Expr::number(1.0)));
                    }
                    let matches_pi_three = {
                        // Exact check for denominator 3.0 (PI/3)
                        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 3.0")]
                        let is_three = matches!(&arg.kind, ExprKind::Div(num, den) if is_pi(num) && matches!(&den.kind, ExprKind::Number(val) if *val == 3.0));
                        is_three
                    };
                    if approx_eq(arg_val, PI / 3.0) || matches_pi_three
                    {
                        return if is_numeric_input {
                            Some(Arc::new(Expr::number((3.0_f64).sqrt())))
                        } else {
                            Some(Arc::new(Expr::func_symbol(get_symbol(KS.sqrt), Expr::number(3.0))))
                        };
                    }
                    let matches_pi_six = {
                        // Exact check for denominator 6.0 (PI/6)
                        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 6.0")]
                        let is_six = matches!(&arg.kind, ExprKind::Div(num, den) if is_pi(num) && matches!(&den.kind, ExprKind::Number(val) if *val == 6.0));
                        is_six
                    };
                    if approx_eq(arg_val, PI / 6.0) || matches_pi_six
                    {
                        return if is_numeric_input {
                            Some(Arc::new(Expr::number(1.0 / (3.0_f64).sqrt())))
                        } else {
                            Some(Arc::new(Expr::div_from_arcs(
                                Arc::new(Expr::func_symbol(get_symbol(KS.sqrt), Expr::number(3.0))),
                                Arc::new(Expr::number(3.0)),
                            )))
                        };
                    }
                }
                _ => {}
            }
        }
        None
    }
);

// Ratio rules: convert 1/trig to reciprocal functions
rule_arc!(
    OneCosToSecRule,
    "one_cos_to_sec",
    85,
    Trigonometric,
    &[RuleExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Div(num, den) = &expr.kind
            && let ExprKind::Number(n) = &num.kind
            && (n - 1.0).abs() < EPSILON
        {
            // 1/cos(x) → sec(x)
            if let ExprKind::FunctionCall { name, args } = &den.kind
                && name.id() == KS.cos
                && args.len() == 1
            {
                // Efficiently constructing using Arc<Expr>
                return Some(Arc::new(Expr::func_multi_from_arcs_symbol(
                    get_symbol(KS.sec),
                    vec![Arc::clone(&args[0])],
                )));
            }
            // 1/cos(x)^n → sec(x)^n
            if let ExprKind::Pow(base, exp) = &den.kind
                && let ExprKind::FunctionCall { name, args } = &base.kind
                && name.id() == KS.cos
                && args.len() == 1
            {
                let sec_expr = Expr::func_multi_from_arcs_symbol(
                    get_symbol(KS.sec),
                    vec![Arc::clone(&args[0])],
                );
                return Some(Arc::new(Expr::pow_from_arcs(
                    Arc::new(sec_expr),
                    Arc::clone(exp),
                )));
            }
        }
        None
    }
);

rule_arc!(
    OneSinToCscRule,
    "one_sin_to_csc",
    85,
    Trigonometric,
    &[RuleExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Div(num, den) = &expr.kind
            && let ExprKind::Number(n) = &num.kind
            && (n - 1.0).abs() < EPSILON
        {
            // 1/sin(x) → csc(x)
            if let ExprKind::FunctionCall { name, args } = &den.kind
                && name.id() == KS.sin
                && args.len() == 1
            {
                return Some(Arc::new(Expr::func_multi_from_arcs_symbol(
                    get_symbol(KS.csc),
                    vec![Arc::clone(&args[0])],
                )));
            }
            // 1/sin(x)^n → csc(x)^n
            if let ExprKind::Pow(base, exp) = &den.kind
                && let ExprKind::FunctionCall { name, args } = &base.kind
                && name.id() == KS.sin
                && args.len() == 1
            {
                let csc_expr = Expr::func_multi_from_arcs_symbol(
                    get_symbol(KS.csc),
                    vec![Arc::clone(&args[0])],
                );
                return Some(Arc::new(Expr::pow_from_arcs(
                    Arc::new(csc_expr),
                    Arc::clone(exp),
                )));
            }
        }
        None
    }
);

rule_arc!(
    SinCosToTanRule,
    "sin_cos_to_tan",
    85,
    Trigonometric,
    &[RuleExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Div(num, den) = &expr.kind
            && let ExprKind::FunctionCall {
                name: num_name,
                args: num_args,
            } = &num.kind
            && let ExprKind::FunctionCall {
                name: den_name,
                args: den_args,
            } = &den.kind
            && num_name.id() == KS.sin
            && den_name.id() == KS.cos
            && num_args.len() == 1
            && den_args.len() == 1
            && num_args[0] == den_args[0]
        {
            return Some(Arc::new(Expr::func_multi_from_arcs_symbol(
                get_symbol(KS.tan),
                vec![Arc::clone(&num_args[0])],
            )));
        }
        None
    }
);

rule_arc!(
    CosSinToCotRule,
    "cos_sin_to_cot",
    85,
    Trigonometric,
    &[RuleExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let ExprKind::Div(num, den) = &expr.kind
            && let ExprKind::FunctionCall {
                name: num_name,
                args: num_args,
            } = &num.kind
            && let ExprKind::FunctionCall {
                name: den_name,
                args: den_args,
            } = &den.kind
            && num_name.id() == KS.cos
            && den_name.id() == KS.sin
            && num_args.len() == 1
            && den_args.len() == 1
            && num_args[0] == den_args[0]
        {
            return Some(Arc::new(Expr::func_multi_from_arcs_symbol(
                get_symbol(KS.cot),
                vec![Arc::clone(&num_args[0])],
            )));
        }
        None
    }
);
