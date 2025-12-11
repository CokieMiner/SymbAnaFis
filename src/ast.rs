//! Abstract Syntax Tree for mathematical expressions

use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::symbol::{InternedSymbol, get_or_intern};

/// Type alias for custom evaluation functions map (reduces type complexity)
pub type CustomEvalMap =
    std::collections::HashMap<String, std::sync::Arc<dyn Fn(&[f64]) -> Option<f64> + Send + Sync>>;

/// Global counter for expression IDs
static EXPR_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_id() -> u64 {
    EXPR_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone)]
pub struct Expr {
    /// Unique ID for debugging (not used in equality comparisons)
    pub id: u64,
    pub kind: ExprKind,
}

impl Deref for Expr {
    type Target = ExprKind;

    fn deref(&self) -> &Self::Target {
        &self.kind
    }
}

// Implement Eq and Hash based on KIND only for structural equality
impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl Eq for Expr {}

impl std::hash::Hash for Expr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// Constant number (e.g., 3.14, 1e10)
    Number(f64),

    /// Variable or constant symbol (e.g., "x", "a", "ax")
    /// Uses InternedSymbol for O(1) equality comparisons
    Symbol(InternedSymbol),

    /// Function call (built-in or custom)
    FunctionCall { name: String, args: Vec<Expr> },

    // Binary operations
    /// Addition
    Add(Arc<Expr>, Arc<Expr>),

    /// Subtraction
    Sub(Arc<Expr>, Arc<Expr>),

    /// Multiplication
    Mul(Arc<Expr>, Arc<Expr>),

    /// Division
    Div(Arc<Expr>, Arc<Expr>),

    /// Exponentiation
    Pow(Arc<Expr>, Arc<Expr>),

    /// Partial derivative notation: ∂^order/∂var^order of inner expression
    /// Used for representing derivatives of unknown/custom functions
    Derivative {
        inner: Arc<Expr>,
        var: String,
        order: u32,
    },
}

impl Expr {
    pub fn new(kind: ExprKind) -> Self {
        Expr {
            id: next_id(),
            kind,
        }
    }

    // Accessor methods

    /// Check if expression is a constant number and return its value
    ///
    /// # Example
    /// ```ignore
    /// let expr = Expr::number(3.14);
    /// assert_eq!(expr.as_number(), Some(3.14));
    ///
    /// let sym = Expr::symbol("x");
    /// assert_eq!(sym.as_number(), None);
    /// ```
    pub fn as_number(&self) -> Option<f64> {
        match &self.kind {
            ExprKind::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Check if this expression is the number zero (with tolerance)
    ///
    /// Uses `FLOAT_TOLERANCE` to handle floating-point precision issues.
    #[inline]
    pub fn is_zero_num(&self) -> bool {
        self.as_number().is_some_and(crate::traits::is_zero)
    }

    /// Check if this expression is the number one (with tolerance)
    ///
    /// Uses `FLOAT_TOLERANCE` to handle floating-point precision issues.
    #[inline]
    pub fn is_one_num(&self) -> bool {
        self.as_number().is_some_and(crate::traits::is_one)
    }

    /// Check if this expression is the number negative one (with tolerance)
    ///
    /// Uses `FLOAT_TOLERANCE` to handle floating-point precision issues.
    #[inline]
    pub fn is_neg_one_num(&self) -> bool {
        self.as_number().is_some_and(crate::traits::is_neg_one)
    }

    // Convenience constructors

    /// Create a number expression
    pub fn number(n: f64) -> Self {
        Expr::new(ExprKind::Number(n))
    }

    /// Create a symbol expression
    ///
    /// The symbol name is automatically interned for O(1) comparisons.
    pub fn symbol(s: impl AsRef<str>) -> Self {
        Expr::new(ExprKind::Symbol(get_or_intern(s.as_ref())))
    }

    /// Create a symbol expression from an already-interned symbol
    pub(crate) fn from_interned(interned: InternedSymbol) -> Self {
        Expr::new(ExprKind::Symbol(interned))
    }

    /// Create an addition expression
    pub fn add_expr(left: Expr, right: Expr) -> Self {
        Expr::new(ExprKind::Add(Arc::new(left), Arc::new(right)))
    }

    /// Create a subtraction expression
    pub fn sub_expr(left: Expr, right: Expr) -> Self {
        Expr::new(ExprKind::Sub(Arc::new(left), Arc::new(right)))
    }

    /// Create a multiplication expression
    pub fn mul_expr(left: Expr, right: Expr) -> Self {
        Expr::new(ExprKind::Mul(Arc::new(left), Arc::new(right)))
    }

    /// Create a division expression
    pub fn div_expr(left: Expr, right: Expr) -> Self {
        Expr::new(ExprKind::Div(Arc::new(left), Arc::new(right)))
    }

    /// Create a power expression
    pub fn pow(base: Expr, exponent: Expr) -> Self {
        Expr::new(ExprKind::Pow(Arc::new(base), Arc::new(exponent)))
    }

    /// Create a function call expression (single argument convenience)
    pub fn func(name: impl Into<String>, content: Expr) -> Self {
        Expr::new(ExprKind::FunctionCall {
            name: name.into(),
            args: vec![content],
        })
    }

    /// Create a multi-argument function call expression
    pub fn func_multi(name: impl Into<String>, args: Vec<Expr>) -> Self {
        Expr::new(ExprKind::FunctionCall {
            name: name.into(),
            args,
        })
    }

    /// Create a function call with explicit arguments using array syntax
    ///
    /// This is the ergonomic way to create n-argument function calls.
    ///
    /// # Example
    /// ```ignore
    /// use symb_anafis::{Expr, sym};
    ///
    /// let x = sym("x");
    /// let y = sym("y");
    ///
    /// // Single arg
    /// Expr::call("f", [x.clone().into()]);        // → f(x)
    ///
    /// // Multiple args
    /// Expr::call("G", [x.clone().into(), y.into()]);  // → G(x, y)
    ///
    /// // Complex expressions as arguments
    /// let z = x.clone() + y.clone();
    /// Expr::call("H", [z, x.into()]);             // → H(x+y, x)
    /// ```
    pub fn call<const N: usize>(name: impl Into<String>, args: [Expr; N]) -> Self {
        Expr::func_multi(name, args.into())
    }

    /// Create a partial derivative expression
    pub fn derivative(inner: Expr, var: String, order: u32) -> Self {
        Expr::new(ExprKind::Derivative {
            inner: Arc::new(inner),
            var,
            order,
        })
    }

    // Analysis methods

    /// Count the total number of nodes in the AST
    pub fn node_count(&self) -> usize {
        match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => 1,
            ExprKind::FunctionCall { args, .. } => {
                1 + args.iter().map(|a| a.node_count()).sum::<usize>()
            }
            ExprKind::Add(l, r)
            | ExprKind::Sub(l, r)
            | ExprKind::Mul(l, r)
            | ExprKind::Div(l, r)
            | ExprKind::Pow(l, r) => 1 + l.node_count() + r.node_count(),
            ExprKind::Derivative { inner, .. } => 1 + inner.node_count(),
        }
    }

    /// Get the maximum nesting depth of the AST
    pub fn max_depth(&self) -> usize {
        match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => 1,
            ExprKind::FunctionCall { args, .. } => {
                1 + args.iter().map(|a| a.max_depth()).max().unwrap_or(0)
            }
            ExprKind::Add(l, r)
            | ExprKind::Sub(l, r)
            | ExprKind::Mul(l, r)
            | ExprKind::Div(l, r)
            | ExprKind::Pow(l, r) => 1 + l.max_depth().max(r.max_depth()),
            ExprKind::Derivative { inner, .. } => 1 + inner.max_depth(),
        }
    }

    /// Check if the expression contains a specific variable
    pub fn contains_var(&self, var: &str) -> bool {
        match &self.kind {
            ExprKind::Number(_) => false,
            ExprKind::Symbol(s) => s == var,
            ExprKind::FunctionCall { args, .. } => args.iter().any(|a| a.contains_var(var)),
            ExprKind::Add(l, r)
            | ExprKind::Sub(l, r)
            | ExprKind::Mul(l, r)
            | ExprKind::Div(l, r)
            | ExprKind::Pow(l, r) => l.contains_var(var) || r.contains_var(var),
            ExprKind::Derivative { inner, var: v, .. } => v == var || inner.contains_var(var),
        }
    }

    /// Check if the expression contains any free variables (symbols not in the excluded set)
    ///
    /// This is useful for determining if an expression is "constant" with respect to
    /// a set of fixed variables during differentiation.
    ///
    /// # Arguments
    /// * `excluded` - Set of symbol names to treat as constants (not free variables)
    ///
    /// # Example
    /// ```ignore
    /// let fixed = vec!["a".to_string(), "b".to_string()].into_iter().collect();
    /// let expr = parse("a * x + b", ...)?;
    /// assert!(expr.has_free_variables(&fixed));  // x is free
    ///
    /// let expr2 = parse("a * b", ...)?;
    /// assert!(!expr2.has_free_variables(&fixed)); // all symbols are in excluded set
    /// ```
    pub fn has_free_variables(&self, excluded: &std::collections::HashSet<String>) -> bool {
        match &self.kind {
            ExprKind::Number(_) => false,
            ExprKind::Symbol(name) => !excluded.contains(name.as_ref()),
            ExprKind::Add(u, v)
            | ExprKind::Sub(u, v)
            | ExprKind::Mul(u, v)
            | ExprKind::Div(u, v)
            | ExprKind::Pow(u, v) => {
                u.has_free_variables(excluded) || v.has_free_variables(excluded)
            }
            ExprKind::FunctionCall { args, .. } => {
                args.iter().any(|arg| arg.has_free_variables(excluded))
            }
            ExprKind::Derivative { inner, var, .. } => {
                !excluded.contains(var) || inner.has_free_variables(excluded)
            }
        }
    }

    /// Collect all variables in the expression
    pub fn variables(&self) -> std::collections::HashSet<String> {
        let mut vars = std::collections::HashSet::new();
        self.collect_variables(&mut vars);
        vars
    }

    fn collect_variables(&self, vars: &mut std::collections::HashSet<String>) {
        match &self.kind {
            ExprKind::Symbol(s) => {
                if let Some(name) = s.name() {
                    vars.insert(name.to_string());
                }
            }
            ExprKind::FunctionCall { args, .. } => {
                for arg in args {
                    arg.collect_variables(vars);
                }
            }
            ExprKind::Add(l, r)
            | ExprKind::Sub(l, r)
            | ExprKind::Mul(l, r)
            | ExprKind::Div(l, r)
            | ExprKind::Pow(l, r) => {
                l.collect_variables(vars);
                r.collect_variables(vars);
            }
            ExprKind::Derivative { inner, var, .. } => {
                vars.insert(var.clone());
                inner.collect_variables(vars);
            }
            ExprKind::Number(_) => {}
        }
    }

    /// Create a deep clone of the expression tree (no shared nodes)
    /// Note: This generates NEW IDs for everything
    pub fn deep_clone(&self) -> Expr {
        match &self.kind {
            ExprKind::Number(n) => Expr::number(*n),
            ExprKind::Symbol(s) => Expr::from_interned(s.clone()),
            ExprKind::FunctionCall { name, args } => Expr::new(ExprKind::FunctionCall {
                name: name.clone(),
                args: args.iter().map(|arg| arg.deep_clone()).collect(),
            }),
            ExprKind::Add(a, b) => Expr::add_expr(a.as_ref().deep_clone(), b.as_ref().deep_clone()),
            ExprKind::Sub(a, b) => Expr::sub_expr(a.as_ref().deep_clone(), b.as_ref().deep_clone()),
            ExprKind::Mul(a, b) => Expr::mul_expr(a.as_ref().deep_clone(), b.as_ref().deep_clone()),
            ExprKind::Div(a, b) => Expr::div_expr(a.as_ref().deep_clone(), b.as_ref().deep_clone()),
            ExprKind::Pow(a, b) => Expr::pow(a.as_ref().deep_clone(), b.as_ref().deep_clone()),
            ExprKind::Derivative { inner, var, order } => {
                Expr::derivative(inner.as_ref().deep_clone(), var.clone(), *order)
            }
        }
    }

    // Convenience methods for ergonomic API

    /// Differentiate this expression with respect to a variable (convenience wrapper)
    ///
    /// This is a shorthand for `Diff::new().differentiate(expr, &Symbol::new(var))`.
    /// For more control over differentiation options, use the `Diff` builder directly.
    ///
    /// # Example
    /// ```ignore
    /// use symb_anafis::{Expr, sym};
    /// let x = sym("x");
    /// let expr = x.clone().pow(2.0);
    /// let derivative = expr.diff("x").unwrap();  // Returns 2x
    /// ```
    pub fn diff(&self, var: &str) -> Result<Expr, crate::DiffError> {
        crate::Diff::new().differentiate(self.clone(), &crate::sym(var))
    }

    /// Simplify this expression (convenience wrapper)
    ///
    /// This is a shorthand for `Simplify::new().simplify(expr)`.
    /// For more control over simplification options, use the `Simplify` builder directly.
    ///
    /// # Example
    /// ```ignore
    /// use symb_anafis::{Expr, sym};
    /// let x = sym("x");
    /// let expr = x.clone() + x.clone();  // x + x
    /// let simplified = expr.simplified().unwrap();  // Returns 2x
    /// ```
    pub fn simplified(&self) -> Result<Expr, crate::DiffError> {
        crate::Simplify::new().simplify(self.clone())
    }

    /// Fold over the expression tree, visiting each node
    /// The folder function receives the accumulated value and a reference to each node
    /// Nodes are visited in pre-order (parent before children)
    ///
    /// # Example
    /// ```ignore
    /// // Count all nodes
    /// let count = expr.fold(0, |acc, _node| acc + 1);
    ///
    /// // Find max depth
    /// let depth = expr.fold_with_depth(0, |max_d, _node, depth| max_d.max(depth));
    /// ```
    pub fn fold<T, F>(&self, init: T, f: F) -> T
    where
        F: Fn(T, &Expr) -> T + Copy,
    {
        let acc = f(init, self);
        match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => acc,
            ExprKind::FunctionCall { args, .. } => args.iter().fold(acc, |a, arg| arg.fold(a, f)),
            ExprKind::Add(l, r)
            | ExprKind::Sub(l, r)
            | ExprKind::Mul(l, r)
            | ExprKind::Div(l, r)
            | ExprKind::Pow(l, r) => {
                let acc = l.fold(acc, f);
                r.fold(acc, f)
            }
            ExprKind::Derivative { inner, .. } => inner.fold(acc, f),
        }
    }

    /// Transform the expression tree by applying a function to each node
    /// The transformer receives a reference to each node and returns a new expression
    /// Nodes are visited in post-order (children before parent)
    ///
    /// # Example
    /// ```ignore
    /// // Replace all x with y
    /// let transformed = expr.map(|node| {
    ///     if let ExprKind::Symbol(s) = &node.kind {
    ///         if s == "x" { return Expr::symbol("y"); }
    ///     }
    ///     node.clone()
    /// });
    /// ```
    pub fn map<F>(&self, f: F) -> Expr
    where
        F: Fn(&Expr) -> Expr + Copy,
    {
        let transformed = match &self.kind {
            ExprKind::Number(_) | ExprKind::Symbol(_) => self.clone(),
            ExprKind::FunctionCall { name, args } => Expr::new(ExprKind::FunctionCall {
                name: name.clone(),
                args: args.iter().map(|arg| arg.map(f)).collect(),
            }),
            ExprKind::Add(a, b) => Expr::add_expr(a.map(f), b.map(f)),
            ExprKind::Sub(a, b) => Expr::sub_expr(a.map(f), b.map(f)),
            ExprKind::Mul(a, b) => Expr::mul_expr(a.map(f), b.map(f)),
            ExprKind::Div(a, b) => Expr::div_expr(a.map(f), b.map(f)),
            ExprKind::Pow(a, b) => Expr::pow(a.map(f), b.map(f)),
            ExprKind::Derivative { inner, var, order } => {
                Expr::derivative(inner.map(f), var.clone(), *order)
            }
        };
        f(&transformed)
    }

    /// Substitute a variable with another expression
    ///
    /// # Example
    /// ```ignore
    /// let expr = parse("x * y", ...)?;
    /// let result = expr.substitute("x", &Expr::number(3.0));
    /// // result is 3 * y
    /// ```
    pub fn substitute(&self, var: &str, replacement: &Expr) -> Expr {
        self.map(|node| {
            if let ExprKind::Symbol(s) = &node.kind
                && s == var
            {
                return replacement.clone();
            }
            node.clone()
        })
    }

    /// Evaluate expression with given variable values
    /// Performs partial evaluation - substitutes known values and simplifies numerically
    /// Returns a Number if all variables are defined, otherwise returns Expr with remaining symbols
    ///
    /// # Example
    /// ```ignore
    /// use std::collections::HashMap;
    /// let expr = parse("x * y + 1", ...)?;
    ///
    /// // Partial evaluation: x=3 -> 3*y + 1
    /// let partial: HashMap<&str, f64> = [("x", 3.0)].into_iter().collect();
    /// let result = expr.evaluate(&partial);
    ///
    /// // Full evaluation: x=3, y=2 -> 7
    /// let full: HashMap<&str, f64> = [("x", 3.0), ("y", 2.0)].into_iter().collect();
    /// let result = expr.evaluate(&full);
    /// ```
    pub fn evaluate(&self, vars: &std::collections::HashMap<&str, f64>) -> Expr {
        self.evaluate_with_custom(vars, &std::collections::HashMap::new())
    }

    /// Evaluate expression with given variable values and custom function evaluators
    ///
    /// Custom evaluators take precedence over built-in functions.
    ///
    /// # Example
    /// ```ignore
    /// use std::collections::HashMap;
    /// use std::sync::Arc;
    ///
    /// let expr = Expr::func("f", Expr::symbol("x"));
    /// let vars: HashMap<&str, f64> = [("x", 3.0)].into_iter().collect();
    ///
    /// // f(x) = x² + 1
    /// let mut custom_evals = HashMap::new();
    /// custom_evals.insert("f".to_string(), Arc::new(|args: &[f64]| Some(args[0].powi(2) + 1.0)));
    ///
    /// let result = expr.evaluate_with_custom(&vars, &custom_evals);
    /// // result = 10 (since f(3) = 3² + 1 = 10)
    /// ```
    pub fn evaluate_with_custom(
        &self,
        vars: &std::collections::HashMap<&str, f64>,
        custom_evals: &CustomEvalMap,
    ) -> Expr {
        match &self.kind {
            ExprKind::Number(n) => Expr::number(*n),
            ExprKind::Symbol(s) => {
                if let Some(name) = s.name()
                    && let Some(&val) = vars.get(name)
                {
                    return Expr::number(val);
                }
                self.clone()
            }
            ExprKind::FunctionCall { name, args } => {
                let eval_args: Vec<Expr> = args
                    .iter()
                    .map(|a| a.evaluate_with_custom(vars, custom_evals))
                    .collect();

                // Try to evaluate if all args are numeric
                let numeric_args: Option<Vec<f64>> = eval_args
                    .iter()
                    .map(|e| {
                        if let ExprKind::Number(n) = &e.kind {
                            Some(*n)
                        } else {
                            None
                        }
                    })
                    .collect();

                if let Some(args_vec) = numeric_args {
                    // Check custom evaluators FIRST
                    if let Some(custom_eval) = custom_evals.get(name)
                        && let Some(result) = custom_eval(&args_vec)
                    {
                        return Expr::number(result);
                    }
                    // Then check built-in registry
                    if let Some(func_def) = crate::functions::registry::Registry::get(name)
                        && let Some(result) = (func_def.eval)(&args_vec)
                    {
                        return Expr::number(result);
                    }
                }

                // Unknown function or non-numeric args: keep function, use evaluated args
                Expr::new(ExprKind::FunctionCall {
                    name: name.clone(),
                    args: eval_args,
                })
            }
            ExprKind::Add(a, b) => {
                let ea = a.evaluate_with_custom(vars, custom_evals);
                let eb = b.evaluate_with_custom(vars, custom_evals);
                match (&ea.kind, &eb.kind) {
                    (ExprKind::Number(x), ExprKind::Number(y)) => Expr::number(x + y),
                    _ => Expr::add_expr(ea, eb),
                }
            }
            ExprKind::Sub(a, b) => {
                let ea = a.evaluate_with_custom(vars, custom_evals);
                let eb = b.evaluate_with_custom(vars, custom_evals);
                match (&ea.kind, &eb.kind) {
                    (ExprKind::Number(x), ExprKind::Number(y)) => Expr::number(x - y),
                    _ => Expr::sub_expr(ea, eb),
                }
            }
            ExprKind::Mul(a, b) => {
                let ea = a.evaluate_with_custom(vars, custom_evals);
                let eb = b.evaluate_with_custom(vars, custom_evals);
                match (&ea.kind, &eb.kind) {
                    (ExprKind::Number(x), ExprKind::Number(y)) => Expr::number(x * y),
                    _ => Expr::mul_expr(ea, eb),
                }
            }
            ExprKind::Div(a, b) => {
                let ea = a.evaluate_with_custom(vars, custom_evals);
                let eb = b.evaluate_with_custom(vars, custom_evals);
                match (&ea.kind, &eb.kind) {
                    (ExprKind::Number(x), ExprKind::Number(y)) if *y != 0.0 => Expr::number(x / y),
                    _ => Expr::div_expr(ea, eb),
                }
            }
            // Note: 0^0 evaluates to 1.0 following IEEE 754 powf behavior
            ExprKind::Pow(a, b) => {
                let ea = a.evaluate_with_custom(vars, custom_evals);
                let eb = b.evaluate_with_custom(vars, custom_evals);
                match (&ea.kind, &eb.kind) {
                    (ExprKind::Number(x), ExprKind::Number(y)) => Expr::number(x.powf(*y)),
                    _ => Expr::pow(ea, eb),
                }
            }
            ExprKind::Derivative { inner, var, order } => {
                // Derivative expressions cannot be further evaluated - just evaluate the inner
                Expr::derivative(
                    inner.evaluate_with_custom(vars, custom_evals),
                    var.clone(),
                    *order,
                )
            }
        }
    }
}
// Manual Hash implementation for ExprKind
// We need this for HashSet<Expr> in cycle detection
impl std::hash::Hash for ExprKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            ExprKind::Number(n) => {
                // Hash the bit representation of f64
                // NaN values will hash the same way
                n.to_bits().hash(state);
            }
            ExprKind::Symbol(s) => s.hash(state),
            ExprKind::FunctionCall { name, args } => {
                name.hash(state);
                args.hash(state);
            }
            ExprKind::Add(l, r)
            | ExprKind::Sub(l, r)
            | ExprKind::Mul(l, r)
            | ExprKind::Div(l, r)
            | ExprKind::Pow(l, r) => {
                l.hash(state);
                r.hash(state);
            }
            ExprKind::Derivative { inner, var, order } => {
                inner.hash(state);
                var.hash(state);
                order.hash(state);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructors() {
        let val = 314.0 / 100.0;
        let num = Expr::number(val);
        match &num.kind {
            ExprKind::Number(n) => assert_eq!(*n, val),
            _ => panic!("Expected Number variant"),
        }

        let sym = Expr::symbol("x");
        match &sym.kind {
            ExprKind::Symbol(s) => assert_eq!(s, "x"),
            _ => panic!("Expected Symbol variant"),
        }

        let add = Expr::add_expr(Expr::number(1.0), Expr::number(2.0));
        match &add.kind {
            ExprKind::Add(_, _) => (),
            _ => panic!("Expected Add variant"),
        }
    }

    #[test]
    fn test_ids() {
        let e1 = Expr::number(1.0);
        let e2 = Expr::number(1.0);
        let e3 = Expr::number(2.0);

        assert_ne!(e1.id, e2.id); // IDs must be unique
        assert_eq!(e1, e2); // Structural equality should pass
        assert_ne!(e1, e3); // Different values
    }

    #[test]
    fn test_node_count() {
        let x = Expr::symbol("x");
        assert_eq!(x.node_count(), 1);

        let x_plus_1 = Expr::add_expr(Expr::symbol("x"), Expr::number(1.0));
        assert_eq!(x_plus_1.node_count(), 3); // Add + x + 1

        let complex = Expr::mul_expr(
            Expr::add_expr(Expr::symbol("x"), Expr::number(1.0)),
            Expr::symbol("y"),
        );
        assert_eq!(complex.node_count(), 5); // Mul + (Add + x + 1) + y
    }

    #[test]
    fn test_max_depth() {
        let x = Expr::symbol("x");
        assert_eq!(x.max_depth(), 1);

        let nested = Expr::add_expr(
            Expr::mul_expr(Expr::symbol("x"), Expr::symbol("y")),
            Expr::number(1.0),
        );
        assert_eq!(nested.max_depth(), 3); // Add -> Mul -> x/y
    }

    #[test]
    fn test_contains_var() {
        let expr = Expr::add_expr(
            Expr::mul_expr(Expr::symbol("x"), Expr::symbol("y")),
            Expr::number(1.0),
        );

        assert!(expr.contains_var("x"));
        assert!(expr.contains_var("y"));
        assert!(!expr.contains_var("z"));
    }
}
