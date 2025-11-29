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
        Expr::Add(u, v) => Expr::Add(
            Box::new(apply_algebraic_rules(*u)),
            Box::new(apply_algebraic_rules(*v)),
        ),
        Expr::Mul(u, v) => Expr::Mul(
            Box::new(apply_algebraic_rules(*u)),
            Box::new(apply_algebraic_rules(*v)),
        ),
        Expr::Sub(u, v) => Expr::Sub(
            Box::new(apply_algebraic_rules(*u)),
            Box::new(apply_algebraic_rules(*v)),
        ),
        Expr::Div(u, v) => Expr::Div(
            Box::new(apply_algebraic_rules(*u)),
            Box::new(apply_algebraic_rules(*v)),
        ),
        Expr::Pow(u, v) => Expr::Pow(
            Box::new(apply_algebraic_rules(*u)),
            Box::new(apply_algebraic_rules(*v)),
        ),
        Expr::FunctionCall { name, args } => Expr::FunctionCall {
            name,
            args: args.into_iter().map(apply_algebraic_rules).collect(),
        },
        other => other,
    };
    // Then apply rules to the current level
    match expr {
        Expr::Add(u, v) => {
            // a + (-b) = a - b
            if let Some(neg_term) = is_negative_term(&v) {
                return Expr::Sub(u, neg_term);
            }
            // (-b) + a = a - b
            if let Some(neg_term) = is_negative_term(&u) {
                return Expr::Sub(v, neg_term);
            }

            // Constant folding: number + number
            if let (Expr::Number(a), Expr::Number(b)) = (&*u, &*v) {
                return Expr::Number(a + b);
            }

            // Combine number + fraction: a + b/c = (a*c + b)/c
            if let (Expr::Number(a), Expr::Div(b, c)) = (&*u, &*v)
                && let (Expr::Number(b_val), Expr::Number(c_val)) = (&**b, &**c)
            {
                let numerator = a * c_val + b_val;
                return Expr::Div(
                    Box::new(Expr::Number(numerator)),
                    Box::new(Expr::Number(*c_val)),
                );
            }
            // Combine fraction + fraction with same denominator: a/c + b/c = (a+b)/c
            if let (Expr::Div(a, c1), Expr::Div(b, c2)) = (&*u, &*v)
                && c1 == c2
                && let (Expr::Number(a_val), Expr::Number(b_val)) = (&**a, &**b)
            {
                let numerator = a_val + b_val;
                return Expr::Div(Box::new(Expr::Number(numerator)), c1.clone());
            }

            // Combine fraction + fraction with different denominators: a/c + b/d = (a*d + b*c)/(c*d)
            if let (Expr::Div(a, c), Expr::Div(b, d)) = (&*u, &*v)
                && let (
                    Expr::Number(a_val),
                    Expr::Number(b_val),
                    Expr::Number(c_val),
                    Expr::Number(d_val),
                ) = (&**a, &**b, &**c, &**d)
            {
                let numerator = a_val * d_val + b_val * c_val;
                let denominator = c_val * d_val;
                return Expr::Div(
                    Box::new(Expr::Number(numerator)),
                    Box::new(Expr::Number(denominator)),
                );
            }

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
            // Constant folding: number * number
            if let (Expr::Number(a), Expr::Number(b)) = (&*u, &*v) {
                return Expr::Number(a * b);
            }

            // 1 * x = x, x * 1 = x
            if let Expr::Number(n) = &*u
                && *n == 1.0
            {
                return *v;
            }
            if let Expr::Number(n) = &*v
                && *n == 1.0
            {
                return *u;
            }

            // Flatten and sort
            let mut terms = flatten_mul(Expr::Mul(u, v));
            terms.sort_by(compare_expr);

            // Special case: a * (b / c) -> (a * b) / c
            let terms = try_flatten_mul_div(terms);

            // CRITICAL FIX: If try_flatten_mul_div created a single Div expression,
            // return it immediately without further processing to preserve the structure
            if terms.len() == 1
                && let Expr::Div(_, _) = &terms[0]
            {
                return terms[0].clone();
            }

            // Combine like terms (x * x = x^2)
            let combined_terms = combine_mul_terms(terms);

            // Combine same exponents (x^n * y^n = (x*y)^n)
            let combined_terms = combine_power_terms(combined_terms);

            // Rebuild
            rebuild_mul(combined_terms)
        }

        Expr::Sub(u, v) => {
            // Constant folding: number - number
            if let (Expr::Number(a), Expr::Number(b)) = (&*u, &*v) {
                return Expr::Number(a - b);
            }

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

            // x^n / y^n = (x/y)^n
            if let (Expr::Pow(base_u, exp_u), Expr::Pow(base_v, exp_v)) = (&*u, &*v)
                && exp_u == exp_v
            {
                return Expr::Pow(
                    Box::new(Expr::Div(base_u.clone(), base_v.clone())),
                    exp_u.clone(),
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
            // (x/y) / (z/a) = (x*a) / (y*z)
            if let Expr::Div(x, y) = &*u
                && let Expr::Div(z, a) = &*v
            {
                let new_num = Expr::Mul(x.clone(), a.clone());
                let new_denom = Expr::Mul(y.clone(), z.clone());
                return apply_algebraic_rules(Expr::Div(Box::new(new_num), Box::new(new_denom)));
            }

            // x / (y/z) = (x*z) / y
            if let Expr::Div(y, z) = &*v {
                let new_num = Expr::Mul(u.clone(), z.clone());
                let new_denom = y.clone();
                return apply_algebraic_rules(Expr::Div(Box::new(new_num), new_denom));
            }

            // (x/y) / z = x / (y*z)
            if let Expr::Div(x, y) = &*u {
                let new_num = x.clone();
                let new_denom = Expr::Mul(y.clone(), v.clone());
                return apply_algebraic_rules(Expr::Div(new_num, Box::new(new_denom)));
            }
            // Simplify fraction by canceling common factors
            // (a * b) / (a * c) -> b / c
            let (new_u, new_v) = simplify_fraction_factors(*u.clone(), *v.clone());
            if new_u != *u || new_v != *v {
                return apply_algebraic_rules(Expr::Div(Box::new(new_u), Box::new(new_v)));
            }

            Expr::Div(u, v)
        }

        Expr::Pow(u, v) => {
            // Constant folding: number ^ number
            if let (Expr::Number(a), Expr::Number(b)) = (&*u, &*v) {
                return Expr::Number(a.powf(*b));
            }

            // (x^a)^b = x^(a*b)
            if let Expr::Pow(base, exp_inner) = *u {
                return Expr::Pow(base, Box::new(Expr::Mul(exp_inner, v)));
            }

            // x^0 = 1 for x != 0
            if let Expr::Number(n) = *v
                && n == 0.0
            {
                return Expr::Number(1.0);
            }

            // x^1 = x
            if let Expr::Number(n) = *v
                && n == 1.0
            {
                return *u;
            }

            // x^-n = 1/x^n for n > 0
            if let Expr::Number(n) = *v
                && n < 0.0
            {
                let positive_exp = Expr::Number(-n);
                let denom = Expr::Pow(u, Box::new(positive_exp));
                return Expr::Div(Box::new(Expr::Number(1.0)), Box::new(denom));
            }

            // x^(-a/b) = 1/x^(a/b) for a > 0, b > 0
            if let Expr::Div(num, den) = &*v
                && let (Expr::Number(n), Expr::Number(d)) = (&**num, &**den)
                && *n < 0.0
                && *d > 0.0
            {
                let positive_exp = Expr::Div(Box::new(Expr::Number(-*n)), den.clone());
                let denom = Expr::Pow(u, Box::new(positive_exp));
                return Expr::Div(Box::new(Expr::Number(1.0)), Box::new(denom));
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

    // Helper: Normalize expression by sorting factors in multiplication
    fn normalize_expr(expr: Expr) -> Expr {
        match expr {
            Expr::Mul(u, v) => {
                let mut factors = flatten_mul(Expr::Mul(u, v));
                factors.sort_by(compare_expr);
                rebuild_mul(factors)
            }
            other => other,
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
                // Check if base is 1.0 (meaning it's just a number)
                if let Expr::Number(n) = &current_base {
                    if *n == 1.0 {
                        result.push(Expr::Number(current_coeff));
                    } else {
                        result.push(Expr::Mul(
                            Box::new(Expr::Number(current_coeff)),
                            Box::new(current_base),
                        ));
                    }
                } else {
                    result.push(Expr::Mul(
                        Box::new(Expr::Number(current_coeff)),
                        Box::new(current_base),
                    ));
                }
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
        // Check if base is 1.0
        if let Expr::Number(n) = &current_base {
            if *n == 1.0 {
                result.push(Expr::Number(current_coeff));
            } else {
                result.push(Expr::Mul(
                    Box::new(Expr::Number(current_coeff)),
                    Box::new(current_base),
                ));
            }
        } else {
            result.push(Expr::Mul(
                Box::new(Expr::Number(current_coeff)),
                Box::new(current_base),
            ));
        }
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

// Helper: Try to flatten multiplication with division
// a * (b / c) -> (a * b) / c
fn try_flatten_mul_div(terms: Vec<Expr>) -> Vec<Expr> {
    let mut result = Vec::new();
    let mut numerator_factors = Vec::new();
    let mut denominator_factors = Vec::new();

    for term in terms {
        match term {
            Expr::Div(num, den) => {
                // Extract factors from numerator and denominator
                let num_factors = flatten_mul(*num);
                let den_factors = flatten_mul(*den);
                numerator_factors.extend(num_factors);
                denominator_factors.extend(den_factors);
            }
            other => {
                numerator_factors.push(other);
            }
        }
    }

    // If we found divisions, reconstruct as division
    if !denominator_factors.is_empty() {
        // Build numerator: product of all numerator factors
        let numerator = if numerator_factors.len() == 1 {
            numerator_factors.into_iter().next().unwrap()
        } else {
            rebuild_mul(numerator_factors)
        };

        // Build denominator: product of all denominator factors
        let denominator = if denominator_factors.len() == 1 {
            denominator_factors.into_iter().next().unwrap()
        } else {
            rebuild_mul(denominator_factors)
        };

        result.push(Expr::Div(Box::new(numerator), Box::new(denominator)));
    } else {
        result = numerator_factors;
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
        // Sort a and b for canonical order
        let (first, second) = if compare_expr(&a, &b) == Ordering::Greater {
            (a, b)
        } else {
            (b, a)
        };
        return vec![Expr::Pow(
            Box::new(Expr::Add(Box::new(first), Box::new(second))),
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
            Expr::Number(n) => {
                if *n == 1.0 {
                    // 1 = 1^2
                    squares.push(Expr::Number(1.0));
                }
            }
            Expr::Mul(coeff, rest) => {
                // Check for 2 * (a * b)
                if let Expr::Number(n) = **coeff
                    && n == 2.0
                {
                    if let Expr::Mul(a, b) = &**rest {
                        cross_terms.push((*a.clone(), *b.clone()));
                    } else {
                        // 2 * a, so b = 1
                        cross_terms.push((*rest.clone(), Expr::Number(1.0)));
                    }
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
        let square_a = if *a == Expr::Number(1.0) {
            Expr::Number(1.0)
        } else {
            a.clone()
        };
        let square_b = if *b == Expr::Number(1.0) {
            Expr::Number(1.0)
        } else {
            b.clone()
        };
        if (squares.contains(&square_a) && squares.contains(&square_b))
            || (squares.contains(&square_b) && squares.contains(&square_a))
        {
            return Some((a.clone(), b.clone()));
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

fn is_negative_term(expr: &Expr) -> Option<Box<Expr>> {
    if let Expr::Mul(a, b) = expr
        && let Expr::Number(n) = **a
        && n == -1.0
    {
        return Some(b.clone());
    }
    None
}

// Helper: Combine terms with same exponent in multiplication (x^n * y^n -> (x*y)^n)
fn combine_power_terms(terms: Vec<Expr>) -> Vec<Expr> {
    if terms.len() < 2 {
        return terms;
    }

    // Group terms by exponent
    // Since Expr doesn't implement Hash, we can't use HashMap easily.
    // We'll use a simple O(N^2) approach or sort by exponent?
    // Sorting by exponent might be better.

    // We need to be careful not to combine things that shouldn't be combined implicitly
    // But x^n * y^n = (x*y)^n is generally valid.

    // Let's try to find groups of same exponent.
    // We will iterate and build a new list.

    let mut result = Vec::new();
    let mut processed = vec![false; terms.len()];

    for i in 0..terms.len() {
        if processed[i] {
            continue;
        }

        let term_i = &terms[i];
        let (base_i, exp_i) = match term_i {
            Expr::Pow(b, e) => (b.clone(), e.clone()),
            _ => (Box::new(term_i.clone()), Box::new(Expr::Number(1.0))),
        };

        // If exponent is 1.0, we usually don't want to group everything into (x*y*z)^1 unless requested?
        // Actually (x*y)^1 is just x*y, so it doesn't change structure much, but might reorder.
        // Let's skip if exponent is 1.0 to preserve canonical ordering of factors?
        // The user asked for x^n * y^n = (x*y)^n.
        if let Expr::Number(n) = *exp_i
            && n == 1.0
        {
            result.push(term_i.clone());
            processed[i] = true;
            continue;
        }

        let mut bases = vec![*base_i];
        processed[i] = true;

        for j in (i + 1)..terms.len() {
            if processed[j] {
                continue;
            }

            let term_j = &terms[j];
            let (base_j, exp_j) = match term_j {
                Expr::Pow(b, e) => (b.clone(), e.clone()),
                _ => (Box::new(term_j.clone()), Box::new(Expr::Number(1.0))),
            };

            if *exp_i == *exp_j {
                bases.push(*base_j);
                processed[j] = true;
            }
        }

        if bases.len() > 1 {
            // Create (b1 * b2 * ...)^n
            // We need to sort bases to ensure canonical form
            bases.sort_by(compare_expr);
            let combined_base = rebuild_mul(bases);
            // Recursively simplify the base? Maybe not needed if we trust inputs are simplified.
            // But rebuild_mul just creates Expr::Mul chain.
            result.push(Expr::Pow(Box::new(combined_base), exp_i));
        } else {
            result.push(term_i.clone());
        }
    }

    result
}

// Helper: Expand powers of products to enable factor cancellation in divisions
// (a*b)^n -> a^n * b^n
// This is applied ONLY in division simplification context to avoid conflicts
// CRITICAL: Non-recursive to avoid stack overflow
fn expand_pow_mul(expr: Expr) -> Expr {
    // Flatten to individual factors first
    let factors = flatten_mul(expr);

    // Expand any Pow(Mul(...)) factors
    let expanded_factors: Vec<Expr> = factors
        .into_iter()
        .flat_map(|factor| {
            if let Expr::Pow(base, exp) = factor {
                if let Expr::Mul(a, b) = *base {
                    // Expand (a*b)^n to [a^n, b^n]
                    // Recursively flatten the multiplication to handle (a*b*c)^n
                    let base_factors = flatten_mul(Expr::Mul(a, b));
                    base_factors
                        .into_iter()
                        .map(|f| Expr::Pow(Box::new(f), exp.clone()))
                        .collect()
                } else {
                    // Keep other powers as-is
                    vec![Expr::Pow(base, exp)]
                }
            } else {
                // Non-power factors pass through
                vec![factor]
            }
        })
        .collect();

    rebuild_mul(expanded_factors)
}

// Helper: Simplify fraction by canceling common factors
fn simplify_fraction_factors(numerator: Expr, denominator: Expr) -> (Expr, Expr) {
    // Keep original denominator to return if no cancellation happens
    // This prevents infinite loops where we expand (a*b)^n -> a^n * b^n,
    // nothing cancels, we return expanded form, and then combine_power_terms
    // puts it back to (a*b)^n, restarting the cycle.
    let original_denominator = denominator.clone();

    // CRITICAL: Expand powers of products in denominator to enable factor cancellation
    // (a*b)^n in denominator should become a^n * b^n so we can cancel individual factors
    let denominator_expanded = expand_pow_mul(denominator);

    let num_factors = flatten_mul(numerator);
    let den_factors = flatten_mul(denominator_expanded);

    let mut new_num_factors = Vec::new();
    let mut new_den_factors = den_factors;
    let mut any_cancellation = false;

    for num_factor in num_factors {
        let mut matched = false;
        for i in 0..new_den_factors.len() {
            // Exact match
            if num_factor == new_den_factors[i] {
                new_den_factors.remove(i);
                matched = true;
                any_cancellation = true;
                break;
            }

            // Power cancellation: x / x^n -> 1 / x^(n-1)
            if let Expr::Pow(base_d, exp_d) = &new_den_factors[i]
                && **base_d == num_factor
            {
                // x / x^n = 1 / x^(n-1)
                let new_exp = Expr::Sub(exp_d.clone(), Box::new(Expr::Number(1.0)));
                let new_den_term = Expr::Pow(base_d.clone(), Box::new(new_exp));
                new_den_factors[i] = new_den_term;
                matched = true;
                any_cancellation = true;
                break;
            }

            // Power cancellation: x^n / x -> x^(n-1) / 1
            if let Expr::Pow(base_n, exp_n) = &num_factor
                && **base_n == new_den_factors[i]
            {
                let new_exp = Expr::Sub(exp_n.clone(), Box::new(Expr::Number(1.0)));
                let new_num_term = Expr::Pow(base_n.clone(), Box::new(new_exp));
                new_num_factors.push(new_num_term);
                new_den_factors.remove(i);
                matched = true;
                any_cancellation = true;
                break;
            }

            // Power cancellation: x^n / x^m
            if let Expr::Pow(base_n, exp_n) = &num_factor
                && let Expr::Pow(base_d, exp_d) = &new_den_factors[i]
                && base_n == base_d
            {
                // We don't know which is larger, so let's just subtract from top
                // x^n / x^m = x^(n-m) / 1
                let new_exp = Expr::Sub(exp_n.clone(), exp_d.clone());
                let new_num_term = Expr::Pow(base_n.clone(), Box::new(new_exp));
                new_num_factors.push(new_num_term);
                new_den_factors.remove(i);
                matched = true;
                any_cancellation = true;
                break;
            }
        }

        if !matched {
            new_num_factors.push(num_factor);
        }
    }

    if !any_cancellation {
        // If no cancellation happened, return the rebuilt numerator and the ORIGINAL denominator
        // This avoids returning an expanded form that will just be recombined later
        return (rebuild_mul(new_num_factors), original_denominator);
    }

    (rebuild_mul(new_num_factors), rebuild_mul(new_den_factors))
}
