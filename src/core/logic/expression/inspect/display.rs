//! Iterative text rendering, including deep DAG paths.

use core::fmt::{Display, Formatter, Result as FmtResult, Write};

use alloc::{string::String, vec, vec::Vec};

use super::{Expr, Node, NumericValue, Polynomial};

/// The spellings of one output style. LaTeX also changes structure: function
/// names, quotients, and exponents take braces.
struct Style {
    latex: bool,
    open: &'static str,
    close: &'static str,
    multiply: &'static str,
    minus: &'static str,
    minus_space: &'static str,
    pi: &'static str,
    euler: &'static str,
}

enum Task<'expr> {
    Expression(&'expr Expr, u8, bool),
    Literal(&'expr NumericValue, bool),
    Text(&'static str),
    Name(&'expr str),
    Anonymous(usize),
    Order(u32),
    IntegerPower(&'expr Expr, u32),
}

impl Style {
    const ASCII: Self = Self {
        latex: false,
        open: "(",
        close: ")",
        multiply: "*",
        minus: "-",
        minus_space: " - ",
        pi: "pi",
        euler: "e",
    };
    const LATEX: Self = Self {
        latex: true,
        open: "\\left(",
        close: "\\right)",
        multiply: "\\cdot ",
        minus: "-",
        minus_space: " - ",
        pi: "\\pi",
        euler: "\\mathrm{e}",
    };
    const UNICODE: Self = Self {
        latex: false,
        open: "(",
        close: ")",
        multiply: "\u{00b7}",
        minus: "\u{2212}",
        minus_space: " \u{2212} ",
        pi: "\u{03c0}",
        euler: "e",
    };
}

impl Display for Expr {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        render(self, &Style::ASCII, formatter)
    }
}

impl Expr {
    /// Render operators and function names as LaTeX.
    #[must_use]
    pub fn to_latex(&self) -> String {
        let mut text = String::new();
        // A `String` accepts every write, so rendering into it reports no error.
        render(self, &Style::LATEX, &mut text).unwrap_or_default();
        text
    }

    /// Render using Unicode multiplication and minus signs.
    #[must_use]
    pub fn to_unicode(&self) -> String {
        let mut text = String::new();
        // A `String` accepts every write, so rendering into it reports no error.
        render(self, &Style::UNICODE, &mut text).unwrap_or_default();
        text
    }
}

fn render(expression: &Expr, style: &Style, output: &mut impl Write) -> FmtResult {
    let mut pending = vec![Task::Expression(expression, 0, false)];
    while let Some(task) = pending.pop() {
        match task {
            Task::Expression(node, parent, magnitude) => {
                schedule(node, parent, magnitude, style, &mut pending);
            }
            Task::Literal(value, magnitude) => {
                if magnitude {
                    value.format_magnitude(output)?;
                } else {
                    write!(output, "{value}")?;
                }
            }
            Task::Text(text) => output.write_str(text)?,
            Task::Name(name) if style.latex => {
                for character in name.chars() {
                    match character {
                        '_' | '&' | '%' | '#' | '$' | '{' | '}' => {
                            write!(output, "\\{character}")?;
                        }
                        '\\' => output.write_str("\\backslash{}")?,
                        '^' => output.write_str("\\textasciicircum{}")?,
                        '~' => output.write_str("\\textasciitilde{}")?,
                        other => output.write_char(other)?,
                    }
                }
            }
            Task::Name(name) => output.write_str(name)?,
            Task::Anonymous(id) => write!(output, "${id}")?,
            Task::Order(order) => write!(output, "{order}")?,
            Task::IntegerPower(base, exponent) => {
                schedule_integer_power(base, exponent, style, &mut pending);
            }
        }
    }
    Ok(())
}

fn schedule<'expr>(
    expression: &'expr Expr,
    parent: u8,
    magnitude: bool,
    style: &Style,
    pending: &mut Vec<Task<'expr>>,
) {
    let brackets = precedence(expression.node()) < parent;
    if brackets {
        pending.push(Task::Text(style.close));
    }
    match expression.node() {
        Node::Number(value) => pending.push(Task::Literal(value, magnitude)),
        Node::ImaginaryUnit => pending.push(Task::Text("i")),
        Node::Pi => pending.push(Task::Text(style.pi)),
        Node::Euler => pending.push(Task::Text(style.euler)),
        Node::Symbol(data) => {
            pending.push(
                data.name()
                    .map_or_else(|| Task::Anonymous(data.id()), Task::Name),
            );
        }
        Node::Function { name, args } => schedule_function(name.as_str(), args, style, pending),
        Node::Sum(terms) => schedule_sum(terms, style, pending),
        Node::Poly(polynomial) => schedule_polynomial(polynomial, magnitude, style, pending),
        Node::Product(factors) => schedule_product(factors, magnitude, style, pending),
        Node::Div(left, right) => schedule_division(left, right, style, pending),
        Node::Pow(base, exponent) => schedule_power(base, exponent, style, pending),
        Node::Derivative { inner, var, order } => {
            pending.push(Task::Text(")"));
            pending.push(Task::Order(order.get()));
            pending.push(Task::Text(", "));
            pending.push(
                var.name()
                    .map_or_else(|| Task::Anonymous(var.id()), Task::Name),
            );
            pending.push(Task::Text(", "));
            pending.push(Task::Expression(inner, 0, false));
            pending.push(Task::Text("Derivative("));
        }
    }
    if brackets {
        pending.push(Task::Text(style.open));
    }
}

fn schedule_function<'expr>(
    name: &'expr str,
    args: &'expr [Expr],
    style: &Style,
    pending: &mut Vec<Task<'expr>>,
) {
    pending.push(Task::Text(style.close));
    for (index, argument) in args.iter().enumerate().rev() {
        pending.push(Task::Expression(argument, 0, false));
        if index != 0 {
            pending.push(Task::Text(", "));
        }
    }
    pending.push(Task::Text(style.open));
    if style.latex {
        pending.push(Task::Text("}"));
        pending.push(Task::Name(name));
        pending.push(Task::Text("\\operatorname{"));
    } else {
        pending.push(Task::Name(name));
    }
}

fn schedule_sum<'expr>(terms: &'expr [Expr], style: &Style, pending: &mut Vec<Task<'expr>>) {
    for (index, term) in terms.iter().enumerate().rev() {
        let negative = is_negative(term);
        pending.push(Task::Expression(term, 1, negative));
        if index != 0 {
            pending.push(Task::Text(if negative { style.minus_space } else { " + " }));
        } else if negative {
            pending.push(Task::Text(style.minus));
        }
    }
}

fn schedule_product<'expr>(
    factors: &'expr [Expr],
    magnitude: bool,
    style: &Style,
    pending: &mut Vec<Task<'expr>>,
) {
    let Some((first, rest)) = factors.split_first() else {
        return;
    };
    let negative_unit = first.literal().and_then(NumericValue::gaussian) == Some((-1, 0));
    for (index, factor) in rest.iter().enumerate().rev() {
        pending.push(Task::Expression(factor, 2, false));
        if index != 0 || !negative_unit {
            pending.push(Task::Text(style.multiply));
        }
    }
    if negative_unit {
        if !magnitude {
            pending.push(Task::Text(style.minus));
        }
    } else {
        pending.push(Task::Expression(first, 2, magnitude));
    }
}

/// A sum that already wrote the sign of the leading term passes `magnitude`.
fn schedule_polynomial<'expr>(
    polynomial: &'expr Polynomial,
    magnitude: bool,
    style: &Style,
    pending: &mut Vec<Task<'expr>>,
) {
    for (index, (coefficient, powers)) in polynomial.monomials().enumerate().rev() {
        let negative = is_negative(coefficient);
        let unit = matches!(
            coefficient.literal().and_then(NumericValue::gaussian),
            Some((1 | -1, 0))
        );
        for (position, (base, exponent)) in powers.enumerate().rev() {
            pending.push(Task::IntegerPower(base, exponent));
            if position != 0 || !unit {
                pending.push(Task::Text(style.multiply));
            }
        }
        if !unit {
            pending.push(Task::Expression(coefficient, 2, negative));
        }
        if index != 0 {
            pending.push(Task::Text(if negative { style.minus_space } else { " + " }));
        } else if negative && !magnitude {
            pending.push(Task::Text(style.minus));
        }
    }
}

fn schedule_integer_power<'expr>(
    base: &'expr Expr,
    exponent: u32,
    style: &Style,
    pending: &mut Vec<Task<'expr>>,
) {
    if exponent == 1 {
        pending.push(Task::Expression(base, 2, false));
        return;
    }
    if style.latex {
        pending.push(Task::Text("}"));
        pending.push(Task::Order(exponent));
        pending.push(Task::Text("^{"));
    } else {
        pending.push(Task::Order(exponent));
        pending.push(Task::Text("^"));
    }
    pending.push(Task::Expression(base, 4, false));
}

fn schedule_division<'expr>(
    left: &'expr Expr,
    right: &'expr Expr,
    style: &Style,
    pending: &mut Vec<Task<'expr>>,
) {
    if style.latex {
        pending.push(Task::Text("}"));
        pending.push(Task::Expression(right, 0, false));
        pending.push(Task::Text("}{"));
        pending.push(Task::Expression(left, 0, false));
        pending.push(Task::Text("\\frac{"));
    } else {
        pending.push(Task::Expression(right, 3, false));
        pending.push(Task::Text("/"));
        pending.push(Task::Expression(left, 2, false));
    }
}

fn schedule_power<'expr>(
    base: &'expr Expr,
    exponent: &'expr Expr,
    style: &Style,
    pending: &mut Vec<Task<'expr>>,
) {
    if style.latex {
        pending.push(Task::Text("}"));
        pending.push(Task::Expression(exponent, 0, false));
        pending.push(Task::Text("^{"));
    } else {
        pending.push(Task::Expression(exponent, 4, false));
        pending.push(Task::Text("^"));
    }
    pending.push(Task::Expression(base, 4, false));
}

/// Whether the leading literal coefficient is negative; compact storage leads
/// with its first monomial.
fn is_negative(expression: &Expr) -> bool {
    let coefficient = match expression.node() {
        Node::Product(factors) => factors.first().and_then(Expr::as_number),
        Node::Poly(polynomial) => polynomial
            .monomials()
            .next()
            .and_then(|(coefficient, _)| coefficient.as_number()),
        Node::Number(_)
        | Node::ImaginaryUnit
        | Node::Pi
        | Node::Euler
        | Node::Symbol(_)
        | Node::Function { .. }
        | Node::Sum(_)
        | Node::Div(..)
        | Node::Pow(..)
        | Node::Derivative { .. } => expression.as_number(),
    };
    coefficient.is_some_and(|value| value.is_finite() && value.is_negative())
}

/// Binding strength: sums 1, products and quotients 2, powers 3, atoms 4. A
/// literal that renders with a sign or an operator binds like that operator.
fn precedence(node: &Node) -> u8 {
    match node {
        Node::Sum(_) | Node::Poly(_) => 1,
        Node::Product(_) | Node::Div(..) => 2,
        Node::Pow(..) => 3,
        Node::Number(value) if value.value().is_complex() => 1,
        Node::Number(value) if value.value().is_rational() => 2,
        Node::Number(value) if value.value().is_negative() => 2,
        Node::Number(_)
        | Node::ImaginaryUnit
        | Node::Pi
        | Node::Euler
        | Node::Symbol(_)
        | Node::Function { .. }
        | Node::Derivative { .. } => 4,
    }
}
