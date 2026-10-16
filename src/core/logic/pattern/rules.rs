//! Rule validation and single-pass application.

use core::{ptr::from_ref, slice::from_ref as one};

use alloc::{borrow::Cow, sync::Arc, vec::Vec};

use crate::collections::{HashMap, HashSet};

use super::{
    AdditiveOperands, AlgebraicOperands, ConcreteMatcher, ExactSources, Expr, ExprView, Group,
    GroupValues, MatchWork, Normalization, Pattern, PatternNode, ReplacementOp, RewriteError,
    RewriteGuard, RewriteRule, Search, Selected, Substitution, Task,
};

impl RewriteRule {
    /// Validate that every replacement capture is supplied by the input pattern.
    ///
    /// # Errors
    /// Returns `UnboundCapture` when the result uses an absent input capture.
    ///
    /// # Panics
    /// Propagates numeric backend capacity panics while normalizing literal
    /// templates. Internal template-order violations also panic.
    pub fn new(pattern: Pattern, replacement: Pattern) -> Result<Self, RewriteError> {
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
                if let PatternNode::Capture(name) = template.node() {
                    Some(name.as_ref())
                } else {
                    None
                }
            })
            .collect();
        let mut instructions = Vec::new();
        let mut indices = HashMap::new();
        for template in PatternNode::template_order(&replacement) {
            let index = |child: &Pattern| {
                *indices
                    .get(&from_ref(child.node()))
                    .expect("compiled children precede their parent")
            };
            let operation = match template.node() {
                PatternNode::Capture(name) if !captures.contains(name.as_ref()) => {
                    return Err(RewriteError::UnboundCapture(name.as_ref().into()));
                }
                PatternNode::Capture(name) => ReplacementOp::Capture(Arc::clone(name)),
                PatternNode::Exact(value) => ReplacementOp::Exact(value.clone()),
                PatternNode::Function { name, args } => ReplacementOp::Function {
                    name: name.clone(),
                    args: args.iter().map(index).collect(),
                },
                PatternNode::Sum(args) => ReplacementOp::Sum(args.iter().map(index).collect()),
                PatternNode::Product(args) => {
                    ReplacementOp::Product(args.iter().map(index).collect())
                }
                PatternNode::Div(left, right) => ReplacementOp::Div(index(left), index(right)),
                PatternNode::Pow(base, exponent) => {
                    ReplacementOp::Pow(index(base), index(exponent))
                }
            };
            indices.insert(from_ref(template.node()), instructions.len());
            instructions.push(operation);
        }
        let concrete = captures.is_empty().then(|| {
            Substitution::new(
                Normalization::default().lower(&pattern),
                ReplacementOp::instantiate(&instructions, &HashMap::new()),
            )
        });
        if concrete.is_some() {
            instructions = Vec::new();
        }
        drop(captures);
        drop(replacement);
        Ok(Self {
            pattern,
            replacement: instructions,
            associative,
            concrete,
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
    /// Negative/fractional powers and arbitrary algebraic identities are opaque.
    /// Missing scalar prefixes are inferred by dividing the whole operand:
    /// `2*a -> f(a)` maps a bare `x` to `f(x/2)`. This rule also matches other
    /// subtrees, including numeric literals. Matching remains bottom-up; use a
    /// function template or guard when only selected nodes should change.
    ///
    /// `match_limit = None` permits unrestricted commutative search. A supplied
    /// limit bounds candidate attempts across the whole pass; complete ordered
    /// matches consume no attempts. Capture-free rules share concrete matching
    /// with Substitution, including implicit units and signed additive scales.
    /// Exhaustion stops traversal and discards the
    /// partial output. The immutable input remains valid.
    /// A counted assignment can cover many identical groups with one guard call;
    /// the bound includes virtual candidate attempts, not every repeated unit.
    ///
    /// `guard = None` accepts every structural match. A guard may reject an
    /// assignment and search another; it can run more than once per input node.
    /// Callback side effects preceding an error are not rolled back.
    ///
    /// # Errors
    /// Returns [`RewriteError::MatchLimitExceeded`] when another attempt is needed
    /// after the supplied candidate limit has been reached.
    ///
    /// # Panics
    /// Propagates guard callback and numeric backend capacity panics.
    ///
    /// ```
    /// use symb_anafis::{Context, Pattern, RewriteRule};
    /// let context = Context::new();
    /// let x = context.symb("x");
    /// let u = context.symb("u");
    /// let rule = RewriteRule::new(Pattern::exact(x.pow(2)), Pattern::exact(u))?;
    /// assert_eq!(rule.apply_all(&x.pow(5), None, None)?, u.pow(2) * x);
    /// # Ok::<(), symb_anafis::RewriteError>(())
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
        if let Some(entry) = &self.concrete {
            let matcher = ConcreteMatcher::new(ExactSources::One(entry));
            return expression.map(|node| {
                let replacement = matcher.replace(node);
                if replacement.is_some()
                    && !node.same_node(entry.from())
                    && let Some(remaining) = &mut work.remaining
                {
                    *remaining = remaining
                        .checked_sub(1)
                        .ok_or(RewriteError::MatchLimitExceeded { limit: work.limit })?;
                }
                Ok(replacement.filter(|_| accepts(&HashMap::new())))
            });
        }
        expression.map(|node| {
            if let Some(captures) = PatternNode::match_ordered(&self.pattern, node)
                && accepts(&captures)
            {
                return Ok(Some(ReplacementOp::instantiate(
                    &self.replacement,
                    &captures,
                )));
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
                    return Ok(Some(ReplacementOp::instantiate(
                        &self.replacement,
                        &matched.captures,
                    )));
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
                    return Ok(Some(ReplacementOp::instantiate(
                        &self.replacement,
                        &matched.captures,
                    )));
                }
            }
            Ok(None)
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
            inserted.push(ReplacementOp::instantiate(
                &self.replacement,
                &matched.captures,
            ));
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
            PatternNode::Pow(_, exponent)
                if matches!(exponent.node(), PatternNode::Exact(value)
                    if AlgebraicOperands::integer(value).is_some_and(|power| power > 0)) =>
            {
                (false, one(&self.pattern))
            }
            PatternNode::Exact(_)
            | PatternNode::Capture(_)
            | PatternNode::Function { .. }
            | PatternNode::Div(..)
            | PatternNode::Pow(..) => return Ok(None),
        };
        if patterns.is_empty()
            || !sum && !matches!(node.view(), ExprView::Product(_) | ExprView::Pow(..))
        {
            return Ok(None);
        }
        let Some(mut operands) = AlgebraicOperands::new(node, sum, 0) else {
            return Ok(None);
        };
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
            let replacement = ReplacementOp::instantiate(&self.replacement, &matched.captures);
            inserted.push(AlgebraicOperands::repeat(replacement, repetitions, sum));
        }
        Ok((!inserted.is_empty()).then(|| operands.finish(inserted)))
    }
}
