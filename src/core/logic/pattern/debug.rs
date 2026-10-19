//! Bounded debug summaries for potentially deep pattern templates.

use core::fmt::{Debug, Formatter, Result as FmtResult};

use super::{Pattern, PatternNode, Replacement, ReplacementProgram, RewriteRule};

struct RootSummary<'pattern>(&'pattern PatternNode);

impl Debug for Pattern {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("Pattern")
            .field("root", &RootSummary(self.node()))
            .finish()
    }
}

impl Debug for RewriteRule {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        let (operations, callback, concrete) = match &self.replacement {
            ReplacementProgram::Template(instructions) => (instructions.len(), false, false),
            ReplacementProgram::Callback(_) => (0, true, false),
            ReplacementProgram::Concrete(_) => (0, false, true),
            ReplacementProgram::ConcreteCallback { .. } => (0, true, true),
        };
        formatter
            .debug_struct("RewriteRule")
            .field("concrete", &concrete)
            .field("pattern", &RootSummary(self.pattern.node()))
            .field("compiled_operations", &operations)
            .field("callback", &callback)
            .field("associative", &self.associative)
            .finish()
    }
}

impl Debug for Replacement {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Pattern(pattern) => formatter
                .debug_tuple("Pattern")
                .field(&RootSummary(pattern.node()))
                .finish(),
            Self::Callback(_) => formatter.write_str("Callback(..)"),
        }
    }
}

impl Debug for RootSummary<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self.0 {
            PatternNode::Capture { name, predicates } => formatter
                .debug_struct("Capture")
                .field("name", name)
                .field("predicates", &predicates.iter().count())
                .finish(),
            PatternNode::Exact(_) => formatter.write_str("Exact(..)"),
            PatternNode::Function { name, args } => formatter
                .debug_struct("Function")
                .field("name", &name.as_str())
                .field("arity", &args.len())
                .finish(),
            PatternNode::Sum(args) => formatter
                .debug_struct("Sum")
                .field("arity", &args.len())
                .finish(),
            PatternNode::Product(args) => formatter
                .debug_struct("Product")
                .field("arity", &args.len())
                .finish(),
            PatternNode::Div(..) => formatter.write_str("Div"),
            PatternNode::Pow(..) => formatter.write_str("Pow"),
        }
    }
}
