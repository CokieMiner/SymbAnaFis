//! Discrete execution and composition contracts.

use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::{sync::Arc, vec};

use crate::{
    Context, Expr, Pattern, RewriteRule, SimplificationSequence, SimplificationStep,
    SimplificationTarget, SimplificationTermination, SimplifyError, UserFunction,
};

use super::simplify;

#[test]
fn empty_zero_iteration_and_default_sequences_are_explicit() {
    let expression = Expr::symbol("empty_sequence_x").sin();
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
    let x = Expr::symbol("ordered_sequence_x");
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
    let x = Expr::symbol("cycle_sequence_x");
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
    let x = Expr::symbol("intermediate_sequence_x");
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
    let context = Context::new();
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
    let x = Expr::symbol("limit_sequence_x");
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
    let x = Expr::symbol("reverse_preset_x");
    let input = x.sin().pow(2) + x.cos().pow(2);
    assert_eq!(simplify(&custom, &input), Expr::number(1));
}

#[test]
fn bounded_rewrites_propagate_match_errors_through_the_sequence() {
    let x = Expr::symbol("bounded_match_x");
    let y = Expr::symbol("bounded_match_y");
    let a = Pattern::capture("a", None);
    let b = Pattern::capture("b", None);
    let rewrite = RewriteRule::new(a.sin() + b.cos(), Pattern::exact(0)).expect("bound captures");
    let input = x.sin() + y.cos() + x.exp();
    let result = SimplificationSequence::new()
        .then(rewrite)
        .match_limit(Some(0))
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
            let mut expression = Expr::symbol("deep_simplification_x");
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
