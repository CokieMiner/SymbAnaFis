//! Template numbering, stack-safe formatting, function builders, and rule validation.

use core::ptr::from_ref;

use std::thread;

use alloc::{format, vec, vec::Vec};

use proptest::{collection, prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::{FixedSet, HashMap},
    core::{BuiltinFunction, Context, Expr, Function, UserFunction},
    errors::RewriteError,
};

use super::super::{
    Pattern, PatternNode, Replacement, ReplacementProgram, RewriteRule, TemplateVisit,
};

/// The children of a template node in numbering order.
fn children(pattern: &Pattern) -> Vec<&Pattern> {
    match pattern.node() {
        PatternNode::Capture { .. } | PatternNode::Exact(_) => Vec::new(),
        PatternNode::Function { args, .. }
        | PatternNode::Sum(args)
        | PatternNode::Product(args) => args.iter().collect(),
        PatternNode::Div(left, right) | PatternNode::Pow(left, right) => vec![left, right],
    }
}

proptest! {
    #[test]
    fn template_numbering_links_earlier_children_in_template_order(
        steps in collection::vec(0_u8..5, 1..=10),
    ) {
        let context = Context::new()?;
        let pair = context.register_function("pair", UserFunction::new(2..=2))?;
        let a = Pattern::capture("a", None);
        let b = Pattern::capture("b", None);
        let mut pattern = a.clone();
        for step in steps {
            pattern = match step {
                0 => Pattern::apply(BuiltinFunction::Sin.function(), vec![pattern])?,
                1 => Pattern::sum(vec![pattern, b.clone()]),
                2 => pattern.pow(2),
                3 => Pattern::apply(pair.clone(), vec![pattern.clone(), pattern])?,
                _ => Pattern::apply(pair.clone(), vec![pattern, a.clone()])?,
            };
        }
        let order = PatternNode::template_order(&pattern);
        let visits: Vec<&TemplateVisit<'_>> = order.descendants.iter().chain([&order.root]).collect();
        let distinct: FixedSet<_> = visits.iter().map(|visit| from_ref(visit.pattern.node())).collect();
        prop_assert_eq!(distinct.len(), visits.len());
        prop_assert_eq!(from_ref(order.root.pattern.node()), from_ref(pattern.node()));
        for (position, visit) in visits.iter().enumerate() {
            let expected = children(visit.pattern);
            prop_assert_eq!(visit.children.len(), expected.len());
            for (&child, template) in visit.children.iter().zip(expected) {
                prop_assert!(child < position);
                let linked = order.descendants.get(child).map(|linked| from_ref(linked.pattern.node()));
                prop_assert_eq!(linked, Some(from_ref(template.node())));
            }
        }
    }

    #[test]
    fn literal_function_arguments_fold_inside_captured_calls(
        value in 0_i32..=16, operation in 0_u8..=5,
    ) {
        let context = Context::new().expect("context");
        let host = context.register_function("host", UserFunction::new(2..=2)).expect("unbound name");
        let x = context.symb("x")?;
        let a = Pattern::capture("a", None);
        let squared = value.saturating_mul(value);
        let (literal, expected) = match operation {
            0 => (Pattern::exact(squared).sqrt(), Expr::number(value)),
            1 => (Pattern::exact(0).sin(), Expr::number(0)),
            2 => (Pattern::exact(0).cos(), Expr::number(1)),
            3 => (Pattern::exact(0).exp(), Expr::number(1)),
            4 => (Pattern::exact(value).floor(), Expr::number(value)),
            _ => (Pattern::exact(0).lambertw(0), Expr::number(0)),
        };
        let pattern = Pattern::apply(host.clone(), vec![a.clone(), literal]).expect("binary host");
        let rule = RewriteRule::new(pattern, a).expect("bound capture");
        let input = host.call(vec![x.into(), expected]).expect("binary host");
        prop_assert_eq!(rule.apply_all(&input, usize::MAX, None), Ok(x.to_expr()));
    }

    #[test]
    fn builtin_templates_bind_the_documented_argument_positions(
        family in 0_u8..=4, first in -3_i32..=3, second in -3_i32..=3,
    ) {
        let context = Context::new().expect("context");
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let a = Pattern::capture("a", None);
        let (pattern, name, arguments) = match family {
            0 => (a.log(first), "log", vec![Expr::number(first), x.into()]),
            1 => (a.besselj(first), "besselj", vec![Expr::number(first), x.into()]),
            2 => (a.beta(first), "beta", vec![x.into(), Expr::number(first)]),
            3 => (a.atan2(first), "atan2", vec![x.into(), Expr::number(first)]),
            _ => (a.assoc_legendre(first, second), "assoc_legendre", vec![Expr::number(first), Expr::number(second), x.into()]),
        };
        let input = Function::builtin(name).expect("known builtin").call(arguments).expect("accepted arity");
        let rule = RewriteRule::new(pattern, Pattern::exact(u)).expect("literal replacement");
        prop_assert_eq!(rule.apply_all(&input, usize::MAX, None), Ok(u.to_expr()));
    }

    #[test]
    fn literal_and_fluent_calls_have_the_same_numeric_policy(value in -8_i32..=8) {
        let context = Context::new().expect("context");
        let host = context.register_function("host", UserFunction::new(2..=2)).expect("unbound name");
        let x = context.symb("x")?;
        let a = Pattern::capture("a", None);
        let argument = Expr::number(f64::from(value));
        let sine = Function::builtin("sin").expect("known builtin");
        let literal = Pattern::apply(sine, vec![Pattern::exact(&argument)]).expect("unary builtin");
        let nested = Pattern::apply(host.clone(), vec![a.clone(), literal]).expect("binary host");
        let rule = RewriteRule::new(nested, a).expect("bound capture");
        prop_assert_eq!(rule.apply_all(&host.call(vec![x.into(), argument.sin()]).expect("binary host"), usize::MAX, None), Ok(x.to_expr()));
    }
}

#[test]
fn instantiation_reports_an_unbound_capture() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol").to_expr();
    let template = Pattern::capture("a", None).pow(2);
    let mut captures = FixedSet::default();
    captures.insert("a");
    let program =
        ReplacementProgram::compile(Replacement::Pattern(template), &captures).expect("bound");
    assert_eq!(program.instantiate(&HashMap::new()), None);
    let mut bound = HashMap::new();
    bound.insert("a", x.clone());
    assert_eq!(program.instantiate(&bound), Some(x.pow(2)));
}

#[test]
fn deeply_nested_patterns_debug_and_drop_on_a_small_stack() {
    thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let capture = Pattern::capture("debug_deep_argument", None);
            let mut pattern = capture.clone();
            for _level in 0..20_000 {
                pattern = pattern.sin();
            }
            let pattern_debug = format!("{pattern:?}");
            assert!(
                pattern_debug.contains("Pattern"),
                "the pattern summary names its type"
            );
            assert!(
                pattern_debug.contains("Function"),
                "the summary identifies its root function"
            );
            assert!(
                pattern_debug.contains("arity: 1"),
                "the root function has one argument"
            );

            let rule = RewriteRule::new(pattern, capture).expect("capture is bound");
            let rule_debug = format!("{rule:?}");
            assert!(
                rule_debug.contains("RewriteRule"),
                "the rule summary names its type"
            );
            assert!(
                rule_debug.contains("compiled_operations"),
                "the summary reports replacement operations"
            );
        })
        .expect("start small-stack pattern test")
        .join()
        .expect("deep pattern debug and drop remain iterative");
}

#[test]
fn required_functions_are_conservative_about_virtual_powers_and_exact_templates() {
    let context = Context::new().expect("context");
    let host = context
        .register_function("host", UserFunction::new(1..=1))
        .expect("unbound name");
    let x = context.symb("x").expect("available symbol");
    let a = Pattern::capture("a", None);
    let source = Pattern::sum(vec![
        a.sqrt(),
        a.sin(),
        a.cos(),
        Pattern::apply(host.clone(), vec![a.clone()]).expect("unary host"),
        Pattern::exact(x.exp()),
    ]);
    let rule = RewriteRule::new(source, a).expect("bound capture");
    let required = rule.required_functions().collect::<Vec<_>>();
    assert_eq!(required.len(), 3);
    assert!(required.contains(&host));
    assert!(required.contains(&Function::builtin("sin").expect("known builtin")));
    assert!(required.contains(&Function::builtin("cos").expect("known builtin")));
}

#[test]
fn replacement_validation_rejects_unbound_captures() {
    assert!(matches!(
        RewriteRule::new(
            Pattern::capture("bound", None),
            Pattern::capture("absent", None)
        ),
        Err(RewriteError::UnboundCapture(_))
    ));
}

#[test]
fn replacement_is_not_rewritten_again_and_false_guards_preserve_input() {
    let capture = Pattern::capture("edge_argument", None);
    let expanding =
        RewriteRule::new(capture.clone(), capture.sin()).expect("the replacement capture is bound");
    let x = Expr::symbol("pattern_edge_x").expect("symbol");
    assert_eq!(expanding.apply_all(&x, usize::MAX, None), Ok(x.sin()));

    let guarded =
        RewriteRule::new(capture.sin(), capture.cos()).expect("the guard capture is bound");
    let context = Context::new().expect("context");
    let pair = context
        .register_function("pair", UserFunction::new(2..=2))
        .expect("the custom name is unbound");
    let input = pair
        .call(vec![x.sin(), x.cos()])
        .expect("pair accepts two arguments");
    assert_eq!(
        guarded.apply_all(&input, usize::MAX, Some(&mut |_| false)),
        Ok(input)
    );
}
