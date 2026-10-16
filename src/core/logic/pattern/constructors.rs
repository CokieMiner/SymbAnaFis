//! Public template constructors.

use alloc::{sync::Arc, vec, vec::Vec};

use super::{
    Arithmetic, BuiltinFunction, Expr, FunctionDefinition, FunctionError, FunctionName, Pattern,
    PatternNode,
};

impl Pattern {
    /// Capture a subtree. Repeated names require the same canonical expression.
    #[must_use]
    pub fn capture(name: impl AsRef<str>) -> Self {
        Self(Some(Arc::new(PatternNode::Capture(Arc::from(
            name.as_ref(),
        )))))
    }

    /// Match one exact canonical expression.
    #[must_use]
    pub fn exact(expression: impl Into<Expr>) -> Self {
        Self(Some(Arc::new(PatternNode::Exact(expression.into()))))
    }

    /// Wrap this template in a symbolic sine.
    #[must_use]
    pub fn sin(&self) -> Self {
        Self(Some(Arc::new(PatternNode::Function {
            name: BuiltinFunction::Sin.function(),
            args: vec![self.clone()],
        })))
    }

    /// Wrap this template in a symbolic cosine.
    #[must_use]
    pub fn cos(&self) -> Self {
        Self(Some(Arc::new(PatternNode::Function {
            name: BuiltinFunction::Cos.function(),
            args: vec![self.clone()],
        })))
    }

    /// Match a known function identity with validated ordered argument templates.
    ///
    /// # Errors
    /// Returns [`FunctionError::InvalidArity`] for an unaccepted argument count.
    pub fn apply(function: FunctionName, args: Vec<Self>) -> Result<Self, FunctionError> {
        FunctionDefinition::validate(&function, args.len())?;
        Ok(Self(Some(Arc::new(PatternNode::Function {
            name: function,
            args,
        }))))
    }

    /// Normalize a sum template, retaining unmatched terms at a rule's root.
    /// Nested groups require the complete normalized operand list.
    ///
    /// # Panics
    /// Numeric folding can propagate numeric backend capacity panics.
    #[must_use]
    pub fn sum(terms: Vec<Self>) -> Self {
        PatternNode::normalize(Arithmetic::Sum(terms))
    }

    /// Normalize a product template, retaining unmatched factors at a rule's root.
    /// Nested groups require the complete normalized operand list.
    ///
    /// # Panics
    /// Numeric folding can propagate numeric backend capacity panics.
    #[must_use]
    pub fn product(factors: Vec<Self>) -> Self {
        PatternNode::normalize(Arithmetic::Product(factors))
    }

    /// Raise this template to a power through expression normalization.
    ///
    /// # Panics
    /// Numeric folding can propagate numeric backend capacity panics.
    #[must_use]
    pub fn pow(&self, exponent: impl Into<Self>) -> Self {
        PatternNode::normalize(Arithmetic::Pow(self.clone(), exponent.into()))
    }
}
