//! Discrete execution and composition contracts.

use core::{
    iter::once,
    ops::{Add, Div, Mul, Sub},
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{format, sync::Arc, vec, vec::Vec};

use proptest::{collection, prop_assert_eq, prop_oneof, proptest};

use crate::{
    Context, DomainSafety, Expr, Pattern, RewriteRule, SimplificationSequence, SimplificationStep,
    SimplificationTarget, SimplificationTermination, SimplifyError, Substitutions, Symbol,
    UserFunction,
};

use super::{Limits, PreparedPlan, PreparedStep, simplify};

/// Every builtin step, in preset order.
const BUILTIN_STEPS: [SimplificationStep; 8] = [
    SimplificationStep::Algebraic,
    SimplificationStep::Roots,
    SimplificationStep::Exponential,
    SimplificationStep::Trigonometric,
    SimplificationStep::Hyperbolic,
    SimplificationStep::Expand,
    SimplificationStep::Factor,
    SimplificationStep::Compilation,
];

proptest! {
    /// Builtin steps number compact storage in symbols and constants as a
    /// leaf, which holds because each of its terms, a literal times positive
    /// powers of distinct scalar leaves with \(e\) to the first power, is a
    /// fixed point of every builtin step under both domain contracts.
    #[test]
    fn builtin_steps_keep_every_term_of_compact_storage_in_scalar_leaves(
        coefficient in prop_oneof![-9_i64..=-1_i64, 1_i64..=9_i64],
        exponents in collection::vec(0_i64..=4_i64, 4),
    ) {
        let bases = [
            Expr::symbol("leaf_term_x")?,
            Expr::symbol("leaf_term_y")?,
            Expr::pi(),
            Expr::e(),
        ];
        let powers = bases
            .iter()
            .zip(&exponents)
            .filter(|(_, exponent)| **exponent > 0)
            .map(|(base, exponent)| if *base == Expr::e() { base.clone() } else { base.pow(*exponent) });
        let term = Expr::product(once(Expr::number(coefficient)).chain(powers).collect::<Vec<_>>());
        for step in &BUILTIN_STEPS {
            for safety in [DomainSafety::Preserving, DomainSafety::AssumptionDependent] {
                let mut plan = PreparedPlan::new(step);
                if safety == DomainSafety::AssumptionDependent {
                    plan.enable_assumptions(step);
                }
                prop_assert_eq!(
                    PreparedStep::new(step, &plan, safety).apply(&term, Limits::DEFAULT)?,
                    term.clone(),
                    "{:?} under {:?} keeps the term",
                    step,
                    safety
                );
            }
        }
    }
}

#[test]
fn empty_zero_iteration_and_default_sequences_are_explicit() {
    let expression = Symbol::intern("empty_sequence_x").expect("symbol").sin();
    let result = SimplificationSequence::new()
        .simplify(&expression)
        .expect("empty program");
    assert_eq!(result.expression(), &expression);
    assert_eq!(result.iterations(), 0);
    assert_eq!(result.termination(), SimplificationTermination::FixedPoint);
    let stopped = SimplificationSequence::compact()
        .max_iterations(0)
        .simplify(&expression)
        .expect("zero sweeps");
    assert_eq!(stopped.expression(), &expression);
    assert_eq!(stopped.iterations(), 0);
    assert_eq!(
        stopped.termination(),
        SimplificationTermination::IterationLimit
    );
    assert_eq!(
        SimplificationSequence::default().steps().len(),
        SimplificationSequence::compact().steps().len()
    );
}

#[test]
fn custom_rules_run_in_order_and_the_program_is_reusable() {
    let x = Symbol::intern("ordered_sequence_x")
        .expect("symbol")
        .to_expr();
    let first =
        RewriteRule::new(Pattern::exact(x.sin()), Pattern::exact(x.cos())).expect("exact rule");
    let second =
        RewriteRule::new(Pattern::exact(x.cos()), Pattern::exact(x.tan())).expect("exact rule");
    let sequence = SimplificationSequence::new().then(first).then(second);
    assert_eq!(simplify(&sequence, &x.sin()), x.tan());
    assert_eq!(simplify(&sequence, &x.exp()), x.exp());
    assert_eq!(sequence.steps().len(), 2);
    let cloned = sequence.clone();
    drop(sequence);
    assert_eq!(simplify(&cloned, &x.sin()), x.tan());
}

#[test]
fn cycles_return_the_best_intermediate_even_when_a_sweep_returns_to_its_input() {
    let x = Symbol::intern("cycle_sequence_x")
        .expect("symbol")
        .to_expr();
    let cosine = x.cos();
    let preferred = cosine.clone();
    let sequence = SimplificationSequence::new()
        .then(
            RewriteRule::new(Pattern::exact(x.sin()), Pattern::exact(&cosine)).expect("exact rule"),
        )
        .then(
            RewriteRule::new(Pattern::exact(&cosine), Pattern::exact(x.sin())).expect("exact rule"),
        )
        .score_with(move |node| usize::from(node != &preferred));
    let result = sequence
        .simplify(&x.sin())
        .expect("cycle is a successful bounded result");
    assert_eq!(result.expression(), &cosine);
    assert_eq!(result.iterations(), 1);
    assert_eq!(result.termination(), SimplificationTermination::Cycle);
}

#[test]
fn target_selection_keeps_temporary_growth_available_to_later_steps() {
    let x = Symbol::intern("intermediate_sequence_x")
        .expect("symbol")
        .to_expr();
    let large = x.sin() + x.cos();
    let program = SimplificationSequence::new()
        .then(RewriteRule::new(Pattern::exact(&x), Pattern::exact(&large)).expect("exact rule"))
        .then(RewriteRule::new(Pattern::exact(&large), Pattern::exact(0)).expect("exact rule"))
        .max_iterations(1);
    assert_eq!(simplify(&program, &x), Expr::number(0));
    let grow = SimplificationSequence::new()
        .then(RewriteRule::new(Pattern::exact(&x), Pattern::exact(&large)).expect("exact rule"))
        .max_iterations(1);
    assert_eq!(simplify(&grow, &x), x);
    let preferred = large.clone();
    let custom = grow.score_with(move |node| usize::from(node != &preferred));
    assert_eq!(simplify(&custom, &x), large);
    assert_eq!(
        simplify(&custom.target(SimplificationTarget::Compact), &x),
        x
    );
}

#[test]
fn custom_maps_visit_shared_inputs_once_and_do_not_revisit_inserted_nodes() {
    let context = Context::new().expect("context");
    let pair = context
        .register_function("sequence_pair", UserFunction::new(2..=2))
        .expect("unique function");
    let x = context.symb("x").expect("symbol").to_expr();
    let input = pair.call(vec![x.clone(), x.clone()]).expect("binary call");
    let original = x.clone();
    let count = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&count);
    let mapped = SimplificationStep::Map(Arc::new(move |node| {
        observed.fetch_add(1, Ordering::Relaxed);
        (node == &original).then(|| original.sin())
    }));
    let expected = pair.call(vec![x.sin(), x.sin()]).expect("binary call");
    let preferred = expected.clone();
    let program = SimplificationSequence::new()
        .then(mapped)
        .max_iterations(1)
        .score_with(move |node| usize::from(node != &preferred));
    assert_eq!(simplify(&program, &input), expected);
    assert_eq!(count.load(Ordering::Relaxed), 2);
}

#[test]
fn candidate_limits_reject_temporary_growth_without_publishing_partial_output() {
    let x = Symbol::intern("limit_sequence_x")
        .expect("symbol")
        .to_expr();
    let input = x.sin();
    assert!(matches!(
        SimplificationSequence::new().max_nodes(1).simplify(&input),
        Err(SimplifyError::NodeLimitExceeded { limit: 1 })
    ));
    assert!(matches!(
        SimplificationSequence::new().max_depth(1).simplify(&input),
        Err(SimplifyError::DepthLimitExceeded { limit: 1 })
    ));
    let bigger = x.sin().cos().exp();
    let program = SimplificationSequence::new().max_nodes(2).then(
        RewriteRule::new(Pattern::exact(&input), Pattern::exact(bigger)).expect("exact rule"),
    );
    assert!(matches!(
        program.simplify(&input),
        Err(SimplifyError::NodeLimitExceeded { limit: 2 })
    ));
    assert_eq!(input, x.sin());
    let expansion = (x + 1).pow(4);
    assert!(matches!(
        SimplificationSequence::expanded()
            .expansion_limit(0)
            .simplify(&expansion),
        Err(SimplifyError::ExpansionLimitExceeded { limit: 0 })
    ));
}

#[test]
fn a_preset_can_be_reordered_by_cloning_its_existing_steps() {
    let preset = SimplificationSequence::compact();
    let custom = preset
        .steps()
        .iter()
        .rev()
        .cloned()
        .fold(SimplificationSequence::new(), SimplificationSequence::then);
    let x = Symbol::intern("reverse_preset_x")
        .expect("symbol")
        .to_expr();
    let input = x.sin().pow(2) + x.cos().pow(2);
    assert_eq!(simplify(&custom, &input), Expr::number(1));
}

#[test]
fn bounded_rewrites_propagate_match_errors_through_the_sequence() {
    let x = Symbol::intern("bounded_match_x").expect("symbol").to_expr();
    let y = Symbol::intern("bounded_match_y").expect("symbol").to_expr();
    let a = Pattern::capture("a", None);
    let b = Pattern::capture("b", None);
    let rewrite = RewriteRule::new(a.sin() + b.cos(), Pattern::exact(0)).expect("bound captures");
    let input = x.sin() + y.cos() + x.exp();
    let result = SimplificationSequence::new()
        .then(rewrite)
        .match_limit(0)
        .simplify(&input);
    assert!(matches!(
        result,
        Err(SimplifyError::Rewrite(
            crate::RewriteError::MatchLimitExceeded { limit: 0 }
        ))
    ));
}

#[test]
fn mapping_and_scoring_remain_iterative_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(96 * 1024)
        .spawn(|| {
            let mut expression = Symbol::intern("deep_simplification_x")
                .expect("symbol")
                .to_expr();
            for _ in 0..1024 {
                expression = expression.sin();
            }
            let program = SimplificationSequence::new()
                .max_depth(2048)
                .then(SimplificationStep::Map(Arc::new(|_| None)));
            assert_eq!(simplify(&program, &expression), expression);
        })
        .expect("small-stack thread starts")
        .join()
        .expect("mapping and scoring use explicit DAG worklists");
}

#[test]
fn settled_subexpressions_simplify_as_fresh_ones() {
    let context = Context::new().expect("context");
    let presets = [
        SimplificationSequence::compact(),
        SimplificationSequence::expanded(),
        SimplificationSequence::factored(),
        SimplificationSequence::compilation(),
    ];
    let parts = |[x, y, z]: &[Expr; 3]| {
        vec![
            Add::add(x.sin().pow(2), x.cos().pow(2)),
            Div::div(Sub::sub(x.pow(2), 1), Sub::sub(x, 1)),
            Add::add(Mul::mul(x, y).ln(), Add::add(z.ln().exp(), x.pow(2).sqrt())),
            Add::add(Add::add(x, y).pow(3), Add::add(y.sinh(), y.cosh())),
            Add::add(
                Add::add(Mul::mul(x.pow(3), 8), Mul::mul(x.pow(2), 12)),
                Add::add(Mul::mul(x, 6), 1),
            ),
            Mul::mul(z.abs(), z.abs()).sqrt(),
        ]
    };
    for (index, preset) in presets.into_iter().enumerate() {
        for safety in [DomainSafety::Preserving, DomainSafety::AssumptionDependent] {
            let sequence = preset.clone().domain_safety(safety);
            // Two symbol families give the same expressions over disjoint
            // nodes; the warm family's parts are settled first.
            let family = |name: &str| {
                ["x", "y", "z"].map(|symbol| {
                    context
                        .symb(&format!("settled_{name}{index}{safety:?}_{symbol}"))
                        .expect("symbol")
                        .to_expr()
                })
            };
            let (warm, cold) = (family("warm"), family("cold"));
            let warm_parts = parts(&warm);
            for part in &warm_parts {
                simplify(&sequence, part);
            }
            let settled = simplify(&sequence, &Expr::sum(warm_parts));
            let fresh = simplify(&sequence, &Expr::sum(parts(&cold)));
            let renaming =
                Substitutions::new(warm.into_iter().zip(cold)).expect("distinct symbols");
            assert_eq!(
                renaming.apply_all(&settled),
                fresh,
                "{index} under {safety:?}"
            );
        }
    }
}

#[test]
fn repeated_identity_matches_in_one_sum_stay_within_the_default_budget() {
    let context = Context::new().expect("context");
    let pairs = 300;
    let terms: Vec<_> = (0..pairs)
        .flat_map(|index| {
            let x = context
                .symb(&format!("budget_pair_{index}"))
                .expect("symbol");
            [x.sin().pow(2), x.cos().pow(2)]
        })
        .collect();
    // Each partner is found by identity, so the attempts grow with the
    // number of matches rather than with its square.
    assert_eq!(
        simplify(&SimplificationSequence::compact(), &Expr::sum(terms)),
        Expr::number(pairs)
    );
}

#[test]
fn every_builtin_rule_template_compiles() {
    for step in &BUILTIN_STEPS {
        for safety in [DomainSafety::Preserving, DomainSafety::AssumptionDependent] {
            assert!(
                PreparedStep::rules(step, safety).is_ok(),
                "every {step:?} destination uses only source captures"
            );
        }
    }
}
