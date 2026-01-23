//! Expression evaluation.
//!
//! Provides the `evaluate` method for partial/full numeric evaluation of expressions.

use std::collections::HashMap;
use std::sync::Arc;

use super::{CustomEvalMap, Expr, ExprKind};

impl Expr {
    /// Partially evaluate expression by substituting known variable values.
    ///
    /// This substitutes numeric values for variables and evaluates any subexpressions
    /// that become fully numeric. Unknown variables are left as-is in the result.
    ///
    /// Returns an `Expr` (not `f64`) because the result may still contain symbolic parts.
    /// Use [`as_number()`](Self::as_number) on the result to extract a numeric value if fully evaluated.
    ///
    /// # Arguments
    /// * `vars` - Map of variable names to their numeric values
    /// * `custom_evals` - Optional custom evaluation functions for user-defined functions
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use).
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{symb, Expr};
    /// use std::collections::HashMap;
    ///
    /// let x = symb("x");
    /// let y = symb("y");
    /// let expr = x + y;  // x + y
    ///
    /// // Partial evaluation: only x is known
    /// let mut vars = HashMap::new();
    /// vars.insert("x", 3.0);
    /// let result = expr.evaluate(&vars, &HashMap::new());  // 3 + y (still an Expr)
    ///
    /// // Full evaluation: both variables known
    /// vars.insert("y", 2.0);
    /// let result = expr.evaluate(&vars, &HashMap::new());  // 5.0 as Expr
    /// assert_eq!(result.as_number(), Some(5.0));
    /// ```
    #[must_use]
    // Expression evaluation handles many expression kinds, length is justified
    #[allow(clippy::too_many_lines)] // Expression evaluation handles many expression kinds
    pub fn evaluate(&self, vars: &HashMap<&str, f64>, custom_evals: &CustomEvalMap) -> Self {
        match &self.kind {
            ExprKind::Number(n) => Self::number(*n),
            ExprKind::Symbol(s) => {
                // First check if it's a user-provided variable value
                if let Some(name) = s.name()
                    && let Some(&val) = vars.get(name)
                {
                    return Self::number(val);
                }
                // Check for mathematical constants using centralized helper
                if let Some(name) = s.name()
                    && let Some(value) = crate::core::known_symbols::get_constant_value(name)
                {
                    return Self::number(value);
                }
                self.clone()
            }
            ExprKind::FunctionCall { name, args } => {
                let eval_args: Vec<Self> = args
                    .iter()
                    .map(|a| a.evaluate(vars, custom_evals))
                    .collect();

                let numeric_args: Option<Vec<f64>> =
                    eval_args.iter().map(Self::as_number).collect();

                if let Some(args_vec) = numeric_args {
                    if let Some(custom_eval) = custom_evals.get(name.as_str())
                        && let Some(result) = custom_eval(&args_vec)
                    {
                        return Self::number(result);
                    }
                    if let Some(func_def) =
                        crate::functions::registry::Registry::get_by_symbol(name)
                        && let Some(result) = (func_def.eval)(&args_vec)
                    {
                        return Self::number(result);
                    }
                }

                Self::new(ExprKind::FunctionCall {
                    name: name.clone(),
                    args: eval_args.into_iter().map(Arc::new).collect(),
                })
            }
            ExprKind::Sum(terms) => {
                // Optimized: single-pass accumulation with lazy Vec allocation
                let mut num_sum: f64 = 0.0;
                // Only allocate when we encounter first non-numeric term
                let mut others: Option<Vec<Self>> = None;

                for t in terms {
                    let eval_t = t.evaluate(vars, custom_evals);
                    if let ExprKind::Number(n) = eval_t.kind {
                        num_sum += n;
                    } else {
                        others.get_or_insert_with(Vec::new).push(eval_t);
                    }
                }

                others.map_or_else(
                    || Self::number(num_sum),
                    |mut v| {
                        if num_sum != 0.0 {
                            v.push(Self::number(num_sum));
                        }
                        if v.len() == 1 {
                            v.pop().expect("v must have exactly one element")
                        } else {
                            Self::sum(v)
                        }
                    },
                )
            }
            ExprKind::Product(factors) => {
                // Optimized: single-pass accumulation with early zero exit and lazy Vec
                let mut num_prod: f64 = 1.0;
                // Only allocate when we encounter first non-numeric factor
                let mut others: Option<Vec<Self>> = None;

                for f in factors {
                    let eval_f = f.evaluate(vars, custom_evals);
                    if let ExprKind::Number(n) = eval_f.kind {
                        if n == 0.0 {
                            return Self::number(0.0); // Early exit
                        }
                        num_prod *= n;
                    } else {
                        others.get_or_insert_with(Vec::new).push(eval_f);
                    }
                }

                others.map_or_else(
                    || Self::number(num_prod),
                    |mut v| {
                        let num_is_one = {
                            #[allow(clippy::float_cmp)] // Comparing against exact constant 1.0
                            let res = num_prod != 1.0;
                            res
                        };
                        if num_is_one {
                            v.insert(0, Self::number(num_prod));
                        }
                        if v.len() == 1 {
                            v.pop().expect("v must have exactly one element")
                        } else {
                            Self::product(v)
                        }
                    },
                )
            }
            ExprKind::Div(a, b) => {
                let ea = a.evaluate(vars, custom_evals);
                let eb = b.evaluate(vars, custom_evals);
                match (&ea.kind, &eb.kind) {
                    (ExprKind::Number(x), ExprKind::Number(y)) if *y != 0.0 => Self::number(x / y),
                    _ => Self::div_expr(ea, eb),
                }
            }
            ExprKind::Pow(a, b) => {
                let ea = a.evaluate(vars, custom_evals);
                let eb = b.evaluate(vars, custom_evals);
                match (&ea.kind, &eb.kind) {
                    (ExprKind::Number(x), ExprKind::Number(y)) => Self::number(x.powf(*y)),
                    _ => Self::pow_static(ea, eb),
                }
            }
            ExprKind::Derivative { inner, var, order } => {
                Self::derivative(inner.evaluate(vars, custom_evals), var.clone(), *order)
            }
            ExprKind::Poly(poly) => {
                // Polynomial evaluation: evaluate P(base) where base is evaluated first
                let base_result = poly.base().evaluate(vars, custom_evals);
                // If base evaluates to a number, compute the polynomial value
                if let ExprKind::Number(base_val) = &base_result.kind {
                    let mut total = 0.0;
                    for &(pow, coeff) in poly.terms() {
                        {
                            // u32->i32: polynomial powers are small positive integers
                            #[allow(clippy::cast_possible_wrap)]
                            // Polynomial powers are small positive integers
                            {
                                total += coeff * base_val.powi(pow as i32);
                            }
                        }
                    }
                    Self::number(total)
                } else {
                    // Can't fully evaluate, return as-is
                    self.clone()
                }
            }
        }
    }
}
