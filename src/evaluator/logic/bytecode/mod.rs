pub mod compile;
pub mod execute;
pub mod instruction;

pub use compile::{VirGenerator, expand_user_functions};
#[cfg(all(feature = "parallel", feature = "python"))]
pub use execute::evaluate_parallel_with_hint;
#[cfg(feature = "parallel")]
pub use execute::{
    EvalResult, ExprInput, SKIP, Value, VarInput, eval_single_expr_chunked, evaluate_parallel,
};
pub use instruction::Instruction;

#[cfg(feature = "parallel")]
pub use super::ToParamName;

pub use super::CompiledEvaluator;

#[cfg(test)]
mod execute_tests;
