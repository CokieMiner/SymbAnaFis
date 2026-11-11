//! Precedence and normalized construction over generated scalar expressions.

use core::{
    fmt::Write,
    ops::{Add, Div, Mul, Neg, Sub},
};

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use proptest::{collection, prelude::Strategy, prop_assert_eq, prop_assume, prop_oneof, proptest};

use crate::core::{Context, Expr, Property, Truth};

/// A generated expression: a symbol among three, a digit, a negation, a sine
/// call, or a binary operation by its index in `+ - * / ^`.
#[derive(Clone, Debug)]
enum Tree {
    Symbol(usize),
    Digit(u8),
    Negate(Box<Self>),
    Sine(Box<Self>),
    Binary(usize, Box<Self>, Box<Self>),
}

fn tree() -> impl Strategy<Value = Tree> {
    let leaf = prop_oneof![
        (0_usize..3).prop_map(Tree::Symbol),
        (0_u8..=9).prop_map(Tree::Digit),
    ];
    leaf.prop_recursive(5, 32, 2, |inner| {
        prop_oneof![
            inner
                .clone()
                .prop_map(|child| Tree::Negate(Box::new(child))),
            inner.clone().prop_map(|child| Tree::Sine(Box::new(child))),
            (0_usize..5, inner.clone(), inner).prop_map(|(operation, left, right)| {
                Tree::Binary(operation, Box::new(left), Box::new(right))
            }),
        ]
    })
}

/// Write `tree` with the parentheses that its precedence needs below an
/// operator of precedence `minimum`, and build it with the arithmetic operators.
/// Sums and products are left-associative, powers right-associative, and
/// unary minus binds below powers: \(-x^2=-(x^2)\). `undefined` records a
/// built node that is proved nonfinite, such as a quotient by zero, whose
/// normal form depends on the grouping of unit factors around it.
fn render(
    tree: &Tree,
    minimum: u8,
    symbols: &[Expr; 3],
    source: &mut String,
    undefined: &mut bool,
) -> Expr {
    let precedence = match tree {
        Tree::Symbol(_) | Tree::Digit(_) | Tree::Sine(_) => 5,
        Tree::Negate(_) => 3,
        Tree::Binary(operation, ..) => [1, 1, 2, 2, 4].get(*operation).copied().unwrap_or(5),
    };
    let grouped = precedence < minimum;
    if grouped {
        source.push('(');
    }
    let built = match tree {
        Tree::Symbol(index) => {
            source.push(['x', 'y', 'z'].get(*index).copied().unwrap_or('x'));
            symbols
                .get(*index)
                .cloned()
                .unwrap_or_else(|| Expr::number(0))
        }
        Tree::Digit(digit) => {
            source.push(char::from(b'0'.saturating_add(*digit)));
            Expr::number(i64::from(*digit))
        }
        Tree::Negate(child) => {
            source.push('-');
            Neg::neg(render(child, 3, symbols, source, undefined))
        }
        Tree::Sine(child) => {
            source.push_str("sin (");
            let argument = render(child, 0, symbols, source, undefined);
            source.push(')');
            argument.sin()
        }
        Tree::Binary(operation, first, second) => {
            // A power's base binds tighter than any operator; its exponent may
            // be negated. A left-associative right operand of equal precedence
            // needs parentheses.
            let (left_minimum, right_minimum) = if precedence == 4 {
                (5, 3)
            } else {
                (precedence, precedence.saturating_add(1))
            };
            let left = render(first, left_minimum, symbols, source, undefined);
            source.push(
                ['+', '-', '*', '/', '^']
                    .get(*operation)
                    .copied()
                    .unwrap_or('+'),
            );
            let right = render(second, right_minimum, symbols, source, undefined);
            match operation {
                0 => Add::add(left, right),
                1 => Sub::sub(left, right),
                2 => Mul::mul(left, right),
                3 => Div::div(left, right),
                _ => left.pow(right),
            }
        }
    };
    if grouped {
        source.push(')');
    }
    *undefined |= built.properties().get(Property::Finite) == Truth::False;
    built
}

proptest! {
    #[test]
    fn generated_sources_parse_to_their_operator_construction(tree in tree()) {
        let context = Context::new()?;
        let symbols = [
            context.symb("x")?.to_expr(),
            context.symb("y")?.to_expr(),
            context.symb("z")?.to_expr(),
        ];
        let mut source = String::new();
        let mut undefined = false;
        let expected = render(&tree, 0, &symbols, &mut source, &mut undefined);
        prop_assume!(!undefined);
        prop_assert_eq!(context.parse(&source)?, expected, "source: {}", source);
    }

    #[test]
    fn polynomial_sources_share_the_operator_dag(
        terms in collection::vec((-10_i32..=10, 1_i32..=12), 1..=24),
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let mut source = String::new();
        let mut addends = Vec::new();
        for (coefficient, exponent) in terms {
            if !source.is_empty() { source.push('+'); }
            write!(source, "({coefficient})x^{exponent}")?;
            addends.push(Mul::mul(coefficient, x.pow(exponent)));
        }
        prop_assert_eq!(context.parse(&source)?, Expr::sum(addends));
    }

    #[test]
    fn product_chains_reuse_normalized_powers(
        exponents in collection::vec(1_i32..=12, 1..=32),
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let mut source = String::new();
        let mut factors = Vec::new();
        for exponent in exponents {
            if !source.is_empty() { source.push('*'); }
            write!(source, "x^{exponent}")?;
            factors.push(x.pow(exponent));
        }
        prop_assert_eq!(context.parse(&source)?, Expr::product(factors));
    }

    #[test]
    fn binary_and_unary_precedence_is_mathematical(
        a in -16_i32..=16,
        b in -16_i32..=16,
        exponent in 1_i32..=8,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        prop_assert_eq!(context.parse(&format!("{a} + {b}*x^ {exponent}"))?, Add::add(a, Mul::mul(b, x.pow(exponent))));
        prop_assert_eq!(context.parse(&format!("-x**{exponent}"))?, Neg::neg(x.pow(exponent)));
        prop_assert_eq!(context.parse(&format!("x^-{exponent}"))?, x.pow(Neg::neg(i64::from(exponent))));
        prop_assert_eq!(context.parse("x^y^x")?, x.pow(y.pow(x)));
        prop_assert_eq!(context.parse("x/y*x")?, Mul::mul(Div::div(x, y), x));
    }

    #[test]
    fn explicit_and_implicit_multiplication_agree(
        coefficient in -100_i32..=100,
        whitespace in "[ \t\n]{0,12}",
    ) {
        let context = Context::new()?;
        let expected = context.parse(&format!("{coefficient} * x * (x + 1) * sin(x)"))?;
        let source = format!("({coefficient}){whitespace}x {whitespace}(x+1){whitespace}sin(x)");
        prop_assert_eq!(context.parse(&source)?, expected);
    }

    #[test]
    fn decimal_and_scientific_literals_keep_float_identity(
        numerator in -10_000_i32..=10_000,
        exponent in -8_i32..=8,
    ) {
        let context = Context::new()?;
        for source in [format!("{numerator}.0e{exponent}"), format!("{numerator}e{exponent}")] {
            let value = source.parse::<f64>().expect("generated finite decimal");
            prop_assert_eq!(context.parse(&source)?, Expr::number(value));
        }
    }

    #[test]
    fn exact_rational_syntax_uses_backend_numbers(
        numerator in -1000_i32..=1000,
        denominator in 1_i32..=1000,
    ) {
        let context = Context::new()?;
        let source = format!("{numerator}/{denominator}");
        prop_assert_eq!(context.parse(&source)?, Div::div(Expr::number(numerator), denominator));
    }

    #[test]
    fn digit_separators_preserve_integer_and_float_representations(
        integer in proptest::num::i64::ANY,
        fractional in 0_u16..=9999,
        exponent in -8_i32..=8,
    ) {
        let context = Context::new()?;
        let decimal = format!("{integer}.{fractional:04}e{exponent}");
        let approximate = decimal.parse::<f64>().expect("generated finite decimal");
        for (literal, expected) in [
            (integer.to_string(), Expr::number(integer)),
            (decimal, Expr::number(approximate)),
        ] {
            let mut source = String::new();
            let mut previous_digit = false;
            for character in literal.chars() {
                if previous_digit && character.is_ascii_digit() { source.push('_'); }
                source.push(character);
                previous_digit = character.is_ascii_digit();
            }
            prop_assert_eq!(context.parse(&source)?, expected);
        }
    }
}

#[test]
fn deferred_addition_matches_canonical_construction() {
    let context = Context::new().expect("context");
    let x = context.symb("deferred_x").expect("free symbol").to_expr();
    let y = context.symb("deferred_y").expect("free symbol").to_expr();
    let z = context.symb("deferred_z").expect("free symbol").to_expr();
    let base = Expr::sum(vec![x.clone(), y.clone()]);
    let cases = [
        (
            "deferred_x^3 + deferred_x^2 + (deferred_x+deferred_y) + (deferred_x+deferred_y)^2",
            Expr::sum(vec![x.pow(3), x.pow(2), base.clone(), base.pow(2)]),
        ),
        (
            "-(deferred_x*deferred_y) + deferred_x^2",
            Expr::sum(vec![-(x.clone() * y.clone()), x.clone().pow(2)]),
        ),
        (
            "deferred_x - deferred_y*deferred_z + 2",
            Expr::sum(vec![x.clone(), -(y.clone() * z.clone()), Expr::number(2)]),
        ),
        (
            "deferred_x^2 + deferred_x^2 + deferred_x^3",
            Expr::sum(vec![Expr::number(2) * x.clone().pow(2), x.pow(3)]),
        ),
        (
            "deferred_x*deferred_y + deferred_x*deferred_z",
            Expr::sum(vec![x.clone() * y.clone(), x.clone() * z]),
        ),
        (
            "1 + deferred_x^2 + deferred_x^3",
            Expr::sum(vec![Expr::number(1), x.clone().pow(2), x.pow(3)]),
        ),
        (
            "deferred_x^3 - 48*deferred_x*deferred_y + 3*deferred_x^2*deferred_y",
            Expr::sum(vec![
                x.clone().pow(3),
                Expr::product(vec![Expr::number(-48), x.clone(), y.clone()]),
                Expr::product(vec![Expr::number(3), x.pow(2), y]),
            ]),
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(context.parse(source), Ok(expected), "source: {source}");
    }
}
