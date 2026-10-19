//! Resource validation at candidate boundaries.

use crate::{core::Expr, errors::SimplifyError};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    iterations: usize,
    nodes: usize,
    depth: usize,
    matches: Option<usize>,
    expansion: usize,
    polynomial: usize,
}

impl Limits {
    pub const DEFAULT: Self = Self {
        iterations: 16,
        nodes: usize::MAX,
        depth: usize::MAX,
        matches: Some(10_000),
        expansion: usize::MAX,
        polynomial: usize::MAX,
    };

    pub const fn with_iterations(mut self, value: usize) -> Self {
        self.iterations = value;
        self
    }
    pub const fn with_nodes(mut self, value: usize) -> Self {
        self.nodes = value;
        self
    }
    pub const fn with_depth(mut self, value: usize) -> Self {
        self.depth = value;
        self
    }
    pub const fn with_matches(mut self, value: Option<usize>) -> Self {
        self.matches = value;
        self
    }
    pub const fn with_expansion(mut self, value: usize) -> Self {
        self.expansion = value;
        self
    }

    pub const fn with_polynomial(mut self, value: usize) -> Self {
        self.polynomial = value;
        self
    }

    pub const fn iterations(self) -> usize {
        self.iterations
    }
    pub const fn matches(self) -> Option<usize> {
        self.matches
    }
    pub const fn expansion(self) -> usize {
        self.expansion
    }

    pub const fn polynomial(self) -> usize {
        self.polynomial
    }

    pub fn validate(self, expression: &Expr) -> Result<(), SimplifyError> {
        if expression.node_count() > self.nodes {
            return Err(SimplifyError::NodeLimitExceeded { limit: self.nodes });
        }
        if expression.max_depth() > self.depth {
            return Err(SimplifyError::DepthLimitExceeded { limit: self.depth });
        }
        Ok(())
    }
}
