//! The ordered and counted matching stages of one rule.
//!
//! Both walk the operands of a rebuilt node: the ordered stage selects whole
//! original operands, and the counted stage matches compressed arithmetic
//! contributions and preserves their remainder.

use core::{num::NonZeroU64, slice::from_ref};

use alloc::vec::Vec;

use crate::collections::HashMap;

use super::{
    AlgebraicOperands, Expr, Group, GroupValues, Heads, MatchWork, NodeShape, Pattern, PatternNode,
    RewriteError, RewriteRule, RewriteTarget, Search, Selected, Taken, Task,
};

impl RewriteTarget<'_> {
    /// The replacement of one rebuilt node under a template or callback
    /// destination: an ordered match, then commutative searches over explicit
    /// operands, nested groups, counted units, and inferred coefficients.
    ///
    /// # Errors
    /// Returns [`RewriteError::MatchLimitExceeded`] when a search needs another
    /// attempt after the pass budget.
    pub fn rewrite_pattern<'rule>(
        &mut self,
        rule: &'rule RewriteRule,
        shape: NodeShape<'_>,
        work: &mut MatchWork<'rule>,
        accepts: &mut impl FnMut(&[Option<Expr>], &mut Option<HashMap<&'rule str, Expr>>) -> bool,
    ) -> Result<Option<Expr>, RewriteError> {
        let node = self.node();
        if !rule.heads.accepts(shape.heads)
            || rule
                .root_function
                .as_ref()
                .is_some_and(|expected| shape.function != Some(expected))
        {
            return Ok(None);
        }
        // Only a sum or product pattern matching its own head reads the node's
        // operand list, so non-matching heads never project compact sums.
        let needs_projection = match rule.pattern.node() {
            PatternNode::Sum(_) => shape.heads == Heads::SUM,
            PatternNode::Product(_) => shape.heads == Heads::PRODUCT,
            PatternNode::Capture { .. }
            | PatternNode::Exact(_)
            | PatternNode::Function { .. }
            | PatternNode::Div(..)
            | PatternNode::Pow(..) => false,
        };
        let projection = if needs_projection {
            self.projection()
        } else {
            None
        };
        if PatternNode::match_ordered(
            &rule.pattern,
            node,
            projection,
            &rule.capture_names,
            &rule.repeats,
            work,
        ) && accepts(&work.slots, &mut work.bindings)
        {
            return Ok(rule.replacement.instantiate(
                &work.slots,
                &rule.capture_names,
                &mut work.values,
                &mut work.bindings,
            ));
        }
        if let PatternNode::Sum(patterns) | PatternNode::Product(patterns) = rule.pattern.node() {
            let sum = matches!(rule.pattern.node(), PatternNode::Sum(_));
            if let Some(replacement) =
                Self::rewrite_explicit(rule, patterns, sum, projection, work, accepts)?
            {
                return Ok(Some(replacement));
            }
        }
        if rule.associative
            && !matches!(
                rule.pattern.node(),
                PatternNode::Sum(_) | PatternNode::Product(_)
            )
        {
            let initial = Task::Match(&rule.pattern, node.clone());
            work.open(0);
            if Search::new(&rule.capture_names, &rule.grounds, None, &rule.repeats)
                .run(initial, work, accepts)?
                .is_some()
            {
                return Ok(rule.replacement.instantiate(
                    &work.slots,
                    &rule.capture_names,
                    &mut work.values,
                    &mut work.bindings,
                ));
            }
        }
        if let Some(replacement) = self.rewrite_counted(rule, shape, work, accepts)? {
            return Ok(Some(replacement));
        }
        if rule.inferred_coefficient {
            let initial = Task::Match(&rule.pattern, node.clone());
            work.open(0);
            if Search::new(&rule.capture_names, &rule.grounds, None, &rule.repeats)
                .run(initial, work, accepts)?
                .is_some()
            {
                return Ok(rule.replacement.instantiate(
                    &work.slots,
                    &rule.capture_names,
                    &mut work.values,
                    &mut work.bindings,
                ));
            }
        }
        Ok(None)
    }

    /// Select whole original operands before searching their remaining counted units.
    ///
    /// # Errors
    /// Returns [`RewriteError::MatchLimitExceeded`] when a search needs another
    /// attempt after the pass budget.
    fn rewrite_explicit<'rule>(
        rule: &'rule RewriteRule,
        patterns: &[Pattern],
        sum: bool,
        projection: Option<&[Expr]>,
        work: &mut MatchWork<'rule>,
        accepts: &mut impl FnMut(&[Option<Expr>], &mut Option<HashMap<&'rule str, Expr>>) -> bool,
    ) -> Result<Option<Expr>, RewriteError> {
        if patterns.is_empty() {
            return Ok(None);
        }
        let Some(operands) = projection else {
            return Ok(None);
        };
        // Operands consumed by earlier matches start selected, so later
        // searches skip them and report them again as a prefix.
        let mut consumed = Taken::default();
        let mut start = 0;
        let mut inserted = Vec::new();
        let mut search = Search::new(&rule.capture_names, &rule.grounds, None, &rule.repeats);
        let scan = operands.len().saturating_mul(rule.nodes);
        while operands.len().saturating_sub(consumed.len()) >= patterns.len() {
            let initial = Task::Group(Group {
                patterns,
                values: GroupValues::Borrowed(operands),
                selected: Selected::Explicit(consumed.clone()),
                next: start,
                projection: 0,
                combination: None,
                sum,
                start,
                root: true,
            });
            work.open(scan);
            let Some(selected) = search.run(initial, work, accepts)? else {
                break;
            };
            // Nonempty groups select distinct available indices, so every match progresses.
            let Selected::Explicit(indices) = selected else {
                return Ok(None);
            };
            consumed = indices;
            start = consumed.first_free(start).min(operands.len());
            let Some(replacement) = rule.replacement.instantiate(
                &work.slots,
                &rule.capture_names,
                &mut work.values,
                &mut work.bindings,
            ) else {
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
            .filter(|(index, _)| !consumed.contains(*index))
            .map(|(_, operand)| operand.clone())
            .collect();
        let remainder = if sum {
            Expr::sum(terms)
        } else {
            Expr::product(terms)
        };
        let transformed = RewriteTarget::new(&remainder).rewrite_counted(
            rule,
            NodeShape::of(&remainder),
            work,
            accepts,
        )?;
        inserted.push(transformed.unwrap_or(remainder));
        Ok(Some(if sum {
            Expr::sum(inserted)
        } else {
            Expr::product(inserted)
        }))
    }

    /// Match compressed original arithmetic contributions and preserve their remainder.
    ///
    /// # Errors
    /// Returns [`RewriteError::MatchLimitExceeded`] when a search needs another
    /// attempt after the pass budget.
    fn rewrite_counted<'rule>(
        &mut self,
        rule: &'rule RewriteRule,
        shape: NodeShape<'_>,
        work: &mut MatchWork<'rule>,
        accepts: &mut impl FnMut(&[Option<Expr>], &mut Option<HashMap<&'rule str, Expr>>) -> bool,
    ) -> Result<Option<Expr>, RewriteError> {
        let Some(denominator) = rule.counted else {
            return Ok(None);
        };
        // A counted root other than a sum or product is a power template.
        let (sum, patterns) = match rule.pattern.node() {
            PatternNode::Sum(patterns) => (true, patterns.as_slice()),
            PatternNode::Product(patterns) => (false, patterns.as_slice()),
            PatternNode::Exact(_)
            | PatternNode::Capture { .. }
            | PatternNode::Function { .. }
            | PatternNode::Div(..)
            | PatternNode::Pow(..) => (false, from_ref(&rule.pattern)),
        };
        let supported = if sum {
            shape.heads == Heads::SUM
        } else {
            Heads::COUNTED.accepts(shape.heads)
        };
        if patterns.is_empty() || !supported {
            return Ok(None);
        }
        let Some((shared, heads)) = self.counted_operands(sum) else {
            return Ok(None);
        };
        // `self` already refined multiplicative operands at denominator one,
        // so only fractional-step rules clone and re-refine before a match.
        let mut owned = (!sum && denominator != NonZeroU64::MIN).then(|| {
            let mut refined = shared.clone();
            refined.refine_powers(denominator);
            refined
        });
        // The scan a successful match must make is fixed for the whole call:
        // neither the operand count nor the template size changes.
        let scan = shared.len().saturating_mul(rule.nodes);
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
            work.open(scan);
            let Some(matched) =
                Search::new(&rule.capture_names, &rule.grounds, heads, &rule.repeats)
                    .run(initial, work, accepts)?
            else {
                break;
            };
            let Selected::Counted(selected) = matched else {
                return Ok(None);
            };
            let repetitions = owned
                .get_or_insert_with(|| shared.clone())
                .consume(&selected);
            let Some(replacement) = rule.replacement.instantiate(
                &work.slots,
                &rule.capture_names,
                &mut work.values,
                &mut work.bindings,
            ) else {
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
