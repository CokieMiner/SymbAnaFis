//! Display implementations for expressions.
//!
//! This module provides three output formats for mathematical expressions:
//!
//! ## Standard Display (`to_string()` / `{}`)
//! Human-readable mathematical notation:
//! - `x^2 + 2*x + 1`
//! - `sin(x) + cos(x)`
//!
//! ## LaTeX Format (`to_latex()`)
//! For typesetting in documents with proper mathematical notation:
//! - `x^{2} + 2 \cdot x + 1`
//! - `\sin\left(x\right) + \cos\left(x\right)`
//! - Supports special functions: Bessel, polygamma, elliptic integrals, etc.
//!
//! ## Unicode Format (`to_unicode()`)
//! Pretty display with Unicode superscripts and Greek letters:
//! - `x² + 2·x + 1`
//! - `sin(x) + cos(x)` with π, α, β, etc. for Greek variables
//!
//! # Display Behavior Notes for N-ary AST
//! - Sum displays terms with +/- signs based on leading coefficients
//! - Product displays with explicit `*` or `·` multiplication
//! - `e^x` is always displayed as `exp(x)` for consistency
//! - Derivatives use ∂ notation

use crate::core::known_symbols as ks;
use crate::core::traits::EPSILON;
use crate::{Expr, ExprKind};
use rustc_hash::FxHashMap;
use std::fmt;
use std::sync::Arc;

// =============================================================================
// HELPER TYPES & FUNCTIONS
// =============================================================================

/// Internal display mode to consolidate redundant formatting logic
#[derive(Clone, Copy)]
enum FormatMode {
    Standard,
    Latex,
    Unicode,
}

/// Cache for symbol names to avoid repetitive global registry lookups
type SymbolCache = FxHashMap<u64, Arc<str>>;

fn collect_symbol_names(expr: &Expr, cache: &mut SymbolCache) {
    match &expr.kind {
        ExprKind::Symbol(s) => {
            let id = s.id();
            if let std::collections::hash_map::Entry::Vacant(e) = cache.entry(id)
                && let Some(name) = s.name_arc()
            {
                e.insert(name);
            }
        }
        ExprKind::FunctionCall { args, .. } | ExprKind::Sum(args) | ExprKind::Product(args) => {
            for arg in args {
                collect_symbol_names(arg, cache);
            }
        }
        ExprKind::Div(u, v) | ExprKind::Pow(u, v) => {
            collect_symbol_names(u, cache);
            collect_symbol_names(v, cache);
        }
        ExprKind::Derivative { inner, .. } => {
            collect_symbol_names(inner, cache);
        }
        ExprKind::Poly(poly) => {
            collect_symbol_names(poly.base(), cache);
        }
        ExprKind::Number(_) => {}
    }
}

#[derive(Clone, Copy)]
enum ParenContext {
    SumOrProduct,
    PowerBase,
}

/// Check if an expression is negative (has a negative leading coefficient)
/// Returns `Some(positive_equivalent)` if the expression has a negative sign
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
                    }
                    let remaining: Vec<Arc<Expr>> = factors[1..].to_vec();
                    return Some(Expr::product_from_arcs(remaining));
                }
                // Other negative coefficient like -2, -3.5: replace with positive
                let mut new_factors: Vec<Arc<Expr>> = Vec::with_capacity(factors.len());
                new_factors.push(Arc::new(Expr::number(abs_coeff)));
                new_factors.extend(factors[1..].iter().cloned());
                return Some(Expr::product_from_arcs(new_factors));
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

/// Helper for Power base parenthesis
fn needs_parens_as_base(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Sum(_) | ExprKind::Product(_) | ExprKind::Div(_, _) | ExprKind::Poly(_) => true,
        ExprKind::Number(n) => *n < 0.0, // Negative numbers need parens: (-1)^x not -1^x
        _ => false,
    }
}

/// Consolidated recursive formatter call
fn format_recursive(
    f: &mut fmt::Formatter<'_>,
    expr: &Expr,
    mode: FormatMode,
    cache: Option<&SymbolCache>,
) -> fmt::Result {
    match mode {
        FormatMode::Standard => write!(f, "{expr}"),
        FormatMode::Latex => write!(f, "{}", LatexFormatter { expr, cache }),
        FormatMode::Unicode => write!(f, "{}", UnicodeFormatter { expr, cache }),
    }
}

/// Consolidated symbol formatting
fn format_symbol_expr(
    f: &mut fmt::Formatter<'_>,
    expr: &Expr,
    mode: FormatMode,
    cache: Option<&SymbolCache>,
) -> fmt::Result {
    let ExprKind::Symbol(s) = &expr.kind else {
        return Err(fmt::Error);
    };

    let name = cache.map_or_else(
        || s.as_ref(),
        |c| c.get(&s.id()).map_or_else(|| s.as_ref(), Arc::as_ref),
    );

    match mode {
        FormatMode::Standard => write!(f, "{name}"),
        FormatMode::Latex => {
            if let Some(greek) = greek_to_latex(name) {
                write!(f, "{greek}")
            } else {
                write!(f, "{name}")
            }
        }
        FormatMode::Unicode => {
            if let Some(greek) = greek_to_unicode(name) {
                write!(f, "{greek}")
            } else {
                write!(f, "{name}")
            }
        }
    }
}

/// Consolidated parenthesis wrapping logic
fn format_wrapped(
    f: &mut fmt::Formatter<'_>,
    expr: &Expr,
    mode: FormatMode,
    context: ParenContext,
    cache: Option<&SymbolCache>,
) -> fmt::Result {
    let needs = match context {
        ParenContext::SumOrProduct => matches!(expr.kind, ExprKind::Sum(_) | ExprKind::Poly(_)),
        ParenContext::PowerBase => needs_parens_as_base(expr),
    };

    if needs {
        let (open, close) = match mode {
            FormatMode::Standard | FormatMode::Unicode => ("(", ")"),
            FormatMode::Latex => (r"\left(", r"\right)"),
        };
        write!(f, "{open}")?;
        format_recursive(f, expr, mode, cache)?;
        write!(f, "{close}")
    } else {
        format_recursive(f, expr, mode, cache)
    }
}

/// Unified Sum formatting
fn format_sum_expr(
    f: &mut fmt::Formatter<'_>,
    terms: &[Arc<Expr>],
    mode: FormatMode,
    cache: Option<&SymbolCache>,
) -> fmt::Result {
    if terms.is_empty() {
        return write!(f, "0");
    }

    let plus = " + ";
    let (minus, minus_sep) = match mode {
        FormatMode::Standard | FormatMode::Latex => ("-", " - "),
        FormatMode::Unicode => ("\u{2212}", " \u{2212} "),
    };

    let mut first = true;
    for term in terms {
        if first {
            if let Some(positive_term) = extract_negative(term) {
                write!(f, "{minus}")?;
                format_wrapped(f, &positive_term, mode, ParenContext::SumOrProduct, cache)?;
            } else {
                format_wrapped(f, term, mode, ParenContext::SumOrProduct, cache)?;
            }
            first = false;
        } else if let Some(positive_term) = extract_negative(term) {
            write!(f, "{minus_sep}")?;
            format_wrapped(f, &positive_term, mode, ParenContext::SumOrProduct, cache)?;
        } else {
            write!(f, "{plus}")?;
            format_wrapped(f, term, mode, ParenContext::SumOrProduct, cache)?;
        }
    }
    Ok(())
}

/// Unified Product formatting
fn format_product_expr(
    f: &mut fmt::Formatter<'_>,
    factors: &[Arc<Expr>],
    mode: FormatMode,
    cache: Option<&SymbolCache>,
) -> fmt::Result {
    if factors.is_empty() {
        return write!(f, "1");
    }

    let sep = match mode {
        FormatMode::Standard | FormatMode::Unicode => {
            if matches!(mode, FormatMode::Unicode) {
                "\u{b7}"
            } else {
                "*"
            }
        }
        FormatMode::Latex => r" \cdot ",
    };

    let minus = match mode {
        FormatMode::Standard | FormatMode::Latex => "-",
        FormatMode::Unicode => "\u{2212}",
    };

    // Check for leading -1
    if !factors.is_empty()
        && let ExprKind::Number(n) = &factors[0].kind
        && (*n + 1.0).abs() < EPSILON
    {
        if factors.len() == 1 {
            return write!(f, "{minus}1");
        }
        let remaining: Vec<Arc<Expr>> = factors[1..].to_vec();
        let rest = Expr::product_from_arcs(remaining);
        write!(f, "{minus}")?;
        return format_wrapped(f, &rest, mode, ParenContext::SumOrProduct, cache);
    }

    let mut first = true;
    for fac in factors {
        if !first {
            write!(f, "{sep}")?;
        }
        format_wrapped(f, fac, mode, ParenContext::SumOrProduct, cache)?;
        first = false;
    }
    Ok(())
}

/// Unified Division formatting
fn format_div_expr(
    f: &mut fmt::Formatter<'_>,
    u: &Expr,
    v: &Expr,
    mode: FormatMode,
    cache: Option<&SymbolCache>,
) -> fmt::Result {
    if matches!(mode, FormatMode::Latex) {
        write!(f, r"\frac{{")?;
        format_recursive(f, u, mode, cache)?;
        write!(f, "}}{{")?;
        format_recursive(f, v, mode, cache)?;
        return write!(f, "}}");
    }

    // Standard/Unicode logic
    // Parenthesize numerator if it's a sum
    if matches!(u.kind, ExprKind::Sum(_)) {
        write!(f, "(")?;
        format_recursive(f, u, mode, cache)?;
        write!(f, ")/")?;
    } else {
        format_recursive(f, u, mode, cache)?;
        write!(f, "/")?;
    }

    // Parenthesize denominator if it's not simple
    let denom_simple = matches!(
        v.kind,
        ExprKind::Symbol(_)
            | ExprKind::Number(_)
            | ExprKind::Pow(_, _)
            | ExprKind::FunctionCall { .. }
    );

    if denom_simple {
        format_recursive(f, v, mode, cache)
    } else {
        write!(f, "(")?;
        format_recursive(f, v, mode, cache)?;
        write!(f, ")")
    }
}

/// Unified Power formatting
fn format_pow_expr(
    f: &mut fmt::Formatter<'_>,
    u: &Expr,
    v: &Expr,
    mode: FormatMode,
    cache: Option<&SymbolCache>,
) -> fmt::Result {
    // Special case: e^x displays as exp(x) (except in LaTeX which uses e^{x})
    if !matches!(mode, FormatMode::Latex)
        && let ExprKind::Symbol(s) = &u.kind
        && s.id() == ks::KS.e
    {
        write!(f, "exp(")?;
        format_recursive(f, v, mode, cache)?;
        return write!(f, ")");
    }

    if matches!(mode, FormatMode::Latex) {
        if let ExprKind::Symbol(s) = &u.kind
            && s.id() == ks::KS.e
        {
            write!(f, "e^{{")?;
            format_recursive(f, v, mode, cache)?;
            return write!(f, "}}");
        }

        if needs_parens_as_base(u) {
            write!(f, r"\left(")?;
            format_recursive(f, u, mode, cache)?;
            write!(f, r"\right)^{{")?;
        } else {
            format_recursive(f, u, mode, cache)?;
            write!(f, "^{{")?;
        }
        format_recursive(f, v, mode, cache)?;
        return write!(f, "}}");
    }

    // Standard/Unicode logic
    format_wrapped(f, u, mode, ParenContext::PowerBase, cache)?;

    if matches!(mode, FormatMode::Unicode) {
        if let ExprKind::Number(n) = &v.kind {
            write!(f, "{}", num_to_superscript(*n))
        } else if matches!(v.kind, ExprKind::Symbol(_)) {
            write!(f, "^")?;
            format_recursive(f, v, mode, cache)
        } else {
            write!(f, "^(")?;
            format_recursive(f, v, mode, cache)?;
            write!(f, ")")
        }
    } else {
        let exp_simple = matches!(v.kind, ExprKind::Number(_) | ExprKind::Symbol(_));
        if exp_simple {
            write!(f, "^")?;
            format_recursive(f, v, mode, cache)
        } else {
            write!(f, "^(")?;
            format_recursive(f, v, mode, cache)?;
            write!(f, ")")
        }
    }
}

/// Unified Function Call formatting
#[allow(clippy::too_many_lines)]
fn format_function_call_expr(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    args: &[Arc<Expr>],
    mode: FormatMode,
    cache: Option<&SymbolCache>,
) -> fmt::Result {
    if matches!(mode, FormatMode::Latex) {
        // Special formatting for specific functions in LaTeX
        match name {
            // === ROOTS ===
            "sqrt" if args.len() == 1 => {
                return write!(
                    f,
                    r"\sqrt{{{}}}",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }
            "cbrt" if args.len() == 1 => {
                return write!(
                    f,
                    r"\sqrt[3]{{{}}}",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }

            // === ABSOLUTE VALUE ===
            "abs" if args.len() == 1 => {
                return write!(
                    f,
                    r"\left|{}\right|",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }

            // === FLOOR/CEIL ===
            "floor" if args.len() == 1 => {
                return write!(
                    f,
                    r"\lfloor{}\rfloor",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }
            "ceil" if args.len() == 1 => {
                return write!(
                    f,
                    r"\lceil{}\rceil",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }

            // === BESSEL FUNCTIONS: J_n(x), Y_n(x), I_n(x), K_n(x) ===
            "besselj" if args.len() == 2 => {
                return write!(
                    f,
                    r"J_{{{}}}\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[1],
                        cache
                    }
                );
            }
            "bessely" if args.len() == 2 => {
                return write!(
                    f,
                    r"Y_{{{}}}\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[1],
                        cache
                    }
                );
            }
            "besseli" if args.len() == 2 => {
                return write!(
                    f,
                    r"I_{{{}}}\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[1],
                        cache
                    }
                );
            }
            "besselk" if args.len() == 2 => {
                return write!(
                    f,
                    r"K_{{{}}}\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[1],
                        cache
                    }
                );
            }

            // === ORTHOGONAL POLYNOMIALS ===
            "hermite" if args.len() == 2 => {
                return write!(
                    f,
                    r"H_{{{}}}\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[1],
                        cache
                    }
                );
            }
            "assoc_legendre" if args.len() == 3 => {
                return write!(
                    f,
                    r"P_{{{}}}^{{{}}}\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[1],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[2],
                        cache
                    }
                );
            }
            "spherical_harmonic" | "ynm" if args.len() == 4 => {
                return write!(
                    f,
                    r"Y_{{{}}}^{{{}}}\left({}, {}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[1],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[2],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[3],
                        cache
                    }
                );
            }

            // === POLYGAMMA FUNCTIONS ===
            "digamma" if args.len() == 1 => {
                return write!(
                    f,
                    r"\psi\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }
            "trigamma" if args.len() == 1 => {
                return write!(
                    f,
                    r"\psi_1\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }
            "tetragamma" if args.len() == 1 => {
                return write!(
                    f,
                    r"\psi_2\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }
            "polygamma" if args.len() == 2 => {
                return write!(
                    f,
                    r"\psi^{{({})}}\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[1],
                        cache
                    }
                );
            }

            // === ELLIPTIC INTEGRALS ===
            "elliptic_k" if args.len() == 1 => {
                return write!(
                    f,
                    r"K\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }
            "elliptic_e" if args.len() == 1 => {
                return write!(
                    f,
                    r"E\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }

            // === ZETA ===
            "zeta" if args.len() == 1 => {
                return write!(
                    f,
                    r"\zeta\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }
            "zeta_deriv" if args.len() == 2 => {
                return write!(
                    f,
                    r"\zeta^{{({})}}\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[1],
                        cache
                    }
                );
            }

            // === LAMBERT W ===
            "lambertw" if args.len() == 1 => {
                return write!(
                    f,
                    r"W\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    }
                );
            }

            // === BETA ===
            "beta" if args.len() == 2 => {
                return write!(
                    f,
                    r"\mathrm{{B}}\left({}, {}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[1],
                        cache
                    }
                );
            }

            // === LOG WITH BASE ===
            "log" if args.len() == 2 => {
                return write!(
                    f,
                    r"\log_{{{}}}\left({}\right)",
                    LatexFormatter {
                        expr: &args[0],
                        cache
                    },
                    LatexFormatter {
                        expr: &args[1],
                        cache
                    }
                );
            }

            _ => {}
        }

        // Standard function name LaTeX mappings
        let latex_name = match name {
            // Trigonometric
            "sin" | "cos" | "tan" | "cot" | "sec" | "csc" | "sinh" | "cosh" | "tanh" | "coth" => {
                format!(r"\{name}")
            }
            "sech" => r"\operatorname{sech}".to_owned(),
            "csch" => r"\operatorname{csch}".to_owned(),
            // Inverse hyperbolic
            "asinh" => r"\operatorname{arsinh}".to_owned(),
            "acosh" => r"\operatorname{arcosh}".to_owned(),
            "atanh" => r"\operatorname{artanh}".to_owned(),
            "acoth" => r"\operatorname{arcoth}".to_owned(),
            "asech" => r"\operatorname{arsech}".to_owned(),
            "acsch" => r"\operatorname{arcsch}".to_owned(),
            // Logarithms
            "ln" => r"\ln".to_owned(),
            "log" | "log10" => r"\log".to_owned(),
            "log2" => r"\log_2".to_owned(),
            // Exponential
            "exp" => r"\exp".to_owned(),
            "exp_polar" => r"\operatorname{exp\_polar}".to_owned(),
            // Special functions
            "gamma" => r"\Gamma".to_owned(),
            "erf" => r"\operatorname{erf}".to_owned(),
            "erfc" => r"\operatorname{erfc}".to_owned(),
            "signum" => r"\operatorname{sgn}".to_owned(),
            "sinc" => r"\operatorname{sinc}".to_owned(),
            "round" => r"\operatorname{round}".to_owned(),
            // Default: wrap in \text{}
            _ => format!(r"\text{{{name}}}"),
        };

        if args.is_empty() {
            write!(f, r"{latex_name}\left(\right)")
        } else {
            write!(f, r"{latex_name}\left(")?;
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                format_recursive(f, arg, mode, cache)?;
            }
            write!(f, r"\right)")
        }
    } else {
        // Standard/Unicode logic
        if args.is_empty() {
            write!(f, "{name}()")
        } else {
            write!(f, "{name}(")?;
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                format_recursive(f, arg, mode, cache)?;
            }
            write!(f, ")")
        }
    }
}

/// Greek letter mappings: (name, latex, unicode)
/// Covers lowercase Greek alphabet commonly used in mathematics and physics
static GREEK_LETTERS: &[(&str, &str, &str)] = &[
    // Common mathematical symbols
    ("pi", r"\pi", "\u{3c0}"),
    ("alpha", r"\alpha", "\u{3b1}"),
    ("beta", r"\beta", "\u{3b2}"),
    ("gamma", r"\gamma", "\u{3b3}"),
    ("delta", r"\delta", "\u{3b4}"),
    ("epsilon", r"\epsilon", "\u{3b5}"),
    ("zeta", r"\zeta", "\u{3b6}"),
    ("eta", r"\eta", "\u{3b7}"),
    ("theta", r"\theta", "\u{3b8}"),
    ("iota", r"\iota", "\u{3b9}"),
    ("kappa", r"\kappa", "\u{3ba}"),
    ("lambda", r"\lambda", "\u{3bb}"),
    ("mu", r"\mu", "\u{3bc}"),
    ("nu", r"\nu", "\u{3bd}"),
    ("xi", r"\xi", "\u{3be}"),
    ("omicron", r"\omicron", "\u{3bf}"),
    ("rho", r"\rho", "\u{3c1}"),
    ("sigma", r"\sigma", "\u{3c3}"),
    ("tau", r"\tau", "\u{3c4}"),
    ("upsilon", r"\upsilon", "\u{3c5}"),
    ("phi", r"\phi", "\u{3c6}"),
    ("chi", r"\chi", "\u{3c7}"),
    ("psi", r"\psi", "\u{3c8}"),
    ("omega", r"\omega", "\u{3c9}"),
    // Alternative forms
    ("varepsilon", r"\varepsilon", "\u{3b5}"),
    ("vartheta", r"\vartheta", "\u{3d1}"),
    ("varphi", r"\varphi", "\u{3c6}"),
    ("varrho", r"\varrho", "\u{3c1}"),
    ("varsigma", r"\varsigma", "\u{3c2}"),
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

/// Format a number based on the display mode
fn format_number_expr(f: &mut fmt::Formatter<'_>, n: f64, mode: FormatMode) -> fmt::Result {
    if n.is_nan() {
        return match mode {
            FormatMode::Standard | FormatMode::Unicode => write!(f, "NaN"),
            FormatMode::Latex => write!(f, r"\text{{NaN}}"),
        };
    }
    if n.is_infinite() {
        return match mode {
            FormatMode::Standard => {
                if n > 0.0 {
                    write!(f, "Infinity")
                } else {
                    write!(f, "-Infinity")
                }
            }
            FormatMode::Latex => {
                if n > 0.0 {
                    write!(f, r"\infty")
                } else {
                    write!(f, r"-\infty")
                }
            }
            FormatMode::Unicode => {
                write!(f, "{}", if n > 0.0 { "\u{221e}" } else { "-\u{221e}" })
            }
        };
    }

    #[allow(clippy::float_cmp)]
    let is_int = n.trunc() == n;
    if is_int && n.abs() < 1e10 {
        #[allow(clippy::cast_possible_truncation)]
        let n_int = n as i64;
        write!(f, "{n_int}")
    } else {
        write!(f, "{n}")
    }
}

// =============================================================================
// DISPLAY IMPLEMENTATION
// =============================================================================

impl fmt::Display for Expr {
    // Large match blocks for different expression kinds
    #[allow(clippy::too_many_lines)] // Large match block for different expression kinds
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ExprKind::Number(n) => format_number_expr(f, *n, FormatMode::Standard),

            ExprKind::Symbol(_) => format_symbol_expr(f, self, FormatMode::Standard, None),

            ExprKind::FunctionCall { name, args } => {
                format_function_call_expr(f, name.as_str(), args, FormatMode::Standard, None)
            }

            // N-ary Sum: display with + and - signs
            ExprKind::Sum(terms) => format_sum_expr(f, terms, FormatMode::Standard, None),

            // N-ary Product: display with * or implicit multiplication
            ExprKind::Product(factors) => {
                format_product_expr(f, factors, FormatMode::Standard, None)
            }

            ExprKind::Div(u, v) => format_div_expr(f, u, v, FormatMode::Standard, None),

            ExprKind::Pow(u, v) => format_pow_expr(f, u, v, FormatMode::Standard, None),

            ExprKind::Derivative { inner, var, order } => {
                write!(f, "\u{2202}^{order}_{inner}/\u{2202}_{var}^{order}")
            }

            // Poly: display inline using Polynomial's Display
            ExprKind::Poly(poly) => {
                write!(f, "{poly}")
            }
        }
    }
}

// =============================================================================
// LATEX FORMATTER
// =============================================================================

pub struct LatexFormatter<'expr> {
    pub(crate) expr: &'expr Expr,
    pub(crate) cache: Option<&'expr SymbolCache>,
}

impl fmt::Display for LatexFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_latex(self.expr, f, self.cache)
    }
}

// Display format functions are naturally lengthy due to many expression kinds
#[allow(clippy::too_many_lines)] // Display format naturally lengthy due to many expr kinds
fn format_latex(
    expr: &Expr,
    f: &mut fmt::Formatter<'_>,
    cache: Option<&SymbolCache>,
) -> fmt::Result {
    match &expr.kind {
        ExprKind::Number(n) => format_number_expr(f, *n, FormatMode::Latex),

        ExprKind::Symbol(_) => format_symbol_expr(f, expr, FormatMode::Latex, cache),

        ExprKind::FunctionCall { name, args } => {
            format_function_call_expr(f, name.as_str(), args, FormatMode::Latex, cache)
        }

        ExprKind::Sum(terms) => format_sum_expr(f, terms, FormatMode::Latex, cache),

        ExprKind::Product(factors) => format_product_expr(f, factors, FormatMode::Latex, cache),

        ExprKind::Div(u, v) => format_div_expr(f, u, v, FormatMode::Latex, cache),

        ExprKind::Pow(u, v) => format_pow_expr(f, u, v, FormatMode::Latex, cache),

        ExprKind::Derivative { inner, var, order } => {
            if *order == 1 {
                write!(
                    f,
                    r"\frac{{\partial {}}}{{\partial {}}}",
                    LatexFormatter { expr: inner, cache },
                    var
                )
            } else {
                write!(
                    f,
                    r"\frac{{\partial^{} {}}}{{\partial {}^{}}}",
                    order,
                    LatexFormatter { expr: inner, cache },
                    var,
                    order
                )
            }
        }

        // Poly: display inline in LaTeX
        ExprKind::Poly(poly) => write!(f, "{poly}"),
    }
}

// =============================================================================
// UNICODE FORMATTER
// =============================================================================

pub struct UnicodeFormatter<'expr> {
    pub(crate) expr: &'expr Expr,
    pub(crate) cache: Option<&'expr SymbolCache>,
}

impl fmt::Display for UnicodeFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_unicode(self.expr, f, self.cache)
    }
}

#[inline]
const fn to_superscript(c: char) -> char {
    match c {
        '0' => '\u{2070}',
        '1' => '\u{b9}',
        '2' => '\u{b2}',
        '3' => '\u{b3}',
        '4' => '\u{2074}',
        '5' => '\u{2075}',
        '6' => '\u{2076}',
        '7' => '\u{2077}',
        '8' => '\u{2078}',
        '9' => '\u{2079}',
        '-' => '\u{207b}',
        '+' => '\u{207a}',
        '(' => '\u{207d}',
        ')' => '\u{207e}',
        _ => c,
    }
}

#[inline]
fn num_to_superscript(n: f64) -> String {
    if {
        #[allow(clippy::float_cmp)] // Checking for exact integer via trunc
        let is_int = n.trunc() == n;
        is_int
    } && n.abs() < 1000.0
    {
        #[allow(clippy::cast_possible_truncation)] // Checked is_int above
        let n_int = n as i64;
        format!("{n_int}").chars().map(to_superscript).collect()
    } else {
        format!("^{n}")
    }
}

// Display format functions are naturally lengthy due to many expression kinds
#[allow(clippy::too_many_lines)] // Display format naturally lengthy due to many expr kinds
fn format_unicode(
    expr: &Expr,
    f: &mut fmt::Formatter<'_>,
    cache: Option<&SymbolCache>,
) -> fmt::Result {
    match &expr.kind {
        ExprKind::Number(n) => format_number_expr(f, *n, FormatMode::Unicode),

        ExprKind::Symbol(_) => format_symbol_expr(f, expr, FormatMode::Unicode, cache),

        ExprKind::FunctionCall { name, args } => {
            format_function_call_expr(f, name.as_str(), args, FormatMode::Unicode, cache)
        }

        ExprKind::Sum(terms) => format_sum_expr(f, terms, FormatMode::Unicode, cache),

        ExprKind::Product(factors) => format_product_expr(f, factors, FormatMode::Unicode, cache),

        ExprKind::Div(u, v) => format_div_expr(f, u, v, FormatMode::Unicode, cache),

        ExprKind::Pow(u, v) => format_pow_expr(f, u, v, FormatMode::Unicode, cache),

        ExprKind::Derivative { inner, var, order } => {
            let sup = num_to_superscript(f64::from(*order));
            write!(
                f,
                "\u{2202}{}({})/\u{2202}{}{}",
                sup,
                UnicodeFormatter { expr: inner, cache },
                var,
                sup
            )
        }

        // Poly: display inline in unicode
        ExprKind::Poly(poly) => write!(f, "{poly}"),
    }
}

// =============================================================================
// EXPR FORMATTING METHODS
// =============================================================================

impl Expr {
    /// Convert the expression to LaTeX format.
    ///
    /// Returns a string suitable for rendering in LaTeX math environments.
    #[must_use]
    pub fn to_latex(&self) -> String {
        let mut cache = SymbolCache::default();
        collect_symbol_names(self, &mut cache);
        format!(
            "{}",
            LatexFormatter {
                expr: self,
                cache: Some(&cache)
            }
        )
    }

    /// Convert the expression to Unicode format.
    ///
    /// Returns a string with Unicode superscripts and Greek letters for display.
    #[must_use]
    pub fn to_unicode(&self) -> String {
        let mut cache = SymbolCache::default();
        collect_symbol_names(self, &mut cache);
        format!(
            "{}",
            UnicodeFormatter {
                expr: self,
                cache: Some(&cache)
            }
        )
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
// Standard test relaxations: unwrap/panic for assertions, precision loss for math
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::let_underscore_must_use,
    clippy::no_effect_underscore_binding
)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    #[allow(clippy::approx_constant)] // Testing exact float display, not mathematical approximation
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
            HashMap::new(),
            None,
            None,
            None,
            false,
        );
        assert_eq!(format!("{expr}"), "1 + x"); // Sorted after simplify: numbers before symbols
    }

    #[test]
    fn test_display_product() {
        let prod = Expr::product(vec![Expr::number(2.0), Expr::symbol("x")]);
        assert_eq!(format!("{prod}"), "2*x");
    }

    #[test]
    fn test_display_negation() {
        let expr = Expr::product(vec![Expr::number(-1.0), Expr::symbol("x")]);
        assert_eq!(format!("{expr}"), "-x");
    }

    #[test]
    fn test_display_subtraction() {
        // x - y = Sum([x, Product([-1, y])])
        let expr = Expr::sub_expr(Expr::symbol("x"), Expr::symbol("y"));
        let display = format!("{expr}");
        // Should display as subtraction
        assert!(
            display.contains('-'),
            "Expected subtraction, got: {display}"
        );
    }

    #[test]
    fn test_display_nested_sum() {
        // Test: x + (y + z) should display with parentheses
        let inner_sum = Expr::sum(vec![Expr::symbol("y"), Expr::symbol("z")]);
        let expr = Expr::sum(vec![Expr::symbol("x"), inner_sum]);
        let display = format!("{expr}");
        // Should display as "x + (y + z)" to preserve structure
        assert_eq!(display, "x + y + z");
    }
}
