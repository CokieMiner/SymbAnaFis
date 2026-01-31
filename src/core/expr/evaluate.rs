//! Expression evaluation.
//!
//! Provides evaluation methods for partial/full numeric evaluation of expressions.
//!
//! ## Flexibility
//! The `evaluate` method accepts any type implementing `VarLookup`, including:
//! - `HashMap<&str, f64>` - string-based keys (convenient)
//! - `FxHashMap<u64, f64>` - ID-based keys (fast, use `symbol.id()`)

use std::collections::HashMap;
use std::sync::Arc;

use super::{CustomEvalMap, Expr, ExprKind};
use crate::core::symbol::InternedSymbol;

/// Trait for variable value lookup during evaluation.
///
/// This abstraction allows `evaluate` to work with both string-based
/// and ID-based variable maps without code duplication.
pub trait VarLookup {
    /// Look up the value for a symbol. Returns `None` if not found.
    fn get_value(&self, symbol: &InternedSymbol) -> Option<f64>;
}

// String-based lookup (backward compatible, convenient)
impl VarLookup for HashMap<&str, f64> {
    #[inline]
    fn get_value(&self, symbol: &InternedSymbol) -> Option<f64> {
        symbol.name().and_then(|name| self.get(name).copied())
    }
}

// ID-based lookup (fast, O(1) with FxHash)
impl VarLookup for rustc_hash::FxHashMap<u64, f64> {
    #[inline]
    fn get_value(&self, symbol: &InternedSymbol) -> Option<f64> {
        self.get(&symbol.id()).copied()
    }
}

// Also support standard HashMap with u64 keys
impl VarLookup for HashMap<u64, f64> {
    #[inline]
    fn get_value(&self, symbol: &InternedSymbol) -> Option<f64> {
        self.get(&symbol.id()).copied()
    }
}

// Support empty map (no variables)
impl VarLookup for () {
    #[inline]
    fn get_value(&self, _symbol: &InternedSymbol) -> Option<f64> {
        None
    }
}

impl Expr {
    /// Evaluate expression by substituting known variable values.
    ///
    /// This substitutes numeric values for variables and evaluates any subexpressions
    /// that become fully numeric. Unknown variables are left as-is in the result.
    ///
    /// Returns an `Expr` (not `f64`) because the result may still contain symbolic parts.
    /// Use [`as_number()`](Self::as_number) on the result to extract a numeric value if fully evaluated.
    ///
    /// # Arguments
    /// * `vars` - Any type implementing `VarLookup`:
    ///   - `HashMap<&str, f64>` for string-based keys (convenient)
    ///   - `FxHashMap<u64, f64>` for ID-based keys (faster, use `symbol.id()`)
    /// * `custom_evals` - Optional custom evaluation functions for user-defined functions
    ///
    /// # Panics
    /// Panics only if internal invariants are violated (never in normal use).
    ///
    /// # Examples
    ///
    /// String-based (convenient):
    /// ```
    /// use symb_anafis::{symb, Expr};
    /// use std::collections::HashMap;
    ///
    /// let x = symb("eval_x");
    /// let y = symb("eval_y");
    /// let expr = x + y;
    ///
    /// let mut vars = HashMap::new();
    /// vars.insert("eval_x", 3.0);
    /// vars.insert("eval_y", 2.0);
    /// let result = expr.evaluate(&vars, &HashMap::new());
    /// assert_eq!(result.as_number(), Some(5.0));
    /// ```
    ///
    /// ID-based (fast, for repeated evaluations):
    /// ```
    /// use symb_anafis::{symb, Expr};
    /// use rustc_hash::FxHashMap;
    /// use std::collections::HashMap;
    ///
    /// let x = symb("eval_id_x");
    /// let y = symb("eval_id_y");
    /// let expr = x.pow(2.0) + y;
    ///
    /// // Build ID map once, reuse for many evaluations
    /// let mut vars: FxHashMap<u64, f64> = FxHashMap::default();
    /// vars.insert(x.id(), 3.0);
    /// vars.insert(y.id(), 2.0);
    ///
    /// let result = expr.evaluate(&vars, &HashMap::new());
    /// assert_eq!(result.as_number(), Some(11.0)); // 3^2 + 2 = 11
    /// ```
    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "Expression evaluation handles many expression kinds"
    )]
    pub fn evaluate<V: VarLookup>(&self, vars: &V, custom_evals: &CustomEvalMap) -> Self {
        match &self.kind {
            ExprKind::Number(n) => Self::number(*n),
            ExprKind::Symbol(s) => {
                // Use trait-based lookup (works for both string and ID keys)
                if let Some(val) = vars.get_value(s) {
                    return Self::number(val);
                }
                // Check for mathematical constants
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
                // Optimized: single-pass with early zero exit and lazy Vec
                let mut num_prod: f64 = 1.0;
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
                        #[allow(clippy::float_cmp, reason = "Comparing against exact constant 1.0")]
                        if num_prod != 1.0 {
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
                let base_result = poly.base().evaluate(vars, custom_evals);
                if let ExprKind::Number(base_val) = &base_result.kind {
                    let mut total = 0.0;
                    for &(pow, coeff) in poly.terms() {
                        #[allow(
                            clippy::cast_possible_wrap,
                            reason = "Polynomial powers are small positive integers"
                        )]
                        {
                            total += coeff * base_val.powi(pow as i32);
                        }
                    }
                    Self::number(total)
                } else if base_result == *poly.base() {
                    self.clone()
                } else {
                    let mut new_poly = poly.clone();
                    new_poly.set_base(Arc::new(base_result));
                    Self::poly(new_poly)
                }
            }
        }
    }
}
