pub mod compile;
pub mod execute;
pub mod instruction;

pub use compile::{Compiler, expand_user_functions};
#[cfg(feature = "parallel")]
pub use execute::{
    EvalResult, ExprInput, SKIP, Value, VarInput, eval_single_expr_chunked, evaluate_parallel,
    evaluate_parallel_with_hint,
};
pub use instruction::Instruction;

#[cfg(feature = "parallel")]
pub use super::ToParamName;

pub use super::CompiledEvaluator;

#[cfg(test)]
mod execute_tests;
