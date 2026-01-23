use crate::core::expr::{Expr, ExprKind as AstKind};
use crate::core::known_symbols::{KS, get_symbol};
use crate::core::traits::EPSILON;
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use std::sync::Arc;

rule!(
    SqrtPowerRule,
    "sqrt_power",
    85,
    Root,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.sqrt
            && args.len() == 1
            && let AstKind::Pow(base, exp) = &args[0].kind
        {
            // Special case: sqrt(x^2) should always return abs(x)
            if let AstKind::Number(n) = &exp.kind {
                // Exact check for power 2.0 (square)
                #[allow(clippy::float_cmp)] // Comparing against exact constant 2.0
                let is_square = *n == 2.0;
                if is_square {
                    // sqrt(x^2) = |x|
                    return Some(Expr::func_symbol(get_symbol(KS.abs), base.as_ref().clone()));
                }
            }

            // Create new exponent: exp / 2
            let new_exp = Expr::div_expr(exp.as_ref().clone(), Expr::number(2.0));

            // Simplify the division immediately
            let simplified_exp = match &new_exp.kind {
                AstKind::Div(u, v) => {
                    if let (AstKind::Number(a), AstKind::Number(b)) = (&u.kind, &v.kind) {
                        if *b == 0.0 {
                            new_exp
                        } else {
                            let result = a / b;
                            if (result - result.round()).abs() < EPSILON {
                                Expr::number(result.round())
                            } else {
                                new_exp
                            }
                        }
                    } else {
                        new_exp
                    }
                }
                _ => new_exp,
            };

            // If exponent simplified to 1, return base directly
            #[allow(clippy::float_cmp)] // Comparing against exact constant 1.0
            let is_match = matches!(&simplified_exp.kind, AstKind::Number(n) if *n == 1.0);
            if is_match {
                return Some(base.as_ref().clone());
            }

            let result = Expr::pow_static(base.as_ref().clone(), simplified_exp);

            return Some(result);
        }
        None
    }
);

rule!(
    CbrtPowerRule,
    "cbrt_power",
    85,
    Root,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.cbrt
            && args.len() == 1
            && let AstKind::Pow(base, exp) = &args[0].kind
        {
            // Create new exponent: exp / 3
            let new_exp = Expr::div_expr(exp.as_ref().clone(), Expr::number(3.0));

            // Simplify the division immediately
            let simplified_exp = match &new_exp.kind {
                AstKind::Div(u, v) => {
                    if let (AstKind::Number(a), AstKind::Number(b)) = (&u.kind, &v.kind) {
                        if *b == 0.0 {
                            new_exp
                        } else {
                            let result = a / b;
                            if (result - result.round()).abs() < EPSILON {
                                Expr::number(result.round())
                            } else {
                                new_exp
                            }
                        }
                    } else {
                        new_exp
                    }
                }
                _ => new_exp,
            };

            // If exponent simplified to 1, return base directly
            #[allow(clippy::float_cmp)] // Comparing against exact constant 1.0
            let is_one = matches!(&simplified_exp.kind, AstKind::Number(n) if *n == 1.0);
            if is_one {
                return Some(base.as_ref().clone());
            }

            return Some(Expr::pow_static(base.as_ref().clone(), simplified_exp));
        }
        None
    }
);

rule!(SqrtProductRule, "sqrt_product", 56, Root, &[ExprKind::Product], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
    if let AstKind::Product(factors) = &expr.kind {
        // Check for sqrt(a) * sqrt(b) among factors
        for (i, f1) in factors.iter().enumerate() {
            for (j, f2) in factors.iter().enumerate() {
                if i >= j {
                    continue;
                }

                if let (
                    AstKind::FunctionCall { name: n1, args: args1 },
                    AstKind::FunctionCall { name: n2, args: args2 },
                ) = (&f1.kind, &f2.kind)
                    && n1.id() == KS.sqrt && n2.id() == KS.sqrt && args1.len() == 1 && args2.len() == 1 {
                        // sqrt(a) * sqrt(b) = sqrt(a*b)
                        let combined = Expr::func_symbol(
                            get_symbol(KS.sqrt),
                            Expr::product(vec![(*args1[0]).clone(), (*args2[0]).clone()]),
                        );

                        let mut new_factors: Vec<Expr> = factors.iter()
                            .enumerate()
                            .filter(|(k, _)| *k != i && *k != j)
                            .map(|(_, f)| (**f).clone())
                            .collect();
                        new_factors.push(combined);

                        if new_factors.len() == 1 {
                            // SAFETY: We just checked that new_factors.len() == 1
                            return new_factors.into_iter().next();
                        }
                        return Some(Expr::product(new_factors));
                    }
            }
        }
    }
    None
});

rule!(SqrtDivRule, "sqrt_div", 56, Root, &[ExprKind::Div], alters_domain: true, |expr: &Expr, _context: &RuleContext| {
    if let AstKind::Div(u, v) = &expr.kind {
        // Check for sqrt(a) / sqrt(b)
        if let (
            AstKind::FunctionCall {
                name: u_name,
                args: u_args,
            },
            AstKind::FunctionCall {
                name: v_name,
                args: v_args,
            },
        ) = (&u.kind, &v.kind)
            && u_name.id() == KS.sqrt
            && v_name.id() == KS.sqrt
            && u_args.len() == 1
            && v_args.len() == 1
        {
            return Some(Expr::func_symbol(
                get_symbol(KS.sqrt),
                Expr::div_expr(
                    (*u_args[0]).clone(),
                    (*v_args[0]).clone(),
                ),
            ));
        }
    }
    None
});

rule!(
    NormalizeRootsRule,
    "normalize_roots",
    50,
    Root,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && args.len() == 1
        {
            if name.id() == KS.sqrt {
                return Some(Expr::pow_static(
                    (*args[0]).clone(),
                    Expr::div_expr(Expr::number(1.0), Expr::number(2.0)),
                ));
            } else if name.id() == KS.cbrt {
                return Some(Expr::pow_static(
                    (*args[0]).clone(),
                    Expr::div_expr(Expr::number(1.0), Expr::number(3.0)),
                ));
            }
        }
        None
    }
);

rule!(
    SqrtExtractSquareRule,
    "sqrt_extract_square",
    84,
    Root,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        // sqrt(a * x^2) → |x| * sqrt(a)
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == KS.sqrt
            && args.len() == 1
            && let AstKind::Product(factors) = &args[0].kind
        {
            // Look for a square factor x^2
            for (i, factor) in factors.iter().enumerate() {
                if let AstKind::Pow(base, exp) = &factor.kind
                    && let AstKind::Number(n) = &exp.kind
                {
                    // Exact check for power 2.0 (square)
                    #[allow(clippy::float_cmp)] // Comparing against exact constant 2.0
                    let is_square = *n == 2.0;
                    if is_square {
                        // Found x^2, extract |x|
                        let abs_base = Expr::func_symbol(get_symbol(KS.abs), (**base).clone());

                        // Build remaining product for sqrt
                        let remaining: Vec<Arc<Expr>> = factors
                            .iter()
                            .enumerate()
                            .filter(|(j, _)| *j != i)
                            .map(|(_, f)| Arc::clone(f))
                            .collect();

                        let inner = Expr::product_from_arcs(remaining);
                        let sqrt_remaining = if matches!(inner.kind, AstKind::Number(n) if (n - 1.0).abs() < EPSILON)
                        {
                            Expr::number(1.0)
                        } else {
                            Expr::func_symbol(get_symbol(KS.sqrt), inner)
                        };

                        return Some(Expr::product(vec![abs_base, sqrt_remaining]));
                    }
                }
            }
        }
        None
    }
);

/// Get all root simplification rules in priority order
pub fn get_root_rules() -> Vec<Arc<dyn Rule + Send + Sync>> {
    vec![
        Arc::new(SqrtPowerRule),
        Arc::new(SqrtExtractSquareRule),
        Arc::new(CbrtPowerRule),
        Arc::new(SqrtProductRule),
        Arc::new(SqrtDivRule),
        Arc::new(NormalizeRootsRule),
    ]
}
