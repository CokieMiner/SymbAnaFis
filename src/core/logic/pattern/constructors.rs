//! Public template constructors.

use core::ops::Neg;

use alloc::{sync::Arc, vec, vec::Vec};

use super::{
    Arithmetic, BuiltinFunction, CaptureConditions, CapturePredicate, Expr, FunctionDefinition,
    FunctionError, FunctionName, Pattern, PatternNode,
};

impl Pattern {
    /// Capture a subtree, accepting it only when the optional predicate succeeds.
    /// Repeated names bind the same canonical expression and enforce all their
    /// predicates, including when arithmetic normalization collects occurrences.
    /// Predicates constrain matching; replacement captures read the bound value.
    /// A predicate must be pure and stable for the same expression. Search may
    /// evaluate it more than once and propagates its panics during application.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use symb_anafis::{Expr, Pattern, Property, RewriteRule, Truth};
    /// let a = Pattern::capture("a", Some(Arc::new(|node| {
    ///     node.properties().get(Property::Real) == Truth::True
    /// })));
    /// let rule = RewriteRule::new(a.pow(2).sqrt(), a.abs())?;
    /// let x = Expr::symbol("predicate_example_x");
    /// assert_eq!(rule.apply_all(&x.pow(2).sqrt(), None, None)?, x.pow(2).sqrt());
    /// # Ok::<(), symb_anafis::RewriteError>(())
    /// ```
    #[must_use]
    pub fn capture(name: impl AsRef<str>, predicate: Option<Arc<CapturePredicate>>) -> Self {
        Self(Some(Arc::new(PatternNode::Capture {
            name: Arc::from(name.as_ref()),
            predicates: predicate.map_or(CaptureConditions::Unrestricted, CaptureConditions::One),
        })))
    }

    /// Match one exact canonical expression.
    #[must_use]
    pub fn exact(expression: impl Into<Expr>) -> Self {
        Self(Some(Arc::new(PatternNode::Exact(expression.into()))))
    }

    /// Match a known function identity with validated ordered argument templates.
    ///
    /// # Errors
    /// Returns [`FunctionError::InvalidArity`] for an unaccepted argument count.
    ///
    /// # Panics
    /// Literal folding can propagate numeric backend capacity panics.
    pub fn apply(function: FunctionName, args: Vec<Self>) -> Result<Self, FunctionError> {
        FunctionDefinition::validate(&function, args.len())?;
        Ok(FunctionDefinition::template(function, args))
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

impl FunctionDefinition {
    /// Construct a validated template, folding calls with concrete arguments.
    /// Builtin aliases normalize; other captured calls retain structural arguments.
    pub fn template(function: FunctionName, args: Vec<Pattern>) -> Pattern {
        if function == BuiltinFunction::Log.function()
            && let [base, argument] = args.as_slice()
            && let PatternNode::Exact(expression) = base.node()
            && let Some(alias) = Self::logarithm_alias(expression)
        {
            return Self::template(alias.function(), vec![argument.clone()]);
        }
        if function == BuiltinFunction::ExpNeg.function()
            && let [argument] = args.as_slice()
        {
            return Self::template(BuiltinFunction::Exp.function(), vec![Neg::neg(argument)]);
        }
        let exact: Option<Vec<_>> = args
            .iter()
            .map(|argument| match argument.node() {
                PatternNode::Exact(expression) => Some(expression.clone()),
                PatternNode::Capture { .. }
                | PatternNode::Function { .. }
                | PatternNode::Sum(_)
                | PatternNode::Product(_)
                | PatternNode::Div(..)
                | PatternNode::Pow(..) => None,
            })
            .collect();
        if let Some(values) = exact {
            return Pattern::exact(Self::construct(function, values));
        }
        Pattern(Some(Arc::new(PatternNode::Function {
            name: function,
            args,
        })))
    }
}
