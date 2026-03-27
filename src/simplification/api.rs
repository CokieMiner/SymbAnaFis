use crate::core::{BodyFn, Context, UserFunction};
use crate::evaluator::ToParamName;
use crate::parser::parse;
use crate::{DEFAULT_MAX_DEPTH, DEFAULT_MAX_NODES, DiffError, Expr};
use rustc_hash::FxHashMap;
use std::collections::{HashMap, HashSet};
use std::string::ToString;
use std::sync::Arc;

use super::logic::{Simplifier, prettify_roots};
/// Type alias for custom body function map (symbolic expansion).
use crate::core::symb_interned;
/// Uses std `HashMap` at the API boundary for caller convenience;
/// converted to `FxHashMap` internally by the engine.
pub type CustomBodyMap = HashMap<u64, BodyFn>;

/// Builder for simplification operations.
#[derive(Clone, Default)]
pub struct Simplify {
    domain_safe: bool,
    user_fns: FxHashMap<String, UserFunction>,
    max_depth: Option<usize>,
    max_nodes: Option<usize>,
    context: Option<Context>,
    known_symbols: HashSet<String>,
}

impl Simplify {
    #[must_use]
    #[doc = "Create a new simplification builder with default settings."]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    #[must_use]
    #[doc = "Enable or disable domain-safe mode."]
    pub const fn domain_safe(mut self, safe: bool) -> Self {
        self.domain_safe = safe;
        self
    }

    #[inline]
    #[must_use]
    #[doc = "Set the Context for parsing and simplification."]
    pub fn context(mut self, context: &Context) -> Self {
        self.context = Some(context.clone());
        self
    }

    #[must_use]
    #[doc = "Register a user-defined function with body and/or partial derivatives."]
    pub fn user_fn(mut self, name: impl Into<String>, def: UserFunction) -> Self {
        self.user_fns.insert(name.into(), def);
        self
    }

    #[inline]
    #[must_use]
    #[doc = "Set maximum AST depth."]
    pub const fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    #[inline]
    #[must_use]
    #[doc = "Set maximum AST node count."]
    pub const fn max_nodes(mut self, nodes: usize) -> Self {
        self.max_nodes = Some(nodes);
        self
    }

    #[inline]
    #[must_use]
    #[doc = "Register a variable as constant during simplification."]
    pub fn fixed_var<P: ToParamName>(mut self, var: &P) -> Self {
        let (_, name) = var.to_param_id_and_name();
        self.known_symbols.insert(name);
        self
    }

    #[inline]
    #[must_use]
    #[doc = "Register multiple variables as constants during simplification."]
    pub fn fixed_vars<P: ToParamName>(mut self, vars: &[P]) -> Self {
        for var in vars {
            let (_, name) = var.to_param_id_and_name();
            self.known_symbols.insert(name);
        }
        self
    }

    fn custom_function_names(&self) -> HashSet<String> {
        self.user_fns.keys().cloned().collect()
    }

    fn build_bodies_map(&self) -> CustomBodyMap {
        self.user_fns
            .iter()
            .filter_map(|(name, func)| {
                func.body.as_ref().map(|b| {
                    let id = symb_interned(name).id();
                    (id, Arc::clone(b))
                })
            })
            .collect()
    }

    /// # Errors
    /// Returns `DiffError` if expression limits are exceeded.
    pub fn simplify(&self, expr: &Expr) -> Result<Expr, DiffError> {
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

        Ok(simplify_expr(
            expr.clone(),
            self.known_symbols.clone(),
            self.build_bodies_map(),
            self.max_depth,
            None,
            None,
            self.domain_safe,
        ))
    }

    /// # Errors
    /// Returns `DiffError` if parsing fails or there is a symbol/function collision.
    pub fn simplify_str(&self, formula: &str, known_symbols: &[&str]) -> Result<String, DiffError> {
        let mut symbols: HashSet<String> = known_symbols.iter().map(ToString::to_string).collect();
        symbols.extend(self.known_symbols.clone());

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

        let ast = parse(formula, &symbols, &custom_functions, self.context.as_ref())?;
        let result = self.simplify(&ast)?;
        Ok(format!("{result}"))
    }
}

pub fn simplify_expr(
    expr: Expr,
    _known_symbols: HashSet<String>,
    mut custom_bodies: CustomBodyMap,
    max_depth: Option<usize>,
    max_iterations: Option<usize>,
    context: Option<&Context>,
    domain_safe: bool,
) -> Expr {
    if let Some(ctx) = context {
        for id in ctx.fn_name_to_id().values() {
            if let Some(body) = ctx.get_body_by_id(*id) {
                custom_bodies.insert(*id, Arc::clone(&body));
            }
        }
    }

    let mut simplifier = Simplifier::new()
        .with_domain_safe(domain_safe)
        .with_custom_bodies(custom_bodies);

    if let Some(depth) = max_depth {
        simplifier = simplifier.with_max_depth(depth);
    }
    if let Some(iters) = max_iterations {
        simplifier = simplifier.with_max_iterations(iters);
    }

    let mut current = simplifier.simplify(expr);
    current = prettify_roots(current);
    current
}

/// Simplify a mathematical expression
///
/// This function applies algebraic, trigonometric, and other mathematical rules to
/// reduce expressions to their simplest form. For advanced simplification control,
/// use the [`Simplify`] builder which offers domain safety and iteration limits.
///
/// # Arguments
/// * `formula` - Mathematical expression to simplify (e.g., "x + x + sin(x)^2 + cos(x)^2")
/// * `known_symbols` - Multi-character symbols for parsing (e.g., `&["alpha", "beta"]`). These are hints to the parser and do NOT affect simplification logic.
/// * `custom_functions` - User-defined function names (e.g., `Some(&["f", "g"])`)
///
/// # Returns
/// The simplified expression as a string, or an error if parsing fails
///
/// # Errors
/// Returns `DiffError` if:
/// - **Syntax error**: Formula cannot be parsed
/// - **Complexity limits**: Expression exceeds safety limits (rare)
///
/// # Examples
///
/// ## Algebraic simplification
/// ```
/// # use symb_anafis::simplify;
///
/// // Like terms
/// let result = simplify("x + x + x", &[], None)?;
/// assert_eq!(result, "3*x");
///
/// // Polynomial expansion
/// let result = simplify("(x + 1)^2", &[], None)?;
/// assert_eq!(result, "(1 + x)^2"); // May not expand automatically
///
/// // Fraction reduction
/// let result = simplify("(x^2 - 1)/(x - 1)", &[], None)?;
/// // Complex expression - may not simplify without domain assumptions
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// ## Trigonometric identities
/// ```
/// # use symb_anafis::simplify;
///
/// // Pythagorean identity
/// let result = simplify("sin(x)^2 + cos(x)^2", &[], None)?;
/// assert_eq!(result, "1");
///
/// // Double angle
/// let result = simplify("2*sin(x)*cos(x)", &[], None)?;
/// assert_eq!(result, "sin(2*x)");
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// ## Exponential and logarithmic
/// ```
/// use symb_anafis::simplify;
///
/// // Log properties
/// let result = simplify("ln(e^x)", &[], None)?;
/// assert_eq!(result, "x");
///
/// // Exponential properties
/// let result = simplify("e^(ln(x))", &[], None)?;
/// assert_eq!(result, "x");
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// ## Multi-character symbols
/// ```
/// # use symb_anafis::simplify;
///
/// let result = simplify("alpha + alpha + beta", &["alpha", "beta"], None)?;
/// assert_eq!(result, "(2*alpha) + beta"); // Order may vary
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// # Simplification Categories
///
/// | Category | Rules Applied | Example |
/// |----------|---------------|----------|
/// | **Algebraic** | Like terms, polynomial operations | `x + 2*x` → `3*x` |
/// | **Trigonometric** | Pythagorean, double angle, etc. | `sin²(x) + cos²(x)` → `1` |
/// | **Exponential** | Log/exp inverses, power rules | `ln(e^x)` → `x` |
/// | **Rational** | Common factors, fraction reduction | `x²/(x*y)` → `x/y` |
/// | **Constants** | Arithmetic with numbers | `2 + 3*x + 1` → `3 + 3*x` |
///
/// # Performance and Limits
/// Default safety limits prevent infinite simplification loops:
/// - **Max depth**: 100 (default)
/// - **Max nodes**: 10,000 (default)
/// - **Max iterations**: 1000 simplification passes
///
/// For complex expressions, use the [`Simplify`] builder:
/// ```
/// use symb_anafis::Simplify;
///
/// let result = Simplify::new()
///     .domain_safe(true)  // Avoid division by zero transformations
///     .simplify_str("x + x", &[])?;
/// assert_eq!(result, "2*x");
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// # Domain Safety
/// By default, simplification may apply transformations that change the expression's domain.
/// For domain-preserving simplification, use the builder:
/// ```
/// use symb_anafis::Simplify;
///
/// // This avoids simplifying x^2/x → x (which is undefined at x=0)
/// let result = Simplify::new()
///     .domain_safe(true)
///     .simplify_str("x^2/x", &[])?;
/// # Ok::<(), symb_anafis::DiffError>(())
/// ```
///
/// # See Also
/// - [`Simplify`]: Builder pattern for advanced simplification control
/// - [`crate::diff`]: Differentiation with automatic simplification
/// - [`crate::Diff::skip_simplification`]: Raw derivatives without simplification
pub fn simplify(
    formula: &str,
    known_symbols: &[&str],
    custom_functions: Option<&[&str]>,
) -> Result<String, DiffError> {
    let mut builder = Simplify::new();

    if let Some(funcs) = custom_functions {
        builder = funcs
            .iter()
            .fold(builder, |b, f| b.user_fn(*f, UserFunction::any_arity()));
    }

    builder
        .max_depth(DEFAULT_MAX_DEPTH)
        .max_nodes(DEFAULT_MAX_NODES)
        .simplify_str(formula, known_symbols)
}
