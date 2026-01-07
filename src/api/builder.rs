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
//!     .differentiate(&expr, &x)
//!     .unwrap();
//! assert!(!format!("{}", derivative).is_empty());
//! ```

use crate::core::evaluator::ToParamName;
use crate::core::unified_context::{Context, UserFunction};
use crate::{DiffError, Expr, Symbol, parser, simplification};
use std::collections::HashMap;
use std::collections::HashSet;

/// Builder for differentiation operations
#[derive(Clone, Default)]
pub struct Diff {
    domain_safe: bool,
    skip_simplification: bool,
    user_fns: HashMap<String, UserFunction>,
    max_depth: Option<usize>,
    max_nodes: Option<usize>,
    context: Option<Context>,
    known_symbols: HashSet<String>,
}

impl Diff {
    /// Create a new differentiation builder with default settings
    #[must_use] 
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable domain-safe mode (skips domain-altering rules)
    #[inline]
    #[must_use] 
    pub const fn domain_safe(mut self, safe: bool) -> Self {
        self.domain_safe = safe;
        self
    }

    /// Skip simplification and return raw derivative (for benchmarking)
    #[inline]
    #[must_use] 
    pub const fn skip_simplification(mut self, skip: bool) -> Self {
        self.skip_simplification = skip;
        self
    }

    /// Set the Context for parsing and differentiation.
    /// Use this when you have Symbol references - the context's symbols are used for parsing.
    #[inline]
    #[must_use] 
    pub fn with_context(mut self, context: &Context) -> Self {
        self.context = Some(context.clone());
        self
    }

    /// Register a user-defined function with explicit partial derivatives
    #[must_use]
    pub fn user_fn(mut self, name: impl Into<String>, def: UserFunction) -> Self {
        self.user_fns.insert(name.into(), def);
        self
    }

    /// Set maximum AST depth
    #[inline]
    #[must_use] 
    pub const fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set maximum AST node count
    #[inline]
    #[must_use] 
    pub const fn max_nodes(mut self, nodes: usize) -> Self {
        self.max_nodes = Some(nodes);
        self
    }

    /// Register a variable as constant during differentiation
    #[inline]
    #[must_use]
    pub fn fixed_var<P: ToParamName>(mut self, var: &P) -> Self {
        let (_, name) = var.to_param_id_and_name();
        self.known_symbols.insert(name);
        self
    }

    /// Register multiple variables as constants during differentiation
    #[inline]
    #[must_use]
    pub fn fixed_vars<P: ToParamName>(mut self, vars: &[P]) -> Self {
        for var in vars {
            let (_, name) = var.to_param_id_and_name();
            self.known_symbols.insert(name);
        }
        self
    }

    /// Differentiate an expression with respect to a variable
    ///
    /// # Errors
    /// Returns `DiffError` if:
    /// - The variable is also in the fixed variables set
    /// - Expression depth exceeds `max_depth`
    /// - Expression node count exceeds `max_nodes`
    pub fn differentiate(&self, expr: &Expr, var: &Symbol) -> Result<Expr, DiffError> {
        let var_name = var.name().unwrap_or_default();
        self.differentiate_by_name(expr, &var_name)
    }

    /// Get custom function names for parsing
    fn custom_function_names(&self) -> HashSet<String> {
        self.user_fns.keys().cloned().collect()
    }

    /// Build body functions map for simplification
    fn build_bodies_map(&self) -> simplification::CustomBodyMap {
        self.user_fns
            .iter()
            .filter_map(|(name, func)| func.body.as_ref().map(|b| (name.clone(), b.clone())))
            .collect()
    }

    /// Build context from builder state
    fn build_context(&self) -> Context {
        self.context.as_ref().map_or_else(|| {
            let mut ctx = Context::new();
            for (name, func) in &self.user_fns {
                ctx = ctx.with_function(name, func.clone());
            }
            ctx
        }, |ctx| {
            let mut merged = ctx.clone();
            for (name, func) in &self.user_fns {
                merged = merged.with_function(name, func.clone());
            }
            merged
        })
    }

    pub(crate) fn differentiate_by_name(&self, expr: &Expr, var: &str) -> Result<Expr, DiffError> {
        if self.known_symbols.contains(var) {
            return Err(DiffError::VariableInBothFixedAndDiff {
                var: var.to_string(),
            });
        }

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

        let context = self.build_context();
        let derivative = expr.derive(var, Some(&context));

        if self.skip_simplification {
            return Ok(derivative);
        }

        let simplified = simplification::simplify_expr(
            derivative,
            self.known_symbols.clone(),
            self.build_bodies_map(),
            self.max_depth,
            None,
            None,
            self.domain_safe,
        );

        Ok(simplified)
    }

    /// Parse and differentiate a string formula
    ///
    /// # Arguments
    /// * `formula` - The mathematical expression to differentiate
    /// * `var` - The variable to differentiate with respect to
    /// * `known_symbols` - Known multi-character symbol names for parsing
    ///
    /// # Example
    /// ```
    /// use symb_anafis::Diff;
    /// let result = Diff::new().diff_str("alpha*x", "x", &["alpha"]).unwrap();
    /// assert_eq!(result, "alpha");
    /// ```
    ///
    /// # Errors
    /// Returns `DiffError` if:
    /// - Parsing fails
    /// - The variable is in the known symbols set
    /// - A name collision between symbols and functions is detected
    pub fn diff_str(
        &self,
        formula: &str,
        var: &str,
        known_symbols: &[&str],
    ) -> Result<String, DiffError> {
        let mut symbols: HashSet<String> = known_symbols
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        symbols.extend(self.known_symbols.clone());

        if symbols.contains(var) {
            return Err(DiffError::VariableInBothFixedAndDiff {
                var: var.to_string(),
            });
        }

        let custom_functions = self.custom_function_names();

        for func in &custom_functions {
            if symbols.contains(func) {
                return Err(DiffError::NameCollision { name: func.clone() });
            }
        }

        // Check for conflicts between builder's known_symbols and custom functions
        for func in &custom_functions {
            if self.known_symbols.contains(func) {
                return Err(DiffError::NameCollision { name: func.clone() });
            }
        }

        let ast = parser::parse(formula, &symbols, &custom_functions, self.context.as_ref())?;

        let var_sym = self.context.as_ref().map_or_else(|| crate::symb(var), |ctx| ctx.symb(var));

        let result = self.differentiate(&ast, &var_sym)?;
        Ok(format!("{result}"))
    }
}

/// Builder for simplification operations
#[derive(Clone, Default)]
pub struct Simplify {
    domain_safe: bool,
    user_fns: HashMap<String, UserFunction>,
    max_depth: Option<usize>,
    max_nodes: Option<usize>,
    context: Option<Context>,
    known_symbols: HashSet<String>,
}

impl Simplify {
    /// Create a new simplification builder with default settings
    #[must_use] 
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable domain-safe mode
    #[inline]
    #[must_use] 
    pub const fn domain_safe(mut self, safe: bool) -> Self {
        self.domain_safe = safe;
        self
    }

    /// Set the Context for parsing and simplification.
    /// Use this when you have Symbol references - the context's symbols are used for parsing.
    #[inline]
    #[must_use] 
    pub fn with_context(mut self, context: &Context) -> Self {
        self.context = Some(context.clone());
        self
    }

    /// Register a user-defined function with body and/or partial derivatives
    #[must_use]
    pub fn user_fn(mut self, name: impl Into<String>, def: UserFunction) -> Self {
        self.user_fns.insert(name.into(), def);
        self
    }

    /// Set maximum AST depth
    #[inline]
    #[must_use] 
    pub const fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set maximum AST node count
    #[inline]
    #[must_use] 
    pub const fn max_nodes(mut self, nodes: usize) -> Self {
        self.max_nodes = Some(nodes);
        self
    }

    /// Register a variable as constant during simplification
    #[inline]
    #[must_use]
    pub fn fixed_var<P: ToParamName>(mut self, var: &P) -> Self {
        let (_, name) = var.to_param_id_and_name();
        self.known_symbols.insert(name);
        self
    }

    /// Register multiple variables as constants during simplification
    #[inline]
    #[must_use]
    pub fn fixed_vars<P: ToParamName>(mut self, vars: &[P]) -> Self {
        for var in vars {
            let (_, name) = var.to_param_id_and_name();
            self.known_symbols.insert(name);
        }
        self
    }

    /// Get custom function names for parsing
    fn custom_function_names(&self) -> HashSet<String> {
        self.user_fns.keys().cloned().collect()
    }

    /// Build body functions map for simplification
    fn build_bodies_map(&self) -> simplification::CustomBodyMap {
        self.user_fns
            .iter()
            .filter_map(|(name, func)| func.body.as_ref().map(|b| (name.clone(), b.clone())))
            .collect()
    }

    /// Simplify an expression
    ///
    /// # Errors
    /// Returns `DiffError` if expression node count exceeds `max_nodes`.
    pub fn simplify(&self, expr: &Expr) -> Result<Expr, DiffError> {
        if let Some(max_n) = self.max_nodes
            && expr.node_count() > max_n
        {
            return Err(DiffError::MaxNodesExceeded);
        }

        let result = simplification::simplify_expr(
            expr.clone(),
            self.known_symbols.clone(),
            self.build_bodies_map(),
            self.max_depth,
            None,
            None,
            self.domain_safe,
        );

        Ok(result)
    }

    /// Parse and simplify a string formula
    ///
    /// # Arguments
    /// * `formula` - The mathematical expression to simplify
    /// * `known_symbols` - Known multi-character symbol names for parsing
    ///
    /// # Example
    /// ```
    /// use symb_anafis::Simplify;
    /// let result = Simplify::new().simplify_str("alpha + alpha", &["alpha"]).unwrap();
    /// assert_eq!(result, "2*alpha");
    /// ```
    ///
    /// # Errors
    /// Returns `DiffError` if parsing or simplification fails.
    pub fn simplify_str(&self, formula: &str, known_symbols: &[&str]) -> Result<String, DiffError> {
        let mut symbols: HashSet<String> = known_symbols
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        symbols.extend(self.known_symbols.clone());

        let ast = parser::parse(
            formula,
            &symbols,
            &self.custom_function_names(),
            self.context.as_ref(),
        )?;
        let result = self.simplify(&ast)?;
        Ok(format!("{result}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::symbol::symb;

    #[test]
    fn test_diff_builder_basic() {
        let result = Diff::new().diff_str("x^2", "x", &[]).unwrap();
        assert_eq!(result, "2*x");
    }

    #[test]
    fn test_diff_with_known_symbol() {
        let result = Diff::new().diff_str("alpha*x", "x", &["alpha"]).unwrap();
        assert_eq!(result, "alpha");
    }

    #[test]
    fn test_diff_domain_safe() {
        let result = Diff::new()
            .domain_safe(true)
            .diff_str("x^2", "x", &[])
            .unwrap();
        assert_eq!(result, "2*x");
    }

    #[test]
    fn test_diff_expr() {
        let x = symb("test_diff_x");
        let expr = x.pow(2.0);
        let result = Diff::new().differentiate(&expr, &x).unwrap();
        assert_eq!(format!("{}", result), "2*test_diff_x");
    }

    #[test]
    fn test_simplify_builder() {
        let result = Simplify::new().simplify_str("x + x", &[]).unwrap();
        assert_eq!(result, "2*x");
    }

    #[test]
    fn test_custom_eval_with_evaluate() {
        use crate::Expr;
        use std::collections::HashMap;
        use std::sync::Arc;

        let x = symb("test_custom_x");
        let f_of_x = Expr::func("f", x.to_expr());

        let mut vars: HashMap<&str, f64> = HashMap::new();
        vars.insert("test_custom_x", 3.0);

        type CustomEval = Arc<dyn Fn(&[f64]) -> Option<f64> + Send + Sync>;
        let mut custom_evals: HashMap<String, CustomEval> = HashMap::new();
        custom_evals.insert(
            "f".to_string(),
            Arc::new(|args: &[f64]| Some(args[0].powi(2) + 1.0)),
        );

        let result = f_of_x.evaluate(&vars, &custom_evals);
        assert_eq!(format!("{}", result), "10");
    }

    #[test]
    fn test_custom_eval_without_evaluator() {
        use crate::Expr;
        use std::collections::HashMap;

        let x = symb("test_noeval_x");
        let f_of_x = Expr::func("f", x.to_expr());

        let mut vars: HashMap<&str, f64> = HashMap::new();
        vars.insert("test_noeval_x", 3.0);

        let result = f_of_x.evaluate(&vars, &HashMap::new());
        assert_eq!(format!("{}", result), "f(3)");
    }

    #[test]
    fn test_fixed_var() {
        let a = symb("a");
        let _x = symb("x");

        // Test fixed_var on Diff
        let result = Diff::new().fixed_var(&a).diff_str("a*x", "x", &[]).unwrap();
        assert_eq!(result, "a");

        // Test fixed_vars on Diff
        let b = symb("b");
        let result2 = Diff::new()
            .fixed_vars(&[&a, &b])
            .diff_str("a*x + b", "x", &[])
            .unwrap();
        assert_eq!(result2, "a");

        // Test fixed_var on Simplify
        let result3 = Simplify::new()
            .fixed_var(&a)
            .simplify_str("a + a", &[])
            .unwrap();
        assert_eq!(result3, "2*a");
    }
}
