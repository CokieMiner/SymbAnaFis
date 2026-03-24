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

use crate::core::context::Context;
use crate::core::known_symbols as ks;
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
        static EMPTY_CONTEXT: std::sync::OnceLock<Context> = std::sync::OnceLock::new();
        let ctx = context.unwrap_or_else(|| EMPTY_CONTEXT.get_or_init(Context::new));

        let var_id = crate::core::symbol::symb_interned(var).id();

        self.derive_impl(var, var_id, ctx)
    }

    /// Inner recursive implementation that carries pre-computed `var_id`
    /// to avoid re-interning the variable name at each node.
    #[allow(
        clippy::too_many_lines,
        reason = "Comprehensive differentiation logic handles many expression types"
    )]
    fn derive_impl(&self, var: &str, var_id: u64, ctx: &Context) -> Self {
        match &self.kind {
            ExprKind::Number(_) => Self::number(0.0),

            ExprKind::Symbol(name) => {
                if name.id() == var_id {
                    Self::number(1.0)
                } else {
                    Self::number(0.0)
                }
            }

            ExprKind::FunctionCall { name, args } => {
                if args.is_empty() {
                    return Self::number(0.0);
                }

                if name.id() == ks::KS.exp && args.len() == 1 {
                    let inner_deriv = args[0].derive_impl(var, var_id, ctx);
                    return Self::mul_expr(
                        Self::func_symbol(ks::get_symbol(ks::KS.exp), (*args[0]).clone()),
                        inner_deriv,
                    );
                }

                if let Some(def) = crate::functions::Registry::get_by_symbol(name)
                    && def.validate_arity(args.len())
                {
                    let arg_primes: Vec<Self> = args
                        .iter()
                        .map(|arg| arg.derive_impl(var, var_id, ctx))
                        .collect();
                    return (def.derivative)(args, &arg_primes);
                }

                if let Some(user_fn) = ctx.get_user_fn_by_id(name.id()) {
                    let mut terms = Vec::new();

                    for (i, arg) in args.iter().enumerate() {
                        let arg_prime = arg.derive_impl(var, var_id, ctx);

                        if arg_prime.is_zero_num() {
                            continue;
                        }

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

                let mut terms = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let arg_prime = arg.derive_impl(var, var_id, ctx);
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

            ExprKind::Sum(terms) => {
                let derivs: Vec<Self> = terms
                    .iter()
                    .map(|t| t.derive_impl(var, var_id, ctx))
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

            ExprKind::Product(factors) => {
                if factors.is_empty() {
                    return Self::number(0.0);
                }
                if factors.len() == 1 {
                    return factors[0].derive_impl(var, var_id, ctx);
                }

                if factors.len() > 10 {
                    let mut log_terms = Vec::with_capacity(factors.len());
                    for factor in factors {
                        let prime = factor.derive_impl(var, var_id, ctx);
                        if !prime.is_zero_num() {
                            log_terms
                                .push(Self::div_from_arcs(Arc::new(prime), Arc::clone(factor)));
                        }
                    }

                    if log_terms.is_empty() {
                        return Self::number(0.0);
                    }

                    let sum_log = Self::sum(log_terms);
                    return Self::mul_expr(self.clone(), sum_log);
                }

                let mut terms = Vec::new();

                for i in 0..factors.len() {
                    if matches!(factors[i].kind, ExprKind::Number(_)) {
                        continue;
                    }

                    let factor_prime = factors[i].derive_impl(var, var_id, ctx);

                    if factor_prime.is_zero_num() {
                        continue;
                    }

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

            ExprKind::Div(u, v) => {
                if let ExprKind::Number(n) = &u.kind {
                    let v_prime = v.derive_impl(var, var_id, ctx);
                    if v_prime.is_zero_num() {
                        return Self::number(0.0);
                    }
                    let neg_n_fprime = Self::product(vec![Self::number(-n), v_prime]);
                    let f_squared = Self::pow_from_arcs(Arc::clone(v), Arc::new(Self::number(2.0)));
                    return Self::div_expr(neg_n_fprime, f_squared);
                }

                if let ExprKind::Number(_) = &v.kind {
                    let u_prime = u.derive_impl(var, var_id, ctx);
                    return Self::div_from_arcs(Arc::new(u_prime), Arc::clone(v));
                }

                let u_prime = u.derive_impl(var, var_id, ctx);
                let v_prime = v.derive_impl(var, var_id, ctx);

                let u_is_zero = u_prime.is_zero_num();
                let v_is_zero = v_prime.is_zero_num();

                if u_is_zero && v_is_zero {
                    Self::number(0.0)
                } else if v_is_zero {
                    Self::div_from_arcs(Arc::new(u_prime), Arc::clone(v))
                } else if u_is_zero {
                    let u_times_vprime =
                        Self::mul_from_arcs(vec![Arc::clone(u), Arc::new(v_prime)]);
                    let neg_u_vprime = u_times_vprime.negate();
                    let v_squared = Self::pow_from_arcs(Arc::clone(v), Arc::new(Self::number(2.0)));
                    Self::div_expr(neg_u_vprime, v_squared)
                } else {
                    let u_prime_v = Self::mul_from_arcs(vec![Arc::new(u_prime), Arc::clone(v)]);
                    let u_v_prime = Self::mul_from_arcs(vec![Arc::clone(u), Arc::new(v_prime)]);
                    let numerator = Self::sub_expr(u_prime_v, u_v_prime);
                    let v_squared = Self::pow_from_arcs(Arc::clone(v), Arc::new(Self::number(2.0)));
                    Self::div_expr(numerator, v_squared)
                }
            }

            ExprKind::Pow(u, v) => {
                let u_contains_var = u.contains_var_id(var_id);
                let v_contains_var = v.contains_var_id(var_id);

                if !u_contains_var && !v_contains_var {
                    Self::number(0.0)
                } else if !v_contains_var {
                    if v.as_number() == Some(0.0) {
                        return Self::number(0.0);
                    }

                    let u_prime = u.derive_impl(var, var_id, ctx);

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
                    let v_prime = v.derive_impl(var, var_id, ctx);

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
                    let u_prime = u.derive_impl(var, var_id, ctx);
                    let v_prime = v.derive_impl(var, var_id, ctx);

                    let ln_u = Self::func_symbol(ks::get_symbol(ks::KS.ln), (**u).clone());

                    let term1 = if v_prime.is_zero_num() {
                        Self::number(0.0)
                    } else if v_prime.is_one_num() {
                        ln_u
                    } else {
                        Self::mul_expr(v_prime, ln_u)
                    };

                    let term2 = if u_prime.is_zero_num() {
                        Self::number(0.0)
                    } else {
                        let u_prime_over_u = Self::div_from_arcs(Arc::new(u_prime), Arc::clone(u));
                        Self::mul_from_arcs(vec![Arc::clone(v), Arc::new(u_prime_over_u)])
                    };

                    let sum = if term1.is_zero_num() && term2.is_zero_num() {
                        return Self::number(0.0);
                    } else if term1.is_zero_num() {
                        term2
                    } else if term2.is_zero_num() {
                        term1
                    } else {
                        Self::add_expr(term1, term2)
                    };

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

            ExprKind::Derivative {
                inner,
                var: deriv_var,
                order,
            } => {
                if deriv_var.id() == var_id {
                    Self::derivative_interned(inner.as_ref().clone(), deriv_var.clone(), order + 1)
                } else if !inner.contains_var_id(var_id) {
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

            ExprKind::Poly(poly) => poly.derivative_expr(var),
        }
    }

    /// Create symbolic partial derivative for unknown function
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
