//! Evaluator Implementation Details (Core Engines)

pub mod bytecode;
pub mod tree;

// Re-exports for api.rs / Evaluator API Boundary
pub use bytecode::compile::Compiler;
pub use bytecode::compile::codegen::expand::expand_user_functions;

#[cfg(feature = "parallel")]
pub use bytecode::execute::drivers::batch::eval_single_expr_chunked;

#[cfg(feature = "parallel")]
pub use bytecode::execute::drivers::parallel::{
    EvalResult, ExprInput, SKIP, Value, VarInput, evaluate_parallel, evaluate_parallel_with_hint,
};

pub use tree::eval::VarLookup;
