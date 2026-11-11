//! Structural expression traversal, stored edge iteration, and topological numbering.

use super::{
    ExactIdentity, Expansion, Expr, Forest, FunctionDefinition, Mapped, Node, Numbering, Postorder,
    Span, StoredForm, Visit,
};

mod edges;
mod postorder;
