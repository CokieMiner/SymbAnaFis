use super::helpers::{
    ExpTerm, extract_negated_term, is_double_of, match_alt_cosh_pattern, match_alt_sech_pattern,
    match_alt_sinh_pattern, match_cosh_pattern, match_e2x_minus_1_direct,
    match_e2x_minus_1_factored, match_e2x_plus_1, match_sinh_pattern_sub,
};
use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols as ks;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};

rule!(
    SinhFromExpRule,
    "sinh_from_exp",
    80,
    Hyperbolic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(numerator, denominator) = &expr.kind {
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
            let is_two = matches!(&denominator.kind, AstKind::Number(d) if *d == 2.0);

            if is_two
                && let AstKind::Sum(terms) = &numerator.kind
                && terms.len() == 2
            {
                let u = &terms[0];
                let v = &terms[1];

                // Check if v is negated
                if let Some(neg_inner) = extract_negated_term(v)
                    && let Some(x) = match_sinh_pattern_sub(u, &neg_inner)
                {
                    return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.sinh), x));
                }
                if let Some(neg_inner) = extract_negated_term(u)
                    && let Some(x) = match_sinh_pattern_sub(v, &neg_inner)
                {
                    return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.sinh), x));
                }
            }

            if let Some(x) = match_alt_sinh_pattern(numerator, denominator) {
                return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.sinh), x));
            }
        }
        None
    }
);

rule!(
    CoshFromExpRule,
    "cosh_from_exp",
    80,
    Hyperbolic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(numerator, denominator) = &expr.kind {
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
            let is_two = matches!(&denominator.kind, AstKind::Number(d) if *d == 2.0);

            if is_two
                && let AstKind::Sum(terms) = &numerator.kind
                && terms.len() == 2
                && let Some(x) = match_cosh_pattern(&terms[0], &terms[1])
            {
                return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.cosh), x));
            }

            if let Some(x) = match_alt_cosh_pattern(numerator, denominator) {
                return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.cosh), x));
            }
        }
        None
    }
);

rule!(
    TanhFromExpRule,
    "tanh_from_exp",
    80,
    Hyperbolic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(numerator, denominator) = &expr.kind {
            // Check for (e^x - e^(-x)) / (e^x + e^(-x)) pattern
            let num_arg = if let AstKind::Sum(terms) = &numerator.kind {
                if terms.len() == 2 {
                    extract_negated_term(&terms[1])
                        .and_then(|negated| match_sinh_pattern_sub(&terms[0], &negated))
                        .or_else(|| {
                            extract_negated_term(&terms[0])
                                .and_then(|negated| match_sinh_pattern_sub(&terms[1], &negated))
                        })
                } else {
                    None
                }
            } else {
                None
            };

            let den_arg = if let AstKind::Sum(terms) = &denominator.kind {
                if terms.len() == 2 {
                    match_cosh_pattern(&terms[0], &terms[1])
                } else {
                    None
                }
            } else {
                None
            };

            if let (Some(n_arg), Some(d_arg)) = (num_arg, den_arg)
                && n_arg == d_arg
            {
                return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.tanh), n_arg));
            }

            if let Some(x_num) = match_e2x_minus_1_factored(numerator)
                && let Some(x_den) = match_e2x_plus_1(denominator)
                && x_num == x_den
            {
                return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.tanh), x_num));
            }

            if let Some(x_num) = match_e2x_minus_1_direct(numerator)
                && let Some(x_den) = match_e2x_plus_1(denominator)
                && x_num == x_den
            {
                return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.tanh), x_num));
            }
        }
        None
    }
);

rule!(
    SechFromExpRule,
    "sech_from_exp",
    80,
    Hyperbolic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(numerator, denominator) = &expr.kind {
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
            let is_two = matches!(&numerator.kind, AstKind::Number(n) if *n == 2.0);

            if is_two
                && let AstKind::Sum(terms) = &denominator.kind
                && terms.len() == 2
                && let Some(x) = match_cosh_pattern(&terms[0], &terms[1])
            {
                return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.sech), x));
            }

            if let Some(x) = match_alt_sech_pattern(numerator, denominator) {
                return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.sech), x));
            }
        }
        None
    }
);

rule!(
    CschFromExpRule,
    "csch_from_exp",
    80,
    Hyperbolic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(numerator, denominator) = &expr.kind {
            // Exact check for numerator 2.0 (csch definition)
            #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
            let is_two = matches!(&numerator.kind, AstKind::Number(n) if *n == 2.0);
            if is_two {
                // Check for Sum([u, Product([-1, v])]) pattern representing subtraction
                if let AstKind::Sum(terms) = &denominator.kind
                    && terms.len() == 2
                {
                    if let Some(negated) = extract_negated_term(&terms[1])
                        && let Some(x) = match_sinh_pattern_sub(&terms[0], &negated)
                    {
                        return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.csch), x));
                    }
                    if let Some(negated) = extract_negated_term(&terms[0])
                        && let Some(x) = match_sinh_pattern_sub(&terms[1], &negated)
                    {
                        return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.csch), x));
                    }
                }
            }

            if let AstKind::Product(factors) = &numerator.kind
                && factors.len() == 2
            {
                let (coeff, exp_term) = if let AstKind::Number(n) = &factors[0].kind {
                    (*n, &factors[1])
                } else if let AstKind::Number(n) = &factors[1].kind {
                    (*n, &factors[0])
                } else {
                    return None;
                };

                // Exact check for coefficient 2.0
                #[allow(clippy::float_cmp, reason = "Comparing against exact constant 2.0")]
                let is_two_coeff = coeff == 2.0;

                if is_two_coeff
                    && let Some(x) = ExpTerm::get_direct_exp_arg(exp_term)
                    && let AstKind::Sum(terms) = &denominator.kind
                    && terms.len() == 2
                {
                    let (minus_one, exp_term_denom) = if let AstKind::Number(n) = &terms[0].kind {
                        // Exact check for constant -1.0
                        #[allow(
                            clippy::float_cmp,
                            reason = "Comparing against exact constant -1.0"
                        )]
                        let is_neg_one = *n == -1.0;
                        if is_neg_one {
                            (Some(&terms[0]), &terms[1])
                        } else {
                            (None, &terms[0]) // Fallback, though we expect -1
                        }
                    } else if let AstKind::Number(n) = &terms[1].kind {
                        // Exact check for constant -1.0
                        #[allow(
                            clippy::float_cmp,
                            reason = "Comparing against exact constant -1.0"
                        )]
                        let is_neg_one = *n == -1.0;
                        if is_neg_one {
                            (Some(&terms[1]), &terms[0])
                        } else {
                            (None, &terms[1])
                        }
                    } else {
                        (None, &terms[0])
                    };

                    if minus_one.is_some()
                        && let Some(denom_arg) = ExpTerm::get_direct_exp_arg(exp_term_denom)
                        && is_double_of(&denom_arg, &x)
                    {
                        return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.csch), x));
                    }
                }
            }
        }
        None
    }
);

rule!(
    CothFromExpRule,
    "coth_from_exp",
    80,
    Hyperbolic,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(numerator, denominator) = &expr.kind {
            let num_arg = if let AstKind::Sum(terms) = &numerator.kind {
                if terms.len() == 2 {
                    match_cosh_pattern(&terms[0], &terms[1])
                } else {
                    None
                }
            } else {
                None
            };

            let den_arg = if let AstKind::Sum(terms) = &denominator.kind {
                if terms.len() == 2 {
                    extract_negated_term(&terms[1])
                        .and_then(|negated| match_sinh_pattern_sub(&terms[0], &negated))
                        .or_else(|| {
                            extract_negated_term(&terms[0])
                                .and_then(|negated| match_sinh_pattern_sub(&terms[1], &negated))
                        })
                } else {
                    None
                }
            } else {
                None
            };

            if let (Some(n_arg), Some(d_arg)) = (num_arg, den_arg)
                && n_arg == d_arg
            {
                return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.coth), n_arg));
            }

            if let Some(x_num) = match_e2x_plus_1(numerator)
                && let Some(x_den) = match_e2x_minus_1_factored(denominator)
                && x_num == x_den
            {
                return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.coth), x_num));
            }

            if let Some(x_num) = match_e2x_plus_1(numerator)
                && let Some(x_den) = match_e2x_minus_1_direct(denominator)
                && x_num == x_den
            {
                return Some(Expr::func_symbol_arc(ks::get_symbol(ks::KS.coth), x_num));
            }
        }
        None
    }
);
