//! Captured structural patterns and explicit, validated rewrite rules.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]
#![expect(
    unsafe_code,
    reason = "a released handle's field is moved out exactly once, so no extra reference count is touched"
)]

use core::{mem::ManuallyDrop, num::NonZeroU64, ops::Range};

use alloc::{boxed::Box, sync::Arc, vec::Vec};

use crate::{
    collections::{FixedMap, FixedSet, HashMap},
    core::{
        AdditiveOperands, AlgebraicOperands, BuiltinFunction, BuiltinSet, ConcreteMatcher,
        ExactIdentity, ExactSources, Expr, ExprView, Function, FunctionDefinition, Mapped,
        OperandDemand, OperandMatch, Postorder, Substitution, Symbol, UserFunction,
    },
    errors::{FunctionError, RewriteError},
};

mod constructors;
mod matching;
mod normalize;
mod rule_set;
mod rules;
mod search;
mod template;

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

#[derive(Default)]
struct Normalization {
    captures: FixedMap<Arc<str>, (Expr, Vec<Arc<CapturePredicate>>)>,
    markers: FixedMap<Function, Arc<str>>,
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
    associative: bool,
    heads: Heads,
    /// The unit denominator of the counted stage: the lcm of the exact power
    /// denominators among a product's factors, one for a sum. `None` when the
    /// root has no counted stage or the lcm overflows.
    counted: Option<NonZeroU64>,
    /// The rigid operand templates of the pattern's sums and products, each
    /// compiled once; see `Search::ground`.
    grounds: Box<[(Pattern, ReplacementProgram)]>,
}

/// Logical node heads, one bit each, at which a rule's pattern can match.
///
/// Every matching stage fails at a node outside the set before it counts an
/// attempt or calls a guard, so testing the set first changes no result.
#[derive(Clone, Copy, Debug)]
struct Heads(u8);

/// Rules applied together in one bottom-up pass; see [`RuleSet::apply`].
pub struct RuleSet {
    rules: Vec<RuleEntry>,
    /// Whether every pattern calls a builtin. Terms of compact storage in
    /// symbols and constants call none, so such a set never matches them and
    /// its pass numbers that storage as a leaf.
    calls_builtins: bool,
}

/// A rule with the builtins its pattern requires below a matched node and a
/// bound on the builtins its replacement calls.
struct RuleEntry {
    rule: RewriteRule,
    required: BuiltinSet,
    introduced: BuiltinSet,
}

/// One rule's state during a pass: its attempt budget and, for a capture-free
/// rule, its concrete matcher.
struct RulePass<'rule> {
    rule: &'rule RewriteRule,
    work: MatchWork,
    matching: PassMatching<'rule>,
    /// The empty bindings that capture-free rules pass to the guard.
    captures: HashMap<&'rule str, Expr>,
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

/// A rebuilt node offered to rules and its counted operands, projected on
/// first use and shared by every rule tried at the node. The node's head fixes
/// the projection: additive for a sum, multiplicative for a product, power,
/// quotient, or square root.
struct RewriteTarget<'node> {
    node: &'node Expr,
    operands: Option<AlgebraicOperands>,
}

/// A node after a rule pass and the builtins called below it.
struct Rewritten {
    expression: Expr,
    builtins: BuiltinSet,
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

/// The candidate attempts left in one pass, and the pass's whole budget.
struct MatchWork {
    remaining: usize,
    limit: usize,
}

#[derive(Debug)]
enum MatchChildren<'pattern, 'expr> {
    Done,
    Many(&'pattern [Pattern], &'expr [Expr]),
    Pair(
        (&'pattern Pattern, &'expr Expr),
        (&'pattern Pattern, &'expr Expr),
    ),
    Projected(&'pattern [Pattern], Vec<Expr>),
}

#[derive(Debug)]
struct MatchResult<'pattern> {
    captures: HashMap<&'pattern str, Expr>,
    selected: Selected,
}

#[derive(Clone, Debug)]
enum Selected {
    Explicit(Vec<usize>),
    Counted(Vec<OperandMatch>),
}

/// A borrowed `(template, input)` key of an expansion set; a lookup with it
/// takes no reference count.
#[derive(Hash)]
struct Expansion<'input>(*const PatternNode, &'input Expr);

struct Search<'pattern, 'input> {
    pending: Vec<Task<'pattern, 'input>>,
    /// Tasks popped under an open choice, restored in reverse by `Undo::Popped`.
    popped: Vec<Task<'pattern, 'input>>,
    /// Operands of nested sum and product templates. Groups name them by
    /// range; backtracking truncates what the undone expansions appended.
    projected: Vec<Expr>,
    captured: HashMap<&'pattern str, Expr>,
    expanded: FixedSet<(*const PatternNode, Expr)>,
    undo: Vec<Undo<'pattern>>,
    choices: Vec<Choice<'pattern, 'input>>,
    /// The rule's compiled rigid operand templates.
    grounds: &'pattern [(Pattern, ReplacementProgram)],
}

#[derive(Clone, Debug)]
enum Task<'pattern, 'input> {
    Match(&'pattern Pattern, Expr),
    Group(Group<'pattern, 'input>),
}

#[derive(Clone, Debug)]
struct Group<'pattern, 'input> {
    patterns: &'pattern [Pattern],
    values: GroupValues<'input>,
    selected: Selected,
    next: usize,
    projection: i64,
    /// Ranks of the next grouped partition; boxed so a choice is at most
    /// 128 bytes and moves without a `memcpy` call.
    combination: Option<Box<[usize]>>,
    sum: bool,
    start: usize,
    root: bool,
}

enum Assignment<'pattern> {
    Explicit(usize),
    Group(Vec<usize>),
    Counted(OperandMatch),
    Projected {
        operand: OperandMatch,
        base: &'pattern Pattern,
    },
}

/// How a template draws operands from a group, fixed while one assignment
/// tries its candidates.
struct Draw<'pattern> {
    power: Option<PowerTemplate<'pattern>>,
    basic: OperandDemand,
    /// Whether one operand can supply several candidates in turn.
    multiple: bool,
}

struct PowerTemplate<'pattern> {
    base: &'pattern Pattern,
    numerator: i64,
    denominator: NonZeroU64,
    inverse: bool,
}

#[derive(Debug)]
enum Undo<'pattern> {
    Capture(&'pattern str),
    Expanded(*const PatternNode, Expr),
    Pushed,
    /// Restore the last task of `Search::popped`.
    Popped,
    /// Truncate `Search::projected` to this length.
    Projected(usize),
}

#[derive(Debug)]
struct Choice<'pattern, 'input> {
    checkpoint: usize,
    retry: Group<'pattern, 'input>,
}

#[derive(Clone, Debug)]
enum GroupValues<'input> {
    /// Original operands; those consumed by earlier matches start selected.
    Borrowed(&'input [Expr]),
    /// Operands an expansion appended to `Search::projected`.
    Owned(Range<usize>),
    Counted(&'input AlgebraicOperands),
}

/// The distinct nodes of a pattern DAG, children before parents.
///
/// Each node lists its children as positions of earlier descendants, so values
/// computed in this order are read by position. The root is never a child, so
/// it is kept apart from the descendants.
struct TemplateOrder<'pattern> {
    descendants: Vec<TemplateVisit<'pattern>>,
    root: TemplateVisit<'pattern>,
}

/// A numbered pattern node and the positions of its children, in template order.
struct TemplateVisit<'pattern> {
    pattern: &'pattern Pattern,
    children: Vec<usize>,
}

/// Replacement instructions in children-before-parents order.
///
/// Built only by [`ReplacementProgram::compile`]: every operand position names
/// an earlier instruction, and the root may name any of `operands`.
#[derive(Clone)]
struct Template {
    operands: Vec<ReplacementOp>,
    root: ReplacementOp,
}

#[derive(Clone)]
enum ReplacementOp {
    Capture(Arc<str>),
    Exact(Expr),
    Function { name: Function, args: Vec<usize> },
    Sum(Vec<usize>),
    Product(Vec<usize>),
    Div(usize, usize),
    Pow(usize, usize),
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
/// call depth.
impl Drop for Pattern {
    fn drop(&mut self) {
        // SAFETY: `drop` runs at most once per handle, and `self.0` is never
        // read again after this move.
        let owner = unsafe { ManuallyDrop::take(&mut self.0) };
        if let Some(template) = Arc::into_inner(owner) {
            // Only templates that lose their last owner enter the stack.
            let mut pending = Vec::new();
            release(template, &mut pending);
            while let Some(child) = pending.pop() {
                release(child, &mut pending);
            }
        }
    }
}

fn release(node: PatternNode, pending: &mut Vec<PatternNode>) {
    let mut orphan = |child: Pattern| {
        let mut handle = ManuallyDrop::new(child);
        // SAFETY: `handle` is never dropped or used again, so this is the only
        // move of its field.
        let owner = unsafe { ManuallyDrop::take(&mut handle.0) };
        if let Some(orphaned) = Arc::into_inner(owner) {
            pending.push(orphaned);
        }
    };
    match node {
        PatternNode::Capture { .. } | PatternNode::Exact(_) => {}
        PatternNode::Function { args, .. }
        | PatternNode::Sum(args)
        | PatternNode::Product(args) => args.into_iter().for_each(orphan),
        PatternNode::Div(left, right) | PatternNode::Pow(left, right) => {
            orphan(left);
            orphan(right);
        }
    }
}

impl Heads {
    const SUM: Self = Self(1);
    const PRODUCT: Self = Self(2);
    const DIV: Self = Self(4);
    const POW: Self = Self(8);
    const SQRT: Self = Self(16);
    /// A call of the pattern root's function.
    const CALL: Self = Self(32);
    const OTHER: Self = Self(64);
    /// The heads that supply counted factor groups.
    const COUNTED: Self = Self(Self::PRODUCT.0 | Self::DIV.0 | Self::POW.0 | Self::SQRT.0);
    const ALL: Self = Self(127);
}

#[cfg(test)]
mod tests;
