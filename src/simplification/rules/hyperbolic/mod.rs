use crate::ast::Expr;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use std::rc::Rc;

// ============================================================================
// SHARED HELPER FUNCTIONS FOR HYPERBOLIC PATTERN MATCHING
// ============================================================================

/// Represents an exponential term with its argument
/// e^x has argument x, e^(-x) has argument -x, 1/e^x has argument -x
#[derive(Debug, Clone)]
struct ExpTerm {
    arg: Expr,
}

impl ExpTerm {
    /// Try to extract an exponential term from various forms:
    /// - e^x -> ExpTerm { arg: x }
    /// - exp(x) -> ExpTerm { arg: x }  
    /// - 1/e^x -> ExpTerm { arg: -x }
    /// - 1/exp(x) -> ExpTerm { arg: -x }
    fn from_expr(expr: &Expr) -> Option<Self> {
        // Direct form: e^x or exp(x)
        if let Some(arg) = Self::get_direct_exp_arg(expr) {
            return Some(ExpTerm { arg });
        }

        // Reciprocal form: 1/e^x or 1/exp(x)
        if let Expr::Div(num, den) = expr
            && let Expr::Number(n) = &**num
            && *n == 1.0
            && let Some(arg) = Self::get_direct_exp_arg(den)
        {
            // 1/e^x = e^(-x)
            return Some(ExpTerm {
                arg: Self::negate(&arg),
            });
        }

        None
    }

    /// Get the argument from e^x or exp(x) directly (not handling 1/e^x)
    fn get_direct_exp_arg(expr: &Expr) -> Option<Expr> {
        match expr {
            Expr::Pow(base, exp) => {
                if let Expr::Symbol(b) = &**base
                    && b == "e"
                {
                    return Some((**exp).clone());
                }
                None
            }
            Expr::FunctionCall { name, args } => {
                if name == "exp" && args.len() == 1 {
                    return Some(args[0].clone());
                }
                None
            }
            _ => None,
        }
    }

    /// Check if this argument is the negation of another
    fn is_negation_of(&self, other: &Expr) -> bool {
        Self::args_are_negations(&self.arg, other)
    }

    /// Check if two arguments are negations of each other: arg1 = -arg2
    fn args_are_negations(arg1: &Expr, arg2: &Expr) -> bool {
        // Check if arg1 = -1 * arg2
        if let Expr::Mul(lhs, rhs) = arg1 {
            if let Expr::Number(n) = &**lhs
                && *n == -1.0
                && **rhs == *arg2
            {
                return true;
            }
            if let Expr::Number(n) = &**rhs
                && *n == -1.0
                && **lhs == *arg2
            {
                return true;
            }
        }
        // Check if arg2 = -1 * arg1
        if let Expr::Mul(lhs, rhs) = arg2 {
            if let Expr::Number(n) = &**lhs
                && *n == -1.0
                && **rhs == *arg1
            {
                return true;
            }
            if let Expr::Number(n) = &**rhs
                && *n == -1.0
                && **lhs == *arg1
            {
                return true;
            }
        }
        false
    }

    /// Create the negation of an expression: x -> -1 * x
    fn negate(expr: &Expr) -> Expr {
        // If it's already a negation, return the inner part
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = &**lhs
                && *n == -1.0
            {
                return (**rhs).clone();
            }
            if let Expr::Number(n) = &**rhs
                && *n == -1.0
            {
                return (**lhs).clone();
            }
        }
        Expr::Mul(Box::new(Expr::Number(-1.0)), Box::new(expr.clone()))
    }
}

/// Try to match the pattern (e^x + e^(-x)) for cosh detection
/// Returns Some(x) if pattern matches (always returns the positive argument)
fn match_cosh_pattern(u: &Expr, v: &Expr) -> Option<Expr> {
    let exp1 = ExpTerm::from_expr(u)?;
    let exp2 = ExpTerm::from_expr(v)?;

    // Check if exp2.arg = -exp1.arg, return the positive one
    if exp2.is_negation_of(&exp1.arg) {
        // exp1.arg is positive if exp2.arg is its negation
        return Some(get_positive_form(&exp1.arg));
    }
    // Check reverse: exp1.arg = -exp2.arg
    if exp1.is_negation_of(&exp2.arg) {
        return Some(get_positive_form(&exp2.arg));
    }
    None
}

/// Try to match the pattern (e^x - e^(-x)) for sinh detection
/// Returns Some(x) if pattern matches (always returns the positive argument)
fn match_sinh_pattern_sub(u: &Expr, v: &Expr) -> Option<Expr> {
    // u should be e^x, v should be e^(-x)
    let exp1 = ExpTerm::from_expr(u)?;
    let exp2 = ExpTerm::from_expr(v)?;

    // Check if exp2.arg = -exp1.arg (so u = e^x, v = e^(-x))
    if exp2.is_negation_of(&exp1.arg) {
        return Some(get_positive_form(&exp1.arg));
    }
    None
}

/// Get the positive form of an expression
/// If expr is -x (i.e., Mul(-1, x)), return x
/// Otherwise return expr as-is
fn get_positive_form(expr: &Expr) -> Expr {
    if let Expr::Mul(lhs, rhs) = expr {
        if let Expr::Number(n) = &**lhs
            && *n == -1.0
        {
            return (**rhs).clone();
        }
        if let Expr::Number(n) = &**rhs
            && *n == -1.0
        {
            return (**lhs).clone();
        }
    }
    expr.clone()
}

/// Try to match alternative cosh pattern: (e^(2x) + 1) / (2 * e^x) = cosh(x)
/// Returns Some(x) if pattern matches
fn match_alt_cosh_pattern(numerator: &Expr, denominator: &Expr) -> Option<Expr> {
    // Denominator must be 2 * e^x
    let x = match_two_times_exp(denominator)?;

    // Numerator must be e^(2x) + 1
    if let Expr::Add(u, v) = numerator {
        // Check for e^(2x) + 1 or 1 + e^(2x)
        let (exp_term, _) = if matches!(&**v, Expr::Number(n) if *n == 1.0) {
            (u.as_ref(), v.as_ref())
        } else if matches!(&**u, Expr::Number(n) if *n == 1.0) {
            (v.as_ref(), u.as_ref())
        } else {
            return None;
        };

        // Check exp_term = e^(2x)
        if let Some(exp_arg) = ExpTerm::get_direct_exp_arg(exp_term)
            && is_double_of(&exp_arg, &x)
        {
            return Some(x);
        }
    }
    None
}

/// Try to match alternative sinh pattern: (e^(2x) - 1) / (2 * e^x) = sinh(x)
/// Returns Some(x) if pattern matches
fn match_alt_sinh_pattern(numerator: &Expr, denominator: &Expr) -> Option<Expr> {
    // Denominator must be 2 * e^x
    let x = match_two_times_exp(denominator)?;

    // Numerator must be e^(2x) - 1
    if let Expr::Sub(u, v) = numerator {
        // Check for e^(2x) - 1
        if matches!(&**v, Expr::Number(n) if *n == 1.0)
            && let Some(exp_arg) = ExpTerm::get_direct_exp_arg(u)
            && is_double_of(&exp_arg, &x)
        {
            return Some(x);
        }
    }
    None
}

/// Match pattern: 2 * e^x or e^x * 2
/// Returns the argument x if pattern matches
fn match_two_times_exp(expr: &Expr) -> Option<Expr> {
    if let Expr::Mul(lhs, rhs) = expr {
        // 2 * e^x
        if let Expr::Number(n) = &**lhs
            && *n == 2.0
        {
            return ExpTerm::get_direct_exp_arg(rhs);
        }
        // e^x * 2
        if let Expr::Number(n) = &**rhs
            && *n == 2.0
        {
            return ExpTerm::get_direct_exp_arg(lhs);
        }
    }
    None
}

/// Check if expr = 2 * other (i.e., expr is double of other)
fn is_double_of(expr: &Expr, other: &Expr) -> bool {
    if let Expr::Mul(lhs, rhs) = expr {
        if let Expr::Number(n) = &**lhs
            && *n == 2.0
            && **rhs == *other
        {
            return true;
        }
        if let Expr::Number(n) = &**rhs
            && *n == 2.0
            && **lhs == *other
        {
            return true;
        }
    }
    false
}

/// Try to match alternative sech pattern: (2 * e^x) / (e^(2x) + 1) = sech(x)
/// This is the reciprocal of the alt_cosh form
/// Returns Some(x) if pattern matches
fn match_alt_sech_pattern(numerator: &Expr, denominator: &Expr) -> Option<Expr> {
    // Numerator must be 2 * e^x
    let x = match_two_times_exp(numerator)?;

    // Denominator must be e^(2x) + 1
    if let Expr::Add(u, v) = denominator {
        // Check for e^(2x) + 1 or 1 + e^(2x)
        let exp_term = if matches!(&**v, Expr::Number(n) if *n == 1.0) {
            u.as_ref()
        } else if matches!(&**u, Expr::Number(n) if *n == 1.0) {
            v.as_ref()
        } else {
            return None;
        };

        // Check exp_term = e^(2x)
        if let Some(exp_arg) = ExpTerm::get_direct_exp_arg(exp_term)
            && is_double_of(&exp_arg, &x)
        {
            return Some(x);
        }
    }
    None
}

/// Try to match pattern: (e^x - 1/e^x) * e^x = e^(2x) - 1 (for sinh numerator in tanh)
/// Returns Some(x) if pattern matches
fn match_e2x_minus_1_factored(expr: &Expr) -> Option<Expr> {
    // Pattern: (e^x - 1/e^x) * e^x or e^x * (e^x - 1/e^x)
    if let Expr::Mul(lhs, rhs) = expr {
        // Check both orderings
        if let Some(x) = try_match_factored_sinh_times_exp(lhs, rhs) {
            return Some(x);
        }
        if let Some(x) = try_match_factored_sinh_times_exp(rhs, lhs) {
            return Some(x);
        }
    }
    None
}

/// Helper: try to match (e^x - 1/e^x) * e^x
fn try_match_factored_sinh_times_exp(factor: &Expr, exp_part: &Expr) -> Option<Expr> {
    // exp_part should be e^x
    let x = ExpTerm::get_direct_exp_arg(exp_part)?;

    // factor should be (e^x - 1/e^x)
    if let Expr::Sub(u, v) = factor {
        // u = e^x
        if let Some(arg_u) = ExpTerm::get_direct_exp_arg(u)
            && arg_u == x
        {
            // v = 1/e^x
            if let Expr::Div(num, den) = &**v
                && matches!(&**num, Expr::Number(n) if *n == 1.0)
                && let Some(arg_v) = ExpTerm::get_direct_exp_arg(den)
                && arg_v == x
            {
                return Some(x);
            }
        }
    }
    None
}

/// Match pattern: e^(2x) + 1 directly
/// Returns Some(x) if pattern matches
fn match_e2x_plus_1(expr: &Expr) -> Option<Expr> {
    if let Expr::Add(u, v) = expr {
        // Check for e^(2x) + 1 or 1 + e^(2x)
        let (exp_term, _const_term) = if matches!(&**v, Expr::Number(n) if *n == 1.0) {
            (u.as_ref(), v.as_ref())
        } else if matches!(&**u, Expr::Number(n) if *n == 1.0) {
            (v.as_ref(), u.as_ref())
        } else {
            return None;
        };

        // Check exp_term = e^(2x)
        if let Some(exp_arg) = ExpTerm::get_direct_exp_arg(exp_term) {
            // exp_arg should be 2*x
            if let Expr::Mul(lhs, rhs) = &exp_arg
                && let Expr::Number(n) = &**lhs
                && *n == 2.0
            {
                return Some((**rhs).clone());
            }
            if let Expr::Mul(lhs, rhs) = &exp_arg
                && let Expr::Number(n) = &**rhs
                && *n == 2.0
            {
                return Some((**lhs).clone());
            }
        }
    }
    None
}

/// Match pattern: e^(2x) - 1 directly (not factored form)
/// Returns Some(x) if pattern matches
fn match_e2x_minus_1_direct(expr: &Expr) -> Option<Expr> {
    if let Expr::Sub(u, v) = expr {
        // Check for e^(2x) - 1
        if matches!(&**v, Expr::Number(n) if *n == 1.0) {
            // Check u = e^(2x)
            if let Some(exp_arg) = ExpTerm::get_direct_exp_arg(u) {
                // exp_arg should be 2*x
                if let Expr::Mul(lhs, rhs) = &exp_arg
                    && let Expr::Number(n) = &**lhs
                    && *n == 2.0
                {
                    return Some((**rhs).clone());
                }
                if let Expr::Mul(lhs, rhs) = &exp_arg
                    && let Expr::Number(n) = &**rhs
                    && *n == 2.0
                {
                    return Some((**lhs).clone());
                }
            }
        }
    }
    // Also check Add form where second term is -1: e^(2x) + (-1)
    if let Expr::Add(u, v) = expr {
        // Check for e^(2x) + (-1) (i.e., -1 as a number)
        if matches!(&**v, Expr::Number(n) if *n == -1.0)
            && let Some(exp_arg) = ExpTerm::get_direct_exp_arg(u)
            && let Expr::Mul(lhs, rhs) = &exp_arg
        {
            if let Expr::Number(n) = &**lhs
                && *n == 2.0
            {
                return Some((**rhs).clone());
            }
            if let Expr::Number(n) = &**rhs
                && *n == 2.0
            {
                return Some((**lhs).clone());
            }
        }
        // Check (-1) + e^(2x)
        if matches!(&**u, Expr::Number(n) if *n == -1.0)
            && let Some(exp_arg) = ExpTerm::get_direct_exp_arg(v)
            && let Expr::Mul(lhs, rhs) = &exp_arg
        {
            if let Expr::Number(n) = &**lhs
                && *n == 2.0
            {
                return Some((**rhs).clone());
            }
            if let Expr::Number(n) = &**rhs
                && *n == 2.0
            {
                return Some((**lhs).clone());
            }
        }
    }
    None
}

/// Try to extract the inner expression from -1 * expr
fn extract_negated_term(expr: &Expr) -> Option<&Expr> {
    if let Expr::Mul(lhs, rhs) = expr {
        if let Expr::Number(n) = &**lhs
            && *n == -1.0
        {
            return Some(rhs);
        }
        if let Expr::Number(n) = &**rhs
            && *n == -1.0
        {
            return Some(lhs);
        }
    }
    None
}

// ============================================================================
// HYPERBOLIC FUNCTION RULES
// ============================================================================

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

    fn applies_to(&self) -> &'static [ExprKind] {
        &[ExprKind::Function]
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && name == "sinh"
            && args.len() == 1
            && matches!(args[0], Expr::Number(n) if n == 0.0)
        {
            return Some(Expr::Number(0.0));
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

    fn applies_to(&self) -> &'static [ExprKind] {
        &[ExprKind::Function]
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::FunctionCall { name, args } = expr
            && name == "cosh"
            && args.len() == 1
            && matches!(args[0], Expr::Number(n) if n == 0.0)
        {
            return Some(Expr::Number(1.0));
        }
        None
    }
}

/// Rule for converting (e^x - e^-x) / 2 to sinh(x)
/// Handles: e^x, exp(x), 1/e^x forms
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

    fn applies_to(&self) -> &'static [ExprKind] {
        &[ExprKind::Div]
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr {
            // Standard form: (e^x - e^(-x)) / 2
            if let Expr::Number(d) = &**denominator
                && *d == 2.0
            {
                // Pattern: (e^x - e^(-x)) / 2
                if let Expr::Sub(u, v) = &**numerator
                    && let Some(x) = match_sinh_pattern_sub(u, v)
                {
                    return Some(Expr::FunctionCall {
                        name: "sinh".to_string(),
                        args: vec![x],
                    });
                }
                // Pattern: (e^x + (-1)*e^(-x)) / 2
                if let Expr::Add(u, v) = &**numerator {
                    // Look for negated term
                    if let Some(neg_inner) = extract_negated_term(v)
                        && let Some(x) = match_sinh_pattern_sub(u, neg_inner)
                    {
                        return Some(Expr::FunctionCall {
                            name: "sinh".to_string(),
                            args: vec![x],
                        });
                    }
                    if let Some(neg_inner) = extract_negated_term(u)
                        && let Some(x) = match_sinh_pattern_sub(v, neg_inner)
                    {
                        return Some(Expr::FunctionCall {
                            name: "sinh".to_string(),
                            args: vec![x],
                        });
                    }
                }
            }

            // Alternative form: (e^(2x) - 1) / (2 * e^x) = sinh(x)
            if let Some(x) = match_alt_sinh_pattern(numerator, denominator) {
                return Some(Expr::FunctionCall {
                    name: "sinh".to_string(),
                    args: vec![x],
                });
            }
        }
        None
    }
}

/// Rule for converting (e^x + e^-x) / 2 to cosh(x)
/// Handles: e^x, exp(x), 1/e^x forms and alternative form (e^(2x) + 1) / (2 * e^x)
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

    fn applies_to(&self) -> &'static [ExprKind] {
        &[ExprKind::Div]
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr {
            // Standard form: (e^x + e^(-x)) / 2
            if let Expr::Number(d) = &**denominator
                && *d == 2.0
                && let Expr::Add(u, v) = &**numerator
                && let Some(x) = match_cosh_pattern(u, v)
            {
                return Some(Expr::FunctionCall {
                    name: "cosh".to_string(),
                    args: vec![x],
                });
            }

            // Alternative form: (e^(2x) + 1) / (2 * e^x) = cosh(x)
            if let Some(x) = match_alt_cosh_pattern(numerator, denominator) {
                return Some(Expr::FunctionCall {
                    name: "cosh".to_string(),
                    args: vec![x],
                });
            }
        }
        None
    }
}

/// Rule for converting (e^x - e^-x) / (e^x + e^-x) to tanh(x)
/// Handles: e^x, exp(x), 1/e^x forms
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

    fn applies_to(&self) -> &'static [ExprKind] {
        &[ExprKind::Div]
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr {
            // Standard form: (e^x - e^(-x)) / (e^x + e^(-x))
            let num_arg = if let Expr::Sub(u, v) = &**numerator {
                match_sinh_pattern_sub(u, v)
            } else {
                None
            };

            let den_arg = if let Expr::Add(u, v) = &**denominator {
                match_cosh_pattern(u, v)
            } else {
                None
            };

            if let (Some(n_arg), Some(d_arg)) = (num_arg, den_arg)
                && n_arg == d_arg
            {
                return Some(Expr::FunctionCall {
                    name: "tanh".to_string(),
                    args: vec![n_arg],
                });
            }

            // Alternative form: ((e^x - 1/e^x) * e^x) / (e^(2x) + 1) = (e^(2x) - 1) / (e^(2x) + 1) = tanh(x)
            if let Some(x_num) = match_e2x_minus_1_factored(numerator)
                && let Some(x_den) = match_e2x_plus_1(denominator)
                && x_num == x_den
            {
                return Some(Expr::FunctionCall {
                    name: "tanh".to_string(),
                    args: vec![x_num],
                });
            }

            // Direct form: (e^(2x) - 1) / (e^(2x) + 1) = tanh(x)
            if let Some(x_num) = match_e2x_minus_1_direct(numerator)
                && let Some(x_den) = match_e2x_plus_1(denominator)
                && x_num == x_den
            {
                return Some(Expr::FunctionCall {
                    name: "tanh".to_string(),
                    args: vec![x_num],
                });
            }
        }
        None
    }
}

/// Rule for converting 2 / (e^x + e^-x) to sech(x)
/// Handles: e^x, exp(x), 1/e^x forms
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

    fn applies_to(&self) -> &'static [ExprKind] {
        &[ExprKind::Div]
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr {
            // Standard form: 2 / (e^x + e^(-x))
            if let Expr::Number(n) = &**numerator
                && *n == 2.0
            {
                // Denominator: (e^x + e^(-x)) -> cosh pattern
                if let Expr::Add(u, v) = &**denominator
                    && let Some(x) = match_cosh_pattern(u, v)
                {
                    return Some(Expr::FunctionCall {
                        name: "sech".to_string(),
                        args: vec![x],
                    });
                }
            }

            // Alternative form: (2 * e^x) / (e^(2x) + 1) = sech(x)
            if let Some(x) = match_alt_sech_pattern(numerator, denominator) {
                return Some(Expr::FunctionCall {
                    name: "sech".to_string(),
                    args: vec![x],
                });
            }
        }
        None
    }
}

/// Rule for converting 2 / (e^x - e^-x) to csch(x)
/// Handles: e^x, exp(x), 1/e^x forms
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

    fn applies_to(&self) -> &'static [ExprKind] {
        &[ExprKind::Div]
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr
            && let Expr::Number(n) = &**numerator
            && *n == 2.0
        {
            // Denominator: (e^x - e^(-x)) -> sinh pattern
            if let Expr::Sub(u, v) = &**denominator
                && let Some(x) = match_sinh_pattern_sub(u, v)
            {
                return Some(Expr::FunctionCall {
                    name: "csch".to_string(),
                    args: vec![x],
                });
            }
        }
        None
    }
}

/// Rule for converting (e^x + e^-x) / (e^x - e^-x) to coth(x)
/// Handles: e^x, exp(x), 1/e^x forms
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

    fn applies_to(&self) -> &'static [ExprKind] {
        &[ExprKind::Div]
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(numerator, denominator) = expr {
            // Standard form: (e^x + e^(-x)) / (e^x - e^(-x))
            let num_arg = if let Expr::Add(u, v) = &**numerator {
                match_cosh_pattern(u, v)
            } else {
                None
            };

            let den_arg = if let Expr::Sub(u, v) = &**denominator {
                match_sinh_pattern_sub(u, v)
            } else {
                None
            };

            if let (Some(n_arg), Some(d_arg)) = (num_arg, den_arg)
                && n_arg == d_arg
            {
                return Some(Expr::FunctionCall {
                    name: "coth".to_string(),
                    args: vec![n_arg],
                });
            }

            // Alternative form: (e^(2x) + 1) / ((e^x - 1/e^x) * e^x) = (e^(2x) + 1) / (e^(2x) - 1) = coth(x)
            if let Some(x_num) = match_e2x_plus_1(numerator)
                && let Some(x_den) = match_e2x_minus_1_factored(denominator)
                && x_num == x_den
            {
                return Some(Expr::FunctionCall {
                    name: "coth".to_string(),
                    args: vec![x_num],
                });
            }

            // Direct form: (e^(2x) + 1) / (e^(2x) - 1) = coth(x)
            if let Some(x_num) = match_e2x_plus_1(numerator)
                && let Some(x_den) = match_e2x_minus_1_direct(denominator)
                && x_num == x_den
            {
                return Some(Expr::FunctionCall {
                    name: "coth".to_string(),
                    args: vec![x_num],
                });
            }
        }
        None
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
        if let Expr::Sub(u, v) = expr
            && let (Some((name1, arg1)), Some((name2, arg2))) = (
                Self::get_hyperbolic_power(u, 2.0),
                Self::get_hyperbolic_power(v, 2.0),
            )
            && arg1 == arg2
            && name1 == "cosh"
            && name2 == "sinh"
        {
            return Some(Expr::Number(1.0));
        }

        // Check for cosh^2(x) + (-1 * sinh^2(x))
        if let Expr::Add(u, v) = expr {
            // Check u = cosh^2, v = -sinh^2
            if let Some((name1, arg1)) = Self::get_hyperbolic_power(u, 2.0)
                && name1 == "cosh"
            {
                // Check v
                if let Expr::Mul(lhs, rhs) = &**v {
                    if let Expr::Number(n) = **lhs
                        && n == -1.0
                        && let Some((name2, arg2)) = Self::get_hyperbolic_power(rhs, 2.0)
                        && name2 == "sinh"
                        && arg1 == arg2
                    {
                        return Some(Expr::Number(1.0));
                    }
                    // Check rhs is -1 (commutative)
                    if let Expr::Number(n) = **rhs
                        && n == -1.0
                        && let Some((name2, arg2)) = Self::get_hyperbolic_power(lhs, 2.0)
                        && name2 == "sinh"
                        && arg1 == arg2
                    {
                        return Some(Expr::Number(1.0));
                    }
                }
            }

            // Check v = cosh^2, u = -sinh^2 (commutative add)
            if let Some((name1, arg1)) = Self::get_hyperbolic_power(v, 2.0)
                && name1 == "cosh"
            {
                // Check u
                if let Expr::Mul(lhs, rhs) = &**u {
                    if let Expr::Number(n) = **lhs
                        && n == -1.0
                        && let Some((name2, arg2)) = Self::get_hyperbolic_power(rhs, 2.0)
                        && name2 == "sinh"
                        && arg1 == arg2
                    {
                        return Some(Expr::Number(1.0));
                    }
                    // Check rhs is -1
                    if let Expr::Number(n) = **rhs
                        && n == -1.0
                        && let Some((name2, arg2)) = Self::get_hyperbolic_power(lhs, 2.0)
                        && name2 == "sinh"
                        && arg1 == arg2
                    {
                        return Some(Expr::Number(1.0));
                    }
                }
            }
        }

        // Check for 1 - tanh^2(x) = sech^2(x)
        if let Expr::Sub(u, v) = expr
            && let Expr::Number(n) = **u
            && n == 1.0
            && let Some((name, arg)) = Self::get_hyperbolic_power(v, 2.0)
            && name == "tanh"
        {
            return Some(Expr::Pow(
                Box::new(Expr::FunctionCall {
                    name: "sech".to_string(),
                    args: vec![arg],
                }),
                Box::new(Expr::Number(2.0)),
            ));
        }

        // Check for 1 + (-1 * tanh^2(x)) = sech^2(x) (normalized form)
        if let Expr::Add(u, v) = expr
            && let Expr::Number(n) = **u
            && n == 1.0
            && let Expr::Mul(lhs, rhs) = &**v
            && let Expr::Number(nn) = **lhs
            && nn == -1.0
            && let Some((name, arg)) = Self::get_hyperbolic_power(rhs, 2.0)
            && name == "tanh"
        {
            return Some(Expr::Pow(
                Box::new(Expr::FunctionCall {
                    name: "sech".to_string(),
                    args: vec![arg],
                }),
                Box::new(Expr::Number(2.0)),
            ));
        }

        // Check for tanh^2(x) + (-1) = -sech^2(x) or commutative
        if let Expr::Add(u, v) = expr
            && let Some((name, arg)) = Self::get_hyperbolic_power(u, 2.0)
            && name == "tanh"
            && let Expr::Mul(lhs, rhs) = &**v
            && let Expr::Number(n) = **lhs
            && n == -1.0
            && **rhs == Expr::Number(1.0)
        {
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

        // Check for coth^2(x) - 1 = csch^2(x)
        if let Expr::Sub(u, v) = expr
            && let Expr::Number(n) = **v
            && n == 1.0
            && let Some((name, arg)) = Self::get_hyperbolic_power(u, 2.0)
            && name == "coth"
        {
            return Some(Expr::Pow(
                Box::new(Expr::FunctionCall {
                    name: "csch".to_string(),
                    args: vec![arg],
                }),
                Box::new(Expr::Number(2.0)),
            ));
        }

        // Check for coth^2(x) + (-1) = csch^2(x) (normalized form)
        if let Expr::Add(u, v) = expr
            && let Some((name, arg)) = Self::get_hyperbolic_power(u, 2.0)
            && name == "coth"
            && let Expr::Number(n) = **v
            && n == -1.0
        {
            return Some(Expr::Pow(
                Box::new(Expr::FunctionCall {
                    name: "csch".to_string(),
                    args: vec![arg],
                }),
                Box::new(Expr::Number(2.0)),
            ));
        }

        // Check for (cosh(x) - sinh(x)) * (cosh(x) + sinh(x)) = 1
        if let Expr::Mul(u, v) = expr {
            if let (Some(arg1), Some(arg2)) = (
                Self::is_cosh_minus_sinh_term(u),
                Self::is_cosh_plus_sinh_term(v),
            ) && arg1 == arg2
            {
                return Some(Expr::Number(1.0));
            }
            if let (Some(arg1), Some(arg2)) = (
                Self::is_cosh_minus_sinh_term(v),
                Self::is_cosh_plus_sinh_term(u),
            ) && arg1 == arg2
            {
                return Some(Expr::Number(1.0));
            }
        }

        None
    }
}

impl HyperbolicIdentityRule {
    fn get_hyperbolic_power(expr: &Expr, power: f64) -> Option<(&str, Expr)> {
        if let Expr::Pow(base, exp) = expr
            && let Expr::Number(p) = **exp
            && p == power
            && let Expr::FunctionCall { name, args } = &**base
            && args.len() == 1
            && (name == "sinh" || name == "cosh" || name == "tanh" || name == "coth")
        {
            return Some((name, args[0].clone()));
        }
        None
    }

    fn is_cosh_minus_sinh_term(expr: &Expr) -> Option<Expr> {
        // Check for cosh - sinh: Sub(cosh, sinh)
        if let Expr::Sub(u, v) = expr
            && let Expr::FunctionCall { name: n1, args: a1 } = &**u
            && n1 == "cosh"
            && a1.len() == 1
            && let Expr::FunctionCall { name: n2, args: a2 } = &**v
            && n2 == "sinh"
            && a2.len() == 1
            && a1[0] == a2[0]
        {
            return Some(a1[0].clone());
        }
        // Check for normalized cosh - sinh: Add(cosh, Mul(-1, sinh))
        if let Expr::Add(u, v) = expr
            && let Expr::FunctionCall { name: n1, args: a1 } = &**u
            && n1 == "cosh"
            && a1.len() == 1
            && let Expr::Mul(lhs, rhs) = &**v
            && let Expr::Number(n) = **lhs
            && n == -1.0
            && let Expr::FunctionCall { name: n2, args: a2 } = &**rhs
            && n2 == "sinh"
            && a2.len() == 1
            && a1[0] == a2[0]
        {
            return Some(a1[0].clone());
        }
        None
    }

    fn is_cosh_plus_sinh_term(expr: &Expr) -> Option<Expr> {
        // Check for cosh + sinh: Add(cosh, sinh)
        if let Expr::Add(u, v) = expr
            && let Expr::FunctionCall { name: n1, args: a1 } = &**u
            && n1 == "cosh"
            && a1.len() == 1
            && let Expr::FunctionCall { name: n2, args: a2 } = &**v
            && n2 == "sinh"
            && a2.len() == 1
            && a1[0] == a2[0]
        {
            return Some(a1[0].clone());
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
        if let Expr::FunctionCall { name, args } = expr
            && name == "sinh"
            && args.len() == 1
            && Self::is_negation(&args[0])
        {
            let inner = Self::negate_arg(&args[0]);
            return Some(Expr::Mul(
                Box::new(Expr::Number(-1.0)),
                Box::new(Expr::FunctionCall {
                    name: "sinh".to_string(),
                    args: vec![inner],
                }),
            ));
        }
        None
    }
}

impl SinhNegationRule {
    fn is_negation(expr: &Expr) -> bool {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs
                && n == -1.0
            {
                return true;
            }
            if let Expr::Number(n) = **rhs
                && n == -1.0
            {
                return true;
            }
        }
        false
    }

    fn negate_arg(expr: &Expr) -> Expr {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs
                && n == -1.0
            {
                return *rhs.clone();
            }
            if let Expr::Number(n) = **rhs
                && n == -1.0
            {
                return *lhs.clone();
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
        if let Expr::FunctionCall { name, args } = expr
            && name == "cosh"
            && args.len() == 1
            && let Expr::Mul(lhs, rhs) = &args[0]
            && let Expr::Number(n) = **lhs
            && n == -1.0
        {
            return Some(Expr::FunctionCall {
                name: "cosh".to_string(),
                args: vec![*rhs.clone()],
            });
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
        if let Expr::FunctionCall { name, args } = expr
            && name == "tanh"
            && args.len() == 1
            && Self::is_negation(&args[0])
        {
            let inner = Self::negate_arg(&args[0]);
            return Some(Expr::Mul(
                Box::new(Expr::Number(-1.0)),
                Box::new(Expr::FunctionCall {
                    name: "tanh".to_string(),
                    args: vec![inner],
                }),
            ));
        }
        None
    }
}

impl TanhNegationRule {
    fn is_negation(expr: &Expr) -> bool {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs
                && n == -1.0
            {
                return true;
            }
            if let Expr::Number(n) = **rhs
                && n == -1.0
            {
                return true;
            }
        }
        false
    }

    fn negate_arg(expr: &Expr) -> Expr {
        if let Expr::Mul(lhs, rhs) = expr {
            if let Expr::Number(n) = **lhs
                && n == -1.0
            {
                return *rhs.clone();
            }
            if let Expr::Number(n) = **rhs
                && n == -1.0
            {
                return *lhs.clone();
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
        if let Expr::FunctionCall { name, args } = expr
            && name == "sinh"
            && args.len() == 1
            && let Expr::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0]
            && inner_name == "asinh"
            && inner_args.len() == 1
        {
            return Some(inner_args[0].clone());
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
        if let Expr::FunctionCall { name, args } = expr
            && name == "cosh"
            && args.len() == 1
            && let Expr::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0]
            && inner_name == "acosh"
            && inner_args.len() == 1
        {
            return Some(inner_args[0].clone());
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
        if let Expr::FunctionCall { name, args } = expr
            && name == "tanh"
            && args.len() == 1
            && let Expr::FunctionCall {
                name: inner_name,
                args: inner_args,
            } = &args[0]
            && inner_name == "atanh"
            && inner_args.len() == 1
        {
            return Some(inner_args[0].clone());
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
        ) && arg1 == arg2
        {
            // Allow small floating point tolerance
            let eps = 1e-10;
            if ((c1 == 4.0 || (c1 - 4.0).abs() < eps) && p1 == 3.0) && (c2 == 3.0 && p2 == 1.0)
                || ((c2 == 4.0 || (c2 - 4.0).abs() < eps) && p2 == 3.0) && (c1 == 3.0 && p1 == 1.0)
            {
                return Some(arg1);
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
        ) && arg1 == arg2
        {
            // Allow small floating point tolerance
            let eps = 1e-10;
            if (c1 == 4.0 || (c1 - 4.0).abs() < eps) && p1 == 3.0 && c2 == 3.0 && p2 == 1.0 {
                return Some(arg1);
            }
        }
        None
    }

    // Helper to parse c * func(arg)^p
    fn parse_fn_term(expr: &Expr, func_name: &str) -> Option<(f64, Expr, f64)> {
        // Case 1: func(arg)  -> c=1, p=1
        if let Expr::FunctionCall { name, args } = expr
            && name == func_name
            && args.len() == 1
        {
            return Some((1.0, args[0].clone(), 1.0));
        }
        // Case 2: c * func(arg) -> p=1
        if let Expr::Mul(lhs, rhs) = expr
            && let Expr::Number(c) = **lhs
            && let Expr::FunctionCall { name, args } = &**rhs
            && name == func_name
            && args.len() == 1
        {
            return Some((c, args[0].clone(), 1.0));
        }
        // Case 3: func(arg)^p -> c=1
        if let Expr::Pow(base, exp) = expr
            && let Expr::Number(p) = **exp
            && let Expr::FunctionCall { name, args } = &**base
            && name == func_name
            && args.len() == 1
        {
            return Some((1.0, args[0].clone(), p));
        }
        // Case 4: c * func(arg)^p
        if let Expr::Mul(lhs, rhs) = expr
            && let Expr::Number(c) = **lhs
            && let Expr::Pow(base, exp) = &**rhs
            && let Expr::Number(p) = **exp
            && let Expr::FunctionCall { name, args } = &**base
            && name == func_name
            && args.len() == 1
        {
            return Some((c, args[0].clone(), p));
        }
        None
    }
}

// ============================================================================
// HYPERBOLIC RATIO RULES
// ============================================================================

/// Rule for sinh(x)/cosh(x) -> tanh(x)
pub struct SinhCoshToTanhRule;

impl Rule for SinhCoshToTanhRule {
    fn name(&self) -> &'static str {
        "sinh_cosh_to_tanh"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(num, den) = expr {
            // sinh(x) / cosh(x) -> tanh(x)
            if let Expr::FunctionCall {
                name: num_name,
                args: num_args,
            } = &**num
                && let Expr::FunctionCall {
                    name: den_name,
                    args: den_args,
                } = &**den
                && num_name == "sinh"
                && den_name == "cosh"
                && num_args.len() == 1
                && den_args.len() == 1
                && num_args[0] == den_args[0]
            {
                return Some(Expr::FunctionCall {
                    name: "tanh".to_string(),
                    args: vec![num_args[0].clone()],
                });
            }
        }
        None
    }
}

/// Rule for cosh(x)/sinh(x) -> coth(x)
pub struct CoshSinhToCothRule;

impl Rule for CoshSinhToCothRule {
    fn name(&self) -> &'static str {
        "cosh_sinh_to_coth"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(num, den) = expr {
            // cosh(x) / sinh(x) -> coth(x)
            if let Expr::FunctionCall {
                name: num_name,
                args: num_args,
            } = &**num
                && let Expr::FunctionCall {
                    name: den_name,
                    args: den_args,
                } = &**den
                && num_name == "cosh"
                && den_name == "sinh"
                && num_args.len() == 1
                && den_args.len() == 1
                && num_args[0] == den_args[0]
            {
                return Some(Expr::FunctionCall {
                    name: "coth".to_string(),
                    args: vec![num_args[0].clone()],
                });
            }
        }
        None
    }
}

/// Rule for 1/cosh(x) -> sech(x)
pub struct OneCoshToSechRule;

impl Rule for OneCoshToSechRule {
    fn name(&self) -> &'static str {
        "one_cosh_to_sech"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(num, den) = expr {
            // 1 / cosh(x) -> sech(x)
            if let Expr::Number(n) = &**num
                && (*n - 1.0).abs() < 1e-10
                && let Expr::FunctionCall { name, args } = &**den
                && name == "cosh"
                && args.len() == 1
            {
                return Some(Expr::FunctionCall {
                    name: "sech".to_string(),
                    args: args.clone(),
                });
            }
        }
        None
    }
}

/// Rule for 1/sinh(x) -> csch(x)
pub struct OneSinhToCschRule;

impl Rule for OneSinhToCschRule {
    fn name(&self) -> &'static str {
        "one_sinh_to_csch"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(num, den) = expr {
            // 1 / sinh(x) -> csch(x)
            if let Expr::Number(n) = &**num
                && (*n - 1.0).abs() < 1e-10
                && let Expr::FunctionCall { name, args } = &**den
                && name == "sinh"
                && args.len() == 1
            {
                return Some(Expr::FunctionCall {
                    name: "csch".to_string(),
                    args: args.clone(),
                });
            }
        }
        None
    }
}

/// Rule for 1/tanh(x) -> coth(x)
pub struct OneTanhToCothRule;

impl Rule for OneTanhToCothRule {
    fn name(&self) -> &'static str {
        "one_tanh_to_coth"
    }

    fn priority(&self) -> i32 {
        85
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Hyperbolic
    }

    fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
        if let Expr::Div(num, den) = expr {
            // 1 / tanh(x) -> coth(x)
            if let Expr::Number(n) = &**num
                && (*n - 1.0).abs() < 1e-10
                && let Expr::FunctionCall { name, args } = &**den
                && name == "tanh"
                && args.len() == 1
            {
                return Some(Expr::FunctionCall {
                    name: "coth".to_string(),
                    args: args.clone(),
                });
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
        // Ratio rules - convert to tanh, coth, sech, csch
        Rc::new(SinhCoshToTanhRule),
        Rc::new(CoshSinhToCothRule),
        Rc::new(OneCoshToSechRule),
        Rc::new(OneSinhToCschRule),
        Rc::new(OneTanhToCothRule),
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
