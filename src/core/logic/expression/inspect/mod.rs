//! Expression inspection, formatting, borrowed views, and intrinsic methods.

use super::{
    BuiltinFunction, ExactIdentity, Expr, ExprView, Function, FunctionDefinition, Node,
    NumericValue, Polynomial, Postorder, Symbol, SymbolData, Visit,
};

mod analysis;
mod display;
mod functions;
mod view;

pub use view::ExprChildren;
