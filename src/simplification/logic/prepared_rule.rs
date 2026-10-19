//! A compiled builtin rule with its mathematical contract and dispatch filter.

use crate::{
    collections::HashSet,
    core::{Expr, Function, Pattern, RewriteRule},
    errors::RewriteError,
};

use super::DomainSafety;

pub struct PreparedRule {
    rule: RewriteRule,
    required: HashSet<Function>,
    safety: DomainSafety,
}

impl PreparedRule {
    pub fn new(source: Pattern, destination: Pattern, safety: DomainSafety) -> Self {
        let rule = RewriteRule::new(source, destination)
            .expect("builtin destinations use only source captures");
        let required = rule.required_functions().collect();
        Self {
            rule,
            required,
            safety,
        }
    }

    pub const fn domain_safety(&self) -> DomainSafety {
        self.safety
    }

    pub fn apply(
        &self,
        expression: &Expr,
        functions: &HashSet<Function>,
        match_limit: Option<usize>,
    ) -> Result<Expr, RewriteError> {
        if self.required.is_subset(functions) {
            match self.rule.apply_all(expression, match_limit, None) {
                Ok(rewritten) => Ok(rewritten),
                Err(RewriteError::MatchLimitExceeded { .. }) => Ok(expression.clone()),
                Err(error) => Err(error),
            }
        } else {
            Ok(expression.clone())
        }
    }
}
