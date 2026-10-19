//! Small deterministic cases for validation and rewrite boundaries.

use alloc::vec;

use crate::core::{Context, UserFunction};

use super::super::{Expr, Pattern, RewriteError, RewriteRule};

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
    let x = Expr::symbol("pattern_edge_x");
    assert_eq!(expanding.apply_all(&x, None, None), Ok(x.sin()));

    let guarded =
        RewriteRule::new(capture.sin(), capture.cos()).expect("the guard capture is bound");
    let context = Context::new();
    let pair = context
        .register_function("pair", UserFunction::new(2..=2))
        .expect("the custom name is unbound");
    let input = pair
        .call(vec![x.sin(), x.cos()])
        .expect("pair accepts two arguments");
    assert_eq!(
        guarded.apply_all(&input, None, Some(&mut |_| false)),
        Ok(input)
    );
}
