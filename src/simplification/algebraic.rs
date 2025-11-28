use crate::Expr;
use std::cmp::Ordering;

/// Apply algebraic simplification rules
/// - Canonical ordering of multiplication/addition terms
/// - Cancellation: x - x = 0, x / x = 1
/// - Exponent simplification: x^n / x^m = x^(n-m)
/// - Factoring: x*y + x*z = x*(y+z)
/// - Perfect squares: x^2 + 2*x*y + y^2 = (x+y)^2
pub fn apply_algebraic_rules(expr: Expr) -> Expr {
    // First recursively simplify subexpressions
    let expr = match expr {
        Expr::Add(u, v) => Expr::Add(Box::new(apply_algebraic_rules(*u)), Box::new(apply_algebraic_rules(*v))),
        Expr::Mul(u, v) => Expr::Mul(Box::new(apply_algebraic_rules(*u)), Box::new(apply_algebraic_rules(*v))),
        Expr::Sub(u, v) => Expr::Sub(Box::new(apply_algebraic_rules(*u)), Box::new(apply_algebraic_rules(*v))),
        Expr::Div(u, v) => Expr::Div(Box::new(apply_algebraic_rules(*u)), Box::new(apply_algebraic_rules(*v))),
        Expr::Pow(u, v) => Expr::Pow(Box::new(apply_algebraic_rules(*u)), Box::new(apply_algebraic_rules(*v))),
        Expr::FunctionCall { name, args } => Expr::FunctionCall {
            name,
            args: args.into_iter().map(|arg| apply_algebraic_rules(arg)).collect(),
        },
        other => other,
    };

    // Then apply rules to the current level
    match expr {
        Expr::Add(u, v) => {
            // Flatten and sort for canonical ordering
            let mut terms = flatten_add(Expr::Add(u, v));
            terms.sort_by(compare_expr);

            // Try to factor out common terms
            let factored_terms = try_factor_common_terms(terms);

            // Combine like terms (x + x = 2x)
            let combined_terms = combine_add_terms(factored_terms);

            // Rebuild tree
            rebuild_add(combined_terms)
        }

        Expr::Mul(u, v) => {
            // Flatten and sort
            let mut terms = flatten_mul(Expr::Mul(u, v));
            terms.sort_by(compare_expr);

            // Combine like terms (x * x = x^2)
            let combined_terms = combine_mul_terms(terms);

            // Rebuild
            rebuild_mul(combined_terms)
        }

        Expr::Sub(u, v) => {
            // Convert x - y to x + (-1)*y so that addition rules can combine like terms
            let neg_v = Expr::Mul(Box::new(Expr::Number(-1.0)), v);
            apply_algebraic_rules(Expr::Add(u, Box::new(neg_v)))
        }

        Expr::Div(u, v) => {
            // x / x = 1
            if u == v {
                return Expr::Number(1.0);
            }

            // c * x^n / x^m = c * x^(n-m)
            if let Expr::Mul(coeff, power) = &*u
                && let Expr::Pow(base_v, exp_v) = &*v
                && let Expr::Pow(base_p, exp_p) = &**power
                && base_p == base_v
            {
                let new_exp = Expr::Sub(exp_p.clone(), exp_v.clone());
                let new_power = Expr::Pow(base_v.clone(), Box::new(new_exp));
                return Expr::Mul(coeff.clone(), Box::new(new_power));
            }

            // x^n / x^m = x^(n-m)
            if let (Expr::Pow(base_u, exp_u), Expr::Pow(base_v, exp_v)) = (&*u, &*v)
                && base_u == base_v
            {
                return Expr::Pow(
                    base_u.clone(),
                    Box::new(Expr::Sub(exp_u.clone(), exp_v.clone())),
                );
            }

            // x^n / x = x^(n-1)
            if let Expr::Pow(base_u, exp_u) = &*u
                && base_u == &v
            {
                return Expr::Pow(
                    base_u.clone(),
                    Box::new(Expr::Sub(exp_u.clone(), Box::new(Expr::Number(1.0)))),
                );
            }

            // x / x^n = x^(1-n)
            if let Expr::Pow(base_v, exp_v) = &*v
                && &u == base_v
            {
                return Expr::Pow(
                    base_v.clone(),
                    Box::new(Expr::Sub(Box::new(Expr::Number(1.0)), exp_v.clone())),
                );
            }

            Expr::Div(u, v)
        }

        Expr::Pow(u, v) => {
            // (x^a)^b = x^(a*b)
            if let Expr::Pow(base, exp_inner) = *u {
                return Expr::Pow(base, Box::new(Expr::Mul(exp_inner, v)));
            }
            
            // x^-n = 1/x^n for n > 0
            if let Expr::Number(n) = *v {
                if n < 0.0 {
                    let positive_exp = Expr::Number(-n);
                    let denom = Expr::Pow(u, Box::new(positive_exp));
                    return Expr::Div(Box::new(Expr::Number(1.0)), Box::new(denom));
                }
            }
            
            Expr::Pow(u, v)
        }

        _ => expr,
    }
}

// Helper: Combine like terms in addition (x + x -> 2x, 2x + 3x -> 5x)
fn combine_add_terms(terms: Vec<Expr>) -> Vec<Expr> {
    if terms.is_empty() {
        return terms;
    }

    let mut result = Vec::new();
    let mut iter = terms.into_iter();

    // Helper to extract coefficient and base
    // Returns (coefficient, base_expr)
    // e.g. 2*x -> (2.0, x)
    //      x   -> (1.0, x)
    fn extract_coeff(expr: Expr) -> (f64, Expr) {
        match expr {
            Expr::Mul(l, r) => {
                if let Expr::Number(n) = *l {
                    (n, *r)
                } else {
                    (1.0, Expr::Mul(l, r))
                }
            }
            other => (1.0, other),
        }
    }

    let first = iter.next().unwrap();
    let (mut current_coeff, mut current_base) = extract_coeff(first);

    for term in iter {
        let (coeff, base) = extract_coeff(term);

        if base == current_base {
            current_coeff += coeff;
        } else {
            // Push current
            if current_coeff == 0.0 {
                // Drop term (0 * x = 0)
                // But if it's the only term? handled by rebuild_add returning 0
            } else if current_coeff == 1.0 {
                result.push(current_base);
            } else {
                result.push(Expr::Mul(
                    Box::new(Expr::Number(current_coeff)),
                    Box::new(current_base),
                ));
            }

            current_coeff = coeff;
            current_base = base;
        }
    }

    // Push last
    if current_coeff == 0.0 {
        // Drop
    } else if current_coeff == 1.0 {
        result.push(current_base);
    } else {
        result.push(Expr::Mul(
            Box::new(Expr::Number(current_coeff)),
            Box::new(current_base),
        ));
    }

    // If all terms canceled out, return 0
    if result.is_empty() {
        return vec![Expr::Number(0.0)];
    }

    result
}

// Helper: Combine like factors in multiplication (x * x -> x^2)
fn combine_mul_terms(terms: Vec<Expr>) -> Vec<Expr> {
    if terms.is_empty() {
        return vec![Expr::Number(1.0)];
    }

    // Remove factors of 1 and handle 0
    let mut filtered_terms = Vec::new();
    for term in terms {
        match term {
            Expr::Number(n) => {
                if n == 0.0 {
                    // Anything times 0 is 0
                    return vec![Expr::Number(0.0)];
                } else if n == 1.0 {
                    // Skip 1
                    continue;
                } else {
                    filtered_terms.push(Expr::Number(n));
                }
            }
            other => filtered_terms.push(other),
        }
    }

    if filtered_terms.is_empty() {
        return vec![Expr::Number(1.0)];
    }

    let mut result = Vec::new();
    let mut iter = filtered_terms.into_iter();
    let mut current_base = iter.next().unwrap();
    let mut current_exp = Expr::Number(1.0);

    for term in iter {
        // Check if term is same base (x * x)
        if term == current_base {
            current_exp = Expr::Add(Box::new(current_exp), Box::new(Expr::Number(1.0)));
            continue;
        }

        // Check if term is power with same base (x * x^n)
        // We need to check carefully to avoid borrow checker issues
        let mut merged = false;
        if let Expr::Pow(base, exp) = &term
            && **base == current_base
        {
            current_exp = Expr::Add(Box::new(current_exp), exp.clone());
            merged = true;
        }
        if merged {
            continue;
        }

        // Check if current is power and term is same base (x^n * x)
        if let Expr::Pow(base, exp) = &current_base
            && **base == term
        {
            let new_base = *base.clone();
            let new_exp = Expr::Add(exp.clone(), Box::new(Expr::Number(1.0)));
            current_base = new_base;
            current_exp = new_exp;
            merged = true;
        }
        if merged {
            continue;
        }

        // Check if both are powers with same base (x^n * x^m)
        if let Expr::Pow(b1, e1) = &current_base
            && let Expr::Pow(b2, e2) = &term
            && b1 == b2
        {
            let new_base = *b1.clone();
            let new_exp = Expr::Add(e1.clone(), e2.clone());
            current_base = new_base;
            current_exp = new_exp;
            merged = true;
        }
        if merged {
            continue;
        }

        // Push current and move to next
        if matches!(current_exp, Expr::Number(n) if n == 1.0) {
            result.push(current_base);
        } else {
            result.push(Expr::Pow(Box::new(current_base), Box::new(current_exp)));
        }

        current_base = term;
        current_exp = Expr::Number(1.0);
    }

    if matches!(current_exp, Expr::Number(n) if n == 1.0) {
        result.push(current_base);
    } else {
        result.push(Expr::Pow(Box::new(current_base), Box::new(current_exp)));
    }

    result
}

// Helper: Flatten nested additions
fn flatten_add(expr: Expr) -> Vec<Expr> {
    match expr {
        Expr::Add(l, r) => {
            let mut terms = flatten_add(*l);
            terms.extend(flatten_add(*r));
            terms
        }
        _ => vec![expr],
    }
}

// Helper: Rebuild addition tree (left-associative)
fn rebuild_add(terms: Vec<Expr>) -> Expr {
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

// Helper: Flatten nested multiplications
fn flatten_mul(expr: Expr) -> Vec<Expr> {
    match expr {
        Expr::Mul(l, r) => {
            let mut terms = flatten_mul(*l);
            terms.extend(flatten_mul(*r));
            terms
        }
        _ => vec![expr],
    }
}

// Helper: Rebuild multiplication tree
fn rebuild_mul(terms: Vec<Expr>) -> Expr {
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

// Helper: Try to factor out common terms from a sum
fn try_factor_common_terms(terms: Vec<Expr>) -> Vec<Expr> {
    if terms.len() < 2 {
        return terms;
    }

    // Check for perfect square: a^2 + 2*a*b + b^2 -> (a+b)^2
    if terms.len() == 3
        && let Some((a, b)) = is_perfect_square(&terms)
    {
        return vec![Expr::Pow(
            Box::new(Expr::Add(Box::new(a), Box::new(b))),
            Box::new(Expr::Number(2.0)),
        )];
    }

    // Simple case: if all terms are multiplications with the same first factor
    // e.g., x*y + x*z -> x*(y+z)
    let mut common_factor = None;
    let mut remaining_factors = Vec::new();

    for term in &terms {
        match term {
            Expr::Mul(a, b) => {
                let a_expr = (**a).clone();
                let b_expr = (**b).clone();
                if let Some(cf) = &common_factor {
                    if a_expr == *cf {
                        remaining_factors.push(b_expr);
                    } else {
                        // Not all terms have the same common factor
                        return terms;
                    }
                } else {
                    common_factor = Some(a_expr);
                    remaining_factors.push(b_expr);
                }
            }
            _ => {
                // If any term is not a multiplication, can't factor
                return terms;
            }
        }
    }

    // If we found a common factor, reconstruct as factor * (sum of remaining factors)
    if let Some(factor) = common_factor
        && remaining_factors.len() > 1
    {
        // Build the sum of remaining factors
        let mut sum = remaining_factors[0].clone();
        for factor in &remaining_factors[1..] {
            sum = Expr::Add(Box::new(sum), Box::new(factor.clone()));
        }
        return vec![Expr::Mul(Box::new(factor), Box::new(sum))];
    }

    terms
}

// Helper: Check if terms represent a perfect square: a^2 + 2*a*b + b^2
fn is_perfect_square(terms: &[Expr]) -> Option<(Expr, Expr)> {
    if terms.len() != 3 {
        return None;
    }

    // Extract coefficients and bases
    let mut squares = Vec::new();
    let mut cross_terms = Vec::new();

    for term in terms {
        match term {
            Expr::Pow(base, exp) => {
                if let Expr::Number(n) = **exp
                    && n == 2.0
                {
                    squares.push(*base.clone());
                }
            }
            Expr::Mul(coeff, rest) => {
                // Check for 2 * (a * b)
                if let Expr::Number(n) = **coeff
                    && n == 2.0
                    && let Expr::Mul(a, b) = &**rest
                {
                    cross_terms.push((*a.clone(), *b.clone()));
                }
                // Check for (2 * a) * b
                else if let Expr::Mul(inner_coeff, a) = &**coeff
                    && let Expr::Number(n) = &**inner_coeff
                    && *n == 2.0
                {
                    cross_terms.push((*a.clone(), *rest.clone()));
                }
            }
            _ => {}
        }
    }

    // Should have 2 squares and 1 cross term
    if squares.len() == 2 && cross_terms.len() == 1 {
        let (a, b) = &cross_terms[0];
        // Check if the squares match the cross term variables
        if (squares[0] == *a && squares[1] == *b) || (squares[0] == *b && squares[1] == *a) {
            return Some((squares[0].clone(), squares[1].clone()));
        }
    }

    None
}

fn compare_expr(a: &Expr, b: &Expr) -> Ordering {
    // Priority: Number < Symbol < Function < Add < Sub < Mul < Div < Pow
    let type_score = |e: &Expr| match e {
        Expr::Number(_) => 0,
        Expr::Symbol(_) => 1,
        Expr::FunctionCall { .. } => 2,
        Expr::Add(_, _) => 3,
        Expr::Sub(_, _) => 4,
        Expr::Mul(_, _) => 5,
        Expr::Div(_, _) => 6,
        Expr::Pow(_, _) => 7,
    };

    let score_a = type_score(a);
    let score_b = type_score(b);

    if score_a != score_b {
        return score_a.cmp(&score_b);
    }

    // If types match, compare content
    match (a, b) {
        (Expr::Number(n1), Expr::Number(n2)) => n1.partial_cmp(n2).unwrap_or(Ordering::Equal),
        (Expr::Symbol(s1), Expr::Symbol(s2)) => s1.cmp(s2),
        (
            Expr::FunctionCall {
                name: n1,
                args: args1,
            },
            Expr::FunctionCall {
                name: n2,
                args: args2,
            },
        ) => match n1.cmp(n2) {
            Ordering::Equal => match args1.len().cmp(&args2.len()) {
                Ordering::Equal => {
                    for (a1, a2) in args1.iter().zip(args2.iter()) {
                        match compare_expr(a1, a2) {
                            Ordering::Equal => continue,
                            ord => return ord,
                        }
                    }
                    Ordering::Equal
                }
                ord => ord,
            },
            ord => ord,
        },
        // For binary ops, compare left then right
        (Expr::Add(l1, r1), Expr::Add(l2, r2)) => compare_binary(l1, r1, l2, r2),
        (Expr::Sub(l1, r1), Expr::Sub(l2, r2)) => compare_binary(l1, r1, l2, r2),
        (Expr::Mul(l1, r1), Expr::Mul(l2, r2)) => compare_binary(l1, r1, l2, r2),
        (Expr::Div(l1, r1), Expr::Div(l2, r2)) => compare_binary(l1, r1, l2, r2),
        (Expr::Pow(l1, r1), Expr::Pow(l2, r2)) => compare_binary(l1, r1, l2, r2),
        _ => Ordering::Equal, // Should be covered by type_score
    }
}

fn compare_binary(l1: &Expr, r1: &Expr, l2: &Expr, r2: &Expr) -> Ordering {
    match compare_expr(l1, l2) {
        Ordering::Equal => compare_expr(r1, r2),
        ord => ord,
    }
}
