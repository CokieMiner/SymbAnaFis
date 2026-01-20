use crate::core::known_symbols::{CBRT, SQRT};
use crate::core::poly::Polynomial;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};
use std::sync::Arc;

/// Rule for cancelling common terms in fractions: (a*b)/(a*c) -> b/c
/// Also handles powers: x^a / x^b -> x^(a-b)
pub struct FractionCancellationRule;

impl Rule for FractionCancellationRule {
    fn name(&self) -> &'static str {
        "fraction_cancellation"
    }

    fn priority(&self) -> i32 {
        76
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Algebraic
    }

    fn applies_to(&self) -> &'static [ExprKind] {
        &[ExprKind::Div, ExprKind::Product]
    }

    // Fraction cancellation handles many expression patterns, length is justified
    #[allow(clippy::too_many_lines)] // Complex fraction simplification logic
    fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
        // For Product expressions, check if there's a Div nested inside
        if let AstKind::Product(factors) = &expr.kind {
            // Look for a Div among the factors
            for (i, factor) in factors.iter().enumerate() {
                if let AstKind::Div(num, den) = &factor.kind {
                    // Combine all other factors with numerator
                    let mut new_num_factors: Vec<Arc<Expr>> = factors
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, f)| Arc::clone(f))
                        .collect();
                    new_num_factors.push(Arc::clone(num));

                    let combined_num = if new_num_factors.len() == 1 {
                        new_num_factors
                            .into_iter()
                            .next()
                            .expect("Vector guaranteed to have exactly one element")
                    } else {
                        Arc::new(Expr::product_from_arcs(new_num_factors))
                    };

                    let new_div = Expr::div_from_arcs(combined_num, Arc::clone(den));
                    return self.apply(&Arc::new(new_div), context);
                }
            }
            return None;
        }

        if let AstKind::Div(u, v) = &expr.kind {
            // Helper to get base and exponent
            fn get_base_exp(e: &Arc<Expr>) -> (Arc<Expr>, Arc<Expr>) {
                match &e.kind {
                    AstKind::Pow(b, exp) => (Arc::clone(b), Arc::clone(exp)),
                    AstKind::FunctionCall { name, args } if args.len() == 1 => {
                        // Use ID validation via known_symbols
                        if name.id() == *SQRT {
                            (Arc::clone(&args[0]), Arc::new(Expr::number(0.5)))
                        } else if name.id() == *CBRT {
                            (
                                Arc::clone(&args[0]),
                                Arc::new(Expr::div_expr(Expr::number(1.0), Expr::number(3.0))),
                            )
                        } else {
                            (Arc::clone(e), Arc::new(Expr::number(1.0)))
                        }
                    }
                    _ => (Arc::clone(e), Arc::new(Expr::number(1.0))),
                }
            }

            fn is_safe_to_cancel(base: &Expr) -> bool {
                match &base.kind {
                    AstKind::Number(n) => n.abs() > 1e-10,
                    _ => false,
                }
            }

            // Get factors from numerator and denominator
            let num_factors = get_factors_arcs(u);
            let den_factors = get_factors_arcs(v);

            // 1. Handle numeric coefficients (always safe - nonzero constants)
            let mut num_coeff = 1.0;
            let mut den_coeff = 1.0;
            let mut new_num_factors = Vec::new();
            let mut new_den_factors = Vec::new();

            for f in num_factors {
                if let AstKind::Number(n) = &f.kind {
                    num_coeff *= n;
                } else {
                    new_num_factors.push(f);
                }
            }

            for f in den_factors {
                if let AstKind::Number(n) = &f.kind {
                    den_coeff *= n;
                } else {
                    new_den_factors.push(f);
                }
            }

            // Simplify coefficients
            let ratio = num_coeff / den_coeff;
            if ratio.abs() < 1e-10 {
                return Some(Arc::new(Expr::number(0.0)));
            }

            if (ratio - ratio.round()).abs() < 1e-10 {
                num_coeff = ratio.round();
                den_coeff = 1.0;
            } else if (1.0 / ratio - (1.0 / ratio).round()).abs() < 1e-10 {
                let inv = (1.0 / ratio).round();
                if inv < 0.0 {
                    num_coeff = -1.0;
                    den_coeff = -inv;
                } else {
                    num_coeff = 1.0;
                    den_coeff = inv;
                }
            }

            // 2. Symbolic cancellation
            let mut i = 0;
            while i < new_num_factors.len() {
                let (base_i, exp_i) = get_base_exp(&new_num_factors[i]);
                let mut matched = false;

                for j in 0..new_den_factors.len() {
                    let (base_j, exp_j) = get_base_exp(&new_den_factors[j]);

                    if crate::simplification::helpers::exprs_equivalent(&base_i, &base_j) {
                        if context.domain_safe && !is_safe_to_cancel(&base_i) {
                            break;
                        }

                        let simplified_exp = if let (AstKind::Number(n1), AstKind::Number(n2)) =
                            (&exp_i.kind, &exp_j.kind)
                        {
                            Expr::number(n1 - n2)
                        } else {
                            Expr::sum_from_arcs(vec![
                                Arc::clone(&exp_i),
                                Arc::new(Expr::product_from_arcs(vec![
                                    Arc::new(Expr::number(-1.0)),
                                    Arc::clone(&exp_j),
                                ])),
                            ])
                        };

                        if let AstKind::Number(n) = &simplified_exp.kind {
                            let n = *n;
                            if n == 0.0 {
                                new_num_factors.remove(i);
                                new_den_factors.remove(j);
                                matched = true;
                                break;
                            } else if n > 0.0 {
                                let is_one_exponent = {
                                    // Exact check for 1.0 exponent
                                    #[allow(clippy::float_cmp)]
                                    // Comparing against exact constant 1.0
                                    let res = n == 1.0;
                                    res
                                };
                                if is_one_exponent {
                                    new_num_factors[i] = Arc::clone(&base_i);
                                } else {
                                    new_num_factors[i] = Arc::new(Expr::pow_from_arcs(
                                        Arc::clone(&base_i),
                                        Arc::new(Expr::number(n)),
                                    ));
                                }
                                new_den_factors.remove(j);
                            } else {
                                new_num_factors.remove(i);
                                let pos_n = -n;
                                let is_one_exponent = {
                                    // Exact check for 1.0 exponent
                                    #[allow(clippy::float_cmp)]
                                    // Comparing against exact constant 1.0
                                    let res = pos_n == 1.0;
                                    res
                                };
                                if is_one_exponent {
                                    new_den_factors[j] = Arc::clone(&base_i);
                                } else {
                                    new_den_factors[j] = Arc::new(Expr::pow_from_arcs(
                                        Arc::clone(&base_i),
                                        Arc::new(Expr::number(pos_n)),
                                    ));
                                }
                            }
                            matched = true;
                            break;
                        }
                        new_num_factors[i] = Arc::new(Expr::pow_from_arcs(
                            Arc::clone(&base_i),
                            Arc::new(simplified_exp),
                        ));
                        new_den_factors.remove(j);
                        matched = true;
                        break;
                    }
                }

                if !matched {
                    i += 1;
                }
            }

            // Add coefficients back
            let num_not_one = {
                // Exact check for 1.0 coefficient
                #[allow(clippy::float_cmp)] // Comparing against exact constant 1.0
                let res = num_coeff != 1.0;
                res
            };
            if num_not_one {
                new_num_factors.insert(0, Arc::new(Expr::number(num_coeff)));
            }
            let den_not_one = {
                // Exact check for 1.0 coefficient
                #[allow(clippy::float_cmp)] // Comparing against exact constant 1.0
                let res = den_coeff != 1.0;
                res
            };
            if den_not_one {
                new_den_factors.insert(0, Arc::new(Expr::number(den_coeff)));
            }

            // Rebuild numerator
            let new_num = if new_num_factors.is_empty() {
                Arc::new(Expr::number(1.0))
            } else if new_num_factors.len() == 1 {
                new_num_factors
                    .into_iter()
                    .next()
                    .expect("Vector guaranteed to have exactly one element")
            } else {
                Arc::new(Expr::product_from_arcs(new_num_factors))
            };

            // Rebuild denominator
            let new_den = if new_den_factors.is_empty() {
                Arc::new(Expr::number(1.0))
            } else if new_den_factors.len() == 1 {
                new_den_factors
                    .into_iter()
                    .next()
                    .expect("Filtered vector guaranteed to have one element")
            } else {
                Arc::new(Expr::product_from_arcs(new_den_factors))
            };

            if let AstKind::Number(n) = &new_den.kind {
                let is_one_denominator = {
                    // Exact check for 1.0 denominator
                    #[allow(clippy::float_cmp)] // Comparing against exact constant 1.0
                    let res = *n == 1.0;
                    res
                };
                if is_one_denominator {
                    return Some(new_num);
                }
            }

            let res = Expr::div_from_arcs(new_num, new_den);
            if res.id != expr.id && res != **expr {
                return Some(Arc::new(res));
            }
        }
        None
    }
}

// Helper function to get factors from an expression
fn get_factors_arcs(expr: &Arc<Expr>) -> Vec<Arc<Expr>> {
    match &expr.kind {
        AstKind::Product(factors) => factors.clone(),
        _ => vec![Arc::clone(expr)],
    }
}

/// Rule for perfect squares: a^2 + 2ab + b^2 -> (a+b)^2
pub struct PerfectSquareRule;

impl Rule for PerfectSquareRule {
    fn name(&self) -> &'static str {
        "perfect_square"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Algebraic
    }

    fn applies_to(&self) -> &'static [ExprKind] {
        &[ExprKind::Sum, ExprKind::Poly]
    }

    #[allow(clippy::too_many_lines)] // Complex polynomial GCD logic
    fn apply(&self, expr: &Arc<Expr>, _context: &RuleContext) -> Option<Arc<Expr>> {
        fn apply_inner(expr: &Arc<Expr>) -> Option<Arc<Expr>> {
            fn extract_coeff_and_factors(term: &Arc<Expr>) -> (f64, Vec<Arc<Expr>>) {
                match &term.kind {
                    AstKind::Product(factors) => {
                        let mut coeff = 1.0;
                        let mut non_numeric: Vec<Arc<Expr>> = Vec::new();
                        for f in factors {
                            if let AstKind::Number(n) = &f.kind {
                                coeff *= n;
                            } else {
                                non_numeric.push(Arc::clone(f));
                            }
                        }
                        (coeff, non_numeric)
                    }
                    AstKind::Number(n) => (*n, vec![]),
                    _ => (1.0, vec![Arc::clone(term)]),
                }
            }

            // Extract terms from Sum or Poly, handling Sum([constant, Poly]) case
            let terms_vec: Vec<Arc<Expr>> = match &expr.kind {
                AstKind::Sum(terms) if terms.len() == 3 => terms.clone(),
                AstKind::Sum(terms) if terms.len() == 2 => {
                    // Check for Sum([constant, Poly]) pattern - flatten to 3 terms
                    let mut flat_terms = Vec::new();
                    for t in terms {
                        if let AstKind::Poly(poly) = &t.kind {
                            flat_terms.extend(poly.to_expr_terms().into_iter().map(Arc::new));
                        } else {
                            flat_terms.push(Arc::clone(t));
                        }
                    }
                    if flat_terms.len() == 3 {
                        flat_terms
                    } else {
                        return None;
                    }
                }
                AstKind::Poly(poly) if poly.terms().len() == 3 => {
                    // Convert Poly terms to Expr for analysis
                    poly.to_expr_terms().into_iter().map(Arc::new).collect()
                }
                _ => return None,
            };

            let mut square_terms: Vec<(f64, Arc<Expr>)> = Vec::new();
            let mut linear_terms: Vec<(f64, Arc<Expr>, Arc<Expr>)> = Vec::new();

            for term in &terms_vec {
                match &term.kind {
                    AstKind::Pow(base, exp) => {
                        if let AstKind::Number(n) = &exp.kind
                            && (*n - 2.0).abs() < 1e-10
                        {
                            square_terms.push((1.0, Arc::clone(base)));
                            continue;
                        }
                        linear_terms.push((1.0, Arc::clone(term), Arc::new(Expr::number(1.0))));
                    }
                    AstKind::Number(n) => {
                        // Check if number is a perfect square (positive)
                        if *n > 0.0 {
                            let sqrt_n = n.sqrt();
                            if (sqrt_n - sqrt_n.round()).abs() < 1e-10 {
                                // It's a perfect square constant, treat as square term (coeff=1, base=sqrt(n))
                                // We treat '9' as 1*3^2. So coeff=1.0, base=Number(3)
                                square_terms.push((1.0, Arc::new(Expr::number(sqrt_n.round()))));
                                continue;
                            }
                        }
                        // Treat constant as linear term coeff? Or ignore?
                        // If it's not a square term, it must be part of linear term construction?
                        // Actually linear term is 2ab. If we have constant 9, it's b^2.
                        // If we have constant 6, maybe 2*3?
                        // For now fallback to linear term with var=1
                        linear_terms.push((1.0, Arc::clone(term), Arc::new(Expr::number(1.0))));
                    }
                    AstKind::Product(_) => {
                        let (coeff, factors) = extract_coeff_and_factors(term);

                        if factors.len() == 1 {
                            if let AstKind::Pow(base, exp) = &factors[0].kind
                                && let AstKind::Number(n) = &exp.kind
                                && (*n - 2.0).abs() < 1e-10
                            {
                                square_terms.push((coeff, Arc::clone(base)));
                                continue;
                            }
                            linear_terms.push((
                                coeff,
                                Arc::clone(&factors[0]),
                                Arc::new(Expr::number(1.0)),
                            ));
                        } else if factors.len() == 2 {
                            linear_terms.push((
                                coeff,
                                Arc::clone(&factors[0]),
                                Arc::clone(&factors[1]),
                            ));
                        } else {
                            // Complex product -> treat as linear term unit?
                            linear_terms.push((
                                coeff,
                                Arc::clone(term),
                                Arc::new(Expr::number(1.0)),
                            ));
                        }
                    }
                    _ => {
                        linear_terms.push((1.0, Arc::clone(term), Arc::new(Expr::number(1.0))));
                    }
                }
            }

            // Case 1: Standard perfect square a^2 + 2*a*b + b^2
            if square_terms.len() == 2 && linear_terms.len() == 1 {
                let (c1, a) = &square_terms[0];
                let (c2, b) = &square_terms[1];
                let (cross_coeff, cross_a, cross_b) = &linear_terms[0];

                let sqrt_c1 = c1.sqrt();
                let sqrt_c2 = c2.sqrt();

                if (sqrt_c1 - sqrt_c1.round()).abs() < 1e-10
                    && (sqrt_c2 - sqrt_c2.round()).abs() < 1e-10
                {
                    let expected_cross_abs = (2.0 * sqrt_c1 * sqrt_c2).abs();
                    let cross_coeff_abs = cross_coeff.abs();

                    // Check if variable parts match
                    // We need to match {a, b} with {cross_a, cross_b} ignoring order
                    let match_direct =
                        (a == cross_a && b == cross_b) || (a == cross_b && b == cross_a);

                    // Also check if one of cross factors is 1 (implicit) and matches
                    // e.g. x^2 + 2x + 1. a=x, b=1. Linear term 2x -> cross_a=x, cross_b=1.

                    if (expected_cross_abs - cross_coeff_abs).abs() < 1e-10 && match_direct {
                        let sign = cross_coeff.signum();

                        let term_a = if (sqrt_c1 - 1.0).abs() < 1e-10 {
                            Arc::clone(a)
                        } else {
                            Arc::new(Expr::product_from_arcs(vec![
                                Arc::new(Expr::number(sqrt_c1.round())),
                                Arc::clone(a),
                            ]))
                        };

                        let term_b = if (sqrt_c2 - 1.0).abs() < 1e-10 {
                            Arc::clone(b)
                        } else {
                            Arc::new(Expr::product_from_arcs(vec![
                                Arc::new(Expr::number(sqrt_c2.round())),
                                Arc::clone(b),
                            ]))
                        };

                        let inner = if sign > 0.0 {
                            Expr::sum_from_arcs(vec![term_a, term_b])
                        } else {
                            let neg_b =
                                Expr::product_from_arcs(vec![Arc::new(Expr::number(-1.0)), term_b]);
                            Expr::sum_from_arcs(vec![term_a, Arc::new(neg_b)])
                        };

                        return Some(Arc::new(Expr::pow_from_arcs(
                            Arc::new(inner),
                            Arc::new(Expr::number(2.0)),
                        )));
                    }
                }
            }
            None
        }
        apply_inner(expr)
    }
}

rule_arc!(
    FactorDifferenceOfSquaresRule,
    "factor_difference_of_squares",
    46,
    Algebraic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        // Look for a^2 - b^2 pattern in Sum form
        // In n-ary, this is Sum([a^2, Product([-1, b^2])])

        fn get_square_root_form(e: &Arc<Expr>) -> Option<Arc<Expr>> {
            if let AstKind::Pow(base, exp) = &e.kind {
                if let AstKind::Number(n) = &exp.kind {
                    if (*n - 2.0).abs() < 1e-10 {
                        return Some(Arc::clone(base));
                    } else if n.fract() == 0.0 && *n > 2.0 && (n / 2.0).fract() == 0.0 {
                        let half_exp = n / 2.0;
                        return Some(Arc::new(Expr::pow_from_arcs(
                            Arc::clone(base),
                            Arc::new(Expr::number(half_exp)),
                        )));
                    }
                }
            } else if let AstKind::Number(n) = &e.kind
                && *n > 0.0
            {
                let sqrt_n = n.sqrt();
                if (sqrt_n - sqrt_n.round()).abs() < 1e-10 {
                    return Some(Arc::new(Expr::number(sqrt_n.round())));
                }
            }
            None
        }

        // Check if term is -1 * (something^2) i.e., Product([-1, x^2])
        // OR just a negative number -c
        fn extract_negated_square(term: &Arc<Expr>) -> Option<Arc<Expr>> {
            if let AstKind::Product(factors) = &term.kind {
                if factors.len() == 2
                    && let AstKind::Number(n) = &factors[0].kind
                    && (*n + 1.0).abs() < 1e-10
                {
                    // Note: rules passed to rule! get &Expr, but here we work with Arcs
                    // We need get_square_root_form to take &Arc<Expr>
                    return get_square_root_form(&factors[1]);
                }
            } else if let AstKind::Number(n) = &term.kind
                && *n < 0.0
            {
                let pos_n = -n;
                let sqrt_n = pos_n.sqrt();
                if (sqrt_n - sqrt_n.round()).abs() < 1e-10 {
                    return Some(Arc::new(Expr::number(sqrt_n.round())));
                }
            }
            None
        }

        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2
        {
            let t1 = &terms[0];
            let t2 = &terms[1];

            // Try t1 = a^2, t2 = -b^2
            if let (Some(sqrt_a), Some(sqrt_b)) =
                (get_square_root_form(t1), extract_negated_square(t2))
            {
                // a^2 - b^2 = (a-b)(a+b)
                // Note: canonical order usually puts numbers first, so (a-b)(a+b) might become (-b+a)(a+b)
                // We'll construct (a-b)(a+b)
                let diff_term = Expr::sum_from_arcs(vec![
                    Arc::clone(&sqrt_a),
                    Arc::new(Expr::product_from_arcs(vec![
                        Arc::new(Expr::number(-1.0)),
                        Arc::clone(&sqrt_b),
                    ])),
                ]);
                let sum_term = Expr::sum_from_arcs(vec![sqrt_a, sqrt_b]);
                return Some(Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(diff_term),
                    Arc::new(sum_term),
                ])));
            }

            // Try t2 = a^2, t1 = -b^2 (reversed)
            if let (Some(sqrt_a), Some(sqrt_b)) =
                (get_square_root_form(t2), extract_negated_square(t1))
            {
                // a^2 - b^2 = (a-b)(a+b)
                let diff_term = Expr::sum_from_arcs(vec![
                    Arc::clone(&sqrt_a),
                    Arc::new(Expr::product_from_arcs(vec![
                        Arc::new(Expr::number(-1.0)),
                        Arc::clone(&sqrt_b),
                    ])),
                ]);
                let sum_term = Expr::sum_from_arcs(vec![sqrt_a, sqrt_b]);
                return Some(Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(diff_term),
                    Arc::new(sum_term),
                ])));
            }
        }

        None
    }
);

rule_arc!(
    NumericGcdFactoringRule,
    "numeric_gcd_factoring",
    42,
    Algebraic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind {
            let terms_vec: Vec<Arc<Expr>> = terms.clone();

            // Extract coefficients and variables
            let mut coeffs_and_terms = Vec::new();
            for term in &terms_vec {
                match &term.kind {
                    AstKind::Product(factors) => {
                        let mut coeff = 1.0;
                        let mut non_numeric: Vec<Arc<Expr>> = Vec::new();
                        for f in factors {
                            if let AstKind::Number(n) = &f.kind {
                                coeff *= n;
                            } else {
                                non_numeric.push(Arc::clone(f));
                            }
                        }
                        let var_part = if non_numeric.is_empty() {
                            Arc::new(Expr::number(1.0))
                        } else if non_numeric.len() == 1 {
                            non_numeric
                                .into_iter()
                                .next()
                                .expect("Non-numeric factors guaranteed to have one element")
                        } else {
                            Arc::new(Expr::product_from_arcs(non_numeric))
                        };
                        coeffs_and_terms.push((coeff, var_part));
                    }
                    AstKind::Number(n) => {
                        coeffs_and_terms.push((*n, Arc::new(Expr::number(1.0))));
                    }
                    _ => {
                        coeffs_and_terms.push((1.0, Arc::clone(term)));
                    }
                }
            }

            // Find GCD of coefficients
            let coeffs: Vec<i64> = coeffs_and_terms
                .iter()
                .map(|(c, _)| {
                    // GCD calculation safe for small integers, precision loss handled
                    #[allow(clippy::cast_possible_truncation)]
                    // Safe: checked fract()==0.0 before cast
                    (*c as i64)
                })
                .filter(|&c| c != 0)
                .collect();

            if coeffs.len() <= 1 {
                return None;
            }

            let gcd = coeffs
                .iter()
                .fold(coeffs[0], |a, &b| crate::simplification::helpers::gcd(a, b));

            if gcd <= 1 {
                return None;
            }

            // Factor out the GCD
            // i64->f64: GCD values in symbolic math are typically small integers
            #[allow(clippy::cast_precision_loss)] // GCD values are typically small integers
            let gcd_expr = Arc::new(Expr::number(gcd as f64));
            let mut new_terms = Vec::new();

            for (coeff, term) in coeffs_and_terms {
                // i64->f64: GCD values in symbolic math are typically small integers
                #[allow(clippy::cast_precision_loss)] // GCD values are typically small integers
                let new_coeff = coeff / (gcd as f64);
                if (new_coeff - 1.0).abs() < 1e-10 {
                    new_terms.push(term);
                } else if (new_coeff - (-1.0)).abs() < 1e-10 {
                    new_terms.push(Arc::new(Expr::product_from_arcs(vec![
                        Arc::new(Expr::number(-1.0)),
                        term,
                    ])));
                } else {
                    new_terms.push(Arc::new(Expr::product_from_arcs(vec![
                        Arc::new(Expr::number(new_coeff)),
                        term,
                    ])));
                }
            }

            let factored_terms = if new_terms.len() == 1 {
                new_terms
                    .into_iter()
                    .next()
                    .expect("New terms collection guaranteed to have one element")
            } else {
                Arc::new(Expr::sum_from_arcs(new_terms))
            };
            Some(Arc::new(Expr::product_from_arcs(vec![
                gcd_expr,
                factored_terms,
            ])))
        } else {
            None
        }
    }
);

/// Count how many times a factor appears in an expression
fn count_factor_occurrences(expr: &Expr, factor: &Expr) -> usize {
    match &expr.kind {
        AstKind::Product(factors) => factors.iter().filter(|f| f.as_ref() == factor).count(),
        _ if expr == factor => 1,
        _ => 0,
    }
}

/// Remove a list of factors from an expression (handles duplicates correctly)
fn remove_factors_by_list(expr: &Expr, factors_to_remove: &[Arc<Expr>]) -> Expr {
    match &expr.kind {
        AstKind::Product(expr_factors) => {
            // Build a mutable list of factors to remove
            let mut to_remove: Vec<&Expr> = factors_to_remove.iter().map(AsRef::as_ref).collect();

            // Filter factors - for each matching factor, only remove ONE occurrence
            let mut remaining_factors: Vec<Arc<Expr>> = Vec::new();
            for f in expr_factors {
                if let Some(pos) = to_remove.iter().position(|&r| r == f.as_ref()) {
                    to_remove.remove(pos); // Remove only the first match
                } else {
                    remaining_factors.push(Arc::clone(f));
                }
            }

            match remaining_factors.len() {
                0 => Expr::number(1.0),
                1 => (*remaining_factors[0]).clone(),
                _ => Expr::product_from_arcs(remaining_factors),
            }
        }
        _ => {
            // If the expression is not a multiplication, check if it matches
            // any single factor in the list
            if factors_to_remove.len() == 1 && expr == factors_to_remove[0].as_ref() {
                Expr::number(1.0)
            } else if factors_to_remove.iter().any(|f| expr == f.as_ref()) {
                // Remove one matching factor, return 1
                Expr::number(1.0)
            } else {
                expr.clone()
            }
        }
    }
}

rule_arc!(
    CommonTermFactoringRule,
    "common_term_factoring",
    40,
    Algebraic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind {
            if terms.len() < 2 {
                return None;
            }

            // Find common factors across all terms
            // We need to track factor COUNTS, not just presence

            // Get factors from first term with their counts
            let first_factors: Vec<Arc<Expr>> = match &terms[0].kind {
                AstKind::Product(factors) => factors.clone(),
                _ => vec![Arc::clone(&terms[0])],
            };

            // Count occurrences of each factor in first term
            let mut first_factor_counts: Vec<(Arc<Expr>, usize)> = Vec::new();
            for f in &first_factors {
                if let Some(entry) = first_factor_counts
                    .iter_mut()
                    .find(|(e, _)| e.as_ref() == f.as_ref())
                {
                    entry.1 += 1;
                } else {
                    first_factor_counts.push((Arc::clone(f), 1));
                }
            }

            // For each unique factor, find the minimum count across all terms
            let mut common_factors: Vec<Arc<Expr>> = Vec::new();

            for (factor, first_count) in &first_factor_counts {
                // Find minimum count of this factor across all other terms
                let mut min_count = *first_count;

                for term in &terms[1..] {
                    let term_count = count_factor_occurrences(term, factor);
                    min_count = min_count.min(term_count);
                    if min_count == 0 {
                        break;
                    }
                }

                // Add this factor `min_count` times to common_factors
                for _ in 0..min_count {
                    common_factors.push(Arc::clone(factor));
                }
            }

            if common_factors.is_empty() {
                return None;
            }

            // Don't factor out just -1 alone - it doesn't simplify the expression
            // and creates less canonical form like -(a+b) instead of -a-b
            if common_factors.len() == 1
                && let AstKind::Number(n) = &common_factors[0].kind
                && (*n + 1.0).abs() < 1e-10
            {
                return None;
            }

            // Factor out common factors
            let common_part = if common_factors.len() == 1 {
                Arc::clone(&common_factors[0])
            } else {
                Arc::new(Expr::product_from_arcs(common_factors.clone()))
            };

            let mut remaining_terms = Vec::new();
            for term in terms {
                remaining_terms.push(Arc::new(remove_factors_by_list(term, &common_factors)));
            }

            let remaining_sum = if remaining_terms.len() == 1 {
                remaining_terms
                    .into_iter()
                    .next()
                    .expect("remaining_terms has exactly one element")
            } else {
                Arc::new(Expr::sum_from_arcs(remaining_terms))
            };
            Some(Arc::new(Expr::product_from_arcs(vec![
                common_part,
                remaining_sum,
            ])))
        } else {
            None
        }
    }
);

rule_arc!(
    CommonPowerFactoringRule,
    "common_power_factoring",
    43,
    Algebraic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind {
            if terms.len() < 2 {
                return None;
            }

            // Collect all (base, exponent) pairs from power terms
            let mut base_exponents: std::collections::HashMap<u64, Vec<(f64, Arc<Expr>)>> =
                std::collections::HashMap::new();

            for term in terms {
                let (_, base_expr) = crate::simplification::helpers::extract_coeff(term);

                if let AstKind::Pow(base, exp) = &base_expr.kind {
                    if let AstKind::Number(exp_val) = &exp.kind
                        && *exp_val > 0.0
                        && exp_val.fract() == 0.0
                    {
                        let base_key = crate::simplification::helpers::get_term_hash(base);
                        base_exponents
                            .entry(base_key)
                            .or_default()
                            .push((*exp_val, Arc::clone(term)));
                    }
                } else if let AstKind::Symbol(_s) = &base_expr.kind {
                    // Use get_term_hash for consistency
                    let base_key = crate::simplification::helpers::get_term_hash(&base_expr);
                    base_exponents
                        .entry(base_key)
                        .or_default()
                        .push((1.0, Arc::clone(term)));
                }
            }

            // Find a base that appears in ALL terms with different exponents
            for exp_terms in base_exponents.values() {
                if exp_terms.len() == terms.len() && exp_terms.len() >= 2 {
                    let exponents: Vec<f64> = exp_terms.iter().map(|(e, _)| *e).collect();
                    let min_exp = exponents.iter().copied().fold(f64::INFINITY, f64::min);

                    if exponents.iter().all(|e| (*e - min_exp).abs() < 1e-10) {
                        continue;
                    }

                    if min_exp >= 1.0 {
                        let sample_term = &exp_terms[0].1;
                        let (_, sample_base) =
                            crate::simplification::helpers::extract_coeff(sample_term);

                        let base = if let AstKind::Pow(b, _) = &sample_base.kind {
                            Arc::clone(b)
                        } else {
                            Arc::new(sample_base)
                        };

                        let common_factor = if (min_exp - 1.0).abs() < 1e-10 {
                            Arc::clone(&base)
                        } else {
                            Arc::new(Expr::pow_from_arcs(
                                Arc::clone(&base),
                                Arc::new(Expr::number(min_exp)),
                            ))
                        };

                        let mut remaining_terms = Vec::new();

                        for term in terms {
                            let (coeff, base_expr) =
                                crate::simplification::helpers::extract_coeff(term);

                            let new_exp = if let AstKind::Pow(_, exp) = &base_expr.kind {
                                if let AstKind::Number(e) = &exp.kind {
                                    *e - min_exp
                                } else {
                                    continue;
                                }
                            } else {
                                1.0 - min_exp
                            };

                            let remaining = if new_exp.abs() < 1e-10 {
                                Arc::new(Expr::number(coeff))
                            } else if (new_exp - 1.0).abs() < 1e-10 {
                                if (coeff - 1.0).abs() < 1e-10 {
                                    Arc::clone(&base)
                                } else {
                                    Arc::new(Expr::product_from_arcs(vec![
                                        Arc::new(Expr::number(coeff)),
                                        Arc::clone(&base),
                                    ]))
                                }
                            } else {
                                let power = Arc::new(Expr::pow_from_arcs(
                                    Arc::clone(&base),
                                    Arc::new(Expr::number(new_exp)),
                                ));
                                if (coeff - 1.0).abs() < 1e-10 {
                                    power
                                } else {
                                    Arc::new(Expr::product_from_arcs(vec![
                                        Arc::new(Expr::number(coeff)),
                                        power,
                                    ]))
                                }
                            };

                            remaining_terms.push(remaining);
                        }

                        let remaining_sum = if remaining_terms.len() == 1 {
                            remaining_terms
                                .into_iter()
                                .next()
                                .expect("remaining_terms has exactly one element")
                        } else {
                            Arc::new(Expr::sum_from_arcs(remaining_terms))
                        };
                        return Some(Arc::new(Expr::product_from_arcs(vec![
                            common_factor,
                            remaining_sum,
                        ])));
                    }
                }
            }
        }

        None
    }
);

rule_arc!(
    PerfectCubeRule,
    "perfect_cube",
    40,
    Algebraic,
    &[ExprKind::Sum, ExprKind::Poly],
    |expr: &Expr, _context: &RuleContext| {
        // Pattern: a^3 + 3a^2b + 3ab^2 + b^3 = (a+b)^3
        // Pattern: a^3 - 3a^2b + 3ab^2 - b^3 = (a-b)^3
        // Also: a^3 + b^3 = (a+b)(a^2 - ab + b^2) and a^3 - b^3 = (a-b)(a^2 + ab + b^2)

        // Extract terms from Sum or Poly
        let terms: Vec<Arc<Expr>> = match &expr.kind {
            AstKind::Sum(ts) if ts.len() == 2 => ts.clone(),
            AstKind::Poly(poly) if poly.terms().len() == 2 => {
                poly.to_expr_terms().into_iter().map(Arc::new).collect()
            }
            _ => return None,
        };

        // Check for sum/difference of cubes: a^3 + b^3 or a^3 - b^3
        if terms.len() == 2 {
            fn get_cube_root(e: &Arc<Expr>) -> Option<Arc<Expr>> {
                if let AstKind::Pow(base, exp) = &e.kind
                    && let AstKind::Number(n) = &exp.kind
                {
                    // Exact check for cube exponent
                    #[allow(clippy::float_cmp)] // Comparing against exact constant 3.0
                    let is_cube = *n == 3.0;
                    if is_cube {
                        return Some(Arc::clone(base));
                    }
                }
                None
            }

            fn extract_negated(term: &Arc<Expr>) -> Option<Arc<Expr>> {
                if let AstKind::Product(factors) = &term.kind
                    && factors.len() == 2
                    && let AstKind::Number(n) = &factors[0].kind
                    && (*n + 1.0).abs() < 1e-10
                {
                    return Some(Arc::clone(&factors[1]));
                }
                None
            }

            let t1 = &terms[0];
            let t2 = &terms[1];

            // a^3 + b^3 = (a+b)(a^2 - ab + b^2)
            if let (Some(a), Some(b)) = (get_cube_root(t1), get_cube_root(t2)) {
                let sum = Expr::sum_from_arcs(vec![Arc::clone(&a), Arc::clone(&b)]);
                let a2 = Arc::new(Expr::pow_from_arcs(
                    Arc::clone(&a),
                    Arc::new(Expr::number(2.0)),
                ));
                let ab = Arc::new(Expr::product_from_arcs(vec![
                    Arc::clone(&a),
                    Arc::clone(&b),
                ]));
                let b2 = Arc::new(Expr::pow_from_arcs(
                    Arc::clone(&b),
                    Arc::new(Expr::number(2.0)),
                ));
                let neg_ab = Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(Expr::number(-1.0)),
                    ab,
                ]));
                let trinomial = Expr::sum_from_arcs(vec![a2, neg_ab, b2]);
                return Some(Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(sum),
                    Arc::new(trinomial),
                ])));
            }

            // a^3 + (-b^3) = a^3 - b^3 = (a-b)(a^2 + ab + b^2)
            if let (Some(a), Some(neg_inner)) = (get_cube_root(t1), extract_negated(t2))
                && let Some(b) = get_cube_root(&neg_inner)
            {
                let neg_b = Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(Expr::number(-1.0)),
                    Arc::clone(&b),
                ]));
                let diff = Expr::sum_from_arcs(vec![Arc::clone(&a), neg_b]);
                let a2 = Arc::new(Expr::pow_from_arcs(
                    Arc::clone(&a),
                    Arc::new(Expr::number(2.0)),
                ));
                let ab = Arc::new(Expr::product_from_arcs(vec![
                    Arc::clone(&a),
                    Arc::clone(&b),
                ]));
                let b2 = Arc::new(Expr::pow_from_arcs(
                    Arc::clone(&b),
                    Arc::new(Expr::number(2.0)),
                ));
                let trinomial = Expr::sum_from_arcs(vec![a2, ab, b2]);
                return Some(Arc::new(Expr::product_from_arcs(vec![
                    Arc::new(diff),
                    Arc::new(trinomial),
                ])));
            }
        }
        None
    }
);

// =============================================================================
// POLYNOMIAL GCD SIMPLIFICATION RULE
// =============================================================================

/// Rule for simplifying fractions using polynomial GCD
/// Handles cases like (x²-1)/(x-1) → x+1
pub struct PolyGcdSimplifyRule;

impl Rule for PolyGcdSimplifyRule {
    fn name(&self) -> &'static str {
        "poly_gcd_simplify"
    }

    fn priority(&self) -> i32 {
        // Lower priority than FractionCancellationRule (76) - runs after term-based cancellation
        74
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Algebraic
    }

    fn applies_to(&self) -> &'static [ExprKind] {
        &[ExprKind::Div]
    }

    fn apply(&self, expr: &Arc<Expr>, _context: &RuleContext) -> Option<Arc<Expr>> {
        if let AstKind::Div(num, den) = &expr.kind {
            // Try to convert both numerator and denominator to polynomials
            let num_poly = Polynomial::try_from_expr(num)?;
            let den_poly = Polynomial::try_from_expr(den)?;

            // Skip if either is constant (simpler rules handle that)
            if num_poly.is_constant() || den_poly.is_constant() {
                return None;
            }

            // Compute GCD
            let gcd = num_poly.gcd(&den_poly)?;

            // If GCD is constant (1), no simplification possible
            if gcd.is_constant() {
                return None;
            }

            // Divide both by GCD
            let (new_num, num_rem) = num_poly.div_rem(&gcd)?;
            let (new_den, den_rem) = den_poly.div_rem(&gcd)?;

            // Should divide evenly
            if !num_rem.is_zero() || !den_rem.is_zero() {
                return None;
            }

            // Convert back to expressions
            let new_num_expr = new_num.to_expr();
            let new_den_expr = new_den.to_expr();

            // If denominator is 1, just return numerator
            if new_den_expr.is_one_num() {
                return Some(Arc::new(new_num_expr));
            }

            Some(Arc::new(Expr::div_expr(new_num_expr, new_den_expr)))
        } else {
            None
        }
    }
}
