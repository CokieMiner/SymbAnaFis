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

use crate::core::known_symbols::{EXP, LN, get_symbol};
use crate::{CustomDerivativeFn, Expr, ExprKind};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

impl Expr {
    /// Differentiate this expression with respect to a variable
    ///
    /// # Arguments
    /// * `var` - Variable to differentiate with respect to
    /// * `fixed_vars` - Set of variable names that are constants
    /// * `custom_derivatives` - Map of custom derivative functions (single-arg)
    /// * `custom_fns` - Map of multi-arg custom functions with partial derivatives
    pub(crate) fn derive(
        &self,
        var: &str,
        fixed_vars: &HashSet<String>,
        custom_derivatives: &HashMap<String, CustomDerivativeFn>,
        custom_fns: &HashMap<String, crate::api::builder::CustomFn>,
    ) -> Expr {
        match &self.kind {
            // Base cases
            ExprKind::Number(_) => Expr::number(0.0),

            ExprKind::Symbol(name) => {
                // Standard symbol differentiation
                if name.as_str() == var && !fixed_vars.contains(name.as_ref()) {
                    Expr::number(1.0)
                } else {
                    Expr::number(0.0)
                }
            }

            // Function call
            ExprKind::FunctionCall { name, args } => {
                if args.is_empty() {
                    return Expr::number(0.0);
                }

                // Check for custom derivative first (single-arg functions)
                if let Some(custom_rule) = custom_derivatives.get(name.as_str())
                    && args.len() == 1
                {
                    let inner = &args[0];
                    let inner_prime = inner.derive(var, fixed_vars, custom_derivatives, custom_fns);
                    return custom_rule(inner, var, &inner_prime);
                }

                // Special handling for exponential function e^x
                // d/dx(e^x) = e^x * d/dx(x)
                if name.id() == *EXP && args.len() == 1 {
                    let inner_deriv =
                        args[0].derive(var, fixed_vars, custom_derivatives, custom_fns);
                    return Expr::product(vec![
                        Expr::func_symbol(get_symbol(&EXP), (*args[0]).clone()),
                        inner_deriv,
                    ]);
                }

                // Check Registry (built-in functions like sin, cos, etc.)
                if let Some(def) = crate::functions::registry::Registry::get_by_symbol(name)
                    && def.validate_arity(args.len())
                {
                    let arg_primes: Vec<Expr> = args
                        .iter()
                        .map(|arg| arg.derive(var, fixed_vars, custom_derivatives, custom_fns))
                        .collect();
                    return (def.derivative)(args, &arg_primes);
                }

                // Multi-arg chain rule: dF/dx = Σ (∂F/∂arg[i]) * (darg[i]/dx)
                // Clone args only ONCE here (lazy - only if we need symbolic partials)
                let mut args_cloned: Option<Vec<Expr>> = None;
                let get_args_cloned =
                    |cache: &mut Option<Vec<Expr>>, args: &[Arc<Expr>]| -> Vec<Expr> {
                        cache
                            .get_or_insert_with(|| args.iter().map(|a| (**a).clone()).collect())
                            .clone()
                    };

                let custom_fn = custom_fns.get(name.as_str());
                let mut terms = Vec::new();

                for (i, arg) in args.iter().enumerate() {
                    let arg_prime = arg.derive(var, fixed_vars, custom_derivatives, custom_fns);

                    // Skip if derivative is zero
                    if arg_prime.is_zero_num() {
                        continue;
                    }

                    // Try to get partial from custom_fn, otherwise create symbolic
                    let partial = if let Some(cf) = custom_fn {
                        if let Some(partial_fn) = cf.partials.get(&i) {
                            // Pass args directly - they're already &[Arc<Expr>]
                            partial_fn(args)
                        } else {
                            // No partial registered - symbolic
                            let args_vec = get_args_cloned(&mut args_cloned, args);
                            let inner_func = Expr::func_multi(name.clone(), args_vec);
                            Expr::derivative(inner_func, format!("arg{}", i), 1)
                        }
                    } else {
                        // Fallback: unknown function - symbolic partial
                        let args_vec = get_args_cloned(&mut args_cloned, args);
                        let inner_func = Expr::func_multi(name.clone(), args_vec);
                        Expr::derivative(inner_func, var.to_string(), 1)
                    };

                    terms.push(Expr::mul_expr(partial, arg_prime));
                }

                if terms.is_empty() {
                    Expr::number(0.0)
                } else if terms.len() == 1 {
                    terms.remove(0)
                } else {
                    Expr::sum(terms)
                }
            }

            // Sum rule: (a + b + c + ...)' = a' + b' + c' + ...
            ExprKind::Sum(terms) => {
                let derivatives: Vec<Expr> = terms
                    .iter()
                    .map(|t| t.derive(var, fixed_vars, custom_derivatives, custom_fns))
                    .filter(|d| !d.is_zero_num())
                    .collect();

                if derivatives.is_empty() {
                    Expr::number(0.0)
                } else if derivatives.len() == 1 {
                    derivatives.into_iter().next().unwrap()
                } else {
                    Expr::sum(derivatives)
                }
            }

            // N-ary Product rule: (a * b * c)' = a' * b * c + a * b' * c + a * b * c'
            ExprKind::Product(factors) => {
                let n = factors.len();
                let mut result_terms: Vec<Arc<Expr>> = Vec::new();

                // For each factor, compute: (product of all others) * factor'
                for i in 0..n {
                    let factor_prime =
                        factors[i].derive(var, fixed_vars, custom_derivatives, custom_fns);

                    // Skip if derivative is zero
                    if factor_prime.is_zero_num() {
                        continue;
                    }

                    // Collect all other factors using Arc::clone (just refcount increment)
                    let other_factors: Vec<Arc<Expr>> = factors
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, f)| Arc::clone(f))
                        .collect();

                    if other_factors.is_empty() {
                        // Only one factor: derivative is just factor_prime
                        result_terms.push(Arc::new(factor_prime));
                    } else if factor_prime.is_one_num() {
                        // factor' is 1: just use product of other factors
                        result_terms.push(Arc::new(Expr::product_from_arcs(other_factors)));
                    } else {
                        // General case: other_factors * factor_prime
                        let mut all_factors = other_factors;
                        all_factors.push(Arc::new(factor_prime));
                        result_terms.push(Arc::new(Expr::product_from_arcs(all_factors)));
                    }
                }

                if result_terms.is_empty() {
                    Expr::number(0.0)
                } else if result_terms.len() == 1 {
                    Arc::try_unwrap(result_terms.into_iter().next().unwrap())
                        .unwrap_or_else(|arc| (*arc).clone())
                } else {
                    Expr::sum_from_arcs(result_terms)
                }
            }

            // Quotient rule: (u / v)' = (u' * v - u * v') / v^2
            ExprKind::Div(u, v) => {
                let u_prime = u.derive(var, fixed_vars, custom_derivatives, custom_fns);
                let v_prime = v.derive(var, fixed_vars, custom_derivatives, custom_fns);

                // Fast path: both derivatives are 0
                if u_prime.is_zero_num() && v_prime.is_zero_num() {
                    return Expr::number(0.0);
                }

                // Compute numerator based on which derivatives are non-zero
                let numerator = match (u_prime.is_zero_num(), v_prime.is_zero_num()) {
                    (true, true) => return Expr::number(0.0), // Already handled above
                    (true, false) => {
                        // Numerator = -u * v'
                        Expr::product_from_arcs(vec![
                            Arc::new(Expr::number(-1.0)),
                            Arc::clone(u),
                            Arc::new(v_prime),
                        ])
                    }
                    (false, true) => {
                        // Numerator = u' * v
                        if u_prime.is_one_num() {
                            Expr::unwrap_arc(Arc::clone(v))
                        } else if v.is_one_num() {
                            u_prime
                        } else {
                            Expr::product_from_arcs(vec![Arc::clone(v), Arc::new(u_prime)])
                        }
                    }
                    (false, false) => {
                        // Numerator = u' * v - u * v'
                        let term1 = if u_prime.is_one_num() {
                            Expr::unwrap_arc(Arc::clone(v))
                        } else if v.is_one_num() {
                            u_prime
                        } else {
                            Expr::product_from_arcs(vec![Arc::clone(v), Arc::new(u_prime)])
                        };
                        let term2 = Expr::product_from_arcs(vec![Arc::clone(u), Arc::new(v_prime)]);
                        Expr::sub_expr(term1, term2)
                    }
                };

                // Apply denominator v^2
                if numerator.is_zero_num() {
                    Expr::number(0.0)
                } else if v.is_one_num() {
                    numerator
                } else {
                    let denominator =
                        Expr::pow_from_arcs(Arc::clone(v), Arc::new(Expr::number(2.0)));
                    Expr::div_expr(numerator, denominator)
                }
            }

            // Power rule with LOGARITHMIC DIFFERENTIATION for variable exponents
            ExprKind::Pow(u, v) => {
                // Check if exponent is constant (contains no variables)
                if !v.has_free_variables(fixed_vars) {
                    // Constant exponent - use standard power rule
                    // (u^n)' = n * u^(n-1) * u'
                    let u_prime = u.derive(var, fixed_vars, custom_derivatives, custom_fns);

                    // If u' is 0, result is 0
                    if u_prime.is_zero_num() {
                        Expr::number(0.0)
                    } else {
                        // Use Arc::clone to avoid full Expr cloning
                        let n = Expr::unwrap_arc(Arc::clone(v));
                        if let Some(n_val) = n.as_number() {
                            if n_val == 0.0 {
                                // (u^0)' = 0
                                Expr::number(0.0)
                            } else if n_val == 1.0 {
                                // (u^1)' = u'
                                u_prime
                            } else {
                                let n_minus_1 = Expr::number(n_val - 1.0);
                                let u_pow_n_minus_1 = if u.is_one_num() {
                                    Expr::number(1.0)
                                } else if u.is_zero_num() {
                                    Expr::number(0.0)
                                } else {
                                    Expr::pow_from_arcs(Arc::clone(u), Arc::new(n_minus_1.clone()))
                                };

                                if u_prime.is_one_num() {
                                    Expr::mul_expr(n, u_pow_n_minus_1)
                                } else {
                                    Expr::product_from_arcs(vec![
                                        Arc::new(n),
                                        Arc::new(u_pow_n_minus_1),
                                        Arc::new(u_prime),
                                    ])
                                }
                            }
                        } else {
                            // Non-numeric constant exponent: n * u^(n-1) * u'
                            let n_minus_1 =
                                Expr::sub_expr(Expr::unwrap_arc(Arc::clone(v)), Expr::number(1.0));
                            let u_pow_n_minus_1 =
                                Expr::pow_from_arcs(Arc::clone(u), Arc::new(n_minus_1));

                            if u_prime.is_one_num() {
                                Expr::product_from_arcs(vec![
                                    Arc::clone(v),
                                    Arc::new(u_pow_n_minus_1),
                                ])
                            } else {
                                Expr::product_from_arcs(vec![
                                    Arc::clone(v),
                                    Arc::new(u_pow_n_minus_1),
                                    Arc::new(u_prime),
                                ])
                            }
                        }
                    }
                } else {
                    // Variable exponent - use LOGARITHMIC DIFFERENTIATION!
                    // d/dx[u^v] = u^v * (v' * ln(u) + v * u'/u)
                    let u_prime = u.derive(var, fixed_vars, custom_derivatives, custom_fns);
                    let v_prime = v.derive(var, fixed_vars, custom_derivatives, custom_fns);

                    // If both u' and v' are 0, result is 0
                    if u_prime.is_zero_num() && v_prime.is_zero_num() {
                        Expr::number(0.0)
                    } else {
                        // Term 1: v' * ln(u)
                        let ln_u = if matches!(&u.kind, ExprKind::Symbol(name) if name.id() == *EXP)
                            && !fixed_vars.contains("e")
                        {
                            Expr::number(1.0) // ln(e) = 1
                        } else if u.is_one_num() {
                            Expr::number(0.0) // ln(1) = 0
                        } else {
                            Expr::func_symbol(get_symbol(&LN), Expr::unwrap_arc(Arc::clone(u)))
                        };
                        let term1 = if v_prime.is_zero_num() || ln_u.is_zero_num() {
                            Expr::number(0.0)
                        } else if ln_u.is_one_num() {
                            v_prime // Move - only used in one branch
                        } else if v_prime.is_one_num() {
                            ln_u
                        } else {
                            Expr::mul_expr(v_prime, ln_u) // Move - last use
                        };

                        // Term 2: v * (u'/u)
                        let u_over_u_prime = if u_prime.is_zero_num() {
                            Expr::number(0.0)
                        } else if u.is_one_num() {
                            u_prime // Move instead of clone since only used here
                        } else if u_prime.is_one_num() {
                            // 1/u
                            Expr::pow_from_arcs(Arc::clone(u), Arc::new(Expr::number(-1.0)))
                        } else {
                            Expr::div_from_arcs(Arc::new(u_prime.clone()), Arc::clone(u))
                        };
                        let term2 = if u_over_u_prime.is_zero_num() {
                            Expr::number(0.0)
                        } else if v.is_one_num() {
                            u_over_u_prime
                        } else if u_over_u_prime.is_one_num() {
                            Expr::unwrap_arc(Arc::clone(v))
                        } else {
                            Expr::product_from_arcs(vec![Arc::clone(v), Arc::new(u_over_u_prime)])
                        };

                        // Sum of terms
                        let sum = if term1.is_zero_num() {
                            term2
                        } else if term2.is_zero_num() {
                            term1
                        } else {
                            Expr::add_expr(term1, term2)
                        };

                        // Multiply by u^v
                        if sum.is_zero_num() {
                            Expr::number(0.0)
                        } else {
                            let u_pow_v = Expr::pow_from_arcs(Arc::clone(u), Arc::clone(v));
                            if sum.is_one_num() {
                                u_pow_v
                            } else {
                                Expr::mul_expr(u_pow_v, sum)
                            }
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
                if deriv_var == var {
                    // Differentiating with respect to the same variable: increment order
                    // ∂/∂x (∂^n f/∂x^n) = ∂^(n+1) f/∂x^(n+1)
                    Expr::derivative(inner.as_ref().clone(), deriv_var.clone(), order + 1)
                } else {
                    // Different variable: check if inner expression contains the variable
                    // If f(x,y) doesn't contain z, then ∂f(x,y)/∂z = 0
                    if !inner.contains_var(var) {
                        Expr::number(0.0)
                    } else {
                        // Inner contains var: create mixed partial derivative
                        // ∂/∂y (∂f(x,y)/∂x) = ∂²f/∂x∂y (represented as nested derivatives)
                        Expr::derivative(
                            Expr::new(ExprKind::Derivative {
                                inner: inner.clone(),
                                var: deriv_var.clone(),
                                order: *order,
                            }),
                            var.to_string(),
                            1,
                        )
                    }
                }
            }

            // Polynomial: use fast polynomial derivative with chain rule!
            ExprKind::Poly(poly) => {
                // Use derivative_expr which handles chain rule for the base
                if let Some(result) = poly.derivative_expr(var) {
                    result
                } else {
                    Expr::number(0.0)
                }
            }
        }
    }

    /// Raw differentiation without simplification (for benchmarks)
    ///
    /// This exposes the internal derive() method for performance benchmarking.
    /// For normal usage, use `Diff::new().differentiate()` which includes simplification.
    ///
    /// # Arguments
    /// * `var` - Variable to differentiate with respect to
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{symb, Expr};
    /// let x = symb("derive_raw_doc_x");
    /// let expr = x.pow(2.0);
    /// let derivative = expr.derive_raw("derive_raw_doc_x");
    /// // Returns unsimplified derivative
    /// ```
    #[inline]
    pub fn derive_raw(&self, var: &str) -> Expr {
        self.derive(var, &HashSet::new(), &HashMap::new(), &HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_sinh() {
        let expr = Expr::func("sinh", Expr::symbol("x"));
        let result = expr.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
        // Result should be cosh(x) * 1 (unoptimized) or cosh(x) (optimized)
        match result.kind {
            ExprKind::FunctionCall { name, .. } => assert_eq!(name.as_str(), "cosh"),
            ExprKind::Product(factors) => {
                // Check that one of the factors is cosh
                let has_cosh = factors.iter().any(
                    |f| matches!(&f.kind, ExprKind::FunctionCall { name, .. } if name.as_str() == "cosh"),
                );
                assert!(has_cosh, "Expected cosh in Product, got {:?}", factors);
            }
            _ => panic!("Expected cosh or Product with cosh, got {:?}", result),
        }
    }

    #[test]
    fn test_custom_derivative_fallback() {
        let x = Expr::symbol("x");
        let expr = Expr::func("custom", x);
        // Should return symbolic derivative if no rule
        let result = expr.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
        println!("Result kind: {:?}", result.kind);
        println!("Result: {}", result);
        // Returns Mul(partial, arg_prime) or Sum (from chain rule) or other expression types
        assert!(
            !matches!(result.kind, ExprKind::Number(_)),
            "Expected non-constant derivative, got: {:?}",
            result.kind
        );
    }

    #[test]
    fn test_derive_subtraction() {
        // (x - 1)' = 1 - 0 = 1
        let expr = Expr::sub_expr(Expr::symbol("x"), Expr::number(1.0));
        let result = expr.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
        assert!(matches!(result.kind, ExprKind::Number(n) if n == 1.0));
    }

    #[test]
    fn test_derive_division() {
        // (x / 2)' = (1*2 - x*0) / 2^2
        let expr = Expr::div_expr(Expr::symbol("x"), Expr::number(2.0));
        let result = expr.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
        assert!(matches!(result.kind, ExprKind::Div(_, _)));
    }

    #[test]
    fn test_logarithmic_differentiation() {
        // x^x should use logarithmic differentiation
        let expr = Expr::pow(Expr::symbol("x"), Expr::symbol("x"));
        let result = expr.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
        // Result should be multiplication (complex expression)
        assert!(matches!(result.kind, ExprKind::Product(_)));
    }

    #[test]
    fn test_derivative_order_increment() {
        // Test that differentiating a derivative expression increments the order
        // Create ∂^1_f(x)/∂_x^1 using the new AST-based approach
        let inner_func = Expr::func("f", Expr::symbol("x"));
        let derivative_expr = Expr::derivative(inner_func.clone(), "x".to_string(), 1);

        let result = derivative_expr.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
        match result.kind {
            ExprKind::Derivative { order, var, .. } => {
                assert_eq!(order, 2, "Expected order 2, got {}", order);
                assert_eq!(var, "x");
            }
            _ => panic!("Expected Derivative, got {:?}", result),
        }
    }

    #[test]
    fn test_derivative_order_increment_multi_digit() {
        // Test incrementing from 9 to 10 (single to double digit)
        let inner_func = Expr::func("f", Expr::symbol("x"));
        let ninth_deriv = Expr::derivative(inner_func.clone(), "x".to_string(), 9);
        let result = ninth_deriv.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
        match result.kind {
            ExprKind::Derivative { order, .. } => assert_eq!(order, 10),
            _ => panic!("Expected Derivative, got {:?}", result),
        }

        // Test incrementing from 99 to 100 (double to triple digit)
        let ninety_ninth_deriv = Expr::derivative(inner_func.clone(), "x".to_string(), 99);
        let result =
            ninety_ninth_deriv.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
        match result.kind {
            ExprKind::Derivative { order, .. } => assert_eq!(order, 100),
            _ => panic!("Expected Derivative, got {:?}", result),
        }
    }

    #[test]
    fn test_derivative_variable_not_present_returns_zero() {
        // If f(x) doesn't contain y, then ∂f(x)/∂y = 0
        // So ∂/∂y (∂f(x)/∂x) = 0 as well
        let inner_func = Expr::func("f", Expr::symbol("x"));
        let derivative_expr = Expr::derivative(inner_func, "x".to_string(), 1);

        let result = derivative_expr.derive("y", &HashSet::new(), &HashMap::new(), &HashMap::new());
        assert_eq!(
            result.as_number(),
            Some(0.0),
            "Expected 0 when variable not present in inner"
        );
    }

    #[test]
    fn test_derivative_mixed_partial_when_variable_present() {
        // f(x, y) contains both x and y
        // ∂/∂y (∂f(x,y)/∂x) = ∂²f(x,y)/∂x∂y
        let inner_func = Expr::func_multi("f", vec![Expr::symbol("x"), Expr::symbol("y")]);
        let derivative_expr = Expr::derivative(inner_func, "x".to_string(), 1);

        let result = derivative_expr.derive("y", &HashSet::new(), &HashMap::new(), &HashMap::new());

        // Result should be a nested Derivative representing ∂²f/∂x∂y
        match &result.kind {
            ExprKind::Derivative { var, order, inner } => {
                assert_eq!(var, "y", "Outer derivative should be wrt y");
                assert_eq!(*order, 1, "Outer derivative should have order 1");
                // Inner should be ∂f(x,y)/∂x
                match &inner.kind {
                    ExprKind::Derivative {
                        var: inner_var,
                        order: inner_order,
                        ..
                    } => {
                        assert_eq!(inner_var, "x", "Inner derivative should be wrt x");
                        assert_eq!(*inner_order, 1, "Inner derivative should have order 1");
                    }
                    _ => panic!("Expected inner Derivative, got {:?}", inner),
                }
            }
            _ => panic!("Expected Derivative for mixed partial, got {:?}", result),
        }
    }

    #[test]
    fn test_derivative_multivar_function() {
        // Test differentiating f(x, y) - creates partial derivative, then incrementing
        let inner_func = Expr::func_multi("f", vec![Expr::symbol("x"), Expr::symbol("y")]);
        let partial_deriv = Expr::derivative(inner_func.clone(), "x".to_string(), 1);

        // Differentiating ∂f(x,y)/∂x with respect to x should give ∂²f(x,y)/∂x²
        let result = partial_deriv.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());
        match result.kind {
            ExprKind::Derivative { order, var, .. } => {
                assert_eq!(order, 2);
                assert_eq!(var, "x");
            }
            _ => panic!("Expected Derivative, got {:?}", result),
        }
    }

    #[test]
    fn test_mixed_partial_display() {
        // Test that mixed partials display correctly
        // Use f(x, y) so that differentiating wrt y doesn't return 0
        let f = Expr::func_multi("f", vec![Expr::symbol("x"), Expr::symbol("y")]);
        let df_dx = Expr::derivative(f, "x".to_string(), 1);
        let d2f_dxdy = df_dx.derive("y", &HashSet::new(), &HashMap::new(), &HashMap::new());

        let display = format!("{}", d2f_dxdy);

        println!("Display: {}", display);
        // Should show nested derivative notation
        assert!(
            display.contains("∂^"),
            "Display should contain derivative notation, got: {}",
            display
        );
    }

    #[test]
    fn test_deeply_nested_derivatives() {
        // Test triple derivative: ∂³f(x,y,z)/∂x∂y∂z
        let f_xyz = Expr::func_multi(
            "f",
            vec![Expr::symbol("x"), Expr::symbol("y"), Expr::symbol("z")],
        );

        // First: ∂f/∂x
        let df_dx = Expr::derivative(f_xyz.clone(), "x".to_string(), 1);
        // Second: ∂²f/∂x∂y
        let d2f_dxdy = df_dx.derive("y", &HashSet::new(), &HashMap::new(), &HashMap::new());
        // Third: ∂³f/∂x∂y∂z
        let d3f_dxdydz = d2f_dxdy.derive("z", &HashSet::new(), &HashMap::new(), &HashMap::new());

        // Should be a nested Derivative structure
        match &d3f_dxdydz.kind {
            ExprKind::Derivative { var, .. } => {
                assert_eq!(var, "z", "Outermost derivative should be wrt z");
            }
            _ => panic!(
                "Expected Derivative for triple mixed partial, got {:?}",
                d3f_dxdydz
            ),
        }

        // f(x,y,z) differentiated wrt w (not present) should return 0
        let df_dw = f_xyz.derive("w", &HashSet::new(), &HashMap::new(), &HashMap::new());
        assert_eq!(df_dw.as_number(), Some(0.0));

        // Nested derivative wrt w (not present) should also return 0
        let d2f_dxdw = df_dx.derive("w", &HashSet::new(), &HashMap::new(), &HashMap::new());
        assert_eq!(
            d2f_dxdw.as_number(),
            Some(0.0),
            "∂²f(x,y,z)/∂x∂w should be 0"
        );
    }
    #[test]
    fn test_derive_erfc() {
        // d/dx erfc(x) = -2/sqrt(pi) * exp(-x^2)
        let expr = Expr::func("erfc", Expr::symbol("x"));
        let result = expr.derive("x", &HashSet::new(), &HashMap::new(), &HashMap::new());

        // We expect a specific structure: multiplication of constant/pi term and exp term
        // The exact AST might vary based on simplifications, but we can check key properties
        let s = format!("{}", result);
        // Should contain exp, pi, and negative sign
        assert!(s.contains("exp"), "Result should contain exp: {}", s);
        assert!(s.contains("pi"), "Result should contain pi: {}", s);
        assert!(s.contains("-2"), "Result should contain -2: {}", s);
    }
}
