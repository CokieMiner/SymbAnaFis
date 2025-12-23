//! Display implementations for expressions
//!
//! # Display Behavior Notes for N-ary AST
//! - Sum displays terms with +/- signs based on leading coefficients
//! - Product displays with implicit or explicit multiplication
//! - `e^x` is always displayed as `exp(x)` for consistency
//! - Derivatives use notation like `∂^n_inner/∂_var^n`

use crate::core::known_symbols::E;
use crate::core::traits::EPSILON;
use crate::{Expr, ExprKind};
use std::fmt;
use std::sync::Arc;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Check if an expression is negative (has a negative leading coefficient)
/// Returns Some(positive_equivalent) if the expression has a negative sign
fn extract_negative(expr: &Expr) -> Option<Expr> {
    match &expr.kind {
        ExprKind::Product(factors) => {
            if !factors.is_empty()
                && let ExprKind::Number(n) = &factors[0].kind
                && *n < 0.0
            {
                // Negative leading coefficient
                let abs_coeff = n.abs();
                if (abs_coeff - 1.0).abs() < EPSILON {
                    // Exactly -1: just remove it
                    if factors.len() == 2 {
                        return Some((*factors[1]).clone());
                    } else {
                        let remaining: Vec<Arc<Expr>> = factors[1..].to_vec();
                        return Some(Expr::product_from_arcs(remaining));
                    }
                } else {
                    // Other negative coefficient like -2, -3.5: replace with positive
                    let mut new_factors: Vec<Arc<Expr>> = Vec::with_capacity(factors.len());
                    new_factors.push(Arc::new(Expr::number(abs_coeff)));
                    new_factors.extend(factors[1..].iter().cloned());
                    return Some(Expr::product_from_arcs(new_factors));
                }
            }
        }
        ExprKind::Number(n) => {
            if *n < 0.0 {
                return Some(Expr::number(-*n));
            }
        }
        // Handle Poly with negative first term
        ExprKind::Poly(poly) => {
            if let Some(first_coeff) = poly.first_coeff()
                && first_coeff < 0.0
            {
                // Create a new Poly with ALL terms negated
                // -P = -(P_negated)
                // e.g. -x + x^2  -> negating all terms gives x - x^2
                // displayed as -(x - x^2) which is correct (-x + x^2)
                let negated_poly = poly.negate();
                return Some(Expr::new(ExprKind::Poly(negated_poly)));
            }
        }
        _ => {}
    }
    None
}

/// Check if expression needs parentheses when displayed in a product
fn needs_parens_in_product(expr: &Expr) -> bool {
    matches!(expr.kind, ExprKind::Sum(_) | ExprKind::Poly(_))
}

/// Check if expression needs parentheses when displayed as a power base
fn needs_parens_as_base(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Sum(_) | ExprKind::Product(_) | ExprKind::Div(_, _) | ExprKind::Poly(_) => true,
        ExprKind::Number(n) => *n < 0.0, // Negative numbers need parens: (-1)^x not -1^x
        _ => false,
    }
}

/// Format a single factor for display in a product chain
fn format_factor(expr: &Expr) -> String {
    if needs_parens_in_product(expr) {
        format!("({})", expr)
    } else {
        format!("{}", expr)
    }
}

/// Greek letter mappings: (name, latex, unicode)
static GREEK_LETTERS: &[(&str, &str, &str)] = &[
    ("pi", r"\pi", "π"),
    ("alpha", r"\alpha", "α"),
    ("beta", r"\beta", "β"),
    ("gamma", r"\gamma", "γ"),
    ("delta", r"\delta", "δ"),
    ("epsilon", r"\epsilon", "ε"),
    ("theta", r"\theta", "θ"),
    ("lambda", r"\lambda", "λ"),
    ("mu", r"\mu", "μ"),
    ("sigma", r"\sigma", "σ"),
    ("omega", r"\omega", "ω"),
    ("phi", r"\phi", "φ"),
    ("psi", r"\psi", "ψ"),
    ("tau", r"\tau", "τ"),
];

/// Map symbol name to Greek letter (LaTeX format)
fn greek_to_latex(name: &str) -> Option<&'static str> {
    GREEK_LETTERS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, latex, _)| *latex)
}

/// Map symbol name to Unicode Greek letter
fn greek_to_unicode(name: &str) -> Option<&'static str> {
    GREEK_LETTERS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, _, unicode)| *unicode)
}

// =============================================================================
// DISPLAY IMPLEMENTATION
// =============================================================================

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

            // N-ary Sum: display with + and - signs
            ExprKind::Sum(terms) => {
                if terms.is_empty() {
                    return write!(f, "0");
                }

                let mut first = true;
                for term in terms {
                    if first {
                        // First term: check if negative
                        if let Some(positive_term) = extract_negative(term) {
                            write!(f, "-{}", format_factor(&positive_term))?;
                        } else {
                            write!(f, "{}", term)?;
                        }
                        first = false;
                    } else {
                        // Subsequent terms: show + or -
                        if let Some(positive_term) = extract_negative(term) {
                            write!(f, " - {}", format_factor(&positive_term))?;
                        } else {
                            write!(f, " + {}", term)?;
                        }
                    }
                }
                Ok(())
            }

            // N-ary Product: display with * or implicit multiplication
            ExprKind::Product(factors) => {
                if factors.is_empty() {
                    return write!(f, "1");
                }

                // Check for leading -1
                if !factors.is_empty()
                    && let ExprKind::Number(n) = &factors[0].kind
                    && (*n + 1.0).abs() < EPSILON
                {
                    // Leading -1: display as negation
                    if factors.len() == 1 {
                        return write!(f, "-1");
                    } else if factors.len() == 2 {
                        return write!(f, "-{}", format_factor(&factors[1]));
                    } else {
                        let remaining: Vec<Arc<Expr>> = factors[1..].to_vec();
                        let rest = Expr::product_from_arcs(remaining);
                        return write!(f, "-{}", format_factor(&rest));
                    }
                }

                // Display factors with explicit * separator - write directly to avoid Vec<String>
                let mut first = true;
                for fac in factors {
                    if !first {
                        write!(f, "*")?;
                    }
                    write!(f, "{}", format_factor(fac))?;
                    first = false;
                }
                Ok(())
            }

            ExprKind::Div(u, v) => {
                let num_str = format!("{}", u);
                let denom_str = format!("{}", v);

                // Parenthesize numerator if it's a sum
                let formatted_num = if matches!(u.kind, ExprKind::Sum(_)) {
                    format!("({})", num_str)
                } else {
                    num_str
                };

                // Parenthesize denominator if it's not simple
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
                    && s.id() == *E
                {
                    return write!(f, "exp({})", v);
                }

                let base_str = format!("{}", u);
                let exp_str = format!("{}", v);

                let formatted_base = if needs_parens_as_base(u) {
                    format!("({})", base_str)
                } else {
                    base_str
                };

                let formatted_exp = match &v.kind {
                    ExprKind::Number(_) | ExprKind::Symbol(_) => exp_str,
                    _ => format!("({})", exp_str),
                };

                write!(f, "{}^{}", formatted_base, formatted_exp)
            }

            ExprKind::Derivative { inner, var, order } => {
                write!(f, "∂^{}_{}/∂_{}^{}", order, inner, var, order)
            }

            // Poly: display inline using Polynomial's Display
            ExprKind::Poly(poly) => {
                write!(f, "{}", poly)
            }
        }
    }
}

// =============================================================================
// LATEX FORMATTER
// =============================================================================

pub(crate) struct LatexFormatter<'a>(pub(crate) &'a Expr);

impl fmt::Display for LatexFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_latex(self.0, f)
    }
}

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
            if let Some(greek) = greek_to_latex(name) {
                write!(f, "{}", greek)
            } else {
                write!(f, "{}", name)
            }
        }

        ExprKind::FunctionCall { name, args } => {
            // Special formatting for specific functions
            match name.as_str() {
                // === ROOTS ===
                "sqrt" if args.len() == 1 => {
                    return write!(f, r"\sqrt{{{}}}", LatexFormatter(&args[0]));
                }
                "cbrt" if args.len() == 1 => {
                    return write!(f, r"\sqrt[3]{{{}}}", LatexFormatter(&args[0]));
                }

                // === ABSOLUTE VALUE ===
                "abs" if args.len() == 1 => {
                    return write!(f, r"\left|{}\right|", LatexFormatter(&args[0]));
                }

                // === FLOOR/CEIL ===
                "floor" if args.len() == 1 => {
                    return write!(f, r"\lfloor{}\rfloor", LatexFormatter(&args[0]));
                }
                "ceil" if args.len() == 1 => {
                    return write!(f, r"\lceil{}\rceil", LatexFormatter(&args[0]));
                }

                // === BESSEL FUNCTIONS: J_n(x), Y_n(x), I_n(x), K_n(x) ===
                "besselj" if args.len() == 2 => {
                    return write!(
                        f,
                        r"J_{{{}}}\left({}\right)",
                        LatexFormatter(&args[0]),
                        LatexFormatter(&args[1])
                    );
                }
                "bessely" if args.len() == 2 => {
                    return write!(
                        f,
                        r"Y_{{{}}}\left({}\right)",
                        LatexFormatter(&args[0]),
                        LatexFormatter(&args[1])
                    );
                }
                "besseli" if args.len() == 2 => {
                    return write!(
                        f,
                        r"I_{{{}}}\left({}\right)",
                        LatexFormatter(&args[0]),
                        LatexFormatter(&args[1])
                    );
                }
                "besselk" if args.len() == 2 => {
                    return write!(
                        f,
                        r"K_{{{}}}\left({}\right)",
                        LatexFormatter(&args[0]),
                        LatexFormatter(&args[1])
                    );
                }

                // === ORTHOGONAL POLYNOMIALS ===
                "hermite" if args.len() == 2 => {
                    return write!(
                        f,
                        r"H_{{{}}}\left({}\right)",
                        LatexFormatter(&args[0]),
                        LatexFormatter(&args[1])
                    );
                }
                "assoc_legendre" if args.len() == 3 => {
                    return write!(
                        f,
                        r"P_{{{}}}^{{{}}}\left({}\right)",
                        LatexFormatter(&args[0]),
                        LatexFormatter(&args[1]),
                        LatexFormatter(&args[2])
                    );
                }
                "spherical_harmonic" | "ynm" if args.len() == 4 => {
                    return write!(
                        f,
                        r"Y_{{{}}}^{{{}}}\left({}, {}\right)",
                        LatexFormatter(&args[0]),
                        LatexFormatter(&args[1]),
                        LatexFormatter(&args[2]),
                        LatexFormatter(&args[3])
                    );
                }

                // === POLYGAMMA FUNCTIONS ===
                "digamma" if args.len() == 1 => {
                    return write!(f, r"\psi\left({}\right)", LatexFormatter(&args[0]));
                }
                "trigamma" if args.len() == 1 => {
                    return write!(f, r"\psi_1\left({}\right)", LatexFormatter(&args[0]));
                }
                "tetragamma" if args.len() == 1 => {
                    return write!(f, r"\psi_2\left({}\right)", LatexFormatter(&args[0]));
                }
                "polygamma" if args.len() == 2 => {
                    return write!(
                        f,
                        r"\psi^{{({})}}\left({}\right)",
                        LatexFormatter(&args[0]),
                        LatexFormatter(&args[1])
                    );
                }

                // === ELLIPTIC INTEGRALS ===
                "elliptic_k" if args.len() == 1 => {
                    return write!(f, r"K\left({}\right)", LatexFormatter(&args[0]));
                }
                "elliptic_e" if args.len() == 1 => {
                    return write!(f, r"E\left({}\right)", LatexFormatter(&args[0]));
                }

                // === ZETA ===
                "zeta" if args.len() == 1 => {
                    return write!(f, r"\zeta\left({}\right)", LatexFormatter(&args[0]));
                }
                "zeta_deriv" if args.len() == 2 => {
                    return write!(
                        f,
                        r"\zeta^{{({})}}\left({}\right)",
                        LatexFormatter(&args[0]),
                        LatexFormatter(&args[1])
                    );
                }

                // === LAMBERT W ===
                "lambertw" if args.len() == 1 => {
                    return write!(f, r"W\left({}\right)", LatexFormatter(&args[0]));
                }

                // === BETA ===
                "beta" if args.len() == 2 => {
                    return write!(
                        f,
                        r"\mathrm{{B}}\left({}, {}\right)",
                        LatexFormatter(&args[0]),
                        LatexFormatter(&args[1])
                    );
                }

                _ => {}
            }

            // Standard function name LaTeX mappings
            let latex_name = match name.as_str() {
                // Trigonometric
                "sin" | "cos" | "tan" | "cot" | "sec" | "csc" => format!(r"\{}", name),
                // Inverse trigonometric
                "asin" => r"\arcsin".to_string(),
                "acos" => r"\arccos".to_string(),
                "atan" => r"\arctan".to_string(),
                "acot" => r"\operatorname{arccot}".to_string(),
                "asec" => r"\operatorname{arcsec}".to_string(),
                "acsc" => r"\operatorname{arccsc}".to_string(),
                // Hyperbolic
                "sinh" | "cosh" | "tanh" | "coth" => format!(r"\{}", name),
                "sech" => r"\operatorname{sech}".to_string(),
                "csch" => r"\operatorname{csch}".to_string(),
                // Inverse hyperbolic
                "asinh" => r"\operatorname{arsinh}".to_string(),
                "acosh" => r"\operatorname{arcosh}".to_string(),
                "atanh" => r"\operatorname{artanh}".to_string(),
                "acoth" => r"\operatorname{arcoth}".to_string(),
                "asech" => r"\operatorname{arsech}".to_string(),
                "acsch" => r"\operatorname{arcsch}".to_string(),
                // Logarithms
                "ln" => r"\ln".to_string(),
                "log" | "log10" => r"\log".to_string(),
                "log2" => r"\log_2".to_string(),
                // Exponential
                "exp" => r"\exp".to_string(),
                "exp_polar" => r"\operatorname{exp\_polar}".to_string(),
                // Special functions
                "gamma" => r"\Gamma".to_string(),
                "erf" => r"\operatorname{erf}".to_string(),
                "erfc" => r"\operatorname{erfc}".to_string(),
                "signum" => r"\operatorname{sgn}".to_string(),
                "sinc" => r"\operatorname{sinc}".to_string(),
                "round" => r"\operatorname{round}".to_string(),
                // Default: wrap in \text{}
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

        ExprKind::Sum(terms) => {
            if terms.is_empty() {
                return write!(f, "0");
            }

            let mut first = true;
            for term in terms {
                if first {
                    if let Some(positive_term) = extract_negative(term) {
                        write!(f, "-{}", latex_factor(&positive_term))?;
                    } else {
                        write!(f, "{}", LatexFormatter(term))?;
                    }
                    first = false;
                } else if let Some(positive_term) = extract_negative(term) {
                    write!(f, " - {}", latex_factor(&positive_term))?;
                } else {
                    write!(f, " + {}", LatexFormatter(term))?;
                }
            }
            Ok(())
        }

        ExprKind::Product(factors) => {
            if factors.is_empty() {
                return write!(f, "1");
            }

            // Check for leading -1
            if !factors.is_empty()
                && let ExprKind::Number(n) = &factors[0].kind
                && (*n + 1.0).abs() < EPSILON
            {
                if factors.len() == 1 {
                    return write!(f, "-1");
                }
                let remaining: Vec<Arc<Expr>> = factors[1..].to_vec();
                let rest = Expr::product_from_arcs(remaining);
                return write!(f, "-{}", latex_factor(&rest));
            }

            // Write factors with \cdot separator directly
            let mut first = true;
            for fac in factors {
                if !first {
                    write!(f, r" \cdot ")?;
                }
                write!(f, "{}", latex_factor(fac))?;
                first = false;
            }
            Ok(())
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
            if let ExprKind::Symbol(s) = &u.kind
                && s.id() == *E
            {
                return write!(f, r"e^{{{}}}", LatexFormatter(v));
            }

            let base_str = if needs_parens_as_base(u) {
                format!(r"\left({}\right)", LatexFormatter(u))
            } else {
                format!("{}", LatexFormatter(u))
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

        // Poly: display inline in LaTeX
        ExprKind::Poly(poly) => write!(f, "{}", poly),
    }
}

fn latex_factor(expr: &Expr) -> String {
    if matches!(expr.kind, ExprKind::Sum(_)) {
        format!(r"\left({}\right)", LatexFormatter(expr))
    } else {
        format!("{}", LatexFormatter(expr))
    }
}

// =============================================================================
// UNICODE FORMATTER
// =============================================================================

pub(crate) struct UnicodeFormatter<'a>(pub(crate) &'a Expr);

impl fmt::Display for UnicodeFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_unicode(self.0, f)
    }
}

#[inline]
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

#[inline]
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

fn format_unicode(expr: &Expr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &expr.kind {
        ExprKind::Number(n) => {
            if n.is_nan() {
                write!(f, "NaN")
            } else if n.is_infinite() {
                write!(f, "{}", if *n > 0.0 { "∞" } else { "-∞" })
            } else if n.trunc() == *n && n.abs() < 1e10 {
                write!(f, "{}", *n as i64)
            } else {
                write!(f, "{}", n)
            }
        }

        ExprKind::Symbol(s) => {
            let name = s.as_ref();
            if let Some(greek) = greek_to_unicode(name) {
                write!(f, "{}", greek)
            } else {
                write!(f, "{}", name)
            }
        }

        ExprKind::FunctionCall { name, args } => {
            if args.is_empty() {
                write!(f, "{}()", name)
            } else {
                let args_str: Vec<String> = args
                    .iter()
                    .map(|a| format!("{}", UnicodeFormatter(a)))
                    .collect();
                write!(f, "{}({})", name, args_str.join(", "))
            }
        }

        ExprKind::Sum(terms) => {
            if terms.is_empty() {
                return write!(f, "0");
            }
            let mut first = true;
            for term in terms {
                if first {
                    if let Some(positive) = extract_negative(term) {
                        write!(f, "−{}", unicode_factor(&positive))?;
                    } else {
                        write!(f, "{}", UnicodeFormatter(term))?;
                    }
                    first = false;
                } else if let Some(positive) = extract_negative(term) {
                    write!(f, " − {}", unicode_factor(&positive))?;
                } else {
                    write!(f, " + {}", UnicodeFormatter(term))?;
                }
            }
            Ok(())
        }

        ExprKind::Product(factors) => {
            if factors.is_empty() {
                return write!(f, "1");
            }
            if !factors.is_empty()
                && let ExprKind::Number(n) = &factors[0].kind
                && (*n + 1.0).abs() < EPSILON
            {
                if factors.len() == 1 {
                    return write!(f, "−1");
                }
                let remaining: Vec<Arc<Expr>> = factors[1..].to_vec();
                let rest = Expr::product_from_arcs(remaining);
                return write!(f, "−{}", unicode_factor(&rest));
            }
            // Write factors with · separator directly
            let mut first = true;
            for fac in factors {
                if !first {
                    write!(f, "·")?;
                }
                write!(f, "{}", unicode_factor(fac))?;
                first = false;
            }
            Ok(())
        }

        ExprKind::Div(u, v) => {
            let num = if matches!(u.kind, ExprKind::Sum(_)) {
                format!("({})", UnicodeFormatter(u))
            } else {
                format!("{}", UnicodeFormatter(u))
            };
            let denom = match &v.kind {
                ExprKind::Symbol(_)
                | ExprKind::Number(_)
                | ExprKind::Pow(_, _)
                | ExprKind::FunctionCall { .. } => format!("{}", UnicodeFormatter(v)),
                _ => format!("({})", UnicodeFormatter(v)),
            };
            write!(f, "{}/{}", num, denom)
        }

        ExprKind::Pow(u, v) => {
            if let ExprKind::Symbol(s) = &u.kind
                && s.id() == *E
            {
                return write!(f, "exp({})", UnicodeFormatter(v));
            }
            let base = if needs_parens_as_base(u) {
                format!("({})", UnicodeFormatter(u))
            } else {
                format!("{}", UnicodeFormatter(u))
            };
            if let ExprKind::Number(n) = &v.kind {
                write!(f, "{}{}", base, num_to_superscript(*n))
            } else if matches!(v.kind, ExprKind::Symbol(_)) {
                write!(f, "{}^{}", base, UnicodeFormatter(v))
            } else {
                write!(f, "{}^({})", base, UnicodeFormatter(v))
            }
        }

        ExprKind::Derivative { inner, var, order } => {
            let sup = num_to_superscript(*order as f64);
            write!(f, "∂{}({})/∂{}{}", sup, UnicodeFormatter(inner), var, sup)
        }

        // Poly: display inline in unicode
        ExprKind::Poly(poly) => write!(f, "{}", poly),
    }
}

fn unicode_factor(expr: &Expr) -> String {
    if matches!(expr.kind, ExprKind::Sum(_)) {
        format!("({})", UnicodeFormatter(expr))
    } else {
        format!("{}", UnicodeFormatter(expr))
    }
}

// =============================================================================
// EXPR FORMATTING METHODS
// =============================================================================

impl Expr {
    /// Convert the expression to LaTeX format.
    ///
    /// Returns a string suitable for rendering in LaTeX math environments.
    pub fn to_latex(&self) -> String {
        format!("{}", LatexFormatter(self))
    }

    /// Convert the expression to Unicode format.
    ///
    /// Returns a string with Unicode superscripts and Greek letters for display.
    pub fn to_unicode(&self) -> String {
        format!("{}", UnicodeFormatter(self))
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_display_number() {
        assert_eq!(format!("{}", Expr::number(3.0)), "3");
        assert!(format!("{}", Expr::number(3.141)).starts_with("3.141"));
    }

    #[test]
    fn test_display_symbol() {
        assert_eq!(format!("{}", Expr::symbol("x")), "x");
    }

    #[test]
    fn test_display_sum() {
        use crate::simplification::simplify_expr;
        use std::collections::HashSet;
        let expr = simplify_expr(
            Expr::sum(vec![Expr::symbol("x"), Expr::number(1.0)]),
            HashSet::new(),
        );
        assert_eq!(format!("{}", expr), "1 + x"); // Sorted after simplify: numbers before symbols
    }

    #[test]
    fn test_display_product() {
        let prod = Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]);
        assert_eq!(format!("{}", prod), "2*x");
    }

    #[test]
    fn test_display_negation() {
        let expr = Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]);
        assert_eq!(format!("{}", expr), "-x");
    }

    #[test]
    fn test_display_subtraction() {
        // x - y = Sum([x, Product([-1, y])])
        let expr = Expr::sub_expr(Expr::symbol("x"), Expr::symbol("y"));
        let display = format!("{}", expr);
        // Should display as subtraction
        assert!(
            display.contains("-"),
            "Expected subtraction, got: {}",
            display
        );
    }
}
