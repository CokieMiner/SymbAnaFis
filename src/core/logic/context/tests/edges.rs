//! Context callback reentrancy boundary.

use core::sync::atomic::{AtomicBool, Ordering};

use alloc::sync::Arc;

use crate::core::Expr;

use super::super::{Context, UserFunction};

#[test]
fn rejected_callback_captures_drop_after_releasing_the_context_lock() {
    struct Capture(Context, Arc<AtomicBool>);

    impl Drop for Capture {
        fn drop(&mut self) {
            self.1
                .store(self.0.bindings.try_lock().is_some(), Ordering::Relaxed);
        }
    }

    let context = Context::new();
    context
        .register_function("context_reentrant_capture", UserFunction::new(0..=0))
        .expect("the first registration succeeds");
    let observed = Arc::new(AtomicBool::new(false));
    let capture = Capture(context.clone(), Arc::clone(&observed));
    context
        .register_function(
            "context_reentrant_capture",
            UserFunction::new(0..=0).body(Arc::new(move |_| {
                core::hint::black_box(&capture);
                Expr::number(0)
            })),
        )
        .expect_err("a duplicate name rejects the captured definition");
    assert!(observed.load(Ordering::Relaxed));
}
