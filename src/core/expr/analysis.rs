//! Expression analysis methods.
//!
//! Provides methods for analyzing expression structure: `node_count`, depth, variables, etc.

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;

use super::{Expr, ExprKind};

impl Expr {
    // -------------------------------------------------------------------------
    // View API - Public pattern matching
    // -------------------------------------------------------------------------

    /// Get a pattern-matchable view of this expression.
    ///
    /// Returns an `ExprView` that can be matched on without exposing internal
    /// implementation details. This is the recommended way to inspect expression
    /// structure from external code.
    ///
    /// # Important
    ///
    /// Polynomial expressions (`ExprKind::Poly`) are **always** presented as `Sum`
    /// in the view. This ensures forward compatibility when the internal polynomial
    /// representation changes (e.g., from univariate to multivariate).
    ///
    /// # Example
    ///
    /// ```rust
    /// use symb_anafis::{symb, visitor::ExprView};
    ///
    /// let x = symb("view_api_x");
    /// let expr = x.pow(2.0) + 2.0*x + 1.0;
    ///
    /// match expr.view() {
    ///     ExprView::Sum(terms) => {
    ///         println!("Converting sum with {} terms", terms.len());
    ///     }
    ///     ExprView::Number(n) => println!("Just a number: {}", n),
    ///     ExprView::Symbol(name) => println!("Variable: {}", name),
    ///     ExprView::Pow(base, exp) => println!("Power expression"),
    ///     _ => println!("Other expression type"),
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// This method is zero-cost for most expression types (returns references).
    /// Only polynomial expressions require allocation to expand into sum form.
    #[must_use]
    pub fn view(&self) -> crate::core::visitor::ExprView<'_> {
        use crate::core::visitor::ExprView;

        match &self.kind {
            ExprKind::Number(n) => ExprView::Number(*n),
            ExprKind::Symbol(s) => s.name().map_or_else(
                || ExprView::Symbol(Cow::Owned(s.to_string())),
                |name| ExprView::Symbol(Cow::Borrowed(name)),
            ),
            ExprKind::FunctionCall { name, args } => ExprView::Function {
                name: name.as_str(),
                args,
            },
            ExprKind::Sum(terms) => ExprView::Sum(Cow::Borrowed(terms)),
            ExprKind::Product(factors) => ExprView::Product(Cow::Borrowed(factors)),
            ExprKind::Div(l, r) => ExprView::Div(l, r),
            ExprKind::Pow(l, r) => ExprView::Pow(l, r),
            ExprKind::Derivative { inner, var, order } => ExprView::Derivative {
                inner,
                var: var.as_str(),
                order: *order,
            },
            // Poly is expanded to Sum for external API stability
            ExprKind::Poly(poly) => {
                let terms: Vec<Arc<Self>> =
                    poly.to_expr_terms().into_iter().map(Arc::new).collect();
                ExprView::Sum(Cow::Owned(terms))
            }
        }
    }

    // -------------------------------------------------------------------------
    // Analysis methods
    // -------------------------------------------------------------------------

    /// Count the total number of nodes in the AST
    #[must_use]
    pub fn node_count(&self) -> usize {
        match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => 1,
            ExprKind::FunctionCall { args, .. } => {
                1 + args.iter().map(|a| a.node_count()).sum::<usize>()
            }
            ExprKind::Sum(terms) => 1 + terms.iter().map(|t| t.node_count()).sum::<usize>(),
            ExprKind::Product(factors) => 1 + factors.iter().map(|f| f.node_count()).sum::<usize>(),
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => 1 + l.node_count() + r.node_count(),
            ExprKind::Derivative { inner, .. } => 1 + inner.node_count(),
            // Poly is counted as 1 node + its expanded form
            ExprKind::Poly(poly) => 1 + poly.terms().len(),
        }
    }

    /// Get the maximum nesting depth of the AST
    #[must_use]
    pub fn max_depth(&self) -> usize {
        match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => 1,
            ExprKind::FunctionCall { args, .. } => {
                1 + args.iter().map(|a| a.max_depth()).max().unwrap_or(0)
            }
            ExprKind::Sum(terms) => 1 + terms.iter().map(|t| t.max_depth()).max().unwrap_or(0),
            ExprKind::Product(factors) => {
                1 + factors.iter().map(|f| f.max_depth()).max().unwrap_or(0)
            }
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => 1 + l.max_depth().max(r.max_depth()),
            ExprKind::Derivative { inner, .. } => 1 + inner.max_depth(),
            ExprKind::Poly(_) => 2, // Poly is shallow: one level for poly, one for terms
        }
    }

    /// Check if the expression contains a specific variable (by symbol ID)
    #[inline]
    #[must_use]
    pub fn contains_var_id(&self, var_id: u64) -> bool {
        match &self.kind {
            ExprKind::Number(_) => false,
            ExprKind::Symbol(s) => s.id() == var_id,
            ExprKind::FunctionCall { args, .. } => args.iter().any(|a| a.contains_var_id(var_id)),
            ExprKind::Sum(terms) => terms.iter().any(|t| t.contains_var_id(var_id)),
            ExprKind::Product(factors) => factors.iter().any(|f| f.contains_var_id(var_id)),
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => {
                l.contains_var_id(var_id) || r.contains_var_id(var_id)
            }
            ExprKind::Derivative { inner, var: v, .. } => {
                // Compare derivative var symbol ID directly (O(1))
                v.id() == var_id || inner.contains_var_id(var_id)
            }
            ExprKind::Poly(poly) => poly.base().contains_var_id(var_id),
        }
    }

    /// Check if the expression contains a specific variable (by name)
    /// Uses ID comparison when possible, falls back to string matching
    #[inline]
    #[must_use]
    pub fn contains_var(&self, var: &str) -> bool {
        // Try to look up the symbol ID first for O(1) comparison
        crate::core::symbol::symb_get(var).map_or_else(
            |_| self.contains_var_str(var),
            |sym| self.contains_var_id(sym.id()),
        )
    }

    /// Check if the expression contains a specific variable (by name string match)
    /// This is used as a fallback when the symbol isn't in the global registry
    #[inline]
    fn contains_var_str(&self, var: &str) -> bool {
        match &self.kind {
            ExprKind::Number(_) => false,
            ExprKind::Symbol(s) => s.as_str() == var,
            ExprKind::FunctionCall { args, .. } => args.iter().any(|a| a.contains_var_str(var)),
            ExprKind::Sum(terms) => terms.iter().any(|t| t.contains_var_str(var)),
            ExprKind::Product(factors) => factors.iter().any(|f| f.contains_var_str(var)),
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => {
                l.contains_var_str(var) || r.contains_var_str(var)
            }
            ExprKind::Derivative { inner, var: v, .. } => {
                v.as_str() == var || inner.contains_var_str(var)
            }
            ExprKind::Poly(poly) => poly.base().contains_var_str(var),
        }
    }

    /// Check if the expression contains any free variables
    #[must_use]
    pub fn has_free_variables(&self, excluded: &HashSet<String>) -> bool {
        match &self.kind {
            ExprKind::Number(_) => false,
            ExprKind::Symbol(name) => !excluded.contains(name.as_ref()),
            ExprKind::Sum(terms) => terms.iter().any(|t| t.has_free_variables(excluded)),
            ExprKind::Product(factors) => factors.iter().any(|f| f.has_free_variables(excluded)),
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => {
                l.has_free_variables(excluded) || r.has_free_variables(excluded)
            }
            ExprKind::FunctionCall { args, .. } => {
                args.iter().any(|arg| arg.has_free_variables(excluded))
            }
            ExprKind::Derivative { inner, var, .. } => {
                !excluded.contains(var.as_str()) || inner.has_free_variables(excluded)
            }
            ExprKind::Poly(poly) => poly.base().has_free_variables(excluded),
        }
    }

    /// Collect all variables in the expression
    #[must_use]
    pub fn variables(&self) -> HashSet<String> {
        let mut vars = HashSet::new();
        self.collect_variables(&mut vars);
        vars
    }

    /// Collect all variable names used in this expression
    fn collect_variables(&self, vars: &mut HashSet<String>) {
        match &self.kind {
            ExprKind::Symbol(s) => {
                if let Some(name) = s.name() {
                    vars.insert(name.to_owned());
                }
            }
            ExprKind::FunctionCall { args, .. } => {
                for arg in args {
                    arg.collect_variables(vars);
                }
            }
            ExprKind::Sum(terms) => {
                for t in terms {
                    t.collect_variables(vars);
                }
            }
            ExprKind::Product(factors) => {
                for f in factors {
                    f.collect_variables(vars);
                }
            }
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => {
                l.collect_variables(vars);
                r.collect_variables(vars);
            }
            ExprKind::Derivative { inner, var, .. } => {
                vars.insert(var.as_str().to_owned());
                inner.collect_variables(vars);
            }
            ExprKind::Number(_) => {}
            ExprKind::Poly(poly) => {
                // Collect variables from the base expression
                poly.base().collect_variables(vars);
            }
        }
    }

    /// Create a deep clone (with new IDs)
    #[must_use]
    pub fn deep_clone(&self) -> Self {
        match &self.kind {
            ExprKind::Number(n) => Self::number(*n),
            ExprKind::Symbol(s) => Self::from_interned(s.clone()),
            ExprKind::FunctionCall { name, args } => Self::new(ExprKind::FunctionCall {
                name: name.clone(),
                args: args.iter().map(|arg| Arc::new(arg.deep_clone())).collect(),
            }),
            ExprKind::Sum(terms) => {
                let cloned: Vec<Arc<Self>> = terms
                    .iter()
                    .map(|t| Arc::new(t.as_ref().deep_clone()))
                    .collect();
                Self::new(ExprKind::Sum(cloned))
            }
            ExprKind::Product(factors) => {
                let cloned: Vec<Arc<Self>> = factors
                    .iter()
                    .map(|f| Arc::new(f.as_ref().deep_clone()))
                    .collect();
                Self::new(ExprKind::Product(cloned))
            }
            ExprKind::Div(a, b) => Self::div_expr(a.as_ref().deep_clone(), b.as_ref().deep_clone()),
            ExprKind::Pow(a, b) => {
                Self::pow_static(a.as_ref().deep_clone(), b.as_ref().deep_clone())
            }
            ExprKind::Derivative { inner, var, order } => {
                Self::derivative(inner.as_ref().deep_clone(), var.clone(), *order)
            }
            ExprKind::Poly(poly) => {
                // For performance, Poly is not recursively deep-cloned.
                // This is safe as Polynomial is designed to be immutable.
                Self::new(ExprKind::Poly(poly.clone()))
            }
        }
    }

    // -------------------------------------------------------------------------
    // Convenience methods
    // -------------------------------------------------------------------------

    /// Differentiate with respect to a variable
    ///
    /// # Errors
    /// Returns `DiffError` if differentiation fails (e.g., unsupported operation).
    pub fn diff(&self, var: &str) -> Result<Self, crate::DiffError> {
        crate::Diff::new().differentiate(self, &crate::symb(var))
    }

    /// Simplify this expression
    ///
    /// # Errors
    /// Returns `DiffError` if simplification fails.
    pub fn simplified(&self) -> Result<Self, crate::DiffError> {
        crate::Simplify::new().simplify(self)
    }

    /// Compile this expression for fast numerical evaluation
    ///
    /// Creates a compiled evaluator that can be reused for many evaluations.
    /// Much faster than `evaluate()` when evaluating the same expression
    /// at multiple points.
    ///
    /// # Example
    /// ```
    /// use symb_anafis::{Expr, parse};
    /// use std::collections::HashSet;
    /// let expr = parse("x^2 + 2*x", &HashSet::new(), &HashSet::new(), None).expect("Should parse");
    /// let evaluator = expr.compile().expect("Should compile");
    ///
    /// // Fast evaluation at multiple points
    /// let result_at_3 = evaluator.evaluate(&[3.0]); // 3^2 + 2*3 = 15
    /// assert!((result_at_3 - 15.0).abs() < 1e-10);
    /// ```
    ///
    /// # Errors
    /// Returns `DiffError` if the expression cannot be compiled.
    pub fn compile(&self) -> Result<crate::evaluator::CompiledEvaluator, crate::DiffError> {
        crate::evaluator::CompiledEvaluator::compile_auto(self, None)
    }

    /// Compile this expression with explicit parameter ordering
    ///
    /// Accepts `&[&str]`, `&[&Symbol]`, or any type implementing `ToParamName`.
    ///
    /// # Example
    /// ```
    /// use symb_anafis::symb;
    ///
    /// let x = symb("x");
    /// let y = symb("y");
    /// let expr = x.pow(2.0) + y;
    ///
    /// // Using strings
    /// let compiled = expr.compile_with_params(&["x", "y"]).expect("Should compile");
    ///
    /// // Using symbols
    /// let compiled = expr.compile_with_params(&[&x, &y]).expect("Should compile");
    /// ```
    ///
    /// # Errors
    /// Returns `DiffError` if the expression cannot be compiled.
    pub fn compile_with_params<P: crate::evaluator::ToParamName>(
        &self,
        param_order: &[P],
    ) -> Result<crate::evaluator::CompiledEvaluator, crate::DiffError> {
        crate::evaluator::CompiledEvaluator::compile(self, param_order, None)
    }

    /// Fold over the expression tree (pre-order)
    pub fn fold<T, F>(&self, init: T, f: F) -> T
    where
        F: Fn(T, &Self) -> T + Copy,
    {
        let acc = f(init, self);
        match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) | ExprKind::Poly(_) => acc, // Poly is opaque for folding
            ExprKind::FunctionCall { args, .. } => args.iter().fold(acc, |a, arg| arg.fold(a, f)),
            ExprKind::Sum(terms) => terms.iter().fold(acc, |a, t| t.fold(a, f)),
            ExprKind::Product(factors) => factors.iter().fold(acc, |a, f_| f_.fold(a, f)),
            ExprKind::Div(l, r) | ExprKind::Pow(l, r) => {
                let acc = l.fold(acc, f);
                r.fold(acc, f)
            }
            ExprKind::Derivative { inner, .. } => inner.fold(acc, f),
        }
    }

    /// Transform the expression tree (post-order)
    #[must_use]
    pub fn map<F>(&self, f: F) -> Self
    where
        F: Fn(&Self) -> Self + Copy,
    {
        let transformed = match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => self.clone(),
            ExprKind::FunctionCall { name, args } => Self::new(ExprKind::FunctionCall {
                name: name.clone(),
                args: args.iter().map(|arg| Arc::new(arg.map(f))).collect(),
            }),
            ExprKind::Sum(terms) => {
                let mapped: Vec<Arc<Self>> =
                    terms.iter().map(|t| Arc::new(t.as_ref().map(f))).collect();
                Self::new(ExprKind::Sum(mapped))
            }
            ExprKind::Product(factors) => {
                let mapped: Vec<Arc<Self>> = factors
                    .iter()
                    .map(|fac| Arc::new(fac.as_ref().map(f)))
                    .collect();
                Self::new(ExprKind::Product(mapped))
            }
            ExprKind::Div(a, b) => Self::div_expr(a.map(f), b.map(f)),
            ExprKind::Pow(a, b) => Self::pow_static(a.map(f), b.map(f)),
            ExprKind::Derivative { inner, var, order } => {
                Self::derivative(inner.map(f), var.clone(), *order)
            }
            ExprKind::Poly(poly) => {
                // Poly is opaque for mapping - just clone
                Self::new(ExprKind::Poly(poly.clone()))
            }
        };
        f(&transformed)
    }

    /// Substitute a variable with another expression
    #[must_use]
    pub fn substitute(&self, var: &str, replacement: &Self) -> Self {
        let var_id = crate::core::symbol::symb_interned(var).id();
        self.map(|node| {
            if let ExprKind::Symbol(s) = &node.kind
                && s.id() == var_id
            {
                return replacement.clone();
            }
            node.clone()
        })
    }
}
