//! Formatter callbacks receive unlocked context snapshots.

use core::fmt::{Result as FmtResult, Write};

use alloc::{format, string::String};

use proptest::{prop_assert, proptest};

use super::super::{Context, UserFunction};

struct Writer<'context> {
    context: &'context Context,
    unlocked: bool,
    output: String,
}

impl Write for Writer<'_> {
    fn write_str(&mut self, text: &str) -> FmtResult {
        self.unlocked &= self.context.bindings.try_lock().is_some();
        self.output.push_str(text);
        Ok(())
    }
}

proptest! {
    #[test]
    fn debug_writers_run_after_unlocking_the_registry(count in 0_usize..=8) {
        let context = Context::new();
        for index in 0..count {
            let _symbol = context.symb(&format!("s{index}"))?;
            context.register_function(&format!("f{index}"), UserFunction::default())
                .expect("the generated names are distinct");
        }
        let mut writer = Writer { context: &context, unlocked: true, output: String::new() };
        #[expect(
            clippy::use_debug,
            reason = "the production formatting restriction is a false positive for a regression test of Debug writer reentrancy"
        )]
        write!(writer, "{context:?}").expect("the writer accepts all output");
        prop_assert!(writer.unlocked);
        prop_assert!(writer.output.contains("Context"));
        for index in 0..count {
            prop_assert!(writer.output.contains(&format!("s{index}")), "the snapshot contains each symbol name");
            prop_assert!(writer.output.contains(&format!("f{index}")), "the snapshot contains each function name");
        }
    }
}
