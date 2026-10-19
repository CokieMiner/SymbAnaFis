//! Rule validation and single-pass application.

use core::{cell::OnceCell, num::NonZeroU64, slice::from_ref as one};

use alloc::{borrow::Cow, vec::Vec};

use crate::collections::{HashMap, HashSet};

use super::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, ConcreteMatcher, ExactSources, Expr,
    ExprView, FunctionName, Group, GroupValues, MatchWork, Normalization, Pattern, PatternNode,
    Replacement, ReplacementProgram, RewriteError, RewriteGuard, RewriteRule, Search, Selected,
    Substitution, Task,
};

impl RewriteRule {
    /// Construct a rule with a compiled pattern or a pure replacement callback.
    /// Template captures must be supplied by the input pattern. Callback
    /// destinations receive only accepted bindings and remain dormant here.
    ///
    /// # Errors
    /// Returns `UnboundCapture` when the result uses an absent input capture.
    ///
    /// # Panics
    /// Propagates numeric backend capacity panics while normalizing literal
    /// templates. Internal template-order violations also panic.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use symb_anafis::{Context, Expr, Pattern, Replacement, RewriteRule};
    /// let context = Context::new();
    /// let x = context.symb("x")?;
    /// let a = Pattern::capture("a", None);
    /// let rule = RewriteRule::new(a.sin(), Replacement::Callback(Arc::new(|captures| {
    ///     captures.get("a").map_or_else(|| Expr::number(0), Expr::cos)
    /// })))?;
    /// assert_eq!(rule.apply_all(&x.sin(), None, None)?, x.cos());
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn new(
        pattern: Pattern,
        replacement: impl Into<Replacement>,
    ) -> Result<Self, RewriteError> {
        let pattern_order = PatternNode::template_order(&pattern);
        let associative = pattern_order.iter().any(|template| {
            matches!(
                template.node(),
                PatternNode::Sum(_) | PatternNode::Product(_)
            )
        });
        let captures: HashSet<_> = pattern_order
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
        let destination = if captures.is_empty() {
            let source = Normalization::default().lower(&pattern);
            if let ReplacementProgram::Callback(callback) = program {
                ReplacementProgram::ConcreteCallback { source, callback }
            } else {
                ReplacementProgram::Concrete(Substitution::new(
                    source,
                    program.instantiate(&HashMap::new()),
                ))
            }
        } else {
            program
        };
        drop(captures);
        Ok(Self {
            pattern,
            replacement: destination,
            associative,
        })
    }

    /// Apply bottom-up, sharing replacements on every edge of the input DAG.
    ///
    /// Root sum/product patterns replace disjoint operand groups and retain all
    /// unmatched operands. Nested groups cover the complete operand list,
    /// with captures allowed to bind nonempty groups of terms or factors.
    /// Replacements are not matched again during this pass. Each capture binds
    /// one canonical expression, including operand groups. Positive integer powers, collected
    /// integer addends, and finite nonzero scalar coefficients support
    /// disjoint implicit groups. Coefficient quotients use native numeric
    /// promotion. Captures prefer whole operands before smaller counted blocks
    /// and grouped partitions. Positive power bases are tried in increasing
    /// block size; new
    /// quotients are not capture candidates. Repeated replacements stay compressed.
    /// Exact rational templates consume integral groups on an unchanged principal
    /// base. Integral outer powers can also capture fractional blocks already
    /// present in the input. Reciprocal templates require denominator factors;
    /// nested reciprocal exclusions remain opaque. Bare expressions do not infer
    /// fractional roots. Arbitrary algebraic identities require explicit rules.
    /// Missing scalar prefixes are inferred by dividing the whole operand:
    /// `2*a -> f(a)` maps a bare `x` to `f(x/2)`. This rule also matches other
    /// subtrees, including numeric literals. Matching remains bottom-up; use a
    /// function template or guard when only selected nodes should change.
    ///
    /// `match_limit = None` permits unrestricted commutative search. A supplied
    /// limit bounds candidate attempts across the whole pass; complete ordered
    /// matches consume no attempts. Capture-free rules share concrete matching
    /// with Substitution, including implicit units, common additive factors,
    /// and exact rational power groups whose branches and zero exclusions are preserved.
    /// Rules match rebuilt nodes bottom-up; simultaneous substitutions match
    /// original nodes before rebuilding children.
    /// Exhaustion stops traversal and discards the
    /// partial output. The immutable input remains valid.
    /// A counted assignment can cover many identical groups with one guard call;
    /// the bound includes virtual candidate attempts, not every repeated unit.
    ///
    /// `guard = None` accepts every structural match. A guard may reject an
    /// assignment and search another; it can run more than once per input node.
    /// Callback side effects preceding an error are not rolled back.
    /// Replacement callbacks run only after guard acceptance. They must be
    /// stable for identical bindings: counted groups share one destination,
    /// and capture-free destinations are cached lazily within this pass.
    ///
    /// # Errors
    /// Returns [`RewriteError::MatchLimitExceeded`] when another attempt is needed
    /// after the supplied candidate limit has been reached.
    ///
    /// # Panics
    /// Propagates capture predicate, guard, replacement callback, and numeric
    /// backend capacity panics.
    ///
    /// ```
    /// use symb_anafis::{Context, Pattern, RewriteRule};
    /// let context = Context::new();
    /// let x = context.symb("x")?;
    /// let u = context.symb("u")?;
    /// let rule = RewriteRule::new(Pattern::exact(x.pow(2)), Pattern::exact(u))?;
    /// assert_eq!(rule.apply_all(&x.pow(5), None, None)?, u.pow(2) * x);
    /// # Ok::<(), Box<dyn core::error::Error>>(())
    /// ```
    pub fn apply_all(
        &self,
        expression: &Expr,
        match_limit: Option<usize>,
        mut guard: Option<&mut RewriteGuard<'_>>,
    ) -> Result<Expr, RewriteError> {
        let mut work = MatchWork {
            remaining: match_limit,
            limit: match_limit.unwrap_or(0),
        };
        let mut accepts =
            |captures: &HashMap<&str, Expr>| guard.as_mut().is_none_or(|check| check(captures));
        if let ReplacementProgram::Concrete(entry) = &self.replacement {
            let matcher = ConcreteMatcher::new(ExactSources::One(entry));
            let captures = HashMap::new();
            return expression.map(|node| {
                let replacement = matcher.replace(node);
                if replacement.is_some()
                    && node != entry.from()
                    && let Some(remaining) = &mut work.remaining
                {
                    *remaining = remaining
                        .checked_sub(1)
                        .ok_or(RewriteError::MatchLimitExceeded { limit: work.limit })?;
                }
                Ok(replacement.filter(|_| accepts(&captures)))
            });
        }
        if let ReplacementProgram::ConcreteCallback { source, callback } = &self.replacement {
            let probe = Substitution::new(source.clone(), source.clone());
            let matcher = ConcreteMatcher::new(ExactSources::One(&probe));
            let resolved_entry = OnceCell::new();
            let resolved_matcher = OnceCell::new();
            let captures = HashMap::new();
            return expression.map(|node| {
                let active: &ConcreteMatcher<'_> = resolved_matcher.get().unwrap_or(&matcher);
                let replacement = active.replace(node);
                if replacement.is_some()
                    && node != source
                    && let Some(remaining) = &mut work.remaining
                {
                    *remaining = remaining
                        .checked_sub(1)
                        .ok_or(RewriteError::MatchLimitExceeded { limit: work.limit })?;
                }
                if replacement.is_none() || !accepts(&captures) {
                    return Ok(None);
                }
                if resolved_matcher.get().is_some() {
                    return Ok(replacement);
                }
                let resolved = resolved_entry
                    .get_or_init(|| Substitution::new(source.clone(), callback(&captures)));
                let prepared = resolved_matcher
                    .get_or_init(|| ConcreteMatcher::new(ExactSources::One(resolved)));
                Ok(prepared.replace(node))
            });
        }
        expression.map(|node| {
            if let Some(captures) = PatternNode::match_ordered(&self.pattern, node)
                && accepts(&captures)
            {
                return Ok(Some(self.replacement.instantiate(&captures)));
            }
            if let PatternNode::Sum(patterns) | PatternNode::Product(patterns) = self.pattern.node()
            {
                let sum = matches!(self.pattern.node(), PatternNode::Sum(_));
                if let Some(replacement) =
                    self.rewrite_explicit(patterns, node, sum, &mut work, &mut accepts)?
                {
                    return Ok(Some(replacement));
                }
            }
            if self.associative
                && !matches!(
                    self.pattern.node(),
                    PatternNode::Sum(_) | PatternNode::Product(_)
                )
            {
                let mut search = Search::new();
                search
                    .pending
                    .push(Task::Match(&self.pattern, node.clone()));
                if let Some(matched) = search.run(&mut work, &mut accepts)? {
                    return Ok(Some(self.replacement.instantiate(&matched.captures)));
                }
            }
            if let Some(replacement) = self.rewrite_counted(node, &mut work, &mut accepts)? {
                return Ok(Some(replacement));
            }
            if let PatternNode::Product(factors) = self.pattern.node()
                && matches!(
                    factors.first().map(Pattern::node),
                    Some(PatternNode::Exact(_))
                )
            {
                let mut search = Search::new();
                search
                    .pending
                    .push(Task::Match(&self.pattern, node.clone()));
                if let Some(matched) = search.run(&mut work, &mut accepts)? {
                    return Ok(Some(self.replacement.instantiate(&matched.captures)));
                }
            }
            Ok(None)
        })
    }

    /// Enumerate function identities whose absence rules out a structural match.
    ///
    /// Captures and exact-expression templates add no requirements. Square roots
    /// are excluded because counted rational powers can supply their groups
    /// without storing a square-root call. Identities may occur more than once;
    /// their presence is necessary but does not establish a match.
    pub fn required_functions(&self) -> impl Iterator<Item = FunctionName> {
        PatternNode::template_order(&self.pattern)
            .into_iter()
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

    /// Select whole original operands before searching their remaining counted units.
    fn rewrite_explicit(
        &self,
        patterns: &[Pattern],
        node: &Expr,
        sum: bool,
        work: &mut MatchWork,
        accepts: &mut impl FnMut(&HashMap<&str, Expr>) -> bool,
    ) -> Result<Option<Expr>, RewriteError> {
        if patterns.is_empty() {
            return Ok(None);
        }
        let projection = if sum {
            node.additive_operands()
        } else if let ExprView::Product(factors) = node.view() {
            Some(Cow::Borrowed(factors))
        } else {
            None
        };
        let Some(operands) = projection else {
            return Ok(None);
        };
        let mut original: Vec<_> = operands
            .iter()
            .cloned()
            .map(|operand| (operand, true))
            .collect();
        let mut remaining = original.len();
        let mut start = 0;
        let mut inserted = Vec::new();
        while remaining >= patterns.len() {
            let mut search = Search::new();
            search.pending.push(Task::Group(Group {
                patterns,
                values: GroupValues::Borrowed(&original),
                selected: Selected::Explicit(Vec::new()),
                next: start,
                projection: 0,
                combination: None,
                sum,
                start,
                root: true,
            }));
            let Some(matched) = search.run(work, accepts)? else {
                break;
            };
            // Nonempty groups select distinct available indices, so every match progresses.
            let Selected::Explicit(indices) = matched.selected else {
                return Ok(None);
            };
            remaining = remaining.saturating_sub(indices.len());
            for index in indices {
                if let Some((_, keep)) = original.get_mut(index) {
                    *keep = false;
                }
            }
            start = original
                .iter()
                .enumerate()
                .skip(start)
                .find_map(|(index, (_, keep))| keep.then_some(index))
                .unwrap_or(original.len());
            inserted.push(self.replacement.instantiate(&matched.captures));
        }
        if inserted.is_empty() {
            return Ok(None);
        }
        let terms = original
            .into_iter()
            .filter_map(|(operand, keep)| keep.then_some(operand))
            .collect();
        let remainder = if sum {
            Expr::sum(terms)
        } else {
            Expr::product(terms)
        };
        let transformed = self
            .rewrite_counted(&remainder, work, accepts)?
            .unwrap_or(remainder);
        inserted.push(transformed);
        Ok(Some(if sum {
            Expr::sum(inserted)
        } else {
            Expr::product(inserted)
        }))
    }

    /// Match compressed original arithmetic contributions and preserve their remainder.
    fn rewrite_counted(
        &self,
        node: &Expr,
        work: &mut MatchWork,
        accepts: &mut impl FnMut(&HashMap<&str, Expr>) -> bool,
    ) -> Result<Option<Expr>, RewriteError> {
        let (sum, patterns) = match self.pattern.node() {
            PatternNode::Sum(patterns) => (true, patterns.as_slice()),
            PatternNode::Product(patterns) => (false, patterns.as_slice()),
            template if template.power_template().is_some() => (false, one(&self.pattern)),
            PatternNode::Exact(_)
            | PatternNode::Capture { .. }
            | PatternNode::Function { .. }
            | PatternNode::Div(..)
            | PatternNode::Pow(..) => return Ok(None),
        };
        let supported = sum
            || matches!(
                node.view(),
                ExprView::Product(_) | ExprView::Pow(..) | ExprView::Div(..)
            )
            || matches!(node.view(), ExprView::Function { name, .. } if *name == BuiltinFunction::Sqrt.function());
        if patterns.is_empty() || !supported {
            return Ok(None);
        }
        let Some(mut operands) = AlgebraicOperands::new(node, sum, 0) else {
            return Ok(None);
        };
        if !sum {
            let mut denominator = NonZeroU64::MIN;
            for pattern in patterns {
                if let Some(power) = pattern.node().power_template() {
                    let mut first = denominator.get();
                    let mut second = power.denominator.get();
                    while second != 0 {
                        let Some(remainder) = first.checked_rem(second) else {
                            break;
                        };
                        first = second;
                        second = remainder;
                    }
                    let Some(common) = power
                        .denominator
                        .get()
                        .checked_div(first)
                        .and_then(|scale| denominator.get().checked_mul(scale))
                        .and_then(NonZeroU64::new)
                    else {
                        return Ok(None);
                    };
                    denominator = common;
                }
            }
            operands.refine_powers(denominator);
        }
        let mut inserted = Vec::new();
        loop {
            let mut search = Search::new();
            search.pending.push(Task::Group(Group {
                patterns,
                values: GroupValues::Counted(&operands),
                selected: Selected::Counted(Vec::new()),
                next: 0,
                projection: 0,
                combination: None,
                sum,
                start: 0,
                root: true,
            }));
            let Some(matched) = search.run(work, accepts)? else {
                break;
            };
            let Selected::Counted(selected) = matched.selected else {
                return Ok(None);
            };
            let repetitions = operands.consume(&selected);
            let replacement = self.replacement.instantiate(&matched.captures);
            inserted.push(AlgebraicOperands::repeat(replacement, repetitions, sum));
        }
        Ok((!inserted.is_empty()).then(|| operands.finish(inserted)))
    }
}
