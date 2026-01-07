use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::{E, EXP};

// ============================================================================
// SHARED HELPER FUNCTIONS FOR HYPERBOLIC PATTERN MATCHING
// ============================================================================

/// Represents an exponential term with its argument
/// e^x has argument x, e^(-x) has argument -x, 1/e^x has argument -x
#[derive(Debug, Clone)]
pub struct ExpTerm {
    pub arg: Expr,
}

impl ExpTerm {
    /// Try to extract an exponential term from various forms:
    /// - e^x -> `ExpTerm` { arg: x }
    /// - exp(x) -> `ExpTerm` { arg: x }  
    /// - 1/e^x -> `ExpTerm` { arg: -x }
    /// - 1/exp(x) -> `ExpTerm` { arg: -x }
    pub fn from_expr(expr: &Expr) -> Option<Self> {
        // Direct form: e^x or exp(x)
        if let Some(arg) = Self::get_direct_exp_arg(expr) {
            return Some(Self { arg });
        }

        // Reciprocal form: 1/e^x or 1/exp(x)
        if let AstKind::Div(num, den) = &expr.kind
            && let AstKind::Number(n) = &num.kind
            && *n == 1.0
            && let Some(arg) = Self::get_direct_exp_arg(den)
        {
            // 1/e^x = e^(-x)
            return Some(Self {
                arg: Self::negate(&arg),
            });
        }

        None
    }

    /// Get the argument from e^x or exp(x) directly (not handling 1/e^x)
    pub fn get_direct_exp_arg(expr: &Expr) -> Option<Expr> {
        match &expr.kind {
            AstKind::Pow(base, exp) => {
                if let AstKind::Symbol(b) = &base.kind
                    && b.id() == *E
                {
                    return Some((**exp).clone());
                }
                None
            }
            AstKind::FunctionCall { name, args } => {
                if name.id() == *EXP && args.len() == 1 {
                    return Some((*args[0]).clone());
                }
                None
            }
            _ => None,
        }
    }

    /// Check if this argument is the negation of another
    pub fn is_negation_of(&self, other: &Expr) -> bool {
        Self::args_are_negations(&self.arg, other)
    }

    /// Check if two arguments are negations of each other: arg1 = -arg2
    pub fn args_are_negations(arg1: &Expr, arg2: &Expr) -> bool {
        // Check if arg1 = Product([-1, arg2])
        if let AstKind::Product(factors) = &arg1.kind
            && factors.len() == 2
        {
            if let AstKind::Number(n) = &factors[0].kind
                && *n == -1.0
                && *factors[1] == *arg2
            {
                return true;
            }
            if let AstKind::Number(n) = &factors[1].kind
                && *n == -1.0
                && *factors[0] == *arg2
            {
                return true;
            }
        }
        // Check if arg2 = Product([-1, arg1])
        if let AstKind::Product(factors) = &arg2.kind
            && factors.len() == 2
        {
            if let AstKind::Number(n) = &factors[0].kind
                && *n == -1.0
                && *factors[1] == *arg1
            {
                return true;
            }
            if let AstKind::Number(n) = &factors[1].kind
                && *n == -1.0
                && *factors[0] == *arg1
            {
                return true;
            }
        }
        false
    }

    /// Create the negation of an expression: x -> -1 * x
    pub fn negate(expr: &Expr) -> Expr {
        // If it's already a negation, return the inner part
        if let AstKind::Product(factors) = &expr.kind
            && factors.len() == 2
        {
            if let AstKind::Number(n) = &factors[0].kind
                && *n == -1.0
            {
                return (*factors[1]).clone();
            }
            if let AstKind::Number(n) = &factors[1].kind
                && *n == -1.0
            {
                return (*factors[0]).clone();
            }
        }
        Expr::product(vec![Expr::number(-1.0), expr.clone()])
    }
}

/// Try to match the pattern (e^x + e^(-x)) for cosh detection
/// Returns Some(x) if pattern matches (always returns the positive argument)
pub fn match_cosh_pattern(u: &Expr, v: &Expr) -> Option<Expr> {
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
pub fn match_sinh_pattern_sub(u: &Expr, v: &Expr) -> Option<Expr> {
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
/// If expr is -x (i.e., Product([-1, x])), return x
/// Otherwise return expr as-is
pub fn get_positive_form(expr: &Expr) -> Expr {
    if let AstKind::Product(factors) = &expr.kind
        && factors.len() == 2
    {
        if let AstKind::Number(n) = &factors[0].kind
            && *n == -1.0
        {
            return (*factors[1]).clone();
        }
        if let AstKind::Number(n) = &factors[1].kind
            && *n == -1.0
        {
            return (*factors[0]).clone();
        }
    }
    expr.clone()
}

/// Try to match alternative cosh pattern: (e^(2x) + 1) / (2 * e^x) = cosh(x)
/// Returns Some(x) if pattern matches
pub fn match_alt_cosh_pattern(numerator: &Expr, denominator: &Expr) -> Option<Expr> {
    // Denominator must be 2 * e^x
    let x = match_two_times_exp(denominator)?;

    // Numerator must be e^(2x) + 1
    if let AstKind::Sum(terms) = &numerator.kind
        && terms.len() == 2
    {
        // Check for e^(2x) + 1 or 1 + e^(2x)
        let exp_term = if matches!(&terms[1].kind, AstKind::Number(n) if *n == 1.0) {
            &terms[0]
        } else if matches!(&terms[0].kind, AstKind::Number(n) if *n == 1.0) {
            &terms[1]
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
pub fn match_alt_sinh_pattern(numerator: &Expr, denominator: &Expr) -> Option<Expr> {
    // Denominator must be 2 * e^x
    let x = match_two_times_exp(denominator)?;

    // Numerator must be e^(2x) + (-1) (n-ary representation of subtraction)
    if let AstKind::Sum(terms) = &numerator.kind
        && terms.len() == 2
    {
        // Check for e^(2x) + (-1)
        if let AstKind::Number(n) = &terms[1].kind
            && *n == -1.0
            && let Some(exp_arg) = ExpTerm::get_direct_exp_arg(&terms[0])
            && is_double_of(&exp_arg, &x)
        {
            return Some(x);
        }
    }
    None
}

/// Match pattern: 2 * e^x or Product([2, e^x])
/// Returns the argument x if pattern matches
pub fn match_two_times_exp(expr: &Expr) -> Option<Expr> {
    if let AstKind::Product(factors) = &expr.kind
        && factors.len() == 2
    {
        // 2 * e^x
        if let AstKind::Number(n) = &factors[0].kind
            && *n == 2.0
        {
            return ExpTerm::get_direct_exp_arg(&factors[1]);
        }
        // e^x * 2
        if let AstKind::Number(n) = &factors[1].kind
            && *n == 2.0
        {
            return ExpTerm::get_direct_exp_arg(&factors[0]);
        }
    }
    None
}

/// Check if expr = 2 * other (i.e., expr is double of other)
pub fn is_double_of(expr: &Expr, other: &Expr) -> bool {
    if let AstKind::Product(factors) = &expr.kind
        && factors.len() == 2
    {
        if let AstKind::Number(n) = &factors[0].kind
            && *n == 2.0
            && *factors[1] == *other
        {
            return true;
        }
        if let AstKind::Number(n) = &factors[1].kind
            && *n == 2.0
            && *factors[0] == *other
        {
            return true;
        }
    }
    false
}

/// Try to match alternative sech pattern: (2 * e^x) / (e^(2x) + 1) = sech(x)
/// This is the reciprocal of the `alt_cosh` form
/// Returns Some(x) if pattern matches
pub fn match_alt_sech_pattern(numerator: &Expr, denominator: &Expr) -> Option<Expr> {
    // Numerator must be 2 * e^x
    let x = match_two_times_exp(numerator)?;

    // Denominator must be e^(2x) + 1
    if let AstKind::Sum(terms) = &denominator.kind
        && terms.len() == 2
    {
        // Check for e^(2x) + 1 or 1 + e^(2x)
        let exp_term = if matches!(&terms[1].kind, AstKind::Number(n) if *n == 1.0) {
            &terms[0]
        } else if matches!(&terms[0].kind, AstKind::Number(n) if *n == 1.0) {
            &terms[1]
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
pub fn match_e2x_minus_1_factored(expr: &Expr) -> Option<Expr> {
    // Pattern: (e^x - 1/e^x) * e^x or Product([..., e^x])
    if let AstKind::Product(factors) = &expr.kind {
        // Check pairwise for factored form
        for (i, f1) in factors.iter().enumerate() {
            for (j, f2) in factors.iter().enumerate() {
                if i != j
                    && let Some(x) = try_match_factored_sinh_times_exp(f1, f2)
                {
                    return Some(x);
                }
            }
        }
    }
    None
}

/// Helper: try to match (e^x - 1/e^x) * e^x
fn try_match_factored_sinh_times_exp(factor: &Expr, exp_part: &Expr) -> Option<Expr> {
    // exp_part should be e^x
    let x = ExpTerm::get_direct_exp_arg(exp_part)?;

    // factor should be Sum([e^x, Product([-1, 1/e^x])]) representing (e^x - 1/e^x)
    if let AstKind::Sum(terms) = &factor.kind
        && terms.len() == 2
    {
        // Check for e^x as first term
        if let Some(arg_u) = ExpTerm::get_direct_exp_arg(&terms[0])
            && arg_u == x
        {
            // Check second term is -1/e^x
            if let Some(negated) = extract_negated_term(&terms[1])
                && let AstKind::Div(num, den) = &negated.kind
                && let AstKind::Number(n) = &num.kind
                && *n == 1.0
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
pub fn match_e2x_plus_1(expr: &Expr) -> Option<Expr> {
    if let AstKind::Sum(terms) = &expr.kind
        && terms.len() == 2
    {
        // Check for e^(2x) + 1 or 1 + e^(2x)
        let exp_term = if matches!(&terms[1].kind, AstKind::Number(n) if *n == 1.0) {
            &terms[0]
        } else if matches!(&terms[0].kind, AstKind::Number(n) if *n == 1.0) {
            &terms[1]
        } else {
            return None;
        };

        // Check exp_term = e^(2x)
        if let Some(exp_arg) = ExpTerm::get_direct_exp_arg(exp_term) {
            // exp_arg should be 2*x
            if let AstKind::Product(factors) = &exp_arg.kind
                && factors.len() == 2
            {
                if let AstKind::Number(n) = &factors[0].kind
                    && *n == 2.0
                {
                    return Some((*factors[1]).clone());
                }
                if let AstKind::Number(n) = &factors[1].kind
                    && *n == 2.0
                {
                    return Some((*factors[0]).clone());
                }
            }
        }
    }
    None
}

/// Match pattern: e^(2x) - 1 directly (not factored form)
/// Returns Some(x) if pattern matches
pub fn match_e2x_minus_1_direct(expr: &Expr) -> Option<Expr> {
    if let AstKind::Sum(terms) = &expr.kind
        && terms.len() == 2
    {
        // Check for e^(2x) + (-1)
        if let AstKind::Number(n) = &terms[1].kind
            && *n == -1.0
        {
            // Check u = e^(2x)
            if let Some(exp_arg) = ExpTerm::get_direct_exp_arg(&terms[0]) {
                // exp_arg should be 2*x
                if let AstKind::Product(factors) = &exp_arg.kind
                    && factors.len() == 2
                {
                    if let AstKind::Number(n) = &factors[0].kind
                        && *n == 2.0
                    {
                        return Some((*factors[1]).clone());
                    }
                    if let AstKind::Number(n) = &factors[1].kind
                        && *n == 2.0
                    {
                        return Some((*factors[0]).clone());
                    }
                }
            }
        }
        // Check (-1) + e^(2x)
        if let AstKind::Number(n) = &terms[0].kind
            && *n == -1.0
            && let Some(exp_arg) = ExpTerm::get_direct_exp_arg(&terms[1])
            && let AstKind::Product(factors) = &exp_arg.kind
            && factors.len() == 2
        {
            if let AstKind::Number(n) = &factors[0].kind
                && *n == 2.0
            {
                return Some((*factors[1]).clone());
            }
            if let AstKind::Number(n) = &factors[1].kind
                && *n == 2.0
            {
                return Some((*factors[0]).clone());
            }
        }
    }
    None
}

/// Try to extract the inner expression from Product([-1, expr])
pub fn extract_negated_term(expr: &Expr) -> Option<Expr> {
    if let AstKind::Product(factors) = &expr.kind
        && factors.len() == 2
    {
        if let AstKind::Number(n) = &factors[0].kind
            && *n == -1.0
        {
            return Some((*factors[1]).clone());
        }
        if let AstKind::Number(n) = &factors[1].kind
            && *n == -1.0
        {
            return Some((*factors[0]).clone());
        }
    }
    None
}
