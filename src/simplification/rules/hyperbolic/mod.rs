use crate::ast::Expr;
use crate::simplification::rules::{Rule, RuleCategory, RuleContext};
use std::rc::Rc;

/// Rule for sinh(0) = 0
pub struct SinhZeroRule;

impl Rule for SinhZeroRule {
    fn name(&self) -> &'static str {
        "sinh_zero"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "sinh" && args.len() == 1 {
                if matches!(args[0], Expr::Number(n) if n == 0.0) {
                    return Some(Expr::Number(0.0));
                }
            }
        }
        None
    }
}

/// Rule for cosh(0) = 1
pub struct CoshZeroRule;

impl Rule for CoshZeroRule {
    fn name(&self) -> &'static str {
        "cosh_zero"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "cosh" && args.len() == 1 {
                if matches!(args[0], Expr::Number(n) if n == 0.0) {
                    return Some(Expr::Number(1.0));
                }
            }
        }
        None
    }
}

/// Rule for converting (e^x - e^-x) / 2 to sinh(x)
pub struct SinhFromExpRule;

impl Rule for SinhFromExpRule {
    fn name(&self) -> &'static str {
        "sinh_from_exp"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr {
            if let Expr::Number(d) = **denominator {
                if d == 2.0 {
                    // Try both Add(e^x, -e^(-x)) and Sub(e^x, e^(-x)) patterns
                    let pattern_result = match &**numerator {
                        Expr::Add(u, v) => Self::match_sinh_pattern(u, v, true),
                        Expr::Sub(u, v) => Self::match_sinh_pattern(u, v, false),
                        _ => None,
                    };
                    
                    if let Some((pos_exp, neg_exp)) = pattern_result {
                        if Self::is_negation(&neg_exp, &pos_exp) {
                            return Some(Expr::FunctionCall {
                                name: "sinh".to_string(),
                                args: vec![pos_exp],
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

impl SinhFromExpRule {
    /// Match patterns for sinh: (e^x - e^(-x)) or (e^x + (-1)*e^(-x))
    /// is_add: true for Add node, false for Sub node
    /// Returns (pos_exp, neg_exp) if pattern matches
    fn match_sinh_pattern(u: &Expr, v: &Expr, is_add: bool) -> Option<(Expr, Expr)> {
        // For Add: e^x + (-1)*e^(-x)
        // For Sub: e^x - e^(-x)
        
        // Try: u = e^x, v = e^(-x) or -e^(-x)
        if let Expr::Pow(base1, exp1) = u {
            if let Expr::Symbol(b1) = &**base1 {
                if b1 == "e" {
                    let v_inner = if is_add {
                        // For Add, expect v = Mul(-1, e^(-x))
                        if let Expr::Mul(lhs, w) = v {
                            if let Expr::Number(n) = **lhs {
                                if n == -1.0 {
                                    Some(&**w)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        // For Sub, v is directly e^(-x)
                        Some(v)
                    };
                    
                    if let Some(v_expr) = v_inner {
                        if let Expr::Pow(base2, exp2) = v_expr {
                            if let Expr::Symbol(b2) = &**base2 {
                                if b2 == "e" {
                                    return Some((*exp1.clone(), *exp2.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Try: u = -e^(-x), v = e^x (for Add only)
        if is_add {
            if let Expr::Mul(lhs, w) = u {
                if let Expr::Number(n) = **lhs {
                    if n == -1.0 {
                        if let Expr::Pow(base1, exp2) = &**w {
                            if let Expr::Symbol(b1) = &**base1 {
                                if b1 == "e" {
                                    if let Expr::Pow(base2, exp1) = v {
                                        if let Expr::Symbol(b2) = &**base2 {
                                            if b2 == "e" {
                                                return Some((*exp1.clone(), *exp2.clone()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }
    
    fn is_negation(expr: &Expr, other: &Expr) -> bool {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs {
                if n == -1.0 && **rhs == *other {
                    return true;
                }
            }
            if let Expr::Number(n) = **rhs {
                if n == -1.0 && **lhs == *other {
                    return true;
                }
            }
        }
        false
    }
}

/// Rule for converting (e^x + e^-x) / 2 to cosh(x)
pub struct CoshFromExpRule;

impl Rule for CoshFromExpRule {
    fn name(&self) -> &'static str {
        "cosh_from_exp"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr {
            if let Expr::Number(d) = **denominator {
                if d == 2.0 {
                    if let Expr::Add(u, v) = &**numerator {
                        // Check both orderings: (e^x + e^(-x)) and (e^(-x) + e^x)
                        if let (Expr::Pow(base1, exp1), Expr::Pow(base2, exp2)) = (&**u, &**v) {
                            if let (Expr::Symbol(b1), Expr::Symbol(b2)) = (&**base1, &**base2) {
                                if b1 == "e" && b2 == "e" {
                                    // Check if exp2 is -exp1 (u = e^x, v = e^(-x))
                                    if Self::is_negation(exp2, exp1) {
                                        return Some(Expr::FunctionCall {
                                            name: "cosh".to_string(),
                                            args: vec![*exp1.clone()],
                                        });
                                    }
                                    // Check if exp1 is -exp2 (u = e^(-x), v = e^x) - commutative
                                    if Self::is_negation(exp1, exp2) {
                                        return Some(Expr::FunctionCall {
                                            name: "cosh".to_string(),
                                            args: vec![*exp2.clone()],
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

impl CoshFromExpRule {
    fn is_negation(expr: &Expr, other: &Expr) -> bool {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs {
                if n == -1.0 && **rhs == *other {
                    return true;
                }
            }
            if let Expr::Number(n) = **rhs {
                if n == -1.0 && **lhs == *other {
                    return true;
                }
            }
        }
        false
    }
}

/// Rule for converting (e^x - e^-x) / (e^x + e^-x) to tanh(x)
pub struct TanhFromExpRule;

impl Rule for TanhFromExpRule {
    fn name(&self) -> &'static str {
        "tanh_from_exp"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr {
            // Handle both Add and Sub in numerator and denominator
            let (num_u, num_v, num_is_add) = match &**numerator {
                Expr::Add(u, v) => (u, v, true),
                Expr::Sub(u, v) => (u, v, false),
                _ => return None,
            };
            
            let (den_u, den_v, _den_is_add) = match &**denominator {
                Expr::Add(u, v) => (u, v, true),
                Expr::Sub(u, v) => (u, v, false),
                _ => return None,
            };
            
            // Match numerator pattern
            let (num_pos, num_neg) = if let Some(result) = Self::match_sinh_pattern(num_u, num_v, num_is_add) {
                result
            } else {
                return None;
            };
            
            // Match denominator pattern (cosh: e^x + e^(-x))
            // Check both orderings for commutative Add
            if let (Expr::Pow(dbase1, dexp1), Expr::Pow(dbase2, dexp2)) = (&**den_u, &**den_v) {
                if let (Expr::Symbol(db1), Expr::Symbol(db2)) = (&**dbase1, &**dbase2) {
                    if db1 == "e" && db2 == "e" {
                        // Case 1: den_u = e^x, den_v = e^(-x)
                        if Self::is_negation(dexp2, dexp1) && **dexp1 == num_pos && num_neg == **dexp2 {
                            return Some(Expr::FunctionCall {
                                name: "tanh".to_string(),
                                args: vec![num_pos],
                            });
                        }
                        // Case 2: den_u = e^(-x), den_v = e^x (commutative)
                        if Self::is_negation(dexp1, dexp2) && **dexp2 == num_pos && num_neg == **dexp1 {
                            return Some(Expr::FunctionCall {
                                name: "tanh".to_string(),
                                args: vec![num_pos],
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

impl TanhFromExpRule {
    /// Match patterns for sinh: (e^x - e^(-x)) or (e^x + (-1)*e^(-x))
    fn match_sinh_pattern(u: &Expr, v: &Expr, is_add: bool) -> Option<(Expr, Expr)> {
        // For Add: e^x + (-1)*e^(-x)
        // For Sub: e^x - e^(-x)
        
        if let Expr::Pow(base1, exp1) = u {
            if let Expr::Symbol(b1) = &**base1 {
                if b1 == "e" {
                    let v_inner = if is_add {
                        if let Expr::Mul(lhs, w) = v {
                            if let Expr::Number(n) = **lhs {
                                if n == -1.0 {
                                    Some(&**w)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        Some(v)
                    };
                    
                    if let Some(v_expr) = v_inner {
                        if let Expr::Pow(base2, exp2) = v_expr {
                            if let Expr::Symbol(b2) = &**base2 {
                                if b2 == "e" {
                                    return Some((*exp1.clone(), *exp2.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if is_add {
            if let Expr::Mul(lhs, w) = u {
                if let Expr::Number(n) = **lhs {
                    if n == -1.0 {
                        if let Expr::Pow(base1, exp2) = &**w {
                            if let Expr::Symbol(b1) = &**base1 {
                                if b1 == "e" {
                                    if let Expr::Pow(base2, exp1) = v {
                                        if let Expr::Symbol(b2) = &**base2 {
                                            if b2 == "e" {
                                                return Some((*exp1.clone(), *exp2.clone()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }
    
    fn is_negation(expr: &Expr, other: &Expr) -> bool {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs {
                if n == -1.0 && **rhs == *other {
                    return true;
                }
            }
            if let Expr::Number(n) = **rhs {
                if n == -1.0 && **lhs == *other {
                    return true;
                }
            }
        }
        false
    }
}

/// Rule for converting 2 / (e^x + e^-x) to sech(x)
pub struct SechFromExpRule;

impl Rule for SechFromExpRule {
    fn name(&self) -> &'static str {
        "sech_from_exp"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr {
            if let Expr::Number(n) = **numerator {
                if n == 2.0 {
                    if let Expr::Add(u, v) = &**denominator {
                        // Check both orderings: (e^x + e^(-x)) and (e^(-x) + e^x)
                        if let (Expr::Pow(base1, exp1), Expr::Pow(base2, exp2)) = (&**u, &**v) {
                            if let (Expr::Symbol(b1), Expr::Symbol(b2)) = (&**base1, &**base2) {
                                if b1 == "e" && b2 == "e" {
                                    // Case 1: u = e^x, v = e^(-x)
                                    if Self::is_negation(exp2, exp1) {
                                        return Some(Expr::FunctionCall {
                                            name: "sech".to_string(),
                                            args: vec![*exp1.clone()],
                                        });
                                    }
                                    // Case 2: u = e^(-x), v = e^x (commutative)
                                    if Self::is_negation(exp1, exp2) {
                                        return Some(Expr::FunctionCall {
                                            name: "sech".to_string(),
                                            args: vec![*exp2.clone()],
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

impl SechFromExpRule {
    fn is_negation(expr: &Expr, other: &Expr) -> bool {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs {
                if n == -1.0 && **rhs == *other {
                    return true;
                }
            }
            if let Expr::Number(n) = **rhs {
                if n == -1.0 && **lhs == *other {
                    return true;
                }
            }
        }
        false
    }
}

/// Rule for converting 2 / (e^x - e^-x) to csch(x)
pub struct CschFromExpRule;

impl Rule for CschFromExpRule {
    fn name(&self) -> &'static str {
        "csch_from_exp"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr {
            if let Expr::Number(n) = **numerator {
                if n == 2.0 {
                    // Handle both Add and Sub patterns
                    let pattern_result = match &**denominator {
                        Expr::Add(u, v) => SinhFromExpRule::match_sinh_pattern(u, v, true),
                        Expr::Sub(u, v) => SinhFromExpRule::match_sinh_pattern(u, v, false),
                        _ => None,
                    };
                    
                    if let Some((pos_exp, neg_exp)) = pattern_result {
                        if Self::is_negation(&neg_exp, &pos_exp) {
                            return Some(Expr::FunctionCall {
                                name: "csch".to_string(),
                                args: vec![pos_exp],
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

impl CschFromExpRule {
    fn is_negation(expr: &Expr, other: &Expr) -> bool {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs {
                if n == -1.0 && **rhs == *other {
                    return true;
                }
            }
            if let Expr::Number(n) = **rhs {
                if n == -1.0 && **lhs == *other {
                    return true;
                }
            }
        }
        false
    }
}

/// Rule for converting (e^x + e^-x) / (e^x - e^-x) to coth(x)
pub struct CothFromExpRule;

impl Rule for CothFromExpRule {
    fn name(&self) -> &'static str {
        "coth_from_exp"
    }

    fn priority(&self) -> i32 {
        80
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr {
            // Handle both Add and Sub in numerator and denominator
            let (num_u, num_v, _num_is_add) = match &**numerator {
                Expr::Add(u, v) => (u, v, true),
                Expr::Sub(u, v) => (u, v, false),
                _ => return None,
            };
            
            let (den_u, den_v, den_is_add) = match &**denominator {
                Expr::Add(u, v) => (u, v, true),
                Expr::Sub(u, v) => (u, v, false),
                _ => return None,
            };
            
            // Match denominator pattern (sinh pattern)
            let (den_pos, den_neg) = if let Some(result) = TanhFromExpRule::match_sinh_pattern(den_u, den_v, den_is_add) {
                result
            } else {
                return None;
            };
            
            // Check numerator (cosh pattern: e^x + e^(-x))
            // Try both orderings
            if let (Expr::Pow(nbase1, nexp1), Expr::Pow(nbase2, nexp2)) = (&**num_u, &**num_v) {
                if let (Expr::Symbol(nb1), Expr::Symbol(nb2)) = (&**nbase1, &**nbase2) {
                    if nb1 == "e" && nb2 == "e" {
                        // Case 1: num_u = e^x, num_v = e^(-x)
                        if Self::is_negation(nexp2, nexp1) && Self::is_negation(&den_neg, &den_pos) && **nexp1 == den_pos && **nexp2 == den_neg {
                            return Some(Expr::FunctionCall {
                                name: "coth".to_string(),
                                args: vec![den_pos],
                            });
                        }
                        // Case 2: num_u = e^(-x), num_v = e^x (commutative)
                        if Self::is_negation(nexp1, nexp2) && Self::is_negation(&den_neg, &den_pos) && **nexp2 == den_pos && **nexp1 == den_neg {
                            return Some(Expr::FunctionCall {
                                name: "coth".to_string(),
                                args: vec![den_pos],
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

impl CothFromExpRule {
    fn is_negation(expr: &Expr, other: &Expr) -> bool {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs {
                if n == -1.0 && **rhs == *other {
                    return true;
                }
            }
            if let Expr::Number(n) = **rhs {
                if n == -1.0 && **lhs == *other {
                    return true;
                }
            }
        }
        false
    }
}

/// Rule for cosh^2(x) - sinh^2(x) = 1
pub struct HyperbolicIdentityRule;

impl Rule for HyperbolicIdentityRule {
    fn name(&self) -> &'static str {
        "hyperbolic_identity"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        // Check for cosh^2(x) - sinh^2(x)
        if let Expr::Sub(u, v) = expr {
            if let (Some((name1, arg1)), Some((name2, arg2))) = (
                Self::get_hyperbolic_power(u, 2.0),
                Self::get_hyperbolic_power(v, 2.0),
            ) {
                if arg1 == arg2 && name1 == "cosh" && name2 == "sinh" {
                    return Some(Expr::Number(1.0));
                }
            }
        }

        // Check for cosh^2(x) + (-1 * sinh^2(x))
        if let Expr::Add(u, v) = expr {
            // Check u = cosh^2, v = -sinh^2
            if let Some((name1, arg1)) = Self::get_hyperbolic_power(u, 2.0) {
                if name1 == "cosh" {
                    // Check v
                    if let Expr::Mul(lhs, rhs) = &**v {
                        if let Expr::Number(n) = **lhs {
                            if n == -1.0 {
                                if let Some((name2, arg2)) = Self::get_hyperbolic_power(rhs, 2.0) {
                                    if name2 == "sinh" && arg1 == arg2 {
                                        return Some(Expr::Number(1.0));
                                    }
                                }
                            }
                        }
                        // Check rhs is -1 (commutative)
                        if let Expr::Number(n) = **rhs {
                            if n == -1.0 {
                                if let Some((name2, arg2)) = Self::get_hyperbolic_power(lhs, 2.0) {
                                    if name2 == "sinh" && arg1 == arg2 {
                                        return Some(Expr::Number(1.0));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Check v = cosh^2, u = -sinh^2 (commutative add)
            if let Some((name1, arg1)) = Self::get_hyperbolic_power(v, 2.0) {
                if name1 == "cosh" {
                    // Check u
                    if let Expr::Mul(lhs, rhs) = &**u {
                        if let Expr::Number(n) = **lhs {
                            if n == -1.0 {
                                if let Some((name2, arg2)) = Self::get_hyperbolic_power(rhs, 2.0) {
                                    if name2 == "sinh" && arg1 == arg2 {
                                        return Some(Expr::Number(1.0));
                                    }
                                }
                            }
                        }
                        // Check rhs is -1
                        if let Expr::Number(n) = **rhs {
                            if n == -1.0 {
                                if let Some((name2, arg2)) = Self::get_hyperbolic_power(lhs, 2.0) {
                                    if name2 == "sinh" && arg1 == arg2 {
                                        return Some(Expr::Number(1.0));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for 1 - tanh^2(x) = sech^2(x)
        if let Expr::Sub(u, v) = expr {
            if let Expr::Number(n) = **u {
                if n == 1.0 {
                    if let Some((name, arg)) = Self::get_hyperbolic_power(v, 2.0) {
                        if name == "tanh" {
                            return Some(Expr::Pow(
                                Box::new(Expr::FunctionCall {
                                    name: "sech".to_string(),
                                    args: vec![arg],
                                }),
                                Box::new(Expr::Number(2.0)),
                            ));
                        }
                    }
                }
            }
        }

        // Check for 1 + (-1 * tanh^2(x)) = sech^2(x) (normalized form)
        if let Expr::Add(u, v) = expr {
            if let Expr::Number(n) = **u {
                if n == 1.0 {
                    if let Expr::Mul(lhs, rhs) = &**v {
                        if let Expr::Number(nn) = **lhs {
                            if nn == -1.0 {
                                if let Some((name, arg)) = Self::get_hyperbolic_power(rhs, 2.0) {
                                    if name == "tanh" {
                                        return Some(Expr::Pow(
                                            Box::new(Expr::FunctionCall {
                                                name: "sech".to_string(),
                                                args: vec![arg],
                                            }),
                                            Box::new(Expr::Number(2.0)),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for tanh^2(x) + (-1) = -sech^2(x) or commutative
        if let Expr::Add(u, v) = expr {
            if let Some((name, arg)) = Self::get_hyperbolic_power(u, 2.0) {
                if name == "tanh" {
                    if let Expr::Mul(lhs, rhs) = &**v {
                        if let Expr::Number(n) = **lhs {
                            if n == -1.0 && **rhs == Expr::Number(1.0) {
                                return Some(Expr::Mul(
                                    Box::new(Expr::Number(-1.0)),
                                    Box::new(Expr::Pow(
                                        Box::new(Expr::FunctionCall {
                                            name: "sech".to_string(),
                                            args: vec![arg],
                                        }),
                                        Box::new(Expr::Number(2.0)),
                                    )),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Check for coth^2(x) - 1 = csch^2(x)
        if let Expr::Sub(u, v) = expr {
            if let Expr::Number(n) = **v {
                if n == 1.0 {
                    if let Some((name, arg)) = Self::get_hyperbolic_power(u, 2.0) {
                        if name == "coth" {
                            return Some(Expr::Pow(
                                Box::new(Expr::FunctionCall {
                                    name: "csch".to_string(),
                                    args: vec![arg],
                                }),
                                Box::new(Expr::Number(2.0)),
                            ));
                        }
                    }
                }
            }
        }

        // Check for coth^2(x) + (-1) = csch^2(x) (normalized form)
        if let Expr::Add(u, v) = expr {
            if let Some((name, arg)) = Self::get_hyperbolic_power(u, 2.0) {
                if name == "coth" {
                    if let Expr::Number(n) = **v {
                        if n == -1.0 {
                            return Some(Expr::Pow(
                                Box::new(Expr::FunctionCall {
                                    name: "csch".to_string(),
                                    args: vec![arg],
                                }),
                                Box::new(Expr::Number(2.0)),
                            ));
                        }
                    }
                }
            }
        }

        // Check for (cosh(x) - sinh(x)) * (cosh(x) + sinh(x)) = 1
        if let Expr::Mul(u, v) = expr {
            if let (Some(arg1), Some(arg2)) = (
                Self::is_cosh_minus_sinh_term(u),
                Self::is_cosh_plus_sinh_term(v),
            ) {
                if arg1 == arg2 {
                    return Some(Expr::Number(1.0));
                }
            }
            if let (Some(arg1), Some(arg2)) = (
                Self::is_cosh_minus_sinh_term(v),
                Self::is_cosh_plus_sinh_term(u),
            ) {
                if arg1 == arg2 {
                    return Some(Expr::Number(1.0));
                }
            }
        }

        None
    }
}

impl HyperbolicIdentityRule {
    fn get_hyperbolic_power(expr: &Expr, power: f64) -> Option<(&str, Expr)> {
        if let Expr::Pow(base, exp) = expr {
            if let Expr::Number(p) = **exp {
                if p == power {
                    if let Expr::FunctionCall { name, args } = &**base {
                        if args.len() == 1 && (name == "sinh" || name == "cosh" || name == "tanh" || name == "coth") {
                            return Some((name, args[0].clone()));
                        }
                    }
                }
            }
        }
        None
    }

    fn is_cosh_minus_sinh_term(expr: &Expr) -> Option<Expr> {
        // Check for cosh - sinh: Sub(cosh, sinh)
        if let Expr::Sub(u, v) = expr {
            if let Expr::FunctionCall { name: n1, args: a1 } = &**u {
                if n1 == "cosh" && a1.len() == 1 {
                    if let Expr::FunctionCall { name: n2, args: a2 } = &**v {
                        if n2 == "sinh" && a2.len() == 1 && a1[0] == a2[0] {
                            return Some(a1[0].clone());
                        }
                    }
                }
            }
        }
        // Check for normalized cosh - sinh: Add(cosh, Mul(-1, sinh))
        if let Expr::Add(u, v) = expr {
            if let Expr::FunctionCall { name: n1, args: a1 } = &**u {
                if n1 == "cosh" && a1.len() == 1 {
                    if let Expr::Mul(lhs, rhs) = &**v {
                        if let Expr::Number(n) = **lhs {
                            if n == -1.0 {
                                if let Expr::FunctionCall { name: n2, args: a2 } = &**rhs {
                                    if n2 == "sinh" && a2.len() == 1 && a1[0] == a2[0] {
                                        return Some(a1[0].clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn is_cosh_plus_sinh_term(expr: &Expr) -> Option<Expr> {
        // Check for cosh + sinh: Add(cosh, sinh)
        if let Expr::Add(u, v) = expr {
            if let Expr::FunctionCall { name: n1, args: a1 } = &**u {
                if n1 == "cosh" && a1.len() == 1 {
                    if let Expr::FunctionCall { name: n2, args: a2 } = &**v {
                        if n2 == "sinh" && a2.len() == 1 && a1[0] == a2[0] {
                            return Some(a1[0].clone());
                        }
                    }
                }
            }
        }
        None
    }
}

/// Rule for sinh(-x) = -sinh(x)
pub struct SinhNegationRule;

impl Rule for SinhNegationRule {
    fn name(&self) -> &'static str {
        "sinh_negation"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "sinh" && args.len() == 1 {
                if Self::is_negation(&args[0]) {
                    let inner = Self::negate_arg(&args[0]);
                    return Some(Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::FunctionCall {
                            name: "sinh".to_string(),
                            args: vec![inner],
                        }),
                    ));
                }
            }
        }
        None
    }
}

impl SinhNegationRule {
    fn is_negation(expr: &Expr) -> bool {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs {
                if n == -1.0 {
                    return true;
                }
            }
            if let Expr::Number(n) = **rhs {
                if n == -1.0 {
                    return true;
                }
            }
        }
        false
    }

    fn negate_arg(expr: &Expr) -> Expr {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs {
                if n == -1.0 {
                    return *rhs.clone();
                }
            }
            if let Expr::Number(n) = **rhs {
                if n == -1.0 {
                    return *lhs.clone();
                }
            }
        }
        expr.clone() // fallback, shouldn't happen
    }
}

/// Rule for cosh(-x) = cosh(x)
pub struct CoshNegationRule;

impl Rule for CoshNegationRule {
    fn name(&self) -> &'static str {
        "cosh_negation"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "cosh" && args.len() == 1 {
                if let Expr::Mul(lhs, rhs) = &args[0] {
                    if let Expr::Number(n) = **lhs {
                        if n == -1.0 {
                            return Some(Expr::FunctionCall {
                                name: "cosh".to_string(),
                                args: vec![*rhs.clone()],
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

/// Rule for tanh(-x) = -tanh(x)
pub struct TanhNegationRule;

impl Rule for TanhNegationRule {
    fn name(&self) -> &'static str {
        "tanh_negation"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "tanh" && args.len() == 1 {
                if Self::is_negation(&args[0]) {
                    let inner = Self::negate_arg(&args[0]);
                    return Some(Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(Expr::FunctionCall {
                            name: "tanh".to_string(),
                            args: vec![inner],
                        }),
                    ));
                }
            }
        }
        None
    }
}

impl TanhNegationRule {
    fn is_negation(expr: &Expr) -> bool {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs {
                if n == -1.0 {
                    return true;
                }
            }
            if let Expr::Number(n) = **rhs {
                if n == -1.0 {
                    return true;
                }
            }
        }
        false
    }

    fn negate_arg(expr: &Expr) -> Expr {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs {
                if n == -1.0 {
                    return *rhs.clone();
                }
            }
            if let Expr::Number(n) = **rhs {
                if n == -1.0 {
                    return *lhs.clone();
                }
            }
        }
        expr.clone() // fallback
    }
}

/// Rule for sinh(asinh(x)) = x
pub struct SinhAsinhIdentityRule;

impl Rule for SinhAsinhIdentityRule {
    fn name(&self) -> &'static str {
        "sinh_asinh_identity"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "sinh" && args.len() == 1 {
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = &args[0]
                {
                    if inner_name == "asinh" && inner_args.len() == 1 {
                        return Some(inner_args[0].clone());
                    }
                }
            }
        }
        None
    }
}

/// Rule for cosh(acosh(x)) = x
pub struct CoshAcoshIdentityRule;

impl Rule for CoshAcoshIdentityRule {
    fn name(&self) -> &'static str {
        "cosh_acosh_identity"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "cosh" && args.len() == 1 {
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = &args[0]
                {
                    if inner_name == "acosh" && inner_args.len() == 1 {
                        return Some(inner_args[0].clone());
                    }
                }
            }
        }
        None
    }
}

/// Rule for tanh(atanh(x)) = x
pub struct TanhAtanhIdentityRule;

impl Rule for TanhAtanhIdentityRule {
    fn name(&self) -> &'static str {
        "tanh_atanh_identity"
    }

    fn priority(&self) -> i32 {
        95
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr {
            if name == "tanh" && args.len() == 1 {
                if let Expr::FunctionCall {
                    name: inner_name,
                    args: inner_args,
                } = &args[0]
                {
                    if inner_name == "atanh" && inner_args.len() == 1 {
                        return Some(inner_args[0].clone());
                    }
                }
            }
        }
        None
    }
}

/// Rule for triple angle folding: 4sinh^3(x) + 3sinh(x) -> sinh(3x), 4cosh^3(x) - 3cosh(x) -> cosh(3x)
pub struct HyperbolicTripleAngleRule;

impl Rule for HyperbolicTripleAngleRule {
    fn name(&self) -> &'static str {
        "hyperbolic_triple_angle"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        match expr {
            Expr::Add(u, v) => {
                // Check for 4sinh^3(x) + 3sinh(x)
                if let Some(arg) = Self::match_triple_sinh(u, v) {
                    return Some(Expr::FunctionCall {
                        name: "sinh".to_string(),
                        args: vec![Expr::Mul(Box::new(Expr::Number(3.0)), Box::new(arg))],
                    });
                }
            }
            Expr::Sub(u, v) => {
                // Check for 4cosh^3(x) - 3cosh(x)
                if let Some(arg) = Self::match_triple_cosh(u, v) {
                    return Some(Expr::FunctionCall {
                        name: "cosh".to_string(),
                        args: vec![Expr::Mul(Box::new(Expr::Number(3.0)), Box::new(arg))],
                    });
                }
            }
            _ => {}
        }
        None
    }
}

impl HyperbolicTripleAngleRule {
    fn match_triple_sinh(u: &Expr, v: &Expr) -> Option<Expr> {
        // We need to match 4*sinh(x)^3 + 3*sinh(x) (commutative)
        if let (Some((c1, arg1, p1)), Some((c2, arg2, p2))) = (
            Self::parse_fn_term(u, "sinh"),
            Self::parse_fn_term(v, "sinh"),
        ) {
            if arg1 == arg2 {
                // Allow small floating point tolerance
                let eps = 1e-10;
                if ((c1 == 4.0 || (c1 - 4.0).abs() < eps) && p1 == 3.0) && (c2 == 3.0 && p2 == 1.0)
                    || ((c2 == 4.0 || (c2 - 4.0).abs() < eps) && p2 == 3.0) && (c1 == 3.0 && p1 == 1.0)
                {
                    return Some(arg1);
                }
            }
        }
        None
    }

    fn match_triple_cosh(u: &Expr, v: &Expr) -> Option<Expr> {
        // We need to match 4*cosh(x)^3 - 3*cosh(x)
        // u - v, so u must be 4*cosh^3 and v must be 3*cosh
        if let (Some((c1, arg1, p1)), Some((c2, arg2, p2))) = (
            Self::parse_fn_term(u, "cosh"),
            Self::parse_fn_term(v, "cosh"),
        ) {
            if arg1 == arg2 {
                // Allow small floating point tolerance
                let eps = 1e-10;
                if (c1 == 4.0 || (c1 - 4.0).abs() < eps) && p1 == 3.0 && c2 == 3.0 && p2 == 1.0 {
                    return Some(arg1);
                }
            }
        }
        None
    }

    // Helper to parse c * func(arg)^p
    fn parse_fn_term(expr: &Expr, func_name: &str) -> Option<(f64, Expr, f64)> {
        // Case 1: func(arg)  -> c=1, p=1
        if let Expr::FunctionCall { name, args } = expr {
            if name == func_name && args.len() == 1 {
                return Some((1.0, args[0].clone(), 1.0));
            }
        }
        // Case 2: c * func(arg) -> p=1
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(c) = **lhs {
                if let Expr::FunctionCall { name, args } = &**rhs {
                    if name == func_name && args.len() == 1 {
                        return Some((c, args[0].clone(), 1.0));
                    }
                }
            }
        }
        // Case 3: func(arg)^p -> c=1
        if let Expr::Pow(base, exp) = expr {
            if let Expr::Number(p) = **exp {
                if let Expr::FunctionCall { name, args } = &**base {
                    if name == func_name && args.len() == 1 {
                        return Some((1.0, args[0].clone(), p));
                    }
                }
            }
        }
        // Case 4: c * func(arg)^p
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(c) = **lhs {
                if let Expr::Pow(base, exp) = &**rhs {
                    if let Expr::Number(p) = **exp {
                        if let Expr::FunctionCall { name, args } = &**base {
                            if name == func_name && args.len() == 1 {
                                return Some((c, args[0].clone(), p));
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

/// Get all hyperbolic rules in priority order
pub fn get_hyperbolic_rules() -> Vec<Rc<dyn Rule>> {
    vec![
        // High priority rules first
        Rc::new(SinhZeroRule),
        Rc::new(CoshZeroRule),
        Rc::new(SinhAsinhIdentityRule),
        Rc::new(CoshAcoshIdentityRule),
        Rc::new(TanhAtanhIdentityRule),
        Rc::new(SinhNegationRule),
        Rc::new(CoshNegationRule),
        Rc::new(TanhNegationRule),
        // Identity rules
        Rc::new(HyperbolicIdentityRule),
        // Conversion from exponential forms
        Rc::new(SinhFromExpRule),
        Rc::new(CoshFromExpRule),
        Rc::new(TanhFromExpRule),
        Rc::new(SechFromExpRule),
        Rc::new(CschFromExpRule),
        Rc::new(CothFromExpRule),
        Rc::new(HyperbolicTripleAngleRule),
    ]
}
