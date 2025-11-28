/// Abstract Syntax Tree for mathematical expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Constant number (e.g., 3.14, 1e10)
    Number(f64),

    /// Variable or constant symbol (e.g., "x", "a", "ax")
    Symbol(String),

    /// Function call (built-in or custom)
    FunctionCall { name: String, args: Vec<Expr> },

    // Binary operations
    /// Addition
    Add(Box<Expr>, Box<Expr>),

    /// Subtraction
    Sub(Box<Expr>, Box<Expr>),

    /// Multiplication
    Mul(Box<Expr>, Box<Expr>),

    /// Division
    Div(Box<Expr>, Box<Expr>),

    /// Exponentiation
    Pow(Box<Expr>, Box<Expr>),
}

impl Expr {
    // Convenience constructors

    /// Create a number expression
    pub fn number(n: f64) -> Self {
        Expr::Number(n)
    }

    /// Create a symbol expression
    pub fn symbol(s: impl Into<String>) -> Self {
        Expr::Symbol(s.into())
    }

    /// Create an addition expression
    pub fn add_expr(left: Expr, right: Expr) -> Self {
        Expr::Add(Box::new(left), Box::new(right))
    }

    /// Create a subtraction expression
    pub fn sub_expr(left: Expr, right: Expr) -> Self {
        Expr::Sub(Box::new(left), Box::new(right))
    }

    /// Create a multiplication expression
    pub fn mul_expr(left: Expr, right: Expr) -> Self {
        Expr::Mul(Box::new(left), Box::new(right))
    }

    /// Create a division expression
    pub fn div_expr(left: Expr, right: Expr) -> Self {
        Expr::Div(Box::new(left), Box::new(right))
    }

    /// Create a power expression
    pub fn pow(base: Expr, exponent: Expr) -> Self {
        Expr::Pow(Box::new(base), Box::new(exponent))
    }

    /// Create a function call expression (single argument convenience)
    pub fn func(name: impl Into<String>, content: Expr) -> Self {
        Expr::FunctionCall {
            name: name.into(),
            args: vec![content],
        }
    }

    /// Create a multi-argument function call expression
    pub fn func_multi(name: impl Into<String>, args: Vec<Expr>) -> Self {
        Expr::FunctionCall {
            name: name.into(),
            args,
        }
    }

    // Analysis methods

    /// Count the total number of nodes in the AST
    pub fn node_count(&self) -> usize {
        match self {
            Expr::Number(_) | Expr::Symbol(_) => 1,
            Expr::FunctionCall { args, .. } => {
                1 + args.iter().map(|a| a.node_count()).sum::<usize>()
            }
            Expr::Add(l, r)
            | Expr::Sub(l, r)
            | Expr::Mul(l, r)
            | Expr::Div(l, r)
            | Expr::Pow(l, r) => 1 + l.node_count() + r.node_count(),
        }
    }

    /// Get the maximum nesting depth of the AST
    pub fn max_depth(&self) -> usize {
        match self {
            Expr::Number(_) | Expr::Symbol(_) => 1,
            Expr::FunctionCall { args, .. } => {
                1 + args.iter().map(|a| a.max_depth()).max().unwrap_or(0)
            }
            Expr::Add(l, r)
            | Expr::Sub(l, r)
            | Expr::Mul(l, r)
            | Expr::Div(l, r)
            | Expr::Pow(l, r) => 1 + l.max_depth().max(r.max_depth()),
        }
    }

    /// Check if the expression contains a specific variable
    pub fn contains_var(&self, var: &str) -> bool {
        match self {
            Expr::Number(_) => false,
            Expr::Symbol(s) => s == var,
            Expr::FunctionCall { args, .. } => args.iter().any(|a| a.contains_var(var)),
            Expr::Add(l, r)
            | Expr::Sub(l, r)
            | Expr::Mul(l, r)
            | Expr::Div(l, r)
            | Expr::Pow(l, r) => l.contains_var(var) || r.contains_var(var),
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
        assert_eq!(num, Expr::Number(val));

        let sym = Expr::symbol("x");
        assert_eq!(sym, Expr::Symbol("x".to_string()));

        let add = Expr::add_expr(Expr::number(1.0), Expr::number(2.0));
        match add {
            Expr::Add(_, _) => (),
            _ => panic!("Expected Add variant"),
        }
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
