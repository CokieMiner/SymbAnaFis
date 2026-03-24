//! User-facing differentiation API.
//!
//! This module provides the [`Diff`] builder and the convenience [`diff`] function.

use crate::core::context::{Context, UserFunction};
use crate::evaluator::ToParamName;
use crate::{
    DEFAULT_MAX_DEPTH, DEFAULT_MAX_NODES, DiffError, Expr, Symbol, parser, simplification,
};
use rustc_hash::FxHashMap;
use std::collections::HashSet;
use std::sync::Arc;

/// Builder for differentiation operations
#[derive(Clone, Default)]
pub struct Diff {
    /// Whether to apply only domain-safe transformations
    domain_safe: bool,
    /// Whether to skip simplification after differentiation
    skip_simplification: bool,
    /// User-defined functions
    user_fns: FxHashMap<String, UserFunction>,
    max_depth: Option<usize>,
    /// Maximum number of nodes in the expression tree
    max_nodes: Option<usize>,
    /// Evaluation context
    context: Option<Context>,
    /// Known symbols for parsing
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
    #[inline]
    #[must_use]
    pub fn context(mut self, context: &Context) -> Self {
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
            .filter_map(|(name, func)| {
                func.body.as_ref().map(|b| {
                    let id = crate::core::symbol::symb_interned(name).id();
                    (id, Arc::clone(b))
                })
            })
            .collect()
    }

    /// Build context from builder state
    fn build_context(&self) -> Context {
        self.context.as_ref().map_or_else(
            || {
                let mut ctx = Context::new();
                for (name, func) in &self.user_fns {
                    ctx = ctx.with_function(name, func.clone());
                }
                ctx
            },
            |ctx| {
                let mut merged = ctx.clone();
                for (name, func) in &self.user_fns {
                    merged = merged.with_function(name, func.clone());
                }
                merged
            },
        )
    }

    /// Differentiates an expression with respect to a variable by name.
    pub(crate) fn differentiate_by_name(&self, expr: &Expr, var: &str) -> Result<Expr, DiffError> {
        if self.known_symbols.contains(var) {
            return Err(DiffError::VariableInBothFixedAndDiff {
                var: var.to_owned(),
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
                var: var.to_owned(),
            });
        }

        let custom_functions = self.custom_function_names();

        for func in &custom_functions {
            if symbols.contains(func) {
                return Err(DiffError::NameCollision { name: func.clone() });
            }
        }

        for func in &custom_functions {
            if self.known_symbols.contains(func) {
                return Err(DiffError::NameCollision { name: func.clone() });
            }
        }

        let ast = parser::parse(formula, &symbols, &custom_functions, self.context.as_ref())?;

        let var_sym = self
            .context
            .as_ref()
            .map_or_else(|| crate::symb(var), |ctx| ctx.symb(var));

        let result = self.differentiate(&ast, &var_sym)?;
        Ok(format!("{result}"))
    }
}

/// Differentiate a mathematical expression
///
/// This function parses a formula, differentiates it with respect to a variable,
/// and simplifies the result automatically. For advanced differentiation control,
/// use the [`Diff`] builder which offers domain safety, custom functions, and limits.
///
/// # Arguments
/// * `formula` - Mathematical expression to differentiate
/// * `var_to_diff` - Variable to differentiate with respect to
/// * `known_symbols` - Multi-character symbols for parsing
/// * `custom_functions` - User-defined function names
///
/// # Errors
/// Returns `DiffError` if parsing, differentiation, or validation fails.
pub fn diff(
    formula: &str,
    var_to_diff: &str,
    known_symbols: &[&str],
    custom_functions: Option<&[&str]>,
) -> Result<String, DiffError> {
    let mut builder = Diff::new();

    if let Some(funcs) = custom_functions {
        builder = funcs
            .iter()
            .fold(builder, |b, f| b.user_fn(*f, UserFunction::any_arity()));
    }

    builder
        .max_depth(DEFAULT_MAX_DEPTH)
        .max_nodes(DEFAULT_MAX_NODES)
        .diff_str(formula, var_to_diff, known_symbols)
}
