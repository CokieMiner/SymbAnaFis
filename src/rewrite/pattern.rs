//! Captured structural patterns and explicit, validated rewrite rules.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]
#![expect(
    unsafe_code,
    reason = "a released handle's field is moved out exactly once, so no extra reference count is touched"
)]

use core::{mem::ManuallyDrop, num::NonZeroU64};

use alloc::{boxed::Box, sync::Arc, vec::Vec};

use crate::{
    collections::{FixedSet, HashMap},
    core::{
        AdditiveOperands, AlgebraicOperands, BuiltinFunction, BuiltinSet, ConcreteMatcher,
        ExactSources, Expr, ExprView, Function, FunctionDefinition, Mapped, OperandDemand,
        OperandMatch, Postorder, Substitution, Symbol, UserFunction,
    },
    errors::{FunctionError, RewriteError},
};

mod constructors;
mod matching;
mod normalize;
mod rules;
mod template;

pub use matching::{
    CountedSlot, Group, GroupValues, Heads, NodeShape, Search, Selected, Taken, Task,
};
pub use normalize::Normalization;
pub use rules::RuleSet;
pub use template::{ReplacementOp, Template, TemplateOrder, TemplateVisit};

/// A structural template with named captures and exact subexpressions.
///
/// Arithmetic operators and constructors normalize templates as expressions:
/// sums and products flatten, constants fold, and repeated captures collect.
/// Sum and product operands match in any order. At a rule's root they match a
/// subset and keep the unmatched operands; nested in another template they
/// must cover the whole operand list. A capture may bind a nonempty group of
/// operands, repeated names must bind the same expression, and capture
/// predicates reject a candidate before the assignment continues.
///
/// Implicit groups (counted powers, collected addends, numeric coefficients,
/// and exact rational powers) follow the rules of [`RewriteRule::apply_all`].
///
/// ```
/// use symb_anafis::{Context, Pattern, RewriteRule};
/// let context = Context::new()?;
/// let u = context.symb("u")?;
/// let x = context.symb("x")?;
/// let a = Pattern::capture("a", None);
/// let rule = RewriteRule::new(u + &a, 2 * &a)?;
/// assert_eq!(rule.apply_all(&(u + x), usize::MAX, None)?, 2 * x);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
///
/// `Debug` shows only the root shape, so formatting a deep pattern uses
/// constant stack depth.
#[derive(Clone)]
pub struct Pattern(ManuallyDrop<Arc<PatternNode>>);

enum PatternNode {
    Capture {
        name: Arc<str>,
        predicates: Box<[Arc<CapturePredicate>]>,
    },
    Exact(Expr),
    Function {
        name: Function,
        args: Vec<Pattern>,
    },
    Sum(Vec<Pattern>),
    Product(Vec<Pattern>),
    Div(Pattern, Pattern),
    Pow(Pattern, Pattern),
}

enum Arithmetic {
    Sum(Vec<Pattern>),
    Sub(Pattern, Pattern),
    Product(Vec<Pattern>),
    Div(Pattern, Pattern),
    Pow(Pattern, Pattern),
}

/// The destination of a rewrite rule.
///
/// A pattern destination is validated and compiled when the rule is built. A
/// callback receives the accepted bindings and returns an expression. It must
/// be pure and return the same result for the same bindings, because counted
/// matches reuse one result and capture-free callbacks run at most once per
/// pass. Neither form is checked for mathematical equivalence.
#[derive(Clone)]
#[non_exhaustive]
pub enum Replacement {
    /// A template whose captures must occur in the input pattern.
    Pattern(Pattern),
    /// A constructor run after matching and guard acceptance; its panics
    /// propagate.
    Callback(Arc<ReplacementFn>),
}

/// A validated rule applied once to every distinct node of an input DAG.
///
/// ```
/// use symb_anafis::{Context, Pattern, RewriteRule, UserFunction};
/// let context = Context::new()?;
/// let pair = context.register_function("pair", UserFunction::new(2..=2))?;
/// let argument = Pattern::capture("argument", None);
/// let rule = RewriteRule::new(argument.sin(), argument.cos())?;
/// let x = context.symb("x")?;
/// let input = pair.call(vec![x.sin(), x.sin()])?;
/// assert_eq!(rule.apply_all(&input, usize::MAX, None)?, pair.call(vec![x.cos(), x.cos()])?);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
///
/// `Debug` shows the pattern's root shape and the number of compiled
/// replacement operations.
#[derive(Clone)]
pub struct RewriteRule {
    pattern: Pattern,
    replacement: ReplacementProgram,
    /// Distinct capture names of `pattern`, indexing `MatchWork::slots`.
    capture_names: Box<[Arc<str>]>,
    associative: bool,
    /// Whether the root product can infer a missing finite nonzero scalar
    /// prefix by dividing the matched node and binding the reciprocal.
    inferred_coefficient: bool,
    heads: Heads,
    /// The function a call-rooted pattern accepts, and `None` for every other
    /// root, so a node outside it is rejected without a stage running.
    root_function: Option<Function>,
    /// The unit denominator of the counted stage: the lcm of the exact power
    /// denominators among a product's factors, one for a sum. `None` when the
    /// root has no counted stage or the lcm overflows.
    counted: Option<NonZeroU64>,
    /// The rigid operand templates of the pattern's sums and products, keyed by
    /// template node address and compiled once; see `Search::ground`.
    grounds: Box<[(usize, Template)]>,
    /// The distinct template nodes, which bound the attempts one operand costs.
    nodes: usize,
    /// The addresses of the template nodes whose expansions can repeat.
    repeats: FixedSet<usize>,
}

/// A caller's condition on the captures of a structural match; `false`
/// rejects the assignment.
type RewriteGuard<'guard> = dyn FnMut(&HashMap<&str, Expr>) -> bool + 'guard;

/// A pure, thread-safe condition on a capture candidate. It must give the same
/// answer for the same expression, since the search may evaluate it more than
/// once.
type CapturePredicate = dyn Fn(&Expr) -> bool + Send + Sync;

/// A pure expression constructor over accepted capture bindings.
type ReplacementFn = dyn Fn(&HashMap<&str, Expr>) -> Expr + Send + Sync;

#[derive(Clone)]
enum ReplacementProgram {
    Template(Template),
    Callback(Arc<ReplacementFn>),
    Concrete(Substitution),
    ConcreteCallback {
        source: Expr,
        callback: Arc<ReplacementFn>,
    },
}

/// The attempts left in one match attempt, its bound, and its reusable scratch
/// space shared across rules in a pass.
#[derive(Clone, Debug)]
pub struct MatchWork<'rule> {
    remaining: usize,
    limit: usize,
    /// Values of the numbered destination instructions, reused by the pass.
    values: Vec<Expr>,
    /// Capture bindings indexed by `RewriteRule::capture_names`, reused across
    /// match attempts without hashing.
    slots: Vec<Option<Expr>>,
    /// Named capture map allocated lazily only when a guard or callback runs,
    /// reused across matches without re-seeding or reallocating.
    bindings: Option<HashMap<&'rule str, Expr>>,
    /// Operands appended by nested sum/product expansions during a search.
    projected: Vec<Expr>,
    /// Memoized `(template, input)` expansions when `RewriteRule::repeats` is
    /// nonempty, reused across match attempts.
    expanded: FixedSet<(*const PatternNode, Expr)>,
}

impl MatchWork<'_> {
    pub fn new(limit: usize) -> Self {
        Self {
            remaining: limit,
            limit,
            values: Vec::new(),
            slots: Vec::new(),
            bindings: None,
            projected: Vec::new(),
            expanded: FixedSet::default(),
        }
    }

    /// Open the budget for one match attempt whose successful path must make
    /// `scan` candidate attempts.
    ///
    /// The bound guards the backtracking inside one attempt, not the linear
    /// work a successful match needs. Assigning a template of $q$ nodes over a
    /// group of $n$ operands costs at most $qn$ attempts, because a search
    /// expands each node at most once per candidate input, so a group attempt
    /// passes $qn$ and a single node passes zero. The attempt budget is the
    /// configured bound plus that unavoidable work. A pass-wide budget instead
    /// makes a rule dead on any sum wider than the bound.
    ///
    /// A zero bound stays zero: it disables matching rather than granting the
    /// scan, so a caller can still ask for no candidate attempts at all.
    const fn open(&mut self, scan: usize) {
        self.remaining = if self.limit == 0 {
            0
        } else {
            self.limit.saturating_add(scan)
        };
    }
}

impl Pattern {
    fn from_node(node: PatternNode) -> Self {
        Self(ManuallyDrop::new(Arc::new(node)))
    }

    #[inline]
    fn node(&self) -> &PatternNode {
        &self.0
    }
}

/// The final owner of a template moves it out of its `Arc` and releases its
/// children from an explicit stack, so dropping a deep template uses constant
/// call depth without allocating for leaf patterns.
impl Drop for Pattern {
    fn drop(&mut self) {
        // SAFETY: `drop` runs at most once per handle, and `self.0` is never
        // read again after this move.
        let owner = unsafe { ManuallyDrop::take(&mut self.0) };
        let Some(mut current) = Arc::into_inner(owner) else {
            return;
        };
        let mut pending = Vec::new();
        loop {
            let mut orphan = |child: Self| {
                let mut handle = ManuallyDrop::new(child);
                // SAFETY: `handle` is never dropped or used again, so this is
                // the only move of its field.
                let inner = unsafe { ManuallyDrop::take(&mut handle.0) };
                if let Some(orphaned) = Arc::into_inner(inner) {
                    pending.push(orphaned);
                }
            };
            match current {
                PatternNode::Capture { .. } | PatternNode::Exact(_) => {}
                PatternNode::Function { args, .. }
                | PatternNode::Sum(args)
                | PatternNode::Product(args) => args.into_iter().for_each(orphan),
                PatternNode::Div(left, right) | PatternNode::Pow(left, right) => {
                    orphan(left);
                    orphan(right);
                }
            }
            let Some(next) = pending.pop() else {
                break;
            };
            current = next;
        }
    }
}

#[cfg(test)]
mod tests;
