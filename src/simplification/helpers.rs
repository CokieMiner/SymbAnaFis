use crate::Expr;

// Extracts (name, arg) for pow of function: name(arg)^power
pub(crate) fn get_fn_pow_named(expr: &Expr, power: f64) -> Option<(&str, Expr)> {
    if let Expr::Pow(base, exp) = expr
        && matches!(**exp, Expr::Number(n) if n == power)
        && let Expr::FunctionCall { name, args } = &**base
        && args.len() == 1
    {
        return Some((name.as_str(), args[0].clone()));
    }
    None
}

// Generic helper to extract arguments from product of two function calls, order-insensitive
pub(crate) fn get_product_fn_args(expr: &Expr, fname1: &str, fname2: &str) -> Option<(Expr, Expr)> {
    if let Expr::Mul(lhs, rhs) = expr
        && let (
            Expr::FunctionCall { name: n1, args: a1 },
            Expr::FunctionCall { name: n2, args: a2 },
        ) = (&**lhs, &**rhs)
        && a1.len() == 1
        && a2.len() == 1
    {
        if n1 == fname1 && n2 == fname2 {
            return Some((a1[0].clone(), a2[0].clone()));
        }
        if n1 == fname2 && n2 == fname1 {
            return Some((a2[0].clone(), a1[0].clone()));
        }
    }
    None
}

// Floating point approx equality used for numeric pattern matching
pub(crate) fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-10
}

// Get numeric value from expression if it's a Number
pub(crate) fn get_numeric_value(expr: &Expr) -> f64 {
    if let Expr::Number(n) = expr {
        *n
    } else {
        f64::NAN
    }
}

// Trigonometric helpers
use std::f64::consts::PI;
pub(crate) fn is_multiple_of_two_pi(expr: &Expr) -> bool {
    if let Expr::Number(n) = expr {
        let two_pi = 2.0 * PI;
        let k = n / two_pi;
        return approx_eq(k, k.round());
    }
    // Handle n * pi
    if let Expr::Mul(lhs, rhs) = expr {
        if let (Expr::Number(n), Expr::Symbol(s)) = (&**lhs, &**rhs)
            && s == "pi"
            && n % 2.0 == 0.0
        {
            return true;
        }
        if let (Expr::Symbol(s), Expr::Number(n)) = (&**lhs, &**rhs)
            && s == "pi"
            && n % 2.0 == 0.0
        {
            return true;
        }
    }
    false
}

pub(crate) fn is_pi(expr: &Expr) -> bool {
    if let Expr::Number(n) = expr {
        return (n - PI).abs() < 1e-10;
    }
    false
}

pub(crate) fn is_three_pi_over_two(expr: &Expr) -> bool {
    if let Expr::Number(n) = expr {
        return (n - 3.0 * PI / 2.0).abs() < 1e-10;
    }
    false
}

/// Flatten nested multiplication into a list of factors
pub(crate) fn flatten_mul(expr: &Expr) -> Vec<Expr> {
    let mut factors = Vec::new();
    let mut stack = vec![expr.clone()];

    while let Some(current) = stack.pop() {
        if let Expr::Mul(a, b) = current {
            stack.push(*b);
            stack.push(*a);
        } else {
            factors.push(current);
        }
    }
    factors
}

/// Compare expressions for canonical ordering
/// Order: Number < Symbol < FunctionCall < Add < Sub < Mul < Div < Pow
pub(crate) fn compare_expr(a: &Expr, b: &Expr) -> std::cmp::Ordering {
    use crate::Expr::*;
    use std::cmp::Ordering;

    match (a, b) {
        (Number(n1), Number(n2)) => n1.partial_cmp(n2).unwrap_or(Ordering::Equal),
        (Number(_), _) => Ordering::Less,
        (_, Number(_)) => Ordering::Greater,

        (Symbol(s1), Symbol(s2)) => s1.cmp(s2),
        (Symbol(_), _) => Ordering::Less,
        (_, Symbol(_)) => Ordering::Greater,

        (FunctionCall { name: n1, args: a1 }, FunctionCall { name: n2, args: a2 }) => {
            match n1.cmp(n2) {
                Ordering::Equal => {
                    for (arg1, arg2) in a1.iter().zip(a2.iter()) {
                        match compare_expr(arg1, arg2) {
                            Ordering::Equal => continue,
                            ord => return ord,
                        }
                    }
                    a1.len().cmp(&a2.len())
                }
                ord => ord,
            }
        }
        (FunctionCall { .. }, _) => Ordering::Less,
        (_, FunctionCall { .. }) => Ordering::Greater,

        // For other types, just use variant order roughly
        (Add(..), Add(..)) => Ordering::Equal, // Deep comparison too expensive?
        (Add(..), _) => Ordering::Less,
        (_, Add(..)) => Ordering::Greater,

        (Sub(..), Sub(..)) => Ordering::Equal,
        (Sub(..), _) => Ordering::Less,
        (_, Sub(..)) => Ordering::Greater,

        (Mul(..), Mul(..)) => Ordering::Equal,
        (Mul(..), _) => Ordering::Less,
        (_, Mul(..)) => Ordering::Greater,

        (Div(..), Div(..)) => Ordering::Equal,
        (Div(..), _) => Ordering::Less,
        (_, Div(..)) => Ordering::Greater,

        (Pow(..), Pow(..)) => Ordering::Equal,
    }
}

/// Helper: Flatten nested additions
pub(crate) fn flatten_add(expr: Expr) -> Vec<Expr> {
    match expr {
        Expr::Add(l, r) => {
            let mut terms = flatten_add(*l);
            terms.extend(flatten_add(*r));
            terms
        }
        Expr::Sub(l, r) => {
            // a - b becomes [a, -1*b]
            let mut terms = flatten_add(*l);
            // Negate each term from the right side
            for term in flatten_add(*r) {
                terms.push(negate_term(term));
            }
            terms
        }
        _ => vec![expr],
    }
}

/// Helper: Negate a term (multiply by -1, or simplify if already negative)
fn negate_term(expr: Expr) -> Expr {
    match expr {
        Expr::Number(n) => Expr::Number(-n),
        Expr::Mul(l, r) => {
            // Check if already has -1 coefficient
            if let Expr::Number(n) = *l {
                if n == -1.0 {
                    return *r; // -1 * x becomes x when negated
                }
                return Expr::Mul(Box::new(Expr::Number(-n)), r);
            }
            if let Expr::Number(n) = *r {
                if n == -1.0 {
                    return *l;
                }
                return Expr::Mul(l, Box::new(Expr::Number(-n)));
            }
            Expr::Mul(Box::new(Expr::Number(-1.0)), Box::new(Expr::Mul(l, r)))
        }
        other => Expr::Mul(Box::new(Expr::Number(-1.0)), Box::new(other)),
    }
}

/// Helper: Rebuild addition tree (left-associative)
pub(crate) fn rebuild_add(terms: Vec<Expr>) -> Expr {
    if terms.is_empty() {
        return Expr::Number(0.0);
    }
    let mut iter = terms.into_iter();
    let mut result = iter.next().unwrap();
    for term in iter {
        result = Expr::Add(Box::new(result), Box::new(term));
    }
    result
}

/// Helper: Rebuild multiplication tree
pub(crate) fn rebuild_mul(terms: Vec<Expr>) -> Expr {
    if terms.is_empty() {
        return Expr::Number(1.0);
    }
    let mut iter = terms.into_iter();
    let mut result = iter.next().unwrap();
    for term in iter {
        result = Expr::Mul(Box::new(result), Box::new(term));
    }
    result
}

/// Helper: Normalize expression by sorting factors in multiplication
pub(crate) fn normalize_expr(expr: Expr) -> Expr {
    match expr {
        Expr::Mul(u, v) => {
            let mut factors = flatten_mul(&Expr::Mul(u, v));
            factors.sort_by(compare_expr);
            rebuild_mul(factors)
        }
        other => other,
    }
}

/// Helper to extract coefficient and base
/// Returns (coefficient, base_expr)
/// e.g. 2*x -> (2.0, x)
///      x   -> (1.0, x)
pub(crate) fn extract_coeff(expr: &Expr) -> (f64, Expr) {
    let flattened = flatten_mul(expr);
    let mut coeff = 1.0;
    let mut non_num = Vec::new();
    for term in flattened {
        if let Expr::Number(n) = term {
            coeff *= n;
        } else {
            non_num.push(term);
        }
    }
    let base = if non_num.is_empty() {
        Expr::Number(1.0)
    } else if non_num.len() == 1 {
        non_num[0].clone()
    } else {
        rebuild_mul(non_num)
    };
    (coeff, normalize_expr(base))
}

/// Convert fractional powers back to roots for display
/// x^(1/2) -> sqrt(x)
/// x^(1/3) -> cbrt(x)
pub(crate) fn prettify_roots(expr: Expr) -> Expr {
    match expr {
        Expr::Pow(base, exp) => {
            let base = prettify_roots(*base);
            let exp = prettify_roots(*exp);

            // x^(1/2) -> sqrt(x)
            if let Expr::Div(num, den) = &exp
                && matches!(**num, Expr::Number(n) if n == 1.0)
                && matches!(**den, Expr::Number(n) if n == 2.0)
            {
                return Expr::FunctionCall {
                    name: "sqrt".to_string(),
                    args: vec![base],
                };
            }
            // x^0.5 -> sqrt(x)
            if let Expr::Number(n) = &exp
                && (n - 0.5).abs() < 1e-10
            {
                return Expr::FunctionCall {
                    name: "sqrt".to_string(),
                    args: vec![base],
                };
            }

            // Note: x^-0.5 is NOT converted to 1/sqrt(x) because that would
            // interfere with fraction consolidation rules. The NegativeExponentToFractionRule
            // handles x^(-n) -> 1/x^n, then prettify_roots converts x^(1/2) -> sqrt(x).

            // x^(1/3) -> cbrt(x)
            if let Expr::Div(num, den) = &exp
                && matches!(**num, Expr::Number(n) if n == 1.0)
                && matches!(**den, Expr::Number(n) if n == 3.0)
            {
                return Expr::FunctionCall {
                    name: "cbrt".to_string(),
                    args: vec![base],
                };
            }

            Expr::Pow(Box::new(base), Box::new(exp))
        }
        // Recursively prettify subexpressions
        Expr::Add(u, v) => Expr::Add(Box::new(prettify_roots(*u)), Box::new(prettify_roots(*v))),
        Expr::Sub(u, v) => Expr::Sub(Box::new(prettify_roots(*u)), Box::new(prettify_roots(*v))),
        Expr::Mul(u, v) => Expr::Mul(Box::new(prettify_roots(*u)), Box::new(prettify_roots(*v))),
        Expr::Div(u, v) => Expr::Div(Box::new(prettify_roots(*u)), Box::new(prettify_roots(*v))),
        Expr::FunctionCall { name, args } => Expr::FunctionCall {
            name,
            args: args.into_iter().map(prettify_roots).collect(),
        },
        _ => expr,
    }
}

/// Check if an expression is known to be non-negative for all real values of its variables.
/// This is a conservative check - returns true only when we can prove non-negativity.
pub(crate) fn is_known_non_negative(expr: &Expr) -> bool {
    match expr {
        // Positive numbers
        Expr::Number(n) => *n >= 0.0,

        // x^2, x^4, x^6, ... are always non-negative
        Expr::Pow(_, exp) => {
            if let Expr::Number(n) = &**exp {
                // Even positive integer exponents
                *n > 0.0 && n.fract() == 0.0 && (*n as i64) % 2 == 0
            } else {
                false
            }
        }

        // abs(x) is always non-negative
        Expr::FunctionCall { name, args } if args.len() == 1 => {
            match name.as_str() {
                "abs" | "Abs" => true,
                // exp(x) is always positive
                "exp" => true,
                // cosh(x) >= 1 for all real x
                "cosh" => true,
                // sqrt, cbrt of non-negative is non-negative (but we can't always prove input is non-negative)
                "sqrt" => is_known_non_negative(&args[0]),
                _ => false,
            }
        }

        // Product of two non-negatives is non-negative
        Expr::Mul(a, b) => is_known_non_negative(a) && is_known_non_negative(b),

        // Sum of two non-negatives is non-negative
        Expr::Add(a, b) => is_known_non_negative(a) && is_known_non_negative(b),

        // Division of non-negative by positive is non-negative (but we can't easily check "positive")
        // Be conservative here
        _ => false,
    }
}

/// Check if an exponent represents a fractional power that requires non-negative base
/// (i.e., exponents like 1/2, 1/4, 3/2, etc. where denominator is even)
pub(crate) fn is_fractional_root_exponent(expr: &Expr) -> bool {
    match expr {
        // Direct fraction: 1/2, 1/4, 3/4, etc.
        Expr::Div(_, den) => {
            if let Expr::Number(d) = &**den {
                // Check if denominator is an even integer
                d.fract() == 0.0 && (*d as i64) % 2 == 0
            } else {
                // Can't determine, be conservative
                false
            }
        }
        // Decimal like 0.5
        Expr::Number(n) => {
            // Check if it's a fractional power (not an integer)
            // For 0.5, 0.25, 1.5, etc. - these involve even roots
            if n.fract() != 0.0 {
                // Check if it's k/2^n for some integers
                // Simple check: 0.5 = 1/2, 0.25 = 1/4, 0.75 = 3/4, etc.
                let doubled = *n * 2.0;
                doubled.fract() == 0.0 // If 2*n is integer, then n = k/2
            } else {
                false
            }
        }
        _ => false,
    }
}
