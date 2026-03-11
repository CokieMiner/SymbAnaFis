//! Simplification helper functions and utilities
//!
//! Provides expression manipulation utilities: flattening, normalization,
//! coefficient extraction, root prettification, and like-term grouping.

use crate::core::known_symbols as ks;
use crate::core::traits::EPSILON;
use crate::{Expr, ExprKind};
use std::f64::consts::PI;
use std::sync::Arc;

/// Floating point approximate equality used for numeric pattern matching
/// in simplification rules. Uses epsilon tolerance for safe float comparison.
#[inline]
pub fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < EPSILON
}

/// Get numeric value from expression if it's a Number.
/// Returns None if the expression is not a Number.
#[inline]
pub const fn get_numeric_value(expr: &Expr) -> Option<f64> {
    if let ExprKind::Number(n) = &expr.kind {
        Some(*n)
    } else {
        None
    }
}

/// Check if expression represents a multiple of 2π.
/// Used for trigonometric simplifications (sin(x + 2πk) = sin(x)).
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
                ExprKind::Symbol(s) if s.id() == ks::KS.pi => has_pi = true,
                _ => return false,
            }
        }

        if has_pi && let Some(n) = num_coeff {
            return n % 2.0 == 0.0;
        }
    }
    false
}

/// Check if expression represents π.
/// Used for trigonometric exact value simplifications.
pub fn is_pi(expr: &Expr) -> bool {
    // Check numeric pi
    if let ExprKind::Number(n) = &expr.kind {
        return (n - PI).abs() < EPSILON;
    }
    // Check symbolic pi
    if let ExprKind::Symbol(s) = &expr.kind {
        return s.id() == ks::KS.pi;
    }
    false
}

/// Check if expression represents 3π/2.
/// Used for trigonometric exact value simplifications.
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
                    && s.id() == ks::KS.pi
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
///
/// Compare expressions for canonical ordering in simplification.
///
/// Used to establish a consistent term order in sums and products
/// for algebraic simplification rules.
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
/// Compare multiplication factors for canonical ordering.
/// Used for organizing terms in products during simplification.
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
/// Extract numeric coefficient from expression.
/// Returns (coefficient, `remaining_expression`).
/// Used for like-term collection in algebraic simplification.
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

/// Helper to extract coefficient and base, returning Arc to avoid deep cloning.
/// Returns (coefficient, Arc<`base_expr`>).
///
/// Accepts `&Arc<Expr>` so the common case (non-Product, non-Number) can
/// return `Arc::clone` (O(1) refcount bump) instead of `Arc::new(expr.clone())`
/// which deep-clones the entire tree.
#[inline]
pub fn extract_coeff_arc(expr: &Arc<Expr>) -> (f64, Arc<Expr>) {
    match &expr.kind {
        ExprKind::Number(n) => (*n, crate::core::expr::arc_number(1.0)),
        ExprKind::Product(factors) => {
            // Fast path: if no numeric factors, coeff is 1.0 and base is the original
            let has_numeric = factors
                .iter()
                .any(|f| matches!(f.kind, ExprKind::Number(_)));
            if !has_numeric {
                return (1.0, Arc::clone(expr));
            }

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
                crate::core::expr::arc_number(1.0)
            } else if non_num_arcs.len() == 1 {
                Arc::clone(&non_num_arcs[0])
            } else {
                Arc::new(Expr::product_from_arcs(non_num_arcs))
            };

            (coeff, base)
        }
        _ => (1.0, Arc::clone(expr)),
    }
}

/// Convert fractional powers back to roots for display
/// x^(1/2) -> sqrt(x)
/// x^(1/3) -> cbrt(x)
/// Optimized: only allocates when transformation occurs
/// Transform fractional exponents to root notation for readability.
/// Used for final expression formatting in simplification.
#[allow(
    clippy::too_many_lines,
    reason = "Iterative post-order traversal is inherently verbose"
)]
pub fn prettify_roots(root: Expr) -> Expr {
    // Post-order iterative transform using two stacks:
    //   `work`    – nodes still to descend into
    //   `results` – already-transformed children, consumed during assembly
    //
    // Each work item is either:
    //   Visit(expr)                       – push children, then push Assemble
    //   Assemble(kind, child_count, expr) – pop children, rebuild if changed

    enum Task {
        Visit(Expr),
        /// (original expr, number of child results to pop)
        Assemble(Expr, usize),
    }

    let mut work: Vec<Task> = vec![Task::Visit(root)];
    let mut results: Vec<Expr> = Vec::new();

    while let Some(task) = work.pop() {
        match task {
            Task::Visit(expr) => match &expr.kind {
                ExprKind::Number(_)
                | ExprKind::Symbol(_)
                | ExprKind::Poly(_)
                | ExprKind::Derivative { .. } => {
                    results.push(expr);
                }
                ExprKind::Sum(terms) => {
                    let n = terms.len();
                    work.push(Task::Assemble(expr.clone(), n));
                    for t in terms.iter().rev() {
                        work.push(Task::Visit((**t).clone()));
                    }
                }
                ExprKind::Product(factors) => {
                    let n = factors.len();
                    work.push(Task::Assemble(expr.clone(), n));
                    for f in factors.iter().rev() {
                        work.push(Task::Visit((**f).clone()));
                    }
                }
                ExprKind::Div(u, v) => {
                    work.push(Task::Assemble(expr.clone(), 2));
                    work.push(Task::Visit((**v).clone()));
                    work.push(Task::Visit((**u).clone()));
                }
                ExprKind::Pow(base, exp) => {
                    work.push(Task::Assemble(expr.clone(), 2));
                    work.push(Task::Visit((**exp).clone()));
                    work.push(Task::Visit((**base).clone()));
                }
                ExprKind::FunctionCall { args, .. } => {
                    let n = args.len();
                    work.push(Task::Assemble(expr.clone(), n));
                    for a in args.iter().rev() {
                        work.push(Task::Visit((**a).clone()));
                    }
                }
            },
            Task::Assemble(orig, n) => {
                let start = results.len() - n;
                // Borrow the transformed children (they sit at the tail of results)
                match &orig.kind {
                    ExprKind::Pow(old_base, old_exp) => {
                        let base_pretty = &results[start];
                        let exp_pretty = &results[start + 1];

                        // x^(1/2) -> sqrt(x)
                        if let ExprKind::Div(num, den) = &exp_pretty.kind {
                            #[allow(
                                clippy::float_cmp,
                                reason = "Precise match required for sqrt/cbrt"
                            )]
                            if matches!(num.kind, ExprKind::Number(v) if v == 1.0)
                                && matches!(den.kind, ExprKind::Number(v) if v == 2.0)
                            {
                                let b = results.drain(start..).next().expect("base");
                                results.push(Expr::func_symbol(ks::get_symbol(ks::KS.sqrt), b));
                                continue;
                            }
                            #[allow(
                                clippy::float_cmp,
                                reason = "Precise match required for sqrt/cbrt"
                            )]
                            if matches!(num.kind, ExprKind::Number(v) if v == 1.0)
                                && matches!(den.kind, ExprKind::Number(v) if v == 3.0)
                            {
                                let b = results.drain(start..).next().expect("base");
                                results.push(Expr::func_symbol(ks::get_symbol(ks::KS.cbrt), b));
                                continue;
                            }
                        }
                        // x^0.5 -> sqrt(x)
                        if let ExprKind::Number(val) = &exp_pretty.kind
                            && (val - 0.5).abs() < EPSILON
                        {
                            let b = results.drain(start..).next().expect("base");
                            results.push(Expr::func_symbol(ks::get_symbol(ks::KS.sqrt), b));
                            continue;
                        }

                        // Check if children changed
                        if base_pretty.id != old_base.id || exp_pretty.id != old_exp.id {
                            let mut drained = results.drain(start..);
                            let b = drained.next().expect("base");
                            let e = drained.next().expect("exp");
                            drop(drained);
                            results.push(Expr::pow(b, e));
                        } else {
                            results.truncate(start);
                            results.push(orig);
                        }
                    }
                    ExprKind::Sum(terms) => {
                        let changed = results[start..]
                            .iter()
                            .zip(terms.iter())
                            .any(|(new, old)| new.id != old.id);
                        if changed {
                            let v: Vec<Expr> = results.drain(start..).collect();
                            results.push(Expr::sum(v));
                        } else {
                            results.truncate(start);
                            results.push(orig);
                        }
                    }
                    ExprKind::Product(factors) => {
                        let changed = results[start..]
                            .iter()
                            .zip(factors.iter())
                            .any(|(new, old)| new.id != old.id);
                        if changed {
                            let v: Vec<Expr> = results.drain(start..).collect();
                            results.push(Expr::product(v));
                        } else {
                            results.truncate(start);
                            results.push(orig);
                        }
                    }
                    ExprKind::Div(old_u, old_v) => {
                        let u_changed = results[start].id != old_u.id;
                        let v_changed = results[start + 1].id != old_v.id;
                        if u_changed || v_changed {
                            let mut drained = results.drain(start..);
                            let u = drained.next().expect("u");
                            let v = drained.next().expect("v");
                            drop(drained);
                            results.push(Expr::div_expr(u, v));
                        } else {
                            results.truncate(start);
                            results.push(orig);
                        }
                    }
                    ExprKind::FunctionCall { name, args } => {
                        let changed = results[start..]
                            .iter()
                            .zip(args.iter())
                            .any(|(new, old)| new.id != old.id);
                        if changed {
                            let v: Vec<Expr> = results.drain(start..).collect();
                            results.push(Expr::func_multi(name, v));
                        } else {
                            results.truncate(start);
                            results.push(orig);
                        }
                    }
                    _ => {
                        results.truncate(start);
                        results.push(orig);
                    }
                }
            }
        }
    }

    results
        .pop()
        .expect("prettify_roots must produce exactly one result")
}

/// Check if an expression is known to be non-negative for all real values of its variables.
/// This is a conservative check - returns true only when we can prove non-negativity.
/// Check if expression is known to be non-negative.
/// Used for safe square root and absolute value simplifications.
pub fn is_known_non_negative(expr: &Expr) -> bool {
    // Iterative: every node on the stack must be non-negative for the
    // overall result to be true.  We push children that need checking
    // (Product factors, Sum terms, Pow bases, sqrt args) and return
    // false as soon as any node fails.
    let mut stack: Vec<&Expr> = vec![expr];
    while let Some(node) = stack.pop() {
        match &node.kind {
            // Positive numbers
            ExprKind::Number(n) => {
                if *n < 0.0 {
                    return false;
                }
            }

            // x^2, x^4, x^6, ... are always non-negative
            ExprKind::Pow(base, exp) => {
                if let ExprKind::Number(n) = &exp.kind {
                    // Even positive integer exponents: x^2, x^4, etc.
                    if *n > 0.0 && n.fract() == 0.0 {
                        #[allow(
                            clippy::cast_possible_truncation,
                            reason = "Checked fract() == 0.0, so cast is safe"
                        )]
                        let is_even = (*n as i64) % 2 == 0;
                        if is_even {
                            continue; // proven non-negative, no need to check base
                        }
                    }
                    // Non-negative base with any positive exponent
                    if *n > 0.0 {
                        stack.push(base); // base must be non-negative
                        continue;
                    }
                }
                return false;
            }

            // abs(x), exp(x), cosh(x) are always non-negative
            ExprKind::FunctionCall { name, args } if args.len() == 1 => {
                if name.id() == ks::KS.abs || name.id() == ks::KS.exp || name.id() == ks::KS.cosh {
                    continue; // always non-negative
                }
                if name.id() == ks::KS.sqrt {
                    stack.push(&args[0]); // non-negative if arg is non-negative
                    continue;
                }
                return false;
            }

            // Product/Sum of non-negatives is non-negative
            ExprKind::Product(args) | ExprKind::Sum(args) => {
                stack.extend(args.iter().map(AsRef::as_ref));
            }

            // Division of non-negative by positive is non-negative (but we can't easily check "positive")
            // Be conservative here
            _ => return false,
        }
    }
    true
}

/// Check if an exponent represents a fractional power that requires non-negative base
/// (i.e., exponents like 1/2, 1/4, 3/2, etc. where denominator is even)
/// Check if expression represents a fractional root exponent.
/// Used for power rule simplifications like (x^(1/2))^2 = x.
pub fn is_fractional_root_exponent(expr: &Expr) -> bool {
    match &expr.kind {
        // Direct fraction: 1/2, 1/4, 3/4, etc.
        ExprKind::Div(_, den) => {
            if let ExprKind::Number(d) = &den.kind {
                // Check if denominator is an even integer
                // Checked fract() == 0.0, so cast is safe
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "Checked fract() == 0.0, so cast is safe"
                )]
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

// Compute greatest common divisor using Euclid's algorithm.
/// Used for rational number simplification in algebraic rules.
pub const fn gcd(a: i64, b: i64) -> i64 {
    let mut a = a.unsigned_abs();
    let mut b = b.unsigned_abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    #[allow(
        clippy::cast_possible_wrap,
        reason = "GCD values are typically small integers, wrap unlikely"
    )]
    {
        a as i64
    }
}

// FNV-1a constants shared by both term hash entry points.
const FNV_TERM_OFFSET: u64 = 14_695_981_039_346_656_037;
const FNV_TERM_PRIME: u64 = 1_099_511_628_211;

#[inline]
fn term_hash_u64(mut hash: u64, n: u64) -> u64 {
    for byte in n.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_TERM_PRIME);
    }
    hash
}

#[inline]
fn term_hash_f64(hash: u64, n: f64) -> u64 {
    term_hash_u64(hash, n.to_bits())
}

#[inline]
const fn term_hash_byte(mut hash: u64, b: u8) -> u64 {
    hash ^= b as u64;
    hash.wrapping_mul(FNV_TERM_PRIME)
}

/// Inner recursive term hash — operates on `&ExprKind`.
/// Children are full `Arc<Expr>` nodes, so we recurse into their `.kind` safely.
///
/// Note: This function is only called for non-Sum/non-Product nodes (Pow, Div,
/// `FunctionCall`, Derivative, Poly) via `compute_term_hash`. The hot path
/// (Sum/Product construction) uses cached `term_hash` values in O(1).
/// The recursion depth here equals the nesting of non-Sum/non-Product nodes,
/// which is typically very shallow (2-5 levels).
fn hash_term_inner(hash: u64, kind: &ExprKind) -> u64 {
    match kind {
        ExprKind::Number(n) => term_hash_f64(term_hash_byte(hash, b'N'), *n),

        ExprKind::Symbol(s) => term_hash_u64(term_hash_byte(hash, b'S'), s.id()),

        ExprKind::Product(factors) => {
            let h = term_hash_byte(hash, b'P');
            let mut acc: u64 = 0;
            for f in factors {
                if !matches!(f.kind, ExprKind::Number(_)) {
                    acc = acc.wrapping_add(hash_term_inner(FNV_TERM_OFFSET, &f.kind));
                }
            }
            term_hash_u64(h, acc)
        }

        ExprKind::Pow(base, exp) => {
            let h = term_hash_byte(hash, b'^');
            let h = hash_term_inner(h, &base.kind);
            match &exp.kind {
                ExprKind::Number(n) => term_hash_f64(h, *n),
                ek => hash_term_inner(h, ek),
            }
        }

        ExprKind::FunctionCall { name, args } => {
            let h = term_hash_byte(hash, b'F');
            let h = term_hash_u64(h, name.id());
            args.iter().fold(h, |acc, a| hash_term_inner(acc, &a.kind))
        }

        ExprKind::Sum(terms) => {
            let h = term_hash_byte(hash, b'+');
            let mut acc: u64 = 0;
            for t in terms {
                acc = acc.wrapping_add(hash_term_inner(FNV_TERM_OFFSET, &t.kind));
            }
            term_hash_u64(h, acc)
        }

        ExprKind::Div(num, den) => {
            let h = term_hash_byte(hash, b'/');
            let h = hash_term_inner(h, &num.kind);
            hash_term_inner(h, &den.kind)
        }

        ExprKind::Derivative { inner, var, order } => {
            let h = term_hash_byte(hash, b'D');
            let h = term_hash_u64(h, var.id());
            let h = term_hash_u64(h, u64::from(*order));
            hash_term_inner(h, &inner.kind)
        }

        ExprKind::Poly(poly) => {
            let h = term_hash_byte(hash, b'Y');
            let h = hash_term_inner(h, &poly.base().kind);
            let mut acc: u64 = 0;
            for &(pow, coeff) in poly.terms() {
                let th = term_hash_u64(term_hash_f64(FNV_TERM_OFFSET, coeff), u64::from(pow));
                acc = acc.wrapping_add(th);
            }
            term_hash_u64(h, acc)
        }
    }
}

/// Compute the coefficient-insensitive term hash from an `ExprKind`.
/// Called by `Expr::new` and static initializers to populate `expr.term_hash`.
/// All other code should read `expr.term_hash` directly — it is cached at construction.
///
/// For `Sum` and `Product` (the dominant construction cases), children are
/// already-built `Arc<Expr>` nodes whose `term_hash` is already cached, so we
/// combine them in O(N direct children) rather than `O(tree_size)`.
/// All other composite types fall back to full recursive computation.
#[inline]
pub fn compute_term_hash(kind: &ExprKind) -> u64 {
    match kind {
        // Leaves: fully inline, O(1)
        ExprKind::Number(n) => term_hash_f64(term_hash_byte(FNV_TERM_OFFSET, b'N'), *n),
        ExprKind::Symbol(s) => term_hash_u64(term_hash_byte(FNV_TERM_OFFSET, b'S'), s.id()),

        // Sum: combine children's cached term_hashes — O(N), not O(tree_size)
        // Correctness: hash_term_inner for Sum does wrapping_add(hash_term_inner(FNV_OFFSET, t))
        // which equals wrapping_add(t.term_hash) since t.term_hash = hash_term_inner(FNV_OFFSET, t.kind)
        ExprKind::Sum(terms) => {
            let h = term_hash_byte(FNV_TERM_OFFSET, b'+');
            let mut acc: u64 = 0;
            for t in terms {
                acc = acc.wrapping_add(t.term_hash);
            }
            term_hash_u64(h, acc)
        }

        // Product: same, but skip Number factors (coefficient-insensitive) — O(N)
        ExprKind::Product(factors) => {
            let h = term_hash_byte(FNV_TERM_OFFSET, b'P');
            let mut acc: u64 = 0;
            for f in factors {
                if !matches!(f.kind, ExprKind::Number(_)) {
                    acc = acc.wrapping_add(f.term_hash);
                }
            }
            term_hash_u64(h, acc)
        }

        // All other composite types: full recursive walk (less common construction path)
        _ => hash_term_inner(FNV_TERM_OFFSET, kind),
    }
}

/// Normalize expression for structural comparison.
/// Used to determine if two expressions are equivalent modulo ordering.
#[allow(
    clippy::too_many_lines,
    reason = "Iterative post-order traversal is inherently verbose"
)]
pub fn normalize_for_comparison(root: &Expr) -> Expr {
    // Post-order iterative normalization using the Visit/Assemble pattern.

    enum Task<'expr> {
        Visit(&'expr Expr),
        Assemble(&'expr Expr, usize),
    }

    let mut work: Vec<Task<'_>> = vec![Task::Visit(root)];
    let mut results: Vec<Expr> = Vec::new();

    while let Some(task) = work.pop() {
        match task {
            Task::Visit(expr) => match &expr.kind {
                ExprKind::Number(_)
                | ExprKind::Symbol(_)
                | ExprKind::Poly(_)
                | ExprKind::Derivative { .. } => {
                    results.push(expr.clone());
                }
                ExprKind::Sum(terms) => {
                    let n = terms.len();
                    work.push(Task::Assemble(expr, n));
                    for t in terms.iter().rev() {
                        work.push(Task::Visit(t));
                    }
                }
                ExprKind::Product(factors) => {
                    let n = factors.len();
                    work.push(Task::Assemble(expr, n));
                    for f in factors.iter().rev() {
                        work.push(Task::Visit(f));
                    }
                }
                ExprKind::Div(a, b) | ExprKind::Pow(a, b) => {
                    work.push(Task::Assemble(expr, 2));
                    work.push(Task::Visit(b));
                    work.push(Task::Visit(a));
                }
                ExprKind::FunctionCall { args, .. } => {
                    let n = args.len();
                    work.push(Task::Assemble(expr, n));
                    for a in args.iter().rev() {
                        work.push(Task::Visit(a));
                    }
                }
            },
            Task::Assemble(orig, n) => {
                let start = results.len() - n;
                match &orig.kind {
                    ExprKind::Sum(terms) => {
                        let changed = results[start..]
                            .iter()
                            .zip(terms.iter())
                            .any(|(new, old)| new.id != old.id);
                        if changed {
                            let v: Vec<Expr> = results.drain(start..).collect();
                            results.push(Expr::sum(v));
                        } else {
                            results.truncate(start);
                            results.push(orig.clone());
                        }
                    }
                    ExprKind::Product(_factors) => {
                        let norm_factors: Vec<Expr> = results.drain(start..).collect();
                        // Combine numeric factors
                        let mut coeff = 1.0;
                        let mut non_numeric: Vec<Expr> = Vec::new();
                        for f in &norm_factors {
                            if let ExprKind::Number(val) = &f.kind {
                                coeff *= val;
                            } else {
                                non_numeric.push(f.clone());
                            }
                        }
                        if non_numeric.is_empty() {
                            results.push(Expr::number(coeff));
                        } else if (coeff - 1.0).abs() < EPSILON {
                            if non_numeric.len() == 1 {
                                results.push(non_numeric.remove(0));
                            } else {
                                results.push(Expr::product(non_numeric));
                            }
                        } else {
                            let mut result = vec![Expr::number(coeff)];
                            result.extend(non_numeric);
                            results.push(Expr::product(result));
                        }
                    }
                    ExprKind::Div(old_a, old_b) => {
                        let a_changed = results[start].id != old_a.id;
                        let b_changed = results[start + 1].id != old_b.id;
                        if a_changed || b_changed {
                            let mut drained = results.drain(start..);
                            let a = drained.next().expect("a");
                            let b = drained.next().expect("b");
                            drop(drained);
                            results.push(Expr::div_expr(a, b));
                        } else {
                            results.truncate(start);
                            results.push(orig.clone());
                        }
                    }
                    ExprKind::Pow(old_a, old_b) => {
                        let a_changed = results[start].id != old_a.id;
                        let b_changed = results[start + 1].id != old_b.id;
                        if a_changed || b_changed {
                            let mut drained = results.drain(start..);
                            let a = drained.next().expect("a");
                            let b = drained.next().expect("b");
                            drop(drained);
                            results.push(Expr::pow(a, b));
                        } else {
                            results.truncate(start);
                            results.push(orig.clone());
                        }
                    }
                    ExprKind::FunctionCall { name, args } => {
                        let changed = results[start..]
                            .iter()
                            .zip(args.iter())
                            .any(|(new, old)| new.id != old.id);
                        if changed {
                            let v: Vec<Expr> = results.drain(start..).collect();
                            results.push(Expr::func_multi(name, v));
                        } else {
                            results.truncate(start);
                            results.push(orig.clone());
                        }
                    }
                    _ => {
                        results.truncate(start);
                        results.push(orig.clone());
                    }
                }
            }
        }
    }

    results
        .pop()
        .expect("normalize_for_comparison must produce exactly one result")
}

/// Check if two expressions are semantically equivalent (same after normalization)
/// Check if two expressions are structurally equivalent.
/// Used for testing and validation in simplification rules.
pub fn exprs_equivalent(a: &Expr, b: &Expr) -> bool {
    normalize_for_comparison(a) == normalize_for_comparison(b)
}
