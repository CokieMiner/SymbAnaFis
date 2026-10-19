//! Precedence and normalized construction over generated scalar expressions.

use core::{
    fmt::Write,
    ops::{Add, Div, Mul, Neg},
};

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use proptest::{collection, prop_assert_eq, proptest};

use crate::core::{Context, Expr};

proptest! {
    #[test]
    fn polynomial_sources_share_the_operator_dag(
        terms in collection::vec((-10_i32..=10, 1_i32..=12), 1..=24),
    ) {
        let context = Context::new();
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
        let context = Context::new();
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
        let context = Context::new();
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
        let context = Context::new();
        let expected = context.parse(&format!("{coefficient} * x * (x + 1) * sin(x)"))?;
        let source = format!("({coefficient}){whitespace}x {whitespace}(x+1){whitespace}sin(x)");
        prop_assert_eq!(context.parse(&source)?, expected);
    }

    #[test]
    fn decimal_and_scientific_literals_keep_float_identity(
        numerator in -10_000_i32..=10_000,
        exponent in -8_i32..=8,
    ) {
        let context = Context::new();
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
        let context = Context::new();
        let source = format!("{numerator}/{denominator}");
        prop_assert_eq!(context.parse(&source)?, Div::div(Expr::number(numerator), denominator));
    }

    #[test]
    fn digit_separators_preserve_integer_and_float_representations(
        integer in proptest::num::i64::ANY,
        fractional in 0_u16..=9999,
        exponent in -8_i32..=8,
    ) {
        let context = Context::new();
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
