//! Simplification helper functions and utilities
//!
//! Provides expression manipulation utilities: flattening, normalization,
//! coefficient extraction, root prettification, and like-term grouping.

use crate::{Expr, ExprKind};
use std::sync::Arc;

// Extracts (name, arg) for pow of function: name(arg)^power
pub(crate) fn get_fn_pow_named(expr: &Expr, power: f64) -> Option<(&str, Expr)> {
    if let ExprKind::Pow(base, exp) = &expr.kind
        && matches!(exp.kind, ExprKind::Number(n) if n == power)
        && let ExprKind::FunctionCall { name, args } = &base.kind
        && args.len() == 1
    {
        return Some((name.as_str(), args[0].clone()));
    }
    None
}

// Floating point approx equality used for numeric pattern matching
pub(crate) fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-10
}

// Get numeric value from expression if it's a Number
pub(crate) fn get_numeric_value(expr: &Expr) -> f64 {
    if let ExprKind::Number(n) = &expr.kind {
        *n
    } else {
        f64::NAN
    }
}

// Trigonometric helpers
use std::f64::consts::PI;
pub(crate) fn is_multiple_of_two_pi(expr: &Expr) -> bool {
    if let ExprKind::Number(n) = &expr.kind {
        let two_pi = 2.0 * PI;
        let k = n / two_pi;
        return approx_eq(k, k.round());
    }
    // Handle n * pi (Product with number and pi symbol)
    if let ExprKind::Product(factors) = &expr.kind {
        let mut num_coeff: Option<f64> = None;
        let mut has_pi = false;

        for f in factors {
            match &f.kind {
                ExprKind::Number(n) => num_coeff = Some(num_coeff.unwrap_or(1.0) * n),
                ExprKind::Symbol(s) if s == "pi" => has_pi = true,
                _ => return false,
            }
        }

        if has_pi {
            if let Some(n) = num_coeff {
                return n % 2.0 == 0.0;
            }
        }
    }
    false
}

pub(crate) fn is_pi(expr: &Expr) -> bool {
    if let ExprKind::Number(n) = &expr.kind {
        return (n - PI).abs() < 1e-10;
    }
    false
}

pub(crate) fn is_three_pi_over_two(expr: &Expr) -> bool {
    if let ExprKind::Number(n) = &expr.kind {
        return (n - 3.0 * PI / 2.0).abs() < 1e-10;
    }
    false
}

/// Flatten nested multiplication into a list of factors
/// Uses references during traversal, only clones leaf nodes
pub(crate) fn flatten_mul(expr: &Expr) -> Vec<Expr> {
    // For n-ary Product, simply return the factors
    if let ExprKind::Product(factors) = &expr.kind {
        return factors.iter().map(|f| (**f).clone()).collect();
    }
    // For non-products, return the expression itself as a single factor
    vec![expr.clone()]
}

/// Compare expressions for canonical polynomial ordering
/// For polynomial form like: ax^n + bx^(n-1) + ... + cx + d
///
/// For ADDITION terms (polynomial ordering):
/// - Higher degree terms first: x^2 before x before constants
/// - Then alphabetically by variable name
///
/// For MULTIPLICATION factors (coefficient ordering):
/// - Numbers (coefficients) first: 2*x not x*2  
/// - Then symbols alphabetically
/// - Then more complex expressions
pub(crate) fn compare_expr(a: &Expr, b: &Expr) -> std::cmp::Ordering {
    use crate::ExprKind::*;
    use std::cmp::Ordering;

    // Helper to get polynomial degree for addition term ordering
    fn get_poly_degree(e: &Expr) -> (i32, f64, String) {
        // Returns (type_priority, exponent/degree, variable_name)
        // Lower type_priority = comes first in sorted order for polynomial terms
        match &e.kind {
            Number(_) => (100, 0.0, String::new()), // Constants come last
            Symbol(s) => (50, -1.0, s.to_string()), // Variables are degree 1, negative so they sort first
            Pow(base, exp) => {
                if let Symbol(s) = &base.kind {
                    if let Number(n) = &exp.kind {
                        (10, -*n, s.to_string()) // Negative exponent so higher powers sort first
                    } else {
                        (20, -999.0, s.to_string()) // Symbolic exponent - treat as very high degree
                    }
                } else {
                    (30, 0.0, String::new()) // Non-symbol base
                }
            }
            Product(factors) => {
                // For c*x^n or c*x, extract the variable part's degree
                let mut best: Option<(i32, f64, String)> = None;
                for f in factors {
                    let info = get_poly_degree(f);
                    if best.is_none() || info.0 < best.as_ref().unwrap().0 {
                        best = Some(info);
                    }
                }
                match best {
                    Some((t, e, v)) => (15.min(t), e, v),
                    None => (15, 0.0, String::new()),
                }
            }
            FunctionCall { name, .. } => (60, 0.0, name.to_string()),
            Sum(..) => (70, 0.0, String::new()),
            Div(..) => (40, 0.0, String::new()),
            Derivative { var, .. } => (65, 0.0, var.clone()),
        }
    }

    let (type_a, exp_a, var_a) = get_poly_degree(a);
    let (type_b, exp_b, var_b) = get_poly_degree(b);

    // First compare by type priority (lower = comes first)
    match type_a.cmp(&type_b) {
        Ordering::Equal => {}
        ord => return ord,
    }

    // Then by exponent (for Pow and Mul with variables)
    match exp_a.partial_cmp(&exp_b).unwrap_or(Ordering::Equal) {
        Ordering::Equal => {}
        ord => return ord,
    }

    // Then alphabetically by variable name
    match var_a.cmp(&var_b) {
        Ordering::Equal => {}
        ord => return ord,
    }

    // For numbers, compare by value (smaller numbers first for coefficients)
    if let (Number(n1), Number(n2)) = (&a.kind, &b.kind) {
        return n1.partial_cmp(n2).unwrap_or(Ordering::Equal);
    }

    // For symbols, alphabetical order
    if let (Symbol(s1), Symbol(s2)) = (&a.kind, &b.kind) {
        return s1.cmp(s2);
    }

    Ordering::Equal
}

/// Compare expressions for multiplication factor ordering
/// Numbers (coefficients) come first, then symbols, then complex expressions
pub(crate) fn compare_mul_factors(a: &Expr, b: &Expr) -> std::cmp::Ordering {
    use crate::ExprKind::*;
    use std::cmp::Ordering;

    fn factor_priority(e: &Expr) -> i32 {
        match &e.kind {
            Number(_) => 0,  // Numbers first (coefficients)
            Symbol(_) => 10, // Then symbols
            Pow(base, _) => {
                if matches!(base.kind, Symbol(_)) {
                    20
                } else {
                    30
                }
            }
            FunctionCall { .. } => 40,
            Product(..) | Div(..) => 50,
            Sum(..) => 60,
            Derivative { .. } => 45, // After functions, before mul/div
        }
    }

    let prio_a = factor_priority(a);
    let prio_b = factor_priority(b);

    match prio_a.cmp(&prio_b) {
        Ordering::Equal => {
            // Within same priority, use compare_expr
            compare_expr(a, b)
        }
        ord => ord,
    }
}

/// Helper: Rebuild multiplication tree
pub(crate) fn rebuild_mul(terms: Vec<Expr>) -> Expr {
    if terms.is_empty() {
        return Expr::number(1.0);
    }
    let mut iter = terms.into_iter();
    let mut result = iter.next().unwrap();
    for term in iter {
        result = Expr::mul_expr(result, term);
    }
    result
}

/// Helper to extract coefficient and base
/// Returns (coefficient, base_expr)
/// e.g. 2*x -> (2.0, x)
///      x   -> (1.0, x)
pub(crate) fn extract_coeff(expr: &Expr) -> (f64, Expr) {
    match &expr.kind {
        ExprKind::Number(n) => (*n, Expr::number(1.0)),
        ExprKind::Product(factors) => {
            let mut coeff = 1.0;
            let mut non_num_arcs = Vec::with_capacity(factors.len());

            for f in factors {
                if let ExprKind::Number(n) = &f.kind {
                    coeff *= n;
                } else {
                    non_num_arcs.push(f.clone());
                }
            }

            let base = if non_num_arcs.is_empty() {
                Expr::number(1.0)
            } else if non_num_arcs.len() == 1 {
                // Unwrap Arc if possible or clone contents
                let arc = non_num_arcs.into_iter().next().unwrap();
                match Arc::try_unwrap(arc) {
                    Ok(e) => e,
                    Err(a) => (*a).clone(),
                }
            } else {
                Expr::product_from_arcs(non_num_arcs)
            };

            (coeff, base)
        }
        _ => (1.0, expr.clone()),
    }
}

/// Convert fractional powers back to roots for display
/// x^(1/2) -> sqrt(x)
/// x^(1/3) -> cbrt(x)
pub(crate) fn prettify_roots(expr: Expr) -> Expr {
    match expr.kind {
        ExprKind::Pow(base, exp) => {
            let base = prettify_roots(base.as_ref().clone());
            let exp = prettify_roots(exp.as_ref().clone());

            // x^(1/2) -> sqrt(x)
            if let ExprKind::Div(num, den) = &exp.kind
                && matches!(num.kind, ExprKind::Number(n) if n == 1.0)
                && matches!(den.kind, ExprKind::Number(n) if n == 2.0)
            {
                return Expr::func("sqrt", base);
            }
            // x^0.5 -> sqrt(x)
            if let ExprKind::Number(n) = &exp.kind
                && (n - 0.5).abs() < 1e-10
            {
                return Expr::func("sqrt", base);
            }

            // Note: x^-0.5 is NOT converted to 1/sqrt(x) because that would
            // interfere with fraction consolidation rules. The NegativeExponentToFractionRule
            // handles x^(-n) -> 1/x^n, then prettify_roots converts x^(1/2) -> sqrt(x).

            // x^(1/3) -> cbrt(x)
            if let ExprKind::Div(num, den) = &exp.kind
                && matches!(num.kind, ExprKind::Number(n) if n == 1.0)
                && matches!(den.kind, ExprKind::Number(n) if n == 3.0)
            {
                return Expr::func("cbrt", base);
            }

            Expr::pow(base, exp)
        }
        // Recursively prettify subexpressions
        ExprKind::Sum(terms) => Expr::sum(
            terms
                .iter()
                .map(|t| prettify_roots((**t).clone()))
                .collect(),
        ),
        ExprKind::Product(factors) => Expr::product(
            factors
                .iter()
                .map(|f| prettify_roots((**f).clone()))
                .collect(),
        ),
        ExprKind::Div(u, v) => Expr::div_expr(
            prettify_roots(u.as_ref().clone()),
            prettify_roots(v.as_ref().clone()),
        ),
        ExprKind::FunctionCall { name, args } => Expr::new(ExprKind::FunctionCall {
            name,
            args: args.into_iter().map(prettify_roots).collect(),
        }),
        _ => Expr::new(expr.kind), // Reconstruct
    }
}

/// Check if an expression is known to be non-negative for all real values of its variables.
/// This is a conservative check - returns true only when we can prove non-negativity.
pub(crate) fn is_known_non_negative(expr: &Expr) -> bool {
    match &expr.kind {
        // Positive numbers
        ExprKind::Number(n) => *n >= 0.0,

        // x^2, x^4, x^6, ... are always non-negative
        ExprKind::Pow(_, exp) => {
            if let ExprKind::Number(n) = &exp.kind {
                // Even positive integer exponents
                *n > 0.0 && n.fract() == 0.0 && (*n as i64) % 2 == 0
            } else {
                false
            }
        }

        // abs(x) is always non-negative
        ExprKind::FunctionCall { name, args } if args.len() == 1 => {
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

        // Product of non-negatives is non-negative
        ExprKind::Product(factors) => factors.iter().all(|f| is_known_non_negative(f)),

        // Sum of non-negatives is non-negative
        ExprKind::Sum(terms) => terms.iter().all(|t| is_known_non_negative(t)),

        // Division of non-negative by positive is non-negative (but we can't easily check "positive")
        // Be conservative here
        _ => false,
    }
}

/// Check if an exponent represents a fractional power that requires non-negative base
/// (i.e., exponents like 1/2, 1/4, 3/2, etc. where denominator is even)
pub(crate) fn is_fractional_root_exponent(expr: &Expr) -> bool {
    match &expr.kind {
        // Direct fraction: 1/2, 1/4, 3/4, etc.
        ExprKind::Div(_, den) => {
            if let ExprKind::Number(d) = &den.kind {
                // Check if denominator is an even integer
                d.fract() == 0.0 && (*d as i64) % 2 == 0
            } else {
                // Can't determine, be conservative
                false
            }
        }
        // Decimal like 0.5
        ExprKind::Number(n) => {
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

/// Simple GCD implementation for integers
pub(crate) fn gcd(a: i64, b: i64) -> i64 {
    let mut a = a.abs();
    let mut b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Check if an expression contains a specific factor
pub(crate) fn contains_factor(expr: &Expr, factor: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Product(factors) => factors.iter().any(|f| &**f == factor),
        _ => expr == factor,
    }
}

/// Remove factors from an expression
pub(crate) fn remove_factors(expr: &Expr, factors_to_remove: &Expr) -> Expr {
    match &expr.kind {
        ExprKind::Product(_) => {
            let expr_factors = flatten_mul(expr);
            let remove_factors = flatten_mul(factors_to_remove);

            let mut remaining_factors = Vec::new();
            for factor in expr_factors {
                let mut should_remove = false;
                for remove_factor in &remove_factors {
                    if factor == *remove_factor {
                        should_remove = true;
                        break;
                    }
                }
                if !should_remove {
                    remaining_factors.push(factor);
                }
            }

            if remaining_factors.is_empty() {
                Expr::number(1.0)
            } else if remaining_factors.len() == 1 {
                remaining_factors.into_iter().next().unwrap()
            } else {
                rebuild_mul(remaining_factors)
            }
        }
        _ => {
            // If the expression is not a multiplication, check if it matches the factors to remove
            if expr == factors_to_remove {
                Expr::number(1.0)
            } else {
                expr.clone()
            }
        }
    }
}

/// Get a structural hash for a term to group like terms (allocation-free)
///
/// Optimized for Speed:
/// 1. Uses FNV-1a constants for mixing.
/// 2. Uses `wrapping_add` for commutative operations (Sum, Product), ensuring
///    order-independence (a+b = b+a) without sorting or allocations.
/// 3. Uses `InternedSymbol::id()` (u64) directly, avoiding string hashing.
pub(crate) fn get_term_hash(expr: &Expr) -> u64 {
    // FNV-1a constants
    const FNV_OFFSET: u64 = 14695981039346656037;
    const FNV_PRIME: u64 = 1099511628211;

    #[inline(always)]
    fn hash_u64(mut hash: u64, n: u64) -> u64 {
        for byte in n.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    #[inline(always)]
    fn hash_f64(hash: u64, n: f64) -> u64 {
        hash_u64(hash, n.to_bits())
    }

    #[inline(always)]
    fn hash_one_byte(mut hash: u64, b: u8) -> u64 {
        hash ^= b as u64;
        hash.wrapping_mul(FNV_PRIME)
    }

    fn hash_term_inner(hash: u64, expr: &Expr) -> u64 {
        match &expr.kind {
            // Numbers are "like terms" with other numbers
            ExprKind::Number(_) => hash_one_byte(hash, b'N'),

            // Symbols: Hash their unique InternedSymbol ID (O(1))
            ExprKind::Symbol(s) => {
                let h = hash_one_byte(hash, b'S');
                hash_u64(h, s.id())
            }

            // Products: Commutative mix of factors (ignore numeric coeffs)
            ExprKind::Product(factors) => {
                let h = hash_one_byte(hash, b'P');

                // Commutative accumulation: sum(hash(factor))
                // This makes order irrelevant: hash(a*b) == hash(b*a)
                let mut product_accumulator: u64 = 0;

                for f in factors {
                    if !matches!(f.kind, ExprKind::Number(_)) {
                        // Hash each factor independently starting from a seed,
                        // then add to accumulator
                        product_accumulator =
                            product_accumulator.wrapping_add(hash_term_inner(FNV_OFFSET, f));
                    }
                }

                // Mix the accumulator into the main hash
                hash_u64(h, product_accumulator)
            }

            // Powers: Non-commutative base^exp
            ExprKind::Pow(base, exp) => {
                let h = hash_one_byte(hash, b'^');
                let h = hash_term_inner(h, base);
                match &exp.kind {
                    ExprKind::Number(n) => hash_f64(h, *n),
                    _ => hash_term_inner(h, exp),
                }
            }

            // Functions: Name ID + ordered args
            ExprKind::FunctionCall { name, args } => {
                let h = hash_one_byte(hash, b'F');
                let h = hash_u64(h, name.id());
                args.iter().fold(h, |acc, arg| hash_term_inner(acc, arg))
            }

            // Sums: Commutative mix of terms
            ExprKind::Sum(terms) => {
                let h = hash_one_byte(hash, b'+');

                // Commutative accumulation
                let mut sum_accumulator: u64 = 0;
                for t in terms {
                    sum_accumulator = sum_accumulator.wrapping_add(hash_term_inner(FNV_OFFSET, t));
                }

                hash_u64(h, sum_accumulator)
            }

            // Division: Non-commutative num/den
            ExprKind::Div(num, den) => {
                let h = hash_one_byte(hash, b'/');
                let h = hash_term_inner(h, num);
                hash_term_inner(h, den)
            }

            // Derivative: Non-commutative var, order, inner
            ExprKind::Derivative { inner, var, order } => {
                let h = hash_one_byte(hash, b'D');

                // Var is a String here (not InternedSymbol yet), so hash bytes
                let mut h = h;
                for byte in var.as_bytes() {
                    h = hash_one_byte(h, *byte);
                }

                let h = hash_u64(h, *order as u64);
                hash_term_inner(h, inner)
            }
        }
    }

    hash_term_inner(FNV_OFFSET, expr)
}

/// Normalize an expression to canonical form for comparison purposes.
/// This ensures that semantically equivalent expressions compare as equal.
pub(crate) fn normalize_for_comparison(expr: &Expr) -> Expr {
    // Early exit for leaf nodes - just clone, no transformation needed
    match &expr.kind {
        ExprKind::Number(_) | ExprKind::Symbol(_) => return expr.clone(),
        _ => {}
    }

    match &expr.kind {
        // Normalize Sum - recurse into terms
        ExprKind::Sum(terms) => {
            let norm_terms: Vec<Expr> = terms.iter().map(|t| normalize_for_comparison(t)).collect();
            // Check if any term changed
            let changed = norm_terms
                .iter()
                .zip(terms.iter())
                .any(|(new, old)| new.id != old.id);
            if !changed {
                return expr.clone();
            }
            Expr::sum(norm_terms)
        }
        // Normalize Product - simplify numeric products
        ExprKind::Product(factors) => {
            let norm_factors: Vec<Expr> = factors
                .iter()
                .map(|f| normalize_for_comparison(f))
                .collect();
            // Combine numeric factors
            let mut coeff = 1.0;
            let mut non_numeric: Vec<Expr> = Vec::new();
            for f in &norm_factors {
                if let ExprKind::Number(n) = &f.kind {
                    coeff *= n;
                } else {
                    non_numeric.push(f.clone());
                }
            }
            // If all numeric, return the product
            if non_numeric.is_empty() {
                return Expr::number(coeff);
            }
            // Check if nothing changed
            let changed = norm_factors
                .iter()
                .zip(factors.iter())
                .any(|(new, old)| new.id != old.id);
            if !changed {
                return expr.clone();
            }
            Expr::product(norm_factors)
        }
        ExprKind::Div(a, b) => {
            let norm_a = normalize_for_comparison(a);
            let norm_b = normalize_for_comparison(b);
            if norm_a.id == a.id && norm_b.id == b.id {
                return expr.clone();
            }
            Expr::div_expr(norm_a, norm_b)
        }
        ExprKind::Pow(a, b) => {
            let norm_a = normalize_for_comparison(a);
            let norm_b = normalize_for_comparison(b);
            if norm_a.id == a.id && norm_b.id == b.id {
                return expr.clone();
            }
            Expr::pow(norm_a, norm_b)
        }
        ExprKind::FunctionCall { name, args } => {
            let norm_args: Vec<Expr> = args.iter().map(normalize_for_comparison).collect();
            // Check if any arg changed
            let changed = norm_args
                .iter()
                .zip(args.iter())
                .any(|(new, old)| new.id != old.id);
            if !changed {
                return expr.clone();
            }
            Expr::new(ExprKind::FunctionCall {
                name: name.clone(),
                args: norm_args,
            })
        }
        // Leaves remain unchanged (already handled at the top)
        _ => expr.clone(),
    }
}

/// Check if two expressions are semantically equivalent (same after normalization)
pub(crate) fn exprs_equivalent(a: &Expr, b: &Expr) -> bool {
    normalize_for_comparison(a) == normalize_for_comparison(b)
}
