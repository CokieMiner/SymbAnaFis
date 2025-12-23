//! Builder pattern API for differentiation and simplification
//!
//! Provides a fluent interface for configuring and executing differentiation/simplification.
//!
//! # Example
//! ```
//! use symb_anafis::{symb, Diff};
//!
//! let x = symb("api_builder_x");
//! let expr = x.pow(2.0) + x.sin();
//!
//! let derivative = Diff::new()
//!     .domain_safe(true)
//!     .differentiate(expr, &x)
//!     .unwrap();
//! assert!(!format!("{}", derivative).is_empty());
//! ```

use crate::{DiffError, Expr, Symbol, core::symbol::SymbolContext, parser, simplification};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

/// Custom derivative function type (for single-arg functions)
/// Takes: (inner_expr, variable, inner_derivative) -> result
pub type CustomDerivativeFn = Arc<dyn Fn(&Expr, &str, &Expr) -> Expr + Send + Sync>;

/// Custom evaluation function type for user-defined functions
/// Takes: array of numeric arguments -> optional numeric result
pub type CustomEvalFn = Arc<dyn Fn(&[f64]) -> Option<f64> + Send + Sync>;

/// Partial derivative function for multi-arg functions
/// Takes: slice of Arc-wrapped argument expressions -> partial derivative expression (∂F/∂arg[i])
/// Uses Arc<Expr> to allow cheap cloning with Arc::clone() instead of deep cloning.
pub type PartialDerivativeFn = Arc<dyn Fn(&[Arc<Expr>]) -> Expr + Send + Sync>;

/// Definition for a multi-argument custom function
///
/// # Example
/// ```
/// use symb_anafis::{Expr, CustomFn};
/// use std::sync::Arc;
///
/// // F(x, y) = x * sin(y)
/// // ∂F/∂x = sin(y)
/// // ∂F/∂y = x * cos(y)
/// let my_fn = CustomFn::new(2)
///     .eval(|args| Some(args[0] * args[1].sin()))
///     .partial(0, |args| Expr::func("sin", &args[1]))      // ∂F/∂arg[0]
///     .partial(1, |args| Expr::from(&args[0]) * Expr::func("cos", &args[1]));  // ∂F/∂arg[1]
/// ```
#[derive(Clone)]
pub struct CustomFn {
    pub(crate) arity: usize,
    pub(crate) eval_fn: Option<CustomEvalFn>,
    pub(crate) partials: HashMap<usize, PartialDerivativeFn>,
}

impl CustomFn {
    /// Get the arity (number of arguments) of this function
    pub fn arity(&self) -> usize {
        self.arity
    }

    /// Create a new custom function with given arity
    pub fn new(arity: usize) -> Self {
        Self {
            arity,
            eval_fn: None,
            partials: HashMap::new(),
        }
    }

    /// Set the numeric evaluation function
    pub fn eval<F>(mut self, f: F) -> Self
    where
        F: Fn(&[f64]) -> Option<f64> + Send + Sync + 'static,
    {
        self.eval_fn = Some(Arc::new(f));
        self
    }

    /// Add a partial derivative for argument at index `i`
    ///
    /// The function receives all argument expressions as `Arc<Expr>` and returns `∂F/∂arg[i]`.
    /// Use `Expr::from(&args[i])` for clean conversion, or `Expr::func("name", &args[i])` directly.
    pub fn partial<F>(mut self, i: usize, f: F) -> Self
    where
        F: Fn(&[Arc<Expr>]) -> Expr + Send + Sync + 'static,
    {
        self.partials.insert(i, Arc::new(f));
        self
    }
}

/// Builder for differentiation operations
#[derive(Clone, Default)]
pub struct Diff {
    domain_safe: bool,
    skip_simplification: bool, // For benchmarking: return raw derivative
    fixed_vars: HashSet<String>,
    custom_functions: HashSet<String>,
    custom_derivatives: HashMap<String, CustomDerivativeFn>,
    custom_evals: HashMap<String, CustomEvalFn>,
    custom_fns: HashMap<String, CustomFn>, // New: multi-arg functions
    max_depth: Option<usize>,
    max_nodes: Option<usize>,
    context: Option<SymbolContext>,
}

impl Diff {
    /// Create a new differentiation builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable domain-safe mode (skips domain-altering rules)
    #[inline]
    pub fn domain_safe(mut self, safe: bool) -> Self {
        self.domain_safe = safe;
        self
    }

    /// Skip simplification and return raw derivative (for benchmarking)
    #[inline]
    pub fn skip_simplification(mut self, skip: bool) -> Self {
        self.skip_simplification = skip;
        self
    }

    /// Add a single fixed variable (treated as constant during differentiation)
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{symb, Diff};
    /// let a = symb("fixed_var_a");
    /// let x = symb("fixed_var_x");
    /// let expr = &a * &x;
    /// let result = Diff::new().fixed_var(&a).differentiate(expr, &x).unwrap();
    /// assert!(format!("{}", result).contains("fixed_var_a"));
    /// ```
    #[inline]
    pub fn fixed_var(mut self, var: &Symbol) -> Self {
        if let Some(name) = var.name() {
            self.fixed_vars.insert(name.to_string());
        }
        self
    }

    /// Add multiple fixed variables
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{symb, Diff};
    /// let a = symb("fixed_vars_a");
    /// let b = symb("fixed_vars_b");
    /// let diff = Diff::new().fixed_vars(&[&a, &b]);
    /// ```
    #[inline]
    pub fn fixed_vars(mut self, vars: &[&Symbol]) -> Self {
        for v in vars {
            if let Some(name) = v.name() {
                self.fixed_vars.insert(name.to_string());
            }
        }
        self
    }

    /// Add a custom function name (for parsing)
    #[inline]
    pub fn custom_fn(mut self, name: impl Into<String>) -> Self {
        self.custom_functions.insert(name.into());
        self
    }

    /// Set the symbol context for parsing and variable creation
    #[inline]
    pub fn with_context(mut self, context: &SymbolContext) -> Self {
        self.context = Some(context.clone());
        self
    }

    /// Register a custom function with its derivative rule
    ///
    /// # Arguments
    /// * `name` - Function name (e.g., "myFunc")
    /// * `derivative_fn` - Function that computes the derivative
    ///   - Arguments: (inner_expr, variable, inner_derivative)
    ///   - Returns: The derivative expression
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{Diff, Expr};
    /// let diff = Diff::new()
    ///     .custom_derivative("my_tan", |inner, _var, inner_prime| {
    ///         // d/dx[tan(u)] = sec²(u) * u'
    ///         Expr::mul_expr(
    ///             Expr::pow(Expr::func("sec", inner.clone()), Expr::number(2.0)),
    ///             inner_prime.clone()
    ///         )
    ///     });
    /// ```
    pub fn custom_derivative<F>(mut self, name: impl Into<String>, derivative_fn: F) -> Self
    where
        F: Fn(&Expr, &str, &Expr) -> Expr + Send + Sync + 'static,
    {
        let name = name.into();
        self.custom_functions.insert(name.clone());
        self.custom_derivatives
            .insert(name, Arc::new(derivative_fn));
        self
    }

    /// Register a custom evaluation function for a user-defined function
    ///
    /// This allows `f(x)` with `x=3` to evaluate to a numeric result instead of staying as `f(3)`.
    ///
    /// # Arguments
    /// * `name` - Function name (e.g., "f")
    /// * `eval_fn` - Function that takes numeric arguments and returns the computed value
    ///
    /// # Example
    /// ```
    /// use symb_anafis::Diff;
    /// // f(x) = x² + 1
    /// let diff = Diff::new()
    ///     .custom_eval("my_f", |args| Some(args[0].powi(2) + 1.0));
    /// ```
    pub fn custom_eval<F>(mut self, name: impl Into<String>, eval_fn: F) -> Self
    where
        F: Fn(&[f64]) -> Option<f64> + Send + Sync + 'static,
    {
        let name = name.into();
        self.custom_functions.insert(name.clone());
        self.custom_evals.insert(name, Arc::new(eval_fn));
        self
    }

    /// Register a multi-argument custom function with explicit partial derivatives
    ///
    /// This is the recommended way to define custom functions with 2+ arguments.
    /// The chain rule is automatically applied: `dF/dx = Σ (∂F/∂arg[i]) * (darg[i]/dx)`
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{Diff, CustomFn, Expr};
    ///
    /// // MyF(x, y) = x * sin(y)
    /// // ∂MyF/∂x = sin(y)       (where y = args[1])
    /// // ∂MyF/∂y = x * cos(y)   (where x = args[0], y = args[1])
    /// let diff = Diff::new()
    ///     .custom_fn_multi("MyF", CustomFn::new(2)
    ///         .eval(|args| Some(args[0] * args[1].sin()))
    ///         .partial(0, |args| Expr::func("sin", Expr::from(&args[1])))
    ///         .partial(1, |args| Expr::mul_expr(Expr::from(&args[0]), Expr::func("cos", Expr::from(&args[1]))))
    ///     );
    /// ```
    pub fn custom_fn_multi(mut self, name: impl Into<String>, def: CustomFn) -> Self {
        let name = name.into();
        self.custom_functions.insert(name.clone());
        // If CustomFn has an eval, also register it for evaluation
        if let Some(ref eval) = def.eval_fn {
            self.custom_evals.insert(name.clone(), eval.clone());
        }
        self.custom_fns.insert(name, def);
        self
    }

    /// Set maximum AST depth
    #[inline]
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set maximum AST node count
    #[inline]
    pub fn max_nodes(mut self, nodes: usize) -> Self {
        self.max_nodes = Some(nodes);
        self
    }

    /// Differentiate an expression with respect to a variable
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{symb, Diff};
    /// let x = symb("diff_example_x");
    /// let expr = x.pow(2.0);
    /// let result = Diff::new().differentiate(expr, &x).unwrap();
    /// assert_eq!(format!("{}", result), "2*diff_example_x");
    /// ```
    pub fn differentiate(&self, expr: Expr, var: &Symbol) -> Result<Expr, DiffError> {
        let var_name = var.name().unwrap_or_default();
        self.differentiate_by_name(expr, &var_name)
    }

    /// Differentiate an expression with respect to a variable name (internal API)
    ///
    /// This is the core differentiation method that takes &str directly.
    /// Use `differentiate` for the ergonomic Symbol-based API.
    pub(crate) fn differentiate_by_name(&self, expr: Expr, var: &str) -> Result<Expr, DiffError> {
        // Check limits
        if let Some(max_d) = self.max_depth
            && expr.max_depth() > max_d
        {
            return Err(DiffError::MaxDepthExceeded);
        }
        if let Some(max_n) = self.max_nodes
            && expr.node_count() > max_n
        {
            return Err(DiffError::MaxNodesExceeded);
        }

        // Differentiate
        let derivative = expr.derive(
            var,
            &self.fixed_vars,
            &self.custom_derivatives,
            &self.custom_fns,
        );

        // Skip simplification if requested (for benchmarking)
        if self.skip_simplification {
            return Ok(derivative);
        }

        // Simplify
        let simplified = if self.domain_safe {
            simplification::simplify_domain_safe(derivative, self.fixed_vars.clone())
        } else {
            simplification::simplify_expr(derivative, self.fixed_vars.clone())
        };

        Ok(simplified)
    }

    /// Parse and differentiate a string formula
    ///
    /// Uses the string-based API - for type-safe version, use `differentiate()` with Symbol
    pub fn diff_str(&self, formula: &str, var: &str) -> Result<String, DiffError> {
        // Validate
        if self.fixed_vars.contains(var) {
            return Err(DiffError::VariableInBothFixedAndDiff {
                var: var.to_string(),
            });
        }

        // Check for collisions between fixed vars and custom functions
        for func in &self.custom_functions {
            if self.fixed_vars.contains(func) {
                return Err(DiffError::NameCollision { name: func.clone() });
            }
        }

        // Parse
        let ast = parser::parse(
            formula,
            &self.fixed_vars,
            &self.custom_functions,
            self.context.as_ref(), // Pass the context
        )?;

        // Create a temporary Symbol for the variable
        let var_sym = if let Some(ctx) = &self.context {
            ctx.symb(var)
        } else {
            crate::symb(var) // Global fallback
        };

        // Differentiate
        let result = self.differentiate(ast, &var_sym)?;

        Ok(format!("{}", result))
    }

    /// Internal method for adding multiple fixed variables by name (for string API compatibility)
    pub(crate) fn fixed_vars_str<I, S>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for v in vars {
            self.fixed_vars.insert(v.as_ref().to_string());
        }
        self
    }
}

/// Builder for simplification operations
#[derive(Clone, Default)]
pub struct Simplify {
    domain_safe: bool,
    fixed_vars: HashSet<String>,
    custom_functions: HashSet<String>,
    custom_evals: HashMap<String, CustomEvalFn>,
    max_depth: Option<usize>,
    max_nodes: Option<usize>,
    context: Option<SymbolContext>,
}

impl Simplify {
    /// Create a new simplification builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable domain-safe mode
    #[inline]
    pub fn domain_safe(mut self, safe: bool) -> Self {
        self.domain_safe = safe;
        self
    }

    /// Add a single fixed variable
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{symb, Simplify};
    /// let a = symb("simplify_fixed_a");
    /// let simplifier = Simplify::new().fixed_var(&a);
    /// ```
    #[inline]
    pub fn fixed_var(mut self, var: &Symbol) -> Self {
        if let Some(name) = var.name() {
            self.fixed_vars.insert(name.to_string());
        }
        self
    }

    /// Add multiple fixed variables
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{symb, Simplify};
    /// let a = symb("simplify_fixed_a2");
    /// let b = symb("simplify_fixed_b2");
    /// let simplifier = Simplify::new().fixed_vars(&[&a, &b]);
    /// ```
    #[inline]
    pub fn fixed_vars(mut self, vars: &[&Symbol]) -> Self {
        for v in vars {
            if let Some(name) = v.name() {
                self.fixed_vars.insert(name.to_string());
            }
        }
        self
    }

    /// Add a custom function name
    #[inline]
    pub fn custom_fn(mut self, name: impl Into<String>) -> Self {
        self.custom_functions.insert(name.into());
        self
    }

    /// Set the symbol context for parsing
    #[inline]
    pub fn with_context(mut self, context: &SymbolContext) -> Self {
        self.context = Some(context.clone());
        self
    }

    /// Register a custom evaluation function for a user-defined function
    ///
    /// # Example
    /// ```
    /// use symb_anafis::Simplify;
    /// // f(x) = x² + 1
    /// let simplifier = Simplify::new()
    ///     .custom_eval("f", |args| Some(args[0].powi(2) + 1.0));
    /// ```
    pub fn custom_eval<F>(mut self, name: impl Into<String>, eval_fn: F) -> Self
    where
        F: Fn(&[f64]) -> Option<f64> + Send + Sync + 'static,
    {
        let name = name.into();
        self.custom_functions.insert(name.clone());
        self.custom_evals.insert(name, Arc::new(eval_fn));
        self
    }

    /// Set maximum AST depth
    #[inline]
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set maximum AST node count
    #[inline]
    pub fn max_nodes(mut self, nodes: usize) -> Self {
        self.max_nodes = Some(nodes);
        self
    }

    /// Simplify an expression
    pub fn simplify(&self, expr: Expr) -> Result<Expr, DiffError> {
        // Check limits
        if let Some(max_d) = self.max_depth
            && expr.max_depth() > max_d
        {
            return Err(DiffError::MaxDepthExceeded);
        }
        if let Some(max_n) = self.max_nodes
            && expr.node_count() > max_n
        {
            return Err(DiffError::MaxNodesExceeded);
        }

        let result = if self.domain_safe {
            simplification::simplify_domain_safe(expr, self.fixed_vars.clone())
        } else {
            simplification::simplify_expr(expr, self.fixed_vars.clone())
        };

        Ok(result)
    }

    /// Parse and simplify a string formula
    pub fn simplify_str(&self, formula: &str) -> Result<String, DiffError> {
        let ast = parser::parse(
            formula,
            &self.fixed_vars,
            &self.custom_functions,
            self.context.as_ref(),
        )?;
        let result = self.simplify(ast)?;
        Ok(format!("{}", result))
    }

    /// Internal method for adding multiple fixed variables by name (for string API compatibility)
    pub(crate) fn fixed_vars_str<I, S>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for v in vars {
            self.fixed_vars.insert(v.as_ref().to_string());
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::symbol::symb;

    #[test]
    fn test_diff_builder_basic() {
        let result = Diff::new().diff_str("x^2", "x").unwrap();
        assert_eq!(result, "2*x");
    }

    #[test]
    fn test_diff_with_fixed_var() {
        let a = symb("a");
        let result = Diff::new().fixed_var(&a).diff_str("a*x", "x").unwrap();
        assert_eq!(result, "a");
    }

    #[test]
    fn test_diff_domain_safe() {
        let result = Diff::new().domain_safe(true).diff_str("x^2", "x").unwrap();
        assert_eq!(result, "2*x");
    }

    #[test]
    fn test_diff_expr() {
        let x = symb("x");
        let expr = x.pow(2.0);

        let result = Diff::new().differentiate(expr, &x).unwrap();
        assert_eq!(format!("{}", result), "2*x");
    }

    #[test]
    fn test_simplify_builder() {
        let result = Simplify::new().simplify_str("x + x").unwrap();
        assert_eq!(result, "2*x");
    }

    #[test]
    fn test_custom_eval_with_evaluate() {
        use crate::Expr;
        use std::collections::HashMap;
        use std::sync::Arc;

        // Create f(x) where f(x) = x² + 1
        let x = symb("x");
        let f_of_x = Expr::func("f", x.to_expr());

        // Set up variable substitution (x = 3)
        let mut vars: HashMap<&str, f64> = HashMap::new();
        vars.insert("x", 3.0);

        // Set up custom eval: f(x) = x² + 1
        type CustomEval = Arc<dyn Fn(&[f64]) -> Option<f64> + Send + Sync>;
        let mut custom_evals: HashMap<String, CustomEval> = HashMap::new();
        custom_evals.insert(
            "f".to_string(),
            Arc::new(|args: &[f64]| Some(args[0].powi(2) + 1.0)),
        );

        // Evaluate f(3) = 3² + 1 = 10
        let result = f_of_x.evaluate_with_custom(&vars, &custom_evals);
        assert_eq!(format!("{}", result), "10");
    }

    #[test]
    fn test_custom_eval_without_evaluator() {
        use crate::Expr;
        use std::collections::HashMap;

        // Create f(x)
        let x = symb("x");
        let f_of_x = Expr::func("f", x.to_expr());

        // Set up variable substitution (x = 3)
        let mut vars: HashMap<&str, f64> = HashMap::new();
        vars.insert("x", 3.0);

        // Without custom evaluator, f(3) should remain as f(3)
        let result = f_of_x.evaluate(&vars);
        assert_eq!(format!("{}", result), "f(3)");
    }
}
