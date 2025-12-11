//! Display implementations for expressions
//!
//! # Display Behavior Notes
//! - `e^x` is always displayed as `exp(x)` for consistency and clarity
//! - Derivatives use notation like `∂^n_inner/∂_var^n` for higher-order derivatives
//! - Implicit multiplication is used where unambiguous (e.g., `2x` instead of `2*x`)

use crate::{Expr, ExprKind};
use std::fmt;
/// Check if an expression is negative (has a leading -1 coefficient)
/// Returns Some(inner) if the expression is -1 * inner, None otherwise
fn extract_negative(expr: &Expr) -> Option<Expr> {
    if let ExprKind::Mul(left, right) = &expr.kind {
        // Direct -1 * x pattern (using tolerance for float comparison)
        if let ExprKind::Number(n) = &left.kind
            && (*n + 1.0).abs() < 1e-10
        {
            return Some(right.as_ref().clone());
        }
        // Nested: (-1 * a) * b = -(a * b)
        if let Some(inner_left) = extract_negative(left) {
            return Some(Expr::mul_expr(inner_left, right.as_ref().clone()));
        }
    }
    None
}

/// Check if an expression starts with a function call or exp() (for nested muls)
fn starts_with_function_or_exp(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::FunctionCall { .. } => true,
        // e^x displays as exp(x), so treat it like a function
        ExprKind::Pow(base, _) => {
            if let ExprKind::Symbol(s) = &base.kind {
                s == "e"
            } else {
                false
            }
        }
        ExprKind::Mul(left, _) => starts_with_function_or_exp(left),
        _ => false,
    }
}

/// Check if we need explicit * between two expressions
fn needs_explicit_mul(left: &Expr, right: &Expr) -> bool {
    // Number * Number always needs explicit *
    if matches!(left.kind, ExprKind::Number(_)) && matches!(right.kind, ExprKind::Number(_)) {
        return true;
    }
    // Number * (something that starts with number) needs *
    if matches!(left.kind, ExprKind::Number(_)) {
        if let ExprKind::Mul(inner_left, _) = &right.kind
            && matches!(inner_left.kind, ExprKind::Number(_))
        {
            return true;
        }
        if matches!(right.kind, ExprKind::Div(_, _)) {
            return true;
        }
    }
    // Symbol * Symbol needs explicit * for readability: alpha*t not alphat
    if matches!(left.kind, ExprKind::Symbol(_)) && matches!(right.kind, ExprKind::Symbol(_)) {
        return true;
    }
    // Symbol * Number needs explicit *: x*2 not x2
    if matches!(left.kind, ExprKind::Symbol(_)) && matches!(right.kind, ExprKind::Number(_)) {
        return true;
    }
    // Symbol * Pow needs explicit *: pi*sigma^2 not pisigma^2
    if matches!(left.kind, ExprKind::Symbol(_)) && matches!(right.kind, ExprKind::Pow(_, _)) {
        return true;
    }
    // Symbol/Pow * FunctionCall or exp() (or mul starting with function/exp) needs explicit *
    if matches!(left.kind, ExprKind::Symbol(_) | ExprKind::Pow(_, _))
        && (matches!(right.kind, ExprKind::FunctionCall { .. })
            || starts_with_function_or_exp(right))
    {
        return true;
    }
    // Pow * Symbol needs explicit *: x^2*y not x^2y
    if matches!(left.kind, ExprKind::Pow(_, _)) && matches!(right.kind, ExprKind::Symbol(_)) {
        return true;
    }
    // FunctionCall * anything needs explicit *
    if matches!(left.kind, ExprKind::FunctionCall { .. }) {
        return true;
    }
    // Nested mul ending in symbol/pow * symbol, pow, or function call or exp
    if let ExprKind::Mul(_, inner_right) = &left.kind {
        if matches!(inner_right.kind, ExprKind::Symbol(_) | ExprKind::Pow(_, _))
            && (matches!(
                right.kind,
                ExprKind::Symbol(_) | ExprKind::Pow(_, _) | ExprKind::FunctionCall { .. }
            ) || starts_with_function_or_exp(right))
        {
            return true;
        }
        if matches!(inner_right.kind, ExprKind::FunctionCall { .. }) {
            return true;
        }
    }
    false
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ExprKind::Number(n) => {
                if n.is_nan() {
                    write!(f, "NaN")
                } else if n.is_infinite() {
                    if *n > 0.0 {
                        write!(f, "Infinity")
                    } else {
                        write!(f, "-Infinity")
                    }
                } else if n.trunc() == *n && n.abs() < 1e10 {
                    // Display as integer if no fractional part
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }

            ExprKind::Symbol(s) => write!(f, "{}", s),

            ExprKind::FunctionCall { name, args } => {
                if args.is_empty() {
                    write!(f, "{}()", name)
                } else {
                    let args_str: Vec<String> = args.iter().map(|arg| format!("{}", arg)).collect();
                    write!(f, "{}({})", name, args_str.join(", "))
                }
            }

            ExprKind::Add(u, v) => {
                // Check if v is a negative term to display as subtraction
                if let Some(positive_v) = extract_negative(v) {
                    let inner_str = format_mul_operand(&positive_v);
                    write!(f, "{} - {}", u, inner_str)
                } else {
                    write!(f, "{} + {}", u, v)
                }
            }

            ExprKind::Sub(u, v) => {
                // Parenthesize RHS when it's an addition or subtraction to preserve
                // the intended grouping: `a - (b + c)` instead of `a - b + c`.
                let right_str = match &v.kind {
                    ExprKind::Add(_, _) | ExprKind::Sub(_, _) => format!("({})", v),
                    _ => format!("{}", v),
                };
                write!(f, "{} - {}", u, right_str)
            }

            ExprKind::Mul(u, v) => {
                if let ExprKind::Number(n) = &u.kind {
                    if *n == -1.0 {
                        write!(f, "-{}", format_mul_operand(v))
                    } else if needs_explicit_mul(u, v) {
                        // Need explicit * between numbers or number and division
                        write!(f, "{}*{}", format_mul_operand(u), format_mul_operand(v))
                    } else {
                        // Compact form: 2x, 3sin(x), etc.
                        write!(f, "{}{}", format_mul_operand(u), format_mul_operand(v))
                    }
                } else if needs_explicit_mul(u, v) {
                    write!(f, "{}*{}", format_mul_operand(u), format_mul_operand(v))
                } else {
                    // symbol * symbol or similar - use compact form
                    write!(f, "{}{}", format_mul_operand(u), format_mul_operand(v))
                }
            }

            ExprKind::Div(u, v) => {
                let num_str = format!("{}", u);
                let denom_str = format!("{}", v);
                // Add parentheses around numerator if it's addition or subtraction
                let formatted_num = match &u.kind {
                    ExprKind::Add(_, _) | ExprKind::Sub(_, _) => format!("({})", num_str),
                    _ => num_str,
                };
                // Add parentheses around denominator if it's not a simple identifier, number, power, or function
                let formatted_denom = match &v.kind {
                    ExprKind::Symbol(_)
                    | ExprKind::Number(_)
                    | ExprKind::Pow(_, _)
                    | ExprKind::FunctionCall { .. } => denom_str,
                    _ => format!("({})", denom_str),
                };
                write!(f, "{}/{}", formatted_num, formatted_denom)
            }

            ExprKind::Pow(u, v) => {
                // Special case: e^x displays as exp(x)
                if let ExprKind::Symbol(s) = &u.kind
                    && s == "e"
                {
                    write!(f, "exp({})", v)
                } else {
                    let base_str = format!("{}", u);
                    let exp_str = format!("{}", v);

                    // Add parentheses around base if it's not a simple expression
                    // CRITICAL: Mul and Div MUST be parenthesized to avoid ambiguity
                    // (C * R)^2 should display as "(C * R)^2", not "C * R^2"
                    let formatted_base = match &u.kind {
                        ExprKind::Add(_, _)
                        | ExprKind::Sub(_, _)
                        | ExprKind::Mul(_, _)
                        | ExprKind::Div(_, _) => {
                            format!("({})", base_str)
                        }
                        _ => base_str,
                    };

                    // Add parentheses around exponent if it's not a simple number or symbol
                    let formatted_exp = match &v.kind {
                        ExprKind::Number(_) | ExprKind::Symbol(_) => exp_str,
                        _ => format!("({})", exp_str),
                    };

                    write!(f, "{}^{}", formatted_base, formatted_exp)
                }
            }

            ExprKind::Derivative { inner, var, order } => {
                // Format as ∂^n_inner/∂_var^n
                write!(f, "∂^{}_{}/∂_{}^{}", order, inner, var, order)
            }
        }
    }
}

/// Format operand for multiplication to minimize parentheses
fn format_mul_operand(expr: &Expr) -> String {
    match expr.kind {
        ExprKind::Add(_, _) | ExprKind::Sub(_, _) => format!("({})", expr),
        _ => format!("{}", expr),
    }
}

// ============================================================================
// LaTeX Formatter
// ============================================================================

/// Wrapper for LaTeX output formatting
pub struct LatexFormatter<'a>(pub &'a Expr);

impl fmt::Display for LatexFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_latex(self.0, f)
    }
}

/// Format expression as LaTeX
fn format_latex(expr: &Expr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &expr.kind {
        ExprKind::Number(n) => {
            if n.is_nan() {
                write!(f, r"\text{{NaN}}")
            } else if n.is_infinite() {
                if *n > 0.0 {
                    write!(f, r"\infty")
                } else {
                    write!(f, r"-\infty")
                }
            } else if n.trunc() == *n && n.abs() < 1e10 {
                write!(f, "{}", *n as i64)
            } else {
                write!(f, "{}", n)
            }
        }

        ExprKind::Symbol(s) => {
            let name = s.as_ref();
            // Convert common symbols to LaTeX
            match name {
                "pi" => write!(f, r"\pi"),
                "alpha" => write!(f, r"\alpha"),
                "beta" => write!(f, r"\beta"),
                "gamma" => write!(f, r"\gamma"),
                "delta" => write!(f, r"\delta"),
                "epsilon" => write!(f, r"\epsilon"),
                "theta" => write!(f, r"\theta"),
                "lambda" => write!(f, r"\lambda"),
                "mu" => write!(f, r"\mu"),
                "sigma" => write!(f, r"\sigma"),
                "omega" => write!(f, r"\omega"),
                "phi" => write!(f, r"\phi"),
                "psi" => write!(f, r"\psi"),
                "tau" => write!(f, r"\tau"),
                _ => write!(f, "{}", name),
            }
        }

        ExprKind::FunctionCall { name, args } => {
            let latex_name = match name.as_str() {
                "sin" | "cos" | "tan" | "cot" | "sec" | "csc" => format!(r"\{}", name),
                "asin" => r"\arcsin".to_string(),
                "acos" => r"\arccos".to_string(),
                "atan" => r"\arctan".to_string(),
                "sinh" | "cosh" | "tanh" | "coth" => format!(r"\{}", name),
                "ln" => r"\ln".to_string(),
                "log" | "log10" => r"\log".to_string(),
                "log2" => r"\log_2".to_string(),
                "exp" => r"\exp".to_string(),
                "sqrt" => {
                    if args.len() == 1 {
                        return write!(f, r"\sqrt{{{}}}", LatexFormatter(&args[0]));
                    }
                    r"\sqrt".to_string()
                }
                "abs" => {
                    if args.len() == 1 {
                        return write!(f, r"\left|{}\right|", LatexFormatter(&args[0]));
                    }
                    r"\text{abs}".to_string()
                }
                "gamma" => r"\Gamma".to_string(),
                _ => format!(r"\text{{{}}}", name),
            };

            if args.is_empty() {
                write!(f, r"{}\left(\right)", latex_name)
            } else {
                let args_str: Vec<String> = args
                    .iter()
                    .map(|arg| format!("{}", LatexFormatter(arg)))
                    .collect();
                write!(f, r"{}\left({}\right)", latex_name, args_str.join(", "))
            }
        }

        ExprKind::Add(u, v) => {
            // Check for subtraction pattern
            if let Some(positive_v) = extract_negative(v) {
                write!(
                    f,
                    "{} - {}",
                    LatexFormatter(u),
                    latex_mul_operand(&positive_v)
                )
            } else {
                write!(f, "{} + {}", LatexFormatter(u), LatexFormatter(v))
            }
        }

        ExprKind::Sub(u, v) => {
            let right_str = match &v.kind {
                ExprKind::Add(_, _) | ExprKind::Sub(_, _) => {
                    format!(r"\left({}\right)", LatexFormatter(v))
                }
                _ => format!("{}", LatexFormatter(v)),
            };
            write!(f, "{} - {}", LatexFormatter(u), right_str)
        }

        ExprKind::Mul(u, v) => {
            if let ExprKind::Number(n) = &u.kind
                && *n == -1.0
            {
                return write!(f, "-{}", latex_mul_operand(v));
            }
            write!(
                f,
                r"{} \cdot {}",
                latex_mul_operand(u),
                latex_mul_operand(v)
            )
        }

        ExprKind::Div(u, v) => {
            write!(
                f,
                r"\frac{{{}}}{{{}}}",
                LatexFormatter(u),
                LatexFormatter(v)
            )
        }

        ExprKind::Pow(u, v) => {
            // Special case: e^x
            if let ExprKind::Symbol(s) = &u.kind
                && s == "e"
            {
                return write!(f, r"e^{{{}}}", LatexFormatter(v));
            }

            let base_str = match &u.kind {
                ExprKind::Add(_, _)
                | ExprKind::Sub(_, _)
                | ExprKind::Mul(_, _)
                | ExprKind::Div(_, _) => {
                    format!(r"\left({}\right)", LatexFormatter(u))
                }
                _ => format!("{}", LatexFormatter(u)),
            };
            write!(f, "{}^{{{}}}", base_str, LatexFormatter(v))
        }

        ExprKind::Derivative { inner, var, order } => {
            if *order == 1 {
                write!(
                    f,
                    r"\frac{{\partial {}}}{{\partial {}}}",
                    LatexFormatter(inner),
                    var
                )
            } else {
                write!(
                    f,
                    r"\frac{{\partial^{} {}}}{{\partial {}^{}}}",
                    order,
                    LatexFormatter(inner),
                    var,
                    order
                )
            }
        }
    }
}

/// Format LaTeX multiplication operand
fn latex_mul_operand(expr: &Expr) -> String {
    match expr.kind {
        ExprKind::Add(_, _) | ExprKind::Sub(_, _) => {
            format!(r"\left({}\right)", LatexFormatter(expr))
        }
        _ => format!("{}", LatexFormatter(expr)),
    }
}

// ============================================================================
// Unicode Formatter
// ============================================================================

/// Wrapper for Unicode output formatting (superscripts, Greek letters)
pub struct UnicodeFormatter<'a>(pub &'a Expr);

impl fmt::Display for UnicodeFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_unicode(self.0, f)
    }
}

/// Convert a digit to Unicode superscript
fn to_superscript(c: char) -> char {
    match c {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        '-' => '⁻',
        '+' => '⁺',
        '(' => '⁽',
        ')' => '⁾',
        _ => c,
    }
}

/// Convert number to superscript string
fn num_to_superscript(n: f64) -> String {
    if n.trunc() == n && n.abs() < 1000.0 {
        format!("{}", n as i64)
            .chars()
            .map(to_superscript)
            .collect()
    } else {
        format!("^{}", n)
    }
}

/// Format expression with Unicode characters
fn format_unicode(expr: &Expr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &expr.kind {
        ExprKind::Number(n) => {
            if n.is_nan() {
                write!(f, "NaN")
            } else if n.is_infinite() {
                if *n > 0.0 {
                    write!(f, "∞")
                } else {
                    write!(f, "-∞")
                }
            } else if n.trunc() == *n && n.abs() < 1e10 {
                write!(f, "{}", *n as i64)
            } else {
                write!(f, "{}", n)
            }
        }

        ExprKind::Symbol(s) => {
            let name = s.as_ref();
            match name {
                "pi" => write!(f, "π"),
                "alpha" => write!(f, "α"),
                "beta" => write!(f, "β"),
                "gamma" => write!(f, "γ"),
                "delta" => write!(f, "δ"),
                "epsilon" => write!(f, "ε"),
                "theta" => write!(f, "θ"),
                "lambda" => write!(f, "λ"),
                "mu" => write!(f, "μ"),
                "sigma" => write!(f, "σ"),
                "omega" => write!(f, "ω"),
                "phi" => write!(f, "φ"),
                "psi" => write!(f, "ψ"),
                "tau" => write!(f, "τ"),
                _ => write!(f, "{}", name),
            }
        }

        ExprKind::FunctionCall { name, args } => {
            if args.is_empty() {
                write!(f, "{}()", name)
            } else {
                let args_str: Vec<String> = args
                    .iter()
                    .map(|arg| format!("{}", UnicodeFormatter(arg)))
                    .collect();
                write!(f, "{}({})", name, args_str.join(", "))
            }
        }

        ExprKind::Add(u, v) => {
            if let Some(positive_v) = extract_negative(v) {
                write!(
                    f,
                    "{} − {}",
                    UnicodeFormatter(u),
                    unicode_mul_operand(&positive_v)
                )
            } else {
                write!(f, "{} + {}", UnicodeFormatter(u), UnicodeFormatter(v))
            }
        }

        ExprKind::Sub(u, v) => {
            let right_str = match &v.kind {
                ExprKind::Add(_, _) | ExprKind::Sub(_, _) => {
                    format!("({})", UnicodeFormatter(v))
                }
                _ => format!("{}", UnicodeFormatter(v)),
            };
            write!(f, "{} − {}", UnicodeFormatter(u), right_str)
        }

        ExprKind::Mul(u, v) => {
            if let ExprKind::Number(n) = &u.kind
                && *n == -1.0
            {
                return write!(f, "−{}", unicode_mul_operand(v));
            }
            write!(f, "{}·{}", unicode_mul_operand(u), unicode_mul_operand(v))
        }

        ExprKind::Div(u, v) => {
            let num_str = format!("{}", UnicodeFormatter(u));
            let formatted_num = match &u.kind {
                ExprKind::Add(_, _) | ExprKind::Sub(_, _) => format!("({})", num_str),
                _ => num_str,
            };
            let denom_str = format!("{}", UnicodeFormatter(v));
            let formatted_denom = match &v.kind {
                ExprKind::Symbol(_)
                | ExprKind::Number(_)
                | ExprKind::Pow(_, _)
                | ExprKind::FunctionCall { .. } => denom_str,
                _ => format!("({})", denom_str),
            };
            write!(f, "{}/{}", formatted_num, formatted_denom)
        }

        ExprKind::Pow(u, v) => {
            // Special case: e^x displays as exp(x)
            if let ExprKind::Symbol(s) = &u.kind
                && s == "e"
            {
                return write!(f, "exp({})", UnicodeFormatter(v));
            }

            let base_str = match &u.kind {
                ExprKind::Add(_, _)
                | ExprKind::Sub(_, _)
                | ExprKind::Mul(_, _)
                | ExprKind::Div(_, _) => {
                    format!("({})", UnicodeFormatter(u))
                }
                _ => format!("{}", UnicodeFormatter(u)),
            };

            // Use superscripts for simple integer exponents
            if let ExprKind::Number(n) = &v.kind {
                write!(f, "{}{}", base_str, num_to_superscript(*n))
            } else if let ExprKind::Symbol(_) = &v.kind {
                write!(f, "{}^{}", base_str, UnicodeFormatter(v))
            } else {
                write!(f, "{}^({})", base_str, UnicodeFormatter(v))
            }
        }

        ExprKind::Derivative { inner, var, order } => {
            let order_sup = num_to_superscript(*order as f64);
            write!(
                f,
                "∂{}({})/∂{}{}",
                order_sup,
                UnicodeFormatter(inner),
                var,
                order_sup
            )
        }
    }
}

/// Format Unicode multiplication operand
fn unicode_mul_operand(expr: &Expr) -> String {
    match expr.kind {
        ExprKind::Add(_, _) | ExprKind::Sub(_, _) => format!("({})", UnicodeFormatter(expr)),
        _ => format!("{}", UnicodeFormatter(expr)),
    }
}

// ============================================================================
// Expr Methods for Formatting
// ============================================================================

impl Expr {
    /// Format expression as LaTeX string
    ///
    /// Uses expressive LaTeX notation:
    /// - Fractions: `\frac{a}{b}`
    /// - Multiplication: `a \cdot b`
    /// - Functions: `\sin`, `\cos`, `\sqrt{x}`
    /// - Greek: `pi` → `\pi`
    ///
    /// # Example
    /// ```ignore
    /// let expr = sym("x").pow(2.0) / sym("y");
    /// assert_eq!(expr.to_latex(), r"\frac{x^{2}}{y}");
    /// ```
    pub fn to_latex(&self) -> String {
        format!("{}", LatexFormatter(self))
    }

    /// Format expression with Unicode characters
    ///
    /// - Superscripts for powers: `x²`, `x³`
    /// - Greek letters: `pi` → `π`
    /// - Minus sign: `−` (proper minus)
    /// - Multiplication: `·`
    ///
    /// # Example
    /// ```ignore
    /// let expr = sym("x").pow(2.0) + sym("pi");
    /// assert_eq!(expr.to_unicode(), "x² + π");
    /// ```
    pub fn to_unicode(&self) -> String {
        format!("{}", UnicodeFormatter(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_number() {
        let expr = Expr::number(3.0);
        assert_eq!(format!("{}", expr), "3");

        let expr = Expr::number(314.0 / 100.0);
        // Formatting may use full precision; ensure it starts with the expected prefix
        assert!(format!("{}", expr).starts_with("3.14"));
    }

    #[test]
    fn test_display_symbol() {
        let expr = Expr::symbol("x");
        assert_eq!(format!("{}", expr), "x");
    }

    #[test]
    fn test_display_addition() {
        let expr = Expr::add_expr(Expr::symbol("x"), Expr::number(1.0));
        assert_eq!(format!("{}", expr), "x + 1");
    }

    #[test]
    fn test_display_multiplication() {
        let expr = Expr::mul_expr(Expr::symbol("x"), Expr::number(2.0));
        assert_eq!(format!("{}", expr), "x*2");
    }

    #[test]
    fn test_display_function() {
        let expr = Expr::new(ExprKind::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expr::symbol("x")],
        });
        assert_eq!(format!("{}", expr), "sin(x)");
    }

    #[test]
    fn test_display_negative_term() {
        let expr = Expr::mul_expr(Expr::number(-1.0), Expr::symbol("x"));
        assert_eq!(format!("{}", expr), "-x");

        let expr2 = Expr::mul_expr(
            Expr::number(-1.0),
            Expr::new(ExprKind::FunctionCall {
                name: "sin".to_string(),
                args: vec![Expr::symbol("x")],
            }),
        );
        assert_eq!(format!("{}", expr2), "-sin(x)");
    }

    #[test]
    fn test_display_fraction_parens() {
        // 1 / x -> 1/x
        let expr = Expr::div_expr(Expr::number(1.0), Expr::symbol("x"));
        assert_eq!(format!("{}", expr), "1/x");

        // 1 / x^2 -> 1/x^2
        let expr = Expr::div_expr(
            Expr::number(1.0),
            Expr::pow(Expr::symbol("x"), Expr::number(2.0)),
        );
        assert_eq!(format!("{}", expr), "1/x^2");

        // 1 / sin(x) -> 1/sin(x)
        let expr = Expr::div_expr(
            Expr::number(1.0),
            Expr::new(ExprKind::FunctionCall {
                name: "sin".to_string(),
                args: vec![Expr::symbol("x")],
            }),
        );
        assert_eq!(format!("{}", expr), "1/sin(x)");

        // 1 / (2 * x) -> 1/(2x)
        let expr = Expr::div_expr(
            Expr::number(1.0),
            Expr::mul_expr(Expr::number(2.0), Expr::symbol("x")),
        );
        assert_eq!(format!("{}", expr), "1/(2x)");

        // 1 / (x + 1) -> 1/(x + 1)
        let expr = Expr::div_expr(
            Expr::number(1.0),
            Expr::add_expr(Expr::symbol("x"), Expr::number(1.0)),
        );
        assert_eq!(format!("{}", expr), "1/(x + 1)");
    }

    #[test]
    fn test_display_neg_x_exp() {
        // -1 * (x * exp(y)) should display as -x*exp(y)
        let exp_y = Expr::new(ExprKind::FunctionCall {
            name: "exp".to_string(),
            args: vec![Expr::symbol("y")],
        });
        let x_times_exp = Expr::mul_expr(Expr::symbol("x"), exp_y);
        let neg_x_times_exp = Expr::mul_expr(Expr::number(-1.0), x_times_exp);

        eprintln!("AST: {:?}", neg_x_times_exp);
        eprintln!("Display: {}", neg_x_times_exp);
        assert_eq!(format!("{}", neg_x_times_exp), "-x*exp(y)");

        // Alternative structure: (-1 * x) * exp(y)
        let neg_x = Expr::mul_expr(Expr::number(-1.0), Expr::symbol("x"));
        let neg_x_times_exp2 = Expr::mul_expr(
            neg_x,
            Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::symbol("y")],
            }),
        );

        eprintln!("AST2: {:?}", neg_x_times_exp2);
        eprintln!("Display2: {}", neg_x_times_exp2);
        assert_eq!(format!("{}", neg_x_times_exp2), "-x*exp(y)");

        // Test: -x * (exp(y) * z) - the exp is nested in a Mul on the right
        let exp_y_times_z = Expr::mul_expr(
            Expr::new(ExprKind::FunctionCall {
                name: "exp".to_string(),
                args: vec![Expr::symbol("y")],
            }),
            Expr::symbol("z"),
        );
        let neg_x_times_exp_z = Expr::mul_expr(
            Expr::mul_expr(Expr::number(-1.0), Expr::symbol("x")),
            exp_y_times_z,
        );

        eprintln!("AST3: {:?}", neg_x_times_exp_z);
        eprintln!("Display3: {}", neg_x_times_exp_z);
    }

    #[test]
    fn test_display_quantum_derivative() {
        // Test actual quantum derivative: -xexp(-x^2/(2sigma^2))/(sigma^3*sqrt(pi))
        // The full expression from the example
        use crate::parser;
        use crate::simplification;
        use std::collections::HashSet;

        let fixed_set: HashSet<String> = ["sigma".to_string()].iter().cloned().collect();
        let custom_funcs: HashSet<String> = HashSet::new();

        let ast = parser::parse(
            "(exp(-x^2 / (4 * sigma^2)) / sqrt(sigma * sqrt(pi)))^2",
            &fixed_set,
            &custom_funcs,
        )
        .unwrap();

        let derivative = ast.derive(
            "x",
            &fixed_set,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
        );
        let simplified = simplification::simplify_expr(derivative, fixed_set);

        eprintln!("Simplified AST: {:?}", simplified);
        eprintln!("Quantum derivative: {}", simplified);

        // Should contain x*exp not xexp
        let display = format!("{}", simplified);
        assert!(
            display.contains("x*exp") || display.contains("x * exp"),
            "Expected 'x*exp' but got: {}",
            display
        );
    }
}
