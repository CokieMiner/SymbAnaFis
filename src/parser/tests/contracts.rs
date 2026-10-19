//! Discrete syntax boundaries, function contracts, and expression ownership.

use core::ops::Add;

use alloc::{format, sync::Arc, vec, vec::Vec};

use proptest::{collection, prop_assert, prop_assert_eq, proptest};

use crate::{
    core::{Context, Expr, ExprView, Function, UserFunction},
    errors::{FunctionError, ParseError, Span},
};

proptest! {
    #[test]
    fn scoped_names_are_whole_and_independent(name in "[a-z][a-z0-9_]{2,24}") {
        let first_context = Context::new();
        let second_context = Context::new();
        let first = first_context.parse(&name)?;
        let second = second_context.parse(&name)?;
        prop_assert!(first != second);
        prop_assert_eq!(first.as_symbol(), first_context.get_symbol(&name));
        prop_assert_eq!(first_context.symbol_names(), vec![name]);
    }

    #[test]
    fn registered_calls_keep_their_definition_after_context_destruction(value in -100_i64..=100) {
        let (parsed, expected) = {
            let context = Context::new();
            let function = context.register_function(
                "custom",
                UserFunction::new(2..=2).body(Arc::new(|_| Expr::number(99))),
            )?;
            let x = context.symb("x")?;
            (context.parse(&format!("custom(x, {value})"))?, function.call(vec![x.into(), Expr::number(value)])?)
        };
        prop_assert_eq!(&parsed, &expected);
        let symbolic = matches!(parsed.view(), ExprView::Function { .. });
        prop_assert!(symbolic);
    }

    #[test]
    fn builtin_calls_dispatch_through_the_core(value in -100_i64..=100) {
        let context = Context::new();
        for name in ["sin", "cos", "sqrt", "exp", "ln", "abs", "zeta", "gamma"] {
            let function = Function::builtin(name).expect("known builtin");
            let input = format!("{name}({value})");
            prop_assert_eq!(context.parse(&input)?, function.call(vec![Expr::number(value)])?);
        }
    }

    #[test]
    fn arbitrary_symbolic_syntax_completes_without_panicking(source in "[xy_()+*/^, \t\n-]{0,256}") {
        let context = Context::new();
        if let Ok(expression) = context.parse(&source) {
            prop_assert!(expression.node_count() > 0);
        }
    }

    #[test]
    fn builtin_argument_order_and_aliases_follow_function_handles(
        first in -10_i32..=10,
        second in -10_i32..=10,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        for name in ["besselj", "bessel_j", "log", "lambert_w", "atan2", "beta"] {
            let function = Function::builtin(name).expect("known binary builtin or alias");
            let source = format!("{name}({first}+x,{second}+y)");
            prop_assert_eq!(context.parse(&source)?, function.call(vec![Add::add(first, x), Add::add(second, y)])?);
        }
        prop_assert_eq!(context.parse("assoc_legendre(3, 1, x)")?, x.assoc_legendre(3, 1));
        prop_assert_eq!(context.parse("spherical_harmonic(3, 1, x, y)")?, x.spherical_harmonic(3, 1, y));
    }

    #[test]
    fn binding_reuse_is_local_to_one_parse(
        names in collection::vec("v[a-z]{1,5}", 5..=24),
    ) {
        let context = Context::new();
        let source = names.join("+");
        let first = context.parse(&source)?;
        context.clear_symbols();
        let second = context.parse(&source)?;
        let mut terms = Vec::new();
        for name in names {
            let symbol = context.symb(&name)?;
            prop_assert!(!first.contains_symbol(symbol));
            prop_assert!(second.contains_symbol(symbol));
            terms.push(symbol.to_expr());
        }
        prop_assert_eq!(&second, &Expr::sum(terms));
    }

    #[test]
    fn function_headers_preserve_utf8_diagnostic_spans(
        padding in "[ \t\n]{0,12}",
        leading in "[ \t\n]{0,12}",
    ) {
        let context = Context::new();
        let name = "\u{03c6}";
        context.register_function(name, UserFunction::new(1..=1))?;
        let header = format!("{leading}{name}{padding}(");
        let opening = header.len().saturating_sub(1);
        let unfinished = format!("{header}x");
        prop_assert_eq!(context.parse(&unfinished), Err(ParseError::UnclosedParenthesis {
            span: Span::new(opening, header.len()),
        }));
        let excessive = format!("{header}x,y)");
        prop_assert_eq!(context.parse(&excessive), Err(ParseError::Function {
            span: Span::new(leading.len(), excessive.len()),
            error: FunctionError::InvalidArity { minimum: 1, maximum: 1, actual: 2 },
        }));
    }
}

#[test]
fn integer_limits_and_float_representation() {
    let context = Context::new();
    assert_eq!(
        context.parse("9223372036854775807"),
        Ok(Expr::number(i64::MAX))
    );
    assert_eq!(
        context.parse("-9223372036854775808"),
        Ok(Expr::number(i64::MIN))
    );
    assert_eq!(
        context.parse("-(9223372036854775808)"),
        Ok(Expr::number(i64::MIN))
    );
    assert_eq!(context.parse("2.0"), Ok(Expr::number(2.0)));
    assert_ne!(context.parse("2.0"), Ok(Expr::number(2)));
    assert_eq!(context.parse("-0.0"), Ok(Expr::number(-0.0)));
    assert_eq!(context.parse("1_024"), Ok(Expr::number(1024)));
    assert_eq!(context.parse(".25"), Ok(Expr::number(0.25)));
    for source in [
        "9223372036854775808",
        "-9223372036854775809",
        "1e9999",
        "1__2",
        "1_",
        "1e+",
    ] {
        assert!(
            matches!(context.parse(source), Err(ParseError::InvalidNumber { .. })),
            "source: {source}"
        );
    }
}

#[test]
fn malformed_input_is_rejected_without_repair() {
    let context = Context::new();
    for source in [
        "", "   ", "()", "1 2", "2.3.4", "x +", "x ** / y", "sin(,x)", "sin(x,)", "(x, y)", "x)",
        "(x", "sin(x", "x = y",
    ] {
        assert!(context.parse(source).is_err(), "source: {source}");
    }
    assert_eq!(
        context.parse("x +"),
        Err(ParseError::ExpectedExpression {
            span: Span::new(3, 3)
        })
    );
    assert_eq!(
        context.parse("(x"),
        Err(ParseError::UnclosedParenthesis {
            span: Span::new(0, 1)
        })
    );
}

#[test]
fn calls_validate_arities_and_do_not_create_unknown_functions() {
    let context = Context::new();
    context
        .register_function("nullary", UserFunction::new(0..=0))
        .expect("nullary registration");
    context
        .register_function("pair", UserFunction::new(2..=2))
        .expect("pair registration");
    assert!(context.parse("nullary()").is_ok());
    assert!(context.parse("nullary(+)").is_err());
    assert!(context.parse("nullary(++)").is_err());
    assert!(context.parse("pair(x,y)").is_ok());
    assert!(matches!(
        context.parse("pair(x)"),
        Err(ParseError::Function {
            error: FunctionError::InvalidArity { actual: 1, .. },
            ..
        })
    ));
    assert!(matches!(
        context.parse("sin()"),
        Err(ParseError::Function { .. })
    ));
    assert!(matches!(
        context.parse("sin(x,y)"),
        Err(ParseError::Function { .. })
    ));
    let unknown = context.parse("unknown(x+1)");
    let f = context.symb("unknown").expect("parsed symbol");
    let x = context.symb("x").expect("parsed symbol");
    assert_eq!(unknown, Ok(f * (x + 1)));
    assert!(context.get_function("unknown").is_none());
}

#[test]
fn constants_aliases_and_unicode_operators() {
    let context = Context::new();
    assert_eq!(context.parse("pi + \u{03c0}"), Ok(2 * Expr::pi()));
    assert_eq!(context.parse("e + i"), Ok(Expr::e() + Expr::i()));
    assert_eq!(context.parse("2e"), Ok(2 * Expr::e()));
    assert_eq!(
        context.parse("2 \u{00d7} x \u{2212} x \u{00f7} 2"),
        context.parse("2*x-x/2")
    );
    assert_eq!(
        context.parse("\u{03b1} \u{00b7} \u{03b2}"),
        Ok(context.symb("\u{03b1}").expect("parsed alpha")
            * context.symb("\u{03b2}").expect("parsed beta"))
    );
    let local_pi = context.symb("pi").expect("constant shadow");
    assert_eq!(context.parse("pi"), Ok(local_pi.to_expr()));
}

#[test]
fn deep_grouping_and_right_associative_powers_use_bounded_stack() {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let context = Context::new();
            let nested = format!("{}x{}", "(".repeat(10_000), ")".repeat(10_000));
            assert_eq!(
                context.parse(&nested).expect("balanced groups"),
                context.symb("x").expect("symbol").to_expr()
            );
            let powers = format!("{}x", "x^".repeat(2_000));
            let expression = context.parse(&powers).expect("nested powers");
            assert!(expression.contains_var("x"), "the base remains present");
            let unfinished = format!("{}x+", "(".repeat(10_000));
            assert!(
                context.parse(&unfinished).is_err(),
                "incomplete groups fail without recursion"
            );
        })
        .expect("small-stack parser worker")
        .join()
        .expect("iterative parsing and destruction");
}
