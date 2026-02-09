use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::KS;
use crate::core::traits::EPSILON;

// ============================================================================
// SHARED HELPER FUNCTIONS FOR HYPERBOLIC PATTERN MATCHING
// ============================================================================

/// Represents an exponential term with its argument
/// e^x has argument x, e^(-x) has argument -x, 1/e^x has argument -x
#[derive(Debug, Clone)]
pub struct ExpTerm {
    /// The argument of the exponential term
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
            && {
                // Exact check for constant 1.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                let is_one = *n == 1.0_f64;
                is_one
            }
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
                    && b.id() == KS.e
                {
                    return Some((**exp).clone());
                }
                None
            }
            AstKind::FunctionCall { name, args } => {
                if name.id() == KS.exp && args.len() == 1 {
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
            if {
                // Exact check for constant -1.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
                let is_neg_one = matches!(&factors[0].kind, AstKind::Number(n) if *n == -1.0_f64);
                is_neg_one
            } && *factors[1] == *arg2
            {
                return true;
            }
            if {
                // Exact check for constant -1.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
                let is_neg_one = matches!(&factors[1].kind, AstKind::Number(n) if *n == -1.0_f64);
                is_neg_one
            } && *factors[0] == *arg2
            {
                return true;
            }
        }
        // Check if arg2 = Product([-1, arg1])
        if let AstKind::Product(factors) = &arg2.kind
            && factors.len() == 2
        {
            if {
                // Exact check for constant -1.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
                let is_neg_one = matches!(&factors[0].kind, AstKind::Number(n) if *n == -1.0);
                is_neg_one
            } && *factors[1] == *arg1
            {
                return true;
            }
            if {
                // Exact check for constant -1.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
                let is_neg_one = matches!(&factors[1].kind, AstKind::Number(n) if *n == -1.0);
                is_neg_one
            } && *factors[0] == *arg1
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
                && {
                    // Exact check for constant -1.0
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
                    let is_neg_one = *n == -1.0;
                    is_neg_one
                }
            {
                return (*factors[1]).clone();
            }
            if let AstKind::Number(n) = &factors[1].kind
                && {
                    // Exact check for constant -1.0
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
                    let is_neg_one = *n == -1.0;
                    is_neg_one
                }
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
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
        let is_neg_one0 = matches!(&factors[0].kind, AstKind::Number(n) if *n == -1.0);
        if is_neg_one0 {
            return (*factors[1]).clone();
        }
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
        let is_neg_one1 = matches!(&factors[1].kind, AstKind::Number(n) if *n == -1.0);
        if is_neg_one1 {
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
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
        let is_one1 = matches!(&terms[1].kind, AstKind::Number(n) if *n == 1.0_f64);
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
        let is_one0 = matches!(&terms[0].kind, AstKind::Number(n) if *n == 1.0_f64);

        let exp_term = if is_one1 {
            &terms[0]
        } else if is_one0 {
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
        if {
            // Exact check for constant -1.0
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
            let is_neg_one = matches!(&terms[1].kind, AstKind::Number(n) if *n == -1.0);
            is_neg_one
        } && let Some(exp_arg) = ExpTerm::get_direct_exp_arg(&terms[0])
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
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
        let is_two0 = matches!(&factors[0].kind, AstKind::Number(n) if *n == 2.0_f64);
        if is_two0 {
            return ExpTerm::get_direct_exp_arg(&factors[1]);
        }
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
        let is_two1 = matches!(&factors[1].kind, AstKind::Number(n) if *n == 2.0_f64);
        if is_two1 {
            return ExpTerm::get_direct_exp_arg(&factors[0]);
        }
    }
    // Check Poly(2*e^x) -> gets e^x, then extracts arg
    if let AstKind::Poly(p) = &expr.kind
        && p.terms().len() == 1
        && p.terms()[0] == (1, 2.0)
    {
        // base should be e^x
        return ExpTerm::get_direct_exp_arg(p.base());
    }
    None
}

/// Check if expr = 2 * other (i.e., expr is double of other)
pub fn is_double_of(expr: &Expr, other: &Expr) -> bool {
    // Check Product([2, other]) or Product([other, 2])
    if let AstKind::Product(factors) = &expr.kind
        && factors.len() == 2
    {
        if matches!(&factors[0].kind, AstKind::Number(n) if (*n - 2.0).abs() < EPSILON)
            && *factors[1] == *other
        {
            return true;
        }
        if matches!(&factors[1].kind, AstKind::Number(n) if (*n - 2.0).abs() < EPSILON)
            && *factors[0] == *other
        {
            return true;
        }
    }
    // Check Poly(2*other)
    if let AstKind::Poly(p) = &expr.kind
        && p.terms().len() == 1
        && p.terms()[0] == (1, 2.0)
        && *p.base() == *other
    {
        return true;
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
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
        let is_one1 = matches!(&terms[1].kind, AstKind::Number(n) if *n == 1.0);
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
        let is_one0 = matches!(&terms[0].kind, AstKind::Number(n) if *n == 1.0);

        let exp_term = if is_one1 {
            &terms[0]
        } else if is_one0 {
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
        // Try e^x at terms[0]
        if let Some(arg_u) = ExpTerm::get_direct_exp_arg(&terms[0])
            && arg_u == x
            && let Some(negated) = extract_negated_term(&terms[1])
            && let AstKind::Div(num, den) = &negated.kind
            && matches!(&num.kind, AstKind::Number(n) if (*n - 1.0).abs() < EPSILON)
            && let Some(arg_v) = ExpTerm::get_direct_exp_arg(den)
            && arg_v == x
        {
            return Some(x);
        }

        // Try e^x at terms[1] (commutative check)
        if let Some(arg_u) = ExpTerm::get_direct_exp_arg(&terms[1])
            && arg_u == x
            && let Some(negated) = extract_negated_term(&terms[0])
            && let AstKind::Div(num, den) = &negated.kind
            && matches!(&num.kind, AstKind::Number(n) if (*n - 1.0).abs() < EPSILON)
            && let Some(arg_v) = ExpTerm::get_direct_exp_arg(den)
            && arg_v == x
        {
            return Some(x);
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
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
        let is_one1 = matches!(&terms[1].kind, AstKind::Number(n) if *n == 1.0);
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
        let is_one0 = matches!(&terms[0].kind, AstKind::Number(n) if *n == 1.0);

        let exp_term = if is_one1 {
            &terms[0]
        } else if is_one0 {
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
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
                let is_two0 = matches!(&factors[0].kind, AstKind::Number(n) if *n == 2.0);
                if is_two0 {
                    return Some((*factors[1]).clone());
                }
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
                let is_two1 = matches!(&factors[1].kind, AstKind::Number(n) if *n == 2.0);
                if is_two1 {
                    return Some((*factors[0]).clone());
                }
            }
            // Check Poly(2*x)
            if let AstKind::Poly(p) = &exp_arg.kind
                && p.terms().len() == 1
                && p.terms()[0] == (1, 2.0)
            {
                return Some(p.base().clone());
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
        // Try to find -1
        let (neg_one, exp_term) = if matches!(&terms[1].kind, AstKind::Number(n) if (*n - -1.0).abs() < EPSILON)
        {
            (true, &terms[0])
        } else if matches!(&terms[0].kind, AstKind::Number(n) if (*n - -1.0).abs() < EPSILON) {
            (true, &terms[1])
        } else {
            (false, &terms[0])
        };

        if neg_one {
            // Check u = e^(2x)
            if let Some(exp_arg) = ExpTerm::get_direct_exp_arg(exp_term) {
                // exp_arg should be 2*x
                // Check Product([2, x])
                if let AstKind::Product(factors) = &exp_arg.kind
                    && factors.len() == 2
                {
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
                    let is_two0 = matches!(&factors[0].kind, AstKind::Number(n) if *n == 2.0);
                    if is_two0 {
                        return Some((*factors[1]).clone());
                    }
                    #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
                    let is_two1 = matches!(&factors[1].kind, AstKind::Number(n) if *n == 2.0);
                    if is_two1 {
                        return Some((*factors[0]).clone());
                    }
                }
                // Check Poly(2*x)
                if let AstKind::Poly(p) = &exp_arg.kind
                    && p.terms().len() == 1
                    && p.terms()[0] == (1, 2.0)
                {
                    return Some(p.base().clone());
                }
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
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
        let is_neg_one0 = matches!(&factors[0].kind, AstKind::Number(n) if *n == -1.0);
        if is_neg_one0 {
            return Some((*factors[1]).clone());
        }
        #[allow(clippy::float_cmp, reason = "Comparing against exact constant -1.0")]
        let is_neg_one1 = matches!(&factors[1].kind, AstKind::Number(n) if *n == -1.0);
        if is_neg_one1 {
            return Some((*factors[0]).clone());
        }
    }
    None
}
