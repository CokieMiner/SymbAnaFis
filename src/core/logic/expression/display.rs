//! Iterative text rendering, including deep DAG paths.

use core::fmt::{Display, Formatter, Result as FmtResult};

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

use super::{Expr, Node, NumericValue, Polynomial};

#[derive(Clone, Copy)]
enum Style {
    Ascii,
    Latex,
    Unicode,
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

struct Styled<'expr> {
    expression: &'expr Expr,
    style: Style,
}

impl Display for Expr {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        render(self, Style::Ascii, formatter)
    }
}

impl Expr {
    /// Render operators and function names as LaTeX.
    #[must_use]
    pub fn to_latex(&self) -> String {
        Styled {
            expression: self,
            style: Style::Latex,
        }
        .to_string()
    }

    /// Render using Unicode multiplication and minus signs.
    #[must_use]
    pub fn to_unicode(&self) -> String {
        Styled {
            expression: self,
            style: Style::Unicode,
        }
        .to_string()
    }
}

impl Display for Styled<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        render(self.expression, self.style, formatter)
    }
}

fn render(expression: &Expr, style: Style, formatter: &mut Formatter<'_>) -> FmtResult {
    let mut pending = vec![Task::Expression(expression, 0, false)];
    while let Some(task) = pending.pop() {
        match task {
            Task::Expression(node, parent, magnitude) => {
                schedule(node, parent, magnitude, style, &mut pending);
            }
            Task::Literal(value, magnitude) => {
                if magnitude {
                    value.format_magnitude(formatter)?;
                } else {
                    write!(formatter, "{value}")?;
                }
            }
            Task::Text(text) => formatter.write_str(text)?,
            Task::Name(name) => write_name(name, style, formatter)?,
            Task::Anonymous(id) => write!(formatter, "${id}")?,
            Task::Order(order) => write!(formatter, "{order}")?,
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
    style: Style,
    pending: &mut Vec<Task<'expr>>,
) {
    let precedence = precedence(expression.node());
    let brackets = precedence < parent;
    if brackets {
        pending.push(Task::Text(close(style)));
    }
    match expression.node() {
        Node::Number(value) => pending.push(Task::Literal(value, magnitude)),
        Node::ImaginaryUnit => pending.push(Task::Text("i")),
        Node::Pi => pending.push(Task::Text(match style {
            Style::Ascii => "pi",
            Style::Latex => "\\pi",
            Style::Unicode => "\u{03c0}",
        })),
        Node::Euler => pending.push(Task::Text(if matches!(style, Style::Latex) {
            "\\mathrm{e}"
        } else {
            "e"
        })),
        Node::Symbol(data) => {
            pending.push(
                data.name()
                    .map_or_else(|| Task::Anonymous(data.id()), Task::Name),
            );
        }
        Node::Function { name, args } => schedule_function(name.as_str(), args, style, pending),
        Node::Sum(terms) => schedule_sum(terms, style, pending),
        Node::Poly(polynomial) => schedule_polynomial(polynomial, style, pending),
        Node::Product(factors) => schedule_product(factors, magnitude, style, pending),
        Node::Div(left, right) => schedule_division(left, right, style, pending),
        Node::Pow(base, exponent) => schedule_power(base, exponent, style, pending),
        Node::Derivative { inner, var, order } => {
            pending.push(Task::Text(")"));
            pending.push(Task::Order(*order));
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
        pending.push(Task::Text(open(style)));
    }
}

fn schedule_function<'expr>(
    name: &'expr str,
    args: &'expr [Expr],
    style: Style,
    pending: &mut Vec<Task<'expr>>,
) {
    pending.push(Task::Text(close(style)));
    for (index, argument) in args.iter().enumerate().rev() {
        pending.push(Task::Expression(argument, 0, false));
        if index != 0 {
            pending.push(Task::Text(", "));
        }
    }
    pending.push(Task::Text(open(style)));
    if matches!(style, Style::Latex) {
        pending.push(Task::Text("}"));
        pending.push(Task::Name(name));
        pending.push(Task::Text("\\operatorname{"));
    } else {
        pending.push(Task::Name(name));
    }
}

fn schedule_sum<'expr>(terms: &'expr [Expr], style: Style, pending: &mut Vec<Task<'expr>>) {
    for (index, term) in terms.iter().enumerate().rev() {
        let negative = is_negative(term);
        pending.push(Task::Expression(term, 1, negative));
        if index != 0 {
            pending.push(Task::Text(if negative {
                minus_space(style)
            } else {
                " + "
            }));
        } else if negative {
            pending.push(Task::Text(minus(style)));
        }
    }
}

fn schedule_product<'expr>(
    factors: &'expr [Expr],
    magnitude: bool,
    style: Style,
    pending: &mut Vec<Task<'expr>>,
) {
    let Some((first, rest)) = factors.split_first() else {
        return;
    };
    let negative_unit = first.is_neg_one_num();
    for (index, factor) in rest.iter().enumerate().rev() {
        pending.push(Task::Expression(factor, 2, false));
        if index != 0 || !negative_unit {
            pending.push(Task::Text(multiply(style)));
        }
    }
    if negative_unit {
        if !magnitude {
            pending.push(Task::Text(minus(style)));
        }
    } else {
        pending.push(Task::Expression(first, 2, magnitude));
    }
}

fn schedule_polynomial<'expr>(
    polynomial: &'expr Polynomial,
    style: Style,
    pending: &mut Vec<Task<'expr>>,
) {
    for (index, term) in polynomial.terms.iter().enumerate().rev() {
        let coefficient = &term.coefficient;
        let negative = is_negative(coefficient);
        let unit = coefficient.is_one_num() || coefficient.is_neg_one_num();
        for (position, (base, exponent)) in term.powers.iter().enumerate().rev() {
            pending.push(Task::IntegerPower(base, *exponent));
            if position != 0 || !unit {
                pending.push(Task::Text(multiply(style)));
            }
        }
        if !unit {
            pending.push(Task::Expression(coefficient, 2, negative));
        }
        if index != 0 {
            pending.push(Task::Text(if negative {
                minus_space(style)
            } else {
                " + "
            }));
        } else if negative {
            pending.push(Task::Text(minus(style)));
        }
    }
}

fn schedule_integer_power<'expr>(
    base: &'expr Expr,
    exponent: u32,
    style: Style,
    pending: &mut Vec<Task<'expr>>,
) {
    if exponent == 1 {
        pending.push(Task::Expression(base, 2, false));
        return;
    }
    if matches!(style, Style::Latex) {
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
    style: Style,
    pending: &mut Vec<Task<'expr>>,
) {
    if matches!(style, Style::Latex) {
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
    style: Style,
    pending: &mut Vec<Task<'expr>>,
) {
    if matches!(style, Style::Latex) {
        pending.push(Task::Text("}"));
        pending.push(Task::Expression(exponent, 0, false));
        pending.push(Task::Text("^{"));
    } else {
        pending.push(Task::Expression(exponent, 4, false));
        pending.push(Task::Text("^"));
    }
    pending.push(Task::Expression(base, 4, false));
}

fn is_negative(expression: &Expr) -> bool {
    let coefficient = if let Node::Product(factors) = expression.node() {
        factors.first().and_then(Expr::as_number)
    } else {
        expression.as_number()
    };
    coefficient.is_some_and(|value| value.is_finite() && value.is_negative())
}

fn write_name(name: &str, style: Style, formatter: &mut Formatter<'_>) -> FmtResult {
    if matches!(style, Style::Latex) {
        for character in name.chars() {
            match character {
                '_' | '&' | '%' | '#' | '$' | '{' | '}' => write!(formatter, "\\{character}")?,
                '\\' => formatter.write_str("\\backslash{}")?,
                '^' => formatter.write_str("\\textasciicircum{}")?,
                '~' => formatter.write_str("\\textasciitilde{}")?,
                other => write!(formatter, "{other}")?,
            }
        }
        Ok(())
    } else {
        formatter.write_str(name)
    }
}

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

const fn open(style: Style) -> &'static str {
    if matches!(style, Style::Latex) {
        "\\left("
    } else {
        "("
    }
}
const fn close(style: Style) -> &'static str {
    if matches!(style, Style::Latex) {
        "\\right)"
    } else {
        ")"
    }
}
const fn multiply(style: Style) -> &'static str {
    match style {
        Style::Ascii => "*",
        Style::Latex => "\\cdot ",
        Style::Unicode => "\u{00b7}",
    }
}
const fn minus(style: Style) -> &'static str {
    if matches!(style, Style::Unicode) {
        "\u{2212}"
    } else {
        "-"
    }
}
const fn minus_space(style: Style) -> &'static str {
    if matches!(style, Style::Unicode) {
        " \u{2212} "
    } else {
        " - "
    }
}
