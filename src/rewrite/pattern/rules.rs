//! Rule validation, grounded subtemplates, and public rule queries.

use core::{iter::once, num::NonZeroU64, slice::from_ref};

use alloc::{boxed::Box, vec::Vec};

use num_anafis::NonZeroU64Ext;

use crate::collections::{FixedSet, HashMap};

use super::{
    BuiltinFunction, Expr, Function, Heads, Normalization, Pattern, PatternNode, Replacement,
    ReplacementOp, ReplacementProgram, RewriteError, RewriteGuard, RewriteRule, RewriteTarget,
    RulePass, Substitution,
};

impl RewriteRule {
    /// A rule from `pattern` to a replacement template or a pure callback.
    /// Template captures must occur in the pattern, and a callback runs only on
    /// accepted bindings.
    ///
    /// # Errors
    /// Returns [`RewriteError::UnboundCapture`] when the replacement uses a
    /// capture absent from the pattern.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use symb_anafis::{Context, Expr, Pattern, Replacement, RewriteRule};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let a = Pattern::capture("a", None);
    /// let rule = RewriteRule::new(a.sin(), Replacement::Callback(Arc::new(|captures| {
    ///     captures.get("a").map_or_else(|| Expr::number(0), Expr::cos)
    /// })))?;
    /// assert_eq!(rule.apply_all(&x.sin(), usize::MAX, None)?, x.cos());
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn new(
        pattern: Pattern,
        replacement: impl Into<Replacement>,
    ) -> Result<Self, RewriteError> {
        let order = PatternNode::template_order(&pattern);
        let pattern_order: Vec<&Pattern> = order
            .descendants
            .iter()
            .map(|visit| visit.pattern)
            .chain(once(order.root.pattern))
            .collect();
        let associative = pattern_order.iter().any(|template| {
            matches!(
                template.node(),
                PatternNode::Sum(_) | PatternNode::Product(_)
            )
        });
        let captures: FixedSet<_> = pattern_order
            .iter()
            .filter_map(|template| {
                if let PatternNode::Capture { name, .. } = template.node() {
                    Some(name.as_ref())
                } else {
                    None
                }
            })
            .collect();
        let program = ReplacementProgram::compile(replacement.into(), &captures)?;
        let grounds = Self::grounds(&pattern_order, &captures);
        let destination = if captures.is_empty() {
            let source = Normalization::default().lower(&pattern);
            if let ReplacementProgram::Callback(callback) = program {
                ReplacementProgram::ConcreteCallback { source, callback }
            } else {
                // Validation admits no capture into a capture-free template.
                program
                    .instantiate(&HashMap::new())
                    .map_or(program, |destination| {
                        ReplacementProgram::Concrete(Substitution::new(source, destination))
                    })
            }
        } else {
            program
        };
        drop(captures);
        // The heads each matching stage of `rewrite_pattern` accepts at the root.
        let power = pattern.node().power_template().is_some();
        let heads = match pattern.node() {
            PatternNode::Capture { .. } | PatternNode::Exact(_) => Heads::ALL,
            PatternNode::Sum(_) => Heads::SUM,
            // A missing exact coefficient is inferred by dividing any node, and
            // the other factors must then cover every factor of the quotient,
            // including the coefficient's reciprocal. Only a capture or an
            // exact value matches that literal, so without one the pattern
            // matches only where a product without inference does.
            PatternNode::Product(factors)
                if let Some((first, rest)) = factors.split_first()
                    && matches!(first.node(), PatternNode::Exact(_)) =>
            {
                if rest.iter().all(|factor| {
                    !matches!(
                        factor.node(),
                        PatternNode::Capture { .. } | PatternNode::Exact(_)
                    )
                }) {
                    Heads::COUNTED
                } else {
                    Heads::ALL
                }
            }
            // Explicit factors, then counted factor groups.
            PatternNode::Product(_) => Heads::COUNTED,
            // An exact power template matches its own head and counted groups.
            PatternNode::Function { .. } | PatternNode::Div(..) | PatternNode::Pow(..) if power => {
                Heads::COUNTED
            }
            PatternNode::Function { name, .. } if *name == BuiltinFunction::Sqrt.function() => {
                Heads::SQRT
            }
            PatternNode::Function { .. } => Heads::CALL,
            PatternNode::Div(..) => Heads::DIV,
            PatternNode::Pow(..) => Heads::POW,
        };
        // Counted factor groups use one unit for every exact power template.
        let common = |factors: &[Pattern]| {
            factors.iter().try_fold(NonZeroU64::MIN, |common, factor| {
                factor
                    .node()
                    .power_template()
                    .map_or(Some(common), |template| common.lcm(template.denominator))
            })
        };
        let counted = match pattern.node() {
            PatternNode::Sum(_) => Some(NonZeroU64::MIN),
            PatternNode::Product(factors) => common(factors),
            PatternNode::Function { .. } | PatternNode::Div(..) | PatternNode::Pow(..) if power => {
                common(from_ref(&pattern))
            }
            PatternNode::Capture { .. }
            | PatternNode::Exact(_)
            | PatternNode::Function { .. }
            | PatternNode::Div(..)
            | PatternNode::Pow(..) => None,
        };
        Ok(Self {
            pattern,
            replacement: destination,
            associative,
            heads,
            counted,
            grounds,
        })
    }

    /// The rigid operand templates among the pattern nodes, compiled with the
    /// pattern's captures. An operand template of a sum or product without
    /// sums or products of its own, other than a bare capture, is rigid: it
    /// matches only the expression it instantiates to.
    fn grounds(
        nodes: &[&Pattern],
        captures: &FixedSet<&str>,
    ) -> Box<[(Pattern, ReplacementProgram)]> {
        nodes
            .iter()
            .filter_map(|template| match template.node() {
                PatternNode::Sum(operands) | PatternNode::Product(operands) => Some(operands),
                PatternNode::Capture { .. }
                | PatternNode::Exact(_)
                | PatternNode::Function { .. }
                | PatternNode::Div(..)
                | PatternNode::Pow(..) => None,
            })
            .flatten()
            .filter(|operand| !matches!(operand.node(), PatternNode::Capture { .. }))
            .filter_map(|operand| {
                let compiled =
                    ReplacementProgram::compile(Replacement::Pattern(operand.clone()), captures)
                        .ok()?;
                let ReplacementProgram::Template(template) = &compiled else {
                    return None;
                };
                (!template
                    .operands
                    .iter()
                    .chain(once(&template.root))
                    .any(|operation| {
                        matches!(operation, ReplacementOp::Sum(_) | ReplacementOp::Product(_))
                    }))
                .then(|| (operand.clone(), compiled))
            })
            .collect()
    }

    /// Apply the rule once, bottom-up, sharing each replacement on every edge to
    /// a shared node. Replacements are not matched again in this pass.
    ///
    /// # Matching
    ///
    /// - A root sum or product pattern replaces disjoint operand groups and keeps
    ///   the unmatched operands; a nested sum or product must cover its whole
    ///   operand list. A capture binds one canonical expression, which may be a
    ///   nonempty group of terms or factors.
    /// - Positive integer powers, collected integer addends, and finite nonzero
    ///   numeric coefficients split into disjoint implicit groups. Captures try
    ///   whole operands first, then smaller counted blocks and grouped
    ///   partitions; power bases are tried in increasing block size. Repeated
    ///   replacements stay compressed, and new quotients are not capture
    ///   candidates.
    /// - Exact rational templates consume integral groups on an unchanged
    ///   principal base. Integer outer powers can capture fractional blocks
    ///   present in the input. Reciprocal templates require denominator factors,
    ///   and nested reciprocal exclusions stay opaque. A bare expression never
    ///   yields an inferred fractional root.
    /// - A missing numeric prefix is inferred by dividing the whole operand:
    ///   `2*a -> f(a)` maps a bare `x` to `f(x/2)`. Such a rule also matches
    ///   other subtrees, numeric literals included; a function template or a
    ///   guard restricts it.
    /// - Capture-free rules use the concrete matching of [`Substitution`]:
    ///   implicit units, common additive factors, and exact rational power
    ///   groups that preserve principal branches and zero exclusions. Rules
    ///   match rebuilt nodes bottom-up, while substitutions match original nodes
    ///   before rebuilding their children.
    ///
    /// Other algebraic identities need explicit rules.
    ///
    /// # Search bound and callbacks
    ///
    /// `match_limit` bounds candidate attempts across the whole pass;
    /// `usize::MAX` is unbounded. Complete ordered matches consume none, and a
    /// counted assignment covering many identical groups counts once.
    /// Exhaustion stops the pass and discards its output.
    ///
    /// `guard = None` accepts every structural match. A guard may reject an
    /// assignment so that another is searched, and it may run more than once per
    /// node. Replacement callbacks run only after the guard accepts. They must
    /// return the same result for the same bindings, because counted groups
    /// share one destination and capture-free destinations are cached for the
    /// pass. Callback side effects before an error are not rolled back.
    ///
    /// # Errors
    /// Returns [`RewriteError::MatchLimitExceeded`] when another attempt is
    /// needed after `match_limit` attempts.
    ///
    /// # Panics
    /// Propagates panics of capture predicates, the guard, and replacement
    /// callbacks.
    ///
    /// ```
    /// use symb_anafis::{Context, Pattern, RewriteRule};
    /// let context = Context::new()?;
    /// let x = context.symb("x")?;
    /// let u = context.symb("u")?;
    /// let rule = RewriteRule::new(Pattern::exact(x.pow(2)), Pattern::exact(u))?;
    /// assert_eq!(rule.apply_all(&x.pow(5), usize::MAX, None)?, u.pow(2) * x);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn apply_all(
        &self,
        expression: &Expr,
        match_limit: usize,
        mut guard: Option<&mut RewriteGuard<'_>>,
    ) -> Result<Expr, RewriteError> {
        let mut accepts =
            |captures: &HashMap<&str, Expr>| guard.as_mut().is_none_or(|check| check(captures));
        let mut pass = RulePass::new(self, match_limit);
        expression.map(|node| pass.rewrite(&mut RewriteTarget::new(node), &mut accepts))
    }

    /// Enumerate function identities whose absence rules out a structural match.
    ///
    /// Captures and exact-expression templates add no requirements. Square roots
    /// are excluded because counted rational powers can supply their groups
    /// without storing a square-root call. Identities may occur more than once;
    /// their presence is necessary but does not establish a match.
    pub fn required_functions(&self) -> impl Iterator<Item = Function> {
        let order = PatternNode::template_order(&self.pattern);
        order
            .descendants
            .into_iter()
            .map(|visit| visit.pattern)
            .chain(once(order.root.pattern))
            .filter_map(|pattern| {
                if let PatternNode::Function { name, .. } = pattern.node()
                    && *name != BuiltinFunction::Sqrt.function()
                {
                    Some(name.clone())
                } else {
                    None
                }
            })
    }
}
