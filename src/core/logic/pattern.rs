//! Captured structural patterns and explicit, validated rewrite rules.

#![expect(
    clippy::self_named_module_files,
    reason = "the type owner keeps fields private while its descendants implement operations; mod.rs files contain plumbing only"
)]

use core::{num::NonZeroU64, slice::from_ref};

use alloc::{sync::Arc, vec::Vec};

use crate::collections::{HashMap, HashSet};

use super::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, ConcreteMatcher, ExactSources, Expr,
    ExprView, FunctionDefinition, FunctionError, FunctionName, OperandDemand, OperandMatch,
    RewriteError, Substitution, Symbol, UserFunction,
};

/// A structural template with named captures and exact expressions.
///
/// Sum and product operands match independently of their order. At the rule's
/// root they also match subsets, retaining unmatched operands automatically.
/// Inside other templates they cover the complete operand list. A capture may
/// bind a nonempty group of operands; repeated names require the same canonical
/// expression, including grouped captures.
/// Arithmetic operators and vector constructors use expression normalization:
/// sums and products flatten, constants fold, and repeated named captures collect.
/// Root arithmetic templates also find positive integer powers and repeated
/// integer addends hidden by collection. Captures bind canonical virtual operands;
/// whole operands are tried before split units and grouped partitions. Positive
/// power blocks are searched lazily. Scalar coefficient quotients use numeric
/// promotion. Distinct names may consume disjoint units of the same base.
/// Exact rational powers supply integral groups on their unchanged principal base.
/// Integer outer powers can capture fractional blocks present in an arithmetic
/// input. Reciprocal templates require denominator factors; nested poles remain
/// opaque. Bare expressions do not infer fractional roots. Capture predicates
/// reject candidates before continuing the assignment. A missing finite nonzero scalar prefix can
/// be inferred: `2*a` matches `x` with `a = x/2`. Such a template can
/// match every subtree; function templates and guards restrict its scope.
///
/// ```
/// use symb_anafis::{Context, Pattern, RewriteRule};
/// let context = Context::new();
/// let u = context.symb("u")?;
/// let x = context.symb("x")?;
/// let a = Pattern::capture("a", None);
/// let rule = RewriteRule::new(u + &a, 2 * &a)?;
/// assert_eq!(rule.apply_all(&(u + x), None, None)?, 2 * x);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
///
/// `Debug` reports only the root shape of a pattern. It deliberately does not
/// recursively format child templates, so inspecting a deeply nested pattern
/// cannot grow the call stack with the pattern depth.
#[derive(Clone)]
pub struct Pattern(Option<Arc<PatternNode>>);

enum PatternNode {
    Capture {
        name: Arc<str>,
        predicates: CaptureConditions,
    },
    Exact(Expr),
    Function {
        name: FunctionName,
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

enum CaptureConditions {
    Unrestricted,
    One(Arc<CapturePredicate>),
    Many(Vec<Arc<CapturePredicate>>),
}

#[derive(Default)]
struct Normalization {
    captures: HashMap<Arc<str>, (Expr, Vec<Arc<CapturePredicate>>)>,
    markers: HashMap<FunctionName, Arc<str>>,
}

/// The destination of a rewrite rule.
///
/// Pattern destinations validate capture names and compile during construction.
/// Callback destinations receive only the accepted input bindings and return an
/// owning expression. They must be pure and stable for the same bindings;
/// counted matches reuse one result, and capture-free callbacks run lazily once
/// per application pass. Neither form validates mathematical equivalence.
#[derive(Clone)]
#[non_exhaustive]
pub enum Replacement {
    /// A normalized template whose captures must occur in the input pattern.
    Pattern(Pattern),
    /// Construct a destination after matching and guard acceptance.
    /// Callback panics propagate; returned expressions retain DAG invariants.
    Callback(Arc<ReplacementFn>),
}

/// A validated rule applied once to every distinct node of an input DAG.
///
/// ```
/// use symb_anafis::{Context, Pattern, RewriteRule, UserFunction};
/// let context = Context::new();
/// let pair = context.register_function("pair", UserFunction::new(2..=2))?;
/// let argument = Pattern::capture("argument", None);
/// let rule = RewriteRule::new(argument.sin(), argument.cos())?;
/// let x = context.symb("x")?;
/// let input = pair.call(vec![x.sin(), x.sin()])?;
/// assert_eq!(rule.apply_all(&input, None, None)?, pair.call(vec![x.cos(), x.cos()])?);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
/// `Debug` reports the input pattern's root shape and the number of compiled
/// replacement operations. It does not expand the replacement DAG.
#[derive(Clone)]
pub struct RewriteRule {
    pattern: Pattern,
    replacement: ReplacementProgram,
    associative: bool,
}

/// A caller's condition on the captured expressions of a structural match.
/// Returning false rejects that assignment without changing the input DAG.
type RewriteGuard<'guard> = dyn FnMut(&HashMap<&str, Expr>) -> bool + 'guard;

/// A pure, thread-safe condition checked before binding a capture candidate.
/// The result must remain stable for the same expression. Search may evaluate
/// the predicate more than once; effects and panics belong to the caller.
type CapturePredicate = dyn Fn(&Expr) -> bool + Send + Sync;

/// A pure expression constructor over accepted capture bindings.
type ReplacementFn = dyn Fn(&HashMap<&str, Expr>) -> Expr + Send + Sync;

#[derive(Clone)]
enum ReplacementProgram {
    Template(Vec<ReplacementOp>),
    Callback(Arc<ReplacementFn>),
    Concrete(Substitution),
    ConcreteCallback {
        source: Expr,
        callback: Arc<ReplacementFn>,
    },
}

struct MatchWork {
    remaining: Option<usize>,
    limit: usize,
}

#[derive(Debug)]
enum MatchInput<'expr> {
    Borrowed(&'expr Expr),
    Owned(Expr),
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

#[derive(Debug)]
struct Search<'pattern, 'input> {
    pending: Vec<Task<'pattern, 'input>>,
    captured: HashMap<&'pattern str, Expr>,
    expanded: HashSet<(*const PatternNode, Expr)>,
    undo: Vec<Undo<'pattern, 'input>>,
    choices: Vec<Choice<'pattern, 'input>>,
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
    combination: Option<Vec<usize>>,
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

struct PowerTemplate<'pattern> {
    base: &'pattern Pattern,
    numerator: i64,
    denominator: NonZeroU64,
    inverse: bool,
}

#[derive(Debug)]
enum Undo<'pattern, 'input> {
    Capture(&'pattern str),
    Expanded(*const PatternNode, Expr),
    Pushed,
    Popped(Task<'pattern, 'input>),
}

#[derive(Debug)]
struct Choice<'pattern, 'input> {
    checkpoint: usize,
    retry: Group<'pattern, 'input>,
}

#[derive(Clone, Debug)]
enum GroupValues<'input> {
    Borrowed(&'input [(Expr, bool)]),
    Owned(Arc<[(Expr, bool)]>),
    Counted(&'input AlgebraicOperands),
}

#[derive(Clone)]
enum ReplacementOp {
    Capture(Arc<str>),
    Exact(Expr),
    Function {
        name: FunctionName,
        args: Vec<usize>,
    },
    Sum(Vec<usize>),
    Product(Vec<usize>),
    Div(usize, usize),
    Pow(usize, usize),
}

impl Pattern {
    fn node(&self) -> &PatternNode {
        self.0.as_deref().expect("live patterns own their template")
    }
}

impl CaptureConditions {
    fn iter(&self) -> impl Iterator<Item = &Arc<CapturePredicate>> {
        let conditions = match self {
            Self::Unrestricted => [].as_slice(),
            Self::One(predicate) => from_ref(predicate),
            Self::Many(predicates) => predicates.as_slice(),
        };
        conditions.iter()
    }
}

mod constructors;
mod debug;
mod groups;
mod matching;
mod normalize;
mod operators;
mod reclaim;
mod replacement;
mod rules;
mod search;

#[cfg(test)]
mod tests;
