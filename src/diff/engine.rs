//! Differentiation engine - applies calculus rules (PHASE 2 ENHANCED)
//!
//! # Design Note: Inline Optimizations
//!
//! This module contains inline simplification checks (e.g., `0 + x → x`, `1 * x → x`)
//! during derivative computation. This is INTENTIONAL and NOT redundant with the
//! simplification module because:
//!
//! 1. **Preventing expression explosion**: Without inline optimization, differentiating
//!    a function like `sin(x^5)` would create massive intermediate expression trees
//!    before simplification runs.
//!
//! 2. **Performance**: The simplification engine does a full bottom-up tree traversal.
//!    Inline checks here are O(1) pattern matches on immediate operands.
//!
//! The simplification engine then handles any remaining optimization opportunities.

use crate::core::known_symbols as ks;
use crate::core::unified_context::Context;
use crate::{Expr, ExprKind};
use std::sync::Arc;

impl Expr {
    /// Differentiate this expression with respect to a variable
    ///
    /// # Arguments
    /// * `var` - Variable to differentiate with respect to
    /// * `context` - Optional context containing fixed vars and user-defined functions.
    ///   If None, uses an empty context (no fixed vars, no custom functions).
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{Context, UserFunction, Expr, symb};
    ///
    /// let ctx = Context::new()
    ///     .with_function("f", UserFunction::new(1..=1)
    ///         .partial(0, |args| Expr::number(2.0) * (*args[0]).clone()).expect("Should pass"));
    ///
    /// let x = symb("x");
    /// let expr = x.pow(2.0);
    /// let derivative = expr.derive("x", Some(&ctx));
    /// ```
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use).
    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "Comprehensive differentiation logic handles many expression types"
    )]
    pub fn derive(&self, var: &str, context: Option<&Context>) -> Self {
        // Use static empty context for None case to avoid allocation
        static EMPTY_CONTEXT: std::sync::OnceLock<Context> = std::sync::OnceLock::new();
        let ctx = context.unwrap_or_else(|| EMPTY_CONTEXT.get_or_init(Context::new));

        match &self.kind {
            // Base cases
            ExprKind::Number(_) => Self::number(0.0),

            ExprKind::Symbol(name) => {
                // d/dx(x) = 1, d/dx(y) = 0 (anything else is constant)
                if name.as_str() == var {
                    Self::number(1.0)
                } else {
                    Self::number(0.0)
                }
            }

            // Function call - check Context for user-defined partials
            ExprKind::FunctionCall { name, args } => {
                // Zero-argument functions don't exist in the registry (all functions have arity ≥ 1).
                // This handles edge cases where an empty FunctionCall might be constructed.
                // Treating as constant (derivative = 0) is the safest fallback.
                if args.is_empty() {
                    return Self::number(0.0);
                }

                // Special handling for exponential function e^x
                if name.id() == ks::KS.exp && args.len() == 1 {
                    let inner_deriv = args[0].derive(var, Some(ctx));
                    return Self::mul_expr(
                        Self::func_symbol(ks::get_symbol(ks::KS.exp), (*args[0]).clone()),
                        inner_deriv,
                    );
                }

                // Check Registry (built-in functions like sin, cos, etc.)
                if let Some(def) = crate::functions::registry::Registry::get_by_symbol(name)
                    && def.validate_arity(args.len()) {
                    let arg_primes: Vec<Self> =
                        args.iter().map(|arg| arg.derive(var, Some(ctx))).collect();
                    return (def.derivative)(args, &arg_primes);
                }

                // Check for user-defined function partials in context
                if let Some(user_fn) = ctx.get_user_fn(name.as_str()) {
                    // Multi-arg chain rule: dF/dx = Σ (∂F/∂arg[i]) * (darg[i]/dx)
                    let mut terms = Vec::new();

                    for (i, arg) in args.iter().enumerate() {
                        let arg_prime = arg.derive(var, Some(ctx));

                        // Skip if derivative is zero
                        if arg_prime.is_zero_num() {
                            continue;
                        }

                        // Try to get partial from user function, or use symbolic fallback
                        let partial = user_fn.partials.get(&i).map_or_else(
                            || Self::symbolic_partial(name, args, i),
                            |partial_fn| partial_fn(args),
                        );

                        terms.push(Self::mul_expr(partial, arg_prime));
                    }

                    return if terms.is_empty() {
                        Self::number(0.0)
                    } else if terms.len() == 1 {
                        terms.remove(0)
                    } else {
                        Self::sum(terms)
                    };
                }

                // Unknown function - symbolic partial derivative using chain rule
                let mut terms = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let arg_prime = arg.derive(var, Some(ctx));
                    if arg_prime.is_zero_num() {
                        continue;
                    }
                    let partial = Self::symbolic_partial(name, args, i);
                    terms.push(Self::mul_expr(partial, arg_prime));
                }

                if terms.is_empty() {
                    Self::number(0.0)
                } else if terms.len() == 1 {
                    terms.remove(0)
                } else {
                    Self::sum(terms)
                }
            }

            // Sum rule: (a + b + c + ...)' = a' + b' + c' + ...
            ExprKind::Sum(terms) => {
                let derivs: Vec<Self> = terms
                    .iter()
                    .map(|t| t.derive(var, Some(ctx)))
                    .filter(|d| !d.is_zero_num())
                    .collect();

                if derivs.is_empty() {
                    Self::number(0.0)
                } else if derivs.len() == 1 {
                    derivs
                        .into_iter()
                        .next()
                        .expect("derivs must have exactly one element")
                } else {
                    Self::sum(derivs)
                }
            }

            // Product rule for N-ary: (f1 * f2 * ... * fn)' = Σ (f1 * ... * fi' * ... * fn)
            ExprKind::Product(factors) => {
                if factors.is_empty() {
                    return Self::number(0.0);
                }
                if factors.len() == 1 {
                    return factors[0].derive(var, Some(ctx));
                }

                // Optimization: For large products, use logarithmic differentiation
                // (fgh)' = fgh * (f'/f + g'/g + h'/h)
                // This reduces node count from O(N²) to O(N)
                if factors.len() > 10 {
                    let mut log_terms = Vec::with_capacity(factors.len());
                    for factor in factors {
                        let prime = factor.derive(var, Some(ctx));
                        if !prime.is_zero_num() {
                            // term = f' / f
                            log_terms.push(Self::div_from_arcs(Arc::new(prime), Arc::clone(factor)));
                        }
                    }
                    
                    if log_terms.is_empty() {
                        return Self::number(0.0);
                    }
                    
                    // Result = self * sum(f'/f)
                    let sum_log = Self::sum(log_terms);
                    return Self::mul_expr(self.clone(), sum_log);
                }

                let mut terms = Vec::new();

                for i in 0..factors.len() {
                    // Fast-path: skip Number factors - their derivative is always 0
                    if matches!(factors[i].kind, ExprKind::Number(_)) {
                        continue;
                    }

                    let factor_prime = factors[i].derive(var, Some(ctx));

                    // Skip zero terms
                    if factor_prime.is_zero_num() {
                        continue;
                    }

                    // Build product of all other factors * this derivative
                    let other_factors: Vec<Arc<Self>> = factors
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, f)| Arc::clone(f))
                        .collect();

                    if other_factors.is_empty() {
                        terms.push(factor_prime);
                    } else if factor_prime.is_one_num() {
                        terms.push(Self::product_from_arcs(other_factors));
                    } else {
                        let mut all = other_factors;
                        all.push(Arc::new(factor_prime));
                        terms.push(Self::product_from_arcs(all));
                    }
                }

                if terms.is_empty() {
                    Self::number(0.0)
                } else if terms.len() == 1 {
                    terms.remove(0)
                } else {
                    Self::sum(terms)
                }
            }

            // Quotient rule: (u/v)' = (u'v - uv') / v²
            // Fast-paths to avoid unnecessary derivative computations
            ExprKind::Div(u, v) => {
                // Fast-path 1: Constant numerator (n/f) → -n*f'/f²
                // Skip computing u' since we know it's 0
                if let ExprKind::Number(n) = &u.kind {
                    let v_prime = v.derive(var, Some(ctx));
                    if v_prime.is_zero_num() {
                        return Self::number(0.0);
                    }
                    // -n * f' / f²
                    let neg_n_fprime = Self::product(vec![Self::number(-n), v_prime]);
                    let f_squared = Self::pow_from_arcs(Arc::clone(v), Arc::new(Self::number(2.0)));
                    return Self::div_expr(neg_n_fprime, f_squared);
                }

                // Fast-path 2: Constant denominator (u/n) → u'/n
                // Skip computing v' since we know it's 0
                if let ExprKind::Number(_) = &v.kind {
                    let u_prime = u.derive(var, Some(ctx));
                    return Self::div_from_arcs(Arc::new(u_prime), Arc::clone(v));
                }

                // General case: compute both derivatives
                let u_prime = u.derive(var, Some(ctx));
                let v_prime = v.derive(var, Some(ctx));

                let u_is_zero = u_prime.is_zero_num();
                let v_is_zero = v_prime.is_zero_num();

                if u_is_zero && v_is_zero {
                    Self::number(0.0)
                } else if v_is_zero {
                    // v is constant: (u'/v)
                    Self::div_from_arcs(Arc::new(u_prime), Arc::clone(v))
                } else if u_is_zero {
                    // u' is zero: (-u * v') / v²
                    let u_times_vprime =
                        Self::mul_from_arcs(vec![Arc::clone(u), Arc::new(v_prime)]);
                    let neg_u_vprime = u_times_vprime.negate();
                    let v_squared = Self::pow_from_arcs(Arc::clone(v), Arc::new(Self::number(2.0)));
                    Self::div_expr(neg_u_vprime, v_squared)
                } else {
                    // Full quotient rule: (u'v - uv') / v²
                    let u_prime_v = Self::mul_from_arcs(vec![Arc::new(u_prime), Arc::clone(v)]);
                    let u_v_prime = Self::mul_from_arcs(vec![Arc::clone(u), Arc::new(v_prime)]);
                    let numerator = Self::sub_expr(u_prime_v, u_v_prime);
                    let v_squared = Self::pow_from_arcs(Arc::clone(v), Arc::new(Self::number(2.0)));
                    Self::div_expr(numerator, v_squared)
                }
            }

            // Power rule with 3 cases:
            // 1. u^n (n constant): n * u^(n-1) * u'
            // 2. a^v (a constant): a^v * ln(a) * v'
            // 3. u^v (both variable): u^v * (v' * ln(u) + v * u'/u)
            //
            // Edge cases handled correctly:
            // - 0^n: u_prime is 0, returns 0 ✓
            // - 1^x: case 2, ln(1)=0 creates 0 term, simplifier handles ✓
            // - x^0: n_minus_1 becomes -1, but final term is 0*u^(-1)*u'=0 via simplifier ✓
            ExprKind::Pow(u, v) => {
                let u_contains_var = u.contains_var(var);
                let v_contains_var = v.contains_var(var);

                if !u_contains_var && !v_contains_var {
                    // Both constant
                    Self::number(0.0)
                } else if !v_contains_var {
                    // Case 1: u^n, n is constant

                    // Fast-path: x^0 = 1, derivative is 0
                    if v.as_number() == Some(0.0) {
                        return Self::number(0.0);
                    }

                    let u_prime = u.derive(var, Some(ctx));

                    if u_prime.is_zero_num() {
                        Self::number(0.0)
                    } else {
                        let n_minus_1 = Self::sub_expr((**v).clone(), Self::number(1.0));
                        let u_pow_n_minus_1 =
                            Self::pow_from_arcs(Arc::clone(u), Arc::new(n_minus_1));

                        if u_prime.is_one_num() {
                            Self::mul_expr((**v).clone(), u_pow_n_minus_1)
                        } else {
                            Self::product(vec![(**v).clone(), u_pow_n_minus_1, u_prime])
                        }
                    }
                } else if !u_contains_var {
                    // Case 2: a^v, base is constant
                    let v_prime = v.derive(var, Some(ctx));

                    if v_prime.is_zero_num() {
                        Self::number(0.0)
                    } else {
                        let a_pow_v = Self::pow_from_arcs(Arc::clone(u), Arc::clone(v));
                        let ln_a = Self::func_symbol(ks::get_symbol(ks::KS.ln), (**u).clone());

                        if v_prime.is_one_num() {
                            Self::mul_expr(a_pow_v, ln_a)
                        } else {
                            Self::product(vec![a_pow_v, ln_a, v_prime])
                        }
                    }
                } else {
                    // Case 3: u^v, both variable - use logarithmic differentiation
                    let u_prime = u.derive(var, Some(ctx));
                    let v_prime = v.derive(var, Some(ctx));

                    let ln_u = Self::func_symbol(ks::get_symbol(ks::KS.ln), (**u).clone());

                    // Term 1: v' * ln(u)
                    let term1 = if v_prime.is_zero_num() {
                        Self::number(0.0)
                    } else if v_prime.is_one_num() {
                        ln_u
                    } else {
                        Self::mul_expr(v_prime, ln_u)
                    };

                    // Term 2: v * u'/u
                    let term2 = if u_prime.is_zero_num() {
                        Self::number(0.0)
                    } else {
                        let u_prime_over_u = Self::div_from_arcs(Arc::new(u_prime), Arc::clone(u));
                        Self::mul_from_arcs(vec![Arc::clone(v), Arc::new(u_prime_over_u)])
                    };

                    // Sum: v' * ln(u) + v * u'/u
                    let sum = if term1.is_zero_num() && term2.is_zero_num() {
                        return Self::number(0.0);
                    } else if term1.is_zero_num() {
                        term2
                    } else if term2.is_zero_num() {
                        term1
                    } else {
                        Self::add_expr(term1, term2)
                    };

                    // Multiply by u^v
                    if sum.is_zero_num() {
                        Self::number(0.0)
                    } else {
                        let u_pow_v = Self::pow_from_arcs(Arc::clone(u), Arc::clone(v));
                        if sum.is_one_num() {
                            u_pow_v
                        } else {
                            Self::mul_expr(u_pow_v, sum)
                        }
                    }
                }
            }

            // Derivative expressions: d/dx (∂^n f / ∂x^n) = ∂^(n+1) f / ∂x^(n+1)
            ExprKind::Derivative {
                inner,
                var: deriv_var,
                order,
            } => {
                if deriv_var.as_str() == var {
                    Self::derivative_interned(inner.as_ref().clone(), deriv_var.clone(), order + 1)
                } else if !inner.contains_var(var) {
                    Self::number(0.0)
                } else {
                    Self::derivative(
                        Self::new(ExprKind::Derivative {
                            inner: Arc::clone(inner),
                            var: deriv_var.clone(),
                            order: *order,
                        }),
                        var,
                        1,
                    )
                }
            }

            // Polynomial
            ExprKind::Poly(poly) => poly.derivative_expr(var),
        }
    }

    /// Create symbolic partial derivative for unknown function
    ///
    /// This helper creates: ∂f/∂argᵢ where f is the function and i is the argument index.
    /// Used when no explicit partial derivative is registered for a user function.
    #[inline]
    fn symbolic_partial(
        name: &crate::core::symbol::InternedSymbol,
        args: &[Arc<Self>],
        arg_index: usize,
    ) -> Self {
        let inner_func = Self::func_multi_from_arcs_symbol(name.clone(), args.to_vec());
        Self::derivative(inner_func, format!("arg{arg_index}"), 1)
    }

    /// Raw differentiation without simplification (for benchmarks)
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{symb, Expr};
    /// let x = symb("derive_raw_doc_x");
    /// let expr = x.pow(2.0);
    /// let derivative = expr.derive_raw("derive_raw_doc_x");
    /// ```
    #[inline]
    #[must_use]
    pub fn derive_raw(&self, var: &str) -> Self {
        self.derive(var, None)
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding,
    reason = "Standard test relaxations"
)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_sinh() {
        let expr = Expr::func_symbol(ks::get_symbol(ks::KS.sinh), Expr::symbol("x"));
        let result = expr.derive("x", None);
        match &result.kind {
            ExprKind::FunctionCall { name, .. } => assert_eq!(name.id(), ks::KS.cosh),
            ExprKind::Product(factors) => {
                let has_cosh = factors.iter().any(
                    |f| matches!(&f.kind, ExprKind::FunctionCall { name, .. } if name.id() == ks::KS.cosh),
                );
                assert!(has_cosh, "Expected cosh in Product, got {factors:?}");
            }
            _ => panic!("Expected cosh or Product with cosh, got {result:?}"),
        }
    }

    #[test]
    fn test_custom_derivative_fallback() {
        let x = Expr::symbol("x");
        let expr = Expr::func("custom", x);
        let result = expr.derive("x", None);
        assert!(
            !matches!(result.kind, ExprKind::Number(_)),
            "Expected non-constant derivative, got: {:?}",
            result.kind
        );
    }

    #[test]
    fn test_derive_subtraction() {
        let expr = Expr::sub_expr(Expr::symbol("x"), Expr::number(1.0));
        let result = expr.derive("x", None);
        assert!(matches!(result.kind, ExprKind::Number(n) if n == 1.0));
    }

    #[test]
    fn test_derive_division() {
        let expr = Expr::div_expr(Expr::symbol("x"), Expr::number(2.0));
        let result = expr.derive("x", None);
        assert!(matches!(result.kind, ExprKind::Div(_, _)));
    }

    #[test]
    fn test_logarithmic_differentiation() {
        let expr = Expr::pow_static(Expr::symbol("x"), Expr::symbol("x"));
        let result = expr.derive("x", None);
        assert!(matches!(result.kind, ExprKind::Product(_)));
    }

    #[test]
    fn test_derivative_order_increment() {
        let inner_func = Expr::func("f", Expr::symbol("x"));
        let derivative_expr = Expr::derivative(inner_func, "x", 1);
        let result = derivative_expr.derive("x", None);
        match &result.kind {
            ExprKind::Derivative { order, var, .. } => {
                assert_eq!(*order, 2);
                assert_eq!(var.as_str(), "x");
            }
            _ => panic!("Expected Derivative, got {result:?}"),
        }
    }

    #[test]
    fn test_derivative_order_increment_multi_digit() {
        let inner_func = Expr::func("f", Expr::symbol("x"));
        let ninth_deriv = Expr::derivative(inner_func.clone(), "x", 9);
        let result = ninth_deriv.derive("x", None);
        match result.kind {
            ExprKind::Derivative { order, .. } => assert_eq!(order, 10),
            _ => panic!("Expected Derivative, got {result:?}"),
        }

        let ninety_ninth_deriv = Expr::derivative(inner_func, "x", 99);
        let result2 = ninety_ninth_deriv.derive("x", None);
        match result2.kind {
            ExprKind::Derivative { order, .. } => assert_eq!(order, 100),
            _ => panic!("Expected Derivative, got {result2:?}"),
        }
    }

    #[test]
    fn test_derivative_variable_not_present_returns_zero() {
        let inner_func = Expr::func("f", Expr::symbol("x"));
        let derivative_expr = Expr::derivative(inner_func, "x", 1);
        let result = derivative_expr.derive("y", None);
        assert_eq!(result.as_number(), Some(0.0));
    }

    #[test]
    fn test_derivative_mixed_partial_when_variable_present() {
        let inner_func = Expr::func_multi("f", vec![Expr::symbol("x"), Expr::symbol("y")]);
        let derivative_expr = Expr::derivative(inner_func, "x", 1);
        let result = derivative_expr.derive("y", None);

        match &result.kind {
            ExprKind::Derivative { var, order, inner } => {
                assert_eq!(var.as_str(), "y");
                assert_eq!(*order, 1);
                match &inner.kind {
                    ExprKind::Derivative {
                        var: inner_var,
                        order: inner_order,
                        ..
                    } => {
                        assert_eq!(inner_var.as_str(), "x");
                        assert_eq!(*inner_order, 1);
                    }
                    _ => panic!("Expected inner Derivative, got {inner:?}"),
                }
            }
            _ => panic!("Expected Derivative for mixed partial, got {result:?}"),
        }
    }

    #[test]
    fn test_derivative_multivar_function() {
        let inner_func = Expr::func_multi("f", vec![Expr::symbol("x"), Expr::symbol("y")]);
        let partial_deriv = Expr::derivative(inner_func, "x", 1);
        let result = partial_deriv.derive("x", None);
        match &result.kind {
            ExprKind::Derivative { order, var, .. } => {
                assert_eq!(*order, 2);
                assert_eq!(var.as_str(), "x");
            }
            _ => panic!("Expected Derivative, got {result:?}"),
        }
    }

    #[test]
    fn test_mixed_partial_display() {
        let f = Expr::func_multi("f", vec![Expr::symbol("x"), Expr::symbol("y")]);
        let df_dx = Expr::derivative(f, "x", 1);
        let d2f_dxdy = df_dx.derive("y", None);
        let display = format!("{d2f_dxdy}");
        assert!(
            display.contains("\u{2202}^"),
            "Display should contain derivative notation, got: {display}"
        );
    }

    #[test]
    fn test_deeply_nested_derivatives() {
        let f_xyz = Expr::func_multi(
            "f",
            vec![Expr::symbol("x"), Expr::symbol("y"), Expr::symbol("z")],
        );
        let deriv_x = Expr::derivative(f_xyz.clone(), "x", 1);
        let deriv_xy = deriv_x.derive("y", None);
        let deriv_xyz = deriv_xy.derive("z", None);

        match &deriv_xyz.kind {
            ExprKind::Derivative { var, .. } => {
                assert_eq!(var.as_str(), "z");
            }
            _ => panic!("Expected Derivative for triple mixed partial, got {deriv_xyz:?}"),
        }

        let deriv_w = f_xyz.derive("w", None);
        assert_eq!(deriv_w.as_number(), Some(0.0));

        let deriv_w_x = deriv_x.derive("w", None);
        assert_eq!(deriv_w_x.as_number(), Some(0.0));
    }

    #[test]
    fn test_derive_erfc() {
        let expr = Expr::func_symbol(ks::get_symbol(ks::KS.erfc), Expr::symbol("x"));
        let result = expr.derive("x", None);
        let s = format!("{result}");
        assert!(s.contains("exp"), "Result should contain exp: {s}");
        assert!(s.contains("pi"), "Result should contain pi: {s}");
        assert!(s.contains("-2"), "Result should contain -2: {s}");
    }

    #[test]
    fn test_empty_product_derivative() {
        // Edge case: Product with no factors should return 0
        use crate::core::expr::ExprKind as AstKind;
        let empty_prod = Expr::new(AstKind::Product(vec![]));
        let result = empty_prod.derive("x", None);
        assert_eq!(result.as_number(), Some(0.0));
    }

    #[test]
    fn test_empty_sum_derivative() {
        // Edge case: Sum with no terms should return 0
        use crate::core::expr::ExprKind as AstKind;
        let empty_sum = Expr::new(AstKind::Sum(vec![]));
        let result = empty_sum.derive("x", None);
        assert_eq!(result.as_number(), Some(0.0));
    }

    #[test]
    fn test_power_rule_edge_cases() {
        // Test edge cases mentioned in power rule comments

        // 0^2: should be 0
        let zero_pow_two = Expr::pow_static(Expr::number(0.0), Expr::number(2.0));
        let result = zero_pow_two.derive("x", None);
        assert_eq!(result.as_number(), Some(0.0));

        // x^0: should be 0 (derivative of 1)
        let x_pow_zero = Expr::pow_static(Expr::symbol("x"), Expr::number(0.0));
        let result3 = x_pow_zero.derive("x", None);
        // Result might be 0*x^(-1) which simplifies to 0
        let simplified = result3.to_string();
        // After simplification, should effectively be 0
        assert!(
            simplified.contains('0') || result3.as_number() == Some(0.0),
            "x^0 derivative should simplify to 0, got: {simplified}"
        );
    }
}
