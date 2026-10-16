//! Bounded debug summaries for potentially deep pattern templates.

use core::fmt::{Debug, Formatter, Result as FmtResult};

use super::{Pattern, PatternNode, RewriteRule};

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
        formatter
            .debug_struct("RewriteRule")
            .field("concrete", &self.concrete.is_some())
            .field("pattern", &RootSummary(self.pattern.node()))
            .field("compiled_operations", &self.replacement.len())
            .field("associative", &self.associative)
            .finish()
    }
}

impl Debug for RootSummary<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self.0 {
            PatternNode::Capture(name) => formatter.debug_tuple("Capture").field(name).finish(),
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

#[cfg(test)]
mod tests;
