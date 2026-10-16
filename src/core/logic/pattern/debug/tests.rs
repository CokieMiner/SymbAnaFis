//! Stack safety for bounded pattern debug output.

use std::thread;

use alloc::format;

use super::super::{Pattern, RewriteRule};

#[test]
fn deeply_nested_patterns_debug_and_drop_on_a_small_stack() {
    thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let capture = Pattern::capture("debug_deep_argument");
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
