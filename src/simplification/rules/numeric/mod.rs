use crate::Expr;
use crate::ExprKind as AstKind;
use crate::core::known_symbols::{CBRT, SQRT};
use crate::simplification::rules::{ExprKind, Rule, RuleCategory, RuleContext};
use std::sync::Arc;
// ===== Identity Rules for Sum (Priority 100) =====

rule!(
    SumIdentityRule,
    "sum_identity",
    100,
    Numeric,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind {
            // Filter out zeros
            let non_zero: Vec<Arc<Expr>> = terms
                .iter()
                .filter(|t| !matches!(&t.kind, AstKind::Number(n) if *n == 0.0))
                .cloned()
                .collect();

            if non_zero.len() < terms.len() {
                // Some zeros were filtered
                if non_zero.is_empty() {
                    return Some(Expr::number(0.0));
                } else if non_zero.len() == 1 {
                    let arc = non_zero.into_iter().next().unwrap();
                    return Some(match Arc::try_unwrap(arc) {
                        Ok(e) => e,
                        Err(a) => (*a).clone(),
                    });
                } else {
                    return Some(Expr::sum_from_arcs(non_zero));
                }
            }
        }
        None
    }
);

// ===== Identity Rules for Product (Priority 100) =====

rule!(
    ProductZeroRule,
    "product_zero",
    100,
    Numeric,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            // If any factor is zero, the product is zero
            if factors
                .iter()
                .any(|f| matches!(&f.kind, AstKind::Number(n) if *n == 0.0))
            {
                return Some(Expr::number(0.0));
            }
        }
        None
    }
);

rule!(
    ProductIdentityRule,
    "product_identity",
    100,
    Numeric,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            // Filter out ones
            let non_one: Vec<Arc<Expr>> = factors
                .iter()
                .filter(|f| !matches!(&f.kind, AstKind::Number(n) if *n == 1.0))
                .cloned()
                .collect();

            if non_one.len() < factors.len() {
                // Some ones were filtered
                if non_one.is_empty() {
                    return Some(Expr::number(1.0));
                } else if non_one.len() == 1 {
                    let arc = non_one.into_iter().next().unwrap();
                    return Some(match Arc::try_unwrap(arc) {
                        Ok(e) => e,
                        Err(a) => (*a).clone(),
                    });
                } else {
                    return Some(Expr::product_from_arcs(non_one));
                }
            }
        }
        None
    }
);

// ===== Division Identity Rules (Priority 100) =====

rule!(
    DivOneRule,
    "div_one",
    100,
    Numeric,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(u, v) = &expr.kind
            && matches!(&v.kind, AstKind::Number(n) if *n == 1.0)
        {
            return Some((**u).clone());
        }
        None
    }
);

rule!(
    ZeroDivRule,
    "zero_div",
    100,
    Numeric,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(u, _v) = &expr.kind
            && matches!(&u.kind, AstKind::Number(n) if *n == 0.0)
        {
            return Some(Expr::number(0.0));
        }
        None
    }
);

// ===== Power Identity Rules (Priority 100) =====

rule!(
    PowZeroRule,
    "pow_zero",
    100,
    Numeric,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(_u, v) = &expr.kind
            && matches!(&v.kind, AstKind::Number(n) if *n == 0.0)
        {
            return Some(Expr::number(1.0));
        }
        None
    }
);

rule!(
    PowOneRule,
    "pow_one",
    100,
    Numeric,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(u, v) = &expr.kind
            && matches!(&v.kind, AstKind::Number(n) if *n == 1.0)
        {
            return Some((**u).clone());
        }
        None
    }
);

rule!(
    ZeroPowRule,
    "zero_pow",
    100,
    Numeric,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(u, v) = &expr.kind
            && matches!(&u.kind, AstKind::Number(n) if *n == 0.0)
        {
            // Only simplify 0^x when x is a positive number
            // 0^0 is conventionally 1, 0^negative is undefined
            if let AstKind::Number(exp) = &v.kind {
                if *exp > 0.0 {
                    return Some(Expr::number(0.0));
                }
            }
            // For symbolic exponents, don't simplify (could be 0 or negative)
        }
        None
    }
);

rule!(
    OnePowRule,
    "one_pow",
    100,
    Numeric,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(u, _v) = &expr.kind
            && matches!(&u.kind, AstKind::Number(n) if *n == 1.0)
        {
            return Some(Expr::number(1.0));
        }
        None
    }
);

// ===== Normalization Rule (Priority 95) =====

rule!(
    NormalizeSignDivRule,
    "normalize_sign_div",
    95,
    Numeric,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(num, den) = &expr.kind {
            // Check if denominator is negative number
            if let AstKind::Number(d) = &den.kind
                && *d < 0.0
            {
                // x / -y -> -x / y (negate numerator)
                return Some(Expr::div_expr(
                    Expr::product(vec![Expr::number(-1.0), num.as_ref().clone()]),
                    Expr::number(-*d),
                ));
            }

            // Check if denominator is a Product with -1 as first factor
            if let AstKind::Product(factors) = &den.kind
                && let Some(first) = factors.first()
                && matches!(&first.kind, AstKind::Number(n) if *n == -1.0)
            {
                // x / (-1 * y) -> -x / y
                let rest: Vec<Arc<Expr>> = factors.iter().skip(1).cloned().collect();
                let new_den = Expr::product_from_arcs(rest);
                return Some(Expr::div_expr(
                    Expr::product(vec![Expr::number(-1.0), num.as_ref().clone()]),
                    new_den,
                ));
            }
        }
        None
    }
);

// ===== Compaction Rules (Priority 90, 80) =====

rule!(
    ConstantFoldSumRule,
    "constant_fold_sum",
    90,
    Numeric,
    &[ExprKind::Sum],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Sum(terms) = &expr.kind {
            // Collect numeric and non-numeric terms
            let mut num_sum = 0.0;
            let mut non_numeric: Vec<Arc<Expr>> = Vec::new();
            let mut has_numeric = false;

            for term in terms {
                if let AstKind::Number(n) = &term.kind {
                    num_sum += n;
                    has_numeric = true;
                } else {
                    non_numeric.push(term.clone());
                }
            }

            // Only fold if we have numeric values that simplified away
            if has_numeric && non_numeric.len() < terms.len() {
                if non_numeric.is_empty() {
                    // All numeric
                    if !num_sum.is_nan() && !num_sum.is_infinite() {
                        return Some(Expr::number(num_sum));
                    }
                } else if num_sum != 0.0 {
                    // Mix - add the combined number to front
                    let mut result_terms = vec![Arc::new(Expr::number(num_sum))];
                    result_terms.extend(non_numeric);
                    return Some(Expr::sum_from_arcs(result_terms));
                } else if non_numeric.len() < terms.len() {
                    // Numbers summed to zero, return non-numeric only
                    if non_numeric.len() == 1 {
                        let arc = non_numeric.into_iter().next().unwrap();
                        return Some(match Arc::try_unwrap(arc) {
                            Ok(e) => e,
                            Err(a) => (*a).clone(),
                        });
                    } else {
                        return Some(Expr::sum_from_arcs(non_numeric));
                    }
                }
            }
        }
        None
    }
);

rule!(
    ConstantFoldProductRule,
    "constant_fold_product",
    90,
    Numeric,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind {
            // Collect numeric and non-numeric factors
            let mut num_product = 1.0;
            let mut non_numeric: Vec<Arc<Expr>> = Vec::new();
            let mut num_count = 0;

            for factor in factors {
                if let AstKind::Number(n) = &factor.kind {
                    num_product *= n;
                    num_count += 1;
                } else {
                    non_numeric.push(factor.clone());
                }
            }

            // Only fold if we have multiple numbers to combine
            if num_count >= 2 {
                if non_numeric.is_empty() {
                    // All numeric
                    if !num_product.is_nan() && !num_product.is_infinite() {
                        return Some(Expr::number(num_product));
                    }
                } else if num_product == 0.0 {
                    return Some(Expr::number(0.0));
                } else if num_product == 1.0 {
                    // Combined to 1, return non-numeric only
                    if non_numeric.len() == 1 {
                        let arc = non_numeric.into_iter().next().unwrap();
                        return Some(match Arc::try_unwrap(arc) {
                            Ok(e) => e,
                            Err(a) => (*a).clone(),
                        });
                    } else {
                        return Some(Expr::product_from_arcs(non_numeric));
                    }
                } else {
                    // Mix - add the combined number
                    let mut result_factors = vec![Arc::new(Expr::number(num_product))];
                    result_factors.extend(non_numeric);
                    return Some(Expr::product_from_arcs(result_factors));
                }
            }
        }
        None
    }
);

rule!(
    ConstantFoldDivRule,
    "constant_fold_div",
    90,
    Numeric,
    &[ExprKind::Div],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(u, v) = &expr.kind
            && let (AstKind::Number(a), AstKind::Number(b)) = (&u.kind, &v.kind)
            && *b != 0.0
        {
            let result = a / b;
            if (result - result.round()).abs() < 1e-10 {
                return Some(Expr::number(result.round()));
            }
        }
        None
    }
);

rule!(
    ConstantFoldPowRule,
    "constant_fold_pow",
    90,
    Numeric,
    &[ExprKind::Pow],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Pow(u, v) = &expr.kind
            && let (AstKind::Number(a), AstKind::Number(b)) = (&u.kind, &v.kind)
        {
            let result = a.powf(*b);
            if !result.is_nan() && !result.is_infinite() {
                return Some(Expr::number(result));
            }
        }
        None
    }
);

rule_with_helpers!(FractionSimplifyRule, "fraction_simplify", 80, Numeric, &[ExprKind::Div],
    helpers: {
        fn gcd(mut a: i64, mut b: i64) -> i64 {
            while b != 0 {
                let temp = b;
                b = a % b;
                a = temp;
            }
            a
        }
    },
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Div(u, v) = &expr.kind
            && let (AstKind::Number(a), AstKind::Number(b)) = (&u.kind, &v.kind)
            && *b != 0.0
        {
            let is_int_a = a.fract() == 0.0;
            let is_int_b = b.fract() == 0.0;

            if is_int_a && is_int_b {
                if a % b == 0.0 {
                    return Some(Expr::number(a / b));
                } else {
                    // Check bounds before casting to i64 to prevent overflow
                    const MAX_SAFE: f64 = i64::MAX as f64;
                    const MIN_SAFE: f64 = i64::MIN as f64;
                    if a.abs() > MAX_SAFE || b.abs() > MAX_SAFE
                        || *a < MIN_SAFE || *b < MIN_SAFE {
                        return None;  // Too large for integer GCD
                    }

                    let a_int = *a as i64;
                    let b_int = *b as i64;
                    let common = gcd(a_int.abs(), b_int.abs());

                    if common > 1 {
                        return Some(Expr::div_expr(
                            Expr::number((a_int / common) as f64),
                            Expr::number((b_int / common) as f64),
                        ));
                    }
                }
            }
        }
        None
    }
);

rule!(
    PerfectSquareRule,
    "perfect_square",
    95,
    Numeric,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == *SQRT
            && args.len() == 1
            && let AstKind::Number(n) = &args[0].kind
        {
            let s = n.sqrt();
            if (s - s.round()).abs() < 1e-10 {
                return Some(Expr::number(s.round()));
            }
        }
        None
    }
);

rule!(
    PerfectCubeRule,
    "perfect_cube",
    95,
    Numeric,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind
            && name.id() == *CBRT
            && args.len() == 1
            && let AstKind::Number(n) = &args[0].kind
        {
            let c = n.cbrt();
            if (c - c.round()).abs() < 1e-10 {
                return Some(Expr::number(c.round()));
            }
        }
        None
    }
);

rule!(
    ProductHalfRule,
    "product_half",
    80,
    Numeric,
    &[ExprKind::Product],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::Product(factors) = &expr.kind
            && factors.len() == 2
        {
            if let AstKind::Number(n) = &factors[0].kind
                && (*n - 0.5).abs() < 1e-10
            {
                return Some(Expr::div_expr((*factors[1]).clone(), Expr::number(2.0)));
            }
            if let AstKind::Number(n) = &factors[1].kind
                && (*n - 0.5).abs() < 1e-10
            {
                return Some(Expr::div_expr((*factors[0]).clone(), Expr::number(2.0)));
            }
        }
        None
    }
);

rule!(
    EvaluateNumericFunctionRule,
    "eval_numeric_func",
    95,
    Numeric,
    &[ExprKind::Function],
    |expr: &Expr, _context: &RuleContext| {
        if let AstKind::FunctionCall { name, args } = &expr.kind {
            // Check if all arguments are numbers
            let mut numeric_args = Vec::with_capacity(args.len());
            for arg in args {
                if let AstKind::Number(n) = &arg.kind {
                    numeric_args.push(*n);
                } else {
                    return None;
                }
            }

            // Look up function in registry and evaluate
            // Use ID-based lookup (via InternedSymbol) for speed
            if let Some(func_def) = crate::functions::registry::Registry::get_by_symbol(name)
                && let Some(result) = (func_def.eval)(&numeric_args)
            {
                // Check for NaN or Inf
                if result.is_nan() || result.is_infinite() {
                    return None;
                }

                // Only evaluate if result is an integer
                // This preserves symbolic forms like sqrt(2), ln(10), etc.
                if (result - result.round()).abs() < 1e-10 {
                    return Some(Expr::number(result.round()));
                }

                // Non-integer result: keep symbolic form
                return None;
            }
        }
        None
    }
);

/// Get all numeric rules in priority order
pub(crate) fn get_numeric_rules() -> Vec<Arc<dyn Rule + Send + Sync>> {
    vec![
        Arc::new(SumIdentityRule),
        Arc::new(ProductZeroRule),
        Arc::new(ProductIdentityRule),
        Arc::new(DivOneRule),
        Arc::new(ZeroDivRule),
        Arc::new(PowZeroRule),
        Arc::new(PowOneRule),
        Arc::new(ZeroPowRule),
        Arc::new(OnePowRule),
        Arc::new(EvaluateNumericFunctionRule), // Integrated evaluation
        Arc::new(NormalizeSignDivRule),
        Arc::new(ConstantFoldSumRule),
        Arc::new(ConstantFoldProductRule),
        Arc::new(ConstantFoldDivRule),
        Arc::new(ConstantFoldPowRule),
        Arc::new(FractionSimplifyRule),
        Arc::new(PerfectSquareRule),
        Arc::new(PerfectCubeRule),
        Arc::new(ProductHalfRule),
    ]
}
