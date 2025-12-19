use crate::ast::{Expr, ExprKind as AstKind};
use crate::simplification::helpers;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};

rule!(
    PythagoreanIdentityRule,
    "pythagorean_identity",
    80,
    Trigonometric,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2 {
                let u = &terms[0];
                let v = &terms[1];

                // sin^2(x) + cos^2(x) = 1
                if let (AstKind::Pow(sin_base, sin_exp), AstKind::Pow(cos_base, cos_exp)) =
                    (&u.kind, &v.kind)
                    && matches!(&sin_exp.kind, AstKind::Number(n) if *n == 2.0)
                        && matches!(&cos_exp.kind, AstKind::Number(n) if *n == 2.0)
                        && let (
                            AstKind::FunctionCall {
                                name: sin_name,
                                args: sin_args,
                            },
                            AstKind::FunctionCall {
                                name: cos_name,
                                args: cos_args,
                            },
                        ) = (&sin_base.kind, &cos_base.kind)
                            && sin_name == "sin"
                                && cos_name == "cos"
                                && sin_args.len() == 1
                                && cos_args.len() == 1
                                && sin_args[0] == cos_args[0]
                            {
                                return Some(Expr::number(1.0));
                            }

                // cos^2(x) + sin^2(x) = 1
                if let (AstKind::Pow(cos_base, cos_exp), AstKind::Pow(sin_base, sin_exp)) =
                    (&u.kind, &v.kind)
                    && matches!(&cos_exp.kind, AstKind::Number(n) if *n == 2.0)
                        && matches!(&sin_exp.kind, AstKind::Number(n) if *n == 2.0)
                        && let (
                            AstKind::FunctionCall {
                                name: cos_name,
                                args: cos_args,
                            },
                            AstKind::FunctionCall {
                                name: sin_name,
                                args: sin_args,
                            },
                        ) = (&cos_base.kind, &sin_base.kind)
                            && cos_name == "cos"
                                && sin_name == "sin"
                                && cos_args.len() == 1
                                && sin_args.len() == 1
                                && cos_args[0] == sin_args[0]
                            {
                                return Some(Expr::number(1.0));
                            }
            }
        None
    }
);

rule!(
    PythagoreanComplementsRule,
    "pythagorean_complements",
    70,
    Trigonometric,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        // 1 - cos^2(x) = sin^2(x)
        // 1 - sin^2(x) = cos^2(x)
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2 {
                let lhs = &terms[0];
                let rhs = &terms[1];

                // Helper to extract negated term from Product([-1, x])
                fn extract_negated(term: &Expr) -> Option<Expr> {
                    if let AstKind::Product(factors) = &term.kind
                        && factors.len() == 2
                            && let AstKind::Number(n) = &factors[0].kind
                                && (*n + 1.0).abs() < 1e-10 {
                                    return Some((*factors[1]).clone());
                                }
                    None
                }

                // 1 + (-cos^2(x)) = sin^2(x)
                if matches!(&lhs.kind, AstKind::Number(n) if *n == 1.0)
                    && let Some(negated) = extract_negated(rhs) {
                        if let Some(("cos", arg)) = helpers::get_fn_pow_named(&negated, 2.0) {
                            return Some(Expr::pow(Expr::func("sin", arg), Expr::number(2.0)));
                        }
                        if let Some(("sin", arg)) = helpers::get_fn_pow_named(&negated, 2.0) {
                            return Some(Expr::pow(Expr::func("cos", arg), Expr::number(2.0)));
                        }
                    }

                // (-cos^2(x)) + 1 = sin^2(x)
                if matches!(&rhs.kind, AstKind::Number(n) if *n == 1.0)
                    && let Some(negated) = extract_negated(lhs) {
                        if let Some(("cos", arg)) = helpers::get_fn_pow_named(&negated, 2.0) {
                            return Some(Expr::pow(Expr::func("sin", arg), Expr::number(2.0)));
                        }
                        if let Some(("sin", arg)) = helpers::get_fn_pow_named(&negated, 2.0) {
                            return Some(Expr::pow(Expr::func("cos", arg), Expr::number(2.0)));
                        }
                    }
            }
        None
    }
);

rule!(
    PythagoreanTangentRule,
    "pythagorean_tangent",
    70,
    Trigonometric,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        // tan^2(x) + 1 = sec^2(x)
        // 1 + tan^2(x) = sec^2(x)
        // cot^2(x) + 1 = csc^2(x)
        // 1 + cot^2(x) = csc^2(x)
        if let AstKind::Sum(terms) = &expr.kind
            && terms.len() == 2 {
                let lhs = &terms[0];
                let rhs = &terms[1];

                // tan^2(x) + 1 = sec^2(x)
                if let Some(("tan", arg)) = helpers::get_fn_pow_named(lhs, 2.0)
                    && matches!(&rhs.kind, AstKind::Number(n) if *n == 1.0) {
                        return Some(Expr::pow(Expr::func("sec", arg), Expr::number(2.0)));
                    }

                // 1 + tan^2(x) = sec^2(x)
                if matches!(&lhs.kind, AstKind::Number(n) if *n == 1.0)
                    && let Some(("tan", arg)) = helpers::get_fn_pow_named(rhs, 2.0) {
                        return Some(Expr::pow(Expr::func("sec", arg), Expr::number(2.0)));
                    }

                // cot^2(x) + 1 = csc^2(x)
                if let Some(("cot", arg)) = helpers::get_fn_pow_named(lhs, 2.0)
                    && matches!(&rhs.kind, AstKind::Number(n) if *n == 1.0) {
                        return Some(Expr::pow(Expr::func("csc", arg), Expr::number(2.0)));
                    }

                // 1 + cot^2(x) = csc^2(x)
                if matches!(&lhs.kind, AstKind::Number(n) if *n == 1.0)
                    && let Some(("cot", arg)) = helpers::get_fn_pow_named(rhs, 2.0) {
                        return Some(Expr::pow(Expr::func("csc", arg), Expr::number(2.0)));
                    }
            }
        None
    }
);
