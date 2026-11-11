//! Rule validation, grounded subtemplates, and public rule queries.

use core::{iter::once, num::NonZeroU64, ptr::from_ref};

use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};

use num_anafis::NonZeroU64Ext;

use crate::collections::{FixedSet, HashMap};

use super::{
    AlgebraicOperands, BuiltinFunction, Expr, Function, Heads, NodeShape, Normalization, Pattern,
    PatternNode, Replacement, ReplacementOp, ReplacementProgram, RewriteError, RewriteGuard,
    RewriteRule, RewriteTarget, RulePass, Substitution, Template, TemplateOrder,
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
        let mut associative = false;
        let mut names: Vec<Arc<str>> = Vec::new();
        for template in &pattern_order {
            match template.node() {
                PatternNode::Sum(_) | PatternNode::Product(_) => associative = true,
                PatternNode::Capture { name, .. }
                    if !names
                        .iter()
                        .any(|seen| Arc::ptr_eq(seen, name) || seen.as_ref() == name.as_ref()) =>
                {
                    names.push(Arc::clone(name));
                }
                PatternNode::Capture { .. }
                | PatternNode::Exact(_)
                | PatternNode::Function { .. }
                | PatternNode::Div(..)
                | PatternNode::Pow(..) => {}
            }
        }
        let capture_names = names.into_boxed_slice();
        let destination = match replacement.into() {
            Replacement::Callback(callback) if capture_names.is_empty() => {
                ReplacementProgram::ConcreteCallback {
                    source: Normalization::default().lower(&pattern),
                    callback,
                }
            }
            Replacement::Callback(callback) => ReplacementProgram::Callback(callback),
            Replacement::Pattern(root) => {
                let template = Template::compile(&root, &capture_names)?;
                if capture_names.is_empty() {
                    let source = Normalization::default().lower(&pattern);
                    // Validation admits no capture into a capture-free template.
                    template.instantiate(&[], &mut Vec::new()).map_or_else(
                        || ReplacementProgram::Template(template),
                        |output| ReplacementProgram::Concrete(Substitution::new(source, output)),
                    )
                } else {
                    ReplacementProgram::Template(template)
                }
            }
        };
        let grounds = Self::grounds(&pattern_order, &capture_names);
        // Classify root head, call filter, counted step denominator, and
        // inferred-scalar eligibility in one pass over the root shape.
        let power = pattern.node().power_template();
        let (inferred_coefficient, heads, root_function, counted) = match pattern.node() {
            PatternNode::Capture { .. } | PatternNode::Exact(_) => (false, Heads::ALL, None, None),
            PatternNode::Sum(_) => (false, Heads::SUM, None, Some(NonZeroU64::MIN)),
            PatternNode::Product(factors) => {
                let inferred = factors.split_first().is_some_and(|(first, rest)| {
                    !rest.is_empty()
                        && matches!(first.node(), PatternNode::Exact(v) if AlgebraicOperands::divide_scalar(&Expr::number(1), v).is_some())
                        && rest.iter().any(|f| {
                            matches!(f.node(), PatternNode::Capture { .. } | PatternNode::Exact(_))
                        })
                });
                let denom = factors.iter().try_fold(NonZeroU64::MIN, |acc, f| {
                    f.node()
                        .power_template()
                        .map_or(Some(acc), |t| acc.lcm(t.denominator))
                });
                let head = if inferred { Heads::ALL } else { Heads::COUNTED };
                (inferred, head, None, denom)
            }
            PatternNode::Function { name, .. } if power.is_none() => {
                if *name == BuiltinFunction::Sqrt.function() {
                    (false, Heads::SQRT, None, None)
                } else {
                    (false, Heads::CALL, Some(name.clone()), None)
                }
            }
            PatternNode::Div(..) if power.is_none() => (false, Heads::DIV, None, None),
            PatternNode::Pow(..) if power.is_none() => (false, Heads::POW, None, None),
            PatternNode::Function { .. } | PatternNode::Div(..) | PatternNode::Pow(..) => {
                (false, Heads::COUNTED, None, power.map(|t| t.denominator))
            }
        };
        let nodes = pattern_order.len();
        let repeats = Self::repeats(&order);
        Ok(Self {
            pattern,
            replacement: destination,
            capture_names,
            associative,
            inferred_coefficient,
            heads,
            root_function,
            counted,
            grounds,
            nodes,
            repeats,
        })
    }

    /// The template nodes whose expansions can repeat.
    ///
    /// A `(template, input)` pair is expanded a second time only when the
    /// template is reached through two parents, or when an ancestor is: a tree
    /// reaches every node through one parent, so its pairs are unique. The set
    /// therefore holds every node that is shared or lies below a shared node,
    /// and a search caches only those.
    fn repeats(order: &TemplateOrder<'_>) -> FixedSet<usize> {
        let mut parents = vec![0_usize; order.descendants.len()];
        for visit in order.descendants.iter().chain(once(&order.root)) {
            for &child in &visit.children {
                if let Some(count) = parents.get_mut(child) {
                    *count = count.saturating_add(1);
                }
            }
        }
        let mut repeats = vec![false; order.descendants.len()];
        for &child in &order.root.children {
            if let Some(flag) = repeats.get_mut(child) {
                *flag = parents.get(child).copied().unwrap_or(0) > 1;
            }
        }
        let mut addresses = FixedSet::default();
        // A parent finishes after its children, so a decreasing position visits
        // every parent before the nodes it marks.
        for (position, visit) in order.descendants.iter().enumerate().rev() {
            let inherited = repeats.get(position).copied().unwrap_or(false);
            for &child in &visit.children {
                if let Some(flag) = repeats.get_mut(child) {
                    *flag = inherited || parents.get(child).copied().unwrap_or(0) > 1;
                }
            }
            if inherited {
                addresses.insert(from_ref(visit.pattern.node()).addr());
            }
        }
        addresses
    }

    /// The rigid operand templates among the pattern nodes, compiled with the
    /// pattern's capture slots. An operand template of a sum or product without
    /// sums or products of its own, other than a bare capture, is rigid: it
    /// matches only the expression it instantiates to.
    fn grounds(nodes: &[&Pattern], capture_names: &[Arc<str>]) -> Box<[(usize, Template)]> {
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
                let template = Template::compile(operand, capture_names).ok()?;
                (!template
                    .operands
                    .iter()
                    .chain(once(&template.root))
                    .any(|operation| {
                        matches!(operation, ReplacementOp::Sum(_) | ReplacementOp::Product(_))
                    }))
                .then_some((from_ref(operand.node()).addr(), template))
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
    /// `match_limit` bounds the candidate attempts one match attempt may spend
    /// beyond the linear scan of its operand group; `usize::MAX` is unbounded
    /// and `0` disables matching. Complete ordered matches consume none, and a
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
        let mut accepts = |slots: &[Option<Expr>], bindings: &mut Option<HashMap<_, Expr>>| {
            let Some(check) = guard.as_mut() else {
                return true;
            };
            let map =
                bindings.get_or_insert_with(|| HashMap::with_capacity(self.capture_names.len()));
            map.clear();
            for (name, value) in self.capture_names.iter().zip(slots) {
                if let Some(bound) = value {
                    map.insert(name.as_ref(), bound.clone());
                }
            }
            check(map)
        };
        let mut pass = RulePass::new(self, match_limit);
        expression.map(|node| {
            pass.rewrite(
                &mut RewriteTarget::new(node),
                NodeShape::of(node),
                &mut accepts,
            )
        })
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
