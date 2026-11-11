//! Several rules applied in one bottom-up pass and single-pass matching stages.
//!
//! Each node's value carries the builtins called below it, the union of its
//! own head and its children's sets, so a rule whose pattern calls a missing
//! builtin is skipped with one mask test. A replacement's set is bounded by
//! the matched node's set and the builtins its destination calls, since its
//! captures are taken from the matched subtree. A builtin that a constructor
//! alias introduces, such as `exp` for a power of `e`, is seen by the next
//! pass.

use core::{borrow::Borrow, iter::once, slice::from_ref};

use alloc::{borrow::Cow, vec, vec::Vec};

use crate::collections::HashMap;

use super::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, BuiltinSet, ConcreteMatcher,
    ExactSources, Expr, ExprView, Group, GroupValues, Heads, Mapped, MatchWork, PassMatching,
    Pattern, PatternNode, Postorder, ReplacementOp, ReplacementProgram, RewriteError, RewriteRule,
    RewriteTarget, Rewritten, RuleEntry, RulePass, RuleSet, Search, Selected, Substitution, Task,
};

impl RuleSet {
    /// Rules tried in the given order at every node.
    pub fn new(templates: impl IntoIterator<Item = RewriteRule>) -> Self {
        let rules: Vec<_> = templates
            .into_iter()
            .map(|rule| RuleEntry {
                required: rule
                    .required_functions()
                    .filter_map(|name| BuiltinFunction::of(&name))
                    .fold(BuiltinSet::default(), BuiltinSet::with),
                introduced: Self::introduced(&rule.replacement),
                rule,
            })
            .collect();
        Self {
            calls_builtins: rules
                .iter()
                .all(|entry| entry.required != BuiltinSet::default()),
            rules,
        }
    }

    /// Apply the rules in one bottom-up pass over the logical DAG.
    ///
    /// At each rebuilt node the rules are tried in order, and the first
    /// replacement is kept and not matched again in this pass. A rule is tried
    /// only where every builtin its pattern calls occurs below the node, so
    /// when every rule calls one, compact storage in symbols and constants,
    /// whose terms call none, is not projected. A rule
    /// that needs more than `match_limit` attempts contributes nothing: the
    /// pass restarts without it, so the result never depends on where its
    /// budget ran out.
    #[must_use]
    pub fn apply(&self, expression: &Expr, match_limit: usize) -> Expr {
        let mut enabled = vec![true; self.rules.len()];
        loop {
            match self.pass(expression, match_limit, &enabled) {
                Ok(result) => return result,
                // The exhausted rule was enabled, so each restart disables one
                // more rule and at most `rules.len()` restarts occur.
                Err(exhausted) => {
                    if let Some(flag) = enabled.get_mut(exhausted) {
                        *flag = false;
                    }
                }
            }
        }
    }

    /// One pass with the enabled rules, or the index of a rule whose attempt
    /// budget ran out.
    fn pass(&self, expression: &Expr, match_limit: usize, enabled: &[bool]) -> Result<Expr, usize> {
        let mut runs = self
            .rules
            .iter()
            .zip(enabled)
            .enumerate()
            .filter(|(_, (_, flag))| **flag)
            .map(|(index, (entry, _))| (index, entry, RulePass::new(&entry.rule, match_limit)))
            .collect::<Vec<_>>();
        if runs.is_empty() {
            return Ok(expression.clone());
        }
        let mut accepts = |_: &HashMap<&str, Expr>| true;
        let order = if self.calls_builtins {
            Postorder::compact(expression)
        } else {
            Postorder::logical(expression)
        };
        let root = order.evaluate(|node: &Mapped<'_, '_, Rewritten>| {
            let rebuilt = node.rebuild();
            let below = node
                .values()
                .fold(BuiltinSet::head(&rebuilt), |set, child| {
                    set.union(child.builtins)
                });
            let mut target = RewriteTarget::new(&rebuilt);
            for (index, entry, run) in &mut runs {
                if !below.contains_all(entry.required) {
                    continue;
                }
                match run.rewrite(&mut target, &mut accepts) {
                    Ok(Some(replacement)) => {
                        return Ok(Rewritten {
                            expression: replacement,
                            builtins: below.union(entry.introduced),
                        });
                    }
                    Ok(None) => {}
                    Err(_) => return Err(*index),
                }
            }
            drop(target);
            Ok(Rewritten {
                expression: rebuilt,
                builtins: below,
            })
        })?;
        Ok(root.expression)
    }

    /// A bound on the builtins a replacement calls beyond its captures.
    fn introduced(program: &ReplacementProgram) -> BuiltinSet {
        let calls = |expression: &Expr| {
            expression.fold(BuiltinSet::default(), |set, node| {
                set.union(BuiltinSet::head(node))
            })
        };
        match program {
            ReplacementProgram::Template(template) => template
                .operands
                .iter()
                .chain(once(&template.root))
                .fold(BuiltinSet::default(), |set, operation| match operation {
                    ReplacementOp::Function { name, .. } => {
                        BuiltinFunction::of(name).map_or(set, |builtin| set.with(builtin))
                    }
                    ReplacementOp::Exact(value) => set.union(calls(value)),
                    ReplacementOp::Capture(_)
                    | ReplacementOp::Sum(_)
                    | ReplacementOp::Product(_)
                    | ReplacementOp::Div(..)
                    | ReplacementOp::Pow(..) => set,
                }),
            ReplacementProgram::Concrete(entry) => calls(entry.to()),
            ReplacementProgram::Callback(_) | ReplacementProgram::ConcreteCallback { .. } => {
                BuiltinSet::ALL
            }
        }
    }
}

impl Borrow<Expr> for Rewritten {
    fn borrow(&self) -> &Expr {
        &self.expression
    }
}

impl<'node> RewriteTarget<'node> {
    /// A target whose projections are not yet built.
    pub const fn new(node: &'node Expr) -> Self {
        Self {
            node,
            operands: None,
        }
    }
}

impl<'rule> RulePass<'rule> {
    /// A fresh budget of `match_limit` attempts for one pass of `rule`.
    pub fn new(rule: &'rule RewriteRule, match_limit: usize) -> Self {
        let matching = match &rule.replacement {
            ReplacementProgram::Concrete(entry) => PassMatching::Concrete {
                source: entry.from(),
                matcher: ConcreteMatcher::new(ExactSources::One(entry.clone())),
            },
            ReplacementProgram::ConcreteCallback { source, callback } => PassMatching::Callback {
                source,
                callback: callback.as_ref(),
                probe: ConcreteMatcher::new(ExactSources::One(Substitution::new(
                    source.clone(),
                    source.clone(),
                ))),
                resolved: None,
            },
            ReplacementProgram::Template(_) | ReplacementProgram::Callback(_) => {
                PassMatching::Pattern
            }
        };
        Self {
            rule,
            work: MatchWork {
                remaining: match_limit,
                limit: match_limit,
            },
            matching,
            captures: HashMap::new(),
        }
    }

    /// The replacement of one rebuilt node, or `None` to keep it.
    ///
    /// # Errors
    /// Returns [`RewriteError::MatchLimitExceeded`] when this pass needs
    /// another attempt after its budget.
    pub fn rewrite(
        &mut self,
        target: &mut RewriteTarget<'_>,
        accepts: &mut impl FnMut(&HashMap<&str, Expr>) -> bool,
    ) -> Result<Option<Expr>, RewriteError> {
        let node = target.node;
        let (replacement, source) = match &self.matching {
            PassMatching::Pattern => {
                return self.rule.rewrite_pattern(target, &mut self.work, accepts);
            }
            PassMatching::Concrete { source, matcher } => (matcher.replace(node), *source),
            PassMatching::Callback {
                source,
                probe,
                resolved,
                ..
            } => (resolved.as_ref().unwrap_or(probe).replace(node), *source),
        };
        if replacement.is_some() && node != source {
            self.work.remaining =
                self.work
                    .remaining
                    .checked_sub(1)
                    .ok_or(RewriteError::MatchLimitExceeded {
                        limit: self.work.limit,
                    })?;
        }
        if replacement.is_none() || !accepts(&self.captures) {
            return Ok(None);
        }
        if let PassMatching::Callback {
            callback,
            resolved: resolved @ None,
            ..
        } = &mut self.matching
        {
            let entry = Substitution::new(source.clone(), callback(&self.captures));
            return Ok(resolved
                .insert(ConcreteMatcher::new(ExactSources::One(entry)))
                .replace(node));
        }
        Ok(replacement)
    }
}

impl RewriteRule {
    /// The replacement of one rebuilt node under a template or callback
    /// destination: an ordered match, then commutative searches over explicit
    /// operands, nested groups, counted units, and inferred coefficients.
    fn rewrite_pattern(
        &self,
        target: &mut RewriteTarget<'_>,
        work: &mut MatchWork,
        accepts: &mut impl FnMut(&HashMap<&str, Expr>) -> bool,
    ) -> Result<Option<Expr>, RewriteError> {
        let node = target.node;
        let head = match node.view() {
            ExprView::Sum(_) => Heads::SUM,
            ExprView::Product(_) => Heads::PRODUCT,
            ExprView::Div(..) => Heads::DIV,
            ExprView::Pow(..) => Heads::POW,
            ExprView::Function { name, .. } if *name == BuiltinFunction::Sqrt.function() => {
                Heads::SQRT
            }
            ExprView::Function { name, .. } if matches!(self.pattern.node(), PatternNode::Function { name: root, .. } if root == name) => {
                Heads::CALL
            }
            ExprView::Function { .. }
            | ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Derivative { .. } => Heads::OTHER,
        };
        if self.heads.0 & head.0 == 0 {
            return Ok(None);
        }
        if let Some(captures) = PatternNode::match_ordered(&self.pattern, node)
            && accepts(&captures)
        {
            return Ok(self.replacement.instantiate(&captures));
        }
        if let PatternNode::Sum(patterns) | PatternNode::Product(patterns) = self.pattern.node() {
            let sum = matches!(self.pattern.node(), PatternNode::Sum(_));
            if let Some(replacement) = self.rewrite_explicit(patterns, node, sum, work, accepts)? {
                return Ok(Some(replacement));
            }
        }
        if self.associative
            && !matches!(
                self.pattern.node(),
                PatternNode::Sum(_) | PatternNode::Product(_)
            )
        {
            let initial = Task::Match(&self.pattern, node.clone());
            if let Some(matched) = Search::new(&self.grounds).run(initial, work, accepts)? {
                return Ok(self.replacement.instantiate(&matched.captures));
            }
        }
        if let Some(replacement) = self.rewrite_counted(target, work, accepts)? {
            return Ok(Some(replacement));
        }
        if let PatternNode::Product(factors) = self.pattern.node()
            && matches!(
                factors.first().map(Pattern::node),
                Some(PatternNode::Exact(_))
            )
        {
            let initial = Task::Match(&self.pattern, node.clone());
            if let Some(matched) = Search::new(&self.grounds).run(initial, work, accepts)? {
                return Ok(self.replacement.instantiate(&matched.captures));
            }
        }
        Ok(None)
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
        // Operands consumed by earlier matches start selected, so later
        // searches skip them and report them again as a prefix.
        let mut consumed = Vec::new();
        let mut start = 0;
        let mut inserted = Vec::new();
        while operands.len().saturating_sub(consumed.len()) >= patterns.len() {
            let initial = Task::Group(Group {
                patterns,
                values: GroupValues::Borrowed(&operands),
                selected: Selected::Explicit(consumed.clone()),
                next: start,
                projection: 0,
                combination: None,
                sum,
                start,
                root: true,
            });
            let Some(matched) = Search::new(&self.grounds).run(initial, work, accepts)? else {
                break;
            };
            // Nonempty groups select distinct available indices, so every match progresses.
            let Selected::Explicit(indices) = matched.selected else {
                return Ok(None);
            };
            consumed = indices;
            start = (start..operands.len())
                .find(|index| !consumed.contains(index))
                .unwrap_or(operands.len());
            let Some(replacement) = self.replacement.instantiate(&matched.captures) else {
                return Ok(None);
            };
            inserted.push(replacement);
        }
        if inserted.is_empty() {
            return Ok(None);
        }
        let terms = operands
            .iter()
            .enumerate()
            .filter(|(index, _)| !consumed.contains(index))
            .map(|(_, operand)| operand.clone())
            .collect();
        let remainder = if sum {
            Expr::sum(terms)
        } else {
            Expr::product(terms)
        };
        let transformed =
            self.rewrite_counted(&mut RewriteTarget::new(&remainder), work, accepts)?;
        inserted.push(transformed.unwrap_or(remainder));
        Ok(Some(if sum {
            Expr::sum(inserted)
        } else {
            Expr::product(inserted)
        }))
    }

    /// Match compressed original arithmetic contributions and preserve their remainder.
    fn rewrite_counted(
        &self,
        target: &mut RewriteTarget<'_>,
        work: &mut MatchWork,
        accepts: &mut impl FnMut(&HashMap<&str, Expr>) -> bool,
    ) -> Result<Option<Expr>, RewriteError> {
        let Some(denominator) = self.counted else {
            return Ok(None);
        };
        // A counted root other than a sum or product is a power template.
        let (sum, patterns) = match self.pattern.node() {
            PatternNode::Sum(patterns) => (true, patterns.as_slice()),
            PatternNode::Product(patterns) => (false, patterns.as_slice()),
            PatternNode::Exact(_)
            | PatternNode::Capture { .. }
            | PatternNode::Function { .. }
            | PatternNode::Div(..)
            | PatternNode::Pow(..) => (false, from_ref(&self.pattern)),
        };
        let node = target.node;
        // Only a sum has additive operands, so the head decides the projection
        // shared through the target.
        let supported = match node.view() {
            ExprView::Sum(_) => sum,
            ExprView::Product(_) | ExprView::Pow(..) | ExprView::Div(..) => !sum,
            ExprView::Function { name, .. } => !sum && *name == BuiltinFunction::Sqrt.function(),
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Derivative { .. } => false,
        };
        if patterns.is_empty() || !supported {
            return Ok(None);
        }
        if target.operands.is_none() {
            target.operands = AlgebraicOperands::new(node, sum, 0);
        }
        let Some(shared) = &target.operands else {
            return Ok(None);
        };
        // Product powers are refined for this rule's denominator, so a product
        // search owns a copy; a sum search borrows the shared projection until
        // a match consumes part of it.
        let mut owned = (!sum).then(|| {
            let mut refined = shared.clone();
            refined.refine_powers(denominator);
            refined
        });
        let mut inserted = Vec::new();
        loop {
            let initial = Task::Group(Group {
                patterns,
                values: GroupValues::Counted(owned.as_ref().unwrap_or(shared)),
                selected: Selected::Counted(Vec::new()),
                next: 0,
                projection: 0,
                combination: None,
                sum,
                start: 0,
                root: true,
            });
            let Some(matched) = Search::new(&self.grounds).run(initial, work, accepts)? else {
                break;
            };
            let Selected::Counted(selected) = matched.selected else {
                return Ok(None);
            };
            let repetitions = owned
                .get_or_insert_with(|| shared.clone())
                .consume(&selected);
            let Some(replacement) = self.replacement.instantiate(&matched.captures) else {
                return Ok(None);
            };
            inserted.push(AlgebraicOperands::repeat(replacement, repetitions, sum));
        }
        if inserted.is_empty() {
            return Ok(None);
        }
        // Every match consumed operands from the owned copy.
        Ok(owned.map(|operands| operands.finish(inserted)))
    }
}
