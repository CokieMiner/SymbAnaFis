//! Structural expression traversal, stored edge iteration, and topological numbering.

use super::{
    ExactIdentity, Expansion, Expr, Facts, Forest, FunctionDefinition, Mapped, Node, Numbering,
    Postorder, SettledStage, Span, StoredForm, Visit,
};

mod edges;
mod postorder;
