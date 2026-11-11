//! Rule validation, per-node matching stages, and multi-rule passes.

use super::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, BuiltinSet, ConcreteMatcher, CountedSlot,
    ExactSources, Expr, ExprView, Function, Group, GroupValues, Heads, Mapped, MatchWork,
    NodeShape, Normalization, Pattern, PatternNode, Postorder, Replacement, ReplacementFn,
    ReplacementOp, ReplacementProgram, RewriteError, RewriteGuard, RewriteRule, Search, Selected,
    Substitution, Taken, Task, Template, TemplateOrder,
};

mod rule;
mod rule_set;
mod stages;

pub use rule_set::{RewriteTarget, RulePass, RuleSet};
