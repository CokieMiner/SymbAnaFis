//! Evaluator Implementation Details (Core Engines)

pub mod compile;
pub(super) mod execute;
pub mod instruction;

// Re-exports for api.rs / Evaluator API Boundary
pub use compile::Compiler;
pub use compile::expand::expand_user_functions;

#[cfg(feature = "parallel")]
pub use execute::batch::eval_single_expr_chunked;

#[cfg(feature = "parallel")]
pub use execute::parallel::{
    EvalResult, ExprInput, SKIP, Value, VarInput, evaluate_parallel, evaluate_parallel_with_hint,
};

pub use execute::tree::VarLookup;
