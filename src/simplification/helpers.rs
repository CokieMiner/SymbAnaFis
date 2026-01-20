//! Simplification helper functions and utilities
//!
//! Provides expression manipulation utilities: flattening, normalization,
//! coefficient extraction, root prettification, and like-term grouping.

use crate::core::known_symbols::{ABS, COSH, EXP, PI as PI_SYM, SQRT};

use crate::core::traits::EPSILON;
use crate::{Expr, ExprKind};
use std::sync::Arc;

// Floating point approx equality used for numeric pattern matching
pub fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < EPSILON
}

/// Get numeric value from expression if it's a Number.
/// Returns None if the expression is not a Number.
pub const fn get_numeric_value(expr: &Expr) -> Option<f64> {
    if let ExprKind::Number(n) = &expr.kind {
        Some(*n)
    } else {
        None
    }
}

// Trigonometric helpers
use std::f64::consts::PI;
pub fn is_multiple_of_two_pi(expr: &Expr) -> bool {
    if let ExprKind::Number(n) = &expr.kind {
        // Fix: If n is too large, we lose fractional precision,
        // so we can't possibly know if it's a multiple of 2pi.
        // 1e15 is slightly below the 2^53 limit (9e15).
        if n.abs() > 1e15 {
            return false;
        }

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
                ExprKind::Symbol(s) if s.id() == *PI_SYM => has_pi = true,
                _ => return false,
            }
        }

        if has_pi && let Some(n) = num_coeff {
            return n % 2.0 == 0.0;
        }
    }
    false
}

pub fn is_pi(expr: &Expr) -> bool {
    // Check numeric pi
    if let ExprKind::Number(n) = &expr.kind {
        return (n - PI).abs() < EPSILON;
    }
    // Check symbolic pi
    if let ExprKind::Symbol(s) = &expr.kind {
        return s.id() == *PI_SYM;
    }
    false
}

pub fn is_three_pi_over_two(expr: &Expr) -> bool {
    // Check numeric 3π/2
    if let ExprKind::Number(n) = &expr.kind {
        return (n - 3.0 * PI / 2.0).abs() < EPSILON;
    }
    // Check symbolic 3*pi/2 or (3/2)*pi
    if let ExprKind::Div(num, den) = &expr.kind
        && let ExprKind::Number(d) = &den.kind
        && (*d - 2.0).abs() < EPSILON
    {
        // Check if numerator is 3*pi
        if let ExprKind::Product(factors) = &num.kind {
            let mut has_three = false;
            let mut has_pi = false;
            for f in factors {
                if let ExprKind::Number(n) = &f.kind
                    && (*n - 3.0).abs() < EPSILON
                {
                    has_three = true;
                }
                if let ExprKind::Symbol(s) = &f.kind
                    && s.id() == *PI_SYM
                {
                    has_pi = true;
                }
            }
            return has_three && has_pi;
        }
    }
    false
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
pub fn compare_expr(a: &Expr, b: &Expr) -> std::cmp::Ordering {
    use crate::ExprKind::{Derivative, Div, FunctionCall, Number, Poly, Pow, Product, Sum, Symbol};
    use std::cmp::Ordering;

    /// Extract (`base_name`, degree) for polynomial-style ordering.
    /// Returns (`type_priority`, `base_symbol_id`, degree) - uses u64 ID to avoid String alloc
    fn get_sort_key(e: &Expr) -> (i32, u64, f64) {
        // Returns (type_priority, symbol_id, degree)
        // Lower type_priority = comes first
        // Lower degree = comes first (low-to-high)
        match &e.kind {
            Number(_) => (0, 0, 0.0),       // Numbers first
            Symbol(s) => (10, s.id(), 1.0), // Variables are degree 1, use symbol ID
            Pow(base, exp) => {
                if let Symbol(s) = &base.kind {
                    if let Number(n) = &exp.kind {
                        (10, s.id(), *n) // Same priority as symbol, degree from exponent
                    } else {
                        (10, s.id(), 999.0) // Symbolic exponent - treat as very high degree
                    }
                } else {
                    (30, 0, 0.0) // Non-symbol base - complex
                }
            }
            Product(factors) => {
                // For c*x^n or c*x, extract the variable part's key
                for f in factors {
                    let key = get_sort_key(f);
                    if key.0 == 10 {
                        // Found a polynomial term
                        return key;
                    }
                }
                (20, 0, 0.0) // No polynomial term found
            }
            FunctionCall { name, .. } => (40, name.id(), 0.0),
            Sum(..) => (50, 0, 0.0),
            Div(..) => (35, 0, 0.0),
            Derivative { .. } => (45, 0, 0.0),
            Poly(_) => (25, 0, 0.0), // Poly treated as complex
        }
    }

    let (type_a, base_a, deg_a) = get_sort_key(a);
    let (type_b, base_b, deg_b) = get_sort_key(b);

    // 1. Compare by type priority (numbers first)
    match type_a.cmp(&type_b) {
        Ordering::Equal => {}
        ord => return ord,
    }

    // 2. For numbers, compare by value
    if let (Number(n1), Number(n2)) = (&a.kind, &b.kind) {
        return n1.partial_cmp(n2).unwrap_or(Ordering::Equal);
    }

    // 3. Compare by base name (alphabetical) - keeps same base adjacent
    match base_a.cmp(&base_b) {
        Ordering::Equal => {}
        ord => return ord,
    }

    // 4. Compare by degree (low-to-high)
    deg_a.partial_cmp(&deg_b).unwrap_or(Ordering::Equal)
}

/// Compare expressions for multiplication factor ordering
/// Numbers (coefficients) come first, then symbols, then complex expressions
pub fn compare_mul_factors(a: &Expr, b: &Expr) -> std::cmp::Ordering {
    use crate::ExprKind::{Derivative, Div, FunctionCall, Number, Poly, Pow, Product, Sum, Symbol};
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
            Poly(_) => 55,           // After products, before sums
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

/// Helper to extract coefficient and base
/// Returns (coefficient, `base_expr`)
/// e.g. 2*x -> (2.0, x)
///      x   -> (1.0, x)
pub fn extract_coeff(expr: &Expr) -> (f64, Expr) {
    match &expr.kind {
        ExprKind::Number(n) => (*n, Expr::number(1.0)),
        ExprKind::Product(factors) => {
            let mut coeff = 1.0;
            let mut non_num_arcs = Vec::with_capacity(factors.len());

            for f in factors {
                if let ExprKind::Number(n) = &f.kind {
                    coeff *= n;
                } else {
                    non_num_arcs.push(Arc::clone(f));
                }
            }

            let base = if non_num_arcs.is_empty() {
                Expr::number(1.0)
            } else if non_num_arcs.len() == 1 {
                // Unwrap Arc if possible or clone contents
                let arc = non_num_arcs
                    .into_iter()
                    .next()
                    .expect("Non-numeric arcs guaranteed to have one element");
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

/// Helper to extract coefficient and base, returning Arc to avoid deep cloning
/// Returns (coefficient, Arc<`base_expr`>)
pub fn extract_coeff_arc(expr: &Expr) -> (f64, Arc<Expr>) {
    match &expr.kind {
        ExprKind::Number(n) => (*n, Arc::new(Expr::number(1.0))),
        ExprKind::Product(factors) => {
            let mut coeff = 1.0;
            let mut non_num_arcs = Vec::with_capacity(factors.len());

            for f in factors {
                if let ExprKind::Number(n) = &f.kind {
                    coeff *= n;
                } else {
                    non_num_arcs.push(Arc::clone(f));
                }
            }

            let base = if non_num_arcs.is_empty() {
                Arc::new(Expr::number(1.0))
            } else if non_num_arcs.len() == 1 {
                Arc::clone(&non_num_arcs[0])
            } else {
                Arc::new(Expr::product_from_arcs(non_num_arcs))
            };

            (coeff, base)
        }
        _ => (1.0, Arc::new(expr.clone())),
    }
}

/// Convert fractional powers back to roots for display
/// x^(1/2) -> sqrt(x)
/// x^(1/3) -> cbrt(x)
/// Optimized: only allocates when transformation occurs
pub fn prettify_roots(expr: Expr) -> Expr {
    match &expr.kind {
        ExprKind::Pow(base, exp) => {
            let base_pretty = prettify_roots((**base).clone());
            let exp_pretty = prettify_roots((**exp).clone());

            // x^(1/2) -> sqrt(x)
            if let ExprKind::Div(num, den) = &exp_pretty.kind {
                // Precise match for 1.0 and 2.0 is required for sqrt transformation
                #[allow(clippy::float_cmp)]
                if matches!(num.kind, ExprKind::Number(n) if n == 1.0)
                    && matches!(den.kind, ExprKind::Number(n) if n == 2.0)
                {
                    return Expr::func("sqrt", base_pretty);
                }
                // x^(1/3) -> cbrt(x)
                // Precise match for 1.0 and 3.0 is required for cbrt transformation
                #[allow(clippy::float_cmp)]
                if matches!(num.kind, ExprKind::Number(n) if n == 1.0)
                    && matches!(den.kind, ExprKind::Number(n) if n == 3.0)
                {
                    return Expr::func("cbrt", base_pretty);
                }
            }
            // x^0.5 -> sqrt(x)
            if let ExprKind::Number(n) = &exp_pretty.kind
                && (n - 0.5).abs() < EPSILON
            {
                return Expr::func("sqrt", base_pretty);
            }

            // Check if children changed
            if base_pretty.id == base.id && exp_pretty.id == exp.id {
                return expr; // No change, return original
            }
            Expr::pow(base_pretty, exp_pretty)
        }
        ExprKind::Sum(terms) => {
            let new_terms: Vec<Expr> = terms
                .iter()
                .map(|t| prettify_roots((**t).clone()))
                .collect();
            let changed = new_terms
                .iter()
                .zip(terms.iter())
                .any(|(new, old)| new.id != old.id);
            if !changed {
                return expr;
            }
            Expr::sum(new_terms)
        }
        ExprKind::Product(factors) => {
            let new_factors: Vec<Expr> = factors
                .iter()
                .map(|f| prettify_roots((**f).clone()))
                .collect();
            let changed = new_factors
                .iter()
                .zip(factors.iter())
                .any(|(new, old)| new.id != old.id);
            if !changed {
                return expr;
            }
            Expr::product(new_factors)
        }
        ExprKind::Div(u, v) => {
            let u_pretty = prettify_roots((**u).clone());
            let v_pretty = prettify_roots((**v).clone());
            if u_pretty.id == u.id && v_pretty.id == v.id {
                return expr;
            }
            Expr::div_expr(u_pretty, v_pretty)
        }
        ExprKind::FunctionCall { name, args } => {
            let new_args: Vec<Expr> = args.iter().map(|a| prettify_roots((**a).clone())).collect();
            let changed = new_args
                .iter()
                .zip(args.iter())
                .any(|(new, old)| new.id != old.id);
            if !changed {
                return expr;
            }
            Expr::func_multi(name, new_args)
        }
        // Leaves: Number, Symbol, Poly, Derivative - no transformation needed
        _ => expr,
    }
}

/// Check if an expression is known to be non-negative for all real values of its variables.
/// This is a conservative check - returns true only when we can prove non-negativity.
pub fn is_known_non_negative(expr: &Expr) -> bool {
    match &expr.kind {
        // Positive numbers
        ExprKind::Number(n) => *n >= 0.0,

        // x^2, x^4, x^6, ... are always non-negative
        // Also: (non_negative_expr)^(positive) is non-negative
        ExprKind::Pow(base, exp) => {
            if let ExprKind::Number(n) = &exp.kind {
                // Even positive integer exponents: x^2, x^4, etc.
                // Checked fract() == 0.0, so cast is safe
                if *n > 0.0 && n.fract() == 0.0 {
                    let is_even = {
                        #[allow(clippy::cast_possible_truncation)]
                        {
                            (*n as i64) % 2 == 0
                        }
                    };
                    if is_even {
                        return true;
                    }
                }
                // Non-negative base with any positive exponent: (x^2)^0.5, abs(x)^1.5
                if *n > 0.0 && is_known_non_negative(base) {
                    return true;
                }
            }
            false
        }

        // abs(x) is always non-negative
        ExprKind::FunctionCall { name, args } if args.len() == 1 => {
            if name.id() == *ABS {
                return true;
            }
            if name.id() == *EXP {
                return true;
            }
            if name.id() == *COSH {
                return true;
            }
            if name.id() == *SQRT {
                return is_known_non_negative(&args[0]);
            }
            false
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
pub fn is_fractional_root_exponent(expr: &Expr) -> bool {
    match &expr.kind {
        // Direct fraction: 1/2, 1/4, 3/4, etc.
        ExprKind::Div(_, den) => {
            if let ExprKind::Number(d) = &den.kind {
                // Check if denominator is an even integer
                // Checked fract() == 0.0, so cast is safe
                #[allow(clippy::cast_possible_truncation)]
                {
                    d.fract() == 0.0 && (*d as i64) % 2 == 0
                }
            } else {
                // Can't determine, be conservative
                false
            }
        }
        // Decimal like 0.5
        ExprKind::Number(n) => {
            // Check if it's a fractional power (not an integer)
            // For 0.5, 0.25, 1.5, etc. - these involve even roots
            if n.fract() == 0.0 {
                false
            } else {
                // Check if it's k/2^n for some integers
                // Simple check: 0.5 = 1/2, 0.25 = 1/4, 0.75 = 3/4, etc.
                let doubled = *n * 2.0;
                doubled.fract() == 0.0 // If 2*n is integer, then n = k/2
            }
        }
        _ => false,
    }
}

pub const fn gcd(a: i64, b: i64) -> i64 {
    let mut a = a.unsigned_abs();
    let mut b = b.unsigned_abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    #[allow(clippy::cast_possible_wrap)]
    {
        a as i64
    }
}

/// Get a structural hash for a term to group like terms (allocation-free)
///
/// Optimized for Speed:
/// 1. Uses FNV-1a constants for mixing.
/// 2. Uses `wrapping_add` for commutative operations (Sum, Product), ensuring
///    order-independence (a+b = b+a) without sorting or allocations.
/// 3. Uses `InternedSymbol::id()` (u64) directly, avoiding string hashing.
pub fn get_term_hash(expr: &Expr) -> u64 {
    // FNV-1a constants
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;

    #[inline]
    fn hash_u64(mut hash: u64, n: u64) -> u64 {
        for byte in n.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    #[inline]
    fn hash_f64(hash: u64, n: f64) -> u64 {
        hash_u64(hash, n.to_bits())
    }

    #[inline]
    const fn hash_one_byte(mut hash: u64, b: u8) -> u64 {
        hash ^= b as u64;
        hash.wrapping_mul(FNV_PRIME)
    }

    fn hash_term_inner(hash: u64, expr: &Expr) -> u64 {
        match &expr.kind {
            // Numbers: Hash the actual value to distinguish different numbers
            ExprKind::Number(n) => hash_f64(hash_one_byte(hash, b'N'), *n),

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
                args.iter()
                    .fold(h, |acc, a| hash_term_inner(acc, a.as_ref()))
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

                // Use var.id() for O(1) hashing (InternedSymbol)
                let h = hash_u64(h, var.id());

                let h = hash_u64(h, u64::from(*order));
                hash_term_inner(h, inner)
            }

            // Poly: Hash based on base and terms
            ExprKind::Poly(poly) => {
                let h = hash_one_byte(hash, b'Y'); // 'Y' for polY to distinguish
                // Hash the base expression
                let h = hash_term_inner(h, poly.base());
                // Commutative sum of term hashes (power, coeff)
                let mut acc: u64 = 0;
                for &(pow, coeff) in poly.terms() {
                    let term_h = hash_u64(hash_f64(FNV_OFFSET, coeff), u64::from(pow));
                    acc = acc.wrapping_add(term_h);
                }
                hash_u64(h, acc)
            }
        }
    }

    hash_term_inner(FNV_OFFSET, expr)
}

/// Normalize an expression to canonical form for comparison purposes.
/// This ensures that semantically equivalent expressions compare as equal.
pub fn normalize_for_comparison(expr: &Expr) -> Expr {
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
            // Build the normalized product: coeff * non_numeric_factors
            if (coeff - 1.0).abs() < 1e-14 {
                // Coefficient is 1, just return non-numeric factors
                if non_numeric.len() == 1 {
                    return non_numeric.remove(0);
                }
                return Expr::product(non_numeric);
            }
            // Include the combined coefficient
            let mut result = vec![Expr::number(coeff)];
            result.extend(non_numeric);
            Expr::product(result)
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
            let norm_args: Vec<Expr> = args
                .iter()
                .map(|a| normalize_for_comparison(a.as_ref()))
                .collect();
            // Check if any arg changed
            let changed = norm_args
                .iter()
                .zip(args.iter())
                .any(|(new, old)| new.id != old.id);
            if !changed {
                return expr.clone();
            }
            Expr::func_multi(name, norm_args)
        }
        // Leaves remain unchanged (already handled at the top)
        _ => expr.clone(),
    }
}

/// Check if two expressions are semantically equivalent (same after normalization)
pub fn exprs_equivalent(a: &Expr, b: &Expr) -> bool {
    normalize_for_comparison(a) == normalize_for_comparison(b)
}
