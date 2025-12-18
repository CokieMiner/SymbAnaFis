use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use crate::{Expr, ExprKind as AstKind};
use std::sync::Arc;

/// Rule for cancelling common terms in fractions: (a*b)/(a*c) -> b/c
/// Also handles powers: x^a / x^b -> x^(a-b)
pub(crate) struct FractionCancellationRule;

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

    fn apply(&self, expr: &Arc<Expr>, context: &RuleContext) -> Option<Arc<Expr>> {
        // For Product expressions, check if there's a Div nested inside
        if let AstKind::Product(factors) = &expr.kind {
            // Look for a Div among the factors
            for (i, factor) in factors.iter().enumerate() {
                if let AstKind::Div(num, den) = &factor.kind {
                    // Combine all other factors with numerator
                    let mut new_num_factors: Vec<Expr> = factors
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, f)| (**f).clone())
                        .collect();
                    new_num_factors.push((**num).clone());

                    let combined_num = if new_num_factors.len() == 1 {
                        new_num_factors.into_iter().next().unwrap()
                    } else {
                        Expr::product(new_num_factors)
                    };

                    let new_div = Expr::div_expr(combined_num, (**den).clone());
                    return self.apply(&Arc::new(new_div), context);
                }
            }
            return None;
        }

        if let AstKind::Div(u, v) = &expr.kind {
            // Get factors from numerator and denominator
            let num_factors = get_factors(u);
            let den_factors = get_factors(v);

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

            // Helper to get base and exponent
            fn get_base_exp(e: &Expr) -> (Expr, Expr) {
                match &e.kind {
                    AstKind::Pow(b, exp) => (b.as_ref().clone(), exp.as_ref().clone()),
                    AstKind::FunctionCall { name, args } if args.len() == 1 => {
                        if name == "sqrt" {
                            (args[0].clone(), Expr::number(0.5))
                        } else if name == "cbrt" {
                            (
                                args[0].clone(),
                                Expr::div_expr(Expr::number(1.0), Expr::number(3.0)),
                            )
                        } else {
                            (e.clone(), Expr::number(1.0))
                        }
                    }
                    _ => (e.clone(), Expr::number(1.0)),
                }
            }

            fn is_safe_to_cancel(base: &Expr) -> bool {
                match &base.kind {
                    AstKind::Number(n) => n.abs() > 1e-10,
                    _ => false,
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
                            Expr::sum(vec![
                                exp_i.clone(),
                                Expr::product(vec![Expr::number(-1.0), exp_j.clone()]),
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
                                if n == 1.0 {
                                    new_num_factors[i] = base_i.clone();
                                } else {
                                    new_num_factors[i] = Expr::pow(base_i.clone(), Expr::number(n));
                                }
                                new_den_factors.remove(j);
                                matched = true;
                                break;
                            } else {
                                new_num_factors.remove(i);
                                let pos_n = -n;
                                if pos_n == 1.0 {
                                    new_den_factors[j] = base_i.clone();
                                } else {
                                    new_den_factors[j] =
                                        Expr::pow(base_i.clone(), Expr::number(pos_n));
                                }
                                matched = true;
                                break;
                            }
                        } else {
                            new_num_factors[i] = Expr::pow(base_i.clone(), simplified_exp);
                            new_den_factors.remove(j);
                            matched = true;
                            break;
                        }
                    }
                }

                if !matched {
                    i += 1;
                }
            }

            // Add coefficients back
            if num_coeff != 1.0 {
                new_num_factors.insert(0, Expr::number(num_coeff));
            }
            if den_coeff != 1.0 {
                new_den_factors.insert(0, Expr::number(den_coeff));
            }

            // Rebuild numerator
            let new_num = if new_num_factors.is_empty() {
                Expr::number(1.0)
            } else if new_num_factors.len() == 1 {
                new_num_factors.into_iter().next().unwrap()
            } else {
                Expr::product(new_num_factors)
            };

            // Rebuild denominator
            let new_den = if new_den_factors.is_empty() {
                Expr::number(1.0)
            } else if new_den_factors.len() == 1 {
                new_den_factors.into_iter().next().unwrap()
            } else {
                Expr::product(new_den_factors)
            };

            if let AstKind::Number(n) = &new_den.kind {
                if *n == 1.0 {
                    return Some(Arc::new(new_num));
                }
            }

            let res = Expr::div_expr(new_num, new_den);
            if res.id != expr.id && res != **expr {
                return Some(Arc::new(res));
            }
        }
        None
    }
}

// Helper function to get factors from an expression
fn get_factors(expr: &Expr) -> Vec<Expr> {
    match &expr.kind {
        AstKind::Product(factors) => factors.iter().map(|f| (**f).clone()).collect(),
        _ => vec![expr.clone()],
    }
}

/// Rule for perfect squares: a^2 + 2ab + b^2 -> (a+b)^2
pub(crate) struct PerfectSquareRule;

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
        &[ExprKind::Sum]
    }

    fn apply(&self, expr: &Arc<Expr>, _context: &RuleContext) -> Option<Arc<Expr>> {
        fn apply_inner(expr: &Expr) -> Option<Expr> {
            if let AstKind::Sum(terms) = &expr.kind {
                if terms.len() == 3 {
                    let terms_vec: Vec<Expr> = terms.iter().map(|t| (**t).clone()).collect();

                    let mut square_terms: Vec<(f64, Expr)> = Vec::new();
                    let mut linear_terms: Vec<(f64, Expr, Expr)> = Vec::new();
                    let mut constants = Vec::new();

                    fn extract_coeff_and_factors(term: &Expr) -> (f64, Vec<Expr>) {
                        match &term.kind {
                            AstKind::Product(factors) => {
                                let mut coeff = 1.0;
                                let mut non_numeric: Vec<Expr> = Vec::new();
                                for f in factors.iter() {
                                    if let AstKind::Number(n) = &f.kind {
                                        coeff *= n;
                                    } else {
                                        non_numeric.push((**f).clone());
                                    }
                                }
                                (coeff, non_numeric)
                            }
                            AstKind::Number(n) => (*n, vec![]),
                            _ => (1.0, vec![term.clone()]),
                        }
                    }

                    for term in &terms_vec {
                        match &term.kind {
                            AstKind::Pow(base, exp) => {
                                if let AstKind::Number(n) = &exp.kind {
                                    if (*n - 2.0).abs() < 1e-10 {
                                        square_terms.push((1.0, (**base).clone()));
                                        continue;
                                    }
                                }
                                linear_terms.push((1.0, term.clone(), Expr::number(1.0)));
                            }
                            AstKind::Number(n) => {
                                // Check if number is a perfect square (positive)
                                if *n > 0.0 {
                                    let sqrt_n = n.sqrt();
                                    if (sqrt_n - sqrt_n.round()).abs() < 1e-10 {
                                        // It's a perfect square constant, treat as square term (coeff=1, base=sqrt(n))
                                        // We treat '9' as 1*3^2. So coeff=1.0, base=Number(3)
                                        square_terms.push((1.0, Expr::number(sqrt_n.round())));
                                        continue;
                                    }
                                }
                                constants.push(*n);
                            }
                            AstKind::Product(_) => {
                                let (coeff, factors) = extract_coeff_and_factors(term);

                                if factors.len() == 1 {
                                    if let AstKind::Pow(base, exp) = &factors[0].kind {
                                        if let AstKind::Number(n) = &exp.kind {
                                            if (*n - 2.0).abs() < 1e-10 {
                                                square_terms.push((coeff, (**base).clone()));
                                                continue;
                                            }
                                        }
                                    }
                                    linear_terms.push((
                                        coeff,
                                        factors[0].clone(),
                                        Expr::number(1.0),
                                    ));
                                } else if factors.len() == 2 {
                                    linear_terms.push((
                                        coeff,
                                        factors[0].clone(),
                                        factors[1].clone(),
                                    ));
                                }
                            }
                            _ => {
                                linear_terms.push((1.0, term.clone(), Expr::number(1.0)));
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

                            if (expected_cross_abs - cross_coeff_abs).abs() < 1e-10 {
                                if (a == cross_a && b == cross_b) || (a == cross_b && b == cross_a)
                                {
                                    let sign = cross_coeff.signum();

                                    let term_a = if (sqrt_c1 - 1.0).abs() < 1e-10 {
                                        a.clone()
                                    } else {
                                        Expr::product(vec![
                                            Expr::number(sqrt_c1.round()),
                                            a.clone(),
                                        ])
                                    };

                                    let term_b = if (sqrt_c2 - 1.0).abs() < 1e-10 {
                                        b.clone()
                                    } else {
                                        Expr::product(vec![
                                            Expr::number(sqrt_c2.round()),
                                            b.clone(),
                                        ])
                                    };

                                    let inner = if sign > 0.0 {
                                        Expr::sum(vec![term_a, term_b])
                                    } else {
                                        Expr::sum(vec![
                                            term_a,
                                            Expr::product(vec![Expr::number(-1.0), term_b]),
                                        ])
                                    };

                                    return Some(Expr::pow(inner, Expr::number(2.0)));
                                }
                            }
                        }
                    }
                }
            }
            None
        }
        apply_inner(expr.as_ref()).map(Arc::new)
    }
}

rule!(
    FactorDifferenceOfSquaresRule,
    "factor_difference_of_squares",
    46,
    Algebraic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        // Look for a^2 - b^2 pattern in Sum form
        // In n-ary, this is Sum([a^2, Product([-1, b^2])])

        fn get_square_root_form(e: &Expr) -> Option<Expr> {
            if let AstKind::Pow(base, exp) = &e.kind {
                if let AstKind::Number(n) = &exp.kind {
                    if (*n - 2.0).abs() < 1e-10 {
                        return Some((**base).clone());
                    } else if n.fract() == 0.0 && *n > 2.0 && (n / 2.0).fract() == 0.0 {
                        let half_exp = n / 2.0;
                        return Some(Expr::pow((**base).clone(), Expr::number(half_exp)));
                    }
                }
            } else if let AstKind::Number(n) = &e.kind {
                if *n > 0.0 {
                    let sqrt_n = n.sqrt();
                    if (sqrt_n - sqrt_n.round()).abs() < 1e-10 {
                        return Some(Expr::number(sqrt_n.round()));
                    }
                }
            }
            None
        }

        if let AstKind::Sum(terms) = &expr.kind {
            if terms.len() == 2 {
                let t1 = &terms[0];
                let t2 = &terms[1];

                // Check if term is -1 * (something^2) i.e., Product([-1, x^2])
                // OR just a negative number -c
                fn extract_negated_square(term: &Expr) -> Option<Expr> {
                    if let AstKind::Product(factors) = &term.kind {
                        if factors.len() == 2 {
                            if let AstKind::Number(n) = &factors[0].kind {
                                if (*n + 1.0).abs() < 1e-10 {
                                    return get_square_root_form(&factors[1]);
                                }
                            }
                        }
                    } else if let AstKind::Number(n) = &term.kind {
                        if *n < 0.0 {
                            let pos_n = -n;
                            let sqrt_n = pos_n.sqrt();
                            if (sqrt_n - sqrt_n.round()).abs() < 1e-10 {
                                return Some(Expr::number(sqrt_n.round()));
                            }
                        }
                    }
                    None
                }

                // Try t1 = a^2, t2 = -b^2
                if let (Some(sqrt_a), Some(sqrt_b)) =
                    (get_square_root_form(t1), extract_negated_square(t2))
                {
                    // a^2 - b^2 = (a-b)(a+b)
                    // Note: canonical order usually puts numbers first, so (a-b)(a+b) might become (-b+a)(a+b)
                    // We'll construct (a-b)(a+b)
                    let diff_term = Expr::sum(vec![
                        sqrt_a.clone(),
                        Expr::product(vec![Expr::number(-1.0), sqrt_b.clone()]),
                    ]);
                    let sum_term = Expr::sum(vec![sqrt_a, sqrt_b]);
                    return Some(Expr::product(vec![diff_term, sum_term]));
                }

                // Try t2 = a^2, t1 = -b^2 (reversed)
                if let (Some(sqrt_a), Some(sqrt_b)) =
                    (get_square_root_form(t2), extract_negated_square(t1))
                {
                    // a^2 - b^2 = (a-b)(a+b)
                    let diff_term = Expr::sum(vec![
                        sqrt_a.clone(),
                        Expr::product(vec![Expr::number(-1.0), sqrt_b.clone()]),
                    ]);
                    let sum_term = Expr::sum(vec![sqrt_a, sqrt_b]);
                    return Some(Expr::product(vec![diff_term, sum_term]));
                }
            }
        }

        None
    }
);

rule!(
    NumericGcdFactoringRule,
    "numeric_gcd_factoring",
    42,
    Algebraic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind {
            let terms_vec: Vec<Expr> = terms.iter().map(|t| (**t).clone()).collect();

            // Extract coefficients and variables
            let mut coeffs_and_terms = Vec::new();
            for term in &terms_vec {
                match &term.kind {
                    AstKind::Product(factors) => {
                        let mut coeff = 1.0;
                        let mut non_numeric: Vec<Expr> = Vec::new();
                        for f in factors.iter() {
                            if let AstKind::Number(n) = &f.kind {
                                coeff *= n;
                            } else {
                                non_numeric.push((**f).clone());
                            }
                        }
                        let var_part = if non_numeric.is_empty() {
                            Expr::number(1.0)
                        } else if non_numeric.len() == 1 {
                            non_numeric.into_iter().next().unwrap()
                        } else {
                            Expr::product(non_numeric)
                        };
                        coeffs_and_terms.push((coeff, var_part));
                    }
                    AstKind::Number(n) => {
                        coeffs_and_terms.push((*n, Expr::number(1.0)));
                    }
                    _ => {
                        coeffs_and_terms.push((1.0, term.clone()));
                    }
                }
            }

            // Find GCD of coefficients
            let coeffs: Vec<i64> = coeffs_and_terms
                .iter()
                .map(|(c, _)| *c as i64)
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
            let gcd_expr = Expr::number(gcd as f64);
            let mut new_terms = Vec::new();

            for (coeff, term) in coeffs_and_terms {
                let new_coeff = coeff / (gcd as f64);
                if (new_coeff - 1.0).abs() < 1e-10 {
                    new_terms.push(term);
                } else if (new_coeff - (-1.0)).abs() < 1e-10 {
                    new_terms.push(Expr::product(vec![Expr::number(-1.0), term]));
                } else {
                    new_terms.push(Expr::product(vec![Expr::number(new_coeff), term]));
                }
            }

            let factored_terms = if new_terms.len() == 1 {
                new_terms.into_iter().next().unwrap()
            } else {
                Expr::sum(new_terms)
            };
            Some(Expr::product(vec![gcd_expr, factored_terms]))
        } else {
            None
        }
    }
);

rule!(
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
            let mut common_factors = Vec::new();

            // Start with factors from first term
            let first_factors: Vec<Arc<Expr>> = match &terms[0].kind {
                AstKind::Product(factors) => factors.clone(),
                _ => vec![terms[0].clone()],
            };

            for factor in first_factors.iter() {
                // Check if this factor appears in all other terms
                let mut all_have_factor = true;
                for term in &terms[1..] {
                    if !crate::simplification::helpers::contains_factor(term, factor) {
                        all_have_factor = false;
                        break;
                    }
                }
                if all_have_factor {
                    common_factors.push((**factor).clone());
                }
            }

            if common_factors.is_empty() {
                return None;
            }

            // Factor out common factors
            let common_part = if common_factors.len() == 1 {
                common_factors[0].clone()
            } else {
                Expr::product(common_factors)
            };

            let mut remaining_terms = Vec::new();
            for term in terms {
                remaining_terms.push(crate::simplification::helpers::remove_factors(
                    term,
                    &common_part,
                ));
            }

            let remaining_sum = if remaining_terms.len() == 1 {
                remaining_terms.into_iter().next().unwrap()
            } else {
                Expr::sum(remaining_terms)
            };
            Some(Expr::product(vec![common_part, remaining_sum]))
        } else {
            None
        }
    }
);

rule!(
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
                    if let AstKind::Number(exp_val) = &exp.kind {
                        if *exp_val > 0.0 && exp_val.fract() == 0.0 {
                            let base_key = crate::simplification::helpers::get_term_hash(base);
                            base_exponents
                                .entry(base_key)
                                .or_default()
                                .push((*exp_val, term.clone()));
                        }
                    }
                } else if let AstKind::Symbol(_s) = &base_expr.kind {
                    // Use get_term_hash for consistency
                    let base_key = crate::simplification::helpers::get_term_hash(&base_expr);
                    base_exponents
                        .entry(base_key)
                        .or_default()
                        .push((1.0, term.clone()));
                }
            }

            // Find a base that appears in ALL terms with different exponents
            for exp_terms in base_exponents.values() {
                if exp_terms.len() == terms.len() && exp_terms.len() >= 2 {
                    let exponents: Vec<f64> = exp_terms.iter().map(|(e, _)| *e).collect();
                    let min_exp = exponents.iter().cloned().fold(f64::INFINITY, f64::min);

                    if exponents.iter().all(|e| (*e - min_exp).abs() < 1e-10) {
                        continue;
                    }

                    if min_exp >= 1.0 {
                        let sample_term = &exp_terms[0].1;
                        let (_, sample_base) =
                            crate::simplification::helpers::extract_coeff(sample_term);

                        let base = if let AstKind::Pow(b, _) = &sample_base.kind {
                            (**b).clone()
                        } else {
                            sample_base.clone()
                        };

                        let common_factor = if (min_exp - 1.0).abs() < 1e-10 {
                            base.clone()
                        } else {
                            Expr::pow(base.clone(), Expr::number(min_exp))
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
                                Expr::number(coeff)
                            } else if (new_exp - 1.0).abs() < 1e-10 {
                                if (coeff - 1.0).abs() < 1e-10 {
                                    base.clone()
                                } else {
                                    Expr::product(vec![Expr::number(coeff), base.clone()])
                                }
                            } else {
                                let power = Expr::pow(base.clone(), Expr::number(new_exp));
                                if (coeff - 1.0).abs() < 1e-10 {
                                    power
                                } else {
                                    Expr::product(vec![Expr::number(coeff), power])
                                }
                            };

                            remaining_terms.push(remaining);
                        }

                        let remaining_sum = if remaining_terms.len() == 1 {
                            remaining_terms.into_iter().next().unwrap()
                        } else {
                            Expr::sum(remaining_terms)
                        };
                        return Some(Expr::product(vec![common_factor, remaining_sum]));
                    }
                }
            }
        }

        None
    }
);

rule!(
    PerfectCubeRule,
    "perfect_cube",
    40,
    Algebraic,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        // Pattern: a^3 + 3a^2b + 3ab^2 + b^3 = (a+b)^3
        // Pattern: a^3 - 3a^2b + 3ab^2 - b^3 = (a-b)^3
        // Also: a^3 + b^3 = (a+b)(a^2 - ab + b^2) and a^3 - b^3 = (a-b)(a^2 + ab + b^2)
        if let AstKind::Sum(terms) = &expr.kind {
            // Check for sum/difference of cubes: a^3 + b^3 or a^3 - b^3
            if terms.len() == 2 {
                fn get_cube_root(e: &Expr) -> Option<Expr> {
                    if let AstKind::Pow(base, exp) = &e.kind {
                        if let AstKind::Number(n) = &exp.kind {
                            if *n == 3.0 {
                                return Some((**base).clone());
                            }
                        }
                    }
                    None
                }

                fn extract_negated(term: &Expr) -> Option<Expr> {
                    if let AstKind::Product(factors) = &term.kind {
                        if factors.len() == 2 {
                            if let AstKind::Number(n) = &factors[0].kind {
                                if (*n + 1.0).abs() < 1e-10 {
                                    return Some((*factors[1]).clone());
                                }
                            }
                        }
                    }
                    None
                }

                let t1 = &terms[0];
                let t2 = &terms[1];

                // a^3 + b^3 = (a+b)(a^2 - ab + b^2)
                if let (Some(a), Some(b)) = (get_cube_root(t1), get_cube_root(t2)) {
                    let sum = Expr::sum(vec![a.clone(), b.clone()]);
                    let a2 = Expr::pow(a.clone(), Expr::number(2.0));
                    let ab = Expr::product(vec![a.clone(), b.clone()]);
                    let b2 = Expr::pow(b.clone(), Expr::number(2.0));
                    let neg_ab = Expr::product(vec![Expr::number(-1.0), ab]);
                    let trinomial = Expr::sum(vec![a2, neg_ab, b2]);
                    return Some(Expr::product(vec![sum, trinomial]));
                }

                // a^3 + (-b^3) = a^3 - b^3 = (a-b)(a^2 + ab + b^2)
                if let (Some(a), Some(neg_inner)) = (get_cube_root(t1), extract_negated(t2)) {
                    if let Some(b) = get_cube_root(&neg_inner) {
                        let neg_b = Expr::product(vec![Expr::number(-1.0), b.clone()]);
                        let diff = Expr::sum(vec![a.clone(), neg_b]);
                        let a2 = Expr::pow(a.clone(), Expr::number(2.0));
                        let ab = Expr::product(vec![a.clone(), b.clone()]);
                        let b2 = Expr::pow(b.clone(), Expr::number(2.0));
                        let trinomial = Expr::sum(vec![a2, ab, b2]);
                        return Some(Expr::product(vec![diff, trinomial]));
                    }
                }
            }
        }
        None
    }
);
