//! Several rules applied in one bottom-up pass and per-node target caches.
//!
//! Each node's value carries the builtins called below it, the union of its
//! own head and its children's sets, so a rule whose pattern calls a missing
//! builtin is skipped with one mask test. Rules are bucketed by logical head
//! at construction time, so leaf nodes and mismatched heads skip unrelated
//! rules in one table lookup. A replacement's set is bounded by the matched
//! node's set and the builtins its destination calls, since its captures are
//! taken from the matched subtree. A builtin that a constructor alias
//! introduces, such as `exp` for a power of `e`, is seen by the next pass.

use core::{borrow::Borrow, iter::once, num::NonZeroU64};

use alloc::{borrow::Cow, boxed::Box, vec::Vec};

use crate::collections::HashMap;

use super::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, BuiltinSet, ConcreteMatcher, CountedSlot,
    ExactSources, Expr, ExprView, Heads, Mapped, MatchWork, NodeShape, Postorder, ReplacementFn,
    ReplacementOp, ReplacementProgram, RewriteError, RewriteRule, Substitution,
};

/// The seven single-head bits in bucket order for [`RuleSet::by_head`].
const HEAD_BUCKETS: [Heads; 7] = [
    Heads::SUM,
    Heads::PRODUCT,
    Heads::DIV,
    Heads::POW,
    Heads::SQRT,
    Heads::CALL,
    Heads::OTHER,
];

/// Rules applied together in one bottom-up pass; see [`RuleSet::apply`].
pub struct RuleSet {
    rules: Vec<RuleEntry>,
    /// Indices into `rules` accepting each of the seven heads in
    /// `HEAD_BUCKETS`, preserving rule priority order.
    by_head: [Box<[usize]>; 7],
    /// Whether every pattern calls a builtin. Terms of compact storage in
    /// symbols and constants call none, so such a set never matches them and
    /// its pass numbers that storage as a leaf.
    calls_builtins: bool,
    /// Whether any rule uses capture-free concrete matching, which keeps
    /// per-rule matcher state across a pass.
    has_concrete: bool,
}

/// A rule with the builtins its pattern requires below a matched node and a
/// bound on the builtins its replacement calls.
struct RuleEntry {
    rule: RewriteRule,
    required: BuiltinSet,
    introduced: BuiltinSet,
}

/// A node after a rule pass and the builtins called below it.
struct Rewritten {
    expression: Expr,
    builtins: BuiltinSet,
}

/// A rebuilt node offered to rules and its counted operands, projected on
/// first use and shared by every rule tried at the node. The node's head fixes
/// the projection: additive for a sum, multiplicative for a product, power,
/// quotient, or square root.
#[derive(Debug)]
pub struct RewriteTarget<'node> {
    node: &'node Expr,
    /// The ordered operands of the node, projected on first use and shared by
    /// every rule tried at the node.
    projection: Option<Cow<'node, [Expr]>>,
    operands: Option<AlgebraicOperands>,
    /// One slot per counted operand of a sum, each a superset of the heads the
    /// value that operand projects to can have and of the heads of that
    /// value's factors, built on first use and shared by every rule tried at
    /// the node.
    counted_heads: Option<Vec<CountedSlot>>,
}

/// One rule's state during a pass: its attempt budget and, for a capture-free
/// rule, its concrete matcher.
pub struct RulePass<'rule> {
    rule: &'rule RewriteRule,
    work: MatchWork<'rule>,
    matching: PassMatching<'rule>,
}

enum PassMatching<'rule> {
    Pattern,
    Concrete {
        source: &'rule Expr,
        matcher: ConcreteMatcher<'static>,
    },
    /// The source alone decides matches; the callback supplies the
    /// destination once, on the first accepted match.
    Callback {
        source: &'rule Expr,
        callback: &'rule ReplacementFn,
        probe: ConcreteMatcher<'static>,
        resolved: Option<ConcreteMatcher<'static>>,
    },
}

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
        let mut buckets: [Vec<usize>; 7] = Default::default();
        let mut has_concrete = false;
        for (index, entry) in rules.iter().enumerate() {
            let mask = match entry.rule.replacement {
                ReplacementProgram::Concrete(_) | ReplacementProgram::ConcreteCallback { .. } => {
                    has_concrete = true;
                    Heads::ALL
                }
                ReplacementProgram::Template(_) | ReplacementProgram::Callback(_) => {
                    entry.rule.heads
                }
            };
            for (head, bucket) in HEAD_BUCKETS.iter().zip(&mut buckets) {
                if mask.accepts(*head) {
                    bucket.push(index);
                }
            }
        }
        Self {
            calls_builtins: rules
                .iter()
                .all(|entry| entry.required != BuiltinSet::default()),
            by_head: buckets.map(Vec::into_boxed_slice),
            has_concrete,
            rules,
        }
    }

    /// Apply the rules in one bottom-up pass over the logical DAG.
    ///
    /// At each rebuilt node the rules matching its head are tried in order,
    /// and the first replacement is kept and not matched again in this pass. A
    /// rule is tried only where every builtin its pattern calls occurs below
    /// the node, so when every rule calls one, compact storage in symbols and
    /// constants, whose terms call none, is not projected.
    ///
    /// # Errors
    /// Returns [`RewriteError::MatchLimitExceeded`] when one match attempt
    /// needs more candidate attempts than `match_limit` allows beyond the
    /// linear scan of its operand group. Nothing is published: the caller
    /// receives the error instead of a partially rewritten expression.
    pub fn apply(&self, expression: &Expr, match_limit: usize) -> Result<Expr, RewriteError> {
        if self.rules.is_empty() {
            return Ok(expression.clone());
        }
        // Pattern rules share one `MatchWork` across the pass; only capture-free
        // rules allocate per-rule concrete matcher state.
        let mut work = MatchWork::new(match_limit);
        let mut concrete: Vec<Option<RulePass<'_>>> = if self.has_concrete {
            self.rules
                .iter()
                .map(|entry| {
                    matches!(
                        entry.rule.replacement,
                        ReplacementProgram::Concrete(_)
                            | ReplacementProgram::ConcreteCallback { .. }
                    )
                    .then(|| RulePass::new(&entry.rule, match_limit))
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut accepts = |_: &[Option<Expr>], _: &mut Option<HashMap<&str, Expr>>| true;
        let order = if self.calls_builtins {
            Postorder::compact(expression)
        } else {
            Postorder::logical(expression)
        };
        let root = order.evaluate(|node: &Mapped<'_, '_, Rewritten>| {
            let (rebuilt, _) = node.rebuild();
            let below = node
                .values()
                .fold(BuiltinSet::head(&rebuilt), |set, child| {
                    set.union(child.builtins)
                });
            let shape = NodeShape::of(&rebuilt);
            let head_index = match shape.heads.0 {
                1 => 0,
                2 => 1,
                4 => 2,
                8 => 3,
                16 => 4,
                32 => 5,
                _ => 6,
            };
            let candidates = self.by_head.get(head_index).map_or(&[][..], Box::as_ref);
            if !candidates.is_empty() {
                let mut target = RewriteTarget::new(&rebuilt);
                for &index in candidates {
                    let Some(entry) = self.rules.get(index) else {
                        continue;
                    };
                    if !below.contains_all(entry.required) {
                        continue;
                    }
                    let replaced = if let Some(Some(pass)) = concrete.get_mut(index) {
                        pass.rewrite(&mut target, shape, &mut accepts)?
                    } else {
                        target.rewrite_pattern(&entry.rule, shape, &mut work, &mut accepts)?
                    };
                    if let Some(replacement) = replaced {
                        return Ok(Rewritten {
                            expression: replacement,
                            builtins: below.union(entry.introduced),
                        });
                    }
                }
            }
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
            projection: None,
            operands: None,
            counted_heads: None,
        }
    }

    pub const fn node(&self) -> &'node Expr {
        self.node
    }

    /// The node's operands as the ordered stages see them: additive for a sum,
    /// the factors for a product, and nothing for any other head.
    ///
    /// The head fixes the projection, so one build serves every rule tried at
    /// the node.
    pub fn projection(&mut self) -> Option<&[Expr]> {
        if self.projection.is_none() {
            self.projection = match self.node.view() {
                ExprView::Sum(_) => self.node.additive_operands(),
                ExprView::Product(factors) => Some(Cow::Borrowed(factors)),
                ExprView::Div(..)
                | ExprView::Pow(..)
                | ExprView::Function { .. }
                | ExprView::Number(_)
                | ExprView::ImaginaryUnit
                | ExprView::Pi
                | ExprView::Euler
                | ExprView::Symbol(_)
                | ExprView::Derivative { .. } => None,
            };
        }
        self.projection.as_deref()
    }

    /// Counted operands and (for sums) their per-slot head index, built once
    /// and shared by every rule tried at the node.
    ///
    /// Sums reuse `self.projection` when already materialized so compact
    /// storage is not projected twice. Multiplicative operands are refined at
    /// denominator one on creation so integer-power rules borrow them directly.
    pub fn counted_operands(
        &mut self,
        sum: bool,
    ) -> Option<(&AlgebraicOperands, Option<&[CountedSlot]>)> {
        if self.operands.is_none() {
            self.operands = if sum {
                match self.projection.as_deref() {
                    Some(operands) => AlgebraicOperands::from_operands(operands, true, 0),
                    None => AlgebraicOperands::new(self.node, true, 0),
                }
            } else {
                let mut operands = AlgebraicOperands::new(self.node, false, 0)?;
                operands.refine_powers(NonZeroU64::MIN);
                Some(operands)
            };
        }
        if sum && self.counted_heads.is_none() {
            self.counted_heads = self
                .operands
                .as_ref()
                .map(|operands| operands.originals().map(Heads::counted).collect());
        }
        Some((self.operands.as_ref()?, self.counted_heads.as_deref()))
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
            work: MatchWork::new(match_limit),
            matching,
        }
    }

    /// The replacement of one rebuilt node, or `None` to keep it.
    ///
    /// `shape` is the node's head, read once by the caller and shared by every
    /// rule tried at the node.
    ///
    /// # Errors
    /// Returns [`RewriteError::MatchLimitExceeded`] when this pass needs
    /// another attempt after its budget.
    pub fn rewrite(
        &mut self,
        target: &mut RewriteTarget<'_>,
        shape: NodeShape<'_>,
        accepts: &mut impl FnMut(&[Option<Expr>], &mut Option<HashMap<&'rule str, Expr>>) -> bool,
    ) -> Result<Option<Expr>, RewriteError> {
        let node = target.node;
        let (replacement, source) = match &self.matching {
            PassMatching::Pattern => {
                return target.rewrite_pattern(self.rule, shape, &mut self.work, accepts);
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
        if replacement.is_none() || !accepts(&[], &mut self.work.bindings) {
            return Ok(None);
        }
        if let PassMatching::Callback {
            callback,
            resolved: resolved @ None,
            ..
        } = &mut self.matching
        {
            let map = self.work.bindings.get_or_insert_with(HashMap::new);
            map.clear();
            let entry = Substitution::new(source.clone(), callback(map));
            return Ok(resolved
                .insert(ConcreteMatcher::new(ExactSources::One(entry)))
                .replace(node));
        }
        Ok(replacement)
    }
}
