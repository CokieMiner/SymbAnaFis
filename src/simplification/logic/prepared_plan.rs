//! Reusable compiled rule storage, shared when a sequence is cloned.

use alloc::{sync::Arc, vec::Vec};

use super::{DomainSafety, PreparedRule, PreparedStep, SimplificationStep};

#[derive(Clone)]
pub struct PreparedPlan {
    preserving: Option<Arc<[PreparedRule]>>,
    assumed: Option<Arc<[PreparedRule]>>,
}

impl PreparedPlan {
    pub fn new(step: &SimplificationStep) -> Self {
        Self {
            preserving: Self::compile(step, DomainSafety::Preserving),
            assumed: None,
        }
    }

    pub fn enable_assumptions(&mut self, step: &SimplificationStep) {
        if self.assumed.is_none() {
            self.assumed = Self::compile(step, DomainSafety::AssumptionDependent);
        }
    }

    pub fn rules(&self, safety: DomainSafety) -> impl Iterator<Item = &PreparedRule> {
        let preserving = self.preserving.as_deref().unwrap_or(&[]);
        let assumed = if safety == DomainSafety::AssumptionDependent {
            self.assumed.as_deref().unwrap_or(&[])
        } else {
            &[]
        };
        preserving.iter().chain(assumed)
    }

    pub fn is_empty(&self, safety: DomainSafety) -> bool {
        self.preserving.is_none() && (safety == DomainSafety::Preserving || self.assumed.is_none())
    }

    fn compile(step: &SimplificationStep, tag: DomainSafety) -> Option<Arc<[PreparedRule]>> {
        let rules = PreparedStep::rules(step, tag)
            .into_iter()
            .filter(|rule| rule.domain_safety() == tag)
            .collect::<Vec<_>>();
        (!rules.is_empty()).then(|| Arc::from(rules))
    }
}
