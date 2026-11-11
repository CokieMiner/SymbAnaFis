//! Node-head filtering, ordered matching, commutative group selection, and
//! backtracking search.

use super::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, Expr, ExprView, Function, MatchWork,
    OperandDemand, OperandMatch, Pattern, PatternNode, RewriteError, Template,
};

mod group;
mod heads;
mod ordered;
mod search;
mod selection;

pub use group::{Assignment, Group, GroupValues, Selected, Task};
pub use heads::{CountedSlot, Heads, NodeShape, PowerTemplate};
pub use ordered::{Expansion, MatchChildren};
pub use search::Search;
pub use selection::Taken;
