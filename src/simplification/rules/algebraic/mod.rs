use crate::Expr;
use crate::simplification::rules::{Rule, RuleCategory, RuleContext};
use std::rc::Rc;

/// Algebraic simplification rules
pub mod rules {
    use super::*;

    /// Rule for exp(ln(x)) -> x
    pub struct ExpLnRule;

    impl Rule for ExpLnRule {
        fn name(&self) -> &'static str {
            "exp_ln"
        }

        fn priority(&self) -> i32 {
            80
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn alters_domain(&self) -> bool {
            true
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::FunctionCall { name, args } = expr {
                if name == "exp" && args.len() == 1 {
                    if let Expr::FunctionCall {
                        name: inner_name,
                        args: inner_args,
                    } = &args[0]
                        && inner_name == "ln"
                        && inner_args.len() == 1
                    {
                        return Some(inner_args[0].clone());
                    }
                }
            }
            None
        }
    }
    /// Rule for ln(exp(x)) -> x
    pub struct LnExpRule;

    impl Rule for LnExpRule {
        fn name(&self) -> &'static str {
            "ln_exp"
        }

        fn priority(&self) -> i32 {
            80
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::FunctionCall { name, args } = expr {
                if name == "ln" && args.len() == 1 {
                    if let Expr::FunctionCall {
                        name: inner_name,
                        args: inner_args,
                    } = &args[0]
                        && inner_name == "exp"
                        && inner_args.len() == 1
                    {
                        return Some(inner_args[0].clone());
                    }
                }
            }
            None
        }
    }

    /// Rule for exp(a * ln(b)) -> b^a
    pub struct ExpMulLnRule;

    impl Rule for ExpMulLnRule {
        fn name(&self) -> &'static str {
            "exp_mul_ln"
        }

        fn priority(&self) -> i32 {
            80
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::FunctionCall { name, args } = expr {
                if name == "exp" && args.len() == 1 {
                    if let Expr::Mul(a, b) = &args[0] {
                        // Check if b is ln(x)
                        if let Expr::FunctionCall {
                            name: inner_name,
                            args: inner_args,
                        } = &**b
                            && inner_name == "ln"
                            && inner_args.len() == 1
                        {
                            return Some(Expr::Pow(Box::new(inner_args[0].clone()), a.clone()));
                        }
                        // Check if a is ln(x) (commutative)
                        if let Expr::FunctionCall {
                            name: inner_name,
                            args: inner_args,
                        } = &**a
                            && inner_name == "ln"
                            && inner_args.len() == 1
                        {
                            return Some(Expr::Pow(Box::new(inner_args[0].clone()), b.clone()));
                        }
                    }
                }
            }
            None
        }
    }

    /// Rule for e^(ln(x)) -> x (handles Symbol("e") form)
    pub struct EPowLnRule;

    impl Rule for EPowLnRule {
        fn name(&self) -> &'static str {
            "e_pow_ln"
        }

        fn priority(&self) -> i32 {
            85 // Identity/cancellation phase
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn alters_domain(&self) -> bool {
            true
        }

        fn apply(&self, expr: &Expr, context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(base, exp) = expr {
                // Check if base is Symbol("e") AND "e" is not a user variable
                if let Expr::Symbol(ref s) = **base {
                    if s == "e" && !context.variables.contains("e") {
                        // Check if exponent is ln(x)
                        if let Expr::FunctionCall { name, args } = &**exp {
                            if name == "ln" && args.len() == 1 {
                                return Some(args[0].clone());
                            }
                        }
                    }
                }
            }
            None
        }
    }
    /// Rule for e^(a*ln(b)) -> b^a (handles Symbol("e") form)
    pub struct EPowMulLnRule;

    impl Rule for EPowMulLnRule {
        fn name(&self) -> &'static str {
            "e_pow_mul_ln"
        }

        fn priority(&self) -> i32 {
            85 // Identity/cancellation phase
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(base, exp) = expr {
                // Check if base is Symbol("e") AND "e" is not a user variable
                if let Expr::Symbol(ref s) = **base {
                    if s == "e" && !context.variables.contains("e") {
                        // Check if exponent is a*ln(b) or ln(b)*a
                        if let Expr::Mul(a, b) = &**exp {
                            // Check if b is ln(x)
                            if let Expr::FunctionCall {
                                name: inner_name,
                                args: inner_args,
                            } = &**b
                            {
                                if inner_name == "ln" && inner_args.len() == 1 {
                                    return Some(Expr::Pow(
                                        Box::new(inner_args[0].clone()),
                                        a.clone(),
                                    ));
                                }
                            }
                            // Check if a is ln(x) (commutative)
                            if let Expr::FunctionCall {
                                name: inner_name,
                                args: inner_args,
                            } = &**a
                            {
                                if inner_name == "ln" && inner_args.len() == 1 {
                                    return Some(Expr::Pow(
                                        Box::new(inner_args[0].clone()),
                                        b.clone(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            None
        }
    }

    /// Rule for flattening nested divisions: (a/b)/(c/d) -> (a*d)/(b*c)
    pub struct DivDivRule;

    impl Rule for DivDivRule {
        fn name(&self) -> &'static str {
            "div_div_flatten"
        }

        fn priority(&self) -> i32 {
            95 // Expansion phase - flatten to single level early
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Div(num, den) = expr {
                // Case 1: (a/b)/(c/d) -> (a*d)/(b*c)
                if let (Expr::Div(a, b), Expr::Div(c, d)) = (&**num, &**den) {
                    return Some(Expr::Div(
                        Box::new(Expr::Mul(a.clone(), d.clone())),
                        Box::new(Expr::Mul(b.clone(), c.clone())),
                    ));
                }
                // Case 2: x/(c/d) -> (x*d)/c
                if let Expr::Div(c, d) = &**den {
                    return Some(Expr::Div(
                        Box::new(Expr::Mul(num.clone(), d.clone())),
                        c.clone(),
                    ));
                }
                // Case 3: (a/b)/y -> a/(b*y)
                if let Expr::Div(a, b) = &**num {
                    return Some(Expr::Div(
                        a.clone(),
                        Box::new(Expr::Mul(b.clone(), den.clone())),
                    ));
                }
            }
            None
        }
    }

    /// Rule for x / x = 1 (when x != 0)
    pub struct DivSelfRule;

    impl Rule for DivSelfRule {
        fn name(&self) -> &'static str {
            "div_self"
        }

        fn priority(&self) -> i32 {
            90
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn alters_domain(&self) -> bool {
            true
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Div(u, v) = expr {
                if let (Expr::Number(n1), Expr::Number(n2)) = (&**u, &**v) {
                    if *n1 == 2.0 && *n2 == 2.0 {
                        println!("DivSelfRule seeing 2/2. Equal? {}", u == v);
                    }
                }
                if u == v {
                    return Some(Expr::Number(1.0));
                }
            }
            None
        }
    }

    /// Rule for (x^a)^b -> x^(a*b)
    pub struct PowerPowerRule;

    impl Rule for PowerPowerRule {
        fn name(&self) -> &'static str {
            "power_power"
        }

        fn priority(&self) -> i32 {
            85
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(u, v) = expr {
                if let Expr::Pow(base, exp_inner) = &**u {
                    // Create new exponent: exp_inner * v
                    let new_exp = Expr::Mul(exp_inner.clone(), v.clone());

                    // Simplify the exponent arithmetic immediately
                    // This handles cases like 2 * (1/2) → 1
                    let simplified_exp = crate::simplification::simplify(new_exp);

                    return Some(Expr::Pow(base.clone(), Box::new(simplified_exp)));
                }
            }
            None
        }
    }

    /// Rule for x^0 = 1 (when x != 0)
    pub struct PowerZeroRule;

    impl Rule for PowerZeroRule {
        fn name(&self) -> &'static str {
            "power_zero"
        }

        fn priority(&self) -> i32 {
            95
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn alters_domain(&self) -> bool {
            true
        }

        fn dependencies(&self) -> Vec<&'static str> {
            vec![] // No dependencies
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(_u, _v) = expr {
                if matches!(**_v, Expr::Number(n) if n == 0.0) {
                    return Some(Expr::Number(1.0));
                }
            }
            None
        }
    }

    /// Rule for x^1 = x
    pub struct PowerOneRule;

    impl Rule for PowerOneRule {
        fn name(&self) -> &'static str {
            "power_one"
        }

        fn priority(&self) -> i32 {
            95
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(u, v) = expr {
                if matches!(**v, Expr::Number(n) if n == 1.0) {
                    return Some((**u).clone());
                }
            }
            None
        }
    }

    /// Rule for x^a * x^b -> x^(a+b)
    pub struct PowerMulRule;

    impl Rule for PowerMulRule {
        fn name(&self) -> &'static str {
            "power_mul"
        }

        fn priority(&self) -> i32 {
            85
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Mul(u, v) = expr {
                // Check if both terms are powers with the same base
                if let (Expr::Pow(base_u, exp_u), Expr::Pow(base_v, exp_v)) = (&**u, &**v) {
                    if base_u == base_v {
                        return Some(Expr::Pow(
                            base_u.clone(),
                            Box::new(Expr::Add(exp_u.clone(), exp_v.clone())),
                        ));
                    }
                }
                // Check if one is a power and the other is the same base
                if let Expr::Pow(base_u, exp_u) = &**u {
                    if base_u == &*v {
                        return Some(Expr::Pow(
                            base_u.clone(),
                            Box::new(Expr::Add(exp_u.clone(), Box::new(Expr::Number(1.0)))),
                        ));
                    }
                }
                if let Expr::Pow(base_v, exp_v) = &**v {
                    if base_v == &*u {
                        return Some(Expr::Pow(
                            base_v.clone(),
                            Box::new(Expr::Add(Box::new(Expr::Number(1.0)), exp_v.clone())),
                        ));
                    }
                }
            }
            None
        }
    }

    /// Rule for x^a / x^b -> x^(a-b)
    pub struct PowerDivRule;

    impl Rule for PowerDivRule {
        fn name(&self) -> &'static str {
            "power_div"
        }

        fn priority(&self) -> i32 {
            85
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn alters_domain(&self) -> bool {
            true
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Div(u, v) = expr {
                // Check if both numerator and denominator are powers with the same base
                if let (Expr::Pow(base_u, exp_u), Expr::Pow(base_v, exp_v)) = (&**u, &**v) {
                    if base_u == base_v {
                        return Some(Expr::Pow(
                            base_u.clone(),
                            Box::new(Expr::Sub(exp_u.clone(), exp_v.clone())),
                        ));
                    }
                }
                // Check if numerator is a power and denominator is the same base
                if let Expr::Pow(base_u, exp_u) = &**u {
                    if base_u == &*v {
                        return Some(Expr::Pow(
                            base_u.clone(),
                            Box::new(Expr::Sub(exp_u.clone(), Box::new(Expr::Number(1.0)))),
                        ));
                    }
                }
                // Check if denominator is a power and numerator is the same base
                if let Expr::Pow(base_v, exp_v) = &**v {
                    if base_v == &*u {
                        return Some(Expr::Pow(
                            base_v.clone(),
                            Box::new(Expr::Sub(Box::new(Expr::Number(1.0)), exp_v.clone())),
                        ));
                    }
                }
            }
            None
        }
    }

    /// Rule for expanding powers that enable cancellation: (a*b)^n / a -> a^n * b^n / a
    pub struct ExpandPowerForCancellationRule;

    impl Rule for ExpandPowerForCancellationRule {
        fn name(&self) -> &'static str {
            "expand_power_for_cancellation"
        }

        fn priority(&self) -> i32 {
            95 // Higher priority to expand before cancellation and before prettification
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Div(num, den) = expr {
                // Helper to check if a factor is present in an expression
                let contains_factor = |expr: &Expr, factor: &Expr| -> bool {
                    match expr {
                        Expr::Mul(_, _) => {
                            let factors = crate::simplification::helpers::flatten_mul(expr);
                            factors.contains(factor)
                        }
                        _ => expr == factor,
                    }
                };

                // Helper to check if expansion is useful
                let check_and_expand = |target: &Expr, other: &Expr| -> Option<Expr> {
                    if let Expr::Pow(base, exp) = target {
                        if let Expr::Mul(_, _) = &**base {
                            let base_factors = crate::simplification::helpers::flatten_mul(&**base);
                            // Check if any base factor is present in 'other'
                            let mut useful = false;
                            for factor in &base_factors {
                                if contains_factor(other, factor) {
                                    useful = true;
                                    break;
                                }
                            }

                            if useful {
                                let mut pow_factors: Vec<Expr> = Vec::new();
                                for factor in base_factors.into_iter() {
                                    pow_factors.push(Expr::Pow(Box::new(factor), exp.clone()));
                                }
                                return Some(crate::simplification::helpers::rebuild_mul(
                                    pow_factors,
                                ));
                            }
                        }
                    }
                    None
                };

                // Check numerator for expandable powers
                match &**num {
                    Expr::Mul(_, _) => {
                        let num_factors = crate::simplification::helpers::flatten_mul(&**num);
                        let mut new_num_factors = Vec::new();
                        let mut changed = false;
                        for factor in num_factors {
                            if let Some(expanded) = check_and_expand(&factor, den) {
                                new_num_factors.push(expanded);
                                changed = true;
                            } else {
                                new_num_factors.push(factor);
                            }
                        }
                        if changed {
                            return Some(Expr::Div(
                                Box::new(crate::simplification::helpers::rebuild_mul(
                                    new_num_factors,
                                )),
                                den.clone(),
                            ));
                        }
                    }
                    _ => {
                        if let Some(expanded) = check_and_expand(num, den) {
                            return Some(Expr::Div(Box::new(expanded), den.clone()));
                        }
                    }
                }

                // Check denominator for expandable powers
                match &**den {
                    Expr::Mul(_, _) => {
                        let den_factors = crate::simplification::helpers::flatten_mul(&**den);
                        let mut new_den_factors = Vec::new();
                        let mut changed = false;
                        for factor in den_factors {
                            if let Some(expanded) = check_and_expand(&factor, num) {
                                new_den_factors.push(expanded);
                                changed = true;
                            } else {
                                new_den_factors.push(factor);
                            }
                        }
                        if changed {
                            return Some(Expr::Div(
                                num.clone(),
                                Box::new(crate::simplification::helpers::rebuild_mul(
                                    new_den_factors,
                                )),
                            ));
                        }
                    }
                    _ => {
                        if let Some(expanded) = check_and_expand(den, num) {
                            return Some(Expr::Div(num.clone(), Box::new(expanded)));
                        }
                    }
                }
            }
            None
        }
    }

    /// Rule for expanding powers: (a*b)^n -> a^n * b^n
    /// Only expands if it simplifies (e.g. (2*x)^2 -> 4*x^2)
    pub struct PowerExpansionRule;

    impl Rule for PowerExpansionRule {
        fn name(&self) -> &'static str {
            "power_expansion"
        }

        fn priority(&self) -> i32 {
            40
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(base, exp) = expr {
                // Only consider numeric exponents for expansion generally
                // Unless we implement a specific rule for cancellation-driven expansion
                if !matches!(**exp, Expr::Number(_)) {
                    return None;
                }

                let should_expand = {
                    let exp_val = if let Expr::Number(e) = **exp {
                        Some(e)
                    } else if let Expr::Div(n, d) = &**exp {
                        if let (Expr::Number(n_val), Expr::Number(d_val)) = (&**n, &**d) {
                            if *d_val != 0.0 {
                                Some(n_val / d_val)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(e) = exp_val {
                        // Only expand if beneficial (e.g. numbers, sqrt, nested power)
                        // We removed the unconditional expansion for small integers to support power collection
                        {
                            let mut factors = Vec::new();
                            match &**base {
                                Expr::Mul(_, _) => factors
                                    .extend(crate::simplification::helpers::flatten_mul(&**base)),
                                Expr::Div(n, d) => {
                                    match &**n {
                                        Expr::Mul(_, _) => factors.extend(
                                            crate::simplification::helpers::flatten_mul(&**n),
                                        ),
                                        _ => factors.push(*n.clone()),
                                    }
                                    match &**d {
                                        Expr::Mul(_, _) => factors.extend(
                                            crate::simplification::helpers::flatten_mul(&**d),
                                        ),
                                        _ => factors.push(*d.clone()),
                                    }
                                }
                                _ => factors.push((**base).clone()),
                            };

                            let mut beneficial = false;
                            for factor in factors {
                                match factor {
                                    Expr::Number(n) => {
                                        let p = n.powf(e);
                                        if (p - p.round()).abs() < 1e-10 {
                                            beneficial = true;
                                            break;
                                        }
                                    }
                                    Expr::FunctionCall { name, args: _args } if name == "sqrt" => {
                                        if e % 2.0 == 0.0 {
                                            beneficial = true;
                                            break;
                                        }
                                    }
                                    Expr::Pow(_, inner_exp) => {
                                        if let Expr::Number(e2) = &*inner_exp {
                                            let prod = e * e2;
                                            if prod.fract() == 0.0
                                                || (prod - prod.round()).abs() < 1e-10
                                            {
                                                beneficial = true;
                                                break;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            beneficial
                        }
                    } else {
                        false
                    }
                };

                if should_expand {
                    if let Expr::Mul(_, _) = &**base {
                        let base_factors = crate::simplification::helpers::flatten_mul(&**base);
                        let mut pow_factors: Vec<Expr> = Vec::new();
                        for factor in base_factors.into_iter() {
                            pow_factors.push(Expr::Pow(Box::new(factor), exp.clone()));
                        }
                        return Some(crate::simplification::helpers::rebuild_mul(pow_factors));
                    } else if let Expr::Div(num, den) = &**base {
                        return Some(Expr::Div(
                            Box::new(Expr::Pow(num.clone(), exp.clone())),
                            Box::new(Expr::Pow(den.clone(), exp.clone())),
                        ));
                    }
                }
            }
            None
        }
    }

    /// Rule for collecting powers in multiplication: x^a * x^b -> x^(a+b)
    pub struct PowerCollectionRule;

    impl Rule for PowerCollectionRule {
        fn name(&self) -> &'static str {
            "power_collection"
        }

        fn priority(&self) -> i32 {
            80
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Mul(_, _) = expr {
                let factors = crate::simplification::helpers::flatten_mul(expr);

                // Group by base
                use std::collections::HashMap;
                let mut base_to_exponents: HashMap<Expr, Vec<Expr>> = HashMap::new();

                for factor in factors {
                    if let Expr::Pow(base, exp) = factor {
                        base_to_exponents
                            .entry(*base)
                            .or_insert(Vec::new())
                            .push(*exp);
                    } else {
                        // Non-power factor, treat as base^1
                        base_to_exponents
                            .entry(factor)
                            .or_insert(Vec::new())
                            .push(Expr::Number(1.0));
                    }
                }

                // Combine exponents for each base
                let mut result_factors = Vec::new();
                for (base, exponents) in base_to_exponents {
                    if exponents.len() == 1 {
                        if exponents[0] == Expr::Number(1.0) {
                            result_factors.push(base);
                        } else {
                            result_factors
                                .push(Expr::Pow(Box::new(base), Box::new(exponents[0].clone())));
                        }
                    } else {
                        // Sum all exponents
                        let mut sum = exponents[0].clone();
                        for exp in &exponents[1..] {
                            sum = Expr::Add(Box::new(sum), Box::new(exp.clone()));
                        }
                        result_factors.push(Expr::Pow(Box::new(base), Box::new(sum)));
                    }
                }

                // Rebuild the expression
                if result_factors.len() == 1 {
                    Some(result_factors[0].clone())
                } else {
                    let mut result = result_factors[0].clone();
                    for factor in &result_factors[1..] {
                        result = Expr::Mul(Box::new(result), Box::new(factor.clone()));
                    }
                    Some(result)
                }
            } else {
                None
            }
        }
    }

    /// Rule for x^a / y^a -> (x/y)^a
    pub struct CommonExponentDivRule;

    impl Rule for CommonExponentDivRule {
        fn name(&self) -> &'static str {
            "common_exponent_div"
        }

        fn priority(&self) -> i32 {
            80
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Div(num, den) = expr {
                if let (Expr::Pow(base_num, exp_num), Expr::Pow(base_den, exp_den)) =
                    (&**num, &**den)
                {
                    if exp_num == exp_den {
                        return Some(Expr::Pow(
                            Box::new(Expr::Div(base_num.clone(), base_den.clone())),
                            exp_num.clone(),
                        ));
                    }
                }
            }
            None
        }
    }

    /// Rule for x^a * y^a -> (x*y)^a
    pub struct CommonExponentMulRule;

    impl Rule for CommonExponentMulRule {
        fn name(&self) -> &'static str {
            "common_exponent_mul"
        }

        fn priority(&self) -> i32 {
            80
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Mul(left, right) = expr {
                if let (Expr::Pow(base_left, exp_left), Expr::Pow(base_right, exp_right)) =
                    (&**left, &**right)
                {
                    if exp_left == exp_right {
                        return Some(Expr::Pow(
                            Box::new(Expr::Mul(base_left.clone(), base_right.clone())),
                            exp_left.clone(),
                        ));
                    }
                }
            }
            None
        }
    }

    /// Rule for x^(1/2) -> sqrt(x)
    pub struct PowerToSqrtRule;

    impl Rule for PowerToSqrtRule {
        fn name(&self) -> &'static str {
            "power_to_sqrt"
        }

        fn priority(&self) -> i32 {
            90
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(base, exp) = expr {
                if let Expr::Div(num, den) = &**exp {
                    if matches!(**num, Expr::Number(n) if n == 1.0)
                        && matches!(**den, Expr::Number(n) if n == 2.0)
                    {
                        return Some(Expr::FunctionCall {
                            name: "sqrt".to_string(),
                            args: vec![(**base).clone()],
                        });
                    }
                } else if matches!(**exp, Expr::Number(n) if n == 0.5) {
                    return Some(Expr::FunctionCall {
                        name: "sqrt".to_string(),
                        args: vec![(**base).clone()],
                    });
                }
            }
            None
        }
    }

    /// Rule for x^(1/3) -> cbrt(x)
    pub struct PowerToCbrtRule;

    impl Rule for PowerToCbrtRule {
        fn name(&self) -> &'static str {
            "power_to_cbrt"
        }

        fn priority(&self) -> i32 {
            90
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(base, exp) = expr {
                if let Expr::Div(num, den) = &**exp {
                    if matches!(**num, Expr::Number(n) if n == 1.0)
                        && matches!(**den, Expr::Number(n) if n == 3.0)
                    {
                        return Some(Expr::FunctionCall {
                            name: "cbrt".to_string(),
                            args: vec![(**base).clone()],
                        });
                    }
                }
            }
            None
        }
    }

    /// Rule for x^-n -> 1/x^n where n > 0
    pub struct NegativeExponentToFractionRule;

    impl Rule for NegativeExponentToFractionRule {
        fn name(&self) -> &'static str {
            "negative_exponent_to_fraction"
        }

        fn priority(&self) -> i32 {
            90
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Pow(base, exp) = expr {
                if let Expr::Number(n) = **exp {
                    if n < 0.0 {
                        let positive_exp = Expr::Number(-n);
                        let denominator = Expr::Pow(base.clone(), Box::new(positive_exp));
                        return Some(Expr::Div(
                            Box::new(Expr::Number(1.0)),
                            Box::new(denominator),
                        ));
                    }
                }
            }
            None
        }
    }
    /// Rule for cancelling common terms in fractions: (a*b)/(a*c) -> b/c
    /// Also handles powers: x^a / x^b -> x^(a-b)
    pub struct FractionCancellationRule;

    impl Rule for FractionCancellationRule {
        fn name(&self) -> &'static str {
            "fraction_cancellation"
        }

        fn priority(&self) -> i32 {
            90
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn alters_domain(&self) -> bool {
            true
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Div(u, v) = expr {
                let num_factors = crate::simplification::helpers::flatten_mul(u);
                let den_factors = crate::simplification::helpers::flatten_mul(v);

                // 1. Handle numeric coefficients
                let mut num_coeff = 1.0;
                let mut den_coeff = 1.0;
                let mut new_num_factors = Vec::new();
                let mut new_den_factors = Vec::new();

                for f in num_factors {
                    if let Expr::Number(n) = f {
                        num_coeff *= n;
                    } else {
                        new_num_factors.push(f);
                    }
                }

                for f in den_factors {
                    if let Expr::Number(n) = f {
                        den_coeff *= n;
                    } else {
                        new_den_factors.push(f);
                    }
                }

                // Simplify coefficients (e.g. 2/4 -> 1/2)
                let ratio = num_coeff / den_coeff;
                if ratio.abs() < 1e-10 {
                    return Some(Expr::Number(0.0));
                }

                // Check if ratio or 1/ratio is integer-ish
                if (ratio - ratio.round()).abs() < 1e-10 {
                    num_coeff = ratio.round();
                    den_coeff = 1.0;
                } else if (1.0 / ratio - (1.0 / ratio).round()).abs() < 1e-10 {
                    num_coeff = 1.0;
                    den_coeff = (1.0 / ratio).round();
                }
                // Else keep original coefficients (or maybe we should use the ratio if it simplifies?)
                // For now, if we can't simplify to integer num or den, we keep as is?
                // But wait, if we have 2/3, ratio is 0.666.
                // num_coeff becomes 2, den_coeff becomes 3 (if we didn't change them).
                // If we set num=ratio, den=1, we get 0.666/1.
                // We want to preserve integers if possible.
                // So only update if we found a simplification.
                // The logic above updates num_coeff/den_coeff ONLY if one becomes 1.0.
                // This handles 2/4 -> 1/2.
                // It handles 4/2 -> 2/1.
                // It does NOT handle 2/3 -> 2/3 (keeps 2 and 3). Correct.

                // Helper to get base and exponent
                fn get_base_exp(e: &Expr) -> (Expr, Expr) {
                    match e {
                        Expr::Pow(b, e) => (*b.clone(), *e.clone()),
                        Expr::FunctionCall { name, args } if args.len() == 1 => {
                            if name == "sqrt" {
                                (args[0].clone(), Expr::Number(0.5))
                            } else if name == "cbrt" {
                                (
                                    args[0].clone(),
                                    Expr::Div(
                                        Box::new(Expr::Number(1.0)),
                                        Box::new(Expr::Number(3.0)),
                                    ),
                                )
                            } else {
                                (e.clone(), Expr::Number(1.0))
                            }
                        }
                        _ => (e.clone(), Expr::Number(1.0)),
                    }
                }

                // 2. Symbolic cancellation
                // Iterate through numerator factors and look for matches in denominator
                let mut i = 0;
                while i < new_num_factors.len() {
                    let (base_i, exp_i) = get_base_exp(&new_num_factors[i]);
                    let mut matched = false;

                    for j in 0..new_den_factors.len() {
                        let (base_j, exp_j) = get_base_exp(&new_den_factors[j]);

                        if base_i == base_j {
                            // Found same base, subtract exponents: new_exp = exp_i - exp_j
                            let new_exp =
                                Expr::Sub(Box::new(exp_i.clone()), Box::new(exp_j.clone()));

                            // Simplify exponent
                            let simplified_exp =
                                if let (Expr::Number(n1), Expr::Number(n2)) = (&exp_i, &exp_j) {
                                    Expr::Number(n1 - n2)
                                } else {
                                    new_exp
                                };

                            if let Expr::Number(n) = simplified_exp {
                                if n == 0.0 {
                                    // Cancel completely
                                    new_num_factors.remove(i);
                                    new_den_factors.remove(j);
                                    matched = true;
                                    break;
                                } else if n > 0.0 {
                                    // Remains in numerator
                                    if n == 1.0 {
                                        new_num_factors[i] = base_i.clone();
                                    } else {
                                        new_num_factors[i] = Expr::Pow(
                                            Box::new(base_i.clone()),
                                            Box::new(Expr::Number(n)),
                                        );
                                    }
                                    new_den_factors.remove(j);
                                    matched = true;
                                    break;
                                } else {
                                    // Moves to denominator (n < 0)
                                    new_num_factors.remove(i);
                                    let pos_n = -n;
                                    if pos_n == 1.0 {
                                        new_den_factors[j] = base_i.clone();
                                    } else {
                                        new_den_factors[j] = Expr::Pow(
                                            Box::new(base_i.clone()),
                                            Box::new(Expr::Number(pos_n)),
                                        );
                                    }
                                    matched = true;
                                    break;
                                }
                            } else {
                                // Symbolic exponent subtraction
                                new_num_factors[i] =
                                    Expr::Pow(Box::new(base_i.clone()), Box::new(simplified_exp));
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
                    new_num_factors.insert(0, Expr::Number(num_coeff));
                }
                if den_coeff != 1.0 {
                    new_den_factors.insert(0, Expr::Number(den_coeff));
                }

                // Rebuild numerator
                let new_num = if new_num_factors.is_empty() {
                    Expr::Number(1.0)
                } else {
                    crate::simplification::helpers::rebuild_mul(new_num_factors)
                };

                // Rebuild denominator
                let new_den = if new_den_factors.is_empty() {
                    Expr::Number(1.0)
                } else {
                    crate::simplification::helpers::rebuild_mul(new_den_factors)
                };

                // If denominator is 1, return numerator
                if let Expr::Number(n) = new_den {
                    if n == 1.0 {
                        return Some(new_num);
                    }
                }

                let res = Expr::Div(Box::new(new_num), Box::new(new_den));
                if res != *expr {
                    return Some(res);
                }
            }
            None
        }
    }

    /// Rule for adding fractions: a + b/c -> (a*c + b)/c
    pub struct AddFractionRule;

    impl Rule for AddFractionRule {
        fn name(&self) -> &'static str {
            "add_fraction"
        }

        fn priority(&self) -> i32 {
            40
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Add(u, v) = expr {
                // Case 1: a/b + c/d
                if let (Expr::Div(n1, d1), Expr::Div(n2, d2)) = (&**u, &**v) {
                    // Check for common denominator
                    if d1 == d2 {
                        return Some(Expr::Div(
                            Box::new(Expr::Add(n1.clone(), n2.clone())),
                            d1.clone(),
                        ));
                    }
                    // (n1*d2 + n2*d1) / (d1*d2)
                    let new_num = Expr::Add(
                        Box::new(Expr::Mul(n1.clone(), d2.clone())),
                        Box::new(Expr::Mul(n2.clone(), d1.clone())),
                    );
                    let new_den = Expr::Mul(d1.clone(), d2.clone());
                    return Some(Expr::Div(Box::new(new_num), Box::new(new_den)));
                }

                // Case 2: a + b/c
                if let Expr::Div(n, d) = &**v {
                    // (u*d + n) / d
                    let new_num = Expr::Add(Box::new(Expr::Mul(u.clone(), d.clone())), n.clone());
                    return Some(Expr::Div(Box::new(new_num), d.clone()));
                }

                // Case 3: a/b + c
                if let Expr::Div(n, d) = &**u {
                    // (n + v*d) / d
                    let new_num = Expr::Add(n.clone(), Box::new(Expr::Mul(v.clone(), d.clone())));
                    return Some(Expr::Div(Box::new(new_num), d.clone()));
                }
            }
            None
        }
    }

    /// Get the polynomial degree of an expression (simplified for common cases)
    fn get_polynomial_degree(expr: &Expr) -> i32 {
        match expr {
            Expr::Pow(_, exp) => {
                if let Expr::Number(n) = &**exp {
                    if n.fract() == 0.0 && *n >= 0.0 {
                        return *n as i32;
                    }
                }
                0 // Non-integer or negative exponent
            }
            Expr::Mul(_, _) => {
                let factors = crate::simplification::helpers::flatten_mul(expr);
                let mut total_degree = 0;
                for factor in factors {
                    total_degree += get_polynomial_degree(&factor);
                }
                total_degree
            }
            Expr::Symbol(_) => 1,
            Expr::Number(_) => 0,
            _ => 0, // For other expressions, treat as constant
        }
    }

    /// Rule for canonicalizing expressions (sorting terms)
    pub struct CanonicalizeRule;

    impl Rule for CanonicalizeRule {
        fn name(&self) -> &'static str {
            "canonicalize"
        }

        fn priority(&self) -> i32 {
            40
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            match expr {
                Expr::Mul(_u, _v) => {
                    let mut factors = crate::simplification::helpers::flatten_mul(expr);
                    // Check if already sorted
                    let mut sorted = true;
                    for i in 0..factors.len() - 1 {
                        if crate::simplification::helpers::compare_expr(
                            &factors[i],
                            &factors[i + 1],
                        ) == std::cmp::Ordering::Greater
                        {
                            sorted = false;
                            break;
                        }
                    }

                    if !sorted {
                        factors.sort_by(|a, b| crate::simplification::helpers::compare_expr(a, b));

                        // Rebuild left-associative to match standard parsing
                        let res = crate::simplification::helpers::rebuild_mul(factors);
                        return Some(res);
                    }
                }
                Expr::Add(_u, _v) => {
                    let mut terms = crate::simplification::helpers::flatten_add(expr.clone());
                    // Check if already sorted by degree descending
                    let mut sorted = true;
                    for i in 0..terms.len() - 1 {
                        let deg_i = get_polynomial_degree(&terms[i]);
                        let deg_j = get_polynomial_degree(&terms[i + 1]);
                        if deg_i < deg_j
                            || (deg_i == deg_j
                                && crate::simplification::helpers::compare_expr(
                                    &terms[i],
                                    &terms[i + 1],
                                ) == std::cmp::Ordering::Greater)
                        {
                            sorted = false;
                            break;
                        }
                    }

                    if !sorted {
                        terms.sort_by(|a, b| {
                            let deg_a = get_polynomial_degree(a);
                            let deg_b = get_polynomial_degree(b);
                            match deg_b.cmp(&deg_a) {
                                // Reverse for descending
                                std::cmp::Ordering::Equal => {
                                    crate::simplification::helpers::compare_expr(a, b)
                                }
                                ord => ord,
                            }
                        });

                        // Rebuild left-associative
                        let res = crate::simplification::helpers::rebuild_add(terms);
                        return Some(res);
                    }
                }
                _ => {}
            }
            None
        }
    }

    /// Rule for a * (b / c) -> (a * b) / c
    pub struct MulDivCombinationRule;

    impl Rule for MulDivCombinationRule {
        fn name(&self) -> &'static str {
            "mul_div_combination"
        }

        fn priority(&self) -> i32 {
            85 // High priority to enable cancellation
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Mul(u, v) = expr {
                // Case 1: a * (b / c) -> (a * b) / c
                if let Expr::Div(num, den) = &**v {
                    return Some(Expr::Div(
                        Box::new(Expr::Mul(u.clone(), num.clone())),
                        den.clone(),
                    ));
                }
                // Case 2: (a / b) * c -> (a * c) / b
                if let Expr::Div(num, den) = &**u {
                    return Some(Expr::Div(
                        Box::new(Expr::Mul(num.clone(), v.clone())),
                        den.clone(),
                    ));
                }
            }
            None
        }
    }

    /// Rule for combining like terms in addition: 2x + 3x -> 5x
    pub struct CombineTermsRule;

    impl Rule for CombineTermsRule {
        fn name(&self) -> &'static str {
            "combine_terms"
        }

        fn priority(&self) -> i32 {
            45
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            // Handle both Add and Sub
            let terms = match expr {
                Expr::Add(_, _) => crate::simplification::helpers::flatten_add(expr.clone()),
                Expr::Sub(a, b) => {
                    // Convert x - y to x + (-1)*y and flatten
                    let as_add = Expr::Add(
                        a.clone(),
                        Box::new(Expr::Mul(Box::new(Expr::Number(-1.0)), b.clone())),
                    );
                    crate::simplification::helpers::flatten_add(as_add)
                }
                _ => return None,
            };

            if terms.len() < 2 {
                return None;
            }

            // Sort terms to bring like terms together
            let mut sorted_terms = terms;
            sorted_terms.sort_by(|a, b| crate::simplification::helpers::compare_expr(a, b));

            let mut result = Vec::new();
            let mut iter = sorted_terms.into_iter();

            let first = iter.next().unwrap();
            let (mut current_coeff, mut current_base) =
                crate::simplification::helpers::extract_coeff(&first);

            for term in iter {
                let (coeff, base) = crate::simplification::helpers::extract_coeff(&term);

                if base == current_base {
                    current_coeff += coeff;
                } else {
                    // Push current
                    if current_coeff == 0.0 {
                        // Drop
                    } else {
                        if current_coeff == 1.0 {
                            result.push(current_base);
                        } else {
                            // Check if base is 1.0 (number)
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
                    }
                    current_coeff = coeff;
                    current_base = base;
                }
            }

            // Push last
            if current_coeff != 0.0 {
                if current_coeff == 1.0 {
                    result.push(current_base);
                } else {
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
            }

            if result.is_empty() {
                return Some(Expr::Number(0.0));
            }

            let new_expr = crate::simplification::helpers::rebuild_add(result);
            if new_expr != *expr {
                return Some(new_expr);
            }
            None
        }
    }

    /// Rule for perfect squares: a^2 + 2ab + b^2 -> (a+b)^2
    pub struct PerfectSquareRule;

    impl Rule for PerfectSquareRule {
        fn name(&self) -> &'static str {
            "perfect_square"
        }

        fn priority(&self) -> i32 {
            50 // Higher priority to catch specific patterns before general factoring
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Add(_, _) = expr {
                let terms = crate::simplification::helpers::flatten_add(expr.clone());

                if terms.len() != 3 {
                    return None;
                } else {
                    // Try to match pattern: c1*a^2 + c2*a*b + c3*b^2
                    let mut square_terms: Vec<(f64, Expr)> = Vec::new(); // (coefficient, base)
                    let mut linear_terms: Vec<(f64, Expr, Expr)> = Vec::new(); // (coefficient, base1, base2)
                    let mut constants = Vec::new();

                    for term in &terms {
                        match term {
                            // Match c*x^2 or x^2
                            Expr::Pow(base, exp) if matches!(**exp, Expr::Number(n) if n == 2.0) => {
                                square_terms.push((1.0, *base.clone()));
                            }
                            // Match c*(x^2) where c is a number
                            Expr::Mul(coeff, rest) if matches!(**coeff, Expr::Number(_)) => {
                                if let Expr::Number(c) = **coeff {
                                    match &**rest {
                                        Expr::Pow(base, exp) if matches!(**exp, Expr::Number(n) if n == 2.0) =>
                                        {
                                            square_terms.push((c, *base.clone()));
                                        }
                                        Expr::Mul(a, b) => {
                                            // c*a*b
                                            linear_terms.push((c, *a.clone(), *b.clone()));
                                        }
                                        other => {
                                            // c*a means cross term 2*sqrt(c)*a*1, treat as c*a*1
                                            linear_terms.push((
                                                c,
                                                other.clone(),
                                                Expr::Number(1.0),
                                            ));
                                        }
                                    }
                                }
                            }
                            Expr::Number(n) => {
                                constants.push(*n);
                            }
                            // Handle implicit coefficient 1 for linear terms (e.g. x)
                            other => {
                                // Treat as 1 * other * 1
                                linear_terms.push((1.0, other.clone(), Expr::Number(1.0)));
                            }
                        }
                    }

                    // Case 1: Standard perfect square a^2 + 2*a*b + b^2
                    if square_terms.len() == 2 && linear_terms.len() == 1 {
                        let (c1, a) = &square_terms[0];
                        let (c2, b) = &square_terms[1];
                        let (cross_coeff, cross_a, cross_b) = &linear_terms[0];

                        // Check if c1 and c2 have integer square roots
                        let sqrt_c1 = c1.sqrt();
                        let sqrt_c2 = c2.sqrt();

                        if (sqrt_c1 - sqrt_c1.round()).abs() < 1e-10
                            && (sqrt_c2 - sqrt_c2.round()).abs() < 1e-10
                        {
                            // Check if cross_coeff = +/- 2 * sqrt(c1) * sqrt(c2)
                            let expected_cross_abs = (2.0 * sqrt_c1 * sqrt_c2).abs();
                            let cross_coeff_abs = cross_coeff.abs();

                            if (expected_cross_abs - cross_coeff_abs).abs() < 1e-10 {
                                // Check if the variables match
                                if (a == cross_a && b == cross_b) || (a == cross_b && b == cross_a)
                                {
                                    let sign = cross_coeff.signum();

                                    // Build (sqrt(c1)*a + sign * sqrt(c2)*b)
                                    let term_a = if (sqrt_c1 - 1.0).abs() < 1e-10 {
                                        a.clone()
                                    } else {
                                        Expr::Mul(
                                            Box::new(Expr::Number(sqrt_c1.round())),
                                            Box::new(a.clone()),
                                        )
                                    };

                                    let term_b = if (sqrt_c2 - 1.0).abs() < 1e-10 {
                                        b.clone()
                                    } else {
                                        Expr::Mul(
                                            Box::new(Expr::Number(sqrt_c2.round())),
                                            Box::new(b.clone()),
                                        )
                                    };

                                    let inner = if sign > 0.0 {
                                        Expr::Add(Box::new(term_a), Box::new(term_b))
                                    } else {
                                        // term_a - term_b
                                        Expr::Add(
                                            Box::new(term_a),
                                            Box::new(Expr::Mul(
                                                Box::new(Expr::Number(-1.0)),
                                                Box::new(term_b),
                                            )),
                                        )
                                    };

                                    return Some(Expr::Pow(
                                        Box::new(inner),
                                        Box::new(Expr::Number(2.0)),
                                    ));
                                }
                            }
                        }
                    }

                    // Case 2: One square + constant + linear: c1*a^2 + c2*a + c3
                    if square_terms.len() == 1 && linear_terms.len() == 1 && constants.len() == 1 {
                        let (c1, a) = &square_terms[0];
                        let (c2, cross_a, cross_b) = &linear_terms[0];
                        let c3 = constants[0];

                        // Check if c1 and c3 have integer square roots
                        let sqrt_c1 = c1.sqrt();
                        let sqrt_c3 = c3.sqrt();

                        if (sqrt_c1 - sqrt_c1.round()).abs() < 1e-10
                            && (sqrt_c3 - sqrt_c3.round()).abs() < 1e-10
                        {
                            // Check if c2 = +/- 2 * sqrt(c1) * sqrt(c3)
                            let expected_cross_abs = (2.0 * sqrt_c1 * sqrt_c3).abs();
                            let cross_coeff_abs = c2.abs();

                            if (expected_cross_abs - cross_coeff_abs).abs() < 1e-10 {
                                // Check if linear term matches
                                if (a == cross_a && matches!(cross_b, Expr::Number(n) if *n == 1.0))
                                    || (a == cross_b
                                        && matches!(cross_a, Expr::Number(n) if *n == 1.0))
                                {
                                    let sign = c2.signum();

                                    let term_a = if (sqrt_c1 - 1.0).abs() < 1e-10 {
                                        a.clone()
                                    } else {
                                        Expr::Mul(
                                            Box::new(Expr::Number(sqrt_c1.round())),
                                            Box::new(a.clone()),
                                        )
                                    };

                                    let term_b = Expr::Number(sqrt_c3.round());

                                    let inner = if sign > 0.0 {
                                        Expr::Add(Box::new(term_a), Box::new(term_b))
                                    } else {
                                        // term_a - term_b
                                        Expr::Add(
                                            Box::new(term_a),
                                            Box::new(Expr::Mul(
                                                Box::new(Expr::Number(-1.0)),
                                                Box::new(term_b),
                                            )),
                                        )
                                    };

                                    return Some(Expr::Pow(
                                        Box::new(inner),
                                        Box::new(Expr::Number(2.0)),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            None
        }
    }

    /// Rule for expanding difference of squares products: (a-b)(a+b) -> a^2 - b^2
    /// High priority to enable cancellations
    pub struct ExpandDifferenceOfSquaresProductRule;

    impl Rule for ExpandDifferenceOfSquaresProductRule {
        fn name(&self) -> &'static str {
            "expand_difference_of_squares_product"
        }

        fn priority(&self) -> i32 {
            85 // High priority to expand before cancellations can occur
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            // Detect (a-b)(a+b) pattern in products and expand to a^2 - b^2
            if let Expr::Mul(u, v) = expr {
                // Check for (a-b)(a+b) -> a^2 - b^2
                // Handle (a-b) as Add(a, -b) or Sub(a, b)
                // Handle (a+b) as Add(a, b)

                let get_terms = |e: &Expr| -> Option<(Expr, Expr)> {
                    match e {
                        Expr::Add(a, b) => Some((*a.clone(), *b.clone())),
                        Expr::Sub(a, b) => Some((
                            *a.clone(),
                            Expr::Mul(Box::new(Expr::Number(-1.0)), b.clone()),
                        )),
                        _ => None,
                    }
                };

                if let (Some((a1, b1)), Some((a2, b2))) = (get_terms(u), get_terms(v)) {
                    // We have two sums: (a1 + b1) * (a2 + b2)
                    // We want to check if they are (A - B) * (A + B)
                    // This means one term matches and the other is negated.

                    // Possible combinations:
                    // 1. a1 == a2, b1 == -b2 (or b2 == -b1) -> a1^2 - b1^2
                    // 2. a1 == b2, b1 == -a2 -> a1^2 - b1^2
                    // 3. b1 == a2, a1 == -b2 -> b1^2 - a1^2
                    // 4. b1 == b2, a1 == -a2 -> b1^2 - a1^2

                    let is_neg = |x: &Expr, y: &Expr| -> bool {
                        // Check if x == -y
                        if let Expr::Mul(c, inner) = x {
                            if matches!(**c, Expr::Number(n) if n == -1.0) && **inner == *y {
                                return true;
                            }
                        }
                        if let Expr::Mul(c, inner) = y {
                            if matches!(**c, Expr::Number(n) if n == -1.0) && **inner == *x {
                                return true;
                            }
                        }
                        false
                    };

                    if a1 == a2 && is_neg(&b1, &b2) {
                        // (A + B)(A - B) -> A^2 - B^2
                        // B is the one that is negated.
                        // The positive one in the pair is B.
                        // Wait, if b1 = -b2, then b1^2 = b2^2.
                        // Result is a1^2 - b1^2 (where b1 is the term magnitude).
                        // Actually result is a1^2 - (term that flipped)^2.
                        // If b1 = -b2, then b1 is -B, b2 is B.
                        // (A - B)(A + B) = A^2 - B^2.
                        // So we subtract the square of the term that changed sign.

                        // We need the absolute value of b1/b2.
                        let b_abs = if let Expr::Mul(c, inner) = &b1 {
                            if matches!(**c, Expr::Number(n) if n == -1.0) {
                                *inner.clone()
                            } else {
                                b1.clone()
                            }
                        } else {
                            b1.clone()
                        };

                        return Some(Expr::Sub(
                            Box::new(Expr::Pow(Box::new(a1.clone()), Box::new(Expr::Number(2.0)))),
                            Box::new(Expr::Pow(Box::new(b_abs), Box::new(Expr::Number(2.0)))),
                        ));
                    }

                    if a1 == b2 && is_neg(&b1, &a2) {
                        let b_abs = if let Expr::Mul(c, inner) = &b1 {
                            if matches!(**c, Expr::Number(n) if n == -1.0) {
                                *inner.clone()
                            } else {
                                b1.clone()
                            }
                        } else {
                            b1.clone()
                        };
                        return Some(Expr::Sub(
                            Box::new(Expr::Pow(Box::new(a1.clone()), Box::new(Expr::Number(2.0)))),
                            Box::new(Expr::Pow(Box::new(b_abs), Box::new(Expr::Number(2.0)))),
                        ));
                    }

                    if b1 == a2 && is_neg(&a1, &b2) {
                        let a_abs = if let Expr::Mul(c, inner) = &a1 {
                            if matches!(**c, Expr::Number(n) if n == -1.0) {
                                *inner.clone()
                            } else {
                                a1.clone()
                            }
                        } else {
                            a1.clone()
                        };
                        return Some(Expr::Sub(
                            Box::new(Expr::Pow(Box::new(b1.clone()), Box::new(Expr::Number(2.0)))),
                            Box::new(Expr::Pow(Box::new(a_abs), Box::new(Expr::Number(2.0)))),
                        ));
                    }

                    if b1 == b2 && is_neg(&a1, &a2) {
                        let a_abs = if let Expr::Mul(c, inner) = &a1 {
                            if matches!(**c, Expr::Number(n) if n == -1.0) {
                                *inner.clone()
                            } else {
                                a1.clone()
                            }
                        } else {
                            a1.clone()
                        };
                        return Some(Expr::Sub(
                            Box::new(Expr::Pow(Box::new(b1.clone()), Box::new(Expr::Number(2.0)))),
                            Box::new(Expr::Pow(Box::new(a_abs), Box::new(Expr::Number(2.0)))),
                        ));
                    }
                }
            }
            None
        }
    }

    /// Rule for factoring difference of squares: a^2 - b^2 -> (a-b)(a+b)
    /// Low priority - only factors if expansion didn't lead to cancellations
    pub struct FactorDifferenceOfSquaresRule;

    impl Rule for FactorDifferenceOfSquaresRule {
        fn name(&self) -> &'static str {
            "factor_difference_of_squares"
        }

        fn priority(&self) -> i32 {
            10 // Low priority - only factor after all simplifications attempted
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            // Detect a^2 - b^2 pattern and factor to (a-b)(a+b)
            if let Some((term1, term2, is_sub)) = match expr {
                Expr::Add(u, v) => Some((&**u, &**v, false)),
                Expr::Sub(u, v) => Some((&**u, &**v, true)),
                _ => None,
            } {
                let is_square = |e: &Expr| -> Option<(f64, Expr)> {
                    match e {
                        Expr::Pow(base, exp) => {
                            if let Expr::Number(n) = **exp {
                                if n == 2.0 {
                                    return Some((1.0, *base.clone()));
                                }
                                // Handle even powers: x^4 = (x^2)^2, x^6 = (x^3)^2, etc.
                                if n > 0.0 && (n % 2.0).abs() < 1e-10 {
                                    let half_exp = n / 2.0;
                                    let new_base = Expr::Pow(base.clone(), Box::new(Expr::Number(half_exp)));
                                    return Some((1.0, new_base));
                                }
                            }
                            None
                        }
                        Expr::Mul(coeff, rest) => {
                            if let Expr::Number(c) = **coeff {
                                if let Expr::Pow(base, exp) = &**rest {
                                    if matches!(**exp, Expr::Number(n) if n == 2.0) {
                                        return Some((c, *base.clone()));
                                    }
                                }
                                // Handle Mul(-1, Number(1)) as -1 = -1 * 1^2
                                if let Expr::Number(n) = **rest {
                                    if n.abs() == 1.0 {
                                        return Some((c * n, Expr::Number(1.0)));
                                    }
                                }
                            }
                            None
                        }
                        // Handle standalone numbers as coefficient * 1^2
                        Expr::Number(n) if n.abs() == 1.0 => {
                            Some((*n, Expr::Number(1.0)))
                        }
                        _ => None,
                    }
                };
                
                if let (Some((c1, base1)), Some((c2, base2))) = (is_square(term1), is_square(term2)) {
                    // For Sub: both coefficients should be positive
                    // For Add: c1 positive, c2 negative (canonical form)
                    let (c1_final, c2_final) = if is_sub {
                        (c1, -c2)  // Convert Sub to Add form for checking
                    } else {
                        (c1, c2)
                    };
                    
                    let sqrt_c1 = c1_final.abs().sqrt();
                    let sqrt_c2 = c2_final.abs().sqrt();
                    
                    // Check if both coefficients are perfect squares and have opposite signs
                    if (sqrt_c1 - sqrt_c1.round()).abs() < 1e-10 
                        && (sqrt_c2 - sqrt_c2.round()).abs() < 1e-10
                        && (c1_final * c2_final) < 0.0  // Opposite signs
                        && (sqrt_c1.round() - sqrt_c2.round()).abs() < 1e-10 // Same magnitude
                    {
                        // We have sqrt(c)^2 * a^2 - sqrt(c)^2 * b^2 = (sqrt(c)*a)^2 - (sqrt(c)*b)^2
                        let sqrt_c = sqrt_c1.round();
                        
                        let make_term = |base: &Expr| -> Expr {
                            if (sqrt_c - 1.0).abs() < 1e-10 {
                                base.clone()
                            } else {
                                Expr::Mul(Box::new(Expr::Number(sqrt_c)), Box::new(base.clone()))
                            }
                        };
                        
                        // Determine which term is positive and which is negative
                        // If c1_final is negative (like in 1 - x^2 = 1 - 1*x^2), we need to negate
                        let (a, b, needs_negation) = if c1_final > 0.0 {
                            // Standard case: a^2 - b^2 = (a-b)(a+b)
                            (make_term(&base1), make_term(&base2), false)
                        } else {
                            // Reversed case: -a^2 + b^2 = -(a^2 - b^2) = -(a-b)(a+b) = (b-a)(b+a)
                            // Or equivalently: b^2 - a^2 = (b-a)(b+a)
                            (make_term(&base2), make_term(&base1), false)
                        };
                        
                        // (a - b)(a + b)
                        let factored = Expr::Mul(
                            Box::new(Expr::Sub(Box::new(a.clone()), Box::new(b.clone()))),
                            Box::new(Expr::Add(Box::new(a), Box::new(b))),
                        );
                        
                        return Some(if needs_negation {
                            Expr::Mul(Box::new(Expr::Number(-1.0)), Box::new(factored))
                        } else {
                            factored
                        });
                    }
                }
            }
            None
        }
    }

    /// Rule for perfect cubes: a^3 + 3a^2b + 3ab^2 + b^3 -> (a+b)^3
    pub struct PerfectCubeRule;

    impl Rule for PerfectCubeRule {
        fn name(&self) -> &'static str {
            "perfect_cube"
        }

        fn priority(&self) -> i32 {
            50 // Higher priority to catch specific patterns before general factoring
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Add(_, _) = expr {
                let terms = crate::simplification::helpers::flatten_add(expr.clone());
                if terms.len() != 4 {
                    return None;
                }

                // We're looking for a^3, 3*a^2*b, 3*a*b^2, b^3
                let mut cube_terms: Vec<(f64, Expr)> = Vec::new(); // (coefficient, base) for x^3
                let mut square_linear: Vec<(f64, Expr, Expr)> = Vec::new(); // (coeff, squared_var, linear_var) for 3*a^2*b
                let mut linear_square: Vec<(f64, Expr, Expr)> = Vec::new(); // (coeff, linear_var, squared_var) for 3*a*b^2

                for term in &terms {
                    match term {
                        // Match x^3
                        Expr::Pow(base, exp) if matches!(**exp, Expr::Number(n) if n == 3.0) => {
                            cube_terms.push((1.0, *base.clone()));
                        }
                        // Match constant - check if it's a perfect cube
                        Expr::Number(n) => {
                            let cbrt_n = n.cbrt();
                            if (cbrt_n - cbrt_n.round()).abs() < 1e-10 {
                                // It's a perfect cube like 1, 8, 27, etc.
                                // Store as (n, 1) so that the base is 1, matching implicit linear terms
                                cube_terms.push((*n, Expr::Number(1.0)));
                            }
                        }
                        // Match c*x^3 or c*(product with squares)
                        Expr::Mul(coeff, rest) if matches!(**coeff, Expr::Number(_)) => {
                            if let Expr::Number(c) = **coeff {
                                if let Expr::Pow(base, exp) = &**rest {
                                    if matches!(**exp, Expr::Number(n) if n == 3.0) {
                                        cube_terms.push((c, *base.clone()));
                                    } else if matches!(**exp, Expr::Number(n) if n == 2.0) {
                                        // c*a^2 without another factor means c*a^2*1
                                        square_linear.push((c, *base.clone(), Expr::Number(1.0)));
                                    }
                                } else if let Expr::Mul(inner1, inner2) = &**rest {
                                    // c*inner1*inner2 - could be 3*a^2*b or 3*a*b^2
                                    // Check if inner1 or inner2 is a square
                                    if let Expr::Pow(base, exp) = &**inner1 {
                                        if matches!(**exp, Expr::Number(n) if n == 2.0) {
                                            // c*(a^2)*b
                                            square_linear.push((c, *base.clone(), *inner2.clone()));
                                        }
                                    }
                                    if let Expr::Pow(base, exp) = &**inner2 {
                                        if matches!(**exp, Expr::Number(n) if n == 2.0) {
                                            // c*a*(b^2)
                                            linear_square.push((c, *inner1.clone(), *base.clone()));
                                        }
                                    }
                                } else {
                                    // c*a means c*a*1^2
                                    linear_square.push((c, *rest.clone(), Expr::Number(1.0)));
                                }
                            }
                        }
                        // Implicit coefficient 1 for linear terms (e.g. x)
                        // This is treated as 1 * x * 1^2 -> linear_square with coeff 1, linear x, squared 1
                        other => {
                            linear_square.push((1.0, other.clone(), Expr::Number(1.0)));
                        }
                    }
                }

                // We should have exactly 2 cubes (a^3 and b^3) and 1 each of square_linear and linear_square
                if cube_terms.len() == 2 && square_linear.len() == 1 && linear_square.len() == 1 {
                    let (c1, cube_a) = &cube_terms[0];
                    let (c2, cube_b) = &cube_terms[1];
                    let (coeff_a2b, sq_a, lin_b) = &square_linear[0];
                    let (coeff_ab2, lin_a, sq_b) = &linear_square[0];

                    // Normalize coefficients by taking cube root
                    // If c1*a^3, effective term is (cbrt(c1)*a)^3
                    let cbrt_c1 = c1.cbrt();
                    let cbrt_c2 = c2.cbrt();

                    if (cbrt_c1 - cbrt_c1.round()).abs() < 1e-10
                        && (cbrt_c2 - cbrt_c2.round()).abs() < 1e-10
                    {
                        // Effective bases
                        let eff_a = if (cbrt_c1 - 1.0).abs() < 1e-10 {
                            cube_a.clone()
                        } else {
                            Expr::Mul(
                                Box::new(Expr::Number(cbrt_c1.round())),
                                Box::new(cube_a.clone()),
                            )
                        };

                        let eff_b = if (cbrt_c2 - 1.0).abs() < 1e-10 {
                            cube_b.clone()
                        } else {
                            Expr::Mul(
                                Box::new(Expr::Number(cbrt_c2.round())),
                                Box::new(cube_b.clone()),
                            )
                        };

                        // Check cross terms
                        // 3 * eff_a^2 * eff_b
                        // = 3 * (cbrt(c1)*cube_a)^2 * (cbrt(c2)*cube_b)
                        // = 3 * cbrt(c1)^2 * cbrt(c2) * cube_a^2 * cube_b
                        // Compare with coeff_a2b * sq_a^2 * lin_b

                        let expected_coeff_a2b = 3.0 * cbrt_c1.powi(2) * cbrt_c2;
                        let expected_coeff_ab2 = 3.0 * cbrt_c1 * cbrt_c2.powi(2);

                        // Check matches
                        // Case 1: sq_a matches cube_a, lin_b matches cube_b
                        let match1 = (sq_a == cube_a && lin_b == cube_b)
                            && (lin_a == cube_a && sq_b == cube_b);

                        // Case 2: sq_a matches cube_b, lin_b matches cube_a (swapped roles)
                        // But we fixed eff_a and eff_b based on c1/c2 order.
                        // So we just need to check if the cross terms match the expected values for THIS a/b assignment.
                        // Or if they match the swapped assignment.

                        // Let's check if current assignment works
                        if match1 {
                            if (expected_coeff_a2b - coeff_a2b).abs() < 1e-10
                                && (expected_coeff_ab2 - coeff_ab2).abs() < 1e-10
                            {
                                return Some(Expr::Pow(
                                    Box::new(Expr::Add(Box::new(eff_a), Box::new(eff_b))),
                                    Box::new(Expr::Number(3.0)),
                                ));
                            }
                        } else {
                            // Try swapping a and b in the cross term matching
                            // If sq_a == cube_b and lin_b == cube_a
                            // Then coeff_a2b corresponds to 3*b^2*a term
                            // And coeff_ab2 corresponds to 3*b*a^2 term
                            let match2 = (sq_a == cube_b && lin_b == cube_a)
                                && (lin_a == cube_b && sq_b == cube_a);

                            if match2 {
                                // coeff_a2b is for b^2*a, so it should match 3*b^2*a -> expected_coeff_ab2 (relative to a,b)
                                // expected_coeff_ab2 = 3 * a * b^2
                                // coeff_ab2 is for b*a^2, so it should match 3*b*a^2 -> expected_coeff_a2b (relative to a,b)

                                if (expected_coeff_ab2 - coeff_a2b).abs() < 1e-10
                                    && (expected_coeff_a2b - coeff_ab2).abs() < 1e-10
                                {
                                    return Some(Expr::Pow(
                                        Box::new(Expr::Add(Box::new(eff_a), Box::new(eff_b))),
                                        Box::new(Expr::Number(3.0)),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            None
        }
    }

    /// Rule for combining like factors in multiplication: x * x -> x^2
    pub struct CombineFactorsRule;

    impl Rule for CombineFactorsRule {
        fn name(&self) -> &'static str {
            "combine_factors"
        }

        fn priority(&self) -> i32 {
            45
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Mul(_, _) = expr {
                let terms = crate::simplification::helpers::flatten_mul(expr);
                if terms.len() < 2 {
                    return None;
                }

                // Remove factors of 1 and handle 0
                // Also, multiply all numeric terms together
                let mut numeric_product = 1.0;
                let mut non_numeric_terms = Vec::new();

                for term in terms {
                    match term {
                        Expr::Number(n) => {
                            if n == 0.0 {
                                // Anything times 0 is 0
                                return Some(Expr::Number(0.0));
                            }
                            // Multiply into the numeric product
                            numeric_product *= n;
                        }
                        other => non_numeric_terms.push(other),
                    }
                }

                // Add the numeric product back if it's not 1
                let mut filtered_terms = Vec::new();
                if (numeric_product - 1.0).abs() > 1e-10 {
                    filtered_terms.push(Expr::Number(numeric_product));
                }
                filtered_terms.extend(non_numeric_terms);

                if filtered_terms.is_empty() {
                    return Some(Expr::Number(1.0));
                }

                // Sort terms to group by base
                filtered_terms.sort_by(|a, b| {
                    let base_a = if let Expr::Pow(b, _) = a { &**b } else { a };
                    let base_b = if let Expr::Pow(b, _) = b { &**b } else { b };

                    match crate::simplification::helpers::compare_expr(base_a, base_b) {
                        std::cmp::Ordering::Equal => {
                            let one = Expr::Number(1.0);
                            let exp_a = if let Expr::Pow(_, e) = a { &**e } else { &one };
                            let exp_b = if let Expr::Pow(_, e) = b { &**e } else { &one };
                            crate::simplification::helpers::compare_expr(exp_a, exp_b)
                        }
                        ord => ord,
                    }
                });

                let mut grouped_terms: Vec<(Expr, Expr)> = Vec::new();
                let mut iter = filtered_terms.into_iter();

                // Initialize with first term
                if let Some(first) = iter.next() {
                    if let Expr::Pow(b, e) = first {
                        grouped_terms.push((*b, *e));
                    } else {
                        grouped_terms.push((first, Expr::Number(1.0)));
                    }
                }

                for term in iter {
                    let (term_base, term_exp) = if let Expr::Pow(b, e) = term {
                        (*b, *e)
                    } else {
                        (term, Expr::Number(1.0))
                    };

                    let mut merged = false;
                    if let Some((last_base, last_exp)) = grouped_terms.last_mut() {
                        if *last_base == term_base {
                            // Simplify exponent sum if both are numbers
                            let new_exp = if let (Expr::Number(n1), Expr::Number(n2)) =
                                (last_exp.clone(), term_exp.clone())
                            {
                                Expr::Number(n1 + n2)
                            } else {
                                Expr::Add(Box::new(last_exp.clone()), Box::new(term_exp.clone()))
                            };
                            *last_exp = new_exp;
                            merged = true;
                        }
                    }

                    if !merged {
                        grouped_terms.push((term_base, term_exp));
                    }
                }

                let mut result = Vec::new();
                for (base, exp) in grouped_terms {
                    if matches!(exp, Expr::Number(n) if n == 1.0) {
                        result.push(base);
                    } else {
                        result.push(Expr::Pow(Box::new(base), Box::new(exp)));
                    }
                }

                let new_expr = crate::simplification::helpers::rebuild_mul(result);
                if new_expr != *expr {
                    return Some(new_expr);
                }
            }
            None
        }
    }

    /// Rule for combining like terms in addition: 2x + 3x -> 5x, x^2 - x^2 -> 0
    /// High priority to enable cancellations before other transformations
    pub struct CombineLikeTermsInAdditionRule;

    impl Rule for CombineLikeTermsInAdditionRule {
        fn name(&self) -> &'static str {
            "combine_like_terms_addition"
        }

        fn priority(&self) -> i32 {
            80 // High priority to combine terms early for cancellations
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Add(_, _) = expr {
                // Flatten all nested additions
                let terms = crate::simplification::helpers::flatten_add(expr.clone());
                
                if terms.len() < 2 {
                    return None;
                }
                
                // Group terms by their "base" (the part without numeric coefficient)
                // Map from base expression to coefficient sum
                use std::collections::HashMap;
                let mut term_groups: HashMap<String, (f64, Expr)> = HashMap::new();
                
                for term in terms {
                    let (coeff, base) = crate::simplification::helpers::extract_coeff(&term);
                    let base_key = format!("{:?}", base);
                    
                    term_groups.entry(base_key)
                        .and_modify(|(c, _)| *c += coeff)
                        .or_insert((coeff, base));
                }
                
                // Rebuild terms
                let mut result_terms = Vec::new();
                for (_key, (coeff, base)) in term_groups {
                    if coeff.abs() < 1e-10 {
                        // Coefficient is zero, skip this term
                        continue;
                    } else if (coeff - 1.0).abs() < 1e-10 {
                        // Coefficient is 1, just add the base
                        result_terms.push(base);
                    } else if (coeff + 1.0).abs() < 1e-10 {
                        // Coefficient is -1
                        result_terms.push(Expr::Mul(
                            Box::new(Expr::Number(-1.0)),
                            Box::new(base),
                        ));
                    } else {
                        // General case: coeff * base
                        result_terms.push(Expr::Mul(
                            Box::new(Expr::Number(coeff)),
                            Box::new(base),
                        ));
                    }
                }
                
                if result_terms.is_empty() {
                    return Some(Expr::Number(0.0));
                }
                
                if result_terms.len() == 1 {
                    return Some(result_terms.into_iter().next().unwrap());
                }
                
                let new_expr = crate::simplification::helpers::rebuild_add(result_terms);
                
                // Only return if something changed
                if new_expr != *expr {
                    return Some(new_expr);
                }
            }
            None
        }
    }

    /// Rule for canonicalizing multiplication: flatten nested Mul and order terms
    /// (x*y)*z -> x*y*z, z*x*y -> x*y*z (alphabetical, then by power)
    pub struct CanonicalizeMultiplicationRule;

    impl Rule for CanonicalizeMultiplicationRule {
        fn name(&self) -> &'static str {
            "canonicalize_multiplication"
        }

        fn priority(&self) -> i32 {
            15 // Low priority, after most simplifications but before display normalization
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Mul(_, _) = expr {
                // Flatten all nested multiplications
                let factors = crate::simplification::helpers::flatten_mul(expr);
                
                if factors.len() < 2 {
                    return None;
                }
                
                // Separate numeric coefficients from symbolic terms
                let mut coeff = 1.0;
                let mut symbolic_terms = Vec::new();
                
                for factor in factors {
                    if let Expr::Number(n) = factor {
                        coeff *= n;
                    } else {
                        symbolic_terms.push(factor);
                    }
                }
                
                if symbolic_terms.is_empty() {
                    return Some(Expr::Number(coeff));
                }
                
                // Sort symbolic terms: alphabetically by variable name, then by power (descending)
                symbolic_terms.sort_by(|a, b| {
                    let a_key = Self::get_sort_key(a);
                    let b_key = Self::get_sort_key(b);
                    a_key.cmp(&b_key)
                });
                
                // Build result with coefficient first (if not 1)
                let mut result_terms = Vec::new();
                if (coeff - 1.0).abs() > 1e-10 {
                    result_terms.push(Expr::Number(coeff));
                }
                result_terms.extend(symbolic_terms);
                
                let new_expr = crate::simplification::helpers::rebuild_mul(result_terms);
                
                // Only return if something changed
                if new_expr != *expr {
                    return Some(new_expr);
                }
            }
            None
        }
    }

    impl CanonicalizeMultiplicationRule {
        /// Get a sort key for ordering: (variable_name, -power)
        /// Negative power so higher powers come first
        fn get_sort_key(expr: &Expr) -> (String, i32) {
            match expr {
                Expr::Symbol(name) => (name.clone(), -1),
                Expr::Pow(base, exp) => {
                    let base_name = match &**base {
                        Expr::Symbol(name) => name.clone(),
                        _ => format!("{:?}", base), // Fallback for complex bases
                    };
                    let power = match &**exp {
                        Expr::Number(n) => -(*n as i32), // Negative for descending order
                        _ => -999, // Complex exponents sorted first
                    };
                    (base_name, power)
                }
                Expr::FunctionCall { name, .. } => (name.clone(), 0),
                _ => (format!("{:?}", expr), 0),
            }
        }
    }

    /// Rule for canonicalizing addition (ordering terms)
    pub struct CanonicalizeAdditionRule;

    impl Rule for CanonicalizeAdditionRule {
        fn name(&self) -> &'static str {
            "canonicalize_addition"
        }

        fn priority(&self) -> i32 {
            15 // Same as multiplication canonicalization
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Add(_, _) = expr {
                // Flatten all nested additions
                let terms = crate::simplification::helpers::flatten_add(expr.clone());
                
                if terms.len() < 2 {
                    return None;
                }
                
                // Separate numeric constants from symbolic terms
                let mut constant = 0.0;
                let mut symbolic_terms = Vec::new();
                
                for term in terms {
                    if let Expr::Number(n) = term {
                        constant += n;
                    } else {
                        symbolic_terms.push(term);
                    }
                }
                
                // Sort symbolic terms: by polynomial degree (descending), then alphabetically
                symbolic_terms.sort_by(|a, b| {
                    let deg_a = get_polynomial_degree(a);
                    let deg_b = get_polynomial_degree(b);
                    match deg_b.cmp(&deg_a) {
                        std::cmp::Ordering::Equal => {
                            // Same degree, sort by expression structure
                            let a_key = Self::get_sort_key(a);
                            let b_key = Self::get_sort_key(b);
                            a_key.cmp(&b_key)
                        }
                        ord => ord,
                    }
                });
                
                // Build result with symbolic terms first, then constant (if not 0)
                let mut result_terms = symbolic_terms;
                if constant.abs() > 1e-10 {
                    result_terms.push(Expr::Number(constant));
                }
                
                if result_terms.is_empty() {
                    return Some(Expr::Number(0.0));
                }
                
                let new_expr = crate::simplification::helpers::rebuild_add(result_terms);
                
                // Only return if something changed
                if new_expr != *expr {
                    return Some(new_expr);
                }
            }
            None
        }
    }

    impl CanonicalizeAdditionRule {
        /// Get a sort key for ordering terms in addition
        /// Priority: Pow > Symbol > FunctionCall > Complex expressions
        fn get_sort_key(expr: &Expr) -> (u8, String, i32) {
            match expr {
                Expr::Symbol(name) => (1, name.clone(), -1),
                Expr::Pow(base, exp) => {
                    let base_name = match &**base {
                        Expr::Symbol(name) => name.clone(),
                        _ => format!("{:?}", base),
                    };
                    let power = match &**exp {
                        Expr::Number(n) => -(*n as i32), // Negative for descending order
                        _ => -999,
                    };
                    (0, base_name, power) // Priority 0 for Pow (comes first)
                }
                Expr::Mul(_, _) => {
                    // For multiplication terms, extract the main variable
                    let factors = crate::simplification::helpers::flatten_mul(expr);
                    for factor in &factors {
                        if let Expr::Symbol(name) = factor {
                            return (1, name.clone(), -1);
                        } else if let Expr::Pow(base, exp) = factor {
                            if let Expr::Symbol(name) = &**base {
                                let power = match &**exp {
                                    Expr::Number(n) => -(*n as i32),
                                    _ => -999,
                                };
                                return (0, name.clone(), power);
                            }
                        }
                    }
                    (2, format!("{:?}", expr), 0)
                }
                Expr::FunctionCall { name, .. } => (2, name.clone(), 0),
                _ => (3, format!("{:?}", expr), 0), // Complex expressions last
            }
        }
    }

    /// Rule for canonicalizing subtraction (ordering terms in the subtracted part)
    pub struct CanonicalizeSubtractionRule;

    impl Rule for CanonicalizeSubtractionRule {
        fn name(&self) -> &'static str {
            "canonicalize_subtraction"
        }

        fn priority(&self) -> i32 {
            15 // Same as other canonicalization
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Sub(a, b) = expr {
                // Recursively canonicalize the subtracted expression if it's an addition
                if let Expr::Add(_, _) = &**b {
                    let terms = crate::simplification::helpers::flatten_add(*b.clone());
                    
                    if terms.len() < 2 {
                        return None;
                    }
                    
                    // Separate numeric constants from symbolic terms
                    let mut constant = 0.0;
                    let mut symbolic_terms = Vec::new();
                    
                    for term in terms {
                        if let Expr::Number(n) = term {
                            constant += n;
                        } else {
                            symbolic_terms.push(term);
                        }
                    }
                    
                    // Sort symbolic terms
                    symbolic_terms.sort_by(|x, y| {
                        let deg_x = get_polynomial_degree(x);
                        let deg_y = get_polynomial_degree(y);
                        match deg_y.cmp(&deg_x) {
                            std::cmp::Ordering::Equal => {
                                let x_key = CanonicalizeAdditionRule::get_sort_key(x);
                                let y_key = CanonicalizeAdditionRule::get_sort_key(y);
                                x_key.cmp(&y_key)
                            }
                            ord => ord,
                        }
                    });
                    
                    // Build result
                    let mut result_terms = symbolic_terms;
                    if constant.abs() > 1e-10 {
                        result_terms.push(Expr::Number(constant));
                    }
                    
                    if !result_terms.is_empty() {
                        let new_b = crate::simplification::helpers::rebuild_add(result_terms);
                        let new_expr = Expr::Sub(a.clone(), Box::new(new_b));
                        
                        if new_expr != *expr {
                            return Some(new_expr);
                        }
                    }
                }
            }
            None
        }
    }

    /// Rule for normalizing addition with negation to subtraction
    /// a + (-b) -> a - b, (-a) + b -> b - a
    pub struct NormalizeAddNegationRule;

    impl Rule for NormalizeAddNegationRule {
        fn name(&self) -> &'static str {
            "normalize_add_negation"
        }

        fn priority(&self) -> i32 {
            5 // Very low priority - run after all pattern matching for cleaner display
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Add(u, v) = expr {
                // Case 1: a + (-b) -> a - b
                // Check if v is a negative term: -b, -1*b, or Mul(..., -1, ...)
                if let Some(positive_v) = extract_negation(v) {
                    return Some(Expr::Sub(u.clone(), Box::new(positive_v)));
                }

                // Case 2: (-a) + b -> b - a
                if let Some(positive_u) = extract_negation(u) {
                    return Some(Expr::Sub(v.clone(), Box::new(positive_u)));
                }
            }
            None
        }
    }

    /// Helper function to extract the positive part if expression is negated
    /// Returns Some(x) if expr represents -x, None otherwise
    fn extract_negation(expr: &Expr) -> Option<Expr> {
        match expr {
            // Direct multiplication by -1: -1 * x or x * -1
            Expr::Mul(a, b) => {
                if matches!(**a, Expr::Number(n) if n == -1.0) {
                    return Some(*b.clone());
                }
                if matches!(**b, Expr::Number(n) if n == -1.0) {
                    return Some(*a.clone());
                }

                // Check for more complex cases: (-1) * a * b -> a * b
                let factors = crate::simplification::helpers::flatten_mul(expr);
                let mut has_neg_one = false;
                let mut other_factors = Vec::new();

                for factor in factors {
                    if matches!(factor, Expr::Number(n) if n == -1.0) {
                        has_neg_one = true;
                    } else {
                        other_factors.push(factor);
                    }
                }

                if has_neg_one && !other_factors.is_empty() {
                    return Some(crate::simplification::helpers::rebuild_mul(other_factors));
                }

                None
            }
            // Negative number
            Expr::Number(n) if *n < 0.0 => Some(Expr::Number(-n)),
            _ => None,
        }
    }

    /// Rule for distributing negation: -(A + B) -> -A - B
    pub struct DistributeNegationRule;

    impl Rule for DistributeNegationRule {
        fn name(&self) -> &'static str {
            "distribute_negation"
        }

        fn priority(&self) -> i32 {
            90
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            if let Expr::Mul(c, inner) = expr {
                if matches!(**c, Expr::Number(n) if n == -1.0) {
                    // -(A + B) -> -A - B
                    if let Expr::Add(a, b) = &**inner {
                        // Check if b is -1 * B -> -(A - B) -> B - A
                        if let Expr::Mul(c_b, val_b) = &**b {
                            if matches!(**c_b, Expr::Number(n) if n == -1.0) {
                                return Some(Expr::Sub(val_b.clone(), a.clone()));
                            }
                        }
                        // Check if a is -1 * A -> -(-A + B) -> A - B
                        if let Expr::Mul(c_a, val_a) = &**a {
                            if matches!(**c_a, Expr::Number(n) if n == -1.0) {
                                return Some(Expr::Sub(val_a.clone(), b.clone()));
                            }
                        }

                        return Some(Expr::Sub(
                            Box::new(Expr::Mul(Box::new(Expr::Number(-1.0)), a.clone())),
                            b.clone(),
                        ));
                    }
                    // -(A - B) -> B - A
                    if let Expr::Sub(a, b) = &**inner {
                        return Some(Expr::Sub(b.clone(), a.clone()));
                    }
                }
            }
            None
        }
    }

    /// Helper function to compute GCD of two numbers
    fn gcd_f64(a: f64, b: f64) -> f64 {
        let a = a.abs();
        let b = b.abs();
        
        // Handle near-zero values
        if a < 1e-10 {
            return b;
        }
        if b < 1e-10 {
            return a;
        }
        
        // For integers, use Euclidean algorithm
        if (a - a.round()).abs() < 1e-10 && (b - b.round()).abs() < 1e-10 {
            let mut a = a.round() as i64;
            let mut b = b.round() as i64;
            while b != 0 {
                let temp = b;
                b = a % b;
                a = temp;
            }
            return a.abs() as f64;
        }
        
        // For non-integers, return 1
        1.0
    }

    /// Rule for factoring out numeric GCD from addition: 2*a + 2*b -> 2*(a+b)
    pub struct NumericGcdFactoringRule;

    impl Rule for NumericGcdFactoringRule {
        fn name(&self) -> &'static str {
            "numeric_gcd_factoring"
        }

        fn priority(&self) -> i32 {
            42 // Run after combining like terms but before general factoring
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            // Handle both Add and Sub
            let terms = match expr {
                Expr::Add(_, _) => crate::simplification::helpers::flatten_add(expr.clone()),
                Expr::Sub(a, b) => vec![
                    (**a).clone(),
                    Expr::Mul(Box::new(Expr::Number(-1.0)), b.clone()),
                ],
                _ => return None,
            };

            if terms.len() < 2 {
                return None;
            }

            // Extract coefficients from each term
            let mut coefficients = Vec::new();
            let mut symbolic_parts = Vec::new();

            for term in &terms {
                let (coeff, symbolic) = crate::simplification::helpers::extract_coeff(term);
                coefficients.push(coeff);
                symbolic_parts.push(symbolic);
            }

            // Compute GCD of all coefficients
            let mut result_gcd = coefficients[0].abs();
            for &coeff in &coefficients[1..] {
                result_gcd = gcd_f64(result_gcd, coeff.abs());
            }

            // Only factor out if GCD > 1
            if result_gcd <= 1.0 + 1e-10 {
                return None;
            }

            // Factor out the GCD
            let mut new_terms = Vec::new();
            for i in 0..terms.len() {
                let new_coeff = coefficients[i] / result_gcd;
                
                if (new_coeff - 1.0).abs() < 1e-10 {
                    // Coefficient is 1
                    new_terms.push(symbolic_parts[i].clone());
                } else if (new_coeff + 1.0).abs() < 1e-10 {
                    // Coefficient is -1
                    new_terms.push(Expr::Mul(
                        Box::new(Expr::Number(-1.0)),
                        Box::new(symbolic_parts[i].clone()),
                    ));
                } else {
                    // General coefficient
                    new_terms.push(Expr::Mul(
                        Box::new(Expr::Number(new_coeff)),
                        Box::new(symbolic_parts[i].clone()),
                    ));
                }
            }

            let sum = crate::simplification::helpers::rebuild_add(new_terms);
            Some(Expr::Mul(Box::new(Expr::Number(result_gcd)), Box::new(sum)))
        }
    }

    /// Rule for factoring out common terms: ax + bx -> x(a+b)
    pub struct CommonTermFactoringRule;

    impl Rule for CommonTermFactoringRule {
        fn name(&self) -> &'static str {
            "common_term_factoring"
        }

        fn priority(&self) -> i32 {
            40 // Run after combining like terms
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            // Handle both Add and Sub
            let (terms, _is_sub) = match expr {
                Expr::Add(_, _) => {
                    let terms = crate::simplification::helpers::flatten_add(expr.clone());
                    (terms, false)
                }
                Expr::Sub(a, b) => {
                    // Treat x - y as x + (-1)*y for factoring purposes
                    let terms = vec![
                        (**a).clone(),
                        Expr::Mul(Box::new(Expr::Number(-1.0)), b.clone()),
                    ];
                    (terms, true)
                }
                _ => return None,
            };

            if terms.len() < 2 {
                return None;
            }

            // General factoring: find common factors across all terms
            // 1. Collect all factors for each term
            let mut term_factors: Vec<Vec<Expr>> = Vec::new();
            for term in &terms {
                term_factors.push(crate::simplification::helpers::flatten_mul(term));
            }

            // 2. Find intersection of factors
            let first_term_factors = &term_factors[0];
            let mut common_candidates = Vec::new();

            // Count factors in first term
            let mut checked_indices = vec![false; first_term_factors.len()];
            for (i, factor) in first_term_factors.iter().enumerate() {
                if checked_indices[i] {
                    continue;
                }
                let mut count = 0;
                for (j, f) in first_term_factors.iter().enumerate() {
                    if !checked_indices[j] && f == factor {
                        count += 1;
                        checked_indices[j] = true;
                    }
                }
                common_candidates.push((factor.clone(), count));
            }

            // Filter candidates by checking other terms
            for factors in &term_factors[1..] {
                for (candidate, min_count) in &mut common_candidates {
                    if *min_count == 0 {
                        continue;
                    }

                    // Count occurrences of candidate in this term
                    let mut count = 0;
                    for f in factors {
                        if f == candidate {
                            count += 1;
                        }
                    }
                    *min_count = (*min_count).min(count);
                }
            }

            // 3. Construct common factor
            let mut common_factor_parts = Vec::new();
            for (factor, count) in common_candidates {
                for _ in 0..count {
                    common_factor_parts.push(factor.clone());
                }
            }

            if common_factor_parts.is_empty() {
                return None;
            }

            // 4. Factor out common parts
            let common_factor =
                crate::simplification::helpers::rebuild_mul(common_factor_parts.clone());
            let mut remaining_terms = Vec::new();

            for factors in term_factors {
                // Remove common factors
                let mut current_factors = factors;
                for common in &common_factor_parts {
                    if let Some(pos) = current_factors.iter().position(|x| x == common) {
                        current_factors.remove(pos);
                    }
                }
                remaining_terms.push(crate::simplification::helpers::rebuild_mul(current_factors));
            }

            // 5. Build result: common_factor * (sum of remaining terms)
            // Always use Add form since we already normalized Sub to Add with -1 coefficients
            let sum_remaining = crate::simplification::helpers::rebuild_add(remaining_terms);
            return Some(Expr::Mul(Box::new(common_factor), Box::new(sum_remaining)));
        }
    }

    /// Rule for expanding polynomials with powers ≤ 3 to enable cancellations
    /// Expands (a+b)^n for n=2,3 and (a-b)^n for n=2,3 only when beneficial
    /// Specifically: when numerator/denominator has matching terms that could cancel after expansion
    pub struct PolynomialExpansionRule;

    impl Rule for PolynomialExpansionRule {
        fn name(&self) -> &'static str {
            "polynomial_expansion"
        }

        fn priority(&self) -> i32 {
            92 // High priority to expand before cancellation attempts
        }

        fn category(&self) -> RuleCategory {
            RuleCategory::Algebraic
        }

        fn apply(&self, expr: &Expr, _context: &RuleContext) -> Option<Expr> {
            // Helper to check if expansion would be beneficial
            let would_help_cancellation = |poly_expr: &Expr, other_side: &Expr| -> bool {
                if let Expr::Pow(base, _exp) = poly_expr {
                    match &**base {
                        Expr::Add(a, b) | Expr::Sub(a, b) => {
                            // Check if any component of the binomial appears in the other side
                            let contains_component = |expr: &Expr, component: &Expr| -> bool {
                                if expr == component {
                                    return true;
                                }
                                match expr {
                                    Expr::Mul(_, _) => {
                                        let factors = crate::simplification::helpers::flatten_mul(expr);
                                        factors.iter().any(|f| f == component)
                                    }
                                    Expr::Add(_, _) | Expr::Sub(_, _) => {
                                        let terms = crate::simplification::helpers::flatten_add(expr.clone());
                                        terms.iter().any(|t| t == component)
                                    }
                                    Expr::Pow(b, _) => b.as_ref() == component,
                                    _ => false,
                                }
                            };
                            
                            // Only expand if components appear in other side (potential for cancellation)
                            contains_component(other_side, a) || contains_component(other_side, b)
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            };

            // Only expand powers in division contexts when beneficial
            match expr {
                Expr::Div(num, den) => {
                    let expand_if_beneficial = |e: &Expr, other: &Expr| -> Option<Expr> {
                        if let Expr::Pow(base, exp) = e {
                            if let Expr::Number(n) = **exp {
                                if (n == 2.0 || n == 3.0) && would_help_cancellation(e, other) {
                                    // Check if base is a sum/difference
                                    match &**base {
                                        Expr::Add(a, b) => {
                                            return Some(Self::expand_binomial(a, b, n as i32, false));
                                        }
                                        Expr::Sub(a, b) => {
                                            return Some(Self::expand_binomial(a, b, n as i32, true));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        None
                    };

                    // Try expanding numerator if beneficial
                    if let Some(expanded_num) = expand_if_beneficial(num, den) {
                        return Some(Expr::Div(Box::new(expanded_num), den.clone()));
                    }

                    // Try expanding denominator if beneficial
                    if let Some(expanded_den) = expand_if_beneficial(den, num) {
                        return Some(Expr::Div(num.clone(), Box::new(expanded_den)));
                    }

                    // Check if numerator is a product containing expandable polynomial
                    if let Expr::Mul(_, _) = &**num {
                        let factors = crate::simplification::helpers::flatten_mul(num);
                        let mut changed = false;
                        let mut new_factors = Vec::new();

                        for factor in factors {
                            if let Some(expanded) = expand_if_beneficial(&factor, den) {
                                new_factors.push(expanded);
                                changed = true;
                            } else {
                                new_factors.push(factor);
                            }
                        }

                        if changed {
                            let new_num = crate::simplification::helpers::rebuild_mul(new_factors);
                            return Some(Expr::Div(Box::new(new_num), den.clone()));
                        }
                    }

                    // Check if denominator is a product containing expandable polynomial
                    if let Expr::Mul(_, _) = &**den {
                        let factors = crate::simplification::helpers::flatten_mul(den);
                        let mut changed = false;
                        let mut new_factors = Vec::new();

                        for factor in factors {
                            if let Some(expanded) = expand_if_beneficial(&factor, num) {
                                new_factors.push(expanded);
                                changed = true;
                            } else {
                                new_factors.push(factor);
                            }
                        }

                        if changed {
                            let new_den = crate::simplification::helpers::rebuild_mul(new_factors);
                            return Some(Expr::Div(num.clone(), Box::new(new_den)));
                        }
                    }
                }
                _ => {}
            }
            None
        }
    }

    impl PolynomialExpansionRule {
        /// Expand (a ± b)^n for n = 2 or 3
        fn expand_binomial(a: &Box<Expr>, b: &Box<Expr>, n: i32, is_sub: bool) -> Expr {
            // Helper to avoid creating unnecessary operations with 1
            let smart_mul = |coeff: f64, base: Expr| -> Expr {
                if (coeff - 1.0).abs() < 1e-10 {
                    base
                } else if (coeff + 1.0).abs() < 1e-10 {
                    Expr::Mul(Box::new(Expr::Number(-1.0)), Box::new(base))
                } else {
                    Expr::Mul(Box::new(Expr::Number(coeff)), Box::new(base))
                }
            };

            let smart_pow = |base: &Box<Expr>, exp: i32| -> Expr {
                if exp == 1 {
                    (**base).clone()
                } else if let Expr::Number(n) = **base {
                    Expr::Number(n.powi(exp))
                } else {
                    Expr::Pow(base.clone(), Box::new(Expr::Number(exp as f64)))
                }
            };

            let smart_product = |term1: Expr, term2: Expr| -> Expr {
                match (&term1, &term2) {
                    (Expr::Number(n1), Expr::Number(n2)) => Expr::Number(n1 * n2),
                    (Expr::Number(n), other) | (other, Expr::Number(n)) if (*n - 1.0).abs() < 1e-10 => other.clone(),
                    _ => Expr::Mul(Box::new(term1), Box::new(term2)),
                }
            };

            match n {
                2 => {
                    // (a+b)^2 = a^2 + 2ab + b^2
                    // (a-b)^2 = a^2 - 2ab + b^2
                    let a_sq = smart_pow(a, 2);
                    let b_sq = smart_pow(b, 2);
                    let ab = smart_product((**a).clone(), (**b).clone());
                    let two_ab = smart_mul(2.0, ab);

                    let middle_term = if is_sub {
                        smart_mul(-2.0, smart_product((**a).clone(), (**b).clone()))
                    } else {
                        two_ab
                    };

                    Expr::Add(
                        Box::new(Expr::Add(Box::new(a_sq), Box::new(middle_term))),
                        Box::new(b_sq),
                    )
                }
                3 => {
                    // (a+b)^3 = a^3 + 3a^2b + 3ab^2 + b^3
                    // (a-b)^3 = a^3 - 3a^2b + 3ab^2 - b^3
                    let a_cu = smart_pow(a, 3);
                    let b_cu = smart_pow(b, 3);
                    
                    let a_sq = smart_pow(a, 2);
                    let b_sq = smart_pow(b, 2);
                    
                    let a2b = smart_product(a_sq, (**b).clone());
                    let ab2 = smart_product((**a).clone(), b_sq);
                    
                    let three_a2b = smart_mul(3.0, a2b);
                    let three_ab2 = smart_mul(3.0, ab2);

                    if is_sub {
                        // a^3 - 3a^2b + 3ab^2 - b^3
                        let term1 = Expr::Add(
                            Box::new(a_cu),
                            Box::new(smart_mul(-3.0, smart_product(smart_pow(a, 2), (**b).clone()))),
                        );
                        let term2 = Expr::Add(
                            Box::new(three_ab2),
                            Box::new(smart_mul(-1.0, b_cu)),
                        );
                        Expr::Add(Box::new(term1), Box::new(term2))
                    } else {
                        // a^3 + 3a^2b + 3ab^2 + b^3
                        let term1 = Expr::Add(Box::new(a_cu), Box::new(three_a2b));
                        let term2 = Expr::Add(Box::new(three_ab2), Box::new(b_cu));
                        Expr::Add(Box::new(term1), Box::new(term2))
                    }
                }
                _ => {
                    // Shouldn't reach here based on the filter above
                    if is_sub {
                        Expr::Pow(Box::new(Expr::Sub(a.clone(), b.clone())), Box::new(Expr::Number(n as f64)))
                    } else {
                        Expr::Pow(Box::new(Expr::Add(a.clone(), b.clone())), Box::new(Expr::Number(n as f64)))
                    }
                }
            }
        }
    }
}

/// Get all algebraic rules in priority order
pub fn get_algebraic_rules() -> Vec<Rc<dyn Rule>> {
    vec![
        //Rc::new(rules::NormalizeAddNegationRule),       // Priority 98 - Normalize a + (-b) early
        Rc::new(rules::ExpandPowerForCancellationRule), // Priority 95
        Rc::new(rules::PolynomialExpansionRule),        // Priority 92 - Expand polynomials for cancellation
        Rc::new(rules::FractionCancellationRule),       // Priority 90
        Rc::new(rules::PowerZeroRule),
        Rc::new(rules::PowerOneRule),
        Rc::new(rules::DivDivRule), // NEW: Flatten nested divisions early
        Rc::new(rules::DivSelfRule),
        Rc::new(rules::ExpLnRule),
        Rc::new(rules::LnExpRule),
        Rc::new(rules::ExpMulLnRule),
        Rc::new(rules::EPowLnRule),    // NEW: Handle e^ln(x) form
        Rc::new(rules::EPowMulLnRule), // NEW: Handle e^(a*ln(b)) form
        Rc::new(rules::PowerPowerRule),
        Rc::new(rules::PowerDivRule),
        Rc::new(rules::PowerExpansionRule),
        Rc::new(rules::PowerCollectionRule),
        Rc::new(rules::PowerMulRule),
        Rc::new(rules::PowerDivRule),
        Rc::new(rules::CommonExponentMulRule),
        Rc::new(rules::CommonExponentDivRule),
        Rc::new(rules::PowerToSqrtRule),
        Rc::new(rules::PowerToCbrtRule),
        Rc::new(rules::NegativeExponentToFractionRule),
        Rc::new(rules::AddFractionRule),
        Rc::new(rules::MulDivCombinationRule),
        Rc::new(rules::CanonicalizeRule),
        Rc::new(rules::CombineFactorsRule),
        Rc::new(rules::CombineTermsRule),
        Rc::new(rules::NumericGcdFactoringRule),    // Priority 42 - Factor out numeric GCD
        Rc::new(rules::CommonTermFactoringRule),
        Rc::new(rules::PerfectSquareRule),
        Rc::new(rules::ExpandDifferenceOfSquaresProductRule),  // Priority 85 - Expand (a+b)(a-b) early for cancellations
        Rc::new(rules::CombineLikeTermsInAdditionRule),        // Priority 80 - Combine like terms for cancellations
        Rc::new(rules::FactorDifferenceOfSquaresRule),          // Priority 10 - Factor a^2-b^2 late if no cancellation
        Rc::new(rules::PerfectCubeRule),
        Rc::new(rules::DistributeNegationRule),
        Rc::new(rules::CanonicalizeMultiplicationRule), // Priority 15 - Order and flatten multiplication
        Rc::new(rules::CanonicalizeAdditionRule),       // Priority 15 - Order and flatten addition
        Rc::new(rules::CanonicalizeSubtractionRule),    // Priority 15 - Order subtraction terms
        Rc::new(rules::NormalizeAddNegationRule),       // Priority 5 - Normalize for display
    ]
}
