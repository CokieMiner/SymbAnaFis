//! Sub-module for bulk execution drivers (batch processing, multi-threading).

#[cfg(feature = "parallel")]
pub mod batch;

#[cfg(feature = "parallel")]
pub mod parallel;

#[cfg(feature = "parallel")]
pub use batch::eval_single_expr_chunked;

#[cfg(feature = "parallel")]
pub use parallel::{
    EvalResult, ExprInput, SKIP, Value, VarInput, evaluate_parallel, evaluate_parallel_with_hint,
};

#[cfg(feature = "parallel")]
pub use super::{CompiledEvaluator, ToParamName};

#[cfg(all(test, feature = "parallel"))]
mod tests;
